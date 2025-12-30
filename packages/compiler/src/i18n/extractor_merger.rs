//! Extractor Merger Module
//!
//! Corresponds to packages/compiler/src/i18n/extractor_merger.ts
//! Extracts translatable messages from HTML AST and merges translations

use crate::i18n::i18n_ast::{Message, Node as I18nNode};
use crate::i18n::i18n_parser::{create_i18n_message_factory, I18nMessageFactory};
use crate::i18n::translation_bundle::TranslationBundle;
use crate::ml_parser::ast as html;
use crate::ml_parser::ast::Visitor as HtmlVisitor;
use crate::ml_parser::defaults::DEFAULT_CONTAINER_BLOCKS;
use crate::ml_parser::parser::ParseTreeResult;
use crate::parse_util::{ParseError, ParseLocation, ParseSourceFile, ParseSourceSpan};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{HashMap, HashSet};

lazy_static! {
    static ref I18N_COMMENT_PREFIX_REGEXP: Regex = Regex::new(r"^i18n:?").unwrap();
}

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
    let mut visitor = Visitor::new(
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
    let mut visitor = Visitor::new(implicit_tags.to_vec(), implicit_attrs.clone(), true);
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

#[derive(Clone, Copy, PartialEq)]
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
    translations: Option<*mut TranslationBundle>,
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
    fn extract(&mut self, nodes: &[html::Node]) -> ExtractionResult {
        self.init(VisitorMode::Extract);

        let mut context: *mut () = std::ptr::null_mut();
        for node in nodes {
            html::visit_all(self, &[node.clone()], &mut context);
        }

        if self.in_i18n_block {
            if let Some(last_node) = nodes.last() {
                self.report_error(last_node, "Unclosed block");
            }
        }

        ExtractionResult::new(self.messages.clone(), self.errors.clone())
    }

    /// Returns a tree where all translatable nodes are translated
    fn merge(
        &mut self,
        nodes: &[html::Node],
        translations: &mut TranslationBundle,
    ) -> ParseTreeResult {
        self.init(VisitorMode::Merge);
        self.translations = Some(translations as *mut TranslationBundle);

        // Construct a single fake root element
        let start_loc = ParseLocation::from_source(String::new(), String::new(), 0, 0, 0);
        let end_loc = ParseLocation::from_source(String::new(), String::new(), 0, 0, 0);
        let source_span = ParseSourceSpan::new(start_loc.clone(), end_loc.clone());

        let wrapper = html::Element {
            name: "wrapper".into(),
            attrs: Vec::new(),
            directives: Vec::new(),
            children: nodes.to_vec(),
            is_self_closing: false,
            source_span: source_span.clone(),
            start_source_span: ParseSourceSpan::new(start_loc.clone(), start_loc),
            end_source_span: Some(ParseSourceSpan::new(end_loc.clone(), end_loc)),
            is_void: false,
            i18n: None,
        };

        let mut context: *mut () = std::ptr::null_mut();
        let translated_result = HtmlVisitor::visit_element(self, &wrapper, &mut context);

        if self.in_i18n_block {
            if let Some(last_node) = nodes.last() {
                self.report_error(last_node, "Unclosed block");
            }
        }

        let translated_element = translated_result
            .and_then(|r| r.downcast::<html::Element>().ok())
            .map(|e| *e)
            .unwrap_or(wrapper);

        ParseTreeResult {
            root_nodes: translated_element.children,
            errors: self.errors.clone(),
        }
    }

    fn is_in_translatable_section(&self) -> bool {
        self.msg_count_at_section_start.is_some()
    }

    fn open_translatable_section(&mut self, node: &html::Node) {
        if self.is_in_translatable_section() {
            self.report_error(node, "Unexpected section start");
        } else {
            self.msg_count_at_section_start = Some(self.messages.len());
        }
    }

    fn close_translatable_section(&mut self, node: &html::Node, direct_children: &[html::Node]) {
        if !self.is_in_translatable_section() {
            self.report_error(node, "Unexpected section end");
            return;
        }

        let start_index = self.msg_count_at_section_start.unwrap();
        let significant_children: usize = direct_children
            .iter()
            .filter(|n| !matches!(n, html::Node::Comment(_)))
            .count();

        if significant_children == 1 {
            // Remove the last non-text message
            for i in (start_index..self.messages.len()).rev() {
                let ast = &self.messages[i].nodes;
                if !(ast.len() == 1 && matches!(ast[0], I18nNode::Text(_))) {
                    self.messages.remove(i);
                    break;
                }
            }
        }

        self.msg_count_at_section_start = None;
    }

    fn add_message(&mut self, ast: &[html::Node], msg_meta: Option<&str>) -> Option<Message> {
        if ast.is_empty() {
            return None;
        }

        let meta = parse_message_meta(msg_meta);
        let factory = self.create_i18n_message.as_mut()?;
        let message = factory.create_message(
            ast,
            if meta.meaning.is_empty() {
                None
            } else {
                Some(&meta.meaning)
            },
            if meta.description.is_empty() {
                None
            } else {
                Some(&meta.description)
            },
            if meta.id.is_empty() {
                None
            } else {
                Some(&meta.id)
            },
            None,
        );
        self.messages.push(message.clone());
        Some(message)
    }

    fn translate_message(&mut self, _el: &html::Node, message: &Message) -> Vec<html::Node> {
        if self.mode == VisitorMode::Merge {
            if let Some(translations_ptr) = self.translations {
                unsafe {
                    if let Ok(nodes) = (*translations_ptr).get(message) {
                        return nodes;
                    }
                }
            }
        }
        Vec::new()
    }

    fn may_be_add_block_children(&mut self, node: &html::Node) {
        if self.in_i18n_block && !self.in_icu && self.depth == self.block_start_depth {
            self.block_children.push(node.clone());
        }
    }

    fn report_error(&mut self, node: &html::Node, msg: &str) {
        let span = match node {
            html::Node::Element(e) => &e.source_span,
            html::Node::Component(c) => &c.source_span,
            html::Node::Text(t) => &t.source_span,
            html::Node::Comment(c) => &c.source_span,
            html::Node::Expansion(e) => &e.source_span,
            html::Node::ExpansionCase(e) => &e.source_span,
            html::Node::Block(b) => &b.source_span,
            html::Node::Attribute(a) => &a.source_span,
            html::Node::Directive(d) => &d.source_span,
            html::Node::BlockParameter(b) => &b.source_span,
            html::Node::LetDeclaration(l) => &l.source_span,
        };
        self.errors
            .push(ParseError::new(span.clone(), msg.to_string()));
    }

    // Helper to check if element has i18n attribute
    fn has_i18n_attr(el: &html::Element) -> bool {
        el.attrs.iter().any(|attr| attr.name.as_ref() == I18N_ATTR)
    }

    fn has_i18n_attr_component(comp: &html::Component) -> bool {
        comp.attrs
            .iter()
            .any(|attr| attr.name.as_ref() == I18N_ATTR)
    }

    // Helper to get i18n meta value
    fn get_i18n_meta_value(el: &html::Element) -> String {
        el.attrs
            .iter()
            .find(|attr| attr.name.as_ref() == I18N_ATTR)
            .map(|a| a.value.to_string())
            .unwrap_or_default()
    }

    fn get_i18n_meta_value_component(comp: &html::Component) -> String {
        comp.attrs
            .iter()
            .find(|attr| attr.name.as_ref() == I18N_ATTR)
            .map(|a| a.value.to_string())
            .unwrap_or_default()
    }
}

