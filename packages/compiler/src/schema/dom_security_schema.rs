//! DOM Security Schema
//!
//! Corresponds to packages/compiler/src/schema/dom_security_schema.ts (102 lines)
//!
//! # Security Warning
//!
//! ```text
//! =================================================================================================
//! =========== S T O P   -  S T O P   -  S T O P   -  S T O P   -  S T O P   -  S T O P  ===========
//! =================================================================================================
//!
//!        DO NOT EDIT THIS LIST OF SECURITY SENSITIVE PROPERTIES WITHOUT A SECURITY REVIEW!
//!                               Reach out to mprobst for details.
//!
//! =================================================================================================
//! ```

use crate::core::SecurityContext;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

/// Map from tagName|propertyName to SecurityContext. Properties applying to all tags use '*'.
static SECURITY_SCHEMA: Lazy<HashMap<String, SecurityContext>> = Lazy::new(|| {
    let mut schema = HashMap::new();

    // Case is insignificant below, all element and attribute names are lower-cased for lookup.

    // SecurityContext::HTML
    register_context(&mut schema, SecurityContext::HTML, &[
        "iframe|srcdoc",
        "*|innerhtml",
        "*|outerhtml",
    ]);

    // SecurityContext::STYLE
    register_context(&mut schema, SecurityContext::STYLE, &[
        "*|style",
    ]);

    // NB: no SCRIPT contexts here, they are never allowed due to the parser stripping them.

    // SecurityContext::URL
    register_context(&mut schema, SecurityContext::URL, &[
        "*|formaction",
        "area|href",
        "area|ping",
        "audio|src",
        "a|href",
        "a|ping",
        "blockquote|cite",
        "body|background",
        "del|cite",
        "form|action",
        "img|src",
        "input|src",
        "ins|cite",
        "q|cite",
        "source|src",
        "track|src",
        "video|poster",
        "video|src",
    ]);

    // SecurityContext::RESOURCE_URL
    register_context(&mut schema, SecurityContext::RESOURCE_URL, &[
        "applet|code",
        "applet|codebase",
        "base|href",
        "embed|src",
        "frame|src",
        "head|profile",
        "html|manifest",
        "iframe|src",
        "link|href",
        "media|src",
        "object|codebase",
        "object|data",
        "script|src",
    ]);

    schema
});

fn register_context(schema: &mut HashMap<String, SecurityContext>, ctx: SecurityContext, specs: &[&str]) {
    for spec in specs {
        schema.insert(spec.to_lowercase(), ctx);
    }
}

/// Get the security schema
pub fn security_schema() -> &'static HashMap<String, SecurityContext> {
    &SECURITY_SCHEMA
}

/// The set of security-sensitive attributes of an `<iframe>` that *must* be
/// applied as a static attribute only. This ensures that all security-sensitive
/// attributes are taken into account while creating an instance of an `<iframe>`
/// at runtime.
///
/// Note: avoid using this set directly, use the `is_iframe_security_sensitive_attr` function
/// in the code instead.
static IFRAME_SECURITY_SENSITIVE_ATTRS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();
    set.insert("sandbox");
    set.insert("allow");
    set.insert("allowfullscreen");
    set.insert("referrerpolicy");
    set.insert("csp");
    set.insert("fetchpriority");
    set
});

/// Checks whether a given attribute name might represent a security-sensitive
/// attribute of an <iframe>.
pub fn is_iframe_security_sensitive_attr(attr_name: &str) -> bool {
    // The `setAttribute` DOM API is case-insensitive, so we lowercase the value
    // before checking it against a known security-sensitive attributes.
    IFRAME_SECURITY_SENSITIVE_ATTRS.contains(attr_name.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_schema() {
        let schema = security_schema();
        assert_eq!(schema.get("iframe|srcdoc"), Some(&SecurityContext::HTML));
        assert_eq!(schema.get("*|style"), Some(&SecurityContext::STYLE));
        assert_eq!(schema.get("a|href"), Some(&SecurityContext::URL));
        assert_eq!(schema.get("script|src"), Some(&SecurityContext::RESOURCE_URL));
    }

    #[test]
    fn test_iframe_security_sensitive_attrs() {
        assert!(is_iframe_security_sensitive_attr("sandbox"));
        assert!(is_iframe_security_sensitive_attr("allow"));
        assert!(is_iframe_security_sensitive_attr("SANDBOX")); // case insensitive
        assert!(!is_iframe_security_sensitive_attr("id"));
    }
}

