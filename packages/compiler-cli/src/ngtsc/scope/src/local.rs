// Local Module Scope Registry
//
// Responsible for tracking the compilation scope of NgModules.

use std::collections::{HashMap, HashSet};
use super::api::{CompilationScope, DirectiveInScope, ExportScope};

/// Registry for local NgModule compilation scopes.
pub struct LocalModuleScopeRegistry {
    /// Cache of module scopes.
    scope_cache: HashMap<String, CompilationScope>,
    /// Set of modules that have had their scope computed.
    sealed_modules: HashSet<String>,
    /// Set of modules with errors during scope computation.
    poisoned_modules: HashSet<String>,
    /// Declarations by module.
    declarations_by_module: HashMap<String, Vec<String>>,
    /// Imports by module.
    imports_by_module: HashMap<String, Vec<String>>,
    /// Exports by module.
    exports_by_module: HashMap<String, Vec<String>>,
}

impl LocalModuleScopeRegistry {
    pub fn new() -> Self {
        Self {
            scope_cache: HashMap::new(),
            sealed_modules: HashSet::new(),
            poisoned_modules: HashSet::new(),
            declarations_by_module: HashMap::new(),
            imports_by_module: HashMap::new(),
            exports_by_module: HashMap::new(),
        }
    }
    
    /// Register a module's declarations, imports, and exports.
    pub fn register_ng_module_metadata(
        &mut self,
        module_ref: impl Into<String>,
        declarations: Vec<String>,
        imports: Vec<String>,
        exports: Vec<String>,
    ) {
        let module = module_ref.into();
        self.declarations_by_module.insert(module.clone(), declarations);
        self.imports_by_module.insert(module.clone(), imports);
        self.exports_by_module.insert(module, exports);
    }
    
    /// Get the compilation scope for a component in a module.
    pub fn get_scope_for_component(&mut self, component_ref: &str) -> Option<&CompilationScope> {
        // First find the module (collect to avoid borrow conflict)
        let module = self.declarations_by_module
            .iter()
            .find(|(_, declarations)| declarations.contains(&component_ref.to_string()))
            .map(|(m, _)| m.clone());
        
        // Then get scope
        if let Some(module_ref) = module {
            self.get_scope_of_module(&module_ref)
        } else {
            None
        }
    }
    
    /// Get the compilation scope of a module.
    pub fn get_scope_of_module(&mut self, module_ref: &str) -> Option<&CompilationScope> {
        if !self.scope_cache.contains_key(module_ref) {
            self.compute_scope_for_module(module_ref);
        }
        self.scope_cache.get(module_ref)
    }
    
    /// Get export scope of a module.
    pub fn get_export_scope_of_module(&self, module_ref: &str) -> Option<ExportScope> {
        // Return exports based on what was registered
        if let Some(_exports) = self.exports_by_module.get(module_ref) {
            Some(ExportScope::empty())
        } else {
            None
        }
    }
    
    /// Check if a module is poisoned.
    pub fn is_poisoned(&self, module_ref: &str) -> bool {
        self.poisoned_modules.contains(module_ref)
    }
    
    /// Compute the scope for a module.
    fn compute_scope_for_module(&mut self, module_ref: &str) {
        let mut scope = CompilationScope::empty();
        scope.ng_module = Some(module_ref.to_string());
        
        // Add declarations to scope
        if let Some(declarations) = self.declarations_by_module.get(module_ref).cloned() {
            for decl in declarations {
                scope.directives.push(DirectiveInScope {
                    directive: decl.clone(),
                    selector: format!("[{}]", decl.to_lowercase()),
                    has_inputs: false,
                    has_outputs: false,
                    is_component: false,
                    is_standalone: false,
                });
            }
        }
        
        // Process imports (would recursively get export scopes)
        if let Some(_imports) = self.imports_by_module.get(module_ref) {
            // Would add exported directives/pipes from imported modules
        }
        
        self.scope_cache.insert(module_ref.to_string(), scope);
        self.sealed_modules.insert(module_ref.to_string());
    }
    
    /// Register a declaration.
    pub fn register_declaration(&mut self, declaration: impl Into<String>, ng_module: impl Into<String>) {
        let module = ng_module.into();
        let decl = declaration.into();
        self.declarations_by_module
            .entry(module)
            .or_insert_with(Vec::new)
            .push(decl);
    }
    
    /// Get all diagnostics for scope errors.
    pub fn get_diagnostics(&self) -> Vec<String> {
        self.poisoned_modules
            .iter()
            .map(|m| format!("Module {} has scope errors", m))
            .collect()
    }
}

impl Default for LocalModuleScopeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
