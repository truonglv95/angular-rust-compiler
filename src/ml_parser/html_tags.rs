//! HTML Tag Definitions
//!
//! Corresponds to packages/compiler/src/ml_parser/html_tags.ts (194 lines)

use super::tags::{get_ns_prefix, TagContentType, TagDefinition};
use crate::schema::dom_element_schema_registry::DomElementSchemaRegistry;
use crate::schema::element_schema_registry::ElementSchemaRegistry;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// HTML tag definition with specific parsing rules
#[derive(Debug, Clone)]
pub struct HtmlTagDefinition {
    pub closed_by_children: HashMap<String, bool>,
    pub content_type: ContentTypeConfig,
    pub closed_by_parent: bool,
    pub implicit_namespace_prefix: Option<String>,
    pub is_void: bool,
    pub ignore_first_lf: bool,
    pub can_self_close: bool,
    pub prevent_namespace_inheritance: bool,
}

/// Content type configuration (can be simple or namespace-specific)
#[derive(Debug, Clone)]
pub enum ContentTypeConfig {
    Simple(TagContentType),
    WithNamespaces {
        default: TagContentType,
        namespaces: HashMap<String, TagContentType>,
    },
}

impl HtmlTagDefinition {
    pub fn new() -> Self {
        HtmlTagDefinition {
            closed_by_children: HashMap::new(),
            content_type: ContentTypeConfig::Simple(TagContentType::ParsableData),
            closed_by_parent: false,
            implicit_namespace_prefix: None,
            is_void: false,
            ignore_first_lf: false,
            can_self_close: true,
            prevent_namespace_inheritance: false,
        }
    }

    pub fn with_void(mut self, is_void: bool) -> Self {
        self.is_void = is_void;
        self.closed_by_parent = self.closed_by_parent || is_void;
        self.can_self_close = is_void;
        self
    }

    pub fn with_closed_by_children(mut self, children: Vec<&str>) -> Self {
        for child in children {
            self.closed_by_children.insert(child.to_lowercase(), true);
        }
        self
    }

    pub fn with_closed_by_parent(mut self, closed_by_parent: bool) -> Self {
        self.closed_by_parent = closed_by_parent;
        self
    }

    pub fn with_implicit_namespace(mut self, prefix: &str) -> Self {
        self.implicit_namespace_prefix = Some(prefix.to_string());
        self
    }

    pub fn with_content_type(mut self, content_type: TagContentType) -> Self {
        self.content_type = ContentTypeConfig::Simple(content_type);
        self
    }

    pub fn with_content_type_namespaced(
        mut self,
        default: TagContentType,
        namespaces: HashMap<String, TagContentType>,
    ) -> Self {
        self.content_type = ContentTypeConfig::WithNamespaces { default, namespaces };
        self
    }

    pub fn with_ignore_first_lf(mut self, ignore: bool) -> Self {
        self.ignore_first_lf = ignore;
        self
    }

    pub fn with_prevent_namespace_inheritance(mut self, prevent: bool) -> Self {
        self.prevent_namespace_inheritance = prevent;
        self
    }

    pub fn with_can_self_close(mut self, can_self_close: bool) -> Self {
        self.can_self_close = can_self_close;
        self
    }
}

impl TagDefinition for HtmlTagDefinition {
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

    fn is_closed_by_child(&self, name: &str) -> bool {
        self.is_void || self.closed_by_children.contains_key(&name.to_lowercase())
    }

    fn get_content_type(&self, prefix: Option<&str>) -> TagContentType {
        match &self.content_type {
            ContentTypeConfig::Simple(ct) => *ct,
            ContentTypeConfig::WithNamespaces { default, namespaces } => {
                prefix
                    .and_then(|p| namespaces.get(p).copied())
                    .unwrap_or(*default)
            }
        }
    }
}

impl Default for HtmlTagDefinition {
    fn default() -> Self {
        Self::new()
    }
}

