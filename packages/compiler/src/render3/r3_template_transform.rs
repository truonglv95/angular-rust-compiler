//! Render3 Template Transform
//!
//! Corresponds to packages/compiler/src/render3/r3_template_transform.ts
//! Contains HTML AST to Ivy AST transformation

use std::collections::{HashMap, HashSet};

use lazy_static::lazy_static;
use regex::Regex;

use crate::expression_parser::ast::AST;
use crate::template_parser::binding_parser::{ParsedProperty, ParsedEvent};
use crate::i18n::i18n_ast as i18n;
use crate::ml_parser::ast as html;
use crate::ml_parser::html_whitespaces::replace_ngsp;
use crate::ml_parser::tags::is_ng_template;
use crate::ml_parser::tokens::Token;
use crate::parse_util::{ParseError, ParseSourceSpan};
use crate::style_url_resolver::is_style_url_resolvable;
use crate::template_parser::binding_parser::BindingParser;
use crate::template_parser::template_preparser::{preparse_element, PreparsedElementType};

use super::r3_ast as t;
use super::r3_control_flow::{
    create_for_loop, preprocess_if_block, create_switch_block, is_connected_for_loop_block,
    is_connected_if_loop_block,
};
use super::r3_deferred_blocks::{create_deferred_block, is_connected_defer_loop_block};

lazy_static! {
    /// Regex to match binding prefixes
    static ref BIND_NAME_REGEXP: Regex = Regex::new(r"^(?:(bind-)|(let-)|(ref-|#)|(on-)|(bindon-)|(@))(.*)$").unwrap();
}

// Group indices for BIND_NAME_REGEXP
const KW_BIND_IDX: usize = 1;
const KW_LET_IDX: usize = 2;
const KW_REF_IDX: usize = 3;
const KW_ON_IDX: usize = 4;
const KW_BINDON_IDX: usize = 5;
const KW_AT_IDX: usize = 6;
const IDENT_KW_IDX: usize = 7;

/// Binding delimiters
struct BindingDelims {
    start: &'static str,
    end: &'static str,
}

const BANANA_BOX_DELIMS: BindingDelims = BindingDelims {
    start: "[(",
    end: ")]",
};
const PROPERTY_DELIMS: BindingDelims = BindingDelims {
    start: "[",
    end: "]",
};
const EVENT_DELIMS: BindingDelims = BindingDelims {
    start: "(",
    end: ")",
};

const TEMPLATE_ATTR_PREFIX: &str = "*";

lazy_static! {
    /// Tags that shouldn't be allowed as selectorless component tags
    static ref UNSUPPORTED_SELECTORLESS_TAGS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert("link");
        set.insert("style");
        set.insert("script");
        set.insert("ng-template");
        set.insert("ng-container");
        set.insert("ng-content");
        set
    };

    /// Attributes that should not be allowed on selectorless directives
    static ref UNSUPPORTED_SELECTORLESS_DIRECTIVE_ATTRS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert("ngProjectAs");
        set.insert("ngNonBindable");
        set
    };
}

/// Result of the HTML AST to Ivy AST transformation
#[derive(Debug)]
pub struct Render3ParseResult {
    pub nodes: Vec<t::R3Node>,
    pub errors: Vec<ParseError>,
    pub styles: Vec<String>,
    pub style_urls: Vec<String>,
    pub ng_content_selectors: Vec<String>,
    pub comment_nodes: Option<Vec<t::Comment>>,
}

/// Options for parsing
#[derive(Debug, Clone)]
pub struct Render3ParseOptions {
    pub collect_comment_nodes: bool,
}

impl Default for Render3ParseOptions {
    fn default() -> Self {
        Render3ParseOptions {
            collect_comment_nodes: false,
        }
    }
}

/// Transforms HTML AST to Render3/Ivy AST
pub fn html_ast_to_render3_ast<'a>(
    html_nodes: &[html::Node],
    binding_parser: &'a mut BindingParser<'a>,
    options: &'a Render3ParseOptions,
) -> Render3ParseResult {
    let mut transformer = HtmlAstToIvyAst::new(binding_parser, options);
    let ivy_nodes = transformer.visit_all(html_nodes);

    // Combine errors from binding parser and transformer
    let binding_errors = transformer.binding_parser.errors.iter().cloned().collect::<Vec<_>>();
    let all_errors: Vec<ParseError> = binding_errors
        .into_iter()
        .chain(transformer.errors.into_iter())
        .collect();

    let mut result = Render3ParseResult {
        nodes: ivy_nodes,
        errors: all_errors,
        style_urls: transformer.style_urls,
        styles: transformer.styles,
        ng_content_selectors: transformer.ng_content_selectors,
        comment_nodes: None,
    };

    if options.collect_comment_nodes {
        result.comment_nodes = Some(transformer.comment_nodes);
    }

    result
}

/// HTML to Ivy AST transformer
struct HtmlAstToIvyAst<'a> {
    binding_parser: &'a mut BindingParser<'a>,
    options: &'a Render3ParseOptions,
    errors: Vec<ParseError>,
    styles: Vec<String>,
    style_urls: Vec<String>,
    ng_content_selectors: Vec<String>,
    comment_nodes: Vec<t::Comment>,
    in_i18n_block: bool,
    processed_nodes: HashSet<usize>,
}

impl<'a> HtmlAstToIvyAst<'a> {
    fn new(binding_parser: &'a mut BindingParser<'a>, options: &'a Render3ParseOptions) -> Self {
        HtmlAstToIvyAst {
            binding_parser,
            options,
            errors: vec![],
            styles: vec![],
            style_urls: vec![],
            ng_content_selectors: vec![],
            comment_nodes: vec![],
            in_i18n_block: false,
            processed_nodes: HashSet::new(),
        }
    }

    fn visit_all(&mut self, nodes: &[html::Node]) -> Vec<t::R3Node> {
        let mut result = vec![];
        for (index, node) in nodes.iter().enumerate() {
            if let Some(r3_node) = self.visit_node(node, nodes, index) {
                result.push(r3_node);
            }
        }
        result
    }

    fn visit_node(
        &mut self,
        node: &html::Node,
        siblings: &[html::Node],
        index: usize,
    ) -> Option<t::R3Node> {
        match node {
            html::Node::Element(element) => self.visit_element(element),
            html::Node::Text(text) => self.visit_text(text),
            html::Node::Comment(comment) => self.visit_comment(comment),
            html::Node::Expansion(expansion) => self.visit_expansion(expansion),
            html::Node::ExpansionCase(_) => None,
            html::Node::Block(block) => self.visit_block(block, siblings, index),
            html::Node::BlockParameter(_) => None,
            html::Node::LetDeclaration(decl) => self.visit_let_declaration(decl),
            html::Node::Component(component) => self.visit_component(component),
            html::Node::Directive(_) => None,
            html::Node::Attribute(_) => None,
        }
    }

