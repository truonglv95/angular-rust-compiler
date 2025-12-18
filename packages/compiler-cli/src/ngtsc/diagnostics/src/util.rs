use super::error_code::ErrorCode;
use regex::Regex;
use std::sync::LazyLock;

static ERROR_CODE_MATCHER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\u001b\[\d+m ?)TS-99(\d+: ?\u001b\[\d+m)").unwrap()
});

/// During formatting of `ts.Diagnostic`s, the numeric code of each diagnostic is prefixed with the
/// hard-coded "TS" prefix. For Angular's own error codes, a prefix of "NG" is desirable. To achieve
/// this, all Angular error codes start with "-99" so that the sequence "TS-99" can be assumed to
/// correspond with an Angular specific error code. This function replaces those occurrences with
/// just "NG".
///
/// @param errors The formatted diagnostics
pub fn replace_ts_with_ng_in_errors(errors: &str) -> String {
    ERROR_CODE_MATCHER.replace_all(errors, "${1}NG${2}").to_string()
}

pub fn ng_error_code(code: ErrorCode) -> i32 {
    let code_val = code as i32;
    // JS: parseInt('-99' + code)
    // Rust: -990000 + code (assuming code is <= 99999, which it is)
    // wait, JS behavior: parseInt("-99" + "1001") -> -991001. 
    format!("-99{}", code_val).parse::<i32>().unwrap()
}