/// Tag definitions registry
static TAG_DEFINITIONS: Lazy<HashMap<String, HtmlTagDefinition>> = Lazy::new(|| {
    let mut defs = HashMap::new();

    // Void elements (self-closing)
    defs.insert("base".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("meta".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("area".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("embed".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("link".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("img".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("input".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("param".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("hr".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("br".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("source".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("track".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("wbr".to_string(), HtmlTagDefinition::new().with_void(true));
    defs.insert("col".to_string(), HtmlTagDefinition::new().with_void(true));

    // <p> tag - closed by many block elements
    defs.insert(
        "p".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec![
                "address", "article", "aside", "blockquote", "div", "dl", "fieldset",
                "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hgroup",
                "hr", "main", "nav", "ol", "p", "pre", "section", "table", "ul",
            ])
            .with_closed_by_parent(true),
    );

    // Table elements
    defs.insert(
        "thead".to_string(),
        HtmlTagDefinition::new().with_closed_by_children(vec!["tbody", "tfoot"]),
    );
    defs.insert(
        "tbody".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["tbody", "tfoot"])
            .with_closed_by_parent(true),
    );
    defs.insert(
        "tfoot".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["tbody"])
            .with_closed_by_parent(true),
    );
    defs.insert(
        "tr".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["tr"])
            .with_closed_by_parent(true),
    );
    defs.insert(
        "td".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["td", "th"])
            .with_closed_by_parent(true),
    );
    defs.insert(
        "th".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["td", "th"])
            .with_closed_by_parent(true),
    );

    // SVG namespace
    defs.insert(
        "svg".to_string(),
        HtmlTagDefinition::new().with_implicit_namespace("svg"),
    );
    defs.insert(
        "foreignobject".to_string(),
        HtmlTagDefinition::new()
            .with_implicit_namespace("svg")
            .with_prevent_namespace_inheritance(true),
    );

    // Math namespace
    defs.insert(
        "math".to_string(),
        HtmlTagDefinition::new().with_implicit_namespace("math"),
    );

    // List elements
    defs.insert(
        "li".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["li"])
            .with_closed_by_parent(true),
    );
    defs.insert(
        "dt".to_string(),
        HtmlTagDefinition::new().with_closed_by_children(vec!["dt", "dd"]),
    );
    defs.insert(
        "dd".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["dt", "dd"])
            .with_closed_by_parent(true),
    );

    // Ruby annotation elements
    defs.insert(
        "rb".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["rb", "rt", "rtc", "rp"])
            .with_closed_by_parent(true),
    );
    defs.insert(
        "rt".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["rb", "rt", "rtc", "rp"])
            .with_closed_by_parent(true),
    );
    defs.insert(
        "rtc".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["rb", "rtc", "rp"])
            .with_closed_by_parent(true),
    );
    defs.insert(
        "rp".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["rb", "rt", "rtc", "rp"])
            .with_closed_by_parent(true),
    );

    // Select elements
    defs.insert(
        "optgroup".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["optgroup"])
            .with_closed_by_parent(true),
    );
    defs.insert(
        "option".to_string(),
        HtmlTagDefinition::new()
            .with_closed_by_children(vec!["option", "optgroup"])
            .with_closed_by_parent(true),
    );

    // Elements that ignore first LF
    defs.insert(
        "pre".to_string(),
        HtmlTagDefinition::new().with_ignore_first_lf(true),
    );
    defs.insert(
        "listing".to_string(),
        HtmlTagDefinition::new().with_ignore_first_lf(true),
    );

    // Raw text content
    defs.insert(
        "style".to_string(),
        HtmlTagDefinition::new().with_content_type(TagContentType::RawText),
    );
    defs.insert(
        "script".to_string(),
        HtmlTagDefinition::new().with_content_type(TagContentType::RawText),
    );

    // Title - different content type for SVG vs HTML
    let mut title_namespaces = HashMap::new();
    title_namespaces.insert("svg".to_string(), TagContentType::ParsableData);
    defs.insert(
        "title".to_string(),
        HtmlTagDefinition::new().with_content_type_namespaced(
            TagContentType::EscapableRawText,
            title_namespaces,
        ),
    );

    // Textarea
    defs.insert(
        "textarea".to_string(),
        HtmlTagDefinition::new()
            .with_content_type(TagContentType::EscapableRawText)
            .with_ignore_first_lf(true),
    );

    // Add all known HTML elements from schema
    let registry = DomElementSchemaRegistry::new();
    for tag_name in registry.all_known_element_names() {
        let tag_lower = tag_name.to_lowercase();
        if !defs.contains_key(&tag_lower) && get_ns_prefix(Some(&tag_name)).is_none() {
            defs.insert(tag_lower, HtmlTagDefinition::new().with_can_self_close(false));
        }
    }

    defs
});

