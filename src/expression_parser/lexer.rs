/**
 * Angular Expression Lexer - Rust Implementation
 *
 * Tokenizes Angular template expressions into tokens for parsing
 */

use serde::{Deserialize, Serialize};
use crate::chars;

/// Token types in Angular expressions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TokenType {
    Character = 0,
    Identifier = 1,
    PrivateIdentifier = 2,
    Keyword = 3,
    String = 4,
    Operator = 5,
    Number = 6,
    RegExpBody = 7,
    RegExpFlags = 8,
    Error = 9,
}

/// String token kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StringTokenKind {
    Plain,
    TemplateLiteralPart,
    TemplateLiteralEnd,
}

/// Token representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub index: usize,
    pub end: usize,
    pub token_type: TokenType,
    pub num_value: f64,
    pub str_value: String,
    pub kind: Option<StringTokenKind>,
}

impl Token {
    pub fn new(
        index: usize,
        end: usize,
        token_type: TokenType,
        num_value: f64,
        str_value: String,
    ) -> Self {
        Token {
            index,
            end,
            token_type,
            num_value,
            str_value,
            kind: None,
        }
    }

    pub fn with_kind(mut self, kind: StringTokenKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn is_character(&self, code: char) -> bool {
        self.token_type == TokenType::Character && self.str_value.chars().next() == Some(code)
    }

    pub fn is_number(&self) -> bool {
        self.token_type == TokenType::Number
    }

    pub fn is_string(&self) -> bool {
        self.token_type == TokenType::String
    }

    pub fn is_identifier(&self) -> bool {
        self.token_type == TokenType::Identifier
    }

    pub fn is_keyword(&self) -> bool {
        self.token_type == TokenType::Keyword
    }

    pub fn is_private_identifier(&self) -> bool {
        self.token_type == TokenType::PrivateIdentifier
    }

    pub fn is_operator(&self, operator: &str) -> bool {
        self.token_type == TokenType::Operator && self.str_value == operator
    }

    pub fn is_keyword_let(&self) -> bool {
        self.token_type == TokenType::Keyword && self.str_value == "let"
    }

    pub fn is_keyword_as(&self) -> bool {
        self.token_type == TokenType::Keyword && self.str_value == "as"
    }

    pub fn is_keyword_null(&self) -> bool {
        self.token_type == TokenType::Keyword && self.str_value == "null"
    }

    pub fn is_keyword_undefined(&self) -> bool {
        self.token_type == TokenType::Keyword && self.str_value == "undefined"
    }

    pub fn is_keyword_true(&self) -> bool {
        self.token_type == TokenType::Keyword && self.str_value == "true"
    }

    pub fn is_keyword_false(&self) -> bool {
        self.token_type == TokenType::Keyword && self.str_value == "false"
    }

    pub fn is_keyword_this(&self) -> bool {
        self.token_type == TokenType::Keyword && self.str_value == "this"
    }

    pub fn is_keyword_typeof(&self) -> bool {
        self.token_type == TokenType::Keyword && self.str_value == "typeof"
    }

    pub fn is_keyword_void(&self) -> bool {
        self.token_type == TokenType::Keyword && self.str_value == "void"
    }

    pub fn is_keyword_in(&self) -> bool {
        self.token_type == TokenType::Keyword && self.str_value == "in"
    }

    pub fn is_error(&self) -> bool {
        self.token_type == TokenType::Error
    }

    pub fn is_regexp_body(&self) -> bool {
        self.token_type == TokenType::RegExpBody
    }

    pub fn is_regexp_flags(&self) -> bool {
        self.token_type == TokenType::RegExpFlags
    }

    pub fn is_template_literal_part(&self) -> bool {
        self.token_type == TokenType::String
            && self.kind == Some(StringTokenKind::TemplateLiteralPart)
    }

    pub fn is_template_literal_end(&self) -> bool {
        self.token_type == TokenType::String
            && self.kind == Some(StringTokenKind::TemplateLiteralEnd)
    }

    pub fn is_template_literal_interpolation_start(&self) -> bool {
        self.is_operator("${")
    }

    pub fn to_number(&self) -> f64 {
        if self.token_type == TokenType::Number {
            self.num_value
        } else {
            -1.0
        }
    }

    pub fn to_string(&self) -> Option<String> {
        match self.token_type {
            TokenType::Character
            | TokenType::Identifier
            | TokenType::Keyword
            | TokenType::Operator
            | TokenType::PrivateIdentifier
            | TokenType::String
            | TokenType::Error
            | TokenType::RegExpBody
            | TokenType::RegExpFlags => Some(self.str_value.clone()),
            TokenType::Number => Some(self.num_value.to_string()),
        }
    }
}

/// StringToken (extends Token for template literals)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringToken {
    pub token: Token,
    pub kind: StringTokenKind,
}

