// Builtin Functions
//
// Built-in functions for partial evaluation.

/// Check if a value is a builtin.
pub fn is_builtin_function(name: &str) -> bool {
    matches!(name, "Array" | "Object" | "String" | "Number" | "Boolean" | "Symbol")
}

/// Get result of builtin function.
pub fn evaluate_builtin(_name: &str, _args: &[super::result::ResolvedValue]) -> super::result::ResolvedValue {
    super::result::ResolvedValue::Unknown
}
