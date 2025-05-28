use chrono::Local;
use std::{
    fs::{create_dir_all, OpenOptions},
    io::Write,
    panic,
    path::Path,
};

use sentry::ClientInitGuard;
use tokio::time::{sleep, Duration};
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
        let time_interval = self.rule.time_interval;

        // Ensure log directory exists
        let log_dir = Path::new("service/log");
        if !log_dir.exists() {
            create_dir_all(log_dir).unwrap();
        }

        let success_log_path = log_dir.join("success.log");
        let failed_log_path = log_dir.join("failed.log");

        // Set up panic hook to log panics
        let failed_log_path_clone = failed_log_path.clone();
        let rule_name = self.rule.name.clone();
        panic::set_hook(Box::new(move |panic_info| {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let panic_message = panic_info.to_string();
            let log_entry = format!("[Failed] {} {} {}\n", rule_name, panic_message, timestamp);

            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&failed_log_path_clone)
            {
                let _ = file.write_all(log_entry.as_bytes());
            }
        }));

        loop {
            match self.get_result().await {
                Ok(_) => {
                    // Log success
                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                    let log_entry = format!("[Info] {} {}\n", self.rule.name, timestamp);

                    if let Ok(mut file) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&success_log_path)
                    {
                        let _ = file.write_all(log_entry.as_bytes());
                    }
                }
                Err(e) => {
                    // Log failure
                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                    let log_entry = format!(
                        "[Failed] {} {} {}\n",
                        self.rule.name,
                        e.to_string(),
                        timestamp
                    );

                    if let Ok(mut file) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&failed_log_path)
                    {
                        let _ = file.write_all(log_entry.as_bytes());
                    }
                    e.log();
                }
            }

            sleep(Duration::from_secs(time_interval)).await;
        }
    }

    async fn get_result(&self) -> Result<(), WorkerError> {
        let result = parse_result(&self.config, &self.rule.script).await.unwrap();

        println!("result: {:?}", result);
        Ok(())
    }
}
