//! Localize Utilities
//!
//! Corresponds to packages/compiler/src/render3/view/i18n/localize_utils.ts
//! Contains utilities for $localize tagged template literals

use std::collections::HashMap;

use crate::i18n::i18n_ast as i18n;
use crate::output::output_ast::{Expression, ReadVarExpr, Statement};
use crate::parse_util::{ParseLocation, ParseSourceSpan};

use super::icu_serializer::serialize_icu_node;
use super::util::format_i18n_placeholder_name;

/// A piece of a localized message - either literal or placeholder
#[derive(Debug, Clone)]
pub enum MessagePiece {
    Literal(LiteralPiece),
    Placeholder(PlaceholderPiece),
}

/// A literal piece of text in a localized message
#[derive(Debug, Clone)]
pub struct LiteralPiece {
    pub text: String,
    pub source_span: ParseSourceSpan,
}

impl LiteralPiece {
    pub fn new(text: String, source_span: ParseSourceSpan) -> Self {
        LiteralPiece { text, source_span }
    }
}

/// A placeholder piece in a localized message
#[derive(Debug, Clone)]
pub struct PlaceholderPiece {
    pub text: String,
    pub source_span: ParseSourceSpan,
    pub associated_message: Option<i18n::Message>,
}

impl PlaceholderPiece {
    pub fn new(
        text: String,
        source_span: ParseSourceSpan,
        associated_message: Option<i18n::Message>,
    ) -> Self {
        PlaceholderPiece {
            text,
            source_span,
            associated_message,
        }
    }
}

/// Localized string expression
#[derive(Debug, Clone)]
pub struct LocalizedString {
    pub message: i18n::Message,
    pub message_parts: Vec<LiteralPiece>,
    pub placeholders: Vec<PlaceholderPiece>,
    pub expressions: Vec<Expression>,
    pub source_span: ParseSourceSpan,
}

impl LocalizedString {
    pub fn new(
        message: i18n::Message,
        message_parts: Vec<LiteralPiece>,
        placeholders: Vec<PlaceholderPiece>,
        expressions: Vec<Expression>,
        source_span: ParseSourceSpan,
    ) -> Self {
        LocalizedString {
            message,
            message_parts,
            placeholders,
            expressions,
            source_span,
        }
    }
}

impl From<LocalizedString> for Expression {
    fn from(val: LocalizedString) -> Self {
        // For now, convert to a literal string as placeholder
        // TODO: Implement proper TaggedTemplateExpr
        Expression::Literal(crate::output::output_ast::LiteralExpr {
            value: crate::output::output_ast::LiteralValue::String(
                val.message_parts
                    .iter()
                    .map(|p| p.text.clone())
                    .collect::<Vec<_>>()
                    .join(""),
            ),
            type_: None,
            source_span: None,
        })
    }
}

/// Create $localize statements
pub fn create_localize_statements(
    variable: &ReadVarExpr,
    message: &i18n::Message,
    params: &HashMap<String, Expression>,
) -> Vec<Statement> {
    let (_message_parts, placeholders) = serialize_i18n_message_for_localize(message);
    let _source_span = get_source_span(message);

    let _expressions: Vec<Expression> = placeholders
        .iter()
        .map(|ph| {
            params.get(&ph.text).cloned().unwrap_or_else(|| {
                Expression::Literal(crate::output::output_ast::LiteralExpr {
                    value: crate::output::output_ast::LiteralValue::String(ph.text.clone()),
                    type_: None,
                    source_span: None,
                })
            })
        })
        .collect();

    // TODO: Implement LocalizedString in output_ast
    // For now, just create a placeholder statement using WriteVarExpr
    let empty_string_expr = Expression::Literal(crate::output::output_ast::LiteralExpr {
        value: crate::output::output_ast::LiteralValue::String(String::new()),
        type_: None,
        source_span: None,
    });
    let write_expr = Expression::WriteVar(crate::output::output_ast::WriteVarExpr {
        name: variable.name.clone(),
        value: Box::new(empty_string_expr),
        type_: None,
        source_span: None,
    });
    vec![Statement::Expression(
        crate::output::output_ast::ExpressionStatement {
            expr: Box::new(write_expr),
            source_span: None,
        },
    )]
}

/// Localize serializer visitor
pub struct LocalizeSerializerVisitor {
    placeholder_to_message: HashMap<String, Box<i18n::Message>>,
    pieces: Vec<MessagePiece>,
}

impl LocalizeSerializerVisitor {
    pub fn new(placeholder_to_message: HashMap<String, Box<i18n::Message>>) -> Self {
        LocalizeSerializerVisitor {
            placeholder_to_message,
            pieces: vec![],
        }
    }

    pub fn visit_text(&mut self, text: &i18n::Text) {
        if let Some(MessagePiece::Literal(ref mut piece)) = self.pieces.last_mut() {
            piece.text.push_str(&text.value);
        } else {
            let source_span =
                ParseSourceSpan::new(text.source_span.start.clone(), text.source_span.end.clone());
            self.pieces.push(MessagePiece::Literal(LiteralPiece::new(
                text.value.clone(),
                source_span,
            )));
        }
    }

    pub fn visit_container(&mut self, container: &i18n::Container) {
        for child in &container.children {
            self.visit_node(child);
        }
    }

    pub fn visit_icu(&mut self, icu: &i18n::Icu) {
        self.pieces.push(MessagePiece::Literal(LiteralPiece::new(
            serialize_icu_node(icu),
            icu.source_span.clone(),
        )));
    }

