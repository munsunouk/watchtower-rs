use url::Url;

use crate::utils::{constants::ChainID, error::ClientError};

/// The metadata of the EVM provider.
#[derive(Clone)]
pub struct ProviderMetadata {
    /// The name of this provider.
    pub name: String,
    /// The provider URL. (Allowed values: `http`, `https`)
    pub urls: Vec<Url>,
    /// Id of chain which this client interact with.
    pub id: ChainID,
}

impl ProviderMetadata {
    pub fn new(name: String, urls: Vec<String>, id: ChainID) -> Self {
        Self {
            name,
            urls: urls
                .iter()
                .map(|url| Url::parse(&url).expect(&ClientError::InvalidProviderURL.to_string()))
                .collect(),
            id,
        }
    }
}