impl StringToken {
    pub fn new(index: usize, end: usize, str_value: String, kind: StringTokenKind) -> Self {
        let mut token = Token::new(index, end, TokenType::String, 0.0, str_value);
        token.kind = Some(kind);
        StringToken { token, kind }
    }
}

/// EOF token constant
pub const EOF: Token = Token {
    index: usize::MAX,
    end: usize::MAX,
    token_type: TokenType::Character,
    num_value: 0.0,
    str_value: String::new(),
    kind: None,
};

/// Helper functions for creating tokens
pub fn new_character_token(index: usize, end: usize, code: char) -> Token {
    Token::new(index, end, TokenType::Character, code as u32 as f64, code.to_string())
}

pub fn new_identifier_token(index: usize, end: usize, text: String) -> Token {
    Token::new(index, end, TokenType::Identifier, 0.0, text)
}

pub fn new_private_identifier_token(index: usize, end: usize, text: String) -> Token {
    Token::new(index, end, TokenType::PrivateIdentifier, 0.0, text)
}

pub fn new_keyword_token(index: usize, end: usize, text: String) -> Token {
    Token::new(index, end, TokenType::Keyword, 0.0, text)
}

pub fn new_operator_token(index: usize, end: usize, text: String) -> Token {
    Token::new(index, end, TokenType::Operator, 0.0, text)
}

pub fn new_number_token(index: usize, end: usize, n: f64) -> Token {
    Token::new(index, end, TokenType::Number, n, String::new())
}

pub fn new_error_token(index: usize, end: usize, message: String) -> Token {
    Token::new(index, end, TokenType::Error, 0.0, message)
}

pub fn new_regexp_body_token(index: usize, end: usize, text: String) -> Token {
    Token::new(index, end, TokenType::RegExpBody, 0.0, text)
}

pub fn new_regexp_flags_token(index: usize, end: usize, text: String) -> Token {
    Token::new(index, end, TokenType::RegExpFlags, 0.0, text)
}

/// Angular expression lexer
pub struct Lexer;

impl Lexer {
    pub fn new() -> Self {
        Lexer
    }

    pub fn tokenize(&self, text: &str) -> Vec<Token> {
        Scanner::new(text).scan()
    }
}

/// Scanner for tokenizing input
struct Scanner {
    input: String,
    length: usize,
    index: usize,
    peek: char,
    tokens: Vec<Token>,
}


// Angular keywords
const KEYWORDS: &[&str] = &[
    "var", "let", "as", "null", "undefined", "true", "false",
    "if", "else", "this", "typeof", "void", "in",
];

impl Scanner {
    fn new(input: &str) -> Self {
        let peek = input.chars().next().unwrap_or(chars::EOF);
        Scanner {
            input: input.to_string(),
            length: input.len(),
            index: 0,
            peek,
            tokens: Vec::new(),
        }
    }

    fn scan(mut self) -> Vec<Token> {
        while let Some(token) = self.scan_token() {
            self.tokens.push(token);
        }
        self.tokens
    }

    fn advance(&mut self) {
        self.index += self.peek.len_utf8();
        self.peek = if self.index < self.length {
            self.input[self.index..].chars().next().unwrap_or(chars::EOF)
        } else {
            chars::EOF
        };
    }

