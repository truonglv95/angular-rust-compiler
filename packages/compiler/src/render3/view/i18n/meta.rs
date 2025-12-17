//! i18n Meta Visitor
//!
//! Corresponds to packages/compiler/src/render3/view/i18n/meta.ts
//! Contains i18n meta processing for templates

use std::collections::HashMap;

use crate::i18n::i18n_ast as i18n;
use crate::i18n::digest::{compute_digest, compute_decimal_digest, decimal_digest};
use crate::ml_parser::ast as html;
use crate::ml_parser::parser::ParseTreeResult;
/// JSDoc tag name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JSDocTagName {
    Desc,
    Meaning,
    Suppress,
}

/// A JSDoc tag
#[derive(Debug, Clone)]
pub struct JSDocTag {
    pub tag_name: JSDocTagName,
    pub text: String,
}

/// A JSDoc comment
#[derive(Debug, Clone)]
pub struct JSDocComment {
    pub tags: Vec<JSDocTag>,
}

impl JSDocComment {
    pub fn new(tags: Vec<JSDocTag>) -> Self {
        JSDocComment { tags }
    }
}
use crate::parse_util::ParseError;

use super::util::{has_i18n_attrs, I18N_ATTR, I18N_ATTR_PREFIX};

/// i18n metadata
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct I18nMeta {
    pub id: Option<String>,
    pub custom_id: Option<String>,
    pub legacy_ids: Option<Vec<String>>,
    pub description: Option<String>,
    pub meaning: Option<String>,
}

/// i18n separators for metadata
const I18N_MEANING_SEPARATOR: char = '|';
const I18N_ID_SEPARATOR: &str = "@@";

/// This visitor walks over HTML parse tree and converts information stored in
/// i18n-related attributes ("i18n" and "i18n-*") into i18n meta objects.
pub struct I18nMetaVisitor {
    /// Whether visited nodes contain i18n information
    pub has_i18n_meta: bool,
    errors: Vec<ParseError>,
    keep_i18n_attrs: bool,
    enable_i18n_legacy_message_id_format: bool,
    preserve_significant_whitespace: bool,
    retain_empty_tokens: bool,
}

impl I18nMetaVisitor {
    pub fn new(
        keep_i18n_attrs: bool,
        enable_i18n_legacy_message_id_format: bool,
        preserve_significant_whitespace: bool,
        retain_empty_tokens: bool,
    ) -> Self {
        I18nMetaVisitor {
            has_i18n_meta: false,
            errors: vec![],
            keep_i18n_attrs,
            enable_i18n_legacy_message_id_format,
            preserve_significant_whitespace,
            retain_empty_tokens,
        }
    }

    pub fn visit_all_with_errors(&mut self, nodes: Vec<html::Node>) -> ParseTreeResult {
        let result: Vec<html::Node> = nodes
            .into_iter()
            .map(|node| self.visit_node(node))
            .collect();
        ParseTreeResult::new(result, self.errors.clone())
    }

    fn visit_node(&mut self, node: html::Node) -> html::Node {
        match node {
            html::Node::Element(el) => self.visit_element(el),
            html::Node::Text(text) => html::Node::Text(self.visit_text(text)),
            html::Node::Comment(comment) => html::Node::Comment(self.visit_comment(comment)),
            html::Node::Expansion(expansion) => self.visit_expansion(expansion, None),
            html::Node::ExpansionCase(case) => html::Node::ExpansionCase(self.visit_expansion_case(case)),
            html::Node::Block(block) => self.visit_block(block),
            html::Node::BlockParameter(param) => html::Node::BlockParameter(self.visit_block_parameter(param)),
            html::Node::LetDeclaration(decl) => html::Node::LetDeclaration(self.visit_let_declaration(decl)),
            _ => node,
        }
    }

    fn visit_element(&mut self, mut element: html::Element) -> html::Node {
        self.visit_element_like(&mut element);
        html::Node::Element(element)
    }

    fn visit_element_like(&mut self, node: &mut html::Element) {
        if has_i18n_attrs(node) {
            self.has_i18n_meta = true;
            let mut attrs: Vec<html::Attribute> = vec![];
            let mut attrs_meta: HashMap<String, String> = HashMap::new();

            for attr in &node.attrs {
                if attr.name == I18N_ATTR {
                    // Root 'i18n' node attribute
                    // TODO: Generate i18n message
                } else if attr.name.starts_with(I18N_ATTR_PREFIX) {
                    // 'i18n-*' attributes
                    let name = attr.name[I18N_ATTR_PREFIX.len()..].to_string();
                    attrs_meta.insert(name, attr.value.clone());
                } else {
                    // non-i18n attributes
                    attrs.push(attr.clone());
                }
            }

            if !self.keep_i18n_attrs {
                node.attrs = attrs;
            }
        }

        // Visit children
        let children: Vec<html::Node> = node.children
            .drain(..)
            .map(|child| self.visit_node(child))
            .collect();
        node.children = children;
    }

    fn visit_text(&self, text: html::Text) -> html::Text {
        text
    }

    fn visit_comment(&self, comment: html::Comment) -> html::Comment {
        comment
    }

    fn visit_expansion(&mut self, expansion: html::Expansion, _current_message: Option<&i18n::Message>) -> html::Node {
        self.has_i18n_meta = true;
        // TODO: Generate i18n message for expansion
        html::Node::Expansion(expansion)
    }

    fn visit_expansion_case(&self, case: html::ExpansionCase) -> html::ExpansionCase {
        case
    }

