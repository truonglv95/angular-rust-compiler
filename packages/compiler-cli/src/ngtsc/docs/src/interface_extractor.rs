// Interface Extractor
//
// Extracts documentation from interface declarations.

use super::entities::*;

/// Extracts interface documentation.
pub struct InterfaceExtractor;

impl InterfaceExtractor {
    /// Extract interface entry.
    pub fn extract(name: &str, members: Vec<MemberEntry>, extends: Vec<String>) -> ClassEntry {
        ClassEntry {
            base: DocEntry::new(name, EntryType::Interface),
            members,
            constructor_params: Vec::new(),
            extends: None,
            implements: extends,
            type_params: Vec::new(),
        }
    }

    /// Extract interface member.
    pub fn extract_member(name: &str, type_annotation: &str, _optional: bool) -> MemberEntry {
        MemberEntry {
            name: name.to_string(),
            member_type: MemberType::Property,
            type_annotation: type_annotation.to_string(),
            description: String::new(),
            inherited: false,
            visibility: Visibility::Public,
        }
    }
}
