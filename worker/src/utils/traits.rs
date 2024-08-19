use cron::Schedule;
use tokio::time::sleep;

use super::{
    constants::DEFAULT_INDEX,
    error::{IndexType, WorkerError},
};

#[async_trait::async_trait]
pub trait Fetcher {
    /// Returns the schedule definition.
    fn schedule(&self) -> Result<Schedule, WorkerError>;

    /// Starts the fetcher.
    async fn run(&mut self) -> Result<(), WorkerError>;

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
