use sentry::ClientInitGuard;

use watch_tower_lib::config::{set_config, set_rule, Configuration, Rule};

use crate::{
    parse::parse_result,
    utils::{constants::CONFIG_PATH, error::WorkerError, setting::build_sentry},
};

pub struct Runner {
    pub config: Configuration,
    pub rule: Rule,
    pub _sentry_guard: ClientInitGuard,
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
    pub async fn new(rule_path: &str) -> Result<Self, WorkerError> {
        let config = set_config(CONFIG_PATH);
        let rule = set_rule(rule_path);

        //Sentry
        let _sentry_guard =
            build_sentry(&config.sentry_config.dsn, &config.sentry_config.environment)?;

        Ok(Self {
            config,
            rule,
            _sentry_guard,
        })
    }

    /// Runs the get_result function periodically based on the rule's time_interval
    pub async fn run(&self) -> Result<(), WorkerError> {
        self.get_result().await
    }

    async fn get_result(&self) -> Result<(), WorkerError> {
        let result = parse_result(&self.config, &self.rule).await.unwrap();

        println!("result: {:?}", result);
        Ok(())
    }
}
