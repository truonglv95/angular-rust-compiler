// References - Reference type for tracking AST node references
//
// A Reference is a pointer to a node that was extracted from the program.
// It tracks different identifiers by which the node is exposed, as well as
// potentially a module specifier which might expose the node.

use angular_compiler::output::output_ast::Expression;

/// Information about the module that owns a particular reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwningModule {
    /// The module specifier (e.g., "@angular/core").
    pub specifier: String,
    /// The resolution context (usually the file path where the import was found).
    pub resolution_context: String,
}

impl OwningModule {
    pub fn new(specifier: impl Into<String>, resolution_context: impl Into<String>) -> Self {
        Self {
            specifier: specifier.into(),
            resolution_context: resolution_context.into(),
        }
    }
}

/// A reference to a TypeScript node.
///
/// The Angular compiler uses `Reference`s instead of raw nodes when tracking 
/// classes or generating imports.
#[derive(Debug, Clone)]
pub struct Reference {
    /// The name of the referenced node (for debugging/display).
    pub name: String,
    
    /// The source file path where the node is defined.
    pub source_file: String,
    
    /// The compiler's best guess at an absolute module specifier which owns this Reference.
    pub best_guess_owning_module: Option<OwningModule>,
    
    /// Indicates that the Reference was created synthetically.
    pub synthetic: bool,
    
    /// Whether this reference is an ambient import.
    pub is_ambient: bool,
    
    /// Alias expression for this reference, if any.
    alias: Option<Box<Expression>>,
    
    /// Known identifiers that can be used to refer to this node.
    identifiers: Vec<String>,
}

impl Reference {
    /// Create a new Reference.
    pub fn new(
        name: impl Into<String>,
        source_file: impl Into<String>,
        owning_module: Option<OwningModule>,
    ) -> Self {
        let name = name.into();
        let identifiers = vec![name.clone()];
        
        Self {
            name,
            source_file: source_file.into(),
            best_guess_owning_module: owning_module,
            synthetic: false,
            is_ambient: false,
            alias: None,
            identifiers,
        }
    }
    
    /// Create an ambient Reference (from an ambient import).
    pub fn ambient(name: impl Into<String>, source_file: impl Into<String>) -> Self {
        let name = name.into();
        let identifiers = vec![name.clone()];
        
        Self {
            name,
            source_file: source_file.into(),
            best_guess_owning_module: None,
            synthetic: false,
            is_ambient: true,
            alias: None,
            identifiers,
        }
    }
    
    /// The best guess at which module specifier owns this reference, or None.
    pub fn owned_by_module_guess(&self) -> Option<&str> {
        self.best_guess_owning_module.as_ref().map(|m| m.specifier.as_str())
    }
    
    /// Whether this reference has a potential owning module.
    pub fn has_owning_module_guess(&self) -> bool {
        self.best_guess_owning_module.is_some()
    }
    
    /// Get the debug name for this reference.
    pub fn debug_name(&self) -> &str {
        &self.name
    }
    
    /// Get the alias expression, if any.
    pub fn alias(&self) -> Option<&Expression> {
        self.alias.as_deref()
    }
    
    /// Record an identifier by which it's valid to refer to this node.
    pub fn add_identifier(&mut self, identifier: impl Into<String>) {
        self.identifiers.push(identifier.into());
    }
    
    /// Get an identifier for this reference if it exists in the given source file.
    pub fn get_identity_in(&self, source_file: &str) -> Option<&str> {
        if self.source_file == source_file && !self.identifiers.is_empty() {
            Some(&self.identifiers[0])
        } else {
            None
        }
    }
    
    /// Clone this reference with a new alias.
    pub fn clone_with_alias(&self, alias: Expression) -> Self {
        let mut cloned = self.clone();
        cloned.alias = Some(Box::new(alias));
        cloned
    }
    
    /// Clone this reference without identifiers.
    pub fn clone_with_no_identifiers(&self) -> Self {
        let mut cloned = self.clone();
        cloned.identifiers.clear();
        cloned
    }
}
