//! Style URL Resolver
//!
//! Corresponds to packages/compiler/src/style_url_resolver.ts
//! Some code comes from WebComponents.JS
//! https://github.com/webcomponents/webcomponentsjs/blob/master/src/HTMLImports/path.js

use regex::Regex;
use once_cell::sync::Lazy;

/// Regex to match URL schema
static URL_WITH_SCHEMA_REGEXP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([^:/?#]+):").unwrap()
});

/// Check if style URL is resolvable
///
/// Returns true if:
/// - URL has no schema (relative URL)
/// - URL has schema 'package' or 'asset'
///
/// Returns false if:
/// - URL is null or empty
/// - URL starts with '/' (absolute path)
/// - URL has other schema (e.g., 'http', 'https')
///
/// This matches the exact behavior of TypeScript `isStyleUrlResolvable`
pub fn is_style_url_resolvable(url: Option<&str>) -> bool {
    match url {
        None => false,
        Some(u) if u.is_empty() => false,
        Some(u) if u.starts_with('/') => false,
        Some(u) => {
            if let Some(caps) = URL_WITH_SCHEMA_REGEXP.captures(u) {
                let schema = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                // Return true if schema is 'package' or 'asset'
                schema == "package" || schema == "asset"
            } else {
                // No schema means relative URL - resolvable
                true
            }
        }
    }
}