impl html::Visitor for Visitor {
    fn visit_element(
        &mut self,
        element: &html::Element,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        self.may_be_add_block_children(&html::Node::Element(element.clone()));
        self.depth += 1;
        let was_in_i18n_node = self.in_i18n_node;
        let was_in_implicit_node = self.in_implicit_node;
        let mut child_nodes: Vec<html::Node> = Vec::new();
        let mut translated_child_nodes: Option<Vec<html::Node>> = None;

        let node_name = element.name.clone();
        let has_i18n_attr = Self::has_i18n_attr(element);
        let i18n_meta = Self::get_i18n_meta_value(element);
        let is_implicit = self.implicit_tags.iter().any(|t| t == node_name.as_ref())
            && !self.in_icu
            && !self.is_in_translatable_section();
        let is_top_level_implicit = !was_in_implicit_node && is_implicit;
        self.in_implicit_node = was_in_implicit_node || is_implicit;

        if !self.is_in_translatable_section() && !self.in_icu {
            if has_i18n_attr || is_top_level_implicit {
                self.in_i18n_node = true;
                if let Some(message) = self.add_message(&element.children, Some(&i18n_meta)) {
                    translated_child_nodes = Some(
                        self.translate_message(&html::Node::Element(element.clone()), &message),
                    );
                }
            }

            if self.mode == VisitorMode::Extract {
                let is_translatable = has_i18n_attr || is_top_level_implicit;
                if is_translatable {
                    self.open_translatable_section(&html::Node::Element(element.clone()));
                }
                let mut context: *mut () = std::ptr::null_mut();
                html::visit_all(self, &element.children, &mut context);
                if is_translatable {
                    self.close_translatable_section(
                        &html::Node::Element(element.clone()),
                        &element.children,
                    );
                }
            }
        } else {
            if has_i18n_attr || is_top_level_implicit {
                self.report_error(
                    &html::Node::Element(element.clone()),
                    "Could not mark an element as translatable inside a translatable section",
                );
            }

            if self.mode == VisitorMode::Extract {
                let mut context: *mut () = std::ptr::null_mut();
                html::visit_all(self, &element.children, &mut context);
            }
        }

        if self.mode == VisitorMode::Merge {
            let visit_nodes = translated_child_nodes.as_ref().unwrap_or(&element.children);
            let mut context: *mut () = std::ptr::null_mut();
            for child in visit_nodes {
                let visit_results = html::visit_all(self, &[child.clone()], &mut context);
                if let Some(visited) = visit_results.into_iter().next() {
                    if !self.is_in_translatable_section() {
                        if let Some(node_ref) = visited.downcast_ref::<html::Node>() {
                            child_nodes.push(node_ref.clone());
                        }
                    }
                }
            }
        }

        // Visit attributes (simplified - just mark messages for extraction)
        if self.mode == VisitorMode::Extract {
            let implicit_attr_names = self
                .implicit_attrs
                .get(element.name.as_ref())
                .cloned()
                .unwrap_or_default();

            for attr in &element.attrs {
                if implicit_attr_names.iter().any(|n| n == attr.name.as_ref()) {
                    self.add_message(&[html::Node::Attribute(attr.clone())], None);
                }
            }
        }

        self.depth -= 1;
        self.in_i18n_node = was_in_i18n_node;
        self.in_implicit_node = was_in_implicit_node;

        if self.mode == VisitorMode::Merge {
            Some(Box::new(html::Element {
                name: element.name.clone(),
                attrs: element.attrs.clone(), // Simplified - should translate attributes
                directives: element.directives.clone(),
                children: child_nodes,
                is_self_closing: element.is_self_closing,
                source_span: element.source_span.clone(),
                start_source_span: element.start_source_span.clone(),
                end_source_span: element.end_source_span.clone(),
                is_void: element.is_void,
                i18n: None,
            }))
        } else {
            None
        }
    }

