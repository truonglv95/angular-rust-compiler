// Class Extractor
//
// Extracts documentation from class declarations.

use super::entities::*;

/// Extracts class documentation.
pub struct ClassExtractor;

impl ClassExtractor {
    /// Extract class entry from AST node.
    pub fn extract(name: &str, _source_file: &str, _line: usize) -> ClassEntry {
        ClassEntry {
            base: DocEntry::new(name, EntryType::Class),
            members: Vec::new(),
            constructor_params: Vec::new(),
            extends: None,
            implements: Vec::new(),
            type_params: Vec::new(),
        }
    }

    /// Extract members from class.
    pub fn extract_members(_members: &[()]) -> Vec<MemberEntry> {
        Vec::new()
    }

    /// Extract constructor parameters.
    pub fn extract_constructor_params(_params: &[()]) -> Vec<ParameterEntry> {
        Vec::new()
    }
}
