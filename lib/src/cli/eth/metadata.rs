use url::Url;

use crate::utils::{error::ClientError, parse_string_to_url, types::ChainID};

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
    pub fn new(name: &str, urls: &[String], id: &ChainID) -> Result<Self, ClientError> {
        Ok(Self {
            name: name.to_string(),
            urls: urls
                .iter()
                .map(|url| parse_string_to_url(url))
                .collect::<Result<Vec<Url>, _>>()?,
            id: *id,
        })
    }
}
