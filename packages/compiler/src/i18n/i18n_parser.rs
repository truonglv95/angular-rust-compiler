//! I18n Parser Module
//!
//! Corresponds to packages/compiler/src/i18n/i18n_parser.ts
//! Converts HTML AST to i18n messages

use crate::expression_parser::parser::Parser as ExpressionParser;
use crate::expression_parser::lexer::Lexer as ExpressionLexer;
use crate::ml_parser::ast as html;
use crate::parse_util::ParseSourceSpan;
use crate::i18n::i18n_ast::{self as i18n, Message, Node};
use crate::i18n::serializers::placeholder::PlaceholderRegistry;
use std::collections::{HashMap, HashSet};

/// Type for node visitor function
pub type VisitNodeFn = fn(&html::Node, &i18n::Node) -> i18n::Node;

/// Factory for creating i18n messages
pub trait I18nMessageFactory {
    fn create_message(
        &mut self,
        nodes: &[html::Node],
        meaning: Option<&str>,
        description: Option<&str>,
        custom_id: Option<&str>,
        visit_node_fn: Option<VisitNodeFn>,
    ) -> Message;
}

/// Returns a function converting html nodes to an i18n Message
pub fn create_i18n_message_factory(
    container_blocks: HashSet<String>,
    retain_empty_tokens: bool,
    preserve_expression_whitespace: bool,
) -> Box<dyn I18nMessageFactory> {
    Box::new(I18nVisitor::new(
        container_blocks,
        retain_empty_tokens,
        preserve_expression_whitespace,
    ))
}

struct I18nMessageVisitorContext {
    is_icu: bool,
    icu_depth: usize,
    placeholder_registry: PlaceholderRegistry,
    placeholder_to_content: HashMap<String, i18n::MessagePlaceholder>,
    placeholder_to_message: HashMap<String, Box<Message>>,
    visit_node_fn: Option<VisitNodeFn>,
}

fn noop_visit_node_fn(_html: &html::Node, i18n: &i18n::Node) -> i18n::Node {
    i18n.clone()
}

struct I18nVisitor {
    expression_parser: ExpressionParser,
    container_blocks: HashSet<String>,
    retain_empty_tokens: bool,
    preserve_expression_whitespace: bool,
}

impl I18nVisitor {
    fn new(
        container_blocks: HashSet<String>,
        retain_empty_tokens: bool,
        preserve_expression_whitespace: bool,
    ) -> Self {
        I18nVisitor {
            expression_parser: ExpressionParser::new(),
            container_blocks,
            retain_empty_tokens,
            preserve_expression_whitespace,
        }
    }

    fn to_i18n_message(
        &mut self,
        nodes: &[html::Node],
        meaning: &str,
        description: &str,
        custom_id: &str,
        visit_node_fn: Option<VisitNodeFn>,
    ) -> Message {
        let is_icu = nodes.len() == 1 && matches!(nodes[0], html::Node::Expansion(_));

        let context = I18nMessageVisitorContext {
            is_icu,
            icu_depth: 0,
            placeholder_registry: PlaceholderRegistry::new(),
            placeholder_to_content: HashMap::new(),
            placeholder_to_message: HashMap::new(),
            visit_node_fn,
        };

        // TODO: Implement HTML visitor pattern to convert nodes to i18n nodes
        let i18n_nodes = vec![]; // Placeholder

        Message::new(
            i18n_nodes,
            context.placeholder_to_content,
            context.placeholder_to_message,
            meaning.to_string(),
            description.to_string(),
            custom_id.to_string(),
        )
    }
}

impl I18nMessageFactory for I18nVisitor {
    fn create_message(
        &mut self,
        nodes: &[html::Node],
        meaning: Option<&str>,
        description: Option<&str>,
        custom_id: Option<&str>,
        visit_node_fn: Option<VisitNodeFn>,
    ) -> Message {
        self.to_i18n_message(
            nodes,
            meaning.unwrap_or(""),
            description.unwrap_or(""),
            custom_id.unwrap_or(""),
            visit_node_fn,
        )
    }
}

// TODO: Implement full HTML visitor for converting HTML AST to i18n AST
// This includes:
// - visitElement
// - visitComponent
// - visitAttribute
// - visitText
// - visitComment
// - visitExpansion (ICU)
// - visitExpansionCase
// - visitBlock
// - visitBlockParameter
// - visitLetDeclaration

