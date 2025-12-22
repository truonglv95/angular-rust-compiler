//! Render3 Template Parser
//!
//! Corresponds to packages/compiler/src/render3/view/template.ts
//! Contains template parsing functionality

use crate::expression_parser::parser::Parser;
use crate::ml_parser::html_parser::HtmlParser;
use crate::ml_parser::lexer::{LexerRange, TokenizeOptions};
use crate::ml_parser::html_whitespaces::{WhitespaceVisitor, visit_all_with_siblings_nodes};
use crate::parse_util::ParseError;
use crate::render3::r3_ast as t;
use crate::render3::r3_template_transform::{html_ast_to_render3_ast, Render3ParseOptions};
use crate::schema::dom_element_schema_registry::DomElementSchemaRegistry;
use crate::template_parser::binding_parser::BindingParser;

/// Leading trivia characters
pub const LEADING_TRIVIA_CHARS: [char; 4] = [' ', '\n', '\r', '\t'];

/// Options that can be used to modify how a template is parsed.
#[derive(Debug, Clone, Default)]
pub struct ParseTemplateOptions {
    /// Include whitespace nodes in the parsed output.
    pub preserve_whitespaces: Option<bool>,
    /// Preserve original line endings instead of normalizing '\r\n' to '\n'.
    pub preserve_line_endings: Option<bool>,
    /// Preserve whitespace significant to rendering.
    pub preserve_significant_whitespace: Option<bool>,
    /// The start and end point of the text to parse within the `source` string.
    pub range: Option<LexerRange>,
    /// If this text is stored in a JavaScript string, deal with escape sequences.
    pub escaped_string: Option<bool>,
    /// An array of characters that should be considered as leading trivia.
    pub leading_trivia_chars: Option<Vec<char>>,
    /// Render `$localize` message ids with additional legacy message ids.
    pub enable_i18n_legacy_message_id_format: Option<bool>,
    /// Whether to normalize line-endings in ICU expressions.
    pub i18n_normalize_line_endings_in_icus: Option<bool>,
    /// Whether to always attempt HTML to R3 AST conversion despite errors.
    pub always_attempt_html_to_r3_ast_conversion: Option<bool>,
    /// Include HTML Comment nodes in the output.
    pub collect_comment_nodes: Option<bool>,
    /// Whether the @ block syntax is enabled.
    pub enable_block_syntax: Option<bool>,
    /// Whether the `@let` syntax is enabled.
    pub enable_let_syntax: Option<bool>,
    /// Whether the selectorless syntax is enabled.
    pub enable_selectorless: Option<bool>,
}

/// Information about the template which was extracted during parsing.
#[derive(Debug, Clone)]
pub struct ParsedTemplate {
    /// Include whitespace nodes in the parsed output.
    pub preserve_whitespaces: Option<bool>,
    /// Any errors from parsing the template.
    pub errors: Option<Vec<ParseError>>,
    /// The template AST, parsed from the template.
    pub nodes: Vec<t::R3Node>,
    /// Any styleUrls extracted from the metadata.
    pub style_urls: Vec<String>,
    /// Any inline styles extracted from the metadata.
    pub styles: Vec<String>,
    /// Any ng-content selectors extracted from the template.
    pub ng_content_selectors: Vec<String>,
    /// Any R3 Comment Nodes extracted from the template.
    pub comment_nodes: Option<Vec<t::Comment>>,
}

impl Default for ParsedTemplate {
    fn default() -> Self {
        ParsedTemplate {
            preserve_whitespaces: None,
            errors: None,
            nodes: vec![],
            style_urls: vec![],
            styles: vec![],
            ng_content_selectors: vec![],
            comment_nodes: None,
        }
    }
}

/// Parse a template into render3 `Node`s and additional metadata.
pub fn parse_template(
    template: &str,
    template_url: &str,
    options: ParseTemplateOptions,
) -> ParsedTemplate {
    let html_parser = HtmlParser::new();
    
    let mut tokenize_options = TokenizeOptions::default();
    tokenize_options.tokenize_expansion_forms = true;
    tokenize_options.leading_trivia_chars = options.leading_trivia_chars.clone()
        .or_else(|| Some(LEADING_TRIVIA_CHARS.to_vec()));
    tokenize_options.selectorless_enabled = options.enable_selectorless.unwrap_or(false);
    tokenize_options.escaped_string = options.escaped_string.unwrap_or(false);
    if let Some(enable_block_syntax) = options.enable_block_syntax {
        tokenize_options.tokenize_blocks = enable_block_syntax;
    }
    if let Some(enable_let_syntax) = options.enable_let_syntax {
        tokenize_options.tokenize_let = enable_let_syntax;
    }
    if let Some(range) = options.range {
        tokenize_options.range = Some(range);
    }
    
    let parse_result = html_parser.parse(template, template_url, Some(tokenize_options));
    
    // Process i18n metadata (simplified for now)
    let mut html_nodes = parse_result.root_nodes;
    
    // Handle whitespace preservation
    if options.preserve_whitespaces != Some(true) {
        let mut visitor = WhitespaceVisitor::new(
            options.preserve_significant_whitespace.unwrap_or(true), // Default to true based on view_util
            None,
            false
        );
        html_nodes = visit_all_with_siblings_nodes(&mut visitor, &html_nodes);
    }
    
    // Create binding parser
    let selectorless_enabled = options.enable_selectorless.unwrap_or(false);
    let mut binding_parser = make_binding_parser(selectorless_enabled);
    
    // Convert HTML AST to R3 AST
    let collect_comment_nodes = options.collect_comment_nodes.unwrap_or(false);
    let r3_options = Render3ParseOptions {
        collect_comment_nodes,
    };
    
    let r3_result = html_ast_to_render3_ast(&html_nodes, &mut binding_parser, &r3_options);
    
    let mut errors = parse_result.errors;
    errors.extend(r3_result.errors);
    
    ParsedTemplate {
        preserve_whitespaces: options.preserve_whitespaces,
        errors: if errors.is_empty() { None } else { Some(errors) },
        nodes: r3_result.nodes,
        style_urls: vec![], // TODO: Extract from metadata
        styles: vec![], // TODO: Extract from metadata
        ng_content_selectors: r3_result.ng_content_selectors,
        comment_nodes: if collect_comment_nodes {
            r3_result.comment_nodes
        } else {
            None
        },
    }
}

lazy_static::lazy_static! {
    static ref ELEMENT_REGISTRY: DomElementSchemaRegistry = DomElementSchemaRegistry::new();
    static ref EXPR_PARSER: Parser = Parser::new();
}

/// Construct a `BindingParser` with a default configuration.
pub fn make_binding_parser(_selectorless_enabled: bool) -> BindingParser<'static> {
    BindingParser::new(
        &EXPR_PARSER,
        &*ELEMENT_REGISTRY as &dyn crate::schema::element_schema_registry::ElementSchemaRegistry,
        vec![],
    )
}

/// Construct a `BindingParser` with a custom parser.
pub fn make_binding_parser_with_parser<'a>(_selectorless_enabled: bool, parser: &'a Parser) -> BindingParser<'a> {
    BindingParser::new(
        parser,
        &*ELEMENT_REGISTRY as &dyn crate::schema::element_schema_registry::ElementSchemaRegistry,
        vec![],
    )
}

