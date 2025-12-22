// Component Scope Reader
//
// Unified scope reader for components.

use super::local::LocalModuleScopeRegistry;
use super::standalone::StandaloneComponentScopeReader;
use super::api::CompilationScope;

/// Reads scope for both standalone and non-standalone components.
pub struct ComponentScopeReader {
    /// Local module scope registry.
    local_registry: LocalModuleScopeRegistry,
    /// Standalone component scope reader.
    standalone_reader: StandaloneComponentScopeReader,
}

impl ComponentScopeReader {
    pub fn new() -> Self {
        Self {
            local_registry: LocalModuleScopeRegistry::new(),
            standalone_reader: StandaloneComponentScopeReader::new(),
        }
    }
    
    /// Get the compilation scope for a component.
    pub fn get_scope_for_component(
        &mut self,
        component_ref: &str,
        is_standalone: bool,
    ) -> Option<&CompilationScope> {
        if is_standalone {
            self.standalone_reader.get_scope_for_component(component_ref)
        } else {
            self.local_registry.get_scope_for_component(component_ref)
        }
    }
    
    /// Get the local module registry for mutation.
    pub fn local_registry_mut(&mut self) -> &mut LocalModuleScopeRegistry {
        &mut self.local_registry
    }
    
    /// Get the standalone reader for mutation.
    pub fn standalone_reader_mut(&mut self) -> &mut StandaloneComponentScopeReader {
        &mut self.standalone_reader
    }
    
    /// Check if a component has scope errors.
    pub fn is_poisoned(&self, component_ref: &str, is_standalone: bool) -> bool {
        if is_standalone {
            self.standalone_reader.is_poisoned(component_ref)
        } else {
            // For module components, would check if declaring module is poisoned
            false
        }
    }
}

impl Default for ComponentScopeReader {
    fn default() -> Self {
        Self::new()
    }
}
