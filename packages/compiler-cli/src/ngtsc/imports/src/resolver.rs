// Module Resolver - Resolves modules for lazy-loaded routes
//
// Used by RouterEntryPointManager and NgModuleRouteAnalyzer for resolving
// module source-files in lazy-loaded routes.

use std::path::PathBuf;

/// Used for resolving module source-files references in lazy-loaded routes.
#[derive(Debug)]
pub struct ModuleResolver {
    /// Base path for resolving modules.
    base_path: PathBuf,
}

impl ModuleResolver {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }
    
    /// Resolve a module by name relative to a containing file.
    ///
    /// # Arguments
    /// * `module_name` - The module specifier to resolve
    /// * `containing_file` - The file from which the module is being referenced
    pub fn resolve_module(&self, module_name: &str, containing_file: &str) -> Option<PathBuf> {
        // Handle relative module paths
        if module_name.starts_with("./") || module_name.starts_with("../") {
            let containing_dir = std::path::Path::new(containing_file)
                .parent()
                .unwrap_or(std::path::Path::new(""));
            
            let resolved = containing_dir.join(module_name);
            
            // Try with .ts extension
            let with_ts = resolved.with_extension("ts");
            if with_ts.exists() {
                return Some(with_ts);
            }
            
            // Try with /index.ts
            let with_index = resolved.join("index.ts");
            if with_index.exists() {
                return Some(with_index);
            }
            
            return Some(resolved);
        }
        
        // Handle absolute/package paths
        // In a real implementation, this would use TypeScript's module resolution
        let resolved = self.base_path.join("node_modules").join(module_name);
        if resolved.exists() {
            return Some(resolved);
        }
        
        None
    }
}
