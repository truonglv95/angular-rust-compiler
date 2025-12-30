#![allow(dead_code)]
#![allow(unused_imports)]

/**
 * ML Parser Test Utilities
 *
 * Helper functions for ML parser tests
 * Mirrors angular/packages/compiler/test/ml_parser/ast_spec_utils.ts
 * and helper functions from lexer_spec.ts
 */
pub mod serializer;

use angular_compiler::ml_parser::ast::*;
use angular_compiler::ml_parser::html_tags::get_html_tag_definition;
use angular_compiler::ml_parser::lexer::{tokenize, TokenizeOptions, TokenizeResult};
use angular_compiler::ml_parser::parser::ParseTreeResult;
use angular_compiler::ml_parser::tokens::{Token, TokenType};
use angular_compiler::parse_util::{ParseLocation, ParseSourceSpan};

pub use serializer::serialize_nodes;

#[allow(dead_code)]
#[allow(unused_imports)]

// Helper functions to extract token information
fn get_token_type(token: &Token) -> TokenType {
    match token {
        Token::TagOpenStart(_) => TokenType::TagOpenStart,
        Token::TagOpenEnd(_) => TokenType::TagOpenEnd,
        Token::TagOpenEndVoid(_) => TokenType::TagOpenEndVoid,
        Token::TagClose(_) => TokenType::TagClose,
        Token::IncompleteTagOpen(_) => TokenType::IncompleteTagOpen,
        Token::Text(_) => TokenType::Text,
        Token::Interpolation(_) => TokenType::Interpolation,
        Token::EncodedEntity(_) => TokenType::EncodedEntity,
        Token::CommentStart(_) => TokenType::CommentStart,
        Token::CommentEnd(_) => TokenType::CommentEnd,
        Token::CdataStart(_) => TokenType::CdataStart,
        Token::CdataEnd(_) => TokenType::CdataEnd,
        Token::AttrName(_) => TokenType::AttrName,
        Token::AttrQuote(_) => TokenType::AttrQuote,
        Token::AttrValueText(_) => TokenType::AttrValueText,
        Token::AttrValueInterpolation(_) => TokenType::AttrValueInterpolation,
        Token::DocType(_) => TokenType::DocType,
        Token::ExpansionFormStart(_) => TokenType::ExpansionFormStart,
        Token::ExpansionCaseValue(_) => TokenType::ExpansionCaseValue,
        Token::ExpansionCaseExpStart(_) => TokenType::ExpansionCaseExpStart,
        Token::ExpansionCaseExpEnd(_) => TokenType::ExpansionCaseExpEnd,
        Token::ExpansionFormEnd(_) => TokenType::ExpansionFormEnd,
        Token::Eof(_) => TokenType::Eof,
        Token::BlockParameter(_) => TokenType::BlockParameter,
        Token::BlockOpenStart(_) => TokenType::BlockOpenStart,
        Token::BlockOpenEnd(_) => TokenType::BlockOpenEnd,
        Token::BlockClose(_) => TokenType::BlockClose,
        Token::IncompleteBlockOpen(_) => TokenType::IncompleteBlockOpen,
        Token::LetStart(_) => TokenType::LetStart,
        Token::LetValue(_) => TokenType::LetValue,
        Token::LetEnd(_) => TokenType::LetEnd,
        Token::IncompleteLet(_) => TokenType::IncompleteLet,
        Token::ComponentOpenStart(_) => TokenType::ComponentOpenStart,
        Token::ComponentOpenEnd(_) => TokenType::ComponentOpenEnd,
        Token::ComponentOpenEndVoid(_) => TokenType::ComponentOpenEndVoid,
        Token::ComponentClose(_) => TokenType::ComponentClose,
        Token::IncompleteComponentOpen(_) => TokenType::IncompleteComponentOpen,
        Token::DirectiveName(_) => TokenType::DirectiveName,
        Token::DirectiveOpen(_) => TokenType::DirectiveOpen,
        Token::DirectiveClose(_) => TokenType::DirectiveClose,
        Token::RawText(_) => TokenType::RawText,
        Token::EscapableRawText(_) => TokenType::EscapableRawText,
    }
}