    fn visit_element(&mut self, element: &html::Element) -> Option<t::R3Node> {
        let is_i18n_root = is_i18n_root_node(&element.i18n);
        if is_i18n_root {
            if self.in_i18n_block {
                self.report_error(
                    "Cannot mark an element as translatable inside of a translatable section. Please remove the nested i18n marker.",
                    &element.source_span,
                );
            }
            self.in_i18n_block = true;
        }

        let preparsed = preparse_element(element);
        
        if preparsed.element_type == PreparsedElementType::Script {
            return None;
        } else if preparsed.element_type == PreparsedElementType::Style {
            if let Some(contents) = text_contents(element) {
                self.styles.push(contents);
            }
            return None;
        } else if preparsed.element_type == PreparsedElementType::Stylesheet
            && preparsed.href_attr.as_ref().map_or(false, |h| is_style_url_resolvable(Some(h.as_str())))
        {
            if let Some(href) = preparsed.href_attr {
                self.style_urls.push(href);
            }
            return None;
        }

        let is_template_element = is_ng_template(&element.name);
        let mut prepared = self.prepare_attributes(&element.attrs, is_template_element);
        
        // Process html::Directive nodes as inline template bindings
        // These are parsed by ml_parser for *ngFor, *ngIf etc.
        // Process html::Directive nodes
        let mut r3_directives: Vec<t::Directive> = vec![];

        // Scan for structural directives in attributes (e.g. *ngIf)
        // because html_parser might not distinguish them from regular attributes
        let structural_directives: Vec<html::Directive> = element.attrs.iter()
            .filter(|a| a.name.starts_with('*'))
            .map(|a| html::Directive {
                name: a.name.clone(),
                attrs: vec![a.clone()],
                source_span: a.source_span.clone(),
                start_source_span: a.source_span.clone(),
                end_source_span: None,
            })
            .collect();
        
        for directive in element.directives.iter().chain(structural_directives.iter()) {
            // Check if it's a structural directive (*) or selectorless directive (@)
            // We look at the source content to determine the prefix
            let content = &directive.source_span.start.file.content;
            let offset = directive.source_span.start.offset;
            let is_structural = content[offset..].starts_with('*');
            
            if is_structural {
                // Structural Directive (*ngIf, *ngFor, etc.) -> Convert to inline template binding
                // The directive.attrs contains the parsed attribute like 'let item of items'
                let template_key = &directive.name;
                
                // Get the value from the directive's first attr (if any) - the full binding expression
                let first_attr = directive.attrs.first();
                let directive_value = first_attr
                    .map(|a| a.value.clone())
                    .unwrap_or_default();
                
                // Use the value_span for correct absolute offset, falling back to directive source_span
                let absolute_value_offset = first_attr
                    .and_then(|a| a.value_span.as_ref())
                    .map(|vs| vs.start.offset)
                    .unwrap_or(directive.source_span.start.offset);
                
                let mut parsed_variables = vec![];
                self.binding_parser.parse_inline_template_binding(
                    template_key,
                    &directive_value,
                    &directive.source_span,
                    absolute_value_offset,
                    &mut vec![],
                    &mut prepared.template_parsed_properties,
                    &mut parsed_variables,
                    true,
                );
                
                for v in parsed_variables {
                    prepared.template_variables.push(t::Variable {
                        name: v.name,
                        value: v.value,
                        source_span: v.source_span,
                        key_span: v.key_span,
                        value_span: v.value_span,
                    });
                }
                
                prepared.element_has_inline_template = true;
            } else {
                // Selectorless Directive (@Dir) -> Create t::Directive node
                // Parse directive's nested attributes to populate inputs/outputs
                let dir_prepared = self.prepare_attributes(
                    &directive.attrs,
                    false, // is_template_element=false for directives
                );

                let categorized = self.categorize_property_attributes(None, &dir_prepared.parsed_properties, &std::collections::HashMap::new());
                
                // Validate directive bindings
                self.validate_directive_bindings(&categorized.bound, &dir_prepared.attributes, &dir_prepared.references);

                // Check for duplicate directives
                if r3_directives.iter().any(|d| d.name == directive.name) {
                    self.report_error(
                        &format!("Directive @{} has already been applied to this element", directive.name),
                        &directive.source_span,
                    );
                }

                r3_directives.push(t::Directive {
                    name: directive.name.clone(),
                    attributes: dir_prepared.attributes,
                    inputs: categorized.bound,
                    outputs: dir_prepared.bound_events,
                    references: dir_prepared.references,
                    source_span: directive.source_span.clone(),
                    start_source_span: directive.start_source_span.clone(),
                    end_source_span: directive.end_source_span.clone(),
                    i18n: None,
                });
            }
        }

        let children = if preparsed.non_bindable {
            visit_all_non_bindable(&element.children)
        } else {
            self.visit_all(&element.children)
        };

        let parsed_element: t::R3Node = if preparsed.element_type == PreparsedElementType::NgContent {
            let selector = preparsed.select_attr.clone();
            let attrs: Vec<t::TextAttribute> = element
                .attrs
                .iter()
                .map(|attr| self.visit_attribute(attr))
                .collect();
            self.ng_content_selectors.push(selector.clone());
            t::R3Node::Content(t::Content {
                selector,
                attributes: attrs,
                children,
                is_self_closing: element.is_self_closing,
                source_span: element.source_span.clone(),
                start_source_span: element.start_source_span.clone(),
                end_source_span: element.end_source_span.clone(),
                i18n: element.i18n.clone(),
            })
        } else if is_template_element {
            let attrs = self.categorize_property_attributes(
                Some(&element.name),
                &prepared.parsed_properties,
                &prepared.i18n_attrs_meta,
            );
            t::R3Node::Template(t::Template {
                tag_name: Some(element.name.clone()),
                attributes: prepared.attributes,
                inputs: attrs.bound,
                outputs: prepared.bound_events,
                directives: vec![], // directives
                template_attrs: vec![], // template_attrs
                children,
                references: prepared.references,
                variables: prepared.variables,
                is_self_closing: element.is_self_closing,
                source_span: element.source_span.clone(),
                start_source_span: element.start_source_span.clone(),
                end_source_span: element.end_source_span.clone(),
                i18n: element.i18n.clone(),
            })
        } else {
            let attrs = self.categorize_property_attributes(
                Some(&element.name),
                &prepared.parsed_properties,
                &prepared.i18n_attrs_meta,
            );

            if element.name == "ng-container" {
                for bound in &attrs.bound {
                    use crate::expression_parser::ast::BindingType as ExprBindingType;
                    if bound.type_ == ExprBindingType::Attribute {
                        self.report_error(
                            "Attribute bindings are not supported on ng-container. Use property bindings instead.",
                            &bound.source_span,
                        );
                    }
                }
            }

            t::R3Node::Element(t::Element::new(
                element.name.clone(),
                prepared.attributes,
                attrs.bound,
                prepared.bound_events,
                r3_directives, // directives
                children,
                prepared.references,
                element.is_self_closing,
                element.source_span.clone(),
                element.start_source_span.clone(),
                element.end_source_span.clone(),
                element.is_void,
                element.i18n.clone(),
            ))
        };

        if is_i18n_root {
            // self.i18n_tree.pop(); // Not implemented yet
        }

        let result = if prepared.element_has_inline_template {
            self.wrap_in_template(
                parsed_element,
                &prepared.template_parsed_properties,
                &prepared.template_variables,
                &prepared.i18n_attrs_meta,
                is_template_element,
                is_i18n_root,
            )
        } else {
            parsed_element
        };

        if is_i18n_root {
            self.in_i18n_block = false;
        }

        Some(result)
    }

