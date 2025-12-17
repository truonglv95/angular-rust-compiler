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

    pub fn operator(index: usize, end: usize, str_value: &str) -> Self {
        Token::new(index, end, TokenType::Operator, 0.0, str_value.to_string())
    }

    pub fn with_kind(mut self, kind: StringTokenKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn is_character(&self, code: char) -> bool {
        self.token_type == TokenType::Character && self.str_value == code.to_string()
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
    // Track brace depth for template interpolation
    interpolation_brace_stack: Vec<i32>,
    brace_depth: i32,
    resume_template: bool,
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
            interpolation_brace_stack: Vec::new(),
            brace_depth: 0,
            resume_template: false,
        }
    }

    fn scan(mut self) -> Vec<Token> {
        let mut count = 0;
        while let Some(token) = self.scan_token() {
            count += 1;
            if count > 10000 {
                panic!("Lexer infinite loop! Generated over 10000 tokens for input: {}", self.input);
            }
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
        if self.resume_template {
            self.resume_template = false;
            return Some(self.scan_template_literal_part(self.index));
        }

        // Skip whitespace
        while self.index < self.length && chars::is_whitespace(self.peek) {
            self.advance();
        }

        if self.index >= self.length {
            return None;
        }

        let start = self.index;
        let ch = self.peek;

        // Handle ${ operator specifically before identifiers
        if ch == chars::DOLLAR {
            let next_char = if self.index + 1 < self.length {
                self.input[self.index + 1..].chars().next()
            } else {
                None
            };
            
            if next_char == Some(chars::LBRACE) {
                self.advance(); // consume $
                self.advance(); // consume {
                
                // Track interpolation
                self.interpolation_brace_stack.push(self.brace_depth);
                self.brace_depth += 1;
                
                return Some(Token::operator(start, self.index, "${"));
            }
        }

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
                self.brace_depth += 1;
                return Some(self.scan_character(start, ch));
            }
            chars::RBRACE => {
                self.brace_depth -= 1;
                let token = self.scan_character(start, ch);
                
                if let Some(&target_depth) = self.interpolation_brace_stack.last() {
                     if self.brace_depth == target_depth {
                         self.interpolation_brace_stack.pop();
                         self.resume_template = true;
                     }
                }
                
                return Some(token);
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
                self.advance();
                if self.peek == chars::EQ {
                    self.advance();
                    return Some(Token::operator(start, self.index, "*="));
                }
                if self.peek == chars::STAR {
                    self.advance();
                    if self.peek == chars::EQ {
                        self.advance();
                        return Some(Token::operator(start, self.index, "**="));
                    }
                    return Some(Token::operator(start, self.index, "**"));
                }
                return Some(Token::operator(start, self.index, "*"));
            }
            chars::SLASH => {
                // Check if this is a comment //
                let next_char = if self.index + 1 < self.length {
                    self.input[self.index + 1..].chars().next()
                } else {
                    None
                };

                if next_char == Some(chars::SLASH) {
                     self.advance(); // consume first /
                     self.advance(); // consume second /
                     while self.index < self.length {
                        if self.peek == chars::CR || self.peek == chars::LF {
                            break;
                        }
                        self.advance();
                     }
                     return None;
                }
                
                // Check if this slash is a division operator or a regex literal
                // If the last token was an identifier, number, or closing brace/paren, it's likely division
                // Otherwise (start of expr, operator, keyword, open brace/paren), it's likely a regex
                
                let is_regex_start = if let Some(last) = self.tokens.last() {
                     match last.token_type {
                         TokenType::Identifier | TokenType::PrivateIdentifier | TokenType::Number | 
                         TokenType::String | TokenType::RegExpFlags => false,
                         TokenType::Character => {
                             let s = &last.str_value;
                             s != ")" && s != "]" && s != "}"
                         },
                         TokenType::Operator => {
                             // SPECIAL CASE: `!` can be prefix (Logical Not) or postfix (Non-null assertion).
                             // If `!` is postfix, then `/` is division.
                             // If `!` is prefix, then `/` is regex.
                             // We determine this by looking at what came BEFORE `!`.
                             if last.str_value == "!" {
                                 if self.tokens.len() > 1 {
                                     let prev = &self.tokens[self.tokens.len() - 2];
                                     match prev.token_type {
                                         TokenType::Identifier | TokenType::PrivateIdentifier | TokenType::Number | 
                                         TokenType::String | TokenType::RegExpFlags => false, // Postfix ! found, so / is division
                                         TokenType::Character => {
                                             let s = &prev.str_value;
                                             // If `!` follows `)`, `]`, `}` -> Postfix
                                             !(s == ")" || s == "]" || s == "}")
                                         },
                                         _ => true // Prefix !
                                     }
                                 } else {
                                     true // Start of input ! -> Prefix
                                 }
                             } else {
                                 true // Other operators imply regex start
                             }
                         }, 
                         TokenType::Keyword => {
                             // "this" can be followed by division, others like "return", "typeof" etc likely regex
                             last.str_value != "this"
                         },
                         _ => true
                     }
                } else {
                    true // Start of input
                };

                if is_regex_start {
                    self.advance();
                    return Some(self.scan_regexp_body(start));
                }

                self.advance();
                if self.peek == chars::EQ {
                    self.advance();
                    return Some(Token::operator(start, self.index, "/="));
                }
                return Some(Token::operator(start, self.index, "/"));
            }
            chars::PERCENT => {
                self.advance();
                if self.peek == chars::EQ {
                    self.advance();
                    return Some(Token::operator(start, self.index, "%="));
                }
                return Some(Token::operator(start, self.index, "%"));
            }
            chars::CARET => {
                self.advance();
                if self.peek == chars::EQ {
                    self.advance();
                    return Some(Token::operator(start, self.index, "^="));
                }
                return Some(Token::operator(start, self.index, "^"));
            }
            chars::AMPERSAND => {
                self.advance();
                if self.peek == chars::AMPERSAND {
                    self.advance();
                    if self.peek == chars::EQ {
                        self.advance();
                        return Some(Token::operator(start, self.index, "&&="));
                    }
                    return Some(Token::operator(start, self.index, "&&"));
                }
                if self.peek == chars::EQ {
                    self.advance();
                    return Some(Token::operator(start, self.index, "&="));
                }
                return Some(Token::operator(start, self.index, "&"));
            }
            chars::BAR => {
                self.advance();
                if self.peek == chars::BAR {
                    self.advance();
                    if self.peek == chars::EQ {
                        self.advance();
                        return Some(Token::operator(start, self.index, "||="));
                    }
                    return Some(Token::operator(start, self.index, "||"));
                }
                if self.peek == chars::EQ {
                    self.advance();
                    return Some(Token::operator(start, self.index, "|="));
                }
                return Some(Token::operator(start, self.index, "|"));
            }
            chars::LT => {
                self.advance();
                if self.peek == chars::EQ {
                    self.advance();
                    return Some(Token::operator(start, self.index, "<="));
                }
                if self.peek == chars::LT {
                    self.advance();
                    if self.peek == chars::EQ {
                        self.advance();
                        return Some(Token::operator(start, self.index, "<<="));
                    }
                    return Some(Token::operator(start, self.index, "<<"));
                }
                return Some(Token::operator(start, self.index, "<"));
            }
            chars::GT => {
                self.advance();
                if self.peek == chars::EQ {
                    self.advance();
                    return Some(Token::operator(start, self.index, ">="));
                }
                if self.peek == chars::GT {
                    self.advance();
                    if self.peek == chars::EQ {
                        self.advance();
                        return Some(Token::operator(start, self.index, ">>="));
                    }
                    if self.peek == chars::GT {
                        self.advance();
                        if self.peek == chars::EQ {
                            self.advance();
                            return Some(Token::operator(start, self.index, ">>>="));
                        }
                        return Some(Token::operator(start, self.index, ">>>"));
                    }
                    return Some(Token::operator(start, self.index, ">>"));
                }
                return Some(Token::operator(start, self.index, ">"));
            }
            chars::QUESTION => {
                self.advance();
                if self.peek == chars::PERIOD {
                    self.advance();
                    return Some(Token::operator(start, self.index, "?."));
                }
                if self.peek == chars::QUESTION {
                    self.advance();
                    if self.peek == chars::EQ {
                        self.advance();
                        return Some(Token::operator(start, self.index, "??="));
                    }
                    return Some(Token::operator(start, self.index, "??"));
                }
                return Some(Token::operator(start, self.index, "?"));
            }
            chars::BANG => {
                self.advance();
                if self.peek == chars::EQ {
                    self.advance();
                    if self.peek == chars::EQ {
                        self.advance();
                        return Some(Token::operator(start, self.index, "!=="));
                    }
                    return Some(Token::operator(start, self.index, "!="));
                }
                return Some(Token::operator(start, self.index, "!"));
            }
            chars::EQ => {
                self.advance();
                if self.peek == chars::EQ {
                    self.advance();
                    if self.peek == chars::EQ {
                        self.advance();
                        return Some(Token::operator(start, self.index, "==="));
                    }
                    return Some(Token::operator(start, self.index, "=="));
                }
                return Some(Token::operator(start, self.index, "="));
            }
            _ => {
                self.advance();
                return Some(Token::new(
                    start,
                    self.index,
                    TokenType::Error,
                    0.0,
                    format!("Lexer Error: Invalid character [{}] at column {} in expression [{}]", ch, start, self.input),
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
                format!("Lexer Error: Invalid character [#] at column {} in expression [{}]", start, self.input),
            );
        }

        while self.index < self.length && chars::is_identifier_part(self.peek) {
            self.advance();
        }

        let str_value = self.input[start..self.index].to_string();
        Token::new(start, self.index, TokenType::PrivateIdentifier, 0.0, str_value)
    }

    fn scan_number(&mut self, start: usize) -> Token {
        let mut simple = true;
        while self.index < self.length {
            if chars::is_digit(self.peek) {
                self.advance();
            } else if self.peek == chars::PERIOD {
                simple = false;
                self.advance();
            } else if self.peek == 'e' || self.peek == 'E' {
                simple = false;
                self.advance();
                if self.peek == '+' || self.peek == '-' {
                    self.advance();
                }
            } else if self.peek == chars::UNDERSCORE {
                simple = false;
                // Separators are only valid when they're surrounded by digits
                // Check previous character
                let prev_char = if self.index > 0 {
                    self.input[..self.index].chars().last()
                } else {
                    None
                };
                let next_char = if self.index + 1 < self.length {
                    self.input[self.index + 1..].chars().next()
                } else {
                    None
                };
                 
                let prev_is_digit = prev_char.map_or(false, |c| chars::is_digit(c));
                let next_is_digit = next_char.map_or(false, |c| chars::is_digit(c));
                
                if !prev_is_digit || !next_is_digit {
                    return Token::new(self.index, self.index, TokenType::Error, 0.0, format!("Lexer Error: Invalid numeric separator at column {} in expression [{}]", self.index, self.input));
                }
                self.advance();
            } else {
                break;
            }
        }

        let str_value = self.input[start..self.index].to_string();
        let value_str = if simple { 
            str_value.clone()
        } else {
            str_value.chars().filter(|&c| c != '_').collect()
        };
        
        // Handle invalid exp like '1e'
        if value_str.ends_with('e') || value_str.ends_with('E') || value_str.ends_with('+') || value_str.ends_with('-') {
             return Token::new(self.index - 1, self.index, TokenType::Error, 0.0, format!("Lexer Error: Invalid exponent at column {} in expression [{}]", self.index - 1, self.input));
        }
        
        let num_value = value_str.parse::<f64>().unwrap_or(0.0);

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
                if ch == 'u' {
                    // Unicode escape \uXXXX
                    self.advance();
                    let mut hex = String::new();
                    for _ in 0..4 {
                        if self.index < self.length {
                            hex.push(self.peek);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if hex.len() == 4 {
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(c) = std::char::from_u32(code) {
                                buffer.push(c);
                            } else {
                                return Token::new(self.index - hex.len() - 1, self.index - hex.len() - 1, TokenType::Error, 0.0, format!("Lexer Error: Invalid unicode escape [\\u{}] at column {} in expression [{}]", hex, self.index - hex.len() - 1, self.input));
                            }
                        } else {
                            return Token::new(self.index - hex.len() - 1, self.index - hex.len() - 1, TokenType::Error, 0.0, format!("Lexer Error: Invalid unicode escape [\\u{}] at column {} in expression [{}]", hex, self.index - hex.len() - 1, self.input));
                        }
                    } else {
                        return Token::new(self.index - hex.len() - 1, self.index - hex.len() - 1, TokenType::Error, 0.0, format!("Lexer Error: Invalid unicode escape [\\u{}] at column {} in expression [{}]", hex, self.index - hex.len() - 1, self.input));
                    }
                } else {
                    buffer.push(match ch {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        'b' => '\x08',
                        'f' => '\x0c',
                        'v' => '\x0b',
                        _ => ch,
                    });
                    self.advance();
                }
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
                self.advance();
            } else if ch == quote {
                self.advance(); // Skip closing quote
                
                return Token::new(start, self.index, TokenType::String, 0.0, buffer)
                    .with_kind(StringTokenKind::Plain);
            } else {
                buffer.push(ch);
                self.advance();
            }
        }
        
        // Unterminated string
        Token::new(start, self.index, TokenType::Error, 0.0, format!("Lexer Error: Unterminated quote at column {} in expression [{}]", start, self.input))
    }
    
    fn scan_regexp_body(&mut self, start: usize) -> Token {
        let mut buffer = String::new();
        let mut in_class = false;
        let mut escaped = false;
        
        while self.index < self.length {
            let ch = self.peek;
            
            if escaped {
                buffer.push(ch);
                escaped = false;
                self.advance();
            } else if ch == '\\' {
                buffer.push(ch);
                escaped = true;
                self.advance();
            } else if in_class {
                if ch == ']' {
                    in_class = false;
                }
                buffer.push(ch);
                self.advance();
            } else if ch == '[' {
                in_class = true;
                buffer.push(ch);
                self.advance();
            } else if ch == '/' {
                self.advance(); // Consume closing slash
                
                // Scan flags
                return self.scan_regexp_flags(start, buffer);
            } else {
                if ch == chars::EOF || ch == chars::CR || ch == chars::LF {
                     return Token::new(self.index, self.index, TokenType::Error, 0.0, format!("Lexer Error: Unterminated regular expression at column {} in expression [{}]", self.index, self.input));
                }
                buffer.push(ch);
                self.advance();
            }
        }
        
        Token::new(self.index, self.index, TokenType::Error, 0.0, format!("Lexer Error: Unterminated regular expression at column {} in expression [{}]", self.index, self.input))
    }

    fn scan_regexp_flags(&mut self, start: usize, body: String) -> Token {
        let mut flags = String::new();
        let mut seen = std::collections::HashSet::new();

        while self.index < self.length && chars::is_identifier_part(self.peek) {
            let ch = self.peek;
            // Validate flag
            if !['g', 'i', 'm', 'u', 'y'].contains(&ch) {
                 return Token::new(
                    self.index, 
                    self.index + 1, 
                    TokenType::Error, 
                    0.0, 
                    format!("Lexer Error: Invalid regular expression flag {}", ch)
                );
            }
            // Check duplicate
            if seen.contains(&ch) {
                 return Token::new(
                    self.index, 
                    self.index + 1, 
                    TokenType::Error, 
                    0.0, 
                    format!("Lexer Error: Duplicated regular expression flag {}", ch)
                );
            }
            
            seen.insert(ch);
            flags.push(ch);
            self.advance();
        }

        if flags.is_empty() {
             Token::new(
                start, 
                self.index, 
                TokenType::RegExpBody, 
                0.0, 
                body
            )
        } else {
            self.tokens.push(Token::new(
                start, 
                self.index - flags.len(), 
                TokenType::RegExpBody, 
                0.0, 
                body
            ));
            
            Token::new(
                self.index - flags.len(),
                self.index,
                TokenType::RegExpFlags,
                0.0,
                flags
            )
        }
    }

    fn scan_template_literal_part(&mut self, start: usize) -> Token {
        let mut buffer = String::new();

        while self.index < self.length {
            let ch = self.peek;

            if ch == chars::BACKSLASH {
                self.advance();
                if self.index < self.length {
                    let escaped_char = self.peek;
                     // Handle escaping: only specific chars strictly or just passthrough?
                     // Verify behavior with scans_string. 
                     // For template literals, \` is escaped backtick. \$ is escaped dollar.
                     // Standard string escapes also usually apply.
                     buffer.push(match escaped_char {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        'b' => '\x08',
                        'f' => '\x0c',
                        'v' => '\x0b',
                        // TODO: Unicode escapes in template literals? Assuming simplified for now or handled by parser?
                        // Tests `should_be_able_to_use_interpolation_characters_inside_template_string` likely uses `\${`.
                        _ => escaped_char,
                     });
                     self.advance();
                } else {
                     buffer.push(chars::BACKSLASH); // Trailing backslash
                }
            } else if ch == chars::BT {
                self.advance();
                return Token::new(start, self.index, TokenType::String, 0.0, buffer)
                    .with_kind(StringTokenKind::TemplateLiteralEnd);
            } else if ch == chars::DOLLAR {
                // Check for ${
                let next_char = if self.index + 1 < self.length {
                    self.input[self.index + 1..].chars().next()
                } else {
                    None
                };

                if next_char == Some(chars::LBRACE) {
                     return Token::new(start, self.index, TokenType::String, 0.0, buffer)
                        .with_kind(StringTokenKind::TemplateLiteralPart);
                }
                buffer.push(chars::DOLLAR);
                self.advance();
            } else if ch == chars::EOF {
                return Token::new(
                    self.index, 
                    self.index, 
                    TokenType::Error,
                    0.0,
                    format!("Lexer Error: Unterminated template literal at column {} in expression [{}]", self.index, self.input)
                );
            } else {
                buffer.push(ch);
                self.advance();
            }
        }

        Token::new(
            self.index, 
            self.index, 
            TokenType::Error,
            0.0,
            format!("Lexer Error: Unterminated template literal at column {} in expression [{}]", self.index, self.input)
        )
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
