//! Template Preparser
//!
//! Corresponds to packages/compiler/src/template_parser/template_preparser.ts
//! Pre-parses elements to identify special Angular elements (ng-content, style, script, etc.)

use crate::ml_parser::ast::Element;
use crate::ml_parser::tags::is_ng_content;

const NG_CONTENT_SELECT_ATTR: &str = "select";
const LINK_ELEMENT: &str = "link";
const LINK_STYLE_REL_ATTR: &str = "rel";
const LINK_STYLE_HREF_ATTR: &str = "href";
const LINK_STYLE_REL_VALUE: &str = "stylesheet";
const STYLE_ELEMENT: &str = "style";
const SCRIPT_ELEMENT: &str = "script";
const NG_NON_BINDABLE_ATTR: &str = "ngNonBindable";
const NG_PROJECT_AS: &str = "ngProjectAs";

/// Type of preparsed element
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparsedElementType {
    NgContent,
    Style,
    Stylesheet,
    Script,
    Other,
}

/// Pre-parsed element information
#[derive(Debug, Clone)]
pub struct PreparsedElement {
    pub element_type: PreparsedElementType,
    pub select_attr: String,
    pub href_attr: Option<String>,
    pub non_bindable: bool,
    pub project_as: String,
}

impl PreparsedElement {
    pub fn new(
        element_type: PreparsedElementType,
        select_attr: String,
        href_attr: Option<String>,
        non_bindable: bool,
        project_as: String,
    ) -> Self {
        PreparsedElement {
            element_type,
            select_attr,
            href_attr,
            non_bindable,
            project_as,
        }
    }
}

/// Pre-parse an element to identify its type and extract special attributes
pub fn preparse_element(ast: &Element) -> PreparsedElement {
    let mut select_attr: Option<String> = None;
    let mut href_attr: Option<String> = None;
    let mut rel_attr: Option<String> = None;
    let mut non_bindable = false;
    let mut project_as = String::new();

    // Iterate through attributes
    for attr in &ast.attrs {
        let lc_attr_name = attr.name.to_lowercase();
        
        if lc_attr_name == NG_CONTENT_SELECT_ATTR {
            select_attr = Some(attr.value.clone());
        } else if lc_attr_name == LINK_STYLE_HREF_ATTR {
            href_attr = Some(attr.value.clone());
        } else if lc_attr_name == LINK_STYLE_REL_ATTR {
            rel_attr = Some(attr.value.clone());
        } else if attr.name == NG_NON_BINDABLE_ATTR {
            non_bindable = true;
        } else if attr.name == NG_PROJECT_AS {
            if !attr.value.is_empty() {
                project_as = attr.value.clone();
            }
        }
    }

    let select_attr = normalize_ng_content_select(select_attr);
    let node_name = ast.name.to_lowercase();
    
    let element_type = if is_ng_content(&node_name) {
        PreparsedElementType::NgContent
    } else if node_name == STYLE_ELEMENT {
        PreparsedElementType::Style
    } else if node_name == SCRIPT_ELEMENT {
        PreparsedElementType::Script
    } else if node_name == LINK_ELEMENT && rel_attr.as_deref() == Some(LINK_STYLE_REL_VALUE) {
        PreparsedElementType::Stylesheet
    } else {
        PreparsedElementType::Other
    };

    PreparsedElement::new(
        element_type,
        select_attr,
        href_attr,
        non_bindable,
        project_as,
    )
}