    pub fn visit_tag_placeholder(&mut self, ph: &i18n::TagPlaceholder) {
        self.pieces
            .push(MessagePiece::Placeholder(self.create_placeholder_piece(
                &ph.start_name,
                &ph.source_span,
                None,
            )));

        if !ph.is_void {
            for child in &ph.children {
                self.visit_node(child);
            }
            self.pieces
                .push(MessagePiece::Placeholder(self.create_placeholder_piece(
                    &ph.close_name,
                    &ph.source_span,
                    None,
                )));
        }
    }

    pub fn visit_placeholder(&mut self, ph: &i18n::Placeholder) {
        self.pieces
            .push(MessagePiece::Placeholder(self.create_placeholder_piece(
                &ph.name,
                &ph.source_span,
                None,
            )));
    }

    pub fn visit_block_placeholder(&mut self, ph: &i18n::BlockPlaceholder) {
        self.pieces
            .push(MessagePiece::Placeholder(self.create_placeholder_piece(
                &ph.start_name,
                &ph.source_span,
                None,
            )));

        for child in &ph.children {
            self.visit_node(child);
        }

        self.pieces
            .push(MessagePiece::Placeholder(self.create_placeholder_piece(
                &ph.close_name,
                &ph.source_span,
                None,
            )));
    }

    pub fn visit_icu_placeholder(&mut self, ph: &i18n::IcuPlaceholder) {
        // Dereference Box<Message> to Message (m is &Box<Message>, *m is Box<Message>, **m is Message)
        let associated_message = self
            .placeholder_to_message
            .get(&ph.name)
            .map(|m| (**m).clone());
        self.pieces
            .push(MessagePiece::Placeholder(self.create_placeholder_piece(
                &ph.name,
                &ph.source_span,
                associated_message,
            )));
    }

    pub fn visit_node(&mut self, node: &i18n::Node) {
        match node {
            i18n::Node::Text(text) => self.visit_text(text),
            i18n::Node::Container(container) => self.visit_container(container),
            i18n::Node::Icu(icu) => self.visit_icu(icu),
            i18n::Node::TagPlaceholder(ph) => self.visit_tag_placeholder(ph),
            i18n::Node::Placeholder(ph) => self.visit_placeholder(ph),
            i18n::Node::BlockPlaceholder(ph) => self.visit_block_placeholder(ph),
            i18n::Node::IcuPlaceholder(ph) => self.visit_icu_placeholder(ph),
        }
    }

    fn create_placeholder_piece(
        &self,
        name: &str,
        source_span: &ParseSourceSpan,
        associated_message: Option<i18n::Message>,
    ) -> PlaceholderPiece {
        PlaceholderPiece::new(
            format_i18n_placeholder_name(name, false),
            source_span.clone(),
            associated_message,
        )
    }

    pub fn into_pieces(self) -> Vec<MessagePiece> {
        self.pieces
    }
}

/// Serialize an i18n message for $localize
pub fn serialize_i18n_message_for_localize(
    message: &i18n::Message,
) -> (Vec<LiteralPiece>, Vec<PlaceholderPiece>) {
    let mut visitor = LocalizeSerializerVisitor::new(message.placeholder_to_message.clone());

    for node in &message.nodes {
        visitor.visit_node(node);
    }

    process_message_pieces(visitor.into_pieces())
}

fn get_source_span(message: &i18n::Message) -> ParseSourceSpan {
    if message.nodes.is_empty() {
        // Create an empty source span
        let empty_location =
            crate::parse_util::ParseLocation::from_source(String::new(), String::new(), 0, 0, 0);
        return ParseSourceSpan::new(empty_location.clone(), empty_location);
    }

    let start_node = &message.nodes[0];
    let end_node = &message.nodes[message.nodes.len() - 1];

    ParseSourceSpan::new(
        start_node.source_span().start.clone(),
        end_node.source_span().end.clone(),
    )
}

fn process_message_pieces(pieces: Vec<MessagePiece>) -> (Vec<LiteralPiece>, Vec<PlaceholderPiece>) {
    let mut message_parts: Vec<LiteralPiece> = vec![];
    let mut placeholders: Vec<PlaceholderPiece> = vec![];

    if pieces.is_empty() {
        return (message_parts, placeholders);
    }

    // If the first piece is a placeholder, add an empty message part
    if matches!(pieces.first(), Some(MessagePiece::Placeholder(_))) {
        if let Some(MessagePiece::Placeholder(ph)) = pieces.first() {
            message_parts.push(create_empty_message_part(&ph.source_span.start));
        }
    }

    for i in 0..pieces.len() {
        match &pieces[i] {
            MessagePiece::Literal(part) => {
                message_parts.push(part.clone());
            }
            MessagePiece::Placeholder(part) => {
                placeholders.push(part.clone());
                // Check if previous was also a placeholder
                if i > 0 && matches!(&pieces[i - 1], MessagePiece::Placeholder(_)) {
                    if let MessagePiece::Placeholder(prev) = &pieces[i - 1] {
                        message_parts.push(create_empty_message_part(&prev.source_span.end));
                    }
                }
            }
        }
    }

    // If the last piece is a placeholder, add a final empty message part
    if matches!(pieces.last(), Some(MessagePiece::Placeholder(_))) {
        if let Some(MessagePiece::Placeholder(ph)) = pieces.last() {
            message_parts.push(create_empty_message_part(&ph.source_span.end));
        }
    }

    (message_parts, placeholders)
}

fn create_empty_message_part(location: &ParseLocation) -> LiteralPiece {
    LiteralPiece::new(
        String::new(),
        ParseSourceSpan::new(location.clone(), location.clone()),
    )
}
