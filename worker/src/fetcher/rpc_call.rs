use cron::Schedule;
use tokio::sync::mpsc::UnboundedSender;

use crate::rule::rpc_call::RpcCall;
use crate::utils::constants::INVALID_RPC_CALL_LOG;
use crate::utils::msg::RpcCallRawMessage;
use crate::utils::traits::PeriodicWorker;

/// Struct representing an RPC call fetcher.
#[derive(Clone)]
pub struct RpcCallFetcher {
    /// The RPC call to be fetched.
    pub rpc_call: RpcCall,
    /// The channel sending event messages.
    pub sender: UnboundedSender<RpcCallRawMessage>,
}

#[async_trait::async_trait]
impl PeriodicWorker for RpcCallFetcher {
    /// Returns the schedule for the periodic worker.
    fn schedule(&self) -> Schedule {
        self.rpc_call.rule.check_interval.clone()
    }

    /// Runs the periodic worker, fetching RPC calls at scheduled intervals.
    async fn run(&mut self) {
        loop {
            self.wait_until_next_time().await;
            self.process_rpc_call().await;
        }
    }
}

impl RpcCallFetcher {
    /// Creates a new `RpcCallFetcher` instance.
    ///
    /// # Arguments
    ///
    /// * `rpc_call` - The RPC call to be fetched.
    /// * `sender` - The channel sending event messages.
    ///
    /// # Returns
    ///
    /// A new instance of `RpcCallFetcher`.
    pub fn new(rpc_call: RpcCall, sender: UnboundedSender<RpcCallRawMessage>) -> Self {
        Self { rpc_call, sender }
    }

    /// Processes an RPC call and sends the result through the channel.
    pub async fn process_rpc_call(&mut self) {
        let status = match self.rpc_call.fetch_rpc_call_status().await {
            Ok(token) => token,
            Err(err) => {
                tracing::error!(
                    "[{}] ❗️ [{}] [Error: {}]",
                    &self.rpc_call.rule.url,
                    INVALID_RPC_CALL_LOG,
                    err
                );
                return;
            }
        };

        self.sender
            .send(RpcCallRawMessage::new(status, self.rpc_call.rule.id))
            .unwrap();

        tracing::info!("[{}] ✨", &self.rpc_call.rule.url);
    }
}
