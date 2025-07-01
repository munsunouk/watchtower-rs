use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use sentry::ClientInitGuard;
use tokio::{sync::Mutex, time};
use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};

use watch_tower_lib::{
    cli::db::{data::RuleData, postgres::PostgresClient},
    utils::DbTable,
};

use crate::{
    parse::evaluation::Evaluator,
    utils::{
        config::{Configuration, ParamConfig},
        constants::{SQLX_QUERY_WARN, TIME_FORMAT},
        error::WorkerError,
        setting::{build_sentry, set_config, set_param_config},
    },
    Args,
};

pub struct Runner {
    pub _sentry_guard: ClientInitGuard,
    pub evaluators: Vec<Arc<Mutex<(Evaluator, DateTime<Utc>)>>>,
}

impl Runner {
    /// # Description
    /// This function creates a new `Runner` instance.
    /// # Arguments
    /// * `config_path` - A string slice that holds the path to the configuration file.
    ///
    /// # Returns
    ///
    /// A new instance of `Runner`.
    pub async fn new(args: Args) -> Result<Self, WorkerError> {
        Self::set_log()?;

        let project_root = std::env::current_dir()?;
        let config_path = project_root.join(&args.config_path);
        let param_config_path = project_root.join(&args.param_path);

        let mut config = set_config(&config_path)?;
        config.set_path(&args.config_path);

        let param_config = set_param_config(&param_config_path)?;

        //DB
        let db_client = Self::build_db(&config.postgres_config.url).await?;

        if !db_client.schema_exists().await? {
            db_client.initiate(&project_root).await?;
        }

        let rules = Self::load_rules(&db_client).await?;

        let evaluators = Self::build_evaluators(&config, &param_config, rules).await?;

        //Sentry
        let _sentry_guard =
            build_sentry(&config.sentry_config.dsn, &config.sentry_config.environment)?;

        Ok(Self {
            _sentry_guard,
            evaluators,
        })
    }

    /// Runs the get_result function periodically based on the rule's time_interval
    pub async fn run(&mut self) -> Result<(), WorkerError> {
        // tracing::info!(
        //     "🚀 Starting Watch Tower Worker with {} tasks",
        //     self.evaluators.len()
        // );

        // Keep the main thread alive indefinitely
        loop {
            let tasks_to_spawn = self.set_tasks_to_spawn().await;

            if !tasks_to_spawn.is_empty() {
                self.spawn_evaluator_tasks(tasks_to_spawn).await?;
            }

            time::sleep(time::Duration::from_millis(100)).await;
        }
    }

    /// # Description
    /// This function builds the database client.
    /// # Arguments
    ///
    /// * `db_url` - The URL of the database.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `PostgresClient` instance.
    async fn build_db(db_url: &str) -> Result<PostgresClient, WorkerError> {
        let client = PostgresClient::new(db_url).await?;
        Ok(client)
    }

    async fn build_evaluators(
        config: &Configuration,
        param_config: &ParamConfig,
        rules: HashMap<String, RuleData>,
    ) -> Result<Vec<Arc<Mutex<(Evaluator, DateTime<Utc>)>>>, WorkerError> {
        let mut evaluators: Vec<Arc<Mutex<(Evaluator, DateTime<Utc>)>>> = Vec::new();

        for (_, rule) in rules {
            let evaluator = Evaluator::new(config, param_config, rule);
            let next_time = Utc::now();
            evaluators.push(Arc::new(Mutex::new((evaluator, next_time))));
        }
        Ok(evaluators)
    }

    /// # Description
    /// This function loads contract call block logs from the database.
    /// # Arguments
    /// * `db_client` - A reference to the Postgres client.
    /// # Returns
    ///
    /// A hashmap of `RuleID` to `U64`.
    pub async fn load_rules(
        db_client: &PostgresClient,
    ) -> Result<HashMap<String, RuleData>, WorkerError> {
        let result = db_client.select_table(DbTable::Rule).await?;

        let mut rules: HashMap<String, RuleData> = HashMap::new();

        for row in result {
            let rule = RuleData::try_from(&row)?;
            let rule_name = rule.name.to_string();
            rules.insert(rule_name, rule);
        }

        Ok(rules)
    }

    /// Sets the log configuration.
    fn set_log() -> Result<(), WorkerError> {
        let format = fmt::format()
            .with_timer(fmt::time::ChronoLocal::new(TIME_FORMAT.to_string()))
            .with_level(true)
            .with_target(false)
            .with_ansi(true)
            .with_file(false)
            .compact();

        tracing_subscriber::fmt()
            .event_format(format)
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive(Level::INFO.into())
                    .add_directive(SQLX_QUERY_WARN.parse()?), // Exclude sqlx::query logs
            )
            .init();

        Ok(())
    }

    pub async fn set_tasks_to_spawn(&self) -> Vec<usize> {
        let now = chrono::Utc::now();
        let mut tasks_to_spawn = Vec::new();

        let evaluators = self.evaluators.clone();

        for (index, evaluator) in evaluators.iter().enumerate() {
            let guard = evaluator.lock().await;

            if now >= guard.1 {
                tasks_to_spawn.push(index);
            }
        }

        tasks_to_spawn
    }

    pub async fn spawn_evaluator_tasks(
        &mut self,
        tasks_to_spawn: Vec<usize>,
    ) -> Result<(), WorkerError> {
        let mut evaluators = self.evaluators.clone();
        let mut handles = Vec::new();

        // Spawn tasks that are ready to execute
        for task_index in tasks_to_spawn {
            let evaluator_arc = evaluators[task_index].clone();

            let handle = tokio::task::spawn(async move {
                // Execute one iteration of the task
                evaluator_arc.lock().await.0.run().await?;

                // Update next execution time
                {
                    let mut guard = evaluator_arc.lock().await;
                    let next_time = guard.0.get_next_execution_time()?;
                    guard.1 = next_time;
                }

                Ok::<Arc<Mutex<(Evaluator, DateTime<Utc>)>>, WorkerError>(evaluator_arc)
            });

            handles.push((task_index, handle));
        }

        for (task_index, handle) in handles {
            let updated_evaluator = handle.await??;
            evaluators[task_index] = updated_evaluator;
        }

        self.evaluators = evaluators;

        Ok(())
    }
}
