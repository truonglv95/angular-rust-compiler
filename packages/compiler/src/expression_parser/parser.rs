/**
 * Angular Expression Parser - Rust Implementation
 *
 * Recursive descent parser for Angular template expressions
 * Mirrors packages/compiler/src/expression_parser/parser.ts (1796 lines)
 */
use super::ast::*;
use super::lexer::{Lexer, Token, TokenType};
use crate::error::{CompilerError, Result};
use crate::parse_util::{
    ParseError as ParseUtilError, ParseLocation, ParseSourceFile, ParseSourceSpan,
};

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

/// Result of parsing an action or binding expression
#[derive(Debug, Clone)]
pub struct ParseActionResult {
    pub ast: AST,
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
        let ast = parse_ast.parse_chain()?;

        Ok(ast)
    }

    /// Parse a binding expression (property binding)
    pub fn parse_binding(&self, input: &str, absolute_offset: usize) -> Result<AST> {
        let tokens = self.lexer.tokenize(input);
        let mut parse_ast = ParseAST::new(input, absolute_offset, tokens, ParseFlags::None);
        let ast = parse_ast.parse_chain()?;

        if parse_ast.index < parse_ast.tokens.len() {
            let token = &parse_ast.tokens[parse_ast.index];
            return Err(CompilerError::ParseError {
                message: format!("Unexpected token '{:?}'", token),
            });
        }

        if !parse_ast.errors.is_empty() {
            return Err(CompilerError::ParseError {
                message: parse_ast.errors[0].msg.clone(),
            });
        }
        Ok(ast)
    }

    /// Parse an action expression with error collection (for tests)
    pub fn parse_action_with_errors(
        &self,
        input: &str,
        absolute_offset: usize,
    ) -> ParseActionResult {
        let tokens = self.lexer.tokenize(input);
        let mut parse_ast = ParseAST::new(input, absolute_offset, tokens, ParseFlags::Action);
        match parse_ast.parse_chain() {
            Ok(ast) => {
                // Check for remaining tokens
                if parse_ast.index < parse_ast.tokens.len() {
                    let token = &parse_ast.tokens[parse_ast.index];
                    parse_ast.record_error(format!("Unexpected token '{:?}'", token));
                }
                ParseActionResult {
                    ast,
                    errors: parse_ast.errors,
                }
            }
            Err(e) => {
                parse_ast.record_error(format!("{:?}", e));
                ParseActionResult {
                    ast: AST::EmptyExpr(EmptyExpr::new(
                        ParseSpan::new(0, input.len()),
                        AbsoluteSourceSpan::new(absolute_offset, absolute_offset + input.len()),
                    )),
                    errors: parse_ast.errors,
                }
            }
        }
    }

    /// Parse a binding expression with error collection (for tests)
    pub fn parse_binding_with_errors(
        &self,
        input: &str,
        absolute_offset: usize,
    ) -> ParseActionResult {
        let tokens = self.lexer.tokenize(input);
        let mut parse_ast = ParseAST::new(input, absolute_offset, tokens, ParseFlags::None);
        match parse_ast.parse_chain() {
            Ok(ast) => {
                // Check for remaining tokens
                if parse_ast.index < parse_ast.tokens.len() {
                    let token = &parse_ast.tokens[parse_ast.index];
                    parse_ast.record_error(format!("Unexpected token '{:?}'", token));
                }
                ParseActionResult {
                    ast,
                    errors: parse_ast.errors,
                }
            }
            Err(e) => {
                parse_ast.record_error(format!("{:?}", e));
                ParseActionResult {
                    ast: AST::EmptyExpr(EmptyExpr::new(
                        ParseSpan::new(0, input.len()),
                        AbsoluteSourceSpan::new(absolute_offset, absolute_offset + input.len()),
                    )),
                    errors: parse_ast.errors,
                }
            }
        }
    }

    /// Parse simple binding (for host bindings)
    pub fn parse_simple_binding(&self, input: &str, absolute_offset: usize) -> Result<AST> {
        let ast = self.parse_binding(input, absolute_offset)?;
        self.check_simple_binding_restrictions(&ast)?;
        Ok(ast)
    }

    fn check_simple_binding_restrictions(&self, ast: &AST) -> Result<()> {
        if self.contains_pipe(ast) {
            return Err(CompilerError::ParseError {
                message: "Bindings cannot contain pipes".to_string(),
            });
        }
        Ok(())
    }

    fn contains_pipe(&self, ast: &AST) -> bool {
        match ast {
            AST::BindingPipe(_) => true,
            AST::Binary(b) => self.contains_pipe(&b.left) || self.contains_pipe(&b.right),
            AST::Chain(c) => c.expressions.iter().any(|e| self.contains_pipe(e)),
            AST::Conditional(c) => {
                self.contains_pipe(&c.condition)
                    || self.contains_pipe(&c.true_exp)
                    || self.contains_pipe(&c.false_exp)
            }
            AST::PropertyRead(p) => self.contains_pipe(&p.receiver),
            AST::SafePropertyRead(p) => self.contains_pipe(&p.receiver),
            AST::KeyedRead(k) => self.contains_pipe(&k.receiver) || self.contains_pipe(&k.key),
            AST::SafeKeyedRead(k) => self.contains_pipe(&k.receiver) || self.contains_pipe(&k.key),
            AST::LiteralArray(a) => a.expressions.iter().any(|e| self.contains_pipe(e)),
            AST::LiteralMap(m) => m.values.iter().any(|v| self.contains_pipe(v)),
            AST::Call(c) => {
                self.contains_pipe(&c.receiver) || c.args.iter().any(|a| self.contains_pipe(a))
            }
            AST::SafeCall(c) => {
                self.contains_pipe(&c.receiver) || c.args.iter().any(|a| self.contains_pipe(a))
            }
            AST::PrefixNot(p) => self.contains_pipe(&p.expression),
            AST::Unary(u) => self.contains_pipe(&u.expr),
            AST::NonNullAssert(n) => self.contains_pipe(&n.expression),
            AST::PropertyWrite(p) => {
                self.contains_pipe(&p.receiver) || self.contains_pipe(&p.value)
            }
            AST::KeyedWrite(k) => {
                self.contains_pipe(&k.receiver)
                    || self.contains_pipe(&k.key)
                    || self.contains_pipe(&k.value)
            }
            AST::TypeofExpression(t) => self.contains_pipe(&t.expression),
            AST::VoidExpression(v) => self.contains_pipe(&v.expression),
            AST::TemplateLiteral(t) => t.expressions.iter().any(|e| self.contains_pipe(e)),
            AST::TaggedTemplateLiteral(t) => {
                self.contains_pipe(&t.tag)
                    || t.template.expressions.iter().any(|e| self.contains_pipe(e))
            }
            AST::ParenthesizedExpression(p) => self.contains_pipe(&p.expression),
            AST::ImplicitReceiver(_)
            | AST::ThisReceiver(_)
            | AST::LiteralPrimitive(_)
            | AST::RegularExpressionLiteral(_)
            | AST::Interpolation(_)
            | AST::EmptyExpr(_) => false,
        }
    }

    /// Parse interpolation string (e.g., "Hello {{name}}!")
    pub fn parse_interpolation(
        &self,
        input: &str,
        absolute_offset: usize,
    ) -> Result<Interpolation> {
        let parts = self.split_interpolation(input, absolute_offset)?;
        let mut strings = Vec::new();
        let mut expressions = Vec::new();

        for piece in &parts.strings {
            strings.push(piece.text.clone());
        }

        for piece in &parts.expressions {
            let expr_text = piece.text.trim();
            if expr_text.is_empty() {
                return Err(CompilerError::ParseError {
                    message: "Blank expressions are not allowed in interpolated strings"
                        .to_string(),
                });
            }
            let tokens = self.lexer.tokenize(&piece.text);
            if tokens.is_empty() {
                return Err(CompilerError::ParseError {
                    message: "Blank expressions are not allowed in interpolated strings"
                        .to_string(),
                });
            }
            let mut parse_ast = ParseAST::new(&piece.text, piece.start, tokens, ParseFlags::None);
            let ast = parse_ast.parse_chain()?;
            expressions.push(Box::new(ast));

            if parse_ast.index < parse_ast.tokens.len() {
                return Err(CompilerError::ParseError {
                    message: format!(
                        "Unexpected token {:?} at column {} in expression [{}]",
                        parse_ast.tokens[parse_ast.index],
                        parse_ast.tokens[parse_ast.index].index,
                        piece.text
                    ),
                });
            }
        }

        // Calculate fullEnd: end of last expression piece + 2 (for }})
        // If there are expressions, use the last expression piece's end + 2
        // Otherwise, use input.len()
        let full_end = if let Some(last_expr_piece) = parts.expressions.last() {
            // last_expr_piece.end is the position of }}, so fullEnd = last_expr_piece.end + 2
            last_expr_piece.end + 2
        } else {
            absolute_offset + input.len()
        };

        Ok(Interpolation {
            span: ParseSpan::new(0, input.len()),
            source_span: AbsoluteSourceSpan::new(absolute_offset, full_end),
            strings,
            expressions,
        })
    }

    /// Split interpolation string into strings and expressions
    pub fn split_interpolation(
        &self,
        input: &str,
        absolute_offset: usize,
    ) -> Result<SplitInterpolation> {
        let mut strings = Vec::new();
        let mut expressions = Vec::new();
        let mut offsets = Vec::new();
        let mut current_pos = 0;
        let mut i = 0;

        while i < input.len() {
            if input[i..].starts_with("{{") {
                // Found start of interpolation
                strings.push(InterpolationPiece {
                    text: input[current_pos..i].to_string(),
                    start: absolute_offset + current_pos,
                    end: absolute_offset + i,
                });
                i += 2;
                let expr_start = i;
                let mut depth = 1;

                // Find matching }}
                let mut in_single_quote = false;
                let mut in_double_quote = false;
                let mut in_backtick = false;
                let mut in_comment = false;
                let mut loop_count = 0;

                while i < input.len() {
                    loop_count += 1;
                    if loop_count > 10000 {
                        panic!(
                            "Parser infinite loop detected at i={} in input: '{}'",
                            i, input
                        );
                    }
                    let char = input[i..].chars().next().unwrap();

                    if in_single_quote {
                        if char == '\\' {
                            i += 1; // Skip backslash
                            if i < input.len() {
                                if let Some(escaped) = input[i..].chars().next() {
                                    i += escaped.len_utf8();
                                }
                            }
                            continue;
                        }
                        if char == '\'' {
                            in_single_quote = false;
                        }
                        i += char.len_utf8();
                        continue;
                    }
                    if in_double_quote {
                        if char == '\\' {
                            i += 1; // Skip backslash
                            if i < input.len() {
                                if let Some(escaped) = input[i..].chars().next() {
                                    i += escaped.len_utf8();
                                }
                            }
                            continue;
                        }
                        if char == '"' {
                            in_double_quote = false;
                        }
                        i += char.len_utf8();
                        continue;
                    }
                    if in_backtick {
                        if char == '\\' {
                            i += 1; // Skip backslash
                            if i < input.len() {
                                if let Some(escaped) = input[i..].chars().next() {
                                    i += escaped.len_utf8();
                                }
                            }
                            continue;
                        }
                        if char == '`' {
                            in_backtick = false;
                        }
                        i += char.len_utf8();
                        continue;
                    }

                    // Check for comment start
                    if !in_comment && char == '/' {
                        if i + 1 < input.len() && input[i + 1..].starts_with('/') {
                            in_comment = true;
                            i += 2;
                            continue;
                        }
                    }

                    if in_comment {
                        if char == '\n' {
                            in_comment = false;
                        }
                    }

                    if !in_comment {
                        if char == '\'' {
                            in_single_quote = true;
                            i += 1;
                            continue;
                        }
                        if char == '"' {
                            in_double_quote = true;
                            i += 1;
                            continue;
                        }
                        if char == '`' {
                            in_backtick = true;
                            i += 1;
                            continue;
                        }
                    }

                    if depth > 0 {
                        if input[i..].starts_with("}}") {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                            i += 2;
                            continue;
                        } else if input[i..].starts_with("{{") {
                            if !in_comment {
                                depth += 1;
                            }
                            i += 2;
                            continue;
                        }
                    }

                    i += char.len_utf8();
                }

                if depth == 0 {
                    let expr_text = input[expr_start..i].to_string();
                    // Check if expression is blank (after trim) but don't trim the text itself
                    // This matches TypeScript behavior where text is not trimmed, only checked
                    if expr_text.trim().is_empty() {
                        return Err(CompilerError::ParseError {
                            message: "Blank expressions are not allowed in interpolated strings"
                                .to_string(),
                        });
                    }
                    expressions.push(InterpolationPiece {
                        text: expr_text,
                        start: absolute_offset + expr_start,
                        end: absolute_offset + i,
                    });
                    offsets.push(absolute_offset + expr_start);
                    i += 2; // Skip }}
                    current_pos = i;
                } else {
                    // Unclosed interpolation, treat as string
                    strings.pop();
                    break;
                }
            } else {
                if let Some(c) = input[i..].chars().next() {
                    i += c.len_utf8();
                } else {
                    i += 1;
                }
            }
        }

        strings.push(InterpolationPiece {
            text: input[current_pos..].to_string(),
            start: absolute_offset + current_pos,
            end: absolute_offset + input.len(),
        });

        // Ensure strings and expressions alternate correctly
        if expressions.is_empty() && !strings.is_empty() {
            // No interpolation, just a string
        } else if expressions.len() != strings.len() - 1 {
            return Err(CompilerError::ParseError {
                message: "Invalid interpolation format".to_string(),
            });
        }

        let res = SplitInterpolation {
            strings,
            expressions,
            offsets,
        };
        Ok(res)
    }

    /// Parse template bindings (e.g., "*ngFor=\"let item of items\"")
    pub fn parse_template_bindings(
        &self,
        input: &str,
        directive_name: Option<&str>,
        absolute_offset: usize,
    ) -> TemplateBindingParseResult {
        let tokens = self.lexer.tokenize(input);
        let mut parse_ast = ParseAST::new(input, absolute_offset, tokens, ParseFlags::None);
        parse_ast.parse_template_bindings(directive_name)
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
    errors: Vec<ParseUtilError>,
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
            errors: Vec::new(),
        }
    }

    fn record_error(&mut self, message: String) {
        let location = ParseLocation::from_source(
            self.input.clone(),
            "".to_string(),
            self.input_index(),
            0,
            0,
        );
        self.errors.push(ParseUtilError::new(
            ParseSourceSpan::new(location.clone(), location),
            message,
        ));
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    #[allow(dead_code)]
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

    fn consume_optional_operator(&mut self, op: &str) -> bool {
        if let Some(token) = self.current() {
            if token.is_operator(op) {
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

    /// Parse a chain of pipes
    fn parse_chain(&mut self) -> Result<AST> {
        let start = self.input_index();
        let mut expressions = vec![self.parse_pipe()?];

        while self.consume_optional_character(';') {
            if self.index >= self.tokens.len() {
                break;
            }
            expressions.push(self.parse_pipe()?);
        }

        if expressions.len() == 1 {
            Ok(expressions.into_iter().next().unwrap())
        } else {
            if self.flags != ParseFlags::Action {
                return Err(CompilerError::ParseError {
                    message: "Bindings cannot contain chained expressions".to_string(),
                });
            }
            Ok(AST::Chain(Chain {
                span: self.span(start),
                source_span: self.source_span(start),
                expressions: expressions.into_iter().map(Box::new).collect(),
            }))
        }
    }

    fn is_assignable(&self, ast: &AST) -> bool {
        match ast {
            AST::PropertyRead(_) | AST::KeyedRead(_) => true,
            _ => false,
        }
    }

    /// Parse assignment (e.g., a = b)
    fn parse_assignment(&mut self) -> Result<AST> {
        let start = self.input_index();
        let left = self.parse_conditional()?;

        if let Some(token) = self.current() {
            // println!("DEBUG: parse_assignment token: {:?}", token);
            if token.token_type == TokenType::Operator {
                let op = &token.str_value;
                if matches!(
                    op.as_str(),
                    "=" | "+="
                        | "-="
                        | "*="
                        | "/="
                        | "%="
                        | "&="
                        | "^="
                        | "|="
                        | "<<="
                        | ">>="
                        | ">>>="
                        | "**="
                        | "&&="
                        | "||="
                        | "??="
                ) {
                    if !self.is_assignable(&left) {
                        return Err(CompilerError::ParseError {
                            message: format!("Expression {:?} is not assignable", left),
                        });
                    }
                    let operation = op.clone();
                    self.advance();
                    let right = self.parse_assignment()?;
                    return Ok(AST::Binary(Binary {
                        span: self.span(start),
                        source_span: self.source_span(start),
                        operation,
                        left: Box::new(left),
                        right: Box::new(right),
                    }));
                }
            }
        }

        Ok(left)
    }

    /// Parse conditional/ternary expression (e.g., `a ? b : c`)
    fn parse_conditional(&mut self) -> Result<AST> {
        let start = self.input_index();
        let result = self.parse_logical_or()?;

        // Check for ternary operator
        if let Some(token) = self.current() {
            if token.token_type == TokenType::Operator && token.str_value == "?" {
                self.advance();
                let true_exp = self.parse_conditional()?;
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
        let mut result = self.parse_assignment()?;

        // Check for pipe operator (| is an Operator token)
        while let Some(token) = self.current() {
            if token.token_type == TokenType::Operator && token.str_value == "|" {
                if self.flags == ParseFlags::Action {
                    return Err(CompilerError::ParseError {
                        message: "Cannot have a pipe in an action expression".to_string(),
                    });
                }
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
                    args.push(Box::new(self.parse_assignment()?)); // Parse pipe args
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
        let mut result = self.parse_nullish_coalescing()?;

        while let Some(token) = self.current() {
            if token.token_type == TokenType::Operator && token.str_value == "&&" {
                self.advance();
                let right = self.parse_nullish_coalescing()?;
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

    /// Parse nullish coalescing (??)
    fn parse_nullish_coalescing(&mut self) -> Result<AST> {
        let start = self.input_index();
        let mut result = self.parse_equality()?;

        while let Some(token) = self.current() {
            if token.token_type == TokenType::Operator && token.str_value == "??" {
                self.advance();
                let right = self.parse_equality()?;
                result = AST::Binary(Binary {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    operation: "??".to_string(),
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
            } else if token.is_keyword_in() {
                let operator = "in".to_string();
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
        let mut result = self.parse_exponentiation()?;

        while let Some(token) = self.current() {
            if token.token_type == TokenType::Operator {
                let op = &token.str_value;
                if matches!(op.as_str(), "*" | "/" | "%") {
                    let operator = op.clone();
                    self.advance();
                    let right = self.parse_exponentiation()?;
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

    /// Parse exponentiation operator (**)
    fn parse_exponentiation(&mut self) -> Result<AST> {
        let start = self.input_index();
        let result = self.parse_prefix()?;

        if let Some(token) = self.current() {
            if token.token_type == TokenType::Operator && token.str_value == "**" {
                self.advance();
                let right = self.parse_exponentiation()?;
                return Ok(AST::Binary(Binary {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    operation: "**".to_string(),
                    left: Box::new(result),
                    right: Box::new(right),
                }));
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
                return Ok(AST::TypeofExpression(TypeofExpression {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    expression: Box::new(expr),
                }));
            }

            // Handle void
            if token.is_keyword() && token.str_value == "void" {
                self.advance();
                let expr = self.parse_prefix()?;
                return Ok(AST::VoidExpression(VoidExpression {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    expression: Box::new(expr),
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
            // Check for safe navigation operator (?.)
            if let Some(token) = self.current() {
                if token.token_type == TokenType::Operator && token.str_value == "?." {
                    self.advance();
                    // Safe property access or method call
                    if let Some(next_token) = self.current() {
                        if next_token.is_character('(') {
                            // Safe call: obj?.(args)
                            self.consume_optional_character('(');
                            self.rparens_expected += 1;
                            let (args, has_trailing_comma) = self.parse_call_arguments()?;
                            self.rparens_expected -= 1;
                            self.expect_character(')')?;

                            result = AST::SafeCall(SafeCall {
                                span: self.span(start),
                                source_span: self.source_span(start),
                                receiver: Box::new(result),
                                args,
                                argument_span: self.source_span(start),
                                has_trailing_comma,
                            });
                        } else if next_token.is_character('[') {
                            // Safe keyed read: obj?.[key]
                            self.consume_optional_character('[');
                            self.rbrackets_expected += 1;
                            let key = self.parse_pipe()?;
                            self.rbrackets_expected -= 1;
                            self.expect_character(']')?;

                            result = AST::SafeKeyedRead(SafeKeyedRead {
                                span: self.span(start),
                                source_span: self.source_span(start),
                                receiver: Box::new(result),
                                key: Box::new(key),
                            });
                        } else {
                            // Safe property access: obj?.prop
                            result = self.parse_access_member(result, start, true)?;
                        }
                    } else {
                        break;
                    }
                } else if self.consume_optional_character('.') {
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
                    let (args, has_trailing_comma) = self.parse_call_arguments()?;
                    self.rparens_expected -= 1;
                    self.expect_character(')')?;

                    result = AST::Call(Call {
                        span: self.span(start),
                        source_span: self.source_span(start),
                        receiver: Box::new(result),
                        args,
                        argument_span: self.source_span(start),
                        has_trailing_comma,
                    });
                } else if let Some(token) = self.current() {
                    if token.is_template_literal_part() || token.is_template_literal_end() {
                        let template =
                            if let AST::TemplateLiteral(t) = self.parse_template_literal()? {
                                t
                            } else {
                                // Should calculate span/error appropriately
                                return Err(CompilerError::ParseError {
                                    message: "Expected template literal".to_string(),
                                });
                            };

                        result = AST::TaggedTemplateLiteral(TaggedTemplateLiteral {
                            span: self.span(start),
                            source_span: self.source_span(start),
                            tag: Box::new(result),
                            template,
                        });
                    } else if token.is_operator("=") {
                        if self.flags != ParseFlags::Action {
                            if !matches!(result, AST::KeyedRead(_)) {
                                self.record_error(format!("Bindings cannot contain assignments"));
                            }
                        }
                        if !self.is_assignable(&result) {
                            return Err(CompilerError::ParseError {
                                message: format!("Expression {:?} is not assignable", result),
                            });
                        }
                        self.advance();
                        let value = self.parse_conditional()?;

                        result = match result {
                            AST::PropertyRead(r) => AST::PropertyWrite(PropertyWrite {
                                span: self.span(start),
                                source_span: self.source_span(start),
                                receiver: r.receiver,
                                name: r.name,
                                value: Box::new(value),
                            }),
                            AST::KeyedRead(r) => AST::KeyedWrite(KeyedWrite {
                                span: self.span(start),
                                source_span: self.source_span(start),
                                receiver: r.receiver,
                                key: r.key,
                                value: Box::new(value),
                            }),
                            _ => {
                                return Err(CompilerError::ParseError {
                                    message: format!("Expression {:?} is not assignable", result),
                                })
                            }
                        };
                    } else if token.is_operator("!") {
                        self.advance();
                        // Non-null assertion: expr!
                        result = AST::NonNullAssert(NonNullAssert {
                            span: self.span(start),
                            source_span: self.source_span(start),
                            expression: Box::new(result),
                        });
                    } else {
                        break;
                    }
                } else {
                    break;
                }
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
            // Template literal: `text ${expr} text`
            if token.is_template_literal_part() || token.is_template_literal_end() {
                return self.parse_template_literal();
            }

            // Regular expression literal: /pattern/flags
            if token.is_regexp_body() {
                return self.parse_regexp_literal();
            }

            // Parenthesized expression
            if token.is_character('(') {
                self.consume_optional_character('(');
                self.rparens_expected += 1;
                let expr = self.parse_pipe()?;
                self.rparens_expected -= 1;
                self.expect_character(')')?;
                return Ok(AST::ParenthesizedExpression(ParenthesizedExpression {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    expression: Box::new(expr),
                }));
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

            // Private Identifier
            if token.is_private_identifier() {
                return Err(CompilerError::ParseError {
                    message: format!(
                        "Private identifier '{}' is not supported on implicit receiver",
                        token.str_value
                    ),
                });
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

        if let Some(token) = self.current() {
            return Err(CompilerError::ParseError {
                message: format!("Unexpected token {}", token.str_value),
            });
        }

        // Empty expression (EOF)
        Ok(AST::EmptyExpr(EmptyExpr::new(
            self.span(start),
            self.source_span(start),
        )))
    }

    /// Parse property access or method call
    fn parse_access_member(&mut self, receiver: AST, start: usize, is_safe: bool) -> Result<AST> {
        if let Some(token) = self.current() {
            if token.is_identifier() || token.is_keyword() {
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
                // Record error for invalid member name (number, string, or other non-identifiers)
                if let Some(current_token) = self.current() {
                    self.record_error(format!(
                        "Unexpected {:?}, expected identifier or keyword",
                        current_token.token_type
                    ));
                } else {
                    self.record_error("expected identifier or keyword".to_string());
                }

                // Return PropertyRead with empty name for error recovery
                let name = "".to_string();
                let name_span = self.source_span(self.input_index()); // Empty span

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
            }
        } else {
            // End of input - record error and return recovery AST
            self.record_error("expected identifier or keyword".to_string());
            let name = "".to_string();
            let name_span = self.source_span(self.input_index());

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
                let key_start = self.input_index();
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

                keys.push(LiteralMapKey {
                    key: key.clone(),
                    quoted,
                });

                // Check for property shorthand - if no colon, use key as value
                if self.consume_optional_character(':') {
                    values.push(Box::new(self.parse_conditional()?));
                } else {
                    // Property shorthand: {a} is equivalent to {a: a}
                    let value = AST::PropertyRead(PropertyRead {
                        span: self.span(key_start),
                        source_span: self.source_span(key_start),
                        name_span: self.source_span(key_start),
                        receiver: Box::new(AST::ImplicitReceiver(ImplicitReceiver::new(
                            self.span(key_start),
                            self.source_span(key_start),
                        ))),
                        name: key,
                    });
                    values.push(Box::new(value));
                }

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
    fn parse_call_arguments(&mut self) -> Result<(Vec<Box<AST>>, bool)> {
        let mut args = Vec::new();
        let mut has_trailing_comma = false;

        if let Some(token) = self.current() {
            if !token.is_character(')') {
                loop {
                    if let Some(token) = self.current() {
                        if token.is_character(')') {
                            break;
                        }
                    }
                    args.push(Box::new(self.parse_pipe()?));

                    if self.consume_optional_character(',') {
                        // Check if next token is ')' - that means trailing comma
                        if let Some(token) = self.current() {
                            if token.is_character(')') {
                                has_trailing_comma = true;
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        Ok((args, has_trailing_comma))
    }

    // Helper methods
    fn input_index(&self) -> usize {
        self.current().map(|t| t.index).unwrap_or(self.input.len())
    }

    fn span(&self, start: usize) -> ParseSpan {
        // When we have consumed tokens, use the end of the last consumed token
        // as the end of the span. This avoids including trailing whitespace
        // which would be included if we used input_index() (start of next token).
        let end = if self.index > 0 && self.index <= self.tokens.len() {
            self.tokens[self.index - 1].end
        } else {
            self.input_index()
        };
        ParseSpan::new(start, end)
    }

    fn source_span(&self, start: usize) -> AbsoluteSourceSpan {
        let end = if self.index > 0 && self.index <= self.tokens.len() {
            self.tokens[self.index - 1].end
        } else {
            self.input_index()
        };
        AbsoluteSourceSpan::new(self.absolute_offset + start, self.absolute_offset + end)
    }

    /// Parse template literal: `text ${expr} text`
    fn parse_template_literal(&mut self) -> Result<AST> {
        let start = self.input_index();
        let mut elements = Vec::new();
        let mut expressions = Vec::new();

        // Parse first template part
        if let Some(token) = self.current() {
            if token.is_template_literal_part() || token.is_template_literal_end() {
                let text = token.str_value.clone();
                let span = self.span(start);
                let source_span = self.source_span(start);
                elements.push(TemplateLiteralElement {
                    span,
                    source_span,
                    text,
                });
                self.advance();
            }
        }

        // Parse interpolations and remaining parts
        while let Some(token) = self.current() {
            if token.is_template_literal_interpolation_start() {
                // Parse ${expression}
                self.advance(); // consume '${'
                let expr = self.parse_pipe()?;
                expressions.push(Box::new(expr));
                self.expect_character('}')?;

                // Parse next template part
                let is_end = if let Some(next_token) = self.current() {
                    if next_token.is_template_literal_part() || next_token.is_template_literal_end()
                    {
                        let text = next_token.str_value.clone();
                        let is_end = next_token.is_template_literal_end();
                        let span = self.span(self.input_index());
                        let source_span = self.source_span(self.input_index());
                        elements.push(TemplateLiteralElement {
                            span,
                            source_span,
                            text,
                        });
                        self.advance();
                        is_end
                    } else {
                        false
                    }
                } else {
                    false
                };
                if is_end {
                    break;
                }
            } else if token.is_template_literal_end() {
                break;
            } else {
                break;
            }
        }

        Ok(AST::TemplateLiteral(TemplateLiteral {
            span: self.span(start),
            source_span: self.source_span(start),
            elements,
            expressions,
        }))
    }

    /// Parse regular expression literal: /pattern/flags
    fn parse_regexp_literal(&mut self) -> Result<AST> {
        let start = self.input_index();

        if let Some(token) = self.current() {
            if token.is_regexp_body() {
                let body = token.str_value.clone();
                self.advance();

                let flags = if let Some(flags_token) = self.current() {
                    if flags_token.is_regexp_flags() {
                        let f = flags_token.str_value.clone();
                        self.advance();
                        Some(f)
                    } else {
                        None
                    }
                } else {
                    None
                };

                return Ok(AST::RegularExpressionLiteral(RegularExpressionLiteral {
                    span: self.span(start),
                    source_span: self.source_span(start),
                    body,
                    flags,
                }));
            }
        }

        Err(CompilerError::ParseError {
            message: "Expected regular expression literal".to_string(),
        })
    }

    /// Parse template bindings (e.g., "let item of items")
    fn parse_template_bindings(
        &mut self,
        directive_name: Option<&str>,
    ) -> TemplateBindingParseResult {
        let mut bindings = Vec::new();
        let warnings = Vec::new();
        let mut errors = Vec::new();

        // Skip whitespace
        while let Some(token) = self.current() {
            if token.token_type == TokenType::Character && token.str_value == " " {
                self.advance();
            } else {
                break;
            }
        }

        while self.index < self.tokens.len() {
            let mut key_is_var = false;
            let mut key_name: Option<String> = None;
            let mut key_span: Option<AbsoluteSourceSpan> = None;
            let mut value: Option<Box<AST>> = None;
            let mut name: Option<String> = None;
            // Track original key name for 'as' binding (before directive prefix transformation)
            let mut original_key_for_as: Option<String> = None;

            if let Some(token) = self.current() {
                if token.is_keyword_let() {
                    // Variable binding: let item [= value]
                    self.advance();
                    key_is_var = true;
                    if let Some(ident) = self.current() {
                        if ident.is_identifier() {
                            key_name = Some(ident.str_value.clone());
                            let start = self.input_index();
                            self.advance();
                            key_span = Some(self.source_span(start));
                        } else {
                            // Error: expected identifier
                            let span = self.source_span(self.input_index());
                            errors.push(ParseUtilError::new(
                                ParseSourceSpan::new(
                                    ParseLocation::from_source(
                                        self.input.clone(),
                                        "".to_string(),
                                        span.start,
                                        0,
                                        0,
                                    ),
                                    ParseLocation::from_source(
                                        self.input.clone(),
                                        "".to_string(),
                                        span.end,
                                        0,
                                        0,
                                    ),
                                ),
                                "Expected identifier after 'let'".to_string(),
                            ));
                        }
                    } else {
                        // Error
                    }

                    // Check for optional value assignment: = value
                    if self.consume_optional_operator("=") {
                        match self.parse_pipe() {
                            Ok(expr) => value = Some(Box::new(expr)),
                            Err(e) => {
                                let span = self.source_span(self.input_index());
                                errors.push(ParseUtilError::new(
                                    ParseSourceSpan::new(
                                        ParseLocation::from_source(
                                            self.input.clone(),
                                            "".to_string(),
                                            span.start,
                                            0,
                                            0,
                                        ),
                                        ParseLocation::from_source(
                                            self.input.clone(),
                                            "".to_string(),
                                            span.end,
                                            0,
                                            0,
                                        ),
                                    ),
                                    e.to_string(),
                                ));
                            }
                        }
                    }
                } else {
                    // Expression binding
                    // Could be:
                    // 1. directive_name [as alias] (if first binding)
                    // 2. key [assignments]
                    // 3. key [as alias]
                    // 4. SPECIAL: 'else templateRef' or 'then templateRef' for ngIf

                    // Check for 'else' or 'then' keywords - these are special in template bindings
                    // e.g., *ngIf="condition; else elseTemplate; then thenTemplate"
                    let is_else_or_then = self.current().map_or(false, |t| {
                        t.is_keyword() && (t.str_value == "else" || t.str_value == "then")
                    });

                    if is_else_or_then {
                        // Handle 'else templateRef' or 'then templateRef'
                        let keyword = self.current().unwrap().str_value.clone();
                        let keyword_span = self.source_span(self.input_index());
                        self.advance(); // consume 'else' or 'then'

                        // Parse the template reference name
                        if let Some(ref_token) = self.current() {
                            if ref_token.is_identifier() {
                                let ref_name = ref_token.str_value.clone();
                                let ref_span = self.source_span(self.input_index());
                                self.advance(); // consume template ref name

                                // Create PropertyRead for template reference
                                let template_ref = AST::PropertyRead(PropertyRead {
                                    span: ParseSpan::new(
                                        ref_span.start - self.absolute_offset,
                                        ref_span.end - self.absolute_offset,
                                    ),
                                    source_span: ref_span,
                                    name_span: self.source_span(self.input_index()),
                                    receiver: Box::new(AST::ImplicitReceiver(ImplicitReceiver {
                                        span: ParseSpan::new(0, 0),
                                        source_span: AbsoluteSourceSpan::new(0, 0),
                                    })),
                                    name: ref_name,
                                });

                                value = Some(Box::new(template_ref));
                                key_name = Some(keyword.clone());
                                key_span = Some(keyword_span);
                                original_key_for_as = Some(keyword.clone());

                                // Transform key with directive prefix (else -> ngIfElse)
                                if let Some(ref dir) = directive_name {
                                    let mut result = dir.to_string();
                                    let mut chars = keyword.chars();
                                    if let Some(first) = chars.next() {
                                        result.push(first.to_ascii_uppercase());
                                        result.push_str(chars.as_str());
                                    }
                                    key_name = Some(result);
                                }
                            }
                        }
                    } else {
                        // Normal expression binding parsing
                        let _start = self.input_index();
                        let start_token_index = self.index;
                        match self.parse_pipe() {
                            Ok(expr) => {
                                let mut is_key = false;

                                // If expr is a simple PropertyRead (identifier), check if we should treat it as a key.
                                // It is a key if:
                                // 1. It is followed by 'as' (e.g. exportAs) ? NO, 'as' binds the result.
                                // 2. It is followed by ':' ? YES.
                                // 3. It is followed by another expression (no comma/semicolon)? YES. (e.g. of items)

                                if let AST::PropertyRead(ref prop) = expr {
                                    if prop.receiver.is_implicit_receiver() {
                                        // Simple identifier
                                        if self.consume_optional_character(':') {
                                            is_key = true;
                                        } else if !bindings.is_empty() {
                                            // If not empty, assumed to be key (unless followed by 'as' which is handled as Alias?)
                                            // But even if followed by 'as': `ngIf as y`. `ngIf` is Key.
                                            is_key = true;
                                        }
                                    }
                                }

                                if is_key {
                                    // The expr was actually a key.
                                    if let AST::PropertyRead(prop) = expr {
                                        // Save original key for 'as' binding before any transformation
                                        original_key_for_as = Some(prop.name.clone());
                                        key_name = Some(prop.name.clone());
                                        key_span = Some(prop.source_span);
                                    }

                                    // Parse actual value
                                    if self.current().map_or(false, |t| {
                                        !t.is_keyword_as()
                                            && t.str_value != ";"
                                            && t.str_value != ","
                                    }) {
                                        match self.parse_pipe() {
                                            Ok(val_expr) => value = Some(Box::new(val_expr)),
                                            Err(e) => {
                                                let span = self.source_span(self.input_index());
                                                errors.push(ParseUtilError::new(
                                                    ParseSourceSpan::new(
                                                        ParseLocation::from_source(
                                                            self.input.clone(),
                                                            "".to_string(),
                                                            span.start,
                                                            0,
                                                            0,
                                                        ),
                                                        ParseLocation::from_source(
                                                            self.input.clone(),
                                                            "".to_string(),
                                                            span.end,
                                                            0,
                                                            0,
                                                        ),
                                                    ),
                                                    e.to_string(),
                                                ));
                                            }
                                        }
                                    }

                                    if let Some(ref dir) = directive_name {
                                        // Transform key: prefix + Capitalized
                                        // (original_key_for_as already saved above)
                                        if let Some(ref k) = key_name {
                                            let mut result = dir.to_string();
                                            let mut chars = k.chars();
                                            if let Some(first) = chars.next() {
                                                result.push(first.to_ascii_uppercase());
                                                result.push_str(chars.as_str());
                                            }
                                            key_name = Some(result);
                                        }
                                    }
                                } else {
                                    // It was the value.
                                    value = Some(Box::new(expr));
                                    if let Some(dir) = directive_name {
                                        if bindings.is_empty() {
                                            key_name = Some(dir.to_string());
                                            // source span? should be empty or cover value?
                                            // For "ngIf", source is "ngIf". But we don't have it in input.
                                            // We'll use a synthetic span or the directive name if available?
                                            // Actually let's use the directive name as source for now.
                                            // key_span?
                                        }
                                    }
                                }

                                // Check for 'as'
                                if let Some(token) = self.current() {
                                    if token.is_keyword_as() {
                                        self.advance();
                                        if let Some(ident) = self.current() {
                                            if ident.is_identifier() {
                                                name = Some(ident.str_value.clone());
                                                self.advance();
                                            } else {
                                                // Error
                                                let span = self.source_span(self.input_index());
                                                errors.push(ParseUtilError::new(
                                                    ParseSourceSpan::new(
                                                        ParseLocation::from_source(
                                                            self.input.clone(),
                                                            "".to_string(),
                                                            span.start,
                                                            0,
                                                            0,
                                                        ),
                                                        ParseLocation::from_source(
                                                            self.input.clone(),
                                                            "".to_string(),
                                                            span.end,
                                                            0,
                                                            0,
                                                        ),
                                                    ),
                                                    "Expected identifier after 'as'".to_string(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let span = self.source_span(self.input_index());
                                errors.push(ParseUtilError::new(
                                    ParseSourceSpan::new(
                                        ParseLocation::from_source(
                                            self.input.clone(),
                                            "".to_string(),
                                            span.start,
                                            0,
                                            0,
                                        ),
                                        ParseLocation::from_source(
                                            self.input.clone(),
                                            "".to_string(),
                                            span.end,
                                            0,
                                            0,
                                        ),
                                    ),
                                    e.to_string(),
                                ));
                                // Fix infinite loop: Advance if parsing failed to consume token
                                if self.index == start_token_index {
                                    self.advance();
                                }
                            }
                        }
                    } // close else block for normal expression binding
                }
            } else {
                break;
            }

            // Push bindings
            if key_is_var {
                if let Some(k) = key_name {
                    bindings.push(TemplateBinding::Variable(VariableBinding {
                        key: TemplateBindingIdentifier {
                            source: k,
                            span: key_span.unwrap_or(AbsoluteSourceSpan::new(0, 0)), // TODO: correct span
                        },
                        value: value,
                        span: AbsoluteSourceSpan::new(0, 0), // TODO: correct span
                    }));
                }
            } else if let Some(k) = key_name {
                let b_span = if let Some(v) = &value {
                    v.source_span().clone()
                } else {
                    AbsoluteSourceSpan::new(0, 0)
                };
                // If there is a value, OR if there is no alias ('as'), we emit the expression binding.
                // If we have `key as alias` (and no value), TS only emits the VariableBinding for alias.
                if value.is_some() || name.is_none() {
                    bindings.push(TemplateBinding::Expression(ExpressionBinding {
                        key: TemplateBindingIdentifier {
                            source: k,
                            span: key_span.unwrap_or(AbsoluteSourceSpan::new(0, 0)), // TODO
                        },
                        value: value,
                        span: b_span, // TOOD: correct binding span
                    }));
                }

                // If there was an 'as' binding
                if let Some(n) = name {
                    // For "key as alias", create a VariableBinding:
                    // - key = alias name (e.g., "i" for "index as i")
                    // - value = PropertyRead(original key) (e.g., "index" for "index as i")
                    //
                    // Use original_key_for_as which is the untransformed key name
                    // (before directive prefix like "ngFor" is added)
                    let value_name = original_key_for_as.unwrap_or_else(|| {
                        directive_name.map(|d| d.to_string()).unwrap_or_default()
                    });

                    let v = AST::PropertyRead(PropertyRead {
                        span: ParseSpan::new(0, 0),
                        source_span: AbsoluteSourceSpan::new(0, 0),
                        name_span: AbsoluteSourceSpan::new(0, 0),
                        receiver: Box::new(AST::ImplicitReceiver(ImplicitReceiver {
                            span: ParseSpan::new(0, 0),
                            source_span: AbsoluteSourceSpan::new(0, 0),
                        })),
                        name: value_name,
                    });
                    bindings.push(TemplateBinding::Variable(VariableBinding {
                        key: TemplateBindingIdentifier {
                            source: n,
                            span: AbsoluteSourceSpan::new(0, 0),
                        },
                        value: Some(Box::new(v)),
                        span: AbsoluteSourceSpan::new(0, 0),
                    }));
                }
            } else if let Some(_v) = value {
                // No key name? Should only happen if directive_name missing?
                // Or logic error.
            }

            // Consume optional separator
            if let Some(token) = self.current() {
                if token.str_value == ";" || token.str_value == "," {
                    self.advance();
                }
            }

            // Skip whitespace
            while let Some(token) = self.current() {
                if token.token_type == TokenType::Character && token.str_value == " " {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        if let Some(dir) = directive_name {
            let has_dir_binding = bindings.iter().any(|b| match b {
                TemplateBinding::Expression(e) => e.key.source == dir,
                _ => false,
            });

            if !has_dir_binding {
                bindings.insert(
                    0,
                    TemplateBinding::Expression(ExpressionBinding {
                        key: TemplateBindingIdentifier {
                            source: dir.to_string(),
                            span: AbsoluteSourceSpan::new(0, 0), // Synthetic span
                        },
                        value: None,
                        span: AbsoluteSourceSpan::new(0, 0),
                    }),
                );
            }
        }

        TemplateBindingParseResult {
            template_bindings: bindings,
            warnings,
            errors,
        }
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