    fn visit_attribute(&self, attribute: &html::Attribute) -> t::TextAttribute {
        t::TextAttribute::new(
            attribute.name.clone(),
            attribute.value.clone(),
            attribute.source_span.clone(),
            attribute.key_span.clone(),
            attribute.value_span.clone(),
            attribute.i18n.clone(),
        )
    }

    fn visit_text(&mut self, text: &html::Text) -> Option<t::R3Node> {

        self.visit_text_with_interpolation(&text.value, &text.source_span, &text.i18n, Some(text.tokens.clone()))
    }

    fn visit_expansion(&mut self, expansion: &html::Expansion) -> Option<t::R3Node> {
        let mut vars = HashMap::new();
        let mut placeholders = HashMap::new();

        // 1. Process switch value (as a variable)
        let switch_value = &expansion.switch_value;
        let switch_span = &expansion.switch_value_source_span;
        
        let switch_expr = self.binding_parser.parse_binding(
            switch_value,
            false,
            switch_span.clone(),
            switch_span.start.offset,
        );

        let bound_text = t::BoundText::new(
             (*switch_expr.ast).clone(),
             switch_span.clone(),
             None, // switch value itself doesn't have i18n meta usually
        );
        vars.insert(switch_value.clone(), bound_text);

        // 2. Process cases recursively
        for case in &expansion.cases {
            for node in &case.expression {
                let r3_node = match node {
                    html::Node::Element(e) => self.visit_element(e),
                    html::Node::Text(txt) => self.visit_text(txt),
                    // html::Node::BoundText does not exist in ml_parser
                    html::Node::Expansion(e) => self.visit_expansion(e),
                    _ => None,
                };

                if let Some(node) = r3_node {
                    match node {
                        t::R3Node::BoundText(bt) => {
                            let key = bt.source_span.to_string(); 
                            placeholders.insert(key, t::IcuPlaceholder::BoundText(bt));
                        },
                        t::R3Node::Text(txt) => {
                             let key = txt.source_span.to_string();
                             placeholders.insert(key, t::IcuPlaceholder::Text(txt));
                        },
                        t::R3Node::Icu(nested_icu) => {
                            // Merge variables and placeholders from nested ICU
                            vars.extend(nested_icu.vars);
                            placeholders.extend(nested_icu.placeholders);
                        }
                        _ => {}
                    }
                }
            }
        }

        Some(t::R3Node::Icu(t::Icu {
            vars,
            placeholders,
            source_span: expansion.source_span.clone(),
            i18n: expansion.i18n.clone(),
        }))
    }

    fn visit_comment(&mut self, comment: &html::Comment) -> Option<t::R3Node> {
        if self.options.collect_comment_nodes {
            let comment_node = t::Comment::new(
                comment.value.clone().unwrap_or_default(),
                comment.source_span.clone(),
            );
            self.comment_nodes.push(comment_node);
        }
        None
    }

    fn visit_let_declaration(&mut self, decl: &html::LetDeclaration) -> Option<t::R3Node> {
        let value = self.binding_parser.parse_binding(
            &decl.value,
            false,
            decl.value_span.clone(),
            decl.value_span.start.offset,
        );

        if value.errors.is_empty() && matches!(*value.ast, AST::EmptyExpr(_)) {
            self.report_error("@let declaration value cannot be empty", &decl.value_span);
        }

        Some(t::R3Node::LetDeclaration(t::LetDeclaration {
            name: decl.name.clone(),
            value: (*value.ast).clone(),
            source_span: decl.source_span.clone(),
            name_span: decl.name_span.clone(),
            value_span: decl.value_span.clone(),
        }))
    }