    fn visit_component(
        &mut self,
        component: &html::Component,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        self.may_be_add_block_children(&html::Node::Component(component.clone()));
        self.depth += 1;
        let was_in_i18n_node = self.in_i18n_node;
        let was_in_implicit_node = self.in_implicit_node;
        let mut child_nodes: Vec<html::Node> = Vec::new();
        let mut translated_child_nodes: Option<Vec<html::Node>> = None;

        let node_name = component.tag_name.clone().unwrap_or_default();
        let has_i18n_attr = Self::has_i18n_attr_component(component);
        let i18n_meta = Self::get_i18n_meta_value_component(component);
        let is_implicit = self.implicit_tags.iter().any(|t| t == node_name.as_ref())
            && !self.in_icu
            && !self.is_in_translatable_section();
        let is_top_level_implicit = !was_in_implicit_node && is_implicit;
        self.in_implicit_node = was_in_implicit_node || is_implicit;

        if !self.is_in_translatable_section() && !self.in_icu {
            if has_i18n_attr || is_top_level_implicit {
                self.in_i18n_node = true;
                if let Some(message) = self.add_message(&component.children, Some(&i18n_meta)) {
                    translated_child_nodes = Some(
                        self.translate_message(&html::Node::Component(component.clone()), &message),
                    );
                }
            }

            if self.mode == VisitorMode::Extract {
                let is_translatable = has_i18n_attr || is_top_level_implicit;
                if is_translatable {
                    self.open_translatable_section(&html::Node::Component(component.clone()));
                }
                let mut context: *mut () = std::ptr::null_mut();
                html::visit_all(self, &component.children, &mut context);
                if is_translatable {
                    self.close_translatable_section(
                        &html::Node::Component(component.clone()),
                        &component.children,
                    );
                }
            }
        } else {
            if has_i18n_attr || is_top_level_implicit {
                self.report_error(
                    &html::Node::Component(component.clone()),
                    "Could not mark an element as translatable inside a translatable section",
                );
            }

            if self.mode == VisitorMode::Extract {
                let mut context: *mut () = std::ptr::null_mut();
                html::visit_all(self, &component.children, &mut context);
            }
        }

        if self.mode == VisitorMode::Merge {
            let visit_nodes = translated_child_nodes
                .as_ref()
                .unwrap_or(&component.children);
            let mut context: *mut () = std::ptr::null_mut();
            for child in visit_nodes {
                let visit_results = html::visit_all(self, &[child.clone()], &mut context);
                if let Some(visited) = visit_results.into_iter().next() {
                    if !self.is_in_translatable_section() {
                        if let Some(node_ref) = visited.downcast_ref::<html::Node>() {
                            child_nodes.push(node_ref.clone());
                        }
                    }
                }
            }
        }

        // Visit attributes
        if self.mode == VisitorMode::Extract {
            let implicit_attr_names = self
                .implicit_attrs
                .get(component.tag_name.as_deref().unwrap_or(""))
                .cloned()
                .unwrap_or_default();

            for attr in &component.attrs {
                if implicit_attr_names.iter().any(|n| n == attr.name.as_ref()) {
                    self.add_message(&[html::Node::Attribute(attr.clone())], None);
                }
            }
        }

        self.depth -= 1;
        self.in_i18n_node = was_in_i18n_node;
        self.in_implicit_node = was_in_implicit_node;

        if self.mode == VisitorMode::Merge {
            Some(Box::new(html::Component {
                component_name: component.component_name.clone(),
                tag_name: component.tag_name.clone(),
                full_name: component.full_name.clone(),
                attrs: component.attrs.clone(), // Simplified
                directives: component.directives.clone(),
                children: child_nodes,
                is_self_closing: component.is_self_closing,
                source_span: component.source_span.clone(),
                start_source_span: component.start_source_span.clone(),
                end_source_span: component.end_source_span.clone(),
                i18n: None,
            }))
        } else {
            None
        }
    }

