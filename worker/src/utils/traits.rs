use cron::Schedule;
use tokio::time::sleep;

#[async_trait::async_trait]
pub trait PeriodicWorker {
    /// Returns the schedule definition.
    fn schedule(&self) -> Schedule;

    /// Starts the periodic worker.
    async fn run(&mut self);

    /// Wait until it reaches the next schedule.
    async fn wait_until_next_time(&self) {
        let sleep_duration =
            self.schedule().upcoming(chrono::Utc).next().unwrap() - chrono::Utc::now();

        match sleep_duration.to_std() {
            Ok(sleep_duration) => {
                tracing::info!("Before sleep: {:?}", sleep_duration);
                sleep(sleep_duration).await;
                tracing::info!("After sleep: {:?}", sleep_duration);
            }
            Err(_) => return,
        }
    }
}
