// TypeCheck Scope
//
// Scope information for type-checking.

use std::collections::HashMap;
use super::api::{DirectiveInScope, PipeInScope};

/// Type-check scope data for a component.
#[derive(Debug, Clone)]
pub struct TypeCheckScope {
    /// Directives available for type-checking.
    pub directives: Vec<TypeCheckDirective>,
    /// Pipes available for type-checking.
    pub pipes: Vec<TypeCheckPipe>,
    /// Schemas allowed.
    pub schemas: Vec<String>,
    /// Whether the scope is poisoned.
    pub is_poisoned: bool,
}

impl TypeCheckScope {
    pub fn empty() -> Self {
        Self {
            directives: Vec::new(),
            pipes: Vec::new(),
            schemas: Vec::new(),
            is_poisoned: false,
        }
    }
    
    pub fn from_compilation_scope(
        directives: &[DirectiveInScope],
        pipes: &[PipeInScope],
    ) -> Self {
        Self {
            directives: directives.iter().map(|d| TypeCheckDirective {
                ref_name: d.directive.clone(),
                selector: d.selector.clone(),
                is_component: d.is_component,
                inputs: Vec::new(),
                outputs: Vec::new(),
                host_directives: Vec::new(),
            }).collect(),
            pipes: pipes.iter().map(|p| TypeCheckPipe {
                ref_name: p.pipe.clone(),
                name: p.name.clone(),
            }).collect(),
            schemas: Vec::new(),
            is_poisoned: false,
        }
    }
}

/// Type-check metadata for a directive.
#[derive(Debug, Clone)]
pub struct TypeCheckDirective {
    /// Reference name.
    pub ref_name: String,
    /// Selector.
    pub selector: String,
    /// Whether a component.
    pub is_component: bool,
    /// Inputs.
    pub inputs: Vec<TypeCheckInput>,
    /// Outputs.
    pub outputs: Vec<TypeCheckOutput>,
    /// Host directives.
    pub host_directives: Vec<String>,
}

/// Type-check metadata for a pipe.
#[derive(Debug, Clone)]
pub struct TypeCheckPipe {
    /// Reference name.
    pub ref_name: String,
    /// Pipe name.
    pub name: String,
}

/// Type-check input.
#[derive(Debug, Clone)]
pub struct TypeCheckInput {
    /// Class property name.
    pub class_property: String,
    /// Binding property name.
    pub binding_property: String,
    /// Whether required.
    pub required: bool,
    /// Type string (for error messages).
    pub type_str: String,
    /// Whether this is a signal input.
    pub is_signal: bool,
}

/// Type-check output.
#[derive(Debug, Clone)]
pub struct TypeCheckOutput {
    /// Class property name.
    pub class_property: String,
    /// Binding property name.
    pub binding_property: String,
}

/// Registry that provides type-check scope to the type-checker.
pub struct TypeCheckScopeRegistry {
    /// Cache of type-check scopes.
    scope_cache: HashMap<String, TypeCheckScope>,
}

impl TypeCheckScopeRegistry {
    pub fn new() -> Self {
        Self {
            scope_cache: HashMap::new(),
        }
    }
    
    /// Get type-check scope for a component.
    pub fn get_type_check_scope(&self, component_ref: &str) -> Option<&TypeCheckScope> {
        self.scope_cache.get(component_ref)
    }
    
    /// Register type-check scope for a component.
    pub fn register_type_check_scope(
        &mut self,
        component_ref: impl Into<String>,
        scope: TypeCheckScope,
    ) {
        self.scope_cache.insert(component_ref.into(), scope);
    }
}

impl Default for TypeCheckScopeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