    fn visit_block(&mut self, mut block: html::Block) -> html::Node {
        let children: Vec<html::Node> = block.children
            .drain(..)
            .map(|child| self.visit_node(child))
            .collect();
        block.children = children;
        html::Node::Block(block)
    }

    fn visit_block_parameter(&self, param: html::BlockParameter) -> html::BlockParameter {
        param
    }

    fn visit_let_declaration(&self, decl: html::LetDeclaration) -> html::LetDeclaration {
        decl
    }

    fn _parse_metadata(&self, meta: &str) -> I18nMeta {
        parse_i18n_meta(meta)
    }

    fn _set_message_id(&self, message: &mut i18n::Message, meta: &I18nMeta) {
        if message.id.is_empty() {
            message.id = meta.id.clone().unwrap_or_else(|| decimal_digest(message));
        }
    }

    fn _set_legacy_ids(&self, message: &mut i18n::Message, _meta: &I18nMeta) {
        if self.enable_i18n_legacy_message_id_format {
            message.legacy_ids = vec![
                compute_digest(message),
                compute_decimal_digest(message),
            ];
        }
    }

    fn _report_error(&mut self, node: &html::Node, msg: &str) {
        let source_span = get_node_source_span(node);
        self.errors.push(ParseError::new(source_span, msg.to_string()));
    }
}

/// Helper to get source_span from an html::Node enum
fn get_node_source_span(node: &html::Node) -> crate::parse_util::ParseSourceSpan {
    match node {
        html::Node::Text(n) => n.source_span.clone(),
        html::Node::Element(n) => n.source_span.clone(),
        html::Node::Comment(n) => n.source_span.clone(),
        html::Node::Expansion(n) => n.source_span.clone(),
        html::Node::ExpansionCase(n) => n.source_span.clone(),
        html::Node::Block(n) => n.source_span.clone(),
        html::Node::BlockParameter(n) => n.source_span.clone(),
        html::Node::LetDeclaration(n) => n.source_span.clone(),
        html::Node::Attribute(n) => n.source_span.clone(),
        html::Node::Component(n) => n.source_span.clone(),
        html::Node::Directive(n) => n.source_span.clone(),
    }
}

/// Parses i18n metas like:
///  - "@@id",
///  - "description[@@id]",
///  - "meaning|description[@@id]"
/// and returns an object with parsed output.
pub fn parse_i18n_meta(meta: &str) -> I18nMeta {
    let meta = meta.trim();
    if meta.is_empty() {
        return I18nMeta::default();
    }

    let (meaning_and_desc, custom_id) = if let Some(id_index) = meta.find(I18N_ID_SEPARATOR) {
        (
            meta[..id_index].to_string(),
            Some(meta[id_index + 2..].to_string()),
        )
    } else {
        (meta.to_string(), None)
    };

    let (meaning, description) = if let Some(desc_index) = meaning_and_desc.find(I18N_MEANING_SEPARATOR) {
        (
            Some(meaning_and_desc[..desc_index].to_string()),
            Some(meaning_and_desc[desc_index + 1..].to_string()),
        )
    } else {
        (None, Some(meaning_and_desc))
    };

    I18nMeta {
        id: None,
        custom_id,
        legacy_ids: None,
        description: description.filter(|s| !s.is_empty()),
        meaning: meaning.filter(|s| !s.is_empty()),
    }
}

/// Converts i18n meta information to JSDoc comment for Closure compiler.
pub fn i18n_meta_to_jsdoc(meta: &I18nMeta) -> JSDocComment {
    let mut tags: Vec<JSDocTag> = vec![];

    if let Some(ref description) = meta.description {
        tags.push(JSDocTag {
            tag_name: JSDocTagName::Desc,
            text: description.clone(),
        });
    } else {
        // Suppress the JSCompiler warning that a `@desc` was not given
        tags.push(JSDocTag {
            tag_name: JSDocTagName::Suppress,
            text: "{msgDescriptions}".to_string(),
        });
    }

    if let Some(ref meaning) = meta.meaning {
        tags.push(JSDocTag {
            tag_name: JSDocTagName::Meaning,
            text: meaning.clone(),
        });
    }

    JSDocComment::new(tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_i18n_meta_id_only() {
        let meta = parse_i18n_meta("@@myId");
        assert_eq!(meta.custom_id, Some("myId".to_string()));
        assert_eq!(meta.description, None);
        assert_eq!(meta.meaning, None);
    }

    #[test]
    fn test_parse_i18n_meta_description_only() {
        let meta = parse_i18n_meta("This is a description");
        assert_eq!(meta.custom_id, None);
        assert_eq!(meta.description, Some("This is a description".to_string()));
        assert_eq!(meta.meaning, None);
    }

    #[test]
    fn test_parse_i18n_meta_meaning_and_description() {
        let meta = parse_i18n_meta("greeting|Hello message");
        assert_eq!(meta.custom_id, None);
        assert_eq!(meta.description, Some("Hello message".to_string()));
        assert_eq!(meta.meaning, Some("greeting".to_string()));
    }

    #[test]
    fn test_parse_i18n_meta_full() {
        let meta = parse_i18n_meta("greeting|Hello message@@myId");
        assert_eq!(meta.custom_id, Some("myId".to_string()));
        assert_eq!(meta.description, Some("Hello message".to_string()));
        assert_eq!(meta.meaning, Some("greeting".to_string()));
    }
}

