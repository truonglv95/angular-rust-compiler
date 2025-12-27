// Factory Compilation
//
// Functions for compiling factory definitions.

/// Result of compiling a factory.
#[derive(Debug, Clone)]
pub struct CompileResult {
    /// The name of the compiled field (e.g., "ɵfac").
    pub name: String,
    /// The initializer expression.
    pub initializer: String,
    /// Additional statements to include.
    pub statements: Vec<String>,
    /// The type expression.
    pub type_expr: Option<String>,
    /// Deferrable imports, if any.
    pub deferrable_imports: Option<Vec<String>>,
    /// Diagnostics produced during compilation.
    pub diagnostics: Vec<String>,
}

impl CompileResult {
    pub fn new(name: impl Into<String>, initializer: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            initializer: initializer.into(),
            statements: Vec::new(),
            type_expr: None,
            deferrable_imports: None,
            diagnostics: Vec::new(),
        }
    }

    pub fn with_statements(mut self, statements: Vec<String>) -> Self {
        self.statements = statements;
        self
    }

    pub fn with_type(mut self, type_expr: impl Into<String>) -> Self {
        self.type_expr = Some(type_expr.into());
        self
    }
}

/// Metadata for factory compilation.
#[derive(Debug, Clone)]
pub struct R3FactoryMetadata {
    /// The name of the type for which a factory is being generated.
    pub name: String,
    /// The type expression for the factory.
    pub type_name: String,
    /// Dependencies for the factory.
    pub deps: Option<Vec<R3DependencyMetadata>>,
    /// Target of the factory (e.g., Directive, Component, Injectable).
    pub target: FactoryTarget,
}

/// Dependency metadata for injection.
#[derive(Debug, Clone)]
pub struct R3DependencyMetadata {
    /// Token expression for the dependency.
    pub token: String,
    /// Whether the dependency is optional.
    pub optional: bool,
    /// Whether the dependency is from the host.
    pub host: bool,
    /// Whether to use self.
    pub self_: bool,
    /// Whether to skip self.
    pub skip_self: bool,
}

/// Target type for factory generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryTarget {
    Directive,
    Component,
    Injectable,
    Pipe,
    NgModule,
}

/// Compile the factory definition field.
pub fn compile_ng_factory_def_field(metadata: &R3FactoryMetadata) -> CompileResult {
    let factory_expr = generate_factory_expression(metadata);

    CompileResult {
        name: "ɵfac".to_string(),
        initializer: factory_expr,
        statements: Vec::new(),
        type_expr: Some(format!("FactoryDeclaration<{}, never>", metadata.type_name)),
        deferrable_imports: None,
        diagnostics: Vec::new(),
    }
}

/// Compile a declare factory function (for partial compilation).
pub fn compile_declare_factory(metadata: &R3FactoryMetadata) -> CompileResult {
    let factory_expr = generate_declare_factory_expression(metadata);

    CompileResult {
        name: "ɵfac".to_string(),
        initializer: factory_expr,
        statements: Vec::new(),
        type_expr: Some(format!("FactoryDeclaration<{}, never>", metadata.type_name)),
        deferrable_imports: None,
        diagnostics: Vec::new(),
    }
}

fn generate_factory_expression(metadata: &R3FactoryMetadata) -> String {
    match &metadata.deps {
        Some(deps) if !deps.is_empty() => {
            let dep_tokens: Vec<String> = deps
                .iter()
                .map(|d| format!("inject({})", d.token))
                .collect();
            format!(
                "function {}Factory(t) {{ return new (t || {})({}); }}",
                metadata.name,
                metadata.name,
                dep_tokens.join(", ")
            )
        }
        _ => {
            format!(
                "function {}Factory(t) {{ return new (t || {})(); }}",
                metadata.name, metadata.name
            )
        }
    }
}

fn generate_declare_factory_expression(metadata: &R3FactoryMetadata) -> String {
    format!(
        "ɵɵngDeclareFactory({{ minVersion: \"14.0.0\", version: \"0.0.0\", ngImport: i0, type: {}, deps: [], target: i0.ɵɵFactoryTarget.{:?} }})",
        metadata.name,
        metadata.target
    )
}