fn get_token_parts(token: &Token) -> &Vec<String> {
    match token {
        Token::TagOpenStart(t) => &t.parts,
        Token::TagOpenEnd(t) => &t.parts,
        Token::TagOpenEndVoid(t) => &t.parts,
        Token::TagClose(t) => &t.parts,
        Token::IncompleteTagOpen(t) => &t.parts,
        Token::Text(t) => &t.parts,
        Token::Interpolation(t) => &t.parts,
        Token::EncodedEntity(t) => &t.parts,
        Token::CommentStart(t) => &t.parts,
        Token::CommentEnd(t) => &t.parts,
        Token::CdataStart(t) => &t.parts,
        Token::CdataEnd(t) => &t.parts,
        Token::AttrName(t) => &t.parts,
        Token::AttrQuote(t) => &t.parts,
        Token::AttrValueText(t) => &t.parts,
        Token::AttrValueInterpolation(t) => &t.parts,
        Token::DocType(t) => &t.parts,
        Token::ExpansionFormStart(t) => &t.parts,
        Token::ExpansionCaseValue(t) => &t.parts,
        Token::ExpansionCaseExpStart(t) => &t.parts,
        Token::ExpansionCaseExpEnd(t) => &t.parts,
        Token::ExpansionFormEnd(t) => &t.parts,
        Token::Eof(t) => &t.parts,
        Token::BlockParameter(t) => &t.parts,
        Token::BlockOpenStart(t) => &t.parts,
        Token::BlockOpenEnd(t) => &t.parts,
        Token::BlockClose(t) => &t.parts,
        Token::IncompleteBlockOpen(t) => &t.parts,
        Token::LetStart(t) => &t.parts,
        Token::LetValue(t) => &t.parts,
        Token::LetEnd(t) => &t.parts,
        Token::IncompleteLet(t) => &t.parts,
        Token::ComponentOpenStart(t) => &t.parts,
        Token::ComponentOpenEnd(t) => &t.parts,
        Token::ComponentOpenEndVoid(t) => &t.parts,
        Token::ComponentClose(t) => &t.parts,
        Token::IncompleteComponentOpen(t) => &t.parts,
        Token::DirectiveName(t) => &t.parts,
        Token::DirectiveOpen(t) => &t.parts,
        Token::DirectiveClose(t) => &t.parts,
        Token::RawText(t) => &t.parts,
        Token::EscapableRawText(t) => &t.parts,
    }
}

fn get_token_source_span(token: &Token) -> &ParseSourceSpan {
    match token {
        Token::TagOpenStart(t) => &t.source_span,
        Token::TagOpenEnd(t) => &t.source_span,
        Token::TagOpenEndVoid(t) => &t.source_span,
        Token::TagClose(t) => &t.source_span,
        Token::IncompleteTagOpen(t) => &t.source_span,
        Token::Text(t) => &t.source_span,
        Token::Interpolation(t) => &t.source_span,
        Token::EncodedEntity(t) => &t.source_span,
        Token::CommentStart(t) => &t.source_span,
        Token::CommentEnd(t) => &t.source_span,
        Token::CdataStart(t) => &t.source_span,
        Token::CdataEnd(t) => &t.source_span,
        Token::AttrName(t) => &t.source_span,
        Token::AttrQuote(t) => &t.source_span,
        Token::AttrValueText(t) => &t.source_span,
        Token::AttrValueInterpolation(t) => &t.source_span,
        Token::DocType(t) => &t.source_span,
        Token::ExpansionFormStart(t) => &t.source_span,
        Token::ExpansionCaseValue(t) => &t.source_span,
        Token::ExpansionCaseExpStart(t) => &t.source_span,
        Token::ExpansionCaseExpEnd(t) => &t.source_span,
        Token::ExpansionFormEnd(t) => &t.source_span,
        Token::Eof(t) => &t.source_span,
        Token::BlockParameter(t) => &t.source_span,
        Token::BlockOpenStart(t) => &t.source_span,
        Token::BlockOpenEnd(t) => &t.source_span,
        Token::BlockClose(t) => &t.source_span,
        Token::IncompleteBlockOpen(t) => &t.source_span,
        Token::LetStart(t) => &t.source_span,
        Token::LetValue(t) => &t.source_span,
        Token::LetEnd(t) => &t.source_span,
        Token::IncompleteLet(t) => &t.source_span,
        Token::ComponentOpenStart(t) => &t.source_span,
        Token::ComponentOpenEnd(t) => &t.source_span,
        Token::ComponentOpenEndVoid(t) => &t.source_span,
        Token::ComponentClose(t) => &t.source_span,
        Token::IncompleteComponentOpen(t) => &t.source_span,
        Token::DirectiveName(t) => &t.source_span,
        Token::DirectiveOpen(t) => &t.source_span,
        Token::DirectiveClose(t) => &t.source_span,
        Token::RawText(t) => &t.source_span,
        Token::EscapableRawText(t) => &t.source_span,
    }
}