/// Normalize ng-content select attribute
/// Returns "*" if selectAttr is null or empty, otherwise returns the selectAttr
fn normalize_ng_content_select(select_attr: Option<String>) -> String {
    match select_attr {
        None => "*".to_string(),
        Some(s) if s.is_empty() => "*".to_string(),
        Some(s) => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml_parser::ast::{Attribute, Element};
    use crate::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};

    fn create_element(name: &str, attrs: Vec<Attribute>) -> Element {
        let source_file = ParseSourceFile::new(String::new(), "test.html".to_string());
        let source_span = ParseSourceSpan::new(
            ParseLocation::new(source_file.clone(), 0, 1, 1),
            ParseLocation::new(source_file.clone(), 0, 1, 1),
        );
        Element {
            name: name.to_string(),
            attrs,
            directives: vec![],
            children: vec![],
            is_self_closing: false,
            source_span: source_span.clone(),
            start_source_span: source_span.clone(),
            end_source_span: None,
            is_void: false,
            i18n: None,
        }
    }

    #[test]
    fn test_preparse_ng_content() {
        let element = create_element("ng-content", vec![]);
        let preparsed = preparse_element(&element);
        assert_eq!(preparsed.element_type, PreparsedElementType::NgContent);
        assert_eq!(preparsed.select_attr, "*");
    }

    #[test]
    fn test_preparse_ng_content_with_select() {
        let source_file = ParseSourceFile::new(String::new(), "test.html".to_string());
        let source_span = ParseSourceSpan::new(
            ParseLocation::new(source_file.clone(), 0, 1, 1),
            ParseLocation::new(source_file.clone(), 0, 1, 1),
        );
        let element = create_element(
            "ng-content",
            vec![Attribute {
                name: "select".to_string(),
                value: ".my-class".to_string(),
                source_span: source_span.clone(),
                key_span: None,
                value_span: None,
                value_tokens: None,
                i18n: None,
            }],
        );
        let preparsed = preparse_element(&element);
        assert_eq!(preparsed.element_type, PreparsedElementType::NgContent);
        assert_eq!(preparsed.select_attr, ".my-class");
    }

    #[test]
    fn test_preparse_style_element() {
        let element = create_element("style", vec![]);
        let preparsed = preparse_element(&element);
        assert_eq!(preparsed.element_type, PreparsedElementType::Style);
    }

    #[test]
    fn test_preparse_script_element() {
        let element = create_element("script", vec![]);
        let preparsed = preparse_element(&element);
        assert_eq!(preparsed.element_type, PreparsedElementType::Script);
    }

    #[test]
    fn test_preparse_stylesheet_link() {
        let source_file = ParseSourceFile::new(String::new(), "test.html".to_string());
        let source_span = ParseSourceSpan::new(
            ParseLocation::new(source_file.clone(), 0, 1, 1),
            ParseLocation::new(source_file.clone(), 0, 1, 1),
        );
        let element = create_element(
            "link",
            vec![
                Attribute {
                    name: "rel".to_string(),
                    value: "stylesheet".to_string(),
                    source_span: source_span.clone(),
                    key_span: None,
                    value_span: None,
                    value_tokens: None,
                    i18n: None,
                },
                Attribute {
                    name: "href".to_string(),
                    value: "styles.css".to_string(),
                    source_span: source_span.clone(),
                    key_span: None,
                    value_span: None,
                    value_tokens: None,
                    i18n: None,
                },
            ],
        );
        let preparsed = preparse_element(&element);
        assert_eq!(preparsed.element_type, PreparsedElementType::Stylesheet);
        assert_eq!(preparsed.href_attr, Some("styles.css".to_string()));
    }

    #[test]
    fn test_preparse_non_bindable() {
        let source_file = ParseSourceFile::new(String::new(), "test.html".to_string());
        let source_span = ParseSourceSpan::new(
            ParseLocation::new(source_file.clone(), 0, 1, 1),
            ParseLocation::new(source_file.clone(), 0, 1, 1),
        );
        let element = create_element(
            "div",
            vec![Attribute {
                name: "ngNonBindable".to_string(),
                value: "".to_string(),
                source_span: source_span.clone(),
                key_span: None,
                value_span: None,
                value_tokens: None,
                i18n: None,
            }],
        );
        let preparsed = preparse_element(&element);
        assert!(preparsed.non_bindable);
    }

    #[test]
    fn test_preparse_project_as() {
        let source_file = ParseSourceFile::new(String::new(), "test.html".to_string());
        let source_span = ParseSourceSpan::new(
            ParseLocation::new(source_file.clone(), 0, 1, 1),
            ParseLocation::new(source_file.clone(), 0, 1, 1),
        );
        let element = create_element(
            "div",
            vec![Attribute {
                name: "ngProjectAs".to_string(),
                value: "my-component".to_string(),
                source_span: source_span.clone(),
                key_span: None,
                value_span: None,
                value_tokens: None,
                i18n: None,
            }],
        );
        let preparsed = preparse_element(&element);
        assert_eq!(preparsed.project_as, "my-component");
    }

    #[test]
    fn test_preparse_other_element() {
        let element = create_element("div", vec![]);
        let preparsed = preparse_element(&element);
        assert_eq!(preparsed.element_type, PreparsedElementType::Other);
    }
}

