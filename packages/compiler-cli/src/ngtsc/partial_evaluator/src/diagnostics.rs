// Diagnostics
//
// Diagnostics for partial evaluation.

/// Partial evaluation diagnostic.
#[derive(Debug, Clone)]
pub struct PartialEvalDiagnostic {
    pub message: String,
    pub code: i32,
    pub is_error: bool,
}

impl PartialEvalDiagnostic {
    pub fn unknown_value(context: &str) -> Self {
        Self {
            message: format!("Unable to evaluate value: {}", context),
            code: 8001,
            is_error: false,
        }
    }
    
    pub fn invalid_expression(context: &str) -> Self {
        Self {
            message: format!("Invalid expression: {}", context),
            code: 8002,
            is_error: true,
        }
    }
}