    fn visit_component(&mut self, component: &html::Component) -> Option<t::R3Node> {
        let is_i18n_root = is_i18n_root_node(&component.i18n);
        if is_i18n_root {
            if self.in_i18n_block {
                self.report_error(
                    "Cannot mark a component as translatable inside of a translatable section. Please remove the nested i18n marker.",
                    &component.source_span,
                );
            }
            self.in_i18n_block = true;
        }

        if let Some(ref tag_name) = component.tag_name {
            if UNSUPPORTED_SELECTORLESS_TAGS.contains(tag_name.as_str()) {
                self.report_error(
                    &format!("Tag name \"{}\" cannot be used as a component tag", tag_name),
                    &component.start_source_span,
                );
                return None;
            }
        }

        let mut prepared = self.prepare_attributes(&component.attrs, false);
        self.validate_selectorless_references(&prepared.references);

        let children = if component.attrs.iter().any(|a| a.name == "ngNonBindable") {
            visit_all_non_bindable(&component.children)
        } else {
            self.visit_all(&component.children)
        };

        let attrs = self.categorize_property_attributes(
            component.tag_name.as_ref().map(|s| s.as_str()),
            &prepared.parsed_properties,
            &prepared.i18n_attrs_meta,
        );

        // Process html::Directive nodes
        let mut r3_directives: Vec<t::Directive> = vec![];
        
        for directive in &component.directives {
            // Check if it's a structural directive (*) or selectorless directive (@)
            // We look at the source content to determine the prefix
            let content = &directive.source_span.start.file.content;
            let offset = directive.source_span.start.offset;
            let is_structural = content[offset..].starts_with('*');
            


            if is_structural {
                // Structural Directive (*ngIf, *ngFor, etc.) -> Convert to inline template binding
                // The directive.attrs contains the parsed attribute like 'let item of items'
                let template_key = &directive.name; 
                
                // Get the value from the directive's first attr (if any) - the full binding expression
                let first_attr = directive.attrs.first();
                let directive_value = first_attr
                    .map(|a| a.value.clone())
                    .unwrap_or_default();
                
                // Use the value_span for correct absolute offset, falling back to directive source_span
                let absolute_value_offset = first_attr
                    .and_then(|a| a.value_span.as_ref())
                    .map(|vs| vs.start.offset)
                    .unwrap_or(directive.source_span.start.offset);
                
                let mut parsed_variables = vec![];
                self.binding_parser.parse_inline_template_binding(
                    template_key,
                    &directive_value,
                    &directive.source_span,
                    absolute_value_offset,
                    &mut vec![],
                    &mut prepared.template_parsed_properties,
                    &mut parsed_variables,
                    true,
                );
                
                for v in parsed_variables {
                    prepared.template_variables.push(t::Variable {
                        name: v.name,
                        value: v.value,
                        source_span: v.source_span,
                        key_span: v.key_span,
                        value_span: v.value_span,
                    });
                }
                
                prepared.element_has_inline_template = true;
            } else {
                // Selectorless Directive (@Dir) -> Create t::Directive node
                // Parse directive's nested attributes to populate inputs/outputs
                let dir_prepared = self.prepare_attributes(
                    &directive.attrs,
                    false, // is_template_element=false for directives
                );

                let categorized = self.categorize_property_attributes(None, &dir_prepared.parsed_properties, &std::collections::HashMap::new());
                
                // Validate directive bindings
                self.validate_directive_bindings(&categorized.bound, &dir_prepared.attributes, &dir_prepared.references);

                // Check for duplicate directives
                // println!("Checking duplicate for {}. Existing: {:?}", directive.name, r3_directives.iter().map(|d| &d.name).collect::<Vec<_>>());
                if r3_directives.iter().any(|d| d.name == directive.name) {
                    // println!("Found duplicate!");
                    self.report_error(
                        &format!("Directive @{} has already been applied to this element", directive.name),
                        &directive.source_span,
                    );
                }

                r3_directives.push(t::Directive {
                    name: directive.name.clone(),
                    attributes: dir_prepared.attributes,
                    inputs: categorized.bound,
                    outputs: dir_prepared.bound_events,
                    references: dir_prepared.references,
                    source_span: directive.source_span.clone(),
                    start_source_span: directive.start_source_span.clone(),
                    end_source_span: directive.end_source_span.clone(),
                    i18n: None,
                });
            }
        }

        let node = t::R3Node::Component(t::Component {
            component_name: component.component_name.clone(),
            tag_name: component.tag_name.clone(),
            full_name: component.full_name.clone(),
            attributes: prepared.attributes,
            inputs: attrs.bound,
            outputs: prepared.bound_events,
            directives: r3_directives, // directives
            children,
            references: prepared.references,
            is_self_closing: component.is_self_closing,
            source_span: component.source_span.clone(),
            start_source_span: component.start_source_span.clone(),
            end_source_span: component.end_source_span.clone(),
            i18n: component.i18n.clone(),
        });

        let result = if prepared.element_has_inline_template {
            self.wrap_in_template(
                node,
                &prepared.template_parsed_properties,
                &prepared.template_variables,
                &prepared.i18n_attrs_meta,
                false,
                is_i18n_root,
            )
        } else {
            node
        };

        if is_i18n_root {
            self.in_i18n_block = false;
        }

        Some(result)
    }

    fn visit_block(
        &mut self,
        block: &html::Block,
        siblings: &[html::Node],
        index: usize,
    ) -> Option<t::R3Node> {
        // Use pointer address as unique identifier
        let block_id = block as *const html::Block as usize;
        if self.processed_nodes.contains(&block_id) {
            return None;
        }

        let result = match block.name.as_str() {
            "defer" => {
                let connected = self.find_connected_blocks(index, siblings, is_connected_defer_loop_block);
                let mut result = create_deferred_block(block, &connected, self.binding_parser);
                self.errors.extend(result.errors);
                
                // Transform and populate main children
                result.node.children = self.visit_all(&block.children);
                
                // Transform and populate children for connected blocks (placeholder, loading, error)
                for connected_block in &connected {
                    let children = self.visit_all(&connected_block.children);
                    match connected_block.name.as_str() {
                        "placeholder" => {
                            if let Some(ref mut pl) = result.node.placeholder {
                                pl.children = children;
                            }
                        }
                        "loading" => {
                            if let Some(ref mut ld) = result.node.loading {
                                ld.children = children;
                            }
                        }
                        "error" => {
                            if let Some(ref mut er) = result.node.error {
                                er.children = children;
                            }
                        }
                        _ => {}
                    }
                }
                
                Some(t::R3Node::DeferredBlock(result.node))
            }
            "switch" => {
                let result = create_switch_block(block, self.binding_parser);
                self.errors.extend(result.errors);
                
                // Transform children for each case via visitor
                result.node.map(|mut switch_block| {
                    // Match case blocks with their children from the original AST
                    for case in &mut switch_block.cases {
                        // Find the corresponding case block in AST children
                        for child in &block.children {
                            if let html::Node::Block(case_block) = child {
                                // Match by checking if this is the right case block (compare by start position)
                                if case_block.source_span.start.offset == case.block.source_span.start.offset {
                                    case.children = self.visit_all(&case_block.children);
                                    break;
                                }
                            }
                        }
                    }
                    
                    t::R3Node::SwitchBlock(switch_block)
                })
            }
            "for" => {
                let connected = self.find_connected_blocks(index, siblings, is_connected_for_loop_block);
                let result = create_for_loop(block, &connected, self.binding_parser);
                self.errors.extend(result.errors);
                
                // Transform children via visitor
                result.node.map(|mut for_block| {
                    for_block.children = self.visit_all(&block.children);
                    
                    // Transform empty block children if present
                    if let Some(ref mut empty) = for_block.empty {
                        // Find the empty connected block to get its children
                        for conn_block in &connected {
                            if conn_block.name == "empty" {
                                empty.children = self.visit_all(&conn_block.children);
                                break;
                            }
                        }
                    }
                    
                    t::R3Node::ForLoopBlock(for_block)
                })
            }
            "if" => {
                let connected = self.find_connected_blocks(index, siblings, is_connected_if_loop_block);
                let preprocess = preprocess_if_block(block, &connected, self.binding_parser);
                self.errors.extend(preprocess.errors);
                
                // Transform children for each branch
                let branches = preprocess.branches.into_iter().map(|branch| {
                    t::IfBlockBranch {
                        expression: branch.expression,
                        children: self.visit_all(branch.html_children),
                        expression_alias: branch.expression_alias,
                        block: branch.block,
                        i18n: branch.i18n,
                    }
                }).collect();
                
                Some(t::R3Node::IfBlock(t::IfBlock {
                    branches,
                    block: t::BlockNode::new(
                        preprocess.name_span,
                        preprocess.whole_source_span,
                        preprocess.start_source_span,
                        preprocess.end_source_span,
                    ),
                }))
            }
            _ => {
                let error_message = if is_connected_defer_loop_block(&block.name) {
                    let block_id = block as *const html::Block as usize;
                    self.processed_nodes.insert(block_id);
                    format!("@{} block can only be used after an @defer block.", block.name)
                } else if is_connected_for_loop_block(&block.name) {
                    let block_id = block as *const html::Block as usize;
                    self.processed_nodes.insert(block_id);
                    format!("@{} block can only be used after an @for block.", block.name)
                } else if is_connected_if_loop_block(&block.name) {
                    let block_id = block as *const html::Block as usize;
                    self.processed_nodes.insert(block_id);
                    format!("@{} block can only be used after an @if or @else if block.", block.name)
                } else {
                    format!("Unrecognized block @{}.", block.name)
                };

                self.errors.push(ParseError::new(
                    block.source_span.clone(),
                    error_message,
                ));
                Some(t::R3Node::UnknownBlock(t::UnknownBlock {
                    name: block.name.clone(),
                    source_span: block.source_span.clone(),
                    name_span: block.name_span.clone(),
                }))
            }
        };

        result
    }

