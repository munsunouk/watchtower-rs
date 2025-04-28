use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;
use watch_tower_lib::utils::types::{ChainID, RuleID};

use crate::rule::ContractEvent;

pub type SharedChainState<T> = Arc<RwLock<HashMap<ChainID, HashMap<RuleID, ContractEvent<T>>>>>;
