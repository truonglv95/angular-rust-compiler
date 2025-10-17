//! Utility Functions
//!
//! Corresponds to packages/compiler/src/util.ts
//! Common utility functions

use regex::Regex;
use once_cell::sync::Lazy;

/// Regex for dash-case to camelCase conversion
static DASH_CASE_REGEXP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"-+([a-z0-9])").unwrap()
});

/// Convert dash-case to camelCase
pub fn dash_case_to_camel_case(input: &str) -> String {
    DASH_CASE_REGEXP.replace_all(input, |caps: &regex::Captures| {
        caps.get(1).unwrap().as_str().to_uppercase()
    }).to_string()
}

/// Split string at colon
pub fn split_at_colon(input: &str, default_values: &[Option<&str>]) -> Vec<Option<String>> {
    split_at(input, ':', default_values)
}

/// Split string at period
pub fn split_at_period(input: &str, default_values: &[Option<&str>]) -> Vec<Option<String>> {
    split_at(input, '.', default_values)
}

fn split_at(input: &str, character: char, default_values: &[Option<&str>]) -> Vec<Option<String>> {
    if let Some(char_index) = input.find(character) {
        vec![
            Some(input[..char_index].trim().to_string()),
            Some(input[char_index + 1..].trim().to_string()),
        ]
    } else {
        default_values.iter().map(|v| v.map(|s| s.to_string())).collect()
    }
}

/// Escape characters that have special meaning in Regular Expressions
pub fn escape_regex(text: &str) -> String {
    let mut result = String::new();
    for ch in text.chars() {
        if matches!(ch, '.' | '*' | '+' | '?' | '^' | '=' | '!' | ':' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '/' | '\\') {
            result.push('\\');
        }
        result.push(ch);
    }
    result
}

/// UTF-8 encode a string
pub fn utf8_encode(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}

/// Stringify any value for debugging
pub fn stringify<T: std::fmt::Debug>(value: &T) -> String {
    format!("{:?}", value)
}

/// Version class
#[derive(Debug, Clone)]
pub struct Version {
    pub full: String,
    pub major: String,
    pub minor: String,
    pub patch: String,
}

impl Version {
    pub fn new(full: &str) -> Self {
        let parts: Vec<&str> = full.split('.').collect();
        Version {
            full: full.to_string(),
            major: parts.get(0).unwrap_or(&"0").to_string(),
            minor: parts.get(1).unwrap_or(&"0").to_string(),
            patch: parts.get(2).unwrap_or(&"0").to_string(),
        }
    }
}

/// Check if standalone should be default for a given version
pub fn get_jit_standalone_default_for_version(version: &str) -> bool {
    if version.starts_with("0.") {
        // 0.0.0 is always "latest", default is true
        return true;
    }

    // Check if version is v1-v18 (default false)
    if let Some(first_part) = version.split('.').next() {
        if let Ok(major) = first_part.parse::<u32>() {
            if (1..=18).contains(&major) {
                return false;
            }
        }
    }

    // All other versions (v19+) default to true
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dash_case_to_camel_case() {
        assert_eq!(dash_case_to_camel_case("hello-world"), "helloWorld");
        assert_eq!(dash_case_to_camel_case("my-component-name"), "myComponentName");
        assert_eq!(dash_case_to_camel_case("simple"), "simple");
    }

    #[test]
    fn test_split_at_colon() {
        let result = split_at_colon("key:value", &[None, None]);
        assert_eq!(result[0], Some("key".to_string()));
        assert_eq!(result[1], Some("value".to_string()));
    }

    #[test]
    fn test_split_at_period() {
        let result = split_at_period("module.component", &[None, None]);
        assert_eq!(result[0], Some("module".to_string()));
        assert_eq!(result[1], Some("component".to_string()));
    }

    #[test]
    fn test_escape_regex() {
        assert_eq!(escape_regex("hello.world"), "hello\\.world");
        assert_eq!(escape_regex("a*b+c?"), "a\\*b\\+c\\?");
    }

    #[test]
    fn test_version() {
        let v = Version::new("1.2.3");
        assert_eq!(v.major, "1");
        assert_eq!(v.minor, "2");
        assert_eq!(v.patch, "3");
        assert_eq!(v.full, "1.2.3");
    }

    #[test]
    fn test_get_jit_standalone_default() {
        assert!(get_jit_standalone_default_for_version("0.0.0"));
        assert!(!get_jit_standalone_default_for_version("18.0.0"));
        assert!(get_jit_standalone_default_for_version("19.0.0"));
        assert!(get_jit_standalone_default_for_version("21.0.0"));
    }

    #[test]
    fn test_utf8_encode() {
        let encoded = utf8_encode("hello");
        assert_eq!(encoded, vec![104, 101, 108, 108, 111]);
    }
}