    fn visit_attribute(
        &mut self,
        _attribute: &html::Attribute,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None
    }

    fn visit_text(
        &mut self,
        text: &html::Text,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        if self.is_in_translatable_section() {
            self.may_be_add_block_children(&html::Node::Text(text.clone()));
        }
        Some(Box::new(text.clone()))
    }

    fn visit_comment(
        &mut self,
        comment: &html::Comment,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        let is_opening = is_opening_comment(comment);

        if is_opening && self.is_in_translatable_section() {
            self.report_error(
                &html::Node::Comment(comment.clone()),
                "Could not start a block inside a translatable section",
            );
            return None;
        }

        let is_closing = is_closing_comment(comment);

        if is_closing && !self.in_i18n_block {
            self.report_error(
                &html::Node::Comment(comment.clone()),
                "Trying to close an unopened block",
            );
            return None;
        }

        if !self.in_i18n_node && !self.in_icu {
            if !self.in_i18n_block {
                if is_opening {
                    self.in_i18n_block = true;
                    self.block_start_depth = self.depth;
                    self.block_children = Vec::new();
                    if let Some(ref value) = comment.value {
                        self.block_meaning_and_desc = I18N_COMMENT_PREFIX_REGEXP
                            .replace(value, "")
                            .trim()
                            .to_string();
                    }
                    self.open_translatable_section(&html::Node::Comment(comment.clone()));
                }
            } else {
                if is_closing {
                    if self.depth == self.block_start_depth {
                        let block_children = self.block_children.clone();
                        let block_meaning_and_desc = self.block_meaning_and_desc.clone();
                        self.close_translatable_section(
                            &html::Node::Comment(comment.clone()),
                            &block_children,
                        );
                        self.in_i18n_block = false;
                        if let Some(message) =
                            self.add_message(&block_children, Some(&block_meaning_and_desc))
                        {
                            let nodes = self
                                .translate_message(&html::Node::Comment(comment.clone()), &message);
                            let mut context: *mut () = std::ptr::null_mut();
                            let results: Vec<html::Node> =
                                html::visit_all(self, &nodes, &mut context)
                                    .into_iter()
                                    .filter_map(|r| r.downcast::<html::Node>().ok().map(|n| *n))
                                    .collect();
                            return Some(Box::new(results));
                        }
                    } else {
                        self.report_error(
                            &html::Node::Comment(comment.clone()),
                            "I18N blocks should not cross element boundaries",
                        );
                        return None;
                    }
                }
            }
        }
        None
    }

