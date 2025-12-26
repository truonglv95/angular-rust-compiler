//! HTML Whitespace Processing
//!
//! Corresponds to packages/compiler/src/ml_parser/html_whitespaces.ts (353 lines)
//!
//! This visitor can walk HTML parse tree and remove / trim text nodes using the following rules:
//! - consider spaces, tabs and new lines as whitespace characters;
//! - drop text nodes consisting of whitespace characters only;
//! - for all other text nodes replace consecutive whitespace characters with one space;
//! - convert &ngsp; pseudo-entity to a single space;

use crate::ml_parser::ast::*;
use crate::ml_parser::entities::NGSP_UNICODE;
use crate::ml_parser::parser::ParseTreeResult;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, HashSet};

pub const PRESERVE_WS_ATTR_NAME: &str = "ngPreserveWhitespaces";

static SKIP_WS_TRIM_TAGS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();
    set.insert("pre");
    set.insert("template");
    set.insert("textarea");
    set.insert("script");
    set.insert("style");
    set
});

// Equivalent to \s with \u00a0 (non-breaking space) excluded.
// Based on https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/RegExp
const WS_CHARS: &str = " \u{000C}\n\r\t\u{000B}\u{1680}\u{180E}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}\u{FEFF}";

static NO_WS_REGEXP: Lazy<Regex> =
    Lazy::new(|| Regex::new(&format!("[^{}]", regex::escape(WS_CHARS))).unwrap());

static WS_REPLACE_REGEXP: Lazy<Regex> =
    Lazy::new(|| Regex::new(&format!("[{}]+", regex::escape(WS_CHARS))).unwrap());

fn has_preserve_whitespaces_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.name == PRESERVE_WS_ATTR_NAME)
}

/// &ngsp; is a placeholder for non-removable space
/// &ngsp; is converted to the 0xE500 PUA (Private Use Areas) unicode character
/// and later on replaced by a space.
pub fn replace_ngsp(value: &str) -> String {
    // lexer is replacing the &ngsp; pseudo-entity with NGSP_UNICODE
    value.replace(NGSP_UNICODE, " ")
}

/// Context for visiting sibling nodes
#[derive(Clone)]
pub struct SiblingVisitorContext {
    pub prev: Option<Box<Node>>,
    pub next: Option<Box<Node>>,
}

/// This visitor can walk HTML parse tree and remove / trim text nodes.
///
/// Removal and trimming of whitespaces have positive performance impact (less code to generate
/// while compiling templates, faster view creation). At the same time it can be "destructive"
/// in some cases (whitespaces can influence layout). Because of the potential of breaking layout
/// this visitor is not activated by default in Angular 5 and people need to explicitly opt-in for
/// whitespace removal.
///
/// If `original_node_map` is provided, the transformed nodes will be mapped back to their original
/// inputs. Any output nodes not in the map were not transformed. This supports correlating and
/// porting information between the trimmed nodes and original nodes (such as `i18n` properties).
pub struct WhitespaceVisitor {
    preserve_significant_whitespace: bool,
    original_node_map: Option<HashMap<String, Node>>,
    require_context: bool,
    icu_expansion_depth: usize,
}

impl WhitespaceVisitor {
    pub fn new(
        preserve_significant_whitespace: bool,
        original_node_map: Option<HashMap<String, Node>>,
        require_context: bool,
    ) -> Self {
        WhitespaceVisitor {
            preserve_significant_whitespace,
            original_node_map,
            require_context,
            icu_expansion_depth: 0,
        }
    }