/// Humanize DOM parse result
pub fn humanize_dom(
    parse_result: &ParseTreeResult,
    add_source_span: bool,
) -> Result<Vec<Vec<String>>, String> {
    if !parse_result.errors.is_empty() {
        let error_string = parse_result
            .errors
            .iter()
            .map(|e| format!("{}", e.msg))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!("Unexpected parse errors:\n{}", error_string));
    }

    Ok(humanize_nodes(&parse_result.root_nodes, add_source_span))
}

/// Humanize DOM with source spans
pub fn humanize_dom_source_spans(
    parse_result: &ParseTreeResult,
) -> Result<Vec<Vec<String>>, String> {
    humanize_dom(parse_result, true)
}

/// Humanize AST nodes
pub fn humanize_nodes(nodes: &[Node], add_source_span: bool) -> Vec<Vec<String>> {
    let mut humanizer = Humanizer::new(add_source_span);
    for node in nodes {
        humanizer.visit(node);
    }
    humanizer.result
}

/// Humanize line and column from ParseLocation
pub fn humanize_line_column(location: &ParseLocation) -> String {
    format!("{}:{}", location.line, location.col)
}

struct Humanizer {
    result: Vec<Vec<String>>,
    el_depth: usize,
    include_source_span: bool,
}

impl Humanizer {
    fn new(include_source_span: bool) -> Self {
        Humanizer {
            result: Vec::new(),
            el_depth: 0,
            include_source_span,
        }
    }

    fn visit(&mut self, node: &Node) {
        match node {
            Node::Element(element) => self.visit_element(element),
            Node::Attribute(attribute) => self.visit_attribute(attribute),
            Node::Text(text) => self.visit_text(text),
            Node::Comment(comment) => self.visit_comment(comment),
            Node::Expansion(expansion) => self.visit_expansion(expansion),
            Node::ExpansionCase(case) => self.visit_expansion_case(case),
            Node::Block(block) => self.visit_block(block),
            Node::BlockParameter(param) => self.visit_block_parameter(param),
            Node::LetDeclaration(decl) => self.visit_let_declaration(decl),
            Node::Component(component) => self.visit_component(component),
            Node::Directive(directive) => self.visit_directive(directive),
        }
    }

    fn visit_element(&mut self, element: &Element) {
        let mut res = vec![
            "Element".to_string(),
            element.name.to_string(),
            self.el_depth.to_string(),
        ];

        if element.is_self_closing {
            res.push("#selfClosing".to_string());
        }

        if self.include_source_span {
            res.push(element.source_span.start.to_string());
            if let Some(ref end_span) = element.end_source_span {
                res.push(end_span.start.to_string());
            } else {
                res.push("null".to_string());
            }
        }

        self.result.push(res);
        self.el_depth += 1;

        for attr in &element.attrs {
            self.visit_attribute(attr);
        }

        for directive in &element.directives {
            self.visit_directive(directive);
        }

        for child in &element.children {
            self.visit(child);
        }

        self.el_depth -= 1;
    }

    fn visit_attribute(&mut self, attribute: &Attribute) {
        let res = vec![
            "Attribute".to_string(),
            attribute.name.to_string(),
            attribute.value.to_string(),
        ];
        self.result.push(res);
    }

    fn visit_text(&mut self, text: &Text) {
        let res = vec![
            "Text".to_string(),
            text.value.to_string(),
            self.el_depth.to_string(),
        ];
        self.result.push(res);
    }

