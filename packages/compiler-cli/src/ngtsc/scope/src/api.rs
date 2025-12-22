// Scope API
//
// Public API types for scope resolution.

/// Represents the exports of a component, directive, pipe, or NgModule.
#[derive(Debug, Clone)]
pub struct ExportScope {
    /// Exported components.
    pub components: Vec<DirectiveExport>,
    /// Exported directives.
    pub directives: Vec<DirectiveExport>,
    /// Exported pipes.
    pub pipes: Vec<PipeExport>,
}

impl ExportScope {
    pub fn empty() -> Self {
        Self {
            components: Vec::new(),
            directives: Vec::new(),
            pipes: Vec::new(),
        }
    }
}

/// A directive/component export.
#[derive(Debug, Clone)]
pub struct DirectiveExport {
    /// Reference to the directive.
    pub directive: String,
    /// Selector.
    pub selector: Option<String>,
    /// Whether standalone.
    pub is_standalone: bool,
}

/// A pipe export.
#[derive(Debug, Clone)]
pub struct PipeExport {
    /// Reference to the pipe.
    pub pipe: String,
    /// Pipe name.
    pub name: String,
    /// Whether standalone.
    pub is_standalone: bool,
}

/// Represents the compilation scope of a component.
#[derive(Debug, Clone)]
pub struct CompilationScope {
    /// Directives available in the scope.
    pub directives: Vec<DirectiveInScope>,
    /// Pipes available in the scope.
    pub pipes: Vec<PipeInScope>,
    /// Whether this scope contains forward references.
    pub contains_forward_decls: bool,
    /// Whether any directive has external styles requiring feature.
    pub has_external_styles: bool,
    /// Whether any directive has poisoned metadata.
    pub is_poisoned: bool,
    /// NgModule (if not standalone).
    pub ng_module: Option<String>,
}

impl CompilationScope {
    pub fn empty() -> Self {
        Self {
            directives: Vec::new(),
            pipes: Vec::new(),
            contains_forward_decls: false,
            has_external_styles: false,
            is_poisoned: false,
            ng_module: None,
        }
    }
}

/// A directive in scope.
#[derive(Debug, Clone)]
pub struct DirectiveInScope {
    /// Directive reference.
    pub directive: String,
    /// Selector.
    pub selector: String,
    /// Whether the directive has inputs.
    pub has_inputs: bool,
    /// Whether the directive has outputs.
    pub has_outputs: bool,
    /// Whether this is a component.
    pub is_component: bool,
    /// Whether standalone.
    pub is_standalone: bool,
}

/// A pipe in scope.
#[derive(Debug, Clone)]
pub struct PipeInScope {
    /// Pipe reference.
    pub pipe: String,
    /// Pipe name.
    pub name: String,
    /// Whether standalone.
    pub is_standalone: bool,
}

/// Result of registering a scope.
#[derive(Debug, Clone)]
pub enum RegisterResult<T> {
    Success(T),
    Error(String),
}