    pub fn visit_element(
        &mut self,
        element: &Element,
        _context: Option<&SiblingVisitorContext>,
    ) -> Option<Node> {
        if SKIP_WS_TRIM_TAGS.contains(element.name.as_str())
            || has_preserve_whitespaces_attr(&element.attrs)
        {
            // don't descend into elements where we need to preserve whitespaces
            // but still visit all attributes to eliminate one used as a marker to preserve WS
            let new_attrs = visit_all_with_siblings_attrs(self, &element.attrs);
            let new_element = Element {
                name: element.name.clone(),
                attrs: new_attrs,
                directives: element.directives.clone(),
                children: element.children.clone(), // Keep children as-is
                is_self_closing: element.is_self_closing,
                source_span: element.source_span.clone(),
                start_source_span: element.start_source_span.clone(),
                end_source_span: element.end_source_span.clone(),
                is_void: element.is_void,
                i18n: element.i18n.clone(),
            };
            return Some(Node::Element(new_element));
        }

        let new_children = visit_all_with_siblings_nodes(self, &element.children);
        let new_element = Element {
            name: element.name.clone(),
            attrs: element.attrs.clone(),
            directives: element.directives.clone(),
            children: new_children,
            is_self_closing: element.is_self_closing,
            source_span: element.source_span.clone(),
            start_source_span: element.start_source_span.clone(),
            end_source_span: element.end_source_span.clone(),
            is_void: element.is_void,
            i18n: element.i18n.clone(),
        };
        Some(Node::Element(new_element))
    }

    pub fn visit_attribute(
        &self,
        attribute: &Attribute,
        _context: Option<&SiblingVisitorContext>,
    ) -> Option<Attribute> {
        if attribute.name != PRESERVE_WS_ATTR_NAME {
            Some(attribute.clone())
        } else {
            None
        }
    }

    pub fn visit_text(&self, text: &Text, context: Option<&SiblingVisitorContext>) -> Option<Node> {
        let is_not_blank = NO_WS_REGEXP.is_match(&text.value);

        let has_expansion_sibling = if let Some(ctx) = context {
            is_expansion_node(&ctx.prev) || is_expansion_node(&ctx.next)
        } else {
            false
        };

        // Do not trim whitespace within ICU expansions when preserving significant whitespace.
        let in_icu_expansion = self.icu_expansion_depth > 0;
        if in_icu_expansion && self.preserve_significant_whitespace {
            return Some(Node::Text(text.clone()));
        }

        if is_not_blank || has_expansion_sibling {
            // Process the whitespace of the value of this Text node
            let processed = process_whitespace(&text.value);

            let final_value = if self.preserve_significant_whitespace {
                processed
            } else {
                let trimmed = trim_leading_and_trailing_whitespace(&processed, context);
                // If trimming resulted in empty string but original wasn't blank, keep processed
                // This preserves nbsp and other non-WS_CHARS characters
                if trimmed.is_empty() && is_not_blank {
                    processed
                } else {
                    trimmed
                }
            };

            let result = Text {
                value: final_value,
                source_span: text.source_span.clone(),
                tokens: text.tokens.clone(),
                i18n: text.i18n.clone(),
            };
            return Some(Node::Text(result));
        }

        None
    }

    pub fn visit_comment(
        &self,
        comment: &Comment,
        _context: Option<&SiblingVisitorContext>,
    ) -> Option<Node> {
        Some(Node::Comment(comment.clone()))
    }

    pub fn visit_expansion(
        &mut self,
        expansion: &Expansion,
        _context: Option<&SiblingVisitorContext>,
    ) -> Option<Node> {
        self.icu_expansion_depth += 1;

        let new_cases = visit_all_with_siblings_expansion_cases(self, &expansion.cases);

        self.icu_expansion_depth -= 1;

        let new_expansion = Expansion {
            switch_value: expansion.switch_value.clone(),
            expansion_type: expansion.expansion_type.clone(),
            cases: new_cases,
            source_span: expansion.source_span.clone(),
            switch_value_source_span: expansion.switch_value_source_span.clone(),
            i18n: expansion.i18n.clone(),
        };

        Some(Node::Expansion(new_expansion))
    }

    pub fn visit_expansion_case(
        &mut self,
        expansion_case: &ExpansionCase,
        _context: Option<&SiblingVisitorContext>,
    ) -> ExpansionCase {
        let new_expression = visit_all_with_siblings_nodes(self, &expansion_case.expression);

        ExpansionCase {
            value: expansion_case.value.clone(),
            expression: new_expression,
            source_span: expansion_case.source_span.clone(),
            value_source_span: expansion_case.value_source_span.clone(),
            exp_source_span: expansion_case.exp_source_span.clone(),
        }
    }