    fn visit_comment(&mut self, comment: &Comment) {
        let value = comment
            .value
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default();
        let res = vec!["Comment".to_string(), value, self.el_depth.to_string()];
        self.result.push(res);
    }

    fn visit_expansion(&mut self, expansion: &Expansion) {
        let res = vec![
            "Expansion".to_string(),
            expansion.switch_value.to_string(),
            expansion.expansion_type.to_string(),
            self.el_depth.to_string(),
        ];
        self.result.push(res);
        self.el_depth += 1;

        for case in &expansion.cases {
            self.visit_expansion_case(case);
        }

        self.el_depth -= 1;
    }

    fn visit_expansion_case(&mut self, case: &ExpansionCase) {
        let res = vec![
            "ExpansionCase".to_string(),
            case.value.to_string(),
            self.el_depth.to_string(),
        ];
        self.result.push(res);
    }

    fn visit_block(&mut self, block: &Block) {
        let mut res = vec![
            "Block".to_string(),
            block.name.to_string(),
            self.el_depth.to_string(),
        ];

        if self.include_source_span {
            res.push(block.source_span.start.to_string());
            if let Some(ref end_span) = block.end_source_span {
                res.push(end_span.start.to_string());
            } else {
                res.push("null".to_string());
            }
        }

        self.result.push(res);
        self.el_depth += 1;

        for param in &block.parameters {
            self.visit_block_parameter(param);
        }

        for child in &block.children {
            self.visit(child);
        }

        self.el_depth -= 1;
    }

    fn visit_block_parameter(&mut self, parameter: &BlockParameter) {
        let res = vec![
            "BlockParameter".to_string(),
            parameter.expression.to_string(),
        ];
        self.result.push(res);
    }

    fn visit_let_declaration(&mut self, decl: &LetDeclaration) {
        let mut res = vec![
            "LetDeclaration".to_string(),
            decl.name.to_string(),
            decl.value.to_string(),
        ];

        if self.include_source_span {
            res.push(decl.name_span.start.to_string());
            res.push(decl.value_span.start.to_string());
        }

        self.result.push(res);
    }

    fn visit_component(&mut self, component: &Component) {
        let mut res = vec![
            "Component".to_string(),
            component.component_name.to_string(),
            self.el_depth.to_string(),
        ];

        if component.is_self_closing {
            res.push("#selfClosing".to_string());
        }

        if self.include_source_span {
            res.push(component.source_span.start.to_string());
            if let Some(ref end_span) = component.end_source_span {
                res.push(end_span.start.to_string());
            } else {
                res.push("null".to_string());
            }
        }

        self.result.push(res);
        self.el_depth += 1;

        for attr in &component.attrs {
            self.visit_attribute(attr);
        }

        for directive in &component.directives {
            self.visit_directive(directive);
        }

        for child in &component.children {
            self.visit(child);
        }

        self.el_depth -= 1;
    }

    fn visit_directive(&mut self, directive: &Directive) {
        let mut res = vec!["Directive".to_string(), directive.name.to_string()];

        if self.include_source_span {
            res.push(directive.source_span.start.to_string());
            if let Some(ref end_span) = directive.end_source_span {
                res.push(end_span.start.to_string());
            } else {
                res.push("null".to_string());
            }
        }

        self.result.push(res);
    }
}

/// Tokenize without errors (panics if errors found)
pub fn tokenize_without_errors(input: &str, options: TokenizeOptions) -> TokenizeResult {
    let result = tokenize(
        input.to_string(),
        "someUrl".to_string(),
        |name| {
            get_html_tag_definition(name)
                as &'static dyn angular_compiler::ml_parser::tags::TagDefinition
        },
        options,
    );

    if !result.errors.is_empty() {
        let error_string = result
            .errors
            .iter()
            .map(|e| format!("{}", e.msg))
            .collect::<Vec<_>>()
            .join("\n");
        panic!("Unexpected parse errors:\n{}", error_string);
    }

    result
}

/// Humanize token parts
pub fn humanize_parts(tokens: &[Token]) -> Vec<Vec<String>> {
    tokens
        .iter()
        .map(|token| {
            let mut parts = vec![to_screaming_snake_case(get_token_type(token))];
            parts.extend(get_token_parts(token).iter().map(|p| p.clone()));
            parts
        })
        .collect()
}

