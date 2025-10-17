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
/// Returns false if:
/// - URL is null or empty
/// - URL starts with '/' (absolute path)
/// - URL has schema 'package' or 'asset'
pub fn is_style_url_resolvable(url: Option<&str>) -> bool {
    match url {
        None => false,
        Some(u) if u.is_empty() => false,
        Some(u) if u.starts_with('/') => false,
        Some(u) => {
            if let Some(caps) = URL_WITH_SCHEMA_REGEXP.captures(u) {
                let schema = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                schema != "package" && schema != "asset"
            } else {
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_style_url_resolvable_empty() {
        assert!(!is_style_url_resolvable(None));
        assert!(!is_style_url_resolvable(Some("")));
    }

    #[test]
    fn test_is_style_url_resolvable_absolute_path() {
        assert!(!is_style_url_resolvable(Some("/absolute/path.css")));
    }

    #[test]
    fn test_is_style_url_resolvable_package_schema() {
        assert!(!is_style_url_resolvable(Some("package:my-package/style.css")));
        assert!(!is_style_url_resolvable(Some("asset:my-asset/style.css")));
    }

    #[test]
    fn test_is_style_url_resolvable_http() {
        assert!(is_style_url_resolvable(Some("http://example.com/style.css")));
        assert!(is_style_url_resolvable(Some("https://example.com/style.css")));
    }

    #[test]
    fn test_is_style_url_resolvable_relative() {
        assert!(is_style_url_resolvable(Some("./style.css")));
        assert!(is_style_url_resolvable(Some("../style.css")));
        assert!(is_style_url_resolvable(Some("style.css")));
    }
}
