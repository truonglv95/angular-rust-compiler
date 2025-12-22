//! Shadow CSS Test Utils
//!
//! Corresponds to packages/compiler/test/shadow_css/utils.ts

use angular_compiler::shadow_css::ShadowCss;
use regex::Regex;

pub fn shim(css: &str, content_attr: &str, host_attr: &str) -> String {
    let shadow_css = ShadowCss::new();
    shadow_css.shim_css_text(css, content_attr, host_attr)
}

pub fn extract_css_content(css: &str) -> String {
    let re1 = Regex::new(r"^\n\s+").unwrap();
    let re2 = Regex::new(r"\n\s+$").unwrap();
    let re3 = Regex::new(r"\s+").unwrap();
    let re4 = Regex::new(r":\s").unwrap();
    let re5 = Regex::new(r" }").unwrap();
    let re6 = Regex::new(r"\{\s+").unwrap();
    let re7 = Regex::new(r"\s+\}").unwrap();
    
    let mut result = re1.replace(css, "").to_string();
    result = re2.replace(&result, "").to_string();
    result = re3.replace_all(&result, " ").to_string();
    result = re4.replace_all(&result, ":").to_string();
    result = re5.replace_all(&result, "}").to_string();
    result = re6.replace_all(&result, "{").to_string();
    result = re7.replace_all(&result, "}").to_string();
    // Trim leading and trailing whitespace
    result.trim().to_string()
}

pub fn assert_equal_css(actual: &str, expected: &str) {
    let actual_css = extract_css_content(actual);
    let expected_css = extract_css_content(expected);
    assert_eq!(actual_css, expected_css, "Expected '{}' to equal '{}'", actual_css, expected_css);
}

#[allow(dead_code)]
pub fn assert_contains(actual: &str, expected: &str) {
    assert!(
        actual.contains(expected),
        "Expected '{}' to contain '{}'",
        actual,
        expected
    );
}

pub fn assert_not_contains(actual: &str, expected: &str) {
    assert!(
        !actual.contains(expected),
        "Expected '{}' to not contain '{}'",
        actual,
        expected
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_css_content() {
        let css = "  \n  one { color: red; }  \n  ";
        let result = extract_css_content(css);
        assert_eq!(result, "one {color:red;}");
    }
}

