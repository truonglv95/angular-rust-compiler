use once_cell::sync::Lazy;
/**
 * Directive Matching - CSS Selector Matching
 *
 * Corresponds to packages/compiler/src/directive_matching.ts
 * Implements CSS selector parsing and matching for Angular directives
 */
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Regex for parsing CSS selectors
static SELECTOR_REGEXP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(\:not\()|(([\.\#]?)[-\w]+)|(?:\[([-.\w*\\$]+)(?:=(?:"([^"]*)"|'([^']*)'|([^\]]*)))?\])|(\))|(\s*,\s*)"#).unwrap()
});

/// Match groups in the selector regex
#[derive(Debug, Clone, Copy)]
enum SelectorRegexp {
    All = 0,
    Not = 1,
    Tag = 2,
    Prefix = 3,
    Attribute = 4,
    AttributeValueDouble = 5,
    AttributeValueSingle = 6,
    AttributeValueUnquoted = 7,
    NotEnd = 8,
    Separator = 9,
}

/// CSS Selector representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssSelector {
    pub element: Option<String>,
    pub class_names: Vec<String>,
    /// Attributes stored in pairs: [name, value, name, value, ...]
    pub attrs: Vec<String>,
    pub not_selectors: Vec<CssSelector>,
}

impl CssSelector {
    pub fn new() -> Self {
        CssSelector {
            element: None,
            class_names: Vec::new(),
            attrs: Vec::new(),
            not_selectors: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.element = None;
        self.class_names.clear();
        self.attrs.clear();
        self.not_selectors.clear();
    }

    /// Parse CSS selector string into CssSelector objects
    pub fn parse(selector: &str) -> Result<Vec<CssSelector>, String> {
        let mut results = Vec::new();
        let mut css_selector = CssSelector::new();
        let mut in_not = false;

        for cap in SELECTOR_REGEXP.captures_iter(selector) {
            // Check for :not(
            if cap.get(SelectorRegexp::Not as usize).is_some() {
                if in_not {
                    return Err("Nesting :not in a selector is not allowed".to_string());
                }
                in_not = true;
                css_selector.not_selectors.push(CssSelector::new());
            }

            // Check for tag/class/id
            if let Some(tag_match) = cap.get(SelectorRegexp::Tag as usize) {
                let tag = tag_match.as_str();
                let prefix = cap
                    .get(SelectorRegexp::Prefix as usize)
                    .map(|m| m.as_str())
                    .unwrap_or("");

                let current = if in_not {
                    css_selector.not_selectors.last_mut().unwrap()
                } else {
                    &mut css_selector
                };

                if prefix == "#" {
                    // ID selector: #id
                    current.add_attribute("id", &tag[1..]);
                } else if prefix == "." {
                    // Class selector: .class
                    current.add_class_name(&tag[1..]);
                } else {
                    // Element selector: div
                    current.set_element(tag);
                }
            }

            // Check for attribute
            if let Some(attr_match) = cap.get(SelectorRegexp::Attribute as usize) {
                let attr = attr_match.as_str();
                let value = if let Some(m) = cap.get(SelectorRegexp::AttributeValueDouble as usize)
                {
                    m.as_str()
                } else if let Some(m) = cap.get(SelectorRegexp::AttributeValueSingle as usize) {
                    m.as_str()
                } else if let Some(m) = cap.get(SelectorRegexp::AttributeValueUnquoted as usize) {
                    m.as_str()
                } else {
                    ""
                };

                let current = if in_not {
                    css_selector.not_selectors.last_mut().unwrap()
                } else {
                    &mut css_selector
                };

                current.add_attribute(&Self::unescape_attribute(attr)?, value);
            }

            // Check for ) closing :not
            if cap.get(SelectorRegexp::NotEnd as usize).is_some() {
                in_not = false;
            }

            // Check for , separator
            if cap.get(SelectorRegexp::Separator as usize).is_some() {
                if in_not {
                    return Err("Multiple selectors in :not are not supported".to_string());
                }
                Self::add_result(&mut results, css_selector);
                css_selector = CssSelector::new();
            }
        }

        Self::add_result(&mut results, css_selector);
        Ok(results)
    }

    fn add_result(results: &mut Vec<CssSelector>, mut css_sel: CssSelector) {
        if !css_sel.not_selectors.is_empty()
            && css_sel.element.is_none()
            && css_sel.class_names.is_empty()
            && css_sel.attrs.is_empty()
        {
            css_sel.element = Some("*".to_string());
        }
        results.push(css_sel);
    }

    /// Unescape \$ sequences from CSS attribute selector
    fn unescape_attribute(attr: &str) -> Result<String, String> {
        let mut result = String::new();
        let mut escaping = false;

        for ch in attr.chars() {
            if ch == '\\' {
                escaping = true;
                continue;
            }
            if ch == '$' && !escaping {
                return Err(format!(
                    "Error in attribute selector \"{}\". Unescaped \"$\" is not supported. Please escape with \"\\$\".",
                    attr
                ));
            }
            escaping = false;
            result.push(ch);
        }

        Ok(result)
    }

    /// Escape $ in attribute for selector output
    fn escape_attribute(attr: &str) -> String {
        attr.replace('$', "\\$")
    }

    pub fn is_element_selector(&self) -> bool {
        self.has_element_selector() && self.class_names.is_empty() && self.attrs.is_empty()
    }

    pub fn has_element_selector(&self) -> bool {
        self.element.is_some() && self.element.as_ref().unwrap() != "*"
    }

    pub fn set_element(&mut self, element: &str) {
        self.element = Some(element.to_string());
    }

    pub fn add_attribute(&mut self, name: &str, value: &str) {
        self.attrs.push(name.to_string());
        self.attrs.push(value.to_lowercase());
    }

    pub fn add_class_name(&mut self, name: &str) {
        self.class_names.push(name.to_lowercase());
    }

    /// Get attribute value by name
    pub fn get_attr(&self, name: &str) -> Option<&str> {
        for i in (0..self.attrs.len()).step_by(2) {
            if self.attrs[i] == name {
                return Some(&self.attrs[i + 1]);
            }
        }
        None
    }
}

impl std::fmt::Display for CssSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = self.element.as_ref().map(|s| s.as_str()).unwrap_or("");
        write!(f, "{}", res)?;

        for class_name in &self.class_names {
            write!(f, ".{}", class_name)?;
        }

        for i in (0..self.attrs.len()).step_by(2) {
            let name = Self::escape_attribute(&self.attrs[i]);
            let value = &self.attrs[i + 1];
            if value.is_empty() {
                write!(f, "[{}]", name)?;
            } else {
                write!(f, "[{}={}]", name, value)?;
            }
        }

        for not_selector in &self.not_selectors {
            write!(f, ":not({})", not_selector)?;
        }

        Ok(())
    }
}