    fn find_connected_blocks(
        &mut self,
        primary_block_index: usize,
        siblings: &[html::Node],
        predicate: fn(&str) -> bool,
    ) -> Vec<html::Block> {
        let mut related_blocks = vec![];

        for i in (primary_block_index + 1)..siblings.len() {
            let node = &siblings[i];

            // Skip comments
            if matches!(node, html::Node::Comment(_)) {
                continue;
            }

            // Ignore empty text nodes between blocks
            if let html::Node::Text(text) = node {
                if text.value.trim().is_empty() {
                    let text_id = text as *const html::Text as usize;
                    self.processed_nodes.insert(text_id);
                    continue;
                }
            }

            // Stop searching when hitting non-block or unrelated block
            if let html::Node::Block(block) = node {
                if predicate(&block.name) {
                    related_blocks.push(block.clone());
                    let block_id = block as *const html::Block as usize;
                    self.processed_nodes.insert(block_id);
                    continue;
                }
            }
            break;
        }

        related_blocks
    }

    fn categorize_property_attributes(
        &mut self,
        element_name: Option<&str>,
        properties: &[ParsedProperty],
        _i18n_props_meta: &std::collections::HashMap<String, i18n::I18nMeta>,
    ) -> CategorizedAttributes {
        let mut bound = vec![];
        let mut literal = vec![];

        for prop in properties {
            if prop.is_literal && !prop.is_legacy_animation {
                literal.push(t::TextAttribute::new(
                    prop.name.clone(),
                    prop.expression.source.clone().unwrap_or_default(),
                    prop.source_span.clone(),
                    prop.key_span.clone(),
                    prop.value_span.clone(),
                    None, // i18n
                ));
            } else {
                let bep = self.binding_parser.create_bound_element_property(
                    element_name,
                    prop,
                    true,  // skip_validation
                    false, // map_property_name
                );
                // Convert binding_parser::BindingType to expression_parser::ast::BindingType
                use crate::expression_parser::ast::BindingType as ExprBindingType;
                let expr_binding_type = match bep.type_ {
                    crate::template_parser::binding_parser::BindingType::Property => ExprBindingType::Property,
                    crate::template_parser::binding_parser::BindingType::Attribute => ExprBindingType::Attribute,
                    crate::template_parser::binding_parser::BindingType::Class => ExprBindingType::Class,
                    crate::template_parser::binding_parser::BindingType::Style => ExprBindingType::Style,
                    crate::template_parser::binding_parser::BindingType::Animation => ExprBindingType::Animation,
                    crate::template_parser::binding_parser::BindingType::TwoWay => ExprBindingType::TwoWay,
                    crate::template_parser::binding_parser::BindingType::LegacyAnimation => ExprBindingType::LegacyAnimation,
                };
                let key_span = bep.key_span.unwrap_or_else(|| bep.source_span.clone());
                let source_span = bep.source_span.clone();
                bound.push(t::BoundAttribute::new(
                    bep.name,
                    expr_binding_type,
                    bep.security_context,
                    (*bep.value.ast).clone(),
                    bep.unit,
                    source_span,
                    key_span,
                    bep.value_span,
                    None, // i18n
                ));
            }
        }

        CategorizedAttributes { bound, literal }
    }

    fn prepare_attributes(
        &mut self,
        attrs: &[html::Attribute],
        is_template_element: bool,
    ) -> PreparedAttributes {
        let mut parsed_properties = vec![];
        let mut bound_events = vec![];
        let mut variables = vec![];
        let mut references = vec![];
        let mut attributes = vec![];
        let mut i18n_attrs_meta = std::collections::HashMap::new();
        let mut template_parsed_properties = vec![];
        let mut template_variables = vec![];
        let mut element_has_inline_template = false;

        for attribute in attrs {
            let mut has_binding = false;
            let normalized_name = normalize_attribute_name(&attribute.name);

            let mut is_template_binding = false;

            if let Some(ref i18n) = attribute.i18n {
                i18n_attrs_meta.insert(attribute.name.clone(), i18n.clone());
            }

            if normalized_name.starts_with(TEMPLATE_ATTR_PREFIX) {
                if element_has_inline_template {
                    self.report_error(
                        "Can't have multiple template bindings on one element. Use only one attribute prefixed with *",
                        &attribute.source_span,
                    );
                }
                is_template_binding = true;
                element_has_inline_template = true;

                let template_key = &normalized_name[TEMPLATE_ATTR_PREFIX.len()..];
                let template_value = &attribute.value;

                let absolute_value_offset = attribute
                    .value_span
                    .as_ref()
                    .map(|vs| vs.start.offset)
                    .unwrap_or(attribute.source_span.start.offset + attribute.name.len());

                let mut parsed_variables = vec![];
                self.binding_parser.parse_inline_template_binding(
                    template_key,
                    template_value,
                    &attribute.source_span,
                    absolute_value_offset,
                    &mut vec![],
                    &mut template_parsed_properties,
                    &mut parsed_variables,
                    true,
                );

                for v in parsed_variables {
                    template_variables.push(t::Variable {
                        name: v.name,
                        value: v.value,
                        source_span: v.source_span,
                        key_span: v.key_span,
                        value_span: v.value_span,
                    });
                }
            } else {
                has_binding = self.parse_attribute(
                    is_template_element,
                    attribute,
                    &mut parsed_properties,
                    &mut bound_events,
                    &mut variables,
                    &mut references,
                );
            }

            if !has_binding && !is_template_binding {
                attributes.push(self.visit_attribute(attribute));
            }
        }

        PreparedAttributes {
            attributes,
            bound_events,
            references,
            variables,
            template_variables,
            element_has_inline_template,
            parsed_properties,
            template_parsed_properties,
            i18n_attrs_meta,
        }
    }

