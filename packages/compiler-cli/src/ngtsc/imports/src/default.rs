// Default Import Tracker
//
// Tracks default imports that need to be preserved during transformation.
// TypeScript has trouble generating default imports inside transformers for some
// module formats. This tracker ensures default imports are not elided.

use std::collections::{HashMap, HashSet};

/// Tracks default imports that need to be preserved during transformation.
///
/// TypeScript will elide imports that are only used in type position. If Angular
/// reuses an imported symbol in a value position (e.g., inject(Foo)), we need to
/// ensure the import is preserved.
#[derive(Debug, Clone, Default)]
pub struct DefaultImportTracker {
    /// Map from source file path to set of import clause identifiers used in that file.
    source_file_to_used_imports: HashMap<String, HashSet<String>>,
}

impl DefaultImportTracker {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Record that a default import has been used and should be preserved.
    ///
    /// # Arguments
    /// * `source_file` - The file path where the import is used
    /// * `import_clause` - The identifier of the import clause
    pub fn record_used_import(&mut self, source_file: &str, import_clause: &str) {
        self.source_file_to_used_imports
            .entry(source_file.to_string())
            .or_insert_with(HashSet::new)
            .insert(import_clause.to_string());
    }
    
    /// Check if a default import in a file should be preserved.
    ///
    /// # Arguments
    /// * `source_file` - The file path to check
    /// * `import_clause` - The import clause identifier to check
    pub fn should_preserve_import(&self, source_file: &str, import_clause: &str) -> bool {
        self.source_file_to_used_imports
            .get(source_file)
            .map(|imports| imports.contains(import_clause))
            .unwrap_or(false)
    }
    
    /// Get all preserved imports for a given source file.
    pub fn get_preserved_imports(&self, source_file: &str) -> Option<&HashSet<String>> {
        self.source_file_to_used_imports.get(source_file)
    }
    
    /// Check if there are any used imports for a file.
    pub fn has_used_imports(&self, source_file: &str) -> bool {
        self.source_file_to_used_imports
            .get(source_file)
            .map(|imports| !imports.is_empty())
            .unwrap_or(false)
    }
}

/// Attach a default import declaration to an expression.
/// Used to indicate the dependency of an expression on a default import.
pub fn attach_default_import_declaration(
    _expr: &mut dyn std::any::Any,
    _import_decl: &str,
) {
    // In Rust, this would typically be handled via a wrapper type or metadata
    // rather than symbol properties like in TypeScript
}

/// Get the default import declaration attached to an expression.
pub fn get_default_import_declaration(_expr: &dyn std::any::Any) -> Option<String> {
    // In Rust, this would retrieve metadata from a wrapper type
    None
}
