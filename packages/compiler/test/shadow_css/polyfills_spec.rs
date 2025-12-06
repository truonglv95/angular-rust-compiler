//! Polyfills Tests
//!
//! Corresponds to packages/compiler/test/shadow_css/polyfills_spec.ts
//! All test cases match exactly with TypeScript version

mod utils;
use utils::{extract_css_content, shim};

fn assert_equal_css(actual: &str, expected: &str) {
    let actual_css = extract_css_content(actual);
    let expected_css = extract_css_content(expected);
    assert_eq!(actual_css, expected_css, "Expected '{}' to equal '{}'", actual_css, expected_css);
}

#[test]
fn should_support_polyfill_next_selector() {
    let css = shim("polyfill-next-selector {content: 'x > y'} z {}", "contenta", "");
    assert_equal_css(&css, "x[contenta] > y[contenta]{}");
    
    let css = shim("polyfill-next-selector {content: \"x > y\"} z {}", "contenta", "");
    assert_equal_css(&css, "x[contenta] > y[contenta]{}");
    
    let css = shim("polyfill-next-selector {content: 'button[priority=\"1\"]'} z {}", "contenta", "");
    assert_equal_css(&css, "button[priority=\"1\"][contenta]{}");
}

#[test]
fn should_support_polyfill_unscoped_rule() {
    let css = shim("polyfill-unscoped-rule {content: '#menu > .bar';color: blue;}", "contenta", "");
    assert!(css.contains("#menu > .bar {;color: blue;}"));
    
    let css = shim("polyfill-unscoped-rule {content: \"#menu > .bar\";color: blue;}", "contenta", "");
    assert!(css.contains("#menu > .bar {;color: blue;}"));
    
    let css = shim("polyfill-unscoped-rule {content: 'button[priority=\"1\"]'}", "contenta", "");
    assert!(css.contains("button[priority=\"1\"] {}"));
}

#[test]
fn should_support_multiple_instances_polyfill_unscoped_rule() {
    let css = shim(
        &("polyfill-unscoped-rule {content: 'foo';color: blue;}".to_string() +
          "polyfill-unscoped-rule {content: 'bar';color: blue;}"),
        "contenta",
        "",
    );
    assert!(css.contains("foo {;color: blue;}"));
    assert!(css.contains("bar {;color: blue;}"));
}

#[test]
fn should_support_polyfill_rule() {
    let css = shim("polyfill-rule {content: ':host.foo .bar';color: blue;}", "contenta", "a-host");
    assert_equal_css(&css, ".foo[a-host] .bar[contenta] {;color:blue;}");
    
    let css = shim("polyfill-rule {content: \":host.foo .bar\";color:blue;}", "contenta", "a-host");
    assert_equal_css(&css, ".foo[a-host] .bar[contenta] {;color:blue;}");
    
    let css = shim("polyfill-rule {content: 'button[priority=\"1\"]'}", "contenta", "a-host");
    assert_equal_css(&css, "button[priority=\"1\"][contenta] {}");
}

