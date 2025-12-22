// Standalone Component Scope
//
// Responsible for computing compilation scope for standalone components.

use std::collections::{HashMap, HashSet};
use super::api::{CompilationScope, DirectiveInScope, PipeInScope};

/// Registry for standalone component scopes.
pub struct StandaloneComponentScopeReader {
    /// Cache of standalone component scopes.
    scope_cache: HashMap<String, CompilationScope>,
    /// Components with errors.
    poisoned_components: HashSet<String>,
}

impl StandaloneComponentScopeReader {
    pub fn new() -> Self {
        Self {
            scope_cache: HashMap::new(),
            poisoned_components: HashSet::new(),
        }
    }
    
    /// Get the scope for a standalone component.
    pub fn get_scope_for_component(&mut self, component_ref: &str) -> Option<&CompilationScope> {
        if !self.scope_cache.contains_key(component_ref) {
            self.compute_scope_for_component(component_ref);
        }
        self.scope_cache.get(component_ref)
    }
    
    /// Register a standalone component's imports.
    pub fn register_standalone_component(
        &mut self,
        component_ref: impl Into<String>,
        imports: Vec<StandaloneImport>,
    ) {
        let component = component_ref.into();
        let mut scope = CompilationScope::empty();
        
        // Process each import
        for import in imports {
            match import {
                StandaloneImport::Directive { name, selector, is_component } => {
                    scope.directives.push(DirectiveInScope {
                        directive: name,
                        selector,
                        has_inputs: false,
                        has_outputs: false,
                        is_component,
                        is_standalone: true,
                    });
                }
                StandaloneImport::Pipe { name, pipe_name } => {
                    scope.pipes.push(PipeInScope {
                        pipe: name,
                        name: pipe_name,
                        is_standalone: true,
                    });
                }
                StandaloneImport::Module { name: _ } => {
                    // Would resolve module exports
                }
            }
        }
        
        self.scope_cache.insert(component, scope);
    }
    
    /// Check if a component has scope errors.
    pub fn is_poisoned(&self, component_ref: &str) -> bool {
        self.poisoned_components.contains(component_ref)
    }
    
    fn compute_scope_for_component(&mut self, component_ref: &str) {
        // If not pre-registered, create empty scope
        if !self.scope_cache.contains_key(component_ref) {
            self.scope_cache.insert(
                component_ref.to_string(),
                CompilationScope::empty(),
            );
        }
    }
    
    /// Get remote scoping requirements for a component.
    pub fn get_remote_scope(&self, _component_ref: &str) -> Option<RemoteScope> {
        // Standalone components don't use remote scoping
        None
    }
}

impl Default for StandaloneComponentScopeReader {
    fn default() -> Self {
        Self::new()
    }
}

/// Import for a standalone component.
#[derive(Debug, Clone)]
pub enum StandaloneImport {
    /// A directive/component import.
    Directive {
        name: String,
        selector: String,
        is_component: bool,
    },
    /// A pipe import.
    Pipe {
        name: String,
        pipe_name: String,
    },
    /// An NgModule import.
    Module {
        name: String,
    },
}

/// Remote scope information.
#[derive(Debug, Clone)]
pub struct RemoteScope {
    pub used_directives: Vec<String>,
    pub used_pipes: Vec<String>,
}
