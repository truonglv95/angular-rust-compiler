//! XML Tag Definitions
//!
//! Corresponds to packages/compiler/src/ml_parser/xml_tags.ts (36 lines)

use super::tags::{TagContentType, TagDefinition};

/// XML tag definition (all tags treated uniformly)
#[derive(Debug, Clone)]
pub struct XmlTagDefinition {
    pub closed_by_parent: bool,
    pub implicit_namespace_prefix: Option<String>,
    pub is_void: bool,
    pub ignore_first_lf: bool,
    pub can_self_close: bool,
    pub prevent_namespace_inheritance: bool,
}

impl XmlTagDefinition {
    pub fn new() -> Self {
        XmlTagDefinition {
            closed_by_parent: false,
            implicit_namespace_prefix: None,
            is_void: false,
            ignore_first_lf: false,
            can_self_close: true,
            prevent_namespace_inheritance: false,
        }
    }

    pub fn require_extra_parent(&self, _current_parent: &str) -> bool {
        false
    }
}

impl TagDefinition for XmlTagDefinition {
    fn closed_by_parent(&self) -> bool {
        self.closed_by_parent
    }

    fn implicit_namespace_prefix(&self) -> Option<&str> {
        self.implicit_namespace_prefix.as_deref()
    }

    fn is_void(&self) -> bool {
        self.is_void
    }

    fn ignore_first_lf(&self) -> bool {
        self.ignore_first_lf
    }

    fn can_self_close(&self) -> bool {
        self.can_self_close
    }

    fn prevent_namespace_inheritance(&self) -> bool {
        self.prevent_namespace_inheritance
    }

    fn is_closed_by_child(&self, _name: &str) -> bool {
        false
    }

    fn get_content_type(&self, _prefix: Option<&str>) -> TagContentType {
        TagContentType::ParsableData
    }
}

impl Default for XmlTagDefinition {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared instance for all XML tags
static TAG_DEFINITION: once_cell::sync::Lazy<XmlTagDefinition> =
    once_cell::sync::Lazy::new(XmlTagDefinition::new);

/// Get XML tag definition (same for all tags)
pub fn get_xml_tag_definition(_tag_name: &str) -> &'static XmlTagDefinition {
    &TAG_DEFINITION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_tag_definition() {
        let def = get_xml_tag_definition("any");
        assert!(!def.is_void);
        assert!(def.can_self_close);
        assert_eq!(def.get_content_type(None), TagContentType::ParsableData);
    }
}