    fn parse_attribute(
        &mut self,
        is_template_element: bool,
        attribute: &html::Attribute,
        parsed_properties: &mut Vec<ParsedProperty>,
        bound_events: &mut Vec<t::BoundEvent>,
        variables: &mut Vec<t::Variable>,
        references: &mut Vec<t::Reference>,
    ) -> bool {
        let name = normalize_attribute_name(&attribute.name);
        let value = &attribute.value;
        let src_span = &attribute.source_span;
        let absolute_offset = attribute
            .value_span
            .as_ref()
            .map(|vs| vs.start.offset)
            .unwrap_or(src_span.start.offset);

        // Check for bind-/let-/ref-/on-/bindon-/@ prefixes
        if let Some(captures) = BIND_NAME_REGEXP.captures(&name) {
            if captures.get(KW_BIND_IDX).is_some() {
                let identifier = captures.get(IDENT_KW_IDX).map(|m| m.as_str()).unwrap_or("");
                let key_span = create_key_span(src_span, &attribute.name, &name, captures.get(KW_BIND_IDX).unwrap().as_str(), identifier);
                self.binding_parser.parse_property_binding(
                    identifier,
                    value,
                    false,
                    false,
                    src_span.clone(),
                    absolute_offset,
                    attribute.value_span.clone(),
                    &mut vec![],
                    parsed_properties,
                    key_span.clone(),
                );
                return true;
            } else if captures.get(KW_LET_IDX).is_some() {
                if is_template_element {
                    let identifier = captures.get(IDENT_KW_IDX).map(|m| m.as_str()).unwrap_or("");
                    let key_span = create_key_span(src_span, &attribute.name, &name, captures.get(KW_LET_IDX).unwrap().as_str(), identifier);
                    self.parse_variable(identifier, value, src_span, &key_span, attribute.value_span.as_ref(), variables);
                } else {
                    self.report_error("\"let-\" is only supported on ng-template elements.", src_span);
                }
                return true;
            } else if captures.get(KW_REF_IDX).is_some() {
                let identifier = captures.get(IDENT_KW_IDX).map(|m| m.as_str()).unwrap_or("");
                let key_span = create_key_span(src_span, &attribute.name, &name, captures.get(KW_REF_IDX).unwrap().as_str(), identifier);
                self.parse_reference(identifier, value, src_span, &key_span, attribute.value_span.as_ref(), references);
                return true;
            } else if captures.get(KW_ON_IDX).is_some() {
                let identifier = captures.get(IDENT_KW_IDX).map(|m| m.as_str()).unwrap_or("");
                let key_span = create_key_span(src_span, &attribute.name, &name, captures.get(KW_ON_IDX).unwrap().as_str(), identifier);
                let mut events = vec![];
                self.binding_parser.parse_event(
                    identifier,
                    value,
                    false,
                    src_span.clone(),
                    attribute.value_span.as_ref().unwrap_or(src_span).clone(),
                    &mut vec![],
                    &mut events,
                    Some(key_span.clone()),
                );
                add_events(&events, bound_events);
                return true;
            } else if captures.get(KW_BINDON_IDX).is_some() {
                let identifier = captures.get(IDENT_KW_IDX).map(|m| m.as_str()).unwrap_or("");
                let key_span = create_key_span(src_span, &attribute.name, &name, captures.get(KW_BINDON_IDX).unwrap().as_str(), identifier);
                self.binding_parser.parse_property_binding(
                    identifier,
                    value,
                    false,
                    true,
                    src_span.clone(),
                    absolute_offset,
                    attribute.value_span.clone(),
                    &mut vec![],
                    parsed_properties,
                    key_span.clone(),
                );
                self.parse_assignment_event(
                    identifier,
                    value,
                    src_span,
                    attribute.value_span.as_ref(),
                    bound_events,
                    &key_span,
                );
                return true;
            } else if captures.get(KW_AT_IDX).is_some() {
                let key_span = create_key_span(src_span, &attribute.name, &name, "", &name);
                // For @animation without value, pass None instead of Some("")
                let value_opt: Option<&str> = if value.is_empty() { None } else { Some(value) };
                self.binding_parser.parse_literal_attr(
                    &name,
                    value_opt,
                    src_span.clone(),
                    absolute_offset,
                    attribute.value_span.clone(),
                    &mut vec![],
                    parsed_properties,
                    key_span.clone(),
                );
                return true;
            }
        }

        // Check for [(...)], [...], (...) delimiters
        let delims = if name.starts_with(BANANA_BOX_DELIMS.start) {
            Some(&BANANA_BOX_DELIMS)
        } else if name.starts_with(PROPERTY_DELIMS.start) {
            Some(&PROPERTY_DELIMS)
        } else if name.starts_with(EVENT_DELIMS.start) {
            Some(&EVENT_DELIMS)
        } else {
            None
        };

        if let Some(delims) = delims {
            if name.ends_with(delims.end) && name.len() > delims.start.len() + delims.end.len() {
                let identifier = &name[delims.start.len()..name.len() - delims.end.len()];
                let key_span = create_key_span(src_span, &attribute.name, &name, delims.start, identifier);

                if delims.start == BANANA_BOX_DELIMS.start {
                    self.binding_parser.parse_property_binding(
                        identifier,
                        value,
                        false,
                        true,
                        src_span.clone(),
                        absolute_offset,
                        attribute.value_span.clone(),
                        &mut vec![],
                        parsed_properties,
                        key_span.clone(),
                    );
                    self.parse_assignment_event(
                        identifier,
                        value,
                        src_span,
                        attribute.value_span.as_ref(),
                        bound_events,
                        &key_span,
                    );
                } else if delims.start == PROPERTY_DELIMS.start {
                    self.binding_parser.parse_property_binding(
                        identifier,
                        value,
                        false,
                        false,
                        src_span.clone(),
                        absolute_offset,
                        attribute.value_span.clone(),
                        &mut vec![],
                        parsed_properties,
                        key_span.clone(),
                    );
                } else {
                    let mut events = vec![];
                    self.binding_parser.parse_event(
                        identifier,
                        value,
                        false,
                        src_span.clone(),
                        attribute.value_span.as_ref().unwrap_or(src_span).clone(),
                        &mut vec![],
                        &mut events,
                        Some(key_span.clone()),
                    );
                    add_events(&events, bound_events);
                }

                return true;
            }
        }

        // Check for interpolation
        let key_span = create_key_span(src_span, &attribute.name, &name, "", &name);
        let value_span_or_src = attribute.value_span.as_ref().unwrap_or(src_span);
        let expr = self.binding_parser.parse_interpolation(value, value_span_or_src, attribute.value_tokens.clone());
        
        // If interpolation was found AND it has expressions, parse it as a property binding
        // If it's pure text (empty expressions), treat as attribute.
        let is_interpolation = if let AST::Interpolation(ref interp) = *expr.ast {
            !interp.expressions.is_empty()
        } else {
            false
        };

        if is_interpolation {
            self.binding_parser.parse_property_ast(
                &name,
                expr,
                false,
                &src_span,
                &key_span,
                attribute.value_span.as_ref(),
                &mut vec![],
                parsed_properties,
            );
            true
        } else {
            false
        }
    }

    fn parse_variable(
        &mut self,
        identifier: &str,
        value: &str,
        source_span: &ParseSourceSpan,
        key_span: &ParseSourceSpan,
        value_span: Option<&ParseSourceSpan>,
        variables: &mut Vec<t::Variable>,
    ) {
        if identifier.contains('-') {
            self.report_error("\"-\" is not allowed in variable names", source_span);
        } else if identifier.is_empty() {
            self.report_error("Variable does not have a name", source_span);
        }

        variables.push(t::Variable {
            name: identifier.to_string(),
            value: value.to_string(),
            source_span: source_span.clone(),
            key_span: key_span.clone(),
            value_span: value_span.cloned(),
        });
    }

