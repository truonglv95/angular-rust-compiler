//! Extractor Merger Module
//!
//! Corresponds to packages/compiler/src/i18n/extractor_merger.ts
//! Extracts translatable messages from HTML AST and merges translations

use crate::ml_parser::ast as html;
use crate::ml_parser::parser::ParseTreeResult;
use crate::parse_util::ParseError;
use crate::i18n::i18n_ast::{Message, Node};
use crate::i18n::i18n_parser::{create_i18n_message_factory, I18nMessageFactory};
use crate::i18n::translation_bundle::TranslationBundle;
use crate::ml_parser::defaults::DEFAULT_CONTAINER_BLOCKS;
use std::collections::{HashMap, HashSet};

const I18N_ATTR: &str = "i18n";
const I18N_ATTR_PREFIX: &str = "i18n-";
const MEANING_SEPARATOR: &str = "|";
const ID_SEPARATOR: &str = "@@";

/// Extract translatable messages from an html AST
pub fn extract_messages(
    nodes: &[html::Node],
    implicit_tags: &[String],
    implicit_attrs: &HashMap<String, Vec<String>>,
    preserve_significant_whitespace: bool,
) -> ExtractionResult {
    let visitor = Visitor::new(
        implicit_tags.to_vec(),
        implicit_attrs.clone(),
        preserve_significant_whitespace,
    );
    visitor.extract(nodes)
}

/// Merge translations into HTML AST
pub fn merge_translations(
    nodes: &[html::Node],
    translations: &mut TranslationBundle,
    implicit_tags: &[String],
    implicit_attrs: &HashMap<String, Vec<String>>,
) -> ParseTreeResult {
    let visitor = Visitor::new(
        implicit_tags.to_vec(),
        implicit_attrs.clone(),
        true,
    );
    visitor.merge(nodes, translations)
}

pub struct ExtractionResult {
    pub messages: Vec<Message>,
    pub errors: Vec<ParseError>,
}

impl ExtractionResult {
    pub fn new(messages: Vec<Message>, errors: Vec<ParseError>) -> Self {
        ExtractionResult { messages, errors }
    }
}

#[derive(Clone, Copy)]
enum VisitorMode {
    Extract,
    Merge,
}

/// This Visitor is used:
/// 1. to extract all the translatable strings from an html AST (see `extract()`),
/// 2. to replace the translatable strings with the actual translations (see `merge()`)
struct Visitor {
    implicit_tags: Vec<String>,
    implicit_attrs: HashMap<String, Vec<String>>,
    preserve_significant_whitespace: bool,

    // State variables
    depth: usize,
    in_i18n_node: bool,
    in_implicit_node: bool,
    in_i18n_block: bool,
    block_meaning_and_desc: String,
    block_children: Vec<html::Node>,
    block_start_depth: usize,
    in_icu: bool,
    msg_count_at_section_start: Option<usize>,
    errors: Vec<ParseError>,
    mode: VisitorMode,
    messages: Vec<Message>,
    translations: Option<TranslationBundle>,
    create_i18n_message: Option<Box<dyn I18nMessageFactory>>,
}

impl Visitor {
    fn new(
        implicit_tags: Vec<String>,
        implicit_attrs: HashMap<String, Vec<String>>,
        preserve_significant_whitespace: bool,
    ) -> Self {
        Visitor {
            implicit_tags,
            implicit_attrs,
            preserve_significant_whitespace,
            depth: 0,
            in_i18n_node: false,
            in_implicit_node: false,
            in_i18n_block: false,
            block_meaning_and_desc: String::new(),
            block_children: Vec::new(),
            block_start_depth: 0,
            in_icu: false,
            msg_count_at_section_start: None,
            errors: Vec::new(),
            mode: VisitorMode::Extract,
            messages: Vec::new(),
            translations: None,
            create_i18n_message: None,
        }
    }

