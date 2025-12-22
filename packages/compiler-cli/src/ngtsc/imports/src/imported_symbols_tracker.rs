// Imported Symbols Tracker
//
// Tracks which symbols are imported in specific files and under what names.
// Allows for efficient querying for references to those symbols without having
// to consult the type checker early in the process.

use std::collections::{HashMap, HashSet};

/// A map of imported symbols to local names under which the symbols are available.
type LocalNamesMap = HashMap<String, HashSet<String>>;

/// Mapping between modules and the named imports consumed by them in a file.
type NamedImportsMap = HashMap<String, LocalNamesMap>;

/// Tracks which symbols are imported in specific files and under what names.
///
/// Note that the tracker doesn't account for variable shadowing so a final
/// verification with the type checker may be necessary, depending on the context.
#[derive(Debug, Clone, Default)]
pub struct ImportedSymbolsTracker {
    /// Map of file path to its named imports.
    file_to_named_imports: HashMap<String, NamedImportsMap>,
    /// Map of file path to its namespace imports.
    file_to_namespace_imports: HashMap<String, LocalNamesMap>,
}

impl ImportedSymbolsTracker {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Register a named import for a file.
    ///
    /// # Arguments
    /// * `file` - The source file path
    /// * `module_name` - The module being imported from (e.g., "@angular/core")
    /// * `exported_name` - The name of the exported symbol
    /// * `local_name` - The local name it's imported as
    pub fn register_named_import(
        &mut self,
        file: &str,
        module_name: &str,
        exported_name: &str,
        local_name: &str,
    ) {
        let file_imports = self.file_to_named_imports
            .entry(file.to_string())
            .or_insert_with(HashMap::new);
        
        let module_imports = file_imports
            .entry(module_name.to_string())
            .or_insert_with(HashMap::new);
        
        let local_names = module_imports
            .entry(exported_name.to_string())
            .or_insert_with(HashSet::new);
        
        local_names.insert(local_name.to_string());
    }
    
    /// Register a namespace import for a file.
    ///
    /// # Arguments
    /// * `file` - The source file path  
    /// * `module_name` - The module being imported from
    /// * `local_name` - The local namespace name
    pub fn register_namespace_import(
        &mut self,
        file: &str,
        module_name: &str,
        local_name: &str,
    ) {
        let namespaces = self.file_to_namespace_imports
            .entry(file.to_string())
            .or_insert_with(HashMap::new);
        
        let module_namespaces = namespaces
            .entry(module_name.to_string())
            .or_insert_with(HashSet::new);
        
        module_namespaces.insert(local_name.to_string());
    }
    
    /// Checks if an identifier is a potential reference to a specific named import.
    ///
    /// # Arguments
    /// * `file` - The source file containing the identifier
    /// * `identifier` - The identifier text
    /// * `exported_name` - Name of the exported symbol
    /// * `module_name` - Module from which the symbol should be imported
    pub fn is_potential_reference_to_named_import(
        &self,
        file: &str,
        identifier: &str,
        exported_name: &str,
        module_name: &str,
    ) -> bool {
        if let Some(file_imports) = self.file_to_named_imports.get(file) {
            if let Some(module_imports) = file_imports.get(module_name) {
                if let Some(local_names) = module_imports.get(exported_name) {
                    return local_names.contains(identifier);
                }
            }
        }
        false
    }
    
    /// Checks if an identifier is a potential reference to a namespace import.
    ///
    /// # Arguments
    /// * `file` - The source file containing the identifier
    /// * `identifier` - The identifier text
    /// * `module_name` - Module from which the namespace is imported
    pub fn is_potential_reference_to_namespace_import(
        &self,
        file: &str,
        identifier: &str,
        module_name: &str,
    ) -> bool {
        if let Some(namespaces) = self.file_to_namespace_imports.get(file) {
            if let Some(module_namespaces) = namespaces.get(module_name) {
                return module_namespaces.contains(identifier);
            }
        }
        false
    }
    
    /// Checks if a file has a named import of a certain symbol.
    ///
    /// # Arguments
    /// * `file` - File to be checked
    /// * `exported_name` - Name of the exported symbol
    /// * `module_name` - Module that exports the symbol
    pub fn has_named_import(&self, file: &str, exported_name: &str, module_name: &str) -> bool {
        if let Some(file_imports) = self.file_to_named_imports.get(file) {
            if let Some(module_imports) = file_imports.get(module_name) {
                return module_imports.contains_key(exported_name);
            }
        }
        false
    }
    
    /// Checks if a file has namespace imports of a module.
    ///
    /// # Arguments
    /// * `file` - File to be checked
    /// * `module_name` - Module whose namespace import is being searched for
    pub fn has_namespace_import(&self, file: &str, module_name: &str) -> bool {
        if let Some(namespaces) = self.file_to_namespace_imports.get(file) {
            return namespaces.contains_key(module_name);
        }
        false
    }
    
    /// Check if a symbol is imported from a specific module (simplified API).
    pub fn is_imported(&self, symbol: &str, from: &str) -> bool {
        for file_imports in self.file_to_named_imports.values() {
            if let Some(module_imports) = file_imports.get(from) {
                if module_imports.contains_key(symbol) {
                    return true;
                }
            }
        }
        false
    }
}
