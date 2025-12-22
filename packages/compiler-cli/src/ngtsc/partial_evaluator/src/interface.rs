// Interface
//
// Public interface for partial evaluator.

use std::collections::HashMap;
use super::result::ResolvedValue;

/// Partial evaluator interface.
pub struct PartialEvaluator {
    known_values: HashMap<String, ResolvedValue>,
}

impl PartialEvaluator {
    pub fn new() -> Self {
        Self {
            known_values: HashMap::new(),
        }
    }
    
    pub fn set_known(&mut self, name: &str, value: ResolvedValue) {
        self.known_values.insert(name.to_string(), value);
    }
    
    pub fn get_known(&self, name: &str) -> Option<&ResolvedValue> {
        self.known_values.get(name)
    }
    
    pub fn evaluate(&self, _expression: &str) -> ResolvedValue {
        ResolvedValue::Unknown
    }
}

impl Default for PartialEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Function reference.
#[derive(Debug, Clone)]
pub struct FunctionRef {
    pub name: String,
    pub module: Option<String>,
}

/// Class reference.
#[derive(Debug, Clone)]
pub struct ClassRef {
    pub name: String,
    pub module: Option<String>,
}
