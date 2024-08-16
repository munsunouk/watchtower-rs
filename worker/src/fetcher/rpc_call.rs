use cron::Schedule;
use tokio::sync::mpsc::UnboundedSender;

use crate::rule::rpc_call::RpcCall;
use crate::utils::error::WorkerError;
use crate::utils::msg::RpcCallRawMessage;
use crate::utils::traits::Fetcher;

/// Struct representing an RPC call fetcher.
#[derive(Clone)]
pub struct RpcCallFetcher {
    /// The RPC call to be fetched.
    pub rpc_call: RpcCall,
    /// The channel sending event messages.
    pub sender: UnboundedSender<RpcCallRawMessage>,
}

#[async_trait::async_trait]
impl Fetcher for RpcCallFetcher {
    /// Returns the schedule for the fetcher.
    fn schedule(&self) -> Schedule {
        self.rpc_call.rule.call_time_interval.clone()
    }

    /// Runs the fetcher, fetching RPC calls at scheduled intervals.
    async fn run(&mut self) {
        loop {
            self.wait_until_next_time().await;
            self.process().await;
        }
    }

    /// Processes the RPC call, fetching the RPC call status and sending the RPC call log.
    async fn process(&mut self) {
        let status = match self.rpc_call.fetch_rpc_call_status().await {
            Ok(token) => token,
            Err(err) => {
                tracing::error!(
                    "[{}] ❗️ [Error: {}]",
                    &self.rpc_call.rule.url,
                    WorkerError::InvalidRpcCallLog(err.to_string())
                );
                return;
            }
        };

        self.sender
            .send(RpcCallRawMessage::new(status, self.rpc_call.rule.id))
            .unwrap();

        tracing::info!("[Rule ID : {}] ✨", &self.rpc_call.rule.id);
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
}
