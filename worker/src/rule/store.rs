use std::collections::HashMap;

use ethers::{abi::Token, types::U256};

pub struct Store {
    pub u256: HashMap<String, U256>,
    pub bool: HashMap<String, bool>,
    pub string: HashMap<String, String>,
    pub array: HashMap<String, Vec<Token>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            u256: HashMap::new(),
            bool: HashMap::new(),
            string: HashMap::new(),
            array: HashMap::new(),
        }
    }

    pub fn assign<T>(&mut self, key: String, value: T)
    where
        T: Into<StoreValue>,
    {
        match value.into() {
            StoreValue::U256(v) => {
                self.u256.insert(key, v);
            }
            StoreValue::Bool(v) => {
                self.bool.insert(key, v);
            }
            StoreValue::String(v) => {
                self.string.insert(key, v);
            }
            StoreValue::Array(v) => {
                self.array.insert(key, v);
            }
        }
    }

    pub fn eval<T>(&self, key: &str) -> T
    where
        T: From<StoreValue>,
    {
        if let Some(v) = self.u256.get(key) {
            return StoreValue::U256(*v).into();
        }
        if let Some(v) = self.bool.get(key) {
            return StoreValue::Bool(*v).into();
        }
        if let Some(v) = self.string.get(key) {
            return StoreValue::String(v.clone()).into();
        }
        if let Some(v) = self.array.get(key) {
            return StoreValue::Array(v.clone()).into();
        } else {
            panic!("Invalid key");
        }
    }
}

pub fn assign<T>(store: &mut Store, key: String, value: T)
where
    T: Into<StoreValue>,
{
    store.assign(key, value);
}

pub fn eval<T>(store: &Store, key: &str) -> T
where
    T: From<StoreValue>,
{
    store.eval(key)
}

pub fn eval_u256(store: &Store, key: &str) -> U256 {
    eval::<U256>(store, key)
}

#[derive(Debug)]
pub enum StoreValue {
    U256(U256),
    Bool(bool),
    String(String),
    Array(Vec<Token>),
}

impl From<U256> for StoreValue {
    fn from(v: U256) -> Self {
        StoreValue::U256(v)
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

impl From<Vec<Token>> for StoreValue {
    fn from(v: Vec<Token>) -> Self {
        StoreValue::Array(v)
    }
}

impl From<StoreValue> for U256 {
    fn from(value: StoreValue) -> Self {
        match value {
            StoreValue::U256(v) => v,
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

impl From<StoreValue> for Vec<Token> {
    fn from(value: StoreValue) -> Self {
        match value {
            StoreValue::Array(v) => v,
            _ => panic!("Cannot convert StoreValue to Vec<Token>"),
        }
    }
}