/// Selector Matcher - matches CSS selectors against directives
pub struct SelectorMatcher<T> {
    element_map: HashMap<String, Vec<SelectorContext<T>>>,
    class_map: HashMap<String, Vec<SelectorContext<T>>>,
    attr_map: HashMap<String, HashMap<String, Vec<SelectorContext<T>>>>,
    counter: usize,
}

#[derive(Clone)]
struct SelectorContext<T> {
    selector: CssSelector,
    callback_data: T,
    id: usize,
}

impl<T: Clone> SelectorMatcher<T> {
    pub fn new() -> Self {
        SelectorMatcher {
            element_map: HashMap::new(),
            class_map: HashMap::new(),
            attr_map: HashMap::new(),
            counter: 0,
        }
    }

    /// Add a selector with associated data
    pub fn add_selectable(&mut self, css_selector: CssSelector, callback_data: T) {
        let context = SelectorContext {
            selector: css_selector.clone(),
            callback_data,
            id: self.counter,
        };
        self.counter += 1;

        // Index by element
        if let Some(ref element) = css_selector.element {
            // Always index by element, even * (for universal lookup)
            self.element_map
                .entry(element.clone())
                .or_insert_with(Vec::new)
                .push(context.clone());
        }

        // Index by class names
        for class_name in &css_selector.class_names {
            self.class_map
                .entry(class_name.clone())
                .or_insert_with(Vec::new)
                .push(context.clone());
        }

        // Index by attributes
        for i in (0..css_selector.attrs.len()).step_by(2) {
            let name = &css_selector.attrs[i];
            let value = &css_selector.attrs[i + 1];

            self.attr_map
                .entry(name.clone())
                .or_insert_with(HashMap::new)
                .entry(value.clone())
                .or_insert_with(Vec::new)
                .push(context.clone());
        }
    }

