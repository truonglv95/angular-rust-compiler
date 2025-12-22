// Synthetic Values
//
// Synthetic values for special evaluation cases.

use super::result::ResolvedValue;

/// Create a synthetic value.
pub fn create_synthetic(_kind: &str) -> ResolvedValue {
    ResolvedValue::Unknown
}

/// Check if value is synthetic.
pub fn is_synthetic(_value: &ResolvedValue) -> bool {
    false
}