    pub fn visit_block(
        &mut self,
        block: &Block,
        _context: Option<&SiblingVisitorContext>,
    ) -> Option<Node> {
        let new_children = visit_all_with_siblings_nodes(self, &block.children);

        let new_block = Block {
            name: block.name.clone(),
            parameters: block.parameters.clone(),
            has_opening_brace: block.has_opening_brace,
            children: new_children,
            source_span: block.source_span.clone(),
            name_span: block.name_span.clone(),
            start_source_span: block.start_source_span.clone(),
            end_source_span: block.end_source_span.clone(),
            i18n: block.i18n.clone(),
        };

        Some(Node::Block(new_block))
    }

    pub fn visit_block_parameter(
        &self,
        parameter: &BlockParameter,
        _context: Option<&SiblingVisitorContext>,
    ) -> Option<Node> {
        Some(Node::BlockParameter(parameter.clone()))
    }

    pub fn visit_let_declaration(
        &self,
        decl: &LetDeclaration,
        _context: Option<&SiblingVisitorContext>,
    ) -> Option<Node> {
        Some(Node::LetDeclaration(decl.clone()))
    }

    pub fn visit_component(
        &mut self,
        component: &Component,
        _context: Option<&SiblingVisitorContext>,
    ) -> Option<Node> {
        if SKIP_WS_TRIM_TAGS.contains(component.component_name.as_str())
            || has_preserve_whitespaces_attr(&component.attrs)
        {
            let new_attrs = visit_all_with_siblings_attrs(self, &component.attrs);
            let new_component = Component {
                component_name: component.component_name.clone(),
                tag_name: component.tag_name.clone(),
                full_name: component.full_name.clone(),
                attrs: new_attrs,
                directives: component.directives.clone(),
                children: component.children.clone(),
                is_self_closing: component.is_self_closing,
                source_span: component.source_span.clone(),
                start_source_span: component.start_source_span.clone(),
                end_source_span: component.end_source_span.clone(),
                i18n: component.i18n.clone(),
            };
            return Some(Node::Component(new_component));
        }

        let new_children = visit_all_with_siblings_nodes(self, &component.children);
        let new_component = Component {
            component_name: component.component_name.clone(),
            tag_name: component.tag_name.clone(),
            full_name: component.full_name.clone(),
            attrs: component.attrs.clone(),
            directives: component.directives.clone(),
            children: new_children,
            is_self_closing: component.is_self_closing,
            source_span: component.source_span.clone(),
            start_source_span: component.start_source_span.clone(),
            end_source_span: component.end_source_span.clone(),
            i18n: component.i18n.clone(),
        };
        Some(Node::Component(new_component))
    }

    pub fn visit_directive(
        &self,
        directive: &Directive,
        _context: Option<&SiblingVisitorContext>,
    ) -> Option<Node> {
        Some(Node::Directive(directive.clone()))
    }
}

fn is_expansion_node(node: &Option<Box<Node>>) -> bool {
    if let Some(n) = node {
        matches!(**n, Node::Expansion(_))
    } else {
        false
    }
}

/// Trim only characters in WS_CHARS from start (excludes nbsp \u{00A0})
fn trim_ws_start(text: &str) -> &str {
    text.trim_start_matches(|c: char| WS_CHARS.contains(c))
}

/// Trim only characters in WS_CHARS from end (excludes nbsp \u{00A0})
fn trim_ws_end(text: &str) -> &str {
    text.trim_end_matches(|c: char| WS_CHARS.contains(c))
}

fn trim_leading_and_trailing_whitespace(
    text: &str,
    context: Option<&SiblingVisitorContext>,
) -> String {
    let is_first_token_in_tag = context.map_or(true, |ctx| ctx.prev.is_none());
    let is_last_token_in_tag = context.map_or(true, |ctx| ctx.next.is_none());

    let maybe_trimmed_start = if is_first_token_in_tag {
        trim_ws_start(text)
    } else {
        text
    };

    let maybe_trimmed = if is_last_token_in_tag {
        trim_ws_end(maybe_trimmed_start)
    } else {
        maybe_trimmed_start
    };

    maybe_trimmed.to_string()
}

