// Local Compilation Extra Imports Tracker
//
// Tracks extra imports to be added to generated files in local compilation mode.
// This is needed for bundling mechanisms that require dev files to have imports
// resembling those generated for full compilation mode.

use std::collections::{HashMap, HashSet};

/// Tracks extra imports to be added to generated files in local compilation mode.
///
/// In full compilation mode, Angular generates extra imports for statically
/// analyzed component dependencies. This tracker provides similar functionality
/// for local compilation mode.
#[derive(Debug, Default)]
pub struct LocalCompilationExtraImportsTracker {
    /// Map from file path to set of module names for local imports.
    local_imports_map: HashMap<String, HashSet<String>>,
    
    /// Set of module names to be added as global imports to all files.
    global_imports_set: HashSet<String>,
    
    /// Names of files marked for extra import generation.
    marked_files_set: HashSet<String>,
}

impl LocalCompilationExtraImportsTracker {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Marks a source file for extra imports generation.
    ///
    /// Extra imports are only generated for files marked through this method.
    pub fn mark_file_for_extra_import_generation(&mut self, file_name: &str) {
        self.marked_files_set.insert(file_name.to_string());
    }
    
    /// Adds an extra import to be added to a specific source file.
    ///
    /// # Arguments
    /// * `file_name` - The source file path
    /// * `module_name` - The module specifier to import
    pub fn add_import_for_file(&mut self, file_name: &str, module_name: &str) {
        self.local_imports_map
            .entry(file_name.to_string())
            .or_insert_with(HashSet::new)
            .insert(module_name.to_string());
    }
    
    /// Adds a global import that will be added to all marked files.
    ///
    /// # Arguments
    /// * `module_name` - The module specifier to import globally
    pub fn add_global_import(&mut self, module_name: &str) {
        self.global_imports_set.insert(module_name.to_string());
    }
    
    /// Returns the list of all module names that a file should include as extra imports.
    ///
    /// Returns empty if the file is not marked for extra import generation.
    pub fn get_imports_for_file(&self, file_name: &str) -> Vec<String> {
        if !self.marked_files_set.contains(file_name) {
            return Vec::new();
        }
        
        let mut imports: Vec<String> = self.global_imports_set.iter().cloned().collect();
        
        if let Some(local_imports) = self.local_imports_map.get(file_name) {
            imports.extend(local_imports.iter().cloned());
        }
        
        imports
    }
    
    /// Check if a file is marked for extra import generation.
    pub fn is_file_marked(&self, file_name: &str) -> bool {
        self.marked_files_set.contains(file_name)
    }
    
    /// Get all global imports.
    pub fn get_global_imports(&self) -> &HashSet<String> {
        &self.global_imports_set
    }
}

/// Remove quotations from a string (helper for parsing import specifiers).
pub fn remove_quotations(s: &str) -> String {
    if s.len() >= 2 && (s.starts_with('"') || s.starts_with('\'')) {
        s[1..s.len() - 1].trim().to_string()
    } else {
        s.to_string()
    }
}
