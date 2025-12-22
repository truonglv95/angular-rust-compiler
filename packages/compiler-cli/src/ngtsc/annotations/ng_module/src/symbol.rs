// NgModule Symbol
//
// Symbol representation for NgModule semantic graph tracking.

/// NgModule symbol for incremental compilation tracking.
#[derive(Debug, Clone)]
pub struct NgModuleSymbol {
    /// Class name.
    pub name: String,
    /// Whether this module has providers.
    pub has_providers: bool,
    /// Remotely scoped components registered by this module.
    pub remotely_scoped_components: Vec<RemotelyScopedComponent>,
    /// Standalone imports.
    pub standalone_transitive_imports: Vec<String>,
}

/// A component that was remotely scoped by this NgModule.
#[derive(Debug, Clone)]
pub struct RemotelyScopedComponent {
    /// Component reference.
    pub component: String,
    /// Used directives.
    pub used_directives: Vec<String>,
    /// Used pipes.
    pub used_pipes: Vec<String>,
}

impl NgModuleSymbol {
    pub fn new(name: impl Into<String>, has_providers: bool) -> Self {
        Self {
            name: name.into(),
            has_providers,
            remotely_scoped_components: Vec::new(),
            standalone_transitive_imports: Vec::new(),
        }
    }
    
    /// Check if public API is affected by changes.
    pub fn is_public_api_affected(&self, previous: &NgModuleSymbol) -> bool {
        self.has_providers != previous.has_providers
    }
    
    /// Check if emit is affected by changes.
    pub fn is_emit_affected(&self, previous: &NgModuleSymbol) -> bool {
        // Check remotely scoped components
        if self.remotely_scoped_components.len() != previous.remotely_scoped_components.len() {
            return true;
        }
        
        // Check standalone imports
        if self.standalone_transitive_imports.len() != previous.standalone_transitive_imports.len() {
            return true;
        }
        
        false
    }
    
    /// Check if type check API is affected.
    pub fn is_type_check_api_affected(&self, previous: &NgModuleSymbol) -> bool {
        self.is_public_api_affected(previous)
    }
    
    /// Add a remotely scoped component.
    pub fn add_remotely_scoped_component(
        &mut self,
        component: impl Into<String>,
        used_directives: Vec<String>,
        used_pipes: Vec<String>,
    ) {
        self.remotely_scoped_components.push(RemotelyScopedComponent {
            component: component.into(),
            used_directives,
            used_pipes,
        });
    }
    
    /// Add a transitive import from a standalone component.
    pub fn add_transitive_import_from_standalone_component(&mut self, imported: impl Into<String>) {
        self.standalone_transitive_imports.push(imported.into());
    }
}
