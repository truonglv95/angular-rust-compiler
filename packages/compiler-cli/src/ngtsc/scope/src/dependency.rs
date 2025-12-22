// Dependency Scope Reader
//
// Reads dependency scope information from external libraries.

use std::collections::HashMap;
use super::api::ExportScope;

/// Reader for .d.ts based scope information.
pub struct DependencyScopeReader {
    /// Cache of export scopes from external libraries.
    export_scope_cache: HashMap<String, ExportScope>,
}

impl DependencyScopeReader {
    pub fn new() -> Self {
        Self {
            export_scope_cache: HashMap::new(),
        }
    }
    
    /// Get the export scope of an external NgModule.
    pub fn get_export_scope(&self, module_ref: &str) -> Option<&ExportScope> {
        self.export_scope_cache.get(module_ref)
    }
    
    /// Register export scope for an external module.
    pub fn register_export_scope(
        &mut self,
        module_ref: impl Into<String>,
        scope: ExportScope,
    ) {
        self.export_scope_cache.insert(module_ref.into(), scope);
    }
    
    /// Check if a module has been processed.
    pub fn has_scope(&self, module_ref: &str) -> bool {
        self.export_scope_cache.contains_key(module_ref)
    }
}

impl Default for DependencyScopeReader {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata for an external directive.
#[derive(Debug, Clone)]
pub struct ExternalDirectiveMetadata {
    /// Reference.
    pub reference: String,
    /// Selector.
    pub selector: Option<String>,
    /// Whether a component.
    pub is_component: bool,
    /// Inputs.
    pub inputs: Vec<(String, String)>,
    /// Outputs.
    pub outputs: Vec<(String, String)>,
    /// Export as names.
    pub export_as: Option<Vec<String>>,
}

/// Metadata for an external pipe.
#[derive(Debug, Clone)]
pub struct ExternalPipeMetadata {
    /// Reference.
    pub reference: String,
    /// Pipe name.
    pub name: String,
}
