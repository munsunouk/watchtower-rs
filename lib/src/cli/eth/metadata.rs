use ethers::types::U64;
use url::Url;

use crate::utils::{constants::ChainID, error::INVALID_PROVIDER_URL};

/// The metadata of the EVM provider.
#[derive(Clone)]
pub struct ProviderMetadata {
    /// The name of this provider.
    pub name: String,
    /// The provider URL. (Allowed values: `http`, `https`)
    pub url: Url,
    /// Id of chain which this client interact with.
    pub id: ChainID,
    /// Block confirmations
    pub block_confirmations: U64,
    /// Get logs batch size
    pub get_logs_batch_size: U64,
}

impl ProviderMetadata {
    pub fn new(
        name: String,
        url: String,
        id: ChainID,
        block_confirmations: u64,
        get_logs_batch_size: u64,
    ) -> Self {
        Self {
            name,
            url: Url::parse(&url).expect(INVALID_PROVIDER_URL),
            id,
            block_confirmations: U64::from(block_confirmations.saturating_add(get_logs_batch_size)),
            get_logs_batch_size: U64::from(get_logs_batch_size),
        }
    }
}
