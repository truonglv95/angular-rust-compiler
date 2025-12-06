//! Assertions Module
//!
//! Corresponds to packages/compiler/src/assertions.ts (31 lines)

use regex::Regex;
use once_cell::sync::Lazy;

static UNUSABLE_INTERPOLATION_REGEXPS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"@").unwrap(),           // control flow reserved symbol
        Regex::new(r"^\s*$").unwrap(),       // empty
        Regex::new(r"[<>]").unwrap(),        // html tag
        Regex::new(r"^[{}]$").unwrap(),      // i18n expansion
        Regex::new(r"&(#|[a-z])").unwrap(),  // character reference
        Regex::new(r"^//").unwrap(),         // comment
    ]
});

pub fn assert_interpolation_symbols(identifier: &str, value: Option<&[String]>) -> Result<(), String> {
    if let Some(val) = value {
        if val.len() != 2 {
            return Err(format!("Expected '{}' to be an array, [start, end].", identifier));
        }

        let start = &val[0];
        let end = &val[1];

        // Check for unusable interpolation symbols
        for regexp in UNUSABLE_INTERPOLATION_REGEXPS.iter() {
            if regexp.is_match(start) || regexp.is_match(end) {
                return Err(format!("['{}', '{}'] contains unusable interpolation symbol.", start, end));
            }
        }
    }

    Ok(())
}
