// Transform API - Types and utilities for property transforms
//
// This module provides the common types and helper functions used by
// all initializer API transforms (input, output, query, model).

use crate::ngtsc::imports::ImportedSymbolsTracker;

/// Result of a property transformation.
#[derive(Debug, Clone)]
pub struct PropertyTransformResult {
    /// Whether the property was transformed.
    pub transformed: bool,
    /// New decorators to add to the property.
    pub decorators: Vec<SyntheticDecorator>,
    /// New initializer expression, if changed.
    pub new_initializer: Option<String>,
}

impl PropertyTransformResult {
    /// Create a result indicating no transformation was needed.
    pub fn unchanged() -> Self {
        Self {
            transformed: false,
            decorators: Vec::new(),
            new_initializer: None,
        }
    }

    /// Create a result with decorators added.
    pub fn with_decorators(decorators: Vec<SyntheticDecorator>) -> Self {
        Self {
            transformed: true,
            decorators,
            new_initializer: None,
        }
    }
}

/// A synthetic decorator to be added to a class member.
#[derive(Debug, Clone)]
pub struct SyntheticDecorator {
    /// The decorator name (e.g., "Input", "Output").
    pub name: String,
    /// Arguments to pass to the decorator.
    pub args: Vec<String>,
    /// Import path for the decorator.
    pub import_from: String,
}

impl SyntheticDecorator {
    /// Create a new synthetic decorator.
    pub fn new(name: impl Into<String>, import_from: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: Vec::new(),
            import_from: import_from.into(),
        }
    }

    /// Add an argument to the decorator.
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments to the decorator.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }
}

/// Simplified property info for transforms.
/// This avoids complex lifetime issues with the actual ClassMember type.
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    /// Property name.
    pub name: String,
    /// Stringified value (for checking call patterns).
    pub value_string: Option<String>,
    /// Whether the property is static.
    pub is_static: bool,
}

/// Function type that can be used to transform class properties.
///
/// Returns a transformation result indicating whether and how the property was transformed.
pub type PropertyTransform = fn(
    property: &PropertyInfo,
    import_tracker: &ImportedSymbolsTracker,
    is_core: bool,
) -> PropertyTransformResult;

/// Creates an import and access for a given Angular core import while
/// ensuring the decorator symbol access can be traced back to an Angular core
/// import in order to make the synthetic decorator compatible with the JIT
/// decorator downlevel transform.
pub fn create_synthetic_angular_core_decorator_access(
    decorator_name: &str,
) -> SyntheticDecorator {
    SyntheticDecorator::new(decorator_name, "@angular/core")
}

/// Helper to cast an expression as `any` type.
#[allow(dead_code)]
pub fn cast_as_any(expr: &str) -> String {
    format!("({} as any)", expr)
}

/// Check if a property value is a signal input call.
pub fn is_signal_input_call(value: Option<&str>, is_core: bool) -> bool {
    if let Some(value_str) = value {
        if is_core {
            // Handle input() and input.required() with or without generic type params
            return value_str.starts_with("input(") 
                || value_str.starts_with("input.required(")
                || value_str.starts_with("input.required<");
        }
        // TODO: Check for imported `input` from @angular/core
    }
    false
}

/// Check if a property value is a signal output call.
pub fn is_signal_output_call(value: Option<&str>, is_core: bool) -> bool {
    if let Some(value_str) = value {
        if is_core {
            return value_str.starts_with("output(") || value_str.starts_with("output<");
        }
    }
    false
}

/// Check if a property value is a signal model call.
pub fn is_signal_model_call(value: Option<&str>, is_core: bool) -> bool {
    if let Some(value_str) = value {
        if is_core {
            // Handle model() and model.required() with or without generic type params
            return value_str.starts_with("model(")
                || value_str.starts_with("model<")
                || value_str.starts_with("model.required(")
                || value_str.starts_with("model.required<");
        }
    }
    false
}

/// Check if a property value is a query function call.
pub fn is_query_function_call(value: Option<&str>, is_core: bool) -> bool {
    if let Some(value_str) = value {
        if is_core {
            return value_str.starts_with("viewChild(")
                || value_str.starts_with("viewChildren(")
                || value_str.starts_with("contentChild(")
                || value_str.starts_with("contentChildren(");
        }
    }
    false
}
