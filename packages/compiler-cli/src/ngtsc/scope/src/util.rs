// Scope Utilities
//
// Helper functions for scope resolution.

/// Reference kinds for scope tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    /// A component reference.
    Component,
    /// A directive reference.
    Directive,
    /// A pipe reference.
    Pipe,
    /// An NgModule reference.
    NgModule,
}

/// Determine if a selector matches an element.
pub fn selector_matches_element(selector: &str, element_name: &str, _attrs: &[(&str, &str)]) -> bool {
    // Simple selector matching
    if selector.starts_with('[') && selector.ends_with(']') {
        // Attribute selector
        let attr_name = &selector[1..selector.len()-1];
        _attrs.iter().any(|(name, _)| *name == attr_name)
    } else if selector.starts_with('.') {
        // Class selector
        let class_name = &selector[1..];
        _attrs.iter().any(|(name, value)| *name == "class" && value.contains(class_name))
    } else {
        // Element selector
        selector == element_name
    }
}

/// Parse a selector string into parts.
pub fn parse_selector(selector: &str) -> Vec<SelectorPart> {
    let mut parts = Vec::new();
    
    for part in selector.split(',').map(str::trim) {
        if part.is_empty() {
            continue;
        }
        
        if part.starts_with('[') && part.ends_with(']') {
            parts.push(SelectorPart::Attribute(part[1..part.len()-1].to_string()));
        } else if part.starts_with('.') {
            parts.push(SelectorPart::Class(part[1..].to_string()));
        } else if part.starts_with('#') {
            parts.push(SelectorPart::Id(part[1..].to_string()));
        } else {
            parts.push(SelectorPart::Element(part.to_string()));
        }
    }
    
    parts
}

/// A parsed selector part.
#[derive(Debug, Clone)]
pub enum SelectorPart {
    /// Element selector.
    Element(String),
    /// Attribute selector.
    Attribute(String),
    /// Class selector.
    Class(String),
    /// ID selector.
    Id(String),
}

/// Flatten an array of export scopes.
pub fn flatten_exports<T: Clone>(exports: &[Vec<T>]) -> Vec<T> {
    exports.iter().flat_map(|e| e.iter().cloned()).collect()
}

/// Check if a module/component is from @angular/core.
pub fn is_from_angular_core(module_path: &str) -> bool {
    module_path.contains("@angular/core") || module_path == "@angular/core"
}

/// Get the name part from a fully qualified reference.
pub fn get_name_from_reference(reference: &str) -> &str {
    reference.rsplit('/').next().unwrap_or(reference)
}
