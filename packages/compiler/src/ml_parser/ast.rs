//! ML Parser AST
//!
//! Corresponds to packages/compiler/src/ml_parser/ast.ts (302 lines)
//! HTML/XML Abstract Syntax Tree node definitions

use super::tokens::{InterpolatedAttributeToken, InterpolatedTextToken};
use crate::i18n::I18nMeta;
use crate::parse_util::ParseSourceSpan;
use std::sync::Arc;

/// Base trait for all AST nodes
pub trait BaseNode {
    fn source_span(&self) -> &ParseSourceSpan;
    fn visit(
        &self,
        visitor: &mut dyn Visitor,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>>;
}

/// Node type union
#[derive(Debug, Clone)]
pub enum Node {
    Attribute(Attribute),
    Comment(Comment),
    Element(Element),
    Expansion(Expansion),
    ExpansionCase(ExpansionCase),
    Text(Text),
    Block(Block),
    BlockParameter(BlockParameter),
    Component(Component),
    Directive(Directive),
    LetDeclaration(LetDeclaration),
}

/// Base node with i18n support
#[derive(Debug, Clone)]
pub struct NodeWithI18n {
    pub source_span: ParseSourceSpan,
    pub i18n: Option<I18nMeta>,
}

impl NodeWithI18n {
    pub fn new(source_span: ParseSourceSpan, i18n: Option<I18nMeta>) -> Self {
        NodeWithI18n { source_span, i18n }
    }
}

/// Text node
#[derive(Debug, Clone)]
pub struct Text {
    pub value: Arc<str>,
    pub source_span: ParseSourceSpan,
    pub tokens: Vec<InterpolatedTextToken>,
    pub i18n: Option<I18nMeta>,
}

impl Text {
    pub fn new(
        value: Arc<str>,
        source_span: ParseSourceSpan,
        tokens: Vec<InterpolatedTextToken>,
        i18n: Option<I18nMeta>,
    ) -> Self {
        Text {
            value,
            source_span,
            tokens,
            i18n,
        }
    }
}

/// Expansion (ICU message format)
#[derive(Debug, Clone)]
pub struct Expansion {
    pub switch_value: Arc<str>,
    pub expansion_type: Arc<str>,
    pub cases: Vec<ExpansionCase>,
    pub source_span: ParseSourceSpan,
    pub switch_value_source_span: ParseSourceSpan,
    pub i18n: Option<I18nMeta>,
}

/// Expansion case
#[derive(Debug, Clone)]
pub struct ExpansionCase {
    pub value: Arc<str>,
    pub expression: Vec<Node>,
    pub source_span: ParseSourceSpan,
    pub value_source_span: ParseSourceSpan,
    pub exp_source_span: ParseSourceSpan,
}

/// Attribute node
#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: Arc<str>,
    pub value: Arc<str>,
    pub source_span: ParseSourceSpan,
    pub key_span: Option<ParseSourceSpan>,
    pub value_span: Option<ParseSourceSpan>,
    pub value_tokens: Option<Vec<InterpolatedAttributeToken>>,
    pub i18n: Option<I18nMeta>,
}

/// Element node
#[derive(Debug, Clone)]
pub struct Element {
    pub name: Arc<str>,
    pub attrs: Vec<Attribute>,
    pub directives: Vec<Directive>,
    pub children: Vec<Node>,
    pub is_self_closing: bool,
    pub source_span: ParseSourceSpan,
    pub start_source_span: ParseSourceSpan,
    pub end_source_span: Option<ParseSourceSpan>,
    pub is_void: bool,
    pub i18n: Option<I18nMeta>,
}

/// Comment node
#[derive(Debug, Clone)]
pub struct Comment {
    pub value: Option<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

impl Comment {
    pub fn new(value: Option<Arc<str>>, source_span: ParseSourceSpan) -> Self {
        Comment { value, source_span }
    }
}

/// Block node (@if, @for, @switch)
#[derive(Debug, Clone)]
pub struct Block {
    pub name: Arc<str>,
    pub parameters: Vec<BlockParameter>,
    pub has_opening_brace: bool, // Track if block received { token
    pub children: Vec<Node>,
    pub source_span: ParseSourceSpan,
    pub name_span: ParseSourceSpan,
    pub start_source_span: ParseSourceSpan,
    pub end_source_span: Option<ParseSourceSpan>,
    pub i18n: Option<I18nMeta>,
}

/// Component node (Angular component usage)
#[derive(Debug, Clone)]
pub struct Component {
    pub component_name: Arc<str>,
    pub tag_name: Option<Arc<str>>,
    pub full_name: Arc<str>,
    pub attrs: Vec<Attribute>,
    pub directives: Vec<Directive>,
    pub children: Vec<Node>,
    pub is_self_closing: bool,
    pub source_span: ParseSourceSpan,
    pub start_source_span: ParseSourceSpan,
    pub end_source_span: Option<ParseSourceSpan>,
    pub i18n: Option<I18nMeta>,
}

/// Directive node
#[derive(Debug, Clone)]
pub struct Directive {
    pub name: Arc<str>,
    pub attrs: Vec<Attribute>,
    pub source_span: ParseSourceSpan,
    pub start_source_span: ParseSourceSpan,
    pub end_source_span: Option<ParseSourceSpan>,
}

/// Block parameter
#[derive(Debug, Clone)]
pub struct BlockParameter {
    pub expression: Arc<str>,
    pub source_span: ParseSourceSpan,
}

impl BlockParameter {
    pub fn new(expression: Arc<str>, source_span: ParseSourceSpan) -> Self {
        BlockParameter {
            expression,
            source_span,
        }
    }
}

/// Let declaration (@let x = value)
#[derive(Debug, Clone)]
pub struct LetDeclaration {
    pub name: Arc<str>,
    pub value: Arc<str>,
    pub source_span: ParseSourceSpan,
    pub name_span: ParseSourceSpan,
    pub value_span: ParseSourceSpan,
}

/// Visitor trait for traversing AST
pub trait Visitor {
    fn visit(
        &mut self,
        _node: &Node,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None
    }