    fn scan_token(&mut self) -> Option<Token> {
        // Skip whitespace
        while self.index < self.length && chars::is_whitespace(self.peek) {
            self.advance();
        }

        if self.index >= self.length {
            return None;
        }

        let start = self.index;
        let ch = self.peek;

        // Handle identifiers and keywords
        if chars::is_identifier_start(ch) {
            return Some(self.scan_identifier());
        }

        // Handle numbers
        if chars::is_digit(ch) {
            return Some(self.scan_number(start));
        }

        // Handle operators and special characters
        match ch {
            chars::PERIOD => {
                self.advance();
                if chars::is_digit(self.peek) {
                    return Some(self.scan_number(start));
                }
                return Some(Token::new(
                    start,
                    self.index,
                    TokenType::Character,
                    chars::PERIOD as i32 as f64,
                    chars::PERIOD.to_string(),
                ));
            }
            chars::LPAREN | chars::RPAREN | chars::LBRACKET | chars::RBRACKET |
            chars::COMMA | chars::COLON | chars::SEMICOLON => {
                return Some(self.scan_character(start, ch));
            }
            chars::LBRACE => {
                return Some(self.scan_character(start, ch));
            }
            chars::RBRACE => {
                return Some(self.scan_character(start, ch));
            }
            chars::SQ | chars::DQ => {
                return Some(self.scan_string(ch));
            }
            chars::BT => {
                self.advance();
                return Some(self.scan_template_literal_part(start));
            }
            chars::HASH => {
                return Some(self.scan_private_identifier());
            }
            chars::PLUS => {
                return Some(self.scan_complex_operator(start, "+", chars::EQ, '='));
            }
            chars::MINUS => {
                return Some(self.scan_complex_operator(start, "-", chars::EQ, '='));
            }
            chars::STAR => {
                return Some(self.scan_complex_operator(start, "*", chars::EQ, '='));
            }
            chars::SLASH => {
                return Some(self.scan_complex_operator(start, "/", chars::EQ, '='));
            }
            chars::PERCENT => {
                return Some(self.scan_complex_operator(start, "%", chars::EQ, '='));
            }
            chars::LT => {
                return Some(self.scan_complex_operator(start, "<", chars::EQ, '='));
            }
            chars::GT => {
                return Some(self.scan_complex_operator(start, ">", chars::EQ, '='));
            }
            chars::EQ => {
                return Some(self.scan_complex_operator(start, "=", chars::EQ, '='));
            }
            chars::BANG => {
                return Some(self.scan_complex_operator(start, "!", chars::EQ, '='));
            }
            chars::AMPERSAND => {
                return Some(self.scan_complex_operator(start, "&", chars::AMPERSAND, '&'));
            }
            chars::BAR => {
                return Some(self.scan_complex_operator(start, "|", chars::BAR, '|'));
            }
            chars::QUESTION => {
                return Some(self.scan_operator(start, "?"));
            }
            _ => {
                self.advance();
                return Some(Token::new(
                    start,
                    self.index,
                    TokenType::Error,
                    0.0,
                    format!("Unexpected character: {}", ch),
                ));
            }
        }
    }

    fn scan_character(&mut self, start: usize, ch: char) -> Token {
        self.advance();
        Token::new(
            start,
            self.index,
            TokenType::Character,
            ch as i32 as f64,
            ch.to_string(),
        )
    }

    fn scan_identifier(&mut self) -> Token {
        let start = self.index;
        self.advance();

        while self.index < self.length && chars::is_identifier_part(self.peek) {
            self.advance();
        }

        let str_value = self.input[start..self.index].to_string();
        let token_type = if KEYWORDS.contains(&str_value.as_str()) {
            TokenType::Keyword
        } else {
            TokenType::Identifier
        };

        Token::new(start, self.index, token_type, 0.0, str_value)
    }

    fn scan_private_identifier(&mut self) -> Token {
        let start = self.index;
        self.advance(); // Skip #

        if !chars::is_identifier_start(self.peek) {
            return Token::new(
                start,
                self.index,
                TokenType::Error,
                0.0,
                "Invalid private identifier".to_string(),
            );
        }

        while self.index < self.length && chars::is_identifier_part(self.peek) {
            self.advance();
        }

        let str_value = self.input[start..self.index].to_string();
        Token::new(start, self.index, TokenType::PrivateIdentifier, 0.0, str_value)
    }