    fn parse_reference(
        &mut self,
        identifier: &str,
        value: &str,
        source_span: &ParseSourceSpan,
        key_span: &ParseSourceSpan,
        value_span: Option<&ParseSourceSpan>,
        references: &mut Vec<t::Reference>,
    ) {
        if identifier.contains('-') {
            self.report_error("\"-\" is not allowed in reference names", source_span);
        } else if identifier.is_empty() {
            self.report_error("Reference does not have a name", source_span);
        } else if references.iter().any(|r| r.name == identifier) {
            self.report_error(
                &format!("Reference \"#{}\" is defined more than once", identifier),
                source_span,
            );
        }

        references.push(t::Reference {
            name: identifier.to_string(),
            value: value.to_string(),
            source_span: source_span.clone(),
            key_span: key_span.clone(),
            value_span: value_span.cloned(),
        });
    }

    fn parse_assignment_event(
        &mut self,
        name: &str,
        expression: &str,
        source_span: &ParseSourceSpan,
        value_span: Option<&ParseSourceSpan>,
        bound_events: &mut Vec<t::BoundEvent>,
        key_span: &ParseSourceSpan,
    ) {
        let mut events = vec![];
        self.binding_parser.parse_event(
            &format!("{}Change", name),
            expression,
            true,
            source_span.clone(),
            value_span.cloned().unwrap_or_else(|| source_span.clone()),
            &mut vec![],
            &mut events,
            Some(key_span.clone()),
        );
        add_events(&events, bound_events);
    }

    fn visit_text_with_interpolation(
        &mut self,
        value: &str,
        source_span: &ParseSourceSpan,
        i18n: &Option<i18n::I18nMeta>,
        tokens: Option<Vec<Token>>,
    ) -> Option<t::R3Node> {
        let value_no_ngsp = replace_ngsp(value);
        let expr = self.binding_parser.parse_interpolation(&value_no_ngsp, source_span, tokens);

        // Check if interpolation is actually static (no expressions)
        let is_static = match &*expr.ast {
             AST::Interpolation(interp) => interp.expressions.is_empty(),
             AST::LiteralPrimitive(_) => true,
             _ => false 
        };

        if is_static {
             return Some(t::R3Node::Text(t::Text::new(
                 value_no_ngsp, // Use processed value or original? TypeScript uses processed.
                 source_span.clone(),
             )));
        }

        Some(t::R3Node::BoundText(t::BoundText::new(
            (*expr.ast).clone(),
            source_span.clone(),
            i18n.clone(),
        )))
    }

    fn wrap_in_template(
        &mut self,
        node: t::R3Node,
        template_properties: &[ParsedProperty],
        template_variables: &[t::Variable],
        i18n_attrs_meta: &std::collections::HashMap<String, i18n::I18nMeta>,
        is_template_element: bool,
        is_i18n_root: bool,
    ) -> t::R3Node {
        let attrs = self.categorize_property_attributes(
            Some("ng-template"),
            template_properties,
            i18n_attrs_meta,
        );

        let mut template_attrs: Vec<t::TemplateAttr> = vec![];
        for attr in attrs.literal {
            template_attrs.push(t::TemplateAttr::Text(attr));
        }
        for attr in attrs.bound {
            template_attrs.push(t::TemplateAttr::Bound(attr));
        }

        let i18n = if is_template_element && is_i18n_root {
            None
        } else {
            match &node {
                t::R3Node::Element(e) => e.i18n.clone(),
                t::R3Node::Component(c) => c.i18n.clone(),
                t::R3Node::Template(t) => t.i18n.clone(),
                t::R3Node::Content(c) => c.i18n.clone(),
                _ => None,
            }
        };

        let (source_span, start_source_span, end_source_span) = match &node {
            t::R3Node::Element(e) => (e.source_span.clone(), e.start_source_span.clone(), e.end_source_span.clone()),
            t::R3Node::Component(c) => (c.source_span.clone(), c.start_source_span.clone(), c.end_source_span.clone()),
            t::R3Node::Template(t) => (t.source_span.clone(), t.start_source_span.clone(), t.end_source_span.clone()),
            t::R3Node::Content(c) => (c.source_span.clone(), c.start_source_span.clone(), c.end_source_span.clone()),
            _ => return node,
        };

        t::R3Node::Template(t::Template {
            tag_name: None,
            attributes: vec![],
            inputs: vec![],
            outputs: vec![],
            directives: vec![],
            template_attrs,
            children: vec![node],
            references: vec![],
            variables: template_variables.to_vec(),
            is_self_closing: false,
            source_span,
            start_source_span,
            end_source_span,
            i18n,
        })
    }

    fn report_error(&mut self, message: &str, source_span: &ParseSourceSpan) {
        self.errors.push(ParseError::new(source_span.clone(), message.to_string()));
    }

    fn validate_directive_bindings(
        &mut self,
        bound_attributes: &[t::BoundAttribute],
        literal_attributes: &[t::TextAttribute],
        references: &[t::Reference],
    ) {
        use crate::expression_parser::ast::BindingType;

        if !references.is_empty() {
             self.validate_selectorless_references(references);
        }

        for attr in bound_attributes {
            match attr.type_ {
                BindingType::Property | BindingType::TwoWay => {}
                BindingType::Attribute => {
                    self.report_error(
                        "Attribute bindings are not allowed inside of @Directive(...)",
                        &attr.source_span,
                    );
                }
                BindingType::Class => {
                    self.report_error(
                        "Class bindings are not allowed inside of @Directive(...)",
                        &attr.source_span,
                    );
                }
                BindingType::Style => {
                    self.report_error(
                        "Style bindings are not allowed inside of @Directive(...)", 
                        &attr.source_span,
                    );
                }
                BindingType::Animation | BindingType::LegacyAnimation => {
                    self.report_error(
                        "Animation bindings are not allowed inside of @Directive(...)",
                        &attr.source_span,
                    );
                }
            }
        }

        for attr in literal_attributes {
            if attr.name == "ngNonBindable" {
                self.report_error(
                    "ngNonBindable is not allowed inside of @Directive(...)",
                    &attr.source_span,
                );
            } else if attr.name == "ngProjectAs" {
                self.report_error(
                    "ngProjectAs is not allowed inside of @Directive(...)",
                    &attr.source_span,
                );
            }
        }
    }

    fn validate_selectorless_references(&mut self, references: &[t::Reference]) {
        let mut seen_names = std::collections::HashSet::new();
        for reference in references {
            if !reference.value.is_empty() {
                self.report_error(
                    "Cannot specify a value for a local reference in this context",
                   reference.value_span.as_ref().unwrap_or(&reference.source_span),
                );
            } else if seen_names.contains(&reference.name) {
                self.report_error(
                    "Duplicate reference names are not allowed",
                    &reference.source_span,
                );
            } else {
                seen_names.insert(reference.name.clone());
            }
        }
    }
}