fn to_screaming_snake_case(t: TokenType) -> String {
    let s = format!("{:?}", t);
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_uppercase());
    }
    // Handle specific cases if any (e.g. valid acronyms that might be split weirdly, but usually fine)
    // E.g. "DocType" -> "DOC_TYPE" (Correct)
    // "IncompleteTagOpen" -> "INCOMPLETE_TAG_OPEN" (Correct)
    result
}

/// Tokenize and humanize parts
pub fn tokenize_and_humanize_parts(input: &str, options: TokenizeOptions) -> Vec<Vec<String>> {
    let result = tokenize_without_errors(input, options);
    humanize_parts(&result.tokens)
}

/// Tokenize and humanize source spans
pub fn tokenize_and_humanize_source_spans(
    input: &str,
    options: TokenizeOptions,
) -> Vec<Vec<String>> {
    let result = tokenize_without_errors(input, options);
    result
        .tokens
        .iter()
        .map(|token| {
            vec![
                to_screaming_snake_case(get_token_type(token)),
                get_token_source_span(token).to_string(),
            ]
        })
        .collect()
}

/// Tokenize and humanize line/column
pub fn tokenize_and_humanize_line_column(
    input: &str,
    options: TokenizeOptions,
) -> Vec<Vec<String>> {
    let result = tokenize_without_errors(input, options);
    result
        .tokens
        .iter()
        .map(|token| {
            vec![
                to_screaming_snake_case(get_token_type(token)),
                humanize_line_column(&get_token_source_span(token).start),
            ]
        })
        .collect()
}

/// Tokenize and humanize full start (with both start and fullStart)
/// Note: In Rust, ParseSourceSpan doesn't have full_start field, so we use start for both
pub fn tokenize_and_humanize_full_start(input: &str, options: TokenizeOptions) -> Vec<Vec<String>> {
    let result = tokenize_without_errors(input, options);
    result
        .tokens
        .iter()
        .map(|token| {
            let span = get_token_source_span(token);
            vec![
                format!("{:?}", get_token_type(token)),
                humanize_line_column(&span.start),
                humanize_line_column(&span.start), // TODO: Use full_start when available
            ]
        })
        .collect()
}

/// Tokenize and humanize errors
pub fn tokenize_and_humanize_errors(input: &str, options: TokenizeOptions) -> Vec<Vec<String>> {
    let result = tokenize(
        input.to_string(),
        "someUrl".to_string(),
        |name| {
            get_html_tag_definition(name)
                as &'static dyn angular_compiler::ml_parser::tags::TagDefinition
        },
        options,
    );
    result
        .errors
        .iter()
        .map(|e| vec![e.msg.clone(), humanize_line_column(&e.span.start)])
        .collect()
}

/// Tokenize ignoring errors (helper)
pub fn tokenize_ignoring_errors(input: &str, options: TokenizeOptions) -> TokenizeResult {
    tokenize(
        input.to_string(),
        "someUrl".to_string(),
        |name| {
            get_html_tag_definition(name)
                as &'static dyn angular_compiler::ml_parser::tags::TagDefinition
        },
        options,
    )
}

/// Tokenize and humanize parts ignoring errors
pub fn tokenize_and_humanize_parts_ignoring_errors(
    input: &str,
    options: TokenizeOptions,
) -> Vec<Vec<String>> {
    let result = tokenize_ignoring_errors(input, options);
    humanize_parts(&result.tokens)
}

/// Tokenize and humanize source spans ignoring errors
pub fn tokenize_and_humanize_source_spans_ignoring_errors(
    input: &str,
    options: TokenizeOptions,
) -> Vec<Vec<String>> {
    let result = tokenize_ignoring_errors(input, options);
    result
        .tokens
        .iter()
        .map(|token| {
            vec![
                to_screaming_snake_case(get_token_type(token)),
                get_token_source_span(token).to_string(),
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_humanize_line_column() {
        use angular_compiler::parse_util::{ParseLocation, ParseSourceFile};

        let file = ParseSourceFile::new("test".to_string(), "test.html".to_string());
        let location = ParseLocation::new(file, 0, 2, 5);

        assert_eq!(humanize_line_column(&location), "2:5");
    }
}