/// Default tag definition
static DEFAULT_TAG_DEFINITION: Lazy<HtmlTagDefinition> = Lazy::new(|| {
    HtmlTagDefinition::new().with_can_self_close(true)
});

/// Get HTML tag definition for a given tag name
pub fn get_html_tag_definition(tag_name: &str) -> &'static HtmlTagDefinition {
    // We have to make both a case-sensitive and a case-insensitive lookup, because
    // HTML tag names are case insensitive, whereas some SVG tags are case sensitive.
    TAG_DEFINITIONS
        .get(tag_name)
        .or_else(|| TAG_DEFINITIONS.get(&tag_name.to_lowercase()))
        .unwrap_or(&DEFAULT_TAG_DEFINITION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_void_elements() {
        assert!(get_html_tag_definition("br").is_void);
        assert!(get_html_tag_definition("img").is_void);
        assert!(get_html_tag_definition("hr").is_void);
        assert!(get_html_tag_definition("input").is_void);
    }

    #[test]
    fn test_non_void_elements() {
        assert!(!get_html_tag_definition("div").is_void);
        assert!(!get_html_tag_definition("p").is_void);
    }

    #[test]
    fn test_p_closed_by_children() {
        let p_def = get_html_tag_definition("p");
        assert!(p_def.is_closed_by_child("div"));
        assert!(p_def.is_closed_by_child("h1"));
        assert!(p_def.is_closed_by_child("p"));
        assert!(!p_def.is_closed_by_child("span"));
    }

    #[test]
    fn test_table_elements() {
        let tr_def = get_html_tag_definition("tr");
        assert!(tr_def.is_closed_by_child("tr"));
        assert!(tr_def.closed_by_parent);
    }

    #[test]
    fn test_raw_text_content() {
        let script_def = get_html_tag_definition("script");
        assert_eq!(script_def.get_content_type(None), TagContentType::RawText);

        let style_def = get_html_tag_definition("style");
        assert_eq!(style_def.get_content_type(None), TagContentType::RawText);
    }

    #[test]
    fn test_ignore_first_lf() {
        assert!(get_html_tag_definition("pre").ignore_first_lf);
        assert!(get_html_tag_definition("textarea").ignore_first_lf);
    }

    #[test]
    fn test_namespace_prefix() {
        let svg_def = get_html_tag_definition("svg");
        assert_eq!(svg_def.implicit_namespace_prefix(), Some("svg"));

        let math_def = get_html_tag_definition("math");
        assert_eq!(math_def.implicit_namespace_prefix(), Some("math"));
    }

    #[test]
    fn test_case_insensitive_lookup() {
        let div_upper = get_html_tag_definition("DIV");
        let div_lower = get_html_tag_definition("div");
        assert!(!div_upper.is_void);
        assert!(!div_lower.is_void);
    }

    #[test]
    fn test_title_content_type_by_namespace() {
        let title_def = get_html_tag_definition("title");

        // Default (HTML) should be ESCAPABLE_RAW_TEXT
        assert_eq!(title_def.get_content_type(None), TagContentType::EscapableRawText);

        // SVG namespace should be PARSABLE_DATA
        assert_eq!(title_def.get_content_type(Some("svg")), TagContentType::ParsableData);
    }
}

