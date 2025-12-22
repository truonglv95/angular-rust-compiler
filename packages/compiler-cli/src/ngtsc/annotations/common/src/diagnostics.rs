// Diagnostics
//
// Functions for generating diagnostic messages related to annotations.

use std::fmt;

/// Error codes for annotation diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Decorator argument is not a literal.
    DecoratorArgNotLiteral = 1001,
    /// Decorator has wrong number of arguments.
    DecoratorArityWrong = 1002,
    /// Value has wrong type.
    ValueHasWrongType = 1003,
    /// Component/directive is not standalone.
    ComponentNotStandalone = 2001,
    /// NgModule has duplicate declarations.
    NgModuleDuplicateDeclaration = 2002,
    /// NgModule has invalid declaration.
    NgModuleInvalidDeclaration = 2003,
    /// Provider is not injectable.
    ProviderNotInjectable = 3001,
    /// Missing generic type for ModuleWithProviders.
    NgModuleMwpMissingGeneric = 3002,
    /// Initializer API disallowed visibility.
    InitializerApiDisallowedVisibility = 4001,
    /// Initializer API no required function.
    InitializerApiNoRequired = 4002,
    /// Local compilation unresolved const.
    LocalCompilationUnresolvedConst = 5001,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NG{:04}", *self as u32)
    }
}

/// A diagnostic with associated node information.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Error code.
    pub code: ErrorCode,
    /// Error message.
    pub message: String,
    /// File path where the error occurred.
    pub file: Option<String>,
    /// Line number (1-indexed).
    pub line: Option<u32>,
    /// Column number (1-indexed).
    pub column: Option<u32>,
    /// Related information.
    pub related: Vec<RelatedInfo>,
}

/// Related information for a diagnostic.
#[derive(Debug, Clone)]
pub struct RelatedInfo {
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
}

impl Diagnostic {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            file: None,
            line: None,
            column: None,
            related: Vec::new(),
        }
    }
    
    pub fn with_location(mut self, file: impl Into<String>, line: u32, column: u32) -> Self {
        self.file = Some(file.into());
        self.line = Some(line);
        self.column = Some(column);
        self
    }
    
    pub fn with_related(mut self, info: RelatedInfo) -> Self {
        self.related.push(info);
        self
    }
    
    pub fn format(&self) -> String {
        let location = match (&self.file, self.line, self.column) {
            (Some(file), Some(line), Some(col)) => format!("{}:{}:{}: ", file, line, col),
            (Some(file), Some(line), None) => format!("{}:{}: ", file, line),
            (Some(file), None, None) => format!("{}: ", file),
            _ => String::new(),
        };
        format!("error {}: {}{}", self.code, location, self.message)
    }
}

/// A fatal diagnostic error that stops compilation.
#[derive(Debug, Clone)]
pub struct FatalDiagnosticError {
    pub code: ErrorCode,
    pub message: String,
    pub node: String,
    pub chain: Vec<String>,
}

impl FatalDiagnosticError {
    pub fn new(code: ErrorCode, node: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            node: node.into(),
            chain: Vec::new(),
        }
    }
    
    pub fn with_chain(mut self, message: impl Into<String>) -> Self {
        self.chain.push(message.into());
        self
    }
}

impl fmt::Display for FatalDiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for FatalDiagnosticError {}

/// Create a diagnostic chain.
pub fn make_diagnostic_chain(message: impl Into<String>, chain: Vec<String>) -> Vec<String> {
    let mut result = vec![message.into()];
    result.extend(chain);
    result
}

/// Create a FatalDiagnosticError for value with wrong type.
pub fn create_value_has_wrong_type_error(
    node: impl Into<String>,
    value_desc: impl Into<String>,
    message: impl Into<String>,
) -> FatalDiagnosticError {
    FatalDiagnosticError::new(
        ErrorCode::ValueHasWrongType,
        node,
        format!("{} {}", message.into(), value_desc.into()),
    )
}

/// Create a diagnostic for duplicate declarations.
pub fn make_duplicate_declaration_error(
    class_name: &str,
    modules: &[(&str, Option<(String, u32)>)],
    kind: &str,
) -> Diagnostic {
    let mut diag = Diagnostic::new(
        ErrorCode::NgModuleDuplicateDeclaration,
        format!(
            "The {} '{}' is declared in multiple NgModules.",
            kind, class_name
        ),
    );
    
    for (module_name, location) in modules {
        let mut info = RelatedInfo {
            message: format!("'{}' is listed in @NgModule.declarations of '{}'", class_name, module_name),
            file: None,
            line: None,
        };
        if let Some((file, line)) = location {
            info.file = Some(file.clone());
            info.line = Some(*line);
        }
        diag.related.push(info);
    }
    
    diag
}

/// Get diagnostics for provider classes.
pub fn get_provider_diagnostics(
    provider_class_names: &[String],
    injectable_classes: &std::collections::HashSet<String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    for class_name in provider_class_names {
        if !injectable_classes.contains(class_name) {
            diagnostics.push(Diagnostic::new(
                ErrorCode::ProviderNotInjectable,
                format!(
                    "Class '{}' is used as a provider but doesn't have an @Injectable() decorator or a constructor that can be injected.",
                    class_name
                ),
            ));
        }
    }
    
    diagnostics
}

/// Get diagnostic for undecorated class with Angular features.
pub fn get_undecorated_class_with_angular_features_diagnostic(
    class_name: &str,
) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::DecoratorArgNotLiteral,
        format!(
            "Class '{}' uses Angular features but is not decorated. Please add an explicit Angular decorator.",
            class_name
        ),
    )
}

/// Assert that a value is resolved in local compilation mode.
pub fn assert_local_compilation_unresolved_const(
    is_local_compilation: bool,
    is_resolved: bool,
    node: &str,
    error_message: &str,
) -> Result<(), FatalDiagnosticError> {
    if is_local_compilation && !is_resolved {
        Err(FatalDiagnosticError::new(
            ErrorCode::LocalCompilationUnresolvedConst,
            node,
            error_message,
        ))
    } else {
        Ok(())
    }
}
