// Result Types
//
// Result types for partial evaluation.

use std::collections::HashMap;

/// Resolved value from partial evaluation.
#[derive(Debug, Clone)]
pub enum ResolvedValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
    Undefined,
    Array(Vec<ResolvedValue>),
    Object(HashMap<String, ResolvedValue>),
    Function(String),
    Class(String),
    Unknown,
    Error(String),
}

impl ResolvedValue {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ResolvedValue::String(s) => Some(s),
            _ => None,
        }
    }
    
    pub fn as_number(&self) -> Option<f64> {
        match self {
            ResolvedValue::Number(n) => Some(*n),
            _ => None,
        }
    }
    
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ResolvedValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
    
    pub fn as_array(&self) -> Option<&Vec<ResolvedValue>> {
        match self {
            ResolvedValue::Array(arr) => Some(arr),
            _ => None,
        }
    }
    
    pub fn as_object(&self) -> Option<&HashMap<String, ResolvedValue>> {
        match self {
            ResolvedValue::Object(obj) => Some(obj),
            _ => None,
        }
    }
    
    pub fn get_property(&self, key: &str) -> Option<&ResolvedValue> {
        match self {
            ResolvedValue::Object(obj) => obj.get(key),
            _ => None,
        }
    }
    
    pub fn is_known(&self) -> bool {
        !matches!(self, ResolvedValue::Unknown | ResolvedValue::Error(_))
    }
}
