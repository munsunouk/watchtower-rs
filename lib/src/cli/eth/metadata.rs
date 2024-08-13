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
}

impl ProviderMetadata {
    pub fn new(name: String, url: String, id: ChainID) -> Self {
        Self {
            name,
            url: Url::parse(&url).expect(INVALID_PROVIDER_URL),
            id,
        }
    }
}
