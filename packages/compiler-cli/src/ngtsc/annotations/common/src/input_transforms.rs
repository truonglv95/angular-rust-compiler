// Input Transforms
//
// Generates additional fields for classes with transformed inputs.

use super::factory::CompileResult;

/// Input mapping with optional transform.
#[derive(Debug, Clone)]
pub struct InputMapping {
    /// The class property name.
    pub class_property_name: String,
    /// The binding property name (public name).
    pub binding_property_name: String,
    /// Whether this is required.
    pub required: bool,
    /// Transform function metadata, if any.
    pub transform: Option<InputTransform>,
}

/// Metadata about an input transform function.
#[derive(Debug, Clone)]
pub struct InputTransform {
    /// The type expression for the transform's input type.
    pub type_expr: String,
}

/// Generates additional fields for inputs with transform functions.
pub fn compile_input_transform_fields(inputs: &[InputMapping]) -> Vec<CompileResult> {
    let mut extra_fields = Vec::new();

    for input in inputs {
        // Signal inputs capture transform in InputSignal, no coercion member needed.
        if let Some(transform) = &input.transform {
            extra_fields.push(CompileResult {
                name: format!("ngAcceptInputType_{}", input.class_property_name),
                initializer: String::new(),
                statements: Vec::new(),
                type_expr: Some(transform.type_expr.clone()),
                deferrable_imports: None,
                diagnostics: Vec::new(),
            });
        }
    }

    extra_fields
}