    /// Match a CSS selector against indexed selectors
    pub fn match_selector<F>(&self, css_selector: &CssSelector, mut callback: F) -> bool
    where
        F: FnMut(&CssSelector, &T),
    {
        let mut matched = false;
        let mut matched_ids = std::collections::HashSet::new();

        self.match_selector_visit(css_selector, |sel, data, id| {
            if matched_ids.insert(id) {
                callback(sel, data);
                matched = true;
            }
        });

        matched
    }

    fn match_selector_visit<F>(&self, css_selector: &CssSelector, mut callback: F)
    where
        F: FnMut(&CssSelector, &T, usize),
    {
        // Match by element
        if let Some(ref element) = css_selector.element {
            if let Some(contexts) = self.element_map.get(element) {
                for context in contexts {
                    if self.is_match(css_selector, &context.selector) {
                        callback(&context.selector, &context.callback_data, context.id);
                    }
                }
            }
        }

        // Always match universal selector *
        if let Some(contexts) = self.element_map.get("*") {
            for context in contexts {
                if self.is_match(css_selector, &context.selector) {
                    callback(&context.selector, &context.callback_data, context.id);
                }
            }
        }

        // Match by class names
        for class_name in &css_selector.class_names {
            if let Some(contexts) = self.class_map.get(class_name) {
                for context in contexts {
                    if self.is_match(css_selector, &context.selector) {
                        callback(&context.selector, &context.callback_data, context.id);
                    }
                }
            }
        }

        // Match by attributes
        for i in (0..css_selector.attrs.len()).step_by(2) {
            let name = &css_selector.attrs[i];
            let value = &css_selector.attrs[i + 1];

            if let Some(attr_values) = self.attr_map.get(name) {
                // Check exact value match
                if let Some(contexts) = attr_values.get(value) {
                    for context in contexts {
                        if self.is_match(css_selector, &context.selector) {
                            callback(&context.selector, &context.callback_data, context.id);
                        }
                    }
                }
                // Check generic attribute match (selector has [attr] without value, indexed as "")
                if !value.is_empty() {
                    if let Some(contexts) = attr_values.get("") {
                        for context in contexts {
                            if self.is_match(css_selector, &context.selector) {
                                callback(&context.selector, &context.callback_data, context.id);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check if two selectors match
    fn is_match(&self, selector: &CssSelector, pattern: &CssSelector) -> bool {
        // Simple match logic - can be enhanced
        if let (Some(ref sel_elem), Some(ref pat_elem)) = (&selector.element, &pattern.element) {
            if sel_elem != pat_elem && pat_elem != "*" {
                return false;
            }
        }

        // Check if all pattern classes are in selector
        for pat_class in &pattern.class_names {
            if !selector.class_names.contains(pat_class) {
                return false;
            }
        }

        // Check if all pattern attributes match
        for i in (0..pattern.attrs.len()).step_by(2) {
            let pat_name = &pattern.attrs[i];
            let pat_value = &pattern.attrs[i + 1];

            let mut found = false;
            for j in (0..selector.attrs.len()).step_by(2) {
                if &selector.attrs[j] == pat_name {
                    // Value matching rules:
                    // Test `should_select_by_attr_name_case_sensitive_and_value_case_insensitive` implies case-insensitive value match.
                    if pat_value.is_empty() || selector.attrs[j + 1].eq_ignore_ascii_case(pat_value)
                    {
                        found = true;
                        break;
                    }
                }
            }

            if !found {
                return false;
            }
        }

        // Check :not selectors
        for not_selector in &pattern.not_selectors {
            if self.is_match(selector, not_selector) {
                return false;
            }
        }

        true
    }
}

impl Default for CssSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Default for SelectorMatcher<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_selector() {
        let selectors = CssSelector::parse("div").unwrap();
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0].element, Some("div".to_string()));
    }

    #[test]
    fn test_parse_class_selector() {
        let selectors = CssSelector::parse(".my-class").unwrap();
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0].class_names, vec!["my-class"]);
    }

    #[test]
    fn test_parse_id_selector() {
        let selectors = CssSelector::parse("#my-id").unwrap();
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0].get_attr("id"), Some("my-id"));
    }

