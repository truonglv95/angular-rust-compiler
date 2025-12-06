/**
 * Angular Expression Parser - Rust Implementation
 *
 * Recursive descent parser for Angular template expressions
 * Mirrors packages/compiler/src/expression_parser/parser.ts (1796 lines)
 */

use super::ast::*;
use super::lexer::{Lexer, Token, TokenType};
use crate::error::{CompilerError, Result};
use crate::parse_util::{ParseError as ParseUtilError, ParseSourceSpan};

/// Interpolation piece (part of interpolation)
#[derive(Debug, Clone)]
pub struct InterpolationPiece {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Split interpolation result
#[derive(Debug, Clone)]
pub struct SplitInterpolation {
    pub strings: Vec<InterpolationPiece>,
    pub expressions: Vec<InterpolationPiece>,
    pub offsets: Vec<usize>,
}

/// Template binding parse result
#[derive(Debug, Clone)]
pub struct TemplateBindingParseResult {
    pub template_bindings: Vec<TemplateBinding>,
    pub warnings: Vec<String>,
    pub errors: Vec<ParseUtilError>,
}

/// Parse flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseFlags {
    None = 0,
    Action = 1 << 0,
}

/// Parser for Angular expressions
pub struct Parser {
    lexer: Lexer,
    supports_direct_pipe_references: bool,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            lexer: Lexer::new(),
            supports_direct_pipe_references: false,
        }
    }

    pub fn with_direct_pipe_references(mut self, enabled: bool) -> Self {
        self.supports_direct_pipe_references = enabled;
        self
    }

    /// Parse an action expression (event handler)
    pub fn parse_action(&self, input: &str, absolute_offset: usize) -> Result<AST> {
        let tokens = self.lexer.tokenize(input);
        let mut parse_ast = ParseAST::new(input, absolute_offset, tokens, ParseFlags::Action);
        parse_ast.parse_chain()
    }

    /// Parse a binding expression (property binding)
    pub fn parse_binding(&self, input: &str, absolute_offset: usize) -> Result<AST> {
        let tokens = self.lexer.tokenize(input);
        let mut parse_ast = ParseAST::new(input, absolute_offset, tokens, ParseFlags::None);
        parse_ast.parse_chain()
    }

    /// Parse simple binding (for host bindings)
    pub fn parse_simple_binding(&self, input: &str, absolute_offset: usize) -> Result<AST> {
        self.parse_binding(input, absolute_offset)
    }
}

/// Internal parser state
struct ParseAST {
    input: String,
    absolute_offset: usize,
    tokens: Vec<Token>,
    index: usize,
    flags: ParseFlags,
    rparens_expected: usize,
    rbrackets_expected: usize,
}

