//! Render3 Template Parsing Utilities for Tests
//!
//! Mirrors angular/packages/compiler/test/render3/view/util.ts

use angular_compiler::ml_parser::html_parser::HtmlParser;
use angular_compiler::ml_parser::lexer::TokenizeOptions;
use angular_compiler::ml_parser::html_whitespaces::{WhitespaceVisitor, visit_all_with_siblings_nodes};
// t is not used directly in this file
use angular_compiler::render3::r3_template_transform::{html_ast_to_render3_ast, Render3ParseResult, Render3ParseOptions};
use angular_compiler::render3::view::template::LEADING_TRIVIA_CHARS;
use angular_compiler::render3::view::template::{make_binding_parser, make_binding_parser_with_parser};
use angular_compiler::expression_parser::parser::Parser;

// Note: We can't use lazy_static with BindingParser because ElementSchemaRegistry is not Sync
// Instead, we create a new BindingParser each time in parse_r3

/// Options for parsing R3 templates in tests
#[derive(Debug, Clone)]
pub struct ParseR3Options {
    pub preserve_whitespaces: Option<bool>,
    pub leading_trivia_chars: Option<Vec<char>>,
    pub ignore_error: Option<bool>,
    pub selectorless_enabled: Option<bool>,
    pub collect_comment_nodes: Option<bool>,
}

impl Default for ParseR3Options {
    fn default() -> Self {
        ParseR3Options {
            preserve_whitespaces: None,
            leading_trivia_chars: None,
            ignore_error: None,
            selectorless_enabled: None,
            collect_comment_nodes: None,
        }
    }
}

/// Parse an HTML string to R3 AST nodes (equivalent to TypeScript parseR3)
pub fn parse_r3(input: &str, options: ParseR3Options) -> Render3ParseResult {
    let html_parser = HtmlParser::new();
    
    let mut tokenize_options = TokenizeOptions::default();
    tokenize_options.tokenize_expansion_forms = true;
    tokenize_options.leading_trivia_chars = options.leading_trivia_chars
        .or_else(|| Some(LEADING_TRIVIA_CHARS.to_vec()));
    tokenize_options.selectorless_enabled = options.selectorless_enabled.unwrap_or(false);
    
    let parse_result = html_parser.parse(input, "path://to/template", Some(tokenize_options));
    
    if parse_result.errors.len() > 0 && options.ignore_error != Some(true) {
        let msg = parse_result.errors.iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
            .join("\n");
        panic!("Parse errors: {}", msg);
    }
    
    // Process i18n metadata (simplified - would need I18nMetaVisitor in real implementation)
    let mut html_nodes = parse_result.root_nodes;
    
    // Handle whitespace preservation
    if options.preserve_whitespaces != Some(true) {
        html_nodes = visit_all_with_siblings_nodes(
            &mut WhitespaceVisitor::new(true, None, false), // preserve significant whitespace
            &html_nodes,
        );
    }
    
    // Create binding parser
    let selectorless_enabled = options.selectorless_enabled.unwrap_or(false);
    
    // We need to keep the parser alive for the duration of binding_parser
    let custom_parser = None;

    let mut binding_parser = if let Some(ref p) = custom_parser {
        make_binding_parser_with_parser(selectorless_enabled, p)
    } else {
        make_binding_parser(selectorless_enabled)
    };
    
    // Convert HTML AST to R3 AST
    let collect_comment_nodes = options.collect_comment_nodes.unwrap_or(false);
    let r3_options = Render3ParseOptions {
        collect_comment_nodes,
    };
    
    let r3_result = html_ast_to_render3_ast(&html_nodes, &mut binding_parser, &r3_options);
    
    if r3_result.errors.len() > 0 {
        let ignore_errors = options.ignore_error.unwrap_or(false);
        if !ignore_errors {
            let msg = r3_result.errors.iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
                .join("\n");
            panic!("R3 transform errors: {}", msg);
        }
    }
    
    r3_result
}

