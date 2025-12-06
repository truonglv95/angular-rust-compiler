//! ng-deep Tests
//!
//! Corresponds to packages/compiler/test/shadow_css/ng_deep_spec.ts
//! All test cases match exactly with TypeScript version

mod utils;
use utils::{extract_css_content, shim};

fn assert_equal_css(actual: &str, expected: &str) {
    let actual_css = extract_css_content(actual);
    let expected_css = extract_css_content(expected);
    assert_eq!(actual_css, expected_css, "Expected '{}' to equal '{}'", actual_css, expected_css);
}

#[test]
fn should_handle_deep() {
    let css = shim("x /deep/ y {}", "contenta", "");
    assert_equal_css(&css, "x[contenta] y {}");
}

#[test]
fn should_handle_triple_gt() {
    let css = shim("x >>> y {}", "contenta", "");
    assert_equal_css(&css, "x[contenta] y {}");
}

#[test]
fn should_handle_ng_deep() {
    let css = shim("::ng-deep y {}", "contenta", "");
    assert_equal_css(&css, "y {}");
    
    let css = shim("x ::ng-deep y {}", "contenta", "");
    assert_equal_css(&css, "x[contenta] y {}");
    
    let css = shim(":host > ::ng-deep .x {}", "contenta", "h");
    assert_equal_css(&css, "[h] > .x {}");
    
    let css = shim(":host ::ng-deep > .x {}", "contenta", "h");
    assert_equal_css(&css, "[h] > .x {}");
    
    let css = shim(":host > ::ng-deep > .x {}", "contenta", "h");
    assert_equal_css(&css, "[h] > > .x {}");
}

