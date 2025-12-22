// Dynamic Value
//
// Represents a dynamic (non-static) value.

/// Dynamic value that cannot be statically evaluated.
#[derive(Debug, Clone)]
pub struct DynamicValue {
    /// Reason for dynamic value.
    pub reason: DynamicReason,
    /// Related node.
    pub node: Option<String>,
}

/// Reason why a value is dynamic.
#[derive(Debug, Clone)]
pub enum DynamicReason {
    /// Value from external module.
    FromExternalModules,
    /// Value requires runtime.
    RequiresRuntime,
    /// Unknown identifier.
    UnknownIdentifier,
    /// Complex expression.
    ComplexExpression,
}

impl DynamicValue {
    pub fn from_external() -> Self {
        Self {
            reason: DynamicReason::FromExternalModules,
            node: None,
        }
    }
    
    pub fn requires_runtime() -> Self {
        Self {
            reason: DynamicReason::RequiresRuntime,
            node: None,
        }
    }
    
    pub fn unknown(node: impl Into<String>) -> Self {
        Self {
            reason: DynamicReason::UnknownIdentifier,
            node: Some(node.into()),
        }
    }
}
