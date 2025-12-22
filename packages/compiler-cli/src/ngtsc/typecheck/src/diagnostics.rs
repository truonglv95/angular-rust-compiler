// TypeCheck Diagnostics
//
// Template type-check diagnostics handling.

use super::super::api::TypeCheckError;

/// Diagnostic code for template errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateDiagnosticCode {
    /// Binding to unknown property.
    UnknownProperty = 8002,
    /// Missing pipe.
    MissingPipe = 8004,
    /// Invalid two-way binding.
    InvalidTwoWayBinding = 8005,
    /// Invalid event binding.
    InvalidEventBinding = 8006,
    /// Type error in binding.
    TypeMismatch = 8100,
    /// Required input not provided.
    MissingRequiredInput = 8101,
    /// Unknown element.
    UnknownElement = 8001,
}

impl TemplateDiagnosticCode {
    pub fn code(&self) -> String {
        format!("NG{:04}", *self as u32)
    }
}

/// Create a diagnostic for unknown property.
pub fn create_unknown_property_diagnostic(
    file: &str,
    element: &str,
    property: &str,
) -> TypeCheckError {
    TypeCheckError {
        message: format!(
            "Can't bind to '{}' since it isn't a known property of '{}'",
            property, element
        ),
        code: TemplateDiagnosticCode::UnknownProperty.code(),
        file: Some(file.to_string()),
        start: None,
        length: None,
    }
}

/// Create a diagnostic for unknown element.
pub fn create_unknown_element_diagnostic(
    file: &str,
    element: &str,
) -> TypeCheckError {
    TypeCheckError {
        message: format!(
            "'{}' is not a known element",
            element
        ),
        code: TemplateDiagnosticCode::UnknownElement.code(),
        file: Some(file.to_string()),
        start: None,
        length: None,
    }
}

/// Create a diagnostic for missing pipe.
pub fn create_missing_pipe_diagnostic(
    file: &str,
    pipe_name: &str,
) -> TypeCheckError {
    TypeCheckError {
        message: format!("The pipe '{}' could not be found", pipe_name),
        code: TemplateDiagnosticCode::MissingPipe.code(),
        file: Some(file.to_string()),
        start: None,
        length: None,
    }
}

/// Create a diagnostic for type mismatch.
pub fn create_type_mismatch_diagnostic(
    file: &str,
    expected: &str,
    actual: &str,
) -> TypeCheckError {
    TypeCheckError {
        message: format!(
            "Type '{}' is not assignable to type '{}'",
            actual, expected
        ),
        code: TemplateDiagnosticCode::TypeMismatch.code(),
        file: Some(file.to_string()),
        start: None,
        length: None,
    }
}

/// Create a diagnostic for missing required input.
pub fn create_missing_required_input_diagnostic(
    file: &str,
    directive: &str,
    input: &str,
) -> TypeCheckError {
    TypeCheckError {
        message: format!(
            "Required input '{}' from directive '{}' must be specified",
            input, directive
        ),
        code: TemplateDiagnosticCode::MissingRequiredInput.code(),
        file: Some(file.to_string()),
        start: None,
        length: None,
    }
}
