// Aliasing - Alternative exports/imports for directives/pipes
//
// Provides aliasing support for when directives or pipes need to be
// exported/imported through alternative paths to avoid issues with
// transitive dependencies.

/// Escape characters that aren't alphanumeric, '/' or '_'.
fn escape_for_alias(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '/' || c == '_' {
                c.to_string()
            } else {
                format!("${:02x}", c as u32)
            }
        })
        .collect()
}

/// Alias strategy to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasStrategy {
    /// Use unified modules aliasing (for monorepo setups).
    UnifiedModules,
    /// Use private export aliasing.
    PrivateExport,
}

/// A host for the aliasing system.
///
/// Allows for alternative exports/imports of directives/pipes when the normal
/// import path would cause issues with transitive dependencies.
pub trait AliasingHost {
    /// Determine a name by which a declaration should be re-exported.
    ///
    /// Returns `None` if no alias export should be generated.
    fn maybe_alias_symbol_as(
        &self,
        decl_name: &str,
        decl_file: &str,
        context_file: &str,
        ng_module_name: &str,
        is_reexport: bool,
    ) -> Option<String>;
    
    /// Determine an expression by which a declaration should be imported using an alias.
    fn get_alias_in(
        &self,
        decl_name: &str,
        decl_file: &str,
        via_file: &str,
        is_reexport: bool,
    ) -> Option<String>;
}

/// Aliasing host for unified modules (monorepo setups with module names).
pub struct UnifiedModulesAliasingHost {
    /// Function to get module name for a file.
    module_name_getter: Box<dyn Fn(&str) -> Option<String> + Send + Sync>,
}

impl UnifiedModulesAliasingHost {
    pub fn new<F>(module_name_getter: F) -> Self
    where
        F: Fn(&str) -> Option<String> + Send + Sync + 'static,
    {
        Self {
            module_name_getter: Box::new(module_name_getter),
        }
    }
    
    /// Generate an alias name for a declaration.
    fn alias_name(&self, decl_file: &str, context_file: &str) -> String {
        let full_path = decl_file;
        let context_module = (self.module_name_getter)(context_file)
            .unwrap_or_else(|| context_file.to_string());
        
        format!("ɵng${}$${}",
            escape_for_alias(&context_module),
            escape_for_alias(full_path)
        )
    }
}

impl AliasingHost for UnifiedModulesAliasingHost {
    fn maybe_alias_symbol_as(
        &self,
        _decl_name: &str,
        decl_file: &str,
        context_file: &str,
        _ng_module_name: &str,
        is_reexport: bool,
    ) -> Option<String> {
        // Only create aliases for declarations not in the same file
        if decl_file == context_file {
            return None;
        }
        
        // Don't alias re-exports from the same file as the original
        if is_reexport {
            return None;
        }
        
        Some(self.alias_name(decl_file, context_file))
    }
    
    fn get_alias_in(
        &self,
        _decl_name: &str,
        decl_file: &str,
        via_file: &str,
        _is_reexport: bool,
    ) -> Option<String> {
        if decl_file == via_file {
            return None;
        }
        
        let alias_name = self.alias_name(decl_file, via_file);
        let via_module = (self.module_name_getter)(via_file)?;
        
        Some(format!("{}#{}", via_module, alias_name))
    }
}

/// Aliasing host for private exports.
///
/// Exports directives from any file containing an NgModule under a private symbol name.
#[derive(Debug, Default)]
pub struct PrivateExportAliasingHost {
    /// Counter for generating unique private names.
    counter: std::sync::atomic::AtomicU32,
}

impl PrivateExportAliasingHost {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Generate a private alias name for a symbol.
    fn generate_private_name(&self, symbol_name: &str, ng_module_name: &str) -> String {
        let id = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("ɵngExportɵ{}ɵ{}${}", ng_module_name, symbol_name, id)
    }
}

impl AliasingHost for PrivateExportAliasingHost {
    fn maybe_alias_symbol_as(
        &self,
        decl_name: &str,
        decl_file: &str,
        context_file: &str,
        ng_module_name: &str,
        _is_reexport: bool,
    ) -> Option<String> {
        // Only alias if declaration is from a different file
        if decl_file == context_file {
            return None;
        }
        
        Some(self.generate_private_name(decl_name, ng_module_name))
    }
    
    fn get_alias_in(
        &self,
        _decl_name: &str,
        _decl_file: &str,
        _via_file: &str,
        _is_reexport: bool,
    ) -> Option<String> {
        // PrivateExportAliasingHost doesn't direct the compiler to consume aliases
        // They're consumed indirectly through AbsoluteModuleStrategy
        None
    }
}