    fn init(&mut self, mode: VisitorMode) {
        self.mode = mode;
        self.in_i18n_block = false;
        self.in_i18n_node = false;
        self.depth = 0;
        self.in_icu = false;
        self.msg_count_at_section_start = None;
        self.errors = Vec::new();
        self.messages = Vec::new();
        self.in_implicit_node = false;

        let container_blocks: HashSet<String> = DEFAULT_CONTAINER_BLOCKS
            .iter()
            .map(|s| s.to_string())
            .collect();

        self.create_i18n_message = Some(create_i18n_message_factory(
            container_blocks,
            !self.preserve_significant_whitespace,
            self.preserve_significant_whitespace,
        ));
    }

    /// Extracts the messages from the tree
    fn extract(mut self, nodes: &[html::Node]) -> ExtractionResult {
        self.init(VisitorMode::Extract);

        // TODO: Visit all nodes
        // for node in nodes {
        //     node.visit(&self, None);
        // }

        if self.in_i18n_block {
            // Report error: Unclosed block
        }

        ExtractionResult::new(self.messages, self.errors)
    }

    /// Returns a tree where all translatable nodes are translated
    fn merge(mut self, nodes: &[html::Node], translations: &mut TranslationBundle) -> ParseTreeResult {
        self.init(VisitorMode::Merge);
        // Note: TranslationBundle doesn't implement Clone, so we need to pass reference
        // This will need to be refactored to work with references instead
        // self.translations = Some(translations.clone());

        // TODO: Construct wrapper element and visit nodes
        // TODO: Return translated nodes

        ParseTreeResult {
            root_nodes: Vec::new(),
            errors: self.errors,
        }
    }

    fn is_in_translatable_section(&self) -> bool {
        self.msg_count_at_section_start.is_some()
    }

    fn open_translatable_section(&mut self, _node: &html::Node) {
        if self.is_in_translatable_section() {
            // Report error: Unexpected section start
        } else {
            self.msg_count_at_section_start = Some(self.messages.len());
        }
    }

    fn close_translatable_section(&mut self, _node: &html::Node, _direct_children: &[html::Node]) {
        if !self.is_in_translatable_section() {
            // Report error: Unexpected section end
            return;
        }

        // TODO: Handle single significant child case

        self.msg_count_at_section_start = None;
    }

    fn add_message(&mut self, ast: &[html::Node], msg_meta: Option<&str>) -> Option<Message> {
        // TODO: Check for empty messages, placeholder-only messages
        // TODO: Parse message metadata
        // TODO: Create i18n message
        // TODO: Push to messages

        None
    }
}

// TODO: Implement html::Visitor trait for Visitor
// This includes:
// - visitElement
// - visitComponent
// - visitAttribute
// - visitText
// - visitComment
// - visitExpansion
// - visitExpansionCase
// - visitBlock
// - visitBlockParameter
// - visitLetDeclaration
// - visitDirective

fn parse_message_meta(i18n: Option<&str>) -> MessageMeta {
    if i18n.is_none() {
        return MessageMeta {
            meaning: String::new(),
            description: String::new(),
            id: String::new(),
        };
    }

    let i18n = i18n.unwrap();
    let id_index = i18n.find(ID_SEPARATOR);
    let desc_index = i18n.find(MEANING_SEPARATOR);

    let (meaning_and_desc, id) = if let Some(idx) = id_index {
        (&i18n[..idx], &i18n[idx + 2..])
    } else {
        (i18n, "")
    };

    let (meaning, description) = if let Some(idx) = desc_index {
        (&meaning_and_desc[..idx], &meaning_and_desc[idx + 1..])
    } else {
        ("", meaning_and_desc)
    };

    MessageMeta {
        meaning: meaning.to_string(),
        description: description.to_string(),
        id: id.trim().to_string(),
    }
}

struct MessageMeta {
    meaning: String,
    description: String,
    id: String,
}

