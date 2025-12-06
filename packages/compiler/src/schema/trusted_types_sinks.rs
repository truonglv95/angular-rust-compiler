//! Trusted Types Sinks
//!
//! Corresponds to packages/compiler/src/schema/trusted_types_sinks.ts (48 lines)
//!
//! Set of tagName|propertyName corresponding to Trusted Types sinks.
//! Extracted from: https://w3c.github.io/webappsec-trusted-types/dist/spec/#integrations

use once_cell::sync::Lazy;
use std::collections::HashSet;

/// Set of tagName|propertyName corresponding to Trusted Types sinks.
/// Properties applying to all tags use '*'.
///
/// NOTE: All strings in this set *must* be lowercase!
static TRUSTED_TYPES_SINKS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();

    // TrustedHTML
    set.insert("iframe|srcdoc");
    set.insert("*|innerhtml");
    set.insert("*|outerhtml");

    // NB: no TrustedScript here, as the corresponding tags are stripped by the compiler.

    // TrustedScriptURL
    set.insert("embed|src");
    set.insert("object|codebase");
    set.insert("object|data");

    set
});

/// Returns true if the given property on the given DOM tag is a Trusted Types sink.
///
/// In that case, use `ElementSchemaRegistry.securityContext` to determine which particular
/// Trusted Type is required for values passed to the sink:
/// - SecurityContext.HTML corresponds to TrustedHTML
/// - SecurityContext.RESOURCE_URL corresponds to TrustedScriptURL
pub fn is_trusted_types_sink(tag_name: &str, prop_name: &str) -> bool {
    // Make sure comparisons are case insensitive, so that case differences between
    // attribute and property names do not have a security impact.
    let tag_lower = tag_name.to_lowercase();
    let prop_lower = prop_name.to_lowercase();

    let combined = format!("{}|{}", tag_lower, prop_lower);
    let wildcard = format!("*|{}", prop_lower);

    TRUSTED_TYPES_SINKS.contains(combined.as_str()) || TRUSTED_TYPES_SINKS.contains(wildcard.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trusted_html_sinks() {
        assert!(is_trusted_types_sink("iframe", "srcdoc"));
        assert!(is_trusted_types_sink("div", "innerHTML"));
        assert!(is_trusted_types_sink("span", "outerHTML"));
    }

    #[test]
    fn test_trusted_script_url_sinks() {
        assert!(is_trusted_types_sink("embed", "src"));
        assert!(is_trusted_types_sink("object", "codebase"));
        assert!(is_trusted_types_sink("object", "data"));
    }

    #[test]
    fn test_case_insensitive() {
        assert!(is_trusted_types_sink("IFRAME", "SRCDOC"));
        assert!(is_trusted_types_sink("Div", "InnerHTML"));
    }

    #[test]
    fn test_not_sink() {
        assert!(!is_trusted_types_sink("div", "id"));
        assert!(!is_trusted_types_sink("a", "href"));
    }
}

