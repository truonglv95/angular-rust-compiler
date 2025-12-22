// Interpreter
//
// Interprets expressions for static evaluation.

use super::result::ResolvedValue;

/// Expression interpreter.
pub struct Interpreter;

impl Interpreter {
    pub fn new() -> Self {
        Self
    }
    
    /// Evaluate a literal.
    pub fn evaluate_literal(&self, value: &str) -> ResolvedValue {
        // Try parsing as number
        if let Ok(n) = value.parse::<f64>() {
            return ResolvedValue::Number(n);
        }
        
        // Try parsing as boolean
        match value {
            "true" => return ResolvedValue::Boolean(true),
            "false" => return ResolvedValue::Boolean(false),
            "null" => return ResolvedValue::Null,
            "undefined" => return ResolvedValue::Undefined,
            _ => {}
        }
        
        // String literal
        if (value.starts_with('"') && value.ends_with('"')) ||
           (value.starts_with('\'') && value.ends_with('\'')) {
            return ResolvedValue::String(value[1..value.len()-1].to_string());
        }
        
        ResolvedValue::Unknown
    }
    
    /// Evaluate binary expression.
    pub fn evaluate_binary(&self, left: &ResolvedValue, op: &str, right: &ResolvedValue) -> ResolvedValue {
        match (left, op, right) {
            (ResolvedValue::Number(l), "+", ResolvedValue::Number(r)) => {
                ResolvedValue::Number(l + r)
            }
            (ResolvedValue::Number(l), "-", ResolvedValue::Number(r)) => {
                ResolvedValue::Number(l - r)
            }
            (ResolvedValue::Number(l), "*", ResolvedValue::Number(r)) => {
                ResolvedValue::Number(l * r)
            }
            (ResolvedValue::Number(l), "/", ResolvedValue::Number(r)) if *r != 0.0 => {
                ResolvedValue::Number(l / r)
            }
            (ResolvedValue::String(l), "+", ResolvedValue::String(r)) => {
                ResolvedValue::String(format!("{}{}", l, r))
            }
            _ => ResolvedValue::Unknown,
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
