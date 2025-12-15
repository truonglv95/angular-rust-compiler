/*
 * Character Codes
 *
 * Corresponds to packages/compiler/src/chars.ts
 */
#![allow(non_upper_case_globals)]

//! Character constants used throughout the compiler

// Special characters
pub const EOF: char = '\0';
pub const BSPACE: char = '\x08';  // Backspace
pub const TAB: char = '\t';
pub const LF: char = '\n';        // Line feed
pub const NEWLINE: char = '\n';   // Alias for LF
pub const VTAB: char = '\x0B';
pub const FF: char = '\x0C';
pub const CR: char = '\r';        // Carriage return
pub const RETURN: char = '\r';    // Alias for CR
pub const SPACE: char = ' ';
pub const NBSP: char = '\u{00A0}';

// Punctuation
pub const BANG: char = '!';
pub const DQ: char = '"';
pub const HASH: char = '#';
pub const DOLLAR: char = '$';
pub const PERCENT: char = '%';
pub const AMPERSAND: char = '&';
pub const SQ: char = '\'';
pub const LPAREN: char = '(';
pub const RPAREN: char = ')';
pub const STAR: char = '*';
pub const PLUS: char = '+';
pub const COMMA: char = ',';
pub const MINUS: char = '-';
pub const PERIOD: char = '.';
pub const SLASH: char = '/';
pub const COLON: char = ':';
pub const SEMICOLON: char = ';';
pub const LT: char = '<';
pub const EQ: char = '=';
pub const GT: char = '>';
pub const QUESTION: char = '?';
pub const AT: char = '@';

// Brackets
pub const LBRACKET: char = '[';
pub const BACKSLASH: char = '\\';
pub const RBRACKET: char = ']';
pub const CARET: char = '^';
pub const UNDERSCORE: char = '_';
pub const BT: char = '`';

// Braces
pub const LBRACE: char = '{';
pub const BAR: char = '|';
pub const PIPE: char = '|';       // Alias for BAR
pub const RBRACE: char = '}';
pub const TILDA: char = '~';

// Letters (for quick checks)
pub const A: char = 'A';
pub const E: char = 'E';
pub const F: char = 'F';
pub const X: char = 'X';
pub const Z: char = 'Z';

pub const a: char = 'a';
pub const b: char = 'b';
pub const e: char = 'e';
pub const f: char = 'f';
pub const n: char = 'n';
pub const r: char = 'r';
pub const t: char = 't';
pub const u: char = 'u';
pub const v: char = 'v';
pub const x: char = 'x';
pub const z: char = 'z';

// Digits
pub const ZERO: char = '0';
pub const CHAR_0: char = '0';     // Alias for ZERO
pub const CHAR_7: char = '7';
pub const CHAR_9: char = '9';
pub const NINE: char = '9';       // Alias for CHAR_9

/// Check if character is whitespace
pub fn is_whitespace(ch: char) -> bool {
    ch == SPACE || ch == TAB || ch == NEWLINE || ch == RETURN || ch == VTAB || ch == FF || ch <= ' '
}

/// Check if character is a digit
pub fn is_digit(ch: char) -> bool {
    ch >= ZERO && ch <= NINE
}

/// Check if character is ASCII letter
pub fn is_ascii_letter(ch: char) -> bool {
    (ch >= a && ch <= z) || (ch >= A && ch <= Z)
}

/// Check if character is ASCII hex digit
pub fn is_ascii_hex_digit(ch: char) -> bool {
    (ch >= a && ch <= f) || (ch >= A && ch <= F) || is_digit(ch)
}

/// Check if character is newline
pub fn is_new_line(ch: char) -> bool {
    ch == NEWLINE || ch == RETURN
}

/// Check if character is octal digit (0-7)
pub fn is_octal_digit(ch: char) -> bool {
    ch >= ZERO && ch <= '7'
}

/// Check if character is a quote
pub fn is_quote(ch: char) -> bool {
    ch == SQ || ch == DQ || ch == BT
}

// Additional helper functions (not in Angular chars.ts but needed for Rust implementation)

/// Check if character can start an identifier
pub fn is_identifier_start(ch: char) -> bool {
    (ch >= a && ch <= z) || (ch >= A && ch <= Z) || ch == UNDERSCORE || ch == DOLLAR
}

/// Check if character can be part of an identifier
pub fn is_identifier_part(ch: char) -> bool {
    is_identifier_start(ch) || is_digit(ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_whitespace() {
        assert!(is_whitespace(' '));
        assert!(is_whitespace('\t'));
        assert!(is_whitespace('\n'));
        assert!(is_whitespace('\r'));
        assert!(!is_whitespace('a'));
    }

    #[test]
    fn test_is_digit() {
        assert!(is_digit('0'));
        assert!(is_digit('5'));
        assert!(is_digit('9'));
        assert!(!is_digit('a'));
        assert!(!is_digit(' '));
    }

    #[test]
    fn test_is_identifier_start() {
        assert!(is_identifier_start('a'));
        assert!(is_identifier_start('Z'));
        assert!(is_identifier_start('_'));
        assert!(is_identifier_start('$'));
        assert!(!is_identifier_start('5'));
        assert!(!is_identifier_start(' '));
    }

    #[test]
    fn test_is_identifier_part() {
        assert!(is_identifier_part('a'));
        assert!(is_identifier_part('5'));
        assert!(is_identifier_part('_'));
        assert!(is_identifier_part('$'));
        assert!(!is_identifier_part(' '));
        assert!(!is_identifier_part('!'));
    }
}
