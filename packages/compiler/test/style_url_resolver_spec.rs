//! Style URL Resolver Tests
//!
//! Corresponds to packages/compiler/test/style_url_resolver_spec.ts
//! All test cases match exactly with TypeScript version

use angular_compiler::style_url_resolver;

#[test]
fn should_resolve_relative_urls() {
    assert!(style_url_resolver::is_style_url_resolvable(Some("someUrl.css")));
}

#[test]
fn should_resolve_package_urls() {
    assert!(style_url_resolver::is_style_url_resolvable(Some("package:someUrl.css")));
}

#[test]
fn should_not_resolve_empty_urls() {
    assert!(!style_url_resolver::is_style_url_resolvable(None));
    assert!(!style_url_resolver::is_style_url_resolvable(Some("")));
}

#[test]
fn should_not_resolve_urls_with_other_schema() {
    assert!(!style_url_resolver::is_style_url_resolvable(Some("http://otherurl")));
}

#[test]
fn should_not_resolve_urls_with_absolute_paths() {
    assert!(!style_url_resolver::is_style_url_resolvable(Some("/otherurl")));
    assert!(!style_url_resolver::is_style_url_resolvable(Some("//otherurl")));
}