    #[test]
    fn test_parse_attribute_selector() {
        let selectors = CssSelector::parse("[attr=value]").unwrap();
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0].get_attr("attr"), Some("value"));
    }

    #[test]
    fn test_parse_combined_selector() {
        let selectors = CssSelector::parse("div.my-class[attr=value]").unwrap();
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0].element, Some("div".to_string()));
        assert_eq!(selectors[0].class_names, vec!["my-class"]);
        assert_eq!(selectors[0].get_attr("attr"), Some("value"));
    }

    #[test]
    fn test_parse_not_selector() {
        let selectors = CssSelector::parse("div:not(.exclude)").unwrap();
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0].element, Some("div".to_string()));
        assert_eq!(selectors[0].not_selectors.len(), 1);
    }

    #[test]
    fn test_parse_multiple_selectors() {
        let selectors = CssSelector::parse("div, span").unwrap();
        assert_eq!(selectors.len(), 2);
        assert_eq!(selectors[0].element, Some("div".to_string()));
        assert_eq!(selectors[1].element, Some("span".to_string()));
    }

    #[test]
    fn test_selector_to_string() {
        let mut selector = CssSelector::new();
        selector.set_element("div");
        selector.add_class_name("my-class");
        selector.add_attribute("attr", "value");

        let result = selector.to_string();
        assert!(result.contains("div"));
        assert!(result.contains(".my-class"));
        assert!(result.contains("[attr=value]"));
    }

    #[test]
    fn test_selector_matcher() {
        let mut matcher = SelectorMatcher::new();

        let directive_selector = CssSelector::parse("div.my-class").unwrap();
        matcher.add_selectable(directive_selector[0].clone(), "MyDirective");

        let element_selector = CssSelector::parse("div.my-class.other").unwrap();
        let mut matched = false;

        matcher.match_selector(&element_selector[0], |_sel, data| {
            matched = true;
            assert_eq!(*data, "MyDirective");
        });

        assert!(matched);
    }

    #[test]
    fn test_unescape_attribute() {
        let result = CssSelector::unescape_attribute("test\\$attr").unwrap();
        assert_eq!(result, "test$attr");
    }

    #[test]
    fn test_unescape_attribute_error() {
        let result = CssSelector::unescape_attribute("test$attr");
        assert!(result.is_err());
    }
}

/// Matcher for directives that don't have CSS selectors (selectorless directives).
/// Matches directives by their name/class name.
pub struct SelectorlessMatcher<T> {
    registry: HashMap<String, Vec<T>>,
}

impl<T> SelectorlessMatcher<T> {
    pub fn new() -> Self {
        SelectorlessMatcher {
            registry: HashMap::new(),
        }
    }

    /// Add a directive to the matcher, keyed by its name.
    pub fn add(&mut self, name: String, directive: T) {
        self.registry
            .entry(name)
            .or_insert_with(Vec::new)
            .push(directive);
    }

    /// Match directives by name. Returns all directives registered for the given name.
    pub fn match_name(&self, name: &str) -> Vec<T>
    where
        T: Clone,
    {
        self.registry.get(name).cloned().unwrap_or_default()
    }
}

impl<T> Default for SelectorlessMatcher<T> {
    fn default() -> Self {
        Self::new()
    }
}
