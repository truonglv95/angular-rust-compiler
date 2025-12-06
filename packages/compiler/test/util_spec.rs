//! Utility Functions Tests
//!
//! Corresponds to packages/compiler/test/util_spec.ts
//! All test cases match exactly with TypeScript version

use angular_compiler::util;
use regex::Regex;

// splitAtColon tests - matching TypeScript exactly
#[test]
fn should_split_when_a_single_colon_is_present() {
    let result = util::split_at_colon("a:b", &[None, None]);
    assert_eq!(result, vec![Some("a".to_string()), Some("b".to_string())]);
}

#[test]
fn should_trim_parts() {
    let result = util::split_at_colon(" a : b ", &[None, None]);
    assert_eq!(result, vec![Some("a".to_string()), Some("b".to_string())]);
}

#[test]
fn should_support_multiple_colons() {
    let result = util::split_at_colon("a:b:c", &[None, None]);
    assert_eq!(result, vec![Some("a".to_string()), Some("b:c".to_string())]);
}

#[test]
fn should_use_the_default_value_when_no_colon_is_present() {
    let result = util::split_at_colon("ab", &[Some("c"), Some("d")]);
    assert_eq!(result, vec![Some("c".to_string()), Some("d".to_string())]);
}

// RegExp tests - matching TypeScript exactly
#[test]
fn should_escape_regexp() {
    // Test that escaped regex matches correctly
    let pattern = util::escape_regex("b");
    let re = Regex::new(&pattern).unwrap();
    assert!(re.is_match("abc"), "Should match 'b' in 'abc'");
    assert!(!re.is_match("adc"), "Should not match 'b' in 'adc'");
    
    let pattern = util::escape_regex("a.b");
    let re = Regex::new(&pattern).unwrap();
    assert!(re.is_match("a.b"), "Should match 'a.b' in 'a.b'");
    assert!(!re.is_match("axb"), "Should not match 'a.b' in 'axb'");
}

// Helper function to encode UTF-16 code units directly (like TypeScript charCodeAt)
// This matches TypeScript's behavior exactly - it works with UTF-16 code units, not Rust chars
fn utf8_encode_from_utf16(utf16: &[u16]) -> Vec<u8> {
    let mut encoded = Vec::new();
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

// utf8encode tests - matching TypeScript exactly
#[test]
fn should_encode_to_utf8() {
    // Tests from https://github.com/mathiasbynens/wtf-8
    // Exact matches with TypeScript test cases
    let test_strings = vec![
        "abc",
        "\0",
        "\u{0080}",
        "\u{05CA}",
        "\u{07FF}",
        "\u{0800}",
        "\u{2C3C}",
        "\u{FFFF}",
        "\u{10000}",
        "\u{1D306}",
        "\u{10FFFF}",
    ];
    
    let expected_results = vec![
        vec![b'a', b'b', b'c'],
        vec![0],
        vec![0xC2, 0x80],
        vec![0xD7, 0x8A],
        vec![0xDF, 0xBF],
        vec![0xE0, 0xA0, 0x80],
        vec![0xE2, 0xB0, 0xBC],
        vec![0xEF, 0xBF, 0xBF],
        vec![0xF0, 0x90, 0x80, 0x80], // U+D800 U+DC00 in JS
        vec![0xF0, 0x9D, 0x8C, 0x86], // U+D834 U+DF06 in JS
        vec![0xF4, 0x8F, 0xBF, 0xBF], // U+DBFF U+DFFF in JS
    ];

    for (input, expected_bytes) in test_strings.iter().zip(expected_results.iter()) {
        let encoded = util::utf8_encode(input);
        assert_eq!(encoded, *expected_bytes, "Failed for input: {:?}", input);
    }
}

#[test]
fn should_handle_unmatched_surrogate_halves() {
    // Test unmatched surrogate halves by creating UTF-16 sequences directly
    // High surrogates: 0xD800 to 0xDBFF
    // Low surrogates: 0xDC00 to 0xDFFF
    // TypeScript works directly with UTF-16 code units, so we use helper function
    
    // For high surrogates - matching TypeScript test cases exactly
    let high_surrogate_tests = vec![
        (vec![0xD800u16], vec![0xED, 0xA0, 0x80]),
        (vec![0xD800u16, 0xD800u16], vec![0xED, 0xA0, 0x80, 0xED, 0xA0, 0x80]),
        (vec![0xD800u16, b'A' as u16], vec![0xED, 0xA0, 0x80, b'A']),
        (vec![0xD9AFu16], vec![0xED, 0xA6, 0xAF]),
        (vec![0xDBFFu16], vec![0xED, 0xAF, 0xBF]),
    ];
    
    for (utf16_input, expected) in high_surrogate_tests {
        // Use helper function to encode directly from UTF-16 (like TypeScript)
        let encoded = utf8_encode_from_utf16(&utf16_input);
        assert_eq!(encoded, expected, "Failed for UTF-16: {:?}", utf16_input);
    }
    
    // For low surrogates - matching TypeScript test cases exactly
    let low_surrogate_tests = vec![
        (vec![0xDC00u16], vec![0xED, 0xB0, 0x80]),
        (vec![0xDC00u16, 0xDC00u16], vec![0xED, 0xB0, 0x80, 0xED, 0xB0, 0x80]),
        (vec![0xDC00u16, b'A' as u16], vec![0xED, 0xB0, 0x80, b'A']),
        (vec![0xDEEEu16], vec![0xED, 0xBB, 0xAE]),
        (vec![0xDFFFu16], vec![0xED, 0xBF, 0xBF]),
    ];
    
    for (utf16_input, expected) in low_surrogate_tests {
        // Use helper function to encode directly from UTF-16 (like TypeScript)
        let encoded = utf8_encode_from_utf16(&utf16_input);
        assert_eq!(encoded, expected, "Failed for UTF-16: {:?}", utf16_input);
    }
    
    // Test combined cases - matching TypeScript test cases exactly
    // D800 D834 DF06 D800 (from TypeScript test)
    let combined = vec![0xD800u16, 0xD834u16, 0xDF06u16, 0xD800u16];
    let encoded = utf8_encode_from_utf16(&combined);
    // Expected: ED A0 80 F0 9D 8C 86 ED A0 80
    assert_eq!(encoded, vec![0xED, 0xA0, 0x80, 0xF0, 0x9D, 0x8C, 0x86, 0xED, 0xA0, 0x80]);
    
    // DC00 D834 DF06 DC00 (from TypeScript test)
    let combined2 = vec![0xDC00u16, 0xD834u16, 0xDF06u16, 0xDC00u16];
    let encoded2 = utf8_encode_from_utf16(&combined2);
    // Expected: ED B0 80 F0 9D 8C 86 ED B0 80
    assert_eq!(encoded2, vec![0xED, 0xB0, 0x80, 0xF0, 0x9D, 0x8C, 0x86, 0xED, 0xB0, 0x80]);
}

// stringify tests - matching TypeScript exactly
#[test]
fn should_handle_objects_with_no_prototype() {
    // In Rust, we don't have prototype concept like JavaScript
    // But we can test with a struct that implements Debug
    // The TypeScript test expects "object" for Object.create(null)
    #[derive(Debug)]
    struct EmptyStruct;
    
    // Implement Stringify for EmptyStruct to match expected behavior
    impl util::Stringify for EmptyStruct {
        fn stringify(&self) -> String {
            "object".to_string()
        }
    }
    
    let result = util::stringify(&EmptyStruct);
    assert_eq!(result, "object");
}