impl ParseAST {
    fn new(input: &str, absolute_offset: usize, tokens: Vec<Token>, flags: ParseFlags) -> Self {
        ParseAST {
            input: input.to_string(),
            absolute_offset,
            tokens,
            index: 0,
            flags,
            rparens_expected: 0,
            rbrackets_expected: 0,
        }
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.index + offset)
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn consume_optional_character(&mut self, code: char) -> bool {
        if let Some(token) = self.current() {
            if token.is_character(code) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn expect_character(&mut self, code: char) -> Result<()> {
        if self.consume_optional_character(code) {
            Ok(())
        } else {
            Err(CompilerError::ParseError {
                message: format!("Expected character '{}'", code),
            })
        }
    }

    /// Parse chain of expressions (e.g., `a; b; c`)
    fn parse_chain(&mut self) -> Result<AST> {
        let start = self.input_index();
        let mut expressions = vec![self.parse_conditional()?];

        while self.consume_optional_character(';') {
            if self.index >= self.tokens.len() {
                break;
            }
            expressions.push(self.parse_conditional()?);
        }

        if expressions.len() == 1 {
            Ok(expressions.into_iter().next().unwrap())
        } else {
            Ok(AST::Chain(Chain {
                span: self.span(start),
                source_span: self.source_span(start),
                expressions: expressions.into_iter().map(Box::new).collect(),
            }))
        }
    }

    /// Parse conditional/ternary expression (e.g., `a ? b : c`)
    fn parse_conditional(&mut self) -> Result<AST> {
        let start = self.input_index();
        let result = self.parse_pipe()?;

        // Check for ternary operator
        if let Some(token) = self.current() {
            if token.token_type == TokenType::Operator && token.str_value == "?" {
                self.advance();
                let true_exp = self.parse_pipe()?;
                self.expect_character(':')?;
                let false_exp = self.parse_conditional()?; // Right-associative

                return Ok(AST::Conditional(Conditional {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    condition: Box::new(result),
                    true_exp: Box::new(true_exp),
                    false_exp: Box::new(false_exp),
                }));
            }
        }

        Ok(result)
    }

    /// Parse pipe expression (e.g., `value | pipeName:arg`)
    fn parse_pipe(&mut self) -> Result<AST> {
        let start = self.input_index();
        let mut result = self.parse_logical_or()?;

        // Check for pipe operator (| is an Operator token)
        while let Some(token) = self.current() {
            if token.token_type == TokenType::Operator && token.str_value == "|" {
                self.advance();
                let name_start = self.input_index();

            let name = if let Some(token) = self.current() {
                if token.is_identifier() {
                    let n = token.str_value.clone();
                    self.advance();
                    n
                } else {
                    return Err(CompilerError::ParseError {
                        message: "Expected pipe name".to_string(),
                    });
                }
            } else {
                return Err(CompilerError::ParseError {
                    message: "Expected pipe name".to_string(),
                });
            };

            let name_span = self.source_span(name_start);
            let mut args = Vec::new();

            while self.consume_optional_character(':') {
                args.push(Box::new(self.parse_logical_or()?)); // Parse pipe args without creating recursion
            }

            result = AST::BindingPipe(BindingPipe {
                span: self.span(start),
                source_span: self.source_span(start),
                name_span,
                exp: Box::new(result),
                name,
                args,
                pipe_type: BindingPipeType::ReferencedByName,
            });
            } else {
                break;
            }
        }

        Ok(result)
    }


    /// Parse logical OR (||)
    fn parse_logical_or(&mut self) -> Result<AST> {
        let start = self.input_index();
        let mut result = self.parse_logical_and()?;

        while let Some(token) = self.current() {
            if token.token_type == TokenType::Operator && token.str_value == "||" {
                self.advance();
                let right = self.parse_logical_and()?;
                result = AST::Binary(Binary {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    operation: "||".to_string(),
                    left: Box::new(result),
                    right: Box::new(right),
                });
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Parse logical AND (&&)
    fn parse_logical_and(&mut self) -> Result<AST> {
        let start = self.input_index();
        let mut result = self.parse_equality()?;

        while let Some(token) = self.current() {
            if token.token_type == TokenType::Operator && token.str_value == "&&" {
                self.advance();
                let right = self.parse_equality()?;
                result = AST::Binary(Binary {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    operation: "&&".to_string(),
                    left: Box::new(result),
                    right: Box::new(right),
                });
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Parse equality operators (==, !=, ===, !==)
    fn parse_equality(&mut self) -> Result<AST> {
        let start = self.input_index();
        let mut result = self.parse_relational()?;

        while let Some(token) = self.current() {
            if token.token_type == TokenType::Operator {
                let op = &token.str_value;
                if matches!(op.as_str(), "==" | "!=" | "===" | "!==") {
                    let operator = op.clone();
                    self.advance();
                    let right = self.parse_relational()?;
                    result = AST::Binary(Binary {
                        span: self.span(start),
                        source_span: self.source_span(start),
                        operation: operator,
                        left: Box::new(result),
                        right: Box::new(right),
                    });
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Parse relational operators (<, >, <=, >=)
    fn parse_relational(&mut self) -> Result<AST> {
        let start = self.input_index();
        let mut result = self.parse_additive()?;

        while let Some(token) = self.current() {
            if token.token_type == TokenType::Operator {
                let op = &token.str_value;
                if matches!(op.as_str(), "<" | ">" | "<=" | ">=") {
                    let operator = op.clone();
                    self.advance();
                    let right = self.parse_additive()?;
                    result = AST::Binary(Binary {
                        span: self.span(start),
                        source_span: self.source_span(start),
                        operation: operator,
                        left: Box::new(result),
                        right: Box::new(right),
                    });
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Parse additive operators (+, -)
    fn parse_additive(&mut self) -> Result<AST> {
        let start = self.input_index();
        let mut result = self.parse_multiplicative()?;

        while let Some(token) = self.current() {
            if token.token_type == TokenType::Operator {
                let op = &token.str_value;
                if matches!(op.as_str(), "+" | "-") {
                    let operator = op.clone();
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    result = AST::Binary(Binary {
                        span: self.span(start),
                        source_span: self.source_span(start),
                        operation: operator,
                        left: Box::new(result),
                        right: Box::new(right),
                    });
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Parse multiplicative operators (*, /, %)
    fn parse_multiplicative(&mut self) -> Result<AST> {
        let start = self.input_index();
        let mut result = self.parse_prefix()?;

        while let Some(token) = self.current() {
            if token.token_type == TokenType::Operator {
                let op = &token.str_value;
                if matches!(op.as_str(), "*" | "/" | "%") {
                    let operator = op.clone();
                    self.advance();
                    let right = self.parse_prefix()?;
                    result = AST::Binary(Binary {
                        span: self.span(start),
                        source_span: self.source_span(start),
                        operation: operator,
                        left: Box::new(result),
                        right: Box::new(right),
                    });
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Parse prefix operators (!, -, +, typeof, void)
    fn parse_prefix(&mut self) -> Result<AST> {
        let start = self.input_index();

        if let Some(token) = self.current() {
            // Handle ! operator
            if token.token_type == TokenType::Operator && token.str_value == "!" {
                self.advance();
                let expr = self.parse_prefix()?;
                return Ok(AST::PrefixNot(PrefixNot {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    expression: Box::new(expr),
                }));
            }

            // Handle unary + and -
            if token.token_type == TokenType::Operator {
                if token.str_value == "+" || token.str_value == "-" {
                    let operator = token.str_value.clone();
                    self.advance();
                    let expr = self.parse_prefix()?;
                    return Ok(AST::Unary(Unary {
                        span: self.span(start),
                        source_span: self.source_span(start),
                        operator,
                        expr: Box::new(expr),
                    }));
                }
            }

            // Handle typeof
            if token.is_keyword() && token.str_value == "typeof" {
                self.advance();
                let expr = self.parse_prefix()?;
                return Ok(AST::Unary(Unary {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    operator: "typeof".to_string(),
                    expr: Box::new(expr),
                }));
            }

            // Handle void
            if token.is_keyword() && token.str_value == "void" {
                self.advance();
                let expr = self.parse_prefix()?;
                return Ok(AST::Unary(Unary {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    operator: "void".to_string(),
                    expr: Box::new(expr),
                }));
            }
        }

        self.parse_call_chain()
    }

    /// Parse call chain (handles property access, method calls, safe navigation)
    fn parse_call_chain(&mut self) -> Result<AST> {
        let start = self.input_index();
        let mut result = self.parse_primary()?;

        loop {
            if self.consume_optional_character('.') {
                // Property access or method call
                result = self.parse_access_member(result, start, false)?;
            } else if self.consume_optional_character('[') {
                // Keyed read: obj[key]
                self.rbrackets_expected += 1;
                let key = self.parse_pipe()?;
                self.rbrackets_expected -= 1;
                self.expect_character(']')?;

                result = AST::KeyedRead(KeyedRead {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    receiver: Box::new(result),
                    key: Box::new(key),
                });
            } else if self.consume_optional_character('(') {
                // Method call: fn(args)
                self.rparens_expected += 1;
                let args = self.parse_call_arguments()?;
                self.rparens_expected -= 1;
                self.expect_character(')')?;

                result = AST::Call(Call {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    receiver: Box::new(result),
                    args,
                    argument_span: self.source_span(start),
                });
            } else if self.consume_optional_character('!') {
                // Non-null assertion: expr!
                result = AST::NonNullAssert(NonNullAssert {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    expression: Box::new(result),
                });
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Parse primary expression (literals, identifiers, parentheses, arrays, objects)
    fn parse_primary(&mut self) -> Result<AST> {
        let start = self.input_index();

        if let Some(token) = self.current() {
            // Parenthesized expression
            if token.is_character('(') {
                self.consume_optional_character('(');
                self.rparens_expected += 1;
                let result = self.parse_pipe()?;
                self.rparens_expected -= 1;
                self.expect_character(')')?;
                return Ok(result);
            }

            // Array literal: [1, 2, 3]
            if token.is_character('[') {
                return self.parse_literal_array();
            }

            // Object literal: {a: 1, b: 2}
            if token.is_character('{') {
                return self.parse_literal_map();
            }

            // Keywords
            if token.is_keyword() {
                match token.str_value.as_str() {
                    "null" => {
                        self.advance();
                        return Ok(AST::LiteralPrimitive(LiteralPrimitive::null(
                            self.span(start),
                            self.source_span(start),
                        )));
                    }
                    "undefined" => {
                        self.advance();
                        return Ok(AST::LiteralPrimitive(LiteralPrimitive::undefined(
                            self.span(start),
                            self.source_span(start),
                        )));
                    }
                    "true" => {
                        self.advance();
                        return Ok(AST::LiteralPrimitive(LiteralPrimitive::boolean(
                            self.span(start),
                            self.source_span(start),
                            true,
                        )));
                    }
                    "false" => {
                        self.advance();
                        return Ok(AST::LiteralPrimitive(LiteralPrimitive::boolean(
                            self.span(start),
                            self.source_span(start),
                            false,
                        )));
                    }
                    "this" => {
                        self.advance();
                        return Ok(AST::ThisReceiver(ThisReceiver::new(
                            self.span(start),
                            self.source_span(start),
                        )));
                    }
                    _ => {}
                }
            }

            // Identifier
            if token.is_identifier() {
                let receiver = AST::ImplicitReceiver(ImplicitReceiver::new(
                    self.span(start),
                    self.source_span(start),
                ));
                return self.parse_access_member(receiver, start, false);
            }

            // Number
            if token.is_number() {
                let value = token.num_value;
                self.advance();
                return Ok(AST::LiteralPrimitive(LiteralPrimitive::number(
                    self.span(start),
                    self.source_span(start),
                    value,
                )));
            }

            // String
            if token.is_string() {
                let value = token.str_value.clone();
                self.advance();
                return Ok(AST::LiteralPrimitive(LiteralPrimitive::string(
                    self.span(start),
                    self.source_span(start),
                    value,
                )));
            }
        }

        // Empty expression or error
        Ok(AST::EmptyExpr(EmptyExpr::new(
            self.span(start),
            self.source_span(start),
        )))
    }

    /// Parse property access or method call
    fn parse_access_member(&mut self, receiver: AST, start: usize, is_safe: bool) -> Result<AST> {
        if let Some(token) = self.current() {
            if token.is_identifier() {
                let name = token.str_value.clone();
                let name_start = self.input_index();
                self.advance();

                let name_span = self.source_span(name_start);

                if is_safe {
                    Ok(AST::SafePropertyRead(SafePropertyRead {
                        span: self.span(start),
                        source_span: self.source_span(start),
                        name_span,
                        receiver: Box::new(receiver),
                        name,
                    }))
                } else {
                    Ok(AST::PropertyRead(PropertyRead {
                        span: self.span(start),
                        source_span: self.source_span(start),
                        name_span,
                        receiver: Box::new(receiver),
                        name,
                    }))
                }
            } else {
                Err(CompilerError::ParseError {
                    message: "Expected identifier for property access".to_string(),
                })
            }
        } else {
            Err(CompilerError::ParseError {
                message: "Unexpected end of expression".to_string(),
            })
        }
    }

    /// Parse array literal [1, 2, 3]
    fn parse_literal_array(&mut self) -> Result<AST> {
        let start = self.input_index();
        self.expect_character('[')?;
        self.rbrackets_expected += 1;

        let mut expressions = Vec::new();

        if !self.consume_optional_character(']') {
            loop {
                expressions.push(Box::new(self.parse_conditional()?));

                if self.consume_optional_character(',') {
                    if self.consume_optional_character(']') {
                        break;
                    }
                } else {
                    self.expect_character(']')?;
                    break;
                }
            }
        }

        self.rbrackets_expected -= 1;

        Ok(AST::LiteralArray(LiteralArray {
            span: self.span(start),
            source_span: self.source_span(start),
            expressions,
        }))
    }

    /// Parse object literal {a: 1, b: 2}
    fn parse_literal_map(&mut self) -> Result<AST> {
        let start = self.input_index();
        self.expect_character('{')?;

        let mut keys = Vec::new();
        let mut values = Vec::new();

        if !self.consume_optional_character('}') {
            loop {
                // Parse key
                let (key, quoted) = if let Some(token) = self.current() {
                    if token.is_identifier() {
                        let k = token.str_value.clone();
                        self.advance();
                        (k, false)
                    } else if token.is_string() {
                        let k = token.str_value.clone();
                        self.advance();
                        (k, true)
                    } else {
                        return Err(CompilerError::ParseError {
                            message: "Expected property name".to_string(),
                        });
                    }
                } else {
                    return Err(CompilerError::ParseError {
                        message: "Expected property name".to_string(),
                    });
                };

                keys.push(LiteralMapKey { key, quoted });

                self.expect_character(':')?;
                values.push(Box::new(self.parse_conditional()?));

                if self.consume_optional_character(',') {
                    if self.consume_optional_character('}') {
                        break;
                    }
                } else {
                    self.expect_character('}')?;
                    break;
                }
            }
        }

        Ok(AST::LiteralMap(LiteralMap {
            span: self.span(start),
            source_span: self.source_span(start),
            keys,
            values,
        }))
    }

    /// Parse call arguments (arg1, arg2, ...)
    fn parse_call_arguments(&mut self) -> Result<Vec<Box<AST>>> {
        let mut args = Vec::new();

        if let Some(token) = self.current() {
            if !token.is_character(')') {
                loop {
                    args.push(Box::new(self.parse_conditional()?));

                    if !self.consume_optional_character(',') {
                        break;
                    }
                }
            }
        }

        Ok(args)
    }

    // Helper methods
    fn input_index(&self) -> usize {
        self.current().map(|t| t.index).unwrap_or(self.input.len())
    }

    fn span(&self, start: usize) -> ParseSpan {
        ParseSpan::new(start, self.input_index())
    }

    fn source_span(&self, start: usize) -> AbsoluteSourceSpan {
        AbsoluteSourceSpan::new(
            self.absolute_offset + start,
            self.absolute_offset + self.input_index(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_expression() {
        let parser = Parser::new();
        let ast = parser.parse_binding("a + b", 0).unwrap();

        match ast {
            AST::Binary(bin) => {
                assert_eq!(bin.operation, "+");
            }
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn test_parse_property_access() {
        let parser = Parser::new();
        let ast = parser.parse_binding("user.name", 0).unwrap();

        match ast {
            AST::PropertyRead(prop) => {
                assert_eq!(prop.name, "name");
            }
            _ => panic!("Expected property read"),
        }
    }

    #[test]
    fn test_parse_ternary() {
        let parser = Parser::new();
        let ast = parser.parse_binding("a ? b : c", 0).unwrap();

        match ast {
            AST::Conditional(_) => {}
            _ => panic!("Expected conditional"),
        }
    }

    #[test]
    fn test_parse_array_literal() {
        let parser = Parser::new();
        let ast = parser.parse_binding("[1, 2, 3]", 0).unwrap();

        match ast {
            AST::LiteralArray(arr) => {
                assert_eq!(arr.expressions.len(), 3);
            }
            _ => panic!("Expected array literal"),
        }
    }

    #[test]
    fn test_parse_object_literal() {
        let parser = Parser::new();
        let ast = parser.parse_binding("{a: 1, b: 2}", 0).unwrap();

        match ast {
            AST::LiteralMap(map) => {
                assert_eq!(map.keys.len(), 2);
                assert_eq!(map.values.len(), 2);
            }
            _ => panic!("Expected object literal"),
        }
    }

    #[test]
    fn test_parse_pipe() {
        let parser = Parser::new();
        let ast = parser.parse_binding("value | uppercase", 0).unwrap();

        match ast {
            AST::BindingPipe(pipe) => {
                assert_eq!(pipe.name, "uppercase");
            }
            _ => panic!("Expected pipe"),
        }
    }
}