    fn scan_number(&mut self, start: usize) -> Token {
        let mut has_dot = false;

        while self.index < self.length {
            if chars::is_digit(self.peek) {
                self.advance();
            } else if self.peek == chars::PERIOD && !has_dot {
                has_dot = true;
                self.advance();
            } else {
                break;
            }
        }

        let str_value = self.input[start..self.index].to_string();
        let num_value = str_value.parse::<f64>().unwrap_or(0.0);

        Token::new(start, self.index, TokenType::Number, num_value, str_value)
    }

    fn scan_string(&mut self, quote: char) -> Token {
        let start = self.index;
        self.advance(); // Skip opening quote

        let mut buffer = String::new();
        let mut escaped = false;

        while self.index < self.length {
            let ch = self.peek;

            if escaped {
                buffer.push(match ch {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    _ => ch,
                });
                escaped = false;
                self.advance();
            } else if ch == '\\' {
                escaped = true;
                self.advance();
            } else if ch == quote {
                self.advance(); // Skip closing quote
                break;
            } else {
                buffer.push(ch);
                self.advance();
            }
        }

        Token::new(start, self.index, TokenType::String, 0.0, buffer)
            .with_kind(StringTokenKind::Plain)
    }

    fn scan_template_literal_part(&mut self, start: usize) -> Token {
        let mut buffer = String::new();

        while self.index < self.length {
            let ch = self.peek;

            if ch == chars::BT {
                self.advance();
                return Token::new(start, self.index, TokenType::String, 0.0, buffer)
                    .with_kind(StringTokenKind::TemplateLiteralEnd);
            } else if ch == chars::DOLLAR {
                // Check for ${
                self.advance();
                if self.peek == chars::LBRACE {
                    return Token::new(start, self.index - 1, TokenType::String, 0.0, buffer)
                        .with_kind(StringTokenKind::TemplateLiteralPart);
                }
                buffer.push(chars::DOLLAR);
            } else {
                buffer.push(ch);
                self.advance();
            }
        }

        Token::new(start, self.index, TokenType::Error, 0.0, "Unterminated template literal".to_string())
    }

    fn scan_operator(&mut self, start: usize, op: &str) -> Token {
        self.advance();
        Token::new(start, self.index, TokenType::Operator, 0.0, op.to_string())
    }

    fn scan_complex_operator(&mut self, start: usize, op1: &str, two: char, op2: char) -> Token {
        self.advance();
        if self.peek == two {
            self.advance();
            Token::new(start, self.index, TokenType::Operator, 0.0, format!("{}{}", op1, op2))
        } else {
            Token::new(start, self.index, TokenType::Operator, 0.0, op1.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple_expression() {
        let lexer = Lexer::new();
        let tokens = lexer.tokenize("a + b");

        assert_eq!(tokens.len(), 3);
        assert!(tokens[0].is_identifier());
        assert_eq!(tokens[0].str_value, "a");
        assert_eq!(tokens[1].token_type, TokenType::Operator);
        assert_eq!(tokens[1].str_value, "+");
        assert!(tokens[2].is_identifier());
        assert_eq!(tokens[2].str_value, "b");
    }

    #[test]
    fn test_tokenize_number() {
        let lexer = Lexer::new();
        let tokens = lexer.tokenize("42.5");

        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].is_number());
        assert_eq!(tokens[0].num_value, 42.5);
    }

    #[test]
    fn test_tokenize_string() {
        let lexer = Lexer::new();
        let tokens = lexer.tokenize("'hello world'");

        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].is_string());
        assert_eq!(tokens[0].str_value, "hello world");
    }

    #[test]
    fn test_tokenize_keywords() {
        let lexer = Lexer::new();
        let tokens = lexer.tokenize("let x = null");

        assert!(tokens[0].is_keyword());
        assert_eq!(tokens[0].str_value, "let");
        assert!(tokens[3].is_keyword());
        assert_eq!(tokens[3].str_value, "null");
    }

    #[test]
    fn test_tokenize_property_access() {
        let lexer = Lexer::new();
        let tokens = lexer.tokenize("user.name");

        assert_eq!(tokens.len(), 3);
        assert!(tokens[0].is_identifier());
        assert!(tokens[1].is_character('.'));
        assert!(tokens[2].is_identifier());
    }
}
