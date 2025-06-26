use ethers::types::U256;
use std::collections::HashMap;

use watch_tower_lib::utils::types::GeneralToken;

#[derive(Clone)]
pub struct SymbolTable {
    pub store: HashMap<String, GeneralToken>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn assign(&mut self, key: &str, value: &GeneralToken) {
        self.store.insert(key.to_string(), value.to_owned());
    }

    pub fn eval(&self, key: &str) -> GeneralToken {
        self.store
            .get(key)
            .cloned()
            .unwrap_or_else(|| panic!("Invalid key: {key}"))
    }

    pub fn check_store_value(&self, key: &str) -> GeneralToken {
        GeneralToken::Bool(self.store.contains_key(key))
    }
}

pub fn assign(store: &mut SymbolTable, key: &str, value: &GeneralToken) {
    store.assign(key, value);
}

pub fn check_store_value(store: &SymbolTable, key: &str) -> GeneralToken {
    store.check_store_value(key)
}

pub fn eval(store: &SymbolTable, key: &str) -> GeneralToken {
    store.eval(key)
}

#[derive(Debug, Clone)]
pub enum StoreValue {
    Uint(U256),
    Bool(bool),
    String(String),
    Array(Vec<GeneralToken>),
}

impl From<U256> for StoreValue {
    fn from(v: U256) -> Self {
        StoreValue::Uint(v)
    }
}

impl From<bool> for StoreValue {
    fn from(v: bool) -> Self {
        StoreValue::Bool(v)
    }
}

impl From<String> for StoreValue {
    fn from(v: String) -> Self {
        StoreValue::String(v)
    }
}

impl From<Vec<GeneralToken>> for StoreValue {
    fn from(v: Vec<GeneralToken>) -> Self {
        StoreValue::Array(v)
    }
}

impl From<StoreValue> for U256 {
    fn from(value: StoreValue) -> Self {
        match value {
            StoreValue::Uint(v) => v,
            _ => panic!("Cannot convert StoreValue to U256"),
        }
    }
}

impl From<StoreValue> for bool {
    fn from(value: StoreValue) -> Self {
        match value {
            StoreValue::Bool(v) => v,
            _ => panic!("Cannot convert StoreValue to bool"),
        }
    }
}

impl From<StoreValue> for String {
    fn from(value: StoreValue) -> Self {
        match value {
            StoreValue::String(v) => v,
            _ => panic!("Cannot convert StoreValue to String"),
        }
    }
}

impl From<StoreValue> for Vec<GeneralToken> {
    fn from(value: StoreValue) -> Self {
        match value {
            StoreValue::Array(v) => v,
            _ => panic!("Cannot convert StoreValue to Vec<Token>"),
        }
    }
}
