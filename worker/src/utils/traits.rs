use cron::Schedule;
use tokio::time::sleep;

#[async_trait::async_trait]
pub trait Fetcher {
    /// Returns the schedule definition.
    fn schedule(&self) -> Schedule;

    /// Starts the fetcher.
    async fn run(&mut self);

    async fn process(&mut self);

    /// Wait until it reaches the next schedule.
    async fn wait_until_next_time(&self) {
        let sleep_duration =
            self.schedule().upcoming(chrono::Utc).next().unwrap() - chrono::Utc::now();

        match sleep_duration.to_std() {
            Ok(sleep_duration) => {
                sleep(sleep_duration).await;
            }
            Err(_) => return,
        }
    }
}