pub fn process_whitespace(text: &str) -> String {
    let replaced = replace_ngsp(text);
    let mut result = String::with_capacity(replaced.len());
    let mut last_was_ws = false;

    for c in replaced.chars() {
        if WS_CHARS.contains(c) {
            if !last_was_ws {
                result.push(' ');
                last_was_ws = true;
            }
        } else {
            result.push(c);
            last_was_ws = false;
        }
    }
    let result = result;
    result
}

/// Remove whitespaces from HTML AST
pub fn remove_whitespaces(
    html_ast_with_errors: ParseTreeResult,
    preserve_significant_whitespace: bool,
) -> ParseTreeResult {
    let mut visitor = WhitespaceVisitor::new(preserve_significant_whitespace, None, false);
    let root_nodes = visit_all_with_siblings_nodes(&mut visitor, &html_ast_with_errors.root_nodes);

    ParseTreeResult {
        root_nodes,
        errors: html_ast_with_errors.errors,
    }
}

/// Visit all nodes with sibling context
pub fn visit_all_with_siblings_nodes(visitor: &mut WhitespaceVisitor, nodes: &[Node]) -> Vec<Node> {
    let mut result = Vec::new();

    for (i, ast) in nodes.iter().enumerate() {
        let context = SiblingVisitorContext {
            prev: if i > 0 {
                Some(Box::new(nodes[i - 1].clone()))
            } else {
                None
            },
            next: if i < nodes.len() - 1 {
                Some(Box::new(nodes[i + 1].clone()))
            } else {
                None
            },
        };

        let ast_result = match ast {
            Node::Element(el) => visitor.visit_element(el, Some(&context)),
            Node::Text(text) => visitor.visit_text(text, Some(&context)),
            Node::Comment(comment) => visitor.visit_comment(comment, Some(&context)),
            Node::Expansion(expansion) => visitor.visit_expansion(expansion, Some(&context)),
            Node::Block(block) => visitor.visit_block(block, Some(&context)),
            Node::BlockParameter(param) => visitor.visit_block_parameter(param, Some(&context)),
            Node::LetDeclaration(decl) => visitor.visit_let_declaration(decl, Some(&context)),
            Node::Component(component) => visitor.visit_component(component, Some(&context)),
            Node::Directive(directive) => visitor.visit_directive(directive, Some(&context)),
            _ => Some(ast.clone()),
        };

        if let Some(result_node) = ast_result {
            result.push(result_node);
        }
    }

    result
}

fn visit_all_with_siblings_attrs(
    visitor: &WhitespaceVisitor,
    attrs: &[Attribute],
) -> Vec<Attribute> {
    let mut result = Vec::new();

    for attr in attrs {
        if let Some(new_attr) = visitor.visit_attribute(attr, None) {
            result.push(new_attr);
        }
    }

    result
}

fn visit_all_with_siblings_expansion_cases(
    visitor: &mut WhitespaceVisitor,
    cases: &[ExpansionCase],
) -> Vec<ExpansionCase> {
    cases
        .iter()
        .map(|case| visitor.visit_expansion_case(case, None))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_ngsp() {
        let input = format!("Hello{}World", NGSP_UNICODE);
        let result = replace_ngsp(&input);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_process_whitespace() {
        assert_eq!(process_whitespace("a  b"), "a b");
        assert_eq!(process_whitespace("a\t\tb"), "a b");
        assert_eq!(process_whitespace("a\n\nb"), "a b");
    }

    #[test]
    fn test_has_preserve_whitespaces_attr() {
        use crate::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};

        let file = ParseSourceFile::new(String::new(), "test.html".to_string());
        let location = ParseLocation::new(file, 0, 0, 0);
        let span = ParseSourceSpan::new(location.clone(), location);

        let attrs = vec![Attribute {
            name: "ngPreserveWhitespaces".to_string(),
            value: "true".to_string(),
            source_span: span.clone(),
            key_span: None,
            value_span: None,
            value_tokens: None,
            i18n: None,
        }];

        assert!(has_preserve_whitespaces_attr(&attrs));
    }
}