    fn visit_expansion(
        &mut self,
        expansion: &html::Expansion,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        self.may_be_add_block_children(&html::Node::Expansion(expansion.clone()));

        let was_in_icu = self.in_icu;

        if !self.in_icu {
            if self.is_in_translatable_section() {
                self.add_message(&[html::Node::Expansion(expansion.clone())], None);
            }
            self.in_icu = true;
        }

        let mut context: *mut () = std::ptr::null_mut();
        let cases: Vec<html::ExpansionCase> = html::visit_all(
            self,
            &expansion
                .cases
                .iter()
                .map(|c| html::Node::ExpansionCase(c.clone()))
                .collect::<Vec<_>>(),
            &mut context,
        )
        .into_iter()
        .filter_map(|r| r.downcast::<html::ExpansionCase>().ok().map(|c| *c))
        .collect();

        self.in_icu = was_in_icu;

        if self.mode == VisitorMode::Merge {
            Some(Box::new(html::Expansion {
                switch_value: expansion.switch_value.clone(),
                expansion_type: expansion.expansion_type.clone(),
                cases,
                source_span: expansion.source_span.clone(),
                switch_value_source_span: expansion.switch_value_source_span.clone(),
                i18n: None,
            }))
        } else {
            None
        }
    }

    fn visit_expansion_case(
        &mut self,
        icu_case: &html::ExpansionCase,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        let mut context: *mut () = std::ptr::null_mut();
        let expression: Vec<html::Node> = html::visit_all(self, &icu_case.expression, &mut context)
            .into_iter()
            .filter_map(|r| r.downcast::<html::Node>().ok().map(|n| *n))
            .collect();

        if self.mode == VisitorMode::Merge {
            Some(Box::new(html::ExpansionCase {
                value: icu_case.value.clone(),
                expression,
                source_span: icu_case.source_span.clone(),
                value_source_span: icu_case.value_source_span.clone(),
                exp_source_span: icu_case.exp_source_span.clone(),
            }))
        } else {
            None
        }
    }

    fn visit_block(
        &mut self,
        block: &html::Block,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        let mut context: *mut () = std::ptr::null_mut();
        html::visit_all(self, &block.children, &mut context);
        None
    }

    fn visit_block_parameter(
        &mut self,
        _parameter: &html::BlockParameter,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None
    }

    fn visit_let_declaration(
        &mut self,
        _decl: &html::LetDeclaration,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None
    }

    fn visit_directive(
        &mut self,
        _directive: &html::Directive,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

fn is_opening_comment(n: &html::Comment) -> bool {
    n.value
        .as_ref()
        .map(|v| v.starts_with("i18n"))
        .unwrap_or(false)
}

fn is_closing_comment(n: &html::Comment) -> bool {
    n.value
        .as_ref()
        .map(|v| v.as_ref() == "/i18n")
        .unwrap_or(false)
}

fn parse_message_meta(i18n: Option<&str>) -> MessageMeta {
    if i18n.is_none() || i18n.unwrap().is_empty() {
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
