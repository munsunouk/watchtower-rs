use ethers::{abi::Token, types::U256};
use std::collections::HashMap;

pub trait TokenConvert {
    fn to_token(self) -> Token;
    fn from_token(token: Token) -> Self;
}

impl TokenConvert for U256 {
    fn to_token(self) -> Token {
        Token::Int(self)
    }

    fn from_token(token: Token) -> Self {
        if let Token::Int(v) = token {
            v
        } else {
            panic!("Expected Token::Int, got {:?}", token);
        }
    }
}

pub struct SymbolTable {
    pub store: HashMap<String, Token>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn assign(&mut self, key: String, value: Token) {
        self.store.insert(key, value);
    }

    pub fn eval(&self, key: &str) -> Token {
        self.store
            .get(key)
            .cloned()
            .unwrap_or_else(|| panic!("Invalid key: {}", key))
    }
}

pub fn assign(store: &mut SymbolTable, key: String, value: Token) {
    store.assign(key, value);
}

// pub fn assign<T: TokenConvert>(store: &mut SymbolTable, key: String, value: T) {
//     store.assign(key, value);
// }

pub fn eval(store: &SymbolTable, key: &str) -> Token {
    store.eval(key)
}

#[derive(Debug, Clone)]
pub enum StoreValue {
    Uint(U256),
    Bool(bool),
    String(String),
    Array(Vec<Token>),
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

impl From<Vec<Token>> for StoreValue {
    fn from(v: Vec<Token>) -> Self {
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

impl From<StoreValue> for Vec<Token> {
    fn from(value: StoreValue) -> Self {
        match value {
            StoreValue::Array(v) => v,
            _ => panic!("Cannot convert StoreValue to Vec<Token>"),
        }
    }
}
