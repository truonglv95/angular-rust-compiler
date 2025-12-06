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
        // Return default values (matching TypeScript behavior)
        default_values.iter().map(|v| v.map(|s| s.to_string())).collect()
    }
}

/// Escape characters that have special meaning in Regular Expressions
/// Matches TypeScript escapeRegExp
pub fn escape_regex(s: &str) -> String {
    let mut result = String::new();
    for ch in s.chars() {
        if matches!(ch, '.' | '*' | '+' | '?' | '^' | '=' | '!' | ':' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '/' | '\\') {
            result.push('\\');
        }
        result.push(ch);
    }
    result
}

/// Convert undefined to null (Rust equivalent - converts Option::None to Option::Some(null-like value))
/// In Rust, we use Option<T> instead of T | undefined
/// This function is mainly for API compatibility
pub fn no_undefined<T>(val: Option<T>) -> Option<T> {
    val
}

/// Internal error function - throws error with message
/// In Rust, we use Result or panic instead
pub fn error(msg: &str) -> ! {
    panic!("Internal Error: {}", msg)
}

/// UTF-8 encode a string
/// Handles surrogate pairs correctly like TypeScript version
/// Matches the exact behavior of TypeScript utf8Encode function
/// 
/// TypeScript uses charCodeAt which returns UTF-16 code units, so we need to
/// work with UTF-16 encoding to match the behavior exactly
pub fn utf8_encode(str: &str) -> Vec<u8> {
    let mut encoded = Vec::new();
    
    // Convert to UTF-16 code units (like JavaScript charCodeAt)
    let utf16: Vec<u16> = str.encode_utf16().collect();
    let mut index = 0;
    
    while index < utf16.len() {
        let mut code_point = utf16[index] as u32;
        
        // Decode surrogate pairs (exactly like TypeScript)
        // High surrogates: 0xD800 to 0xDBFF
        // Low surrogates: 0xDC00 to 0xDFFF
        if code_point >= 0xD800 && code_point <= 0xDBFF && index + 1 < utf16.len() {
            let low = utf16[index + 1] as u32;
            if low >= 0xDC00 && low <= 0xDFFF {
                index += 1;
                code_point = ((code_point - 0xD800) << 10) + low - 0xDC00 + 0x10000;
            }
        }
        
        // Encode to UTF-8 bytes (exactly like TypeScript)
        if code_point <= 0x7f {
            encoded.push(code_point as u8);
        } else if code_point <= 0x7ff {
            encoded.push(((code_point >> 6) & 0x1f | 0xc0) as u8);
            encoded.push((code_point & 0x3f | 0x80) as u8);
        } else if code_point <= 0xffff {
            encoded.push((code_point >> 12 | 0xe0) as u8);
            encoded.push(((code_point >> 6) & 0x3f | 0x80) as u8);
            encoded.push((code_point & 0x3f | 0x80) as u8);
        } else if code_point <= 0x1fffff {
            encoded.push(((code_point >> 18) & 0x07 | 0xf0) as u8);
            encoded.push(((code_point >> 12) & 0x3f | 0x80) as u8);
            encoded.push(((code_point >> 6) & 0x3f | 0x80) as u8);
            encoded.push((code_point & 0x3f | 0x80) as u8);
        }
        
        index += 1;
    }
    
    encoded
}

use std::fmt;

/// Trait for types that can be stringified
/// Matches TypeScript stringify logic
pub trait Stringify {
    fn stringify(&self) -> String;
}

/// Stringify any value - single entry point like TypeScript
/// Usage: stringify(&value) - works for any type that implements Stringify
/// 
/// Examples:
/// ```rust
/// stringify(&"hello")           // "hello"
/// stringify(&vec![1, 2, 3])     // "[1, 2, 3]"
/// stringify(&Some("value"))     // "value"
/// stringify(&None::<&str>)      // "null"
/// stringify(&42)                // "42"
/// ```
pub fn stringify<T: Stringify>(token: &T) -> String {
    token.stringify()
}

// Implement for strings (return as-is)
impl Stringify for str {
    fn stringify(&self) -> String {
        self.to_string()
    }
}

// Also implement for &str references
impl Stringify for &str {
    fn stringify(&self) -> String {
        (*self).to_string()
    }
}

impl Stringify for String {
    fn stringify(&self) -> String {
        self.clone()
    }
}

// Implement for Option (null handling)
impl<T: Stringify> Stringify for Option<T> {
    fn stringify(&self) -> String {
        match self {
            None => "null".to_string(),
            Some(v) => v.stringify(),
        }
    }
}

// Implement for Vec/arrays
impl<T: Stringify> Stringify for Vec<T> {
    fn stringify(&self) -> String {
        format!("[{}]", self.iter().map(|t| t.stringify()).collect::<Vec<_>>().join(", "))
    }
}

// Implement for slices
impl<T: Stringify> Stringify for [T] {
    fn stringify(&self) -> String {
        format!("[{}]", self.iter().map(|t| t.stringify()).collect::<Vec<_>>().join(", "))
    }
}

// Implement for common primitive types
impl Stringify for i8 {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for i16 {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for i32 {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for i64 {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for i128 {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for isize {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for u8 {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for u16 {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for u32 {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for u64 {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for u128 {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for usize {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for f32 {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for f64 {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for bool {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

impl Stringify for char {
    fn stringify(&self) -> String {
        format!("{}", self)
    }
}

// For any other type that implements Debug, use Debug format
// Note: This requires the type to explicitly implement Stringify
// For convenience, you can use stringify_debug() helper function below
pub fn stringify_debug<T: fmt::Debug>(value: &T) -> String {
    let result = format!("{:?}", value);
    // If result contains newline, take only first line (like TypeScript)
    if let Some(newline_pos) = result.find('\n') {
        result[..newline_pos].to_string()
    } else {
        result
    }
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

// Implement Stringify for Version
impl Stringify for Version {
    fn stringify(&self) -> String {
        stringify_debug(self)
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

// Tests are in test/ directory (matching TypeScript structure)
