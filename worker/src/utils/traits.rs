use cron::Schedule;
use tokio::time::sleep;

use watch_tower_lib::utils::{constants::DEFAULT_INDEX, error::IndexType, DbRuleType};

use crate::rule::set_schedule;

use super::{
    constants::{CONTROLLER_NAME, HEALETH_CHECK_INTERVAL},
    error::WorkerError,
    log::TraceLog,
};

#[async_trait::async_trait]
pub trait Fetcher: Send + Sync {
    /// Returns the schedule definition.
    fn schedule(&self) -> Result<Schedule, WorkerError>;

    /// Starts the fetcher.
    async fn run(&mut self);

    async fn process(&mut self) -> Result<(), WorkerError>;

    /// Wait until it reaches the next schedule.
    async fn wait_until_next_time(&self) -> Result<(), WorkerError> {
        let sleep_duration = self
            .schedule()?
            .upcoming(chrono::Utc)
            .next()
            .ok_or(WorkerError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?
            - chrono::Utc::now();

        match sleep_duration.to_std() {
            Ok(sleep_duration) => {
                sleep(sleep_duration).await;
                Ok(())
            }
            Err(_) => Err(WorkerError::InvalidTypeConvert),
        }
    }
}

#[async_trait::async_trait]
pub trait Controller: Send + Sync {
    async fn run(&mut self);

    async fn process(&mut self) -> Result<(), WorkerError>;

    async fn wait_until_next_health_check() -> Result<(), WorkerError>
    where
        Self: Sized,
    {
        let health_check_schedule = set_schedule(HEALETH_CHECK_INTERVAL as usize)?;

        let sleep_duration = health_check_schedule
            .upcoming(chrono::Utc)
            .next()
            .ok_or(WorkerError::InvalidIndex(IndexType::USize(DEFAULT_INDEX)))?
            - chrono::Utc::now();

        match sleep_duration.to_std() {
            Ok(sleep_duration) => {
                sleep(sleep_duration).await;
                Ok(())
            }
            Err(_) => Err(WorkerError::InvalidTypeConvert),
        }
    }

    fn health_check(&self, rule_type: DbRuleType) {
        TraceLog::HealthCheckPassed(rule_type, CONTROLLER_NAME.to_string()).info()
    }
}