    fn visit_element(
        &mut self,
        element: &Element,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>>;
    fn visit_attribute(
        &mut self,
        attribute: &Attribute,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>>;
    fn visit_text(
        &mut self,
        text: &Text,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>>;
    fn visit_comment(
        &mut self,
        comment: &Comment,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>>;
    fn visit_expansion(
        &mut self,
        expansion: &Expansion,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>>;
    fn visit_expansion_case(
        &mut self,
        expansion_case: &ExpansionCase,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>>;
    fn visit_block(
        &mut self,
        block: &Block,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>>;
    fn visit_block_parameter(
        &mut self,
        parameter: &BlockParameter,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>>;
    fn visit_let_declaration(
        &mut self,
        decl: &LetDeclaration,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>>;
    fn visit_component(
        &mut self,
        component: &Component,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>>;
    fn visit_directive(
        &mut self,
        directive: &Directive,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>>;
}

/// Visit all nodes in array
pub fn visit_all(
    visitor: &mut dyn Visitor,
    nodes: &[Node],
    context: &mut dyn std::any::Any,
) -> Vec<Box<dyn std::any::Any>> {
    let mut result = Vec::new();

    for node in nodes {
        let node_result = match node {
            Node::Element(e) => visitor.visit_element(e, context),
            Node::Attribute(a) => visitor.visit_attribute(a, context),
            Node::Text(t) => visitor.visit_text(t, context),
            Node::Comment(c) => visitor.visit_comment(c, context),
            Node::Expansion(e) => visitor.visit_expansion(e, context),
            Node::ExpansionCase(e) => visitor.visit_expansion_case(e, context),
            Node::Block(b) => visitor.visit_block(b, context),
            Node::BlockParameter(p) => visitor.visit_block_parameter(p, context),
            Node::Component(c) => visitor.visit_component(c, context),
            Node::Directive(d) => visitor.visit_directive(d, context),
            Node::LetDeclaration(l) => visitor.visit_let_declaration(l, context),
        };

        if let Some(res) = node_result {
            result.push(res);
        }
    }

    result
}

/// Recursive visitor implementation
pub struct RecursiveVisitor;

impl RecursiveVisitor {
    pub fn new() -> Self {
        RecursiveVisitor
    }
}

impl Visitor for RecursiveVisitor {
    fn visit_element(
        &mut self,
        element: &Element,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        visit_all(self, &element.children, context);
        None
    }

    fn visit_attribute(
        &mut self,
        _attribute: &Attribute,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None
    }

    fn visit_text(
        &mut self,
        _text: &Text,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None
    }

    fn visit_comment(
        &mut self,
        _comment: &Comment,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None
    }

    fn visit_expansion(
        &mut self,
        expansion: &Expansion,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        for case in &expansion.cases {
            self.visit_expansion_case(case, context);
        }
        None
    }

    fn visit_expansion_case(
        &mut self,
        expansion_case: &ExpansionCase,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        visit_all(self, &expansion_case.expression, context);
        None
    }

    fn visit_block(
        &mut self,
        block: &Block,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        visit_all(self, &block.children, context);
        None
    }

    fn visit_block_parameter(
        &mut self,
        _parameter: &BlockParameter,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None
    }

    fn visit_let_declaration(
        &mut self,
        _decl: &LetDeclaration,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None
    }

    fn visit_component(
        &mut self,
        component: &Component,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        visit_all(self, &component.children, context);
        None
    }

    fn visit_directive(
        &mut self,
        _directive: &Directive,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None
    }
}

impl Default for RecursiveVisitor {
    fn default() -> Self {
        Self::new()
    }
}