struct CategorizedAttributes {
    bound: Vec<t::BoundAttribute>,
    literal: Vec<t::TextAttribute>,
}

struct PreparedAttributes {
    attributes: Vec<t::TextAttribute>,
    bound_events: Vec<t::BoundEvent>,
    references: Vec<t::Reference>,
    variables: Vec<t::Variable>,
    template_variables: Vec<t::Variable>,
    element_has_inline_template: bool,
    parsed_properties: Vec<ParsedProperty>,
    template_parsed_properties: Vec<ParsedProperty>,
    i18n_attrs_meta: std::collections::HashMap<String, i18n::I18nMeta>,
}

/// Non-bindable visitor for elements with ngNonBindable
fn visit_all_non_bindable(nodes: &[html::Node]) -> Vec<t::R3Node> {
    let mut result = vec![];
    for node in nodes {
        if let Some(r3_node) = visit_non_bindable_node(node) {
            result.push(r3_node);
        }
    }
    result
}

fn visit_non_bindable_node(node: &html::Node) -> Option<t::R3Node> {
    match node {
        html::Node::Element(element) => {
            let preparsed = preparse_element(element);
            if preparsed.element_type == PreparsedElementType::Script
                || preparsed.element_type == PreparsedElementType::Style
                || preparsed.element_type == PreparsedElementType::Stylesheet
            {
                return None;
            }

            let children = visit_all_non_bindable(&element.children);
            let attrs: Vec<t::TextAttribute> = element
                .attrs
                .iter()
                .map(|attr| t::TextAttribute::new(
                    attr.name.clone(),
                    attr.value.clone(),
                    attr.source_span.clone(),
                    attr.key_span.clone(),
                    attr.value_span.clone(),
                    attr.i18n.clone(),
                ))
                .collect();

            Some(t::R3Node::Element(t::Element::new(
                element.name.clone(),
                attrs,
                vec![],
                vec![],
                vec![],
                children,
                vec![],
                element.is_self_closing,
                element.source_span.clone(),
                element.start_source_span.clone(),
                element.end_source_span.clone(),
                element.is_void,
                None,
            )))
        }
        html::Node::Text(text) => Some(t::R3Node::Text(t::Text::new(
            text.value.clone(),
            text.source_span.clone(),
        ))),
        html::Node::Block(block) => {
            let mut nodes = vec![t::R3Node::Text(t::Text::new(
                block.start_source_span.to_string(),
                block.start_source_span.clone(),
            ))];
            nodes.extend(visit_all_non_bindable(&block.children));
            if let Some(ref end_span) = block.end_source_span {
                nodes.push(t::R3Node::Text(t::Text::new(
                    end_span.to_string(),
                    end_span.clone(),
                )));
            }
            // Return first node, rest are ignored in this simplified implementation
            nodes.into_iter().next()
        }
        html::Node::LetDeclaration(decl) => Some(t::R3Node::Text(t::Text::new(
            format!("@let {} = {};", decl.name, decl.value),
            decl.source_span.clone(),
        ))),
        html::Node::Component(component) => {
            let children = visit_all_non_bindable(&component.children);
            let attrs: Vec<t::TextAttribute> = component
                .attrs
                .iter()
                .map(|attr| t::TextAttribute::new(
                    attr.name.clone(),
                    attr.value.clone(),
                    attr.source_span.clone(),
                    attr.key_span.clone(),
                    attr.value_span.clone(),
                    attr.i18n.clone(),
                ))
                .collect();

            // Use tag_name if available, otherwise component_name
            let name = component.tag_name.as_ref().unwrap_or(&component.component_name).clone();

            Some(t::R3Node::Element(t::Element::new(
                name,
                attrs,
                vec![],
                vec![],
                vec![],
                children,
                vec![],
                component.is_self_closing,
                component.source_span.clone(),
                component.start_source_span.clone(),
                component.end_source_span.clone(),
                false, // Components are custom elements, not void (usually)
                None,
            )))
        }
        _ => None,
    }
}

fn normalize_attribute_name(attr_name: &str) -> String {
    if attr_name.to_lowercase().starts_with("data-") {
        attr_name[5..].to_string()
    } else {
        attr_name.to_string()
    }
}

fn add_events(events: &[ParsedEvent], bound_events: &mut Vec<t::BoundEvent>) {
    use crate::expression_parser::ast::ParsedEventType as ExprParsedEventType;
    for e in events {
        let expr_event_type = match e.type_ {
            crate::template_parser::binding_parser::ParsedEventType::Regular => ExprParsedEventType::Regular,
            crate::template_parser::binding_parser::ParsedEventType::Animation => ExprParsedEventType::Animation,
            crate::template_parser::binding_parser::ParsedEventType::TwoWay => ExprParsedEventType::Regular, // TwoWay not in ExprParsedEventType
            crate::template_parser::binding_parser::ParsedEventType::LegacyAnimation => ExprParsedEventType::Animation,
        };
        
        // Parse target_or_phase to target and phase
        let (target, phase) = if let Some(ref top) = e.target_or_phase {
            if top.contains(':') {
                let parts: Vec<&str> = top.split(':').collect();
                (Some(parts[0].to_string()), Some(parts[1..].join(":")))
            } else if top.contains('.') {
                let parts: Vec<&str> = top.split('.').collect();
                (Some(parts[0].to_string()), Some(parts[1..].join(".")))
            } else {
                (Some(top.clone()), None)
            }
        } else {
            (None, None)
        };
        
        bound_events.push(t::BoundEvent::new(
            e.name.clone(),
            expr_event_type,
            (*e.handler.ast).clone(),
            target,
            phase,
            e.source_span.clone(),
            e.handler_span.clone(),
            e.key_span.clone().unwrap_or_else(|| e.source_span.clone()),
        ));
    }
}

fn text_contents(node: &html::Element) -> Option<String> {
    if node.children.len() != 1 {
        return None;
    }
    match &node.children[0] {
        html::Node::Text(text) => Some(text.value.clone()),
        _ => None,
    }
}

fn is_i18n_root_node(i18n: &Option<i18n::I18nMeta>) -> bool {
    if let Some(meta) = i18n {
        matches!(meta, i18n::I18nMeta::Message(_))
    } else {
        false
    }
}

fn create_key_span(
    src_span: &ParseSourceSpan,
    attr_name: &str,
    normalized_name: &str,
    prefix: &str,
    identifier: &str,
) -> ParseSourceSpan {
    let normalization_adjustment = attr_name.len() - normalized_name.len();
    let key_span_start = src_span.start.move_by((prefix.len() + normalization_adjustment) as i32);
    let key_span_end = key_span_start.move_by(identifier.len() as i32);
    ParseSourceSpan::new(key_span_start, key_span_end)
}

