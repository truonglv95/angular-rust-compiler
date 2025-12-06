//! I18n AST Module
//!
//! Corresponds to packages/compiler/src/i18n/i18n_ast.ts
//! Defines the AST nodes for internationalization messages

use crate::parse_util::ParseSourceSpan;
use std::collections::HashMap;

/// Describes the text contents of a placeholder as it appears in an ICU expression,
/// including its source span information.
#[derive(Debug, Clone)]
pub struct MessagePlaceholder {
    /// The text contents of the placeholder
    pub text: String,
    /// The source span of the placeholder
    pub source_span: ParseSourceSpan,
}

/// Line and columns indexes are 1 based
#[derive(Debug, Clone)]
pub struct MessageSpan {
    pub file_path: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

/// Represents an i18n message
#[derive(Debug, Clone)]
pub struct Message {
    pub nodes: Vec<Node>,
    pub placeholders: HashMap<String, MessagePlaceholder>,
    pub placeholder_to_message: HashMap<String, Box<Message>>,
    pub meaning: String,
    pub description: String,
    pub custom_id: String,
    pub sources: Vec<MessageSpan>,
    pub id: String,
    /// The ids to use if there are no custom id and if `i18nLegacyMessageIdFormat` is not empty
    pub legacy_ids: Vec<String>,
    pub message_string: String,
}

impl Message {
    pub fn new(
        nodes: Vec<Node>,
        placeholders: HashMap<String, MessagePlaceholder>,
        placeholder_to_message: HashMap<String, Box<Message>>,
        meaning: String,
        description: String,
        custom_id: String,
    ) -> Self {
        let id = custom_id.clone();
        let message_string = serialize_message(&nodes);

        let sources = if !nodes.is_empty() {
            vec![MessageSpan {
                file_path: nodes[0].source_span().start.file.url.clone(),
                start_line: nodes[0].source_span().start.line + 1,
                start_col: nodes[0].source_span().start.col + 1,
                end_line: nodes[nodes.len() - 1].source_span().end.line + 1,
                end_col: nodes[0].source_span().start.col + 1,
            }]
        } else {
            vec![]
        };

        Message {
            nodes,
            placeholders,
            placeholder_to_message,
            meaning,
            description,
            custom_id,
            sources,
            id,
            legacy_ids: vec![],
            message_string,
        }
    }
}

/// Base trait for all i18n AST nodes
pub trait NodeTrait {
    fn source_span(&self) -> &ParseSourceSpan;
    fn visit<V: Visitor>(&self, visitor: &mut V, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any>;
}

/// Enum representing all possible i18n AST node types
#[derive(Debug, Clone)]
pub enum Node {
    Text(Text),
    Container(Container),
    Icu(Icu),
    TagPlaceholder(TagPlaceholder),
    Placeholder(Placeholder),
    IcuPlaceholder(IcuPlaceholder),
    BlockPlaceholder(BlockPlaceholder),
}

impl Node {
    pub fn source_span(&self) -> &ParseSourceSpan {
        match self {
            Node::Text(n) => &n.source_span,
            Node::Container(n) => &n.source_span,
            Node::Icu(n) => &n.source_span,
            Node::TagPlaceholder(n) => &n.source_span,
            Node::Placeholder(n) => &n.source_span,
            Node::IcuPlaceholder(n) => &n.source_span,
            Node::BlockPlaceholder(n) => &n.source_span,
        }
    }

    pub fn visit<V: Visitor>(&self, visitor: &mut V, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        match self {
            Node::Text(n) => visitor.visit_text(n, context),
            Node::Container(n) => visitor.visit_container(n, context),
            Node::Icu(n) => visitor.visit_icu(n, context),
            Node::TagPlaceholder(n) => visitor.visit_tag_placeholder(n, context),
            Node::Placeholder(n) => visitor.visit_placeholder(n, context),
            Node::IcuPlaceholder(n) => visitor.visit_icu_placeholder(n, context),
            Node::BlockPlaceholder(n) => visitor.visit_block_placeholder(n, context),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    pub value: String,
    pub source_span: ParseSourceSpan,
}

impl Text {
    pub fn new(value: String, source_span: ParseSourceSpan) -> Self {
        Text { value, source_span }
    }
}

#[derive(Debug, Clone)]
pub struct Container {
    pub children: Vec<Node>,
    pub source_span: ParseSourceSpan,
}

impl Container {
    pub fn new(children: Vec<Node>, source_span: ParseSourceSpan) -> Self {
        Container {
            children,
            source_span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Icu {
    pub expression: String,
    pub type_: String,
    pub cases: HashMap<String, Node>,
    pub source_span: ParseSourceSpan,
    pub expression_placeholder: Option<String>,
}

impl Icu {
    pub fn new(
        expression: String,
        type_: String,
        cases: HashMap<String, Node>,
        source_span: ParseSourceSpan,
        expression_placeholder: Option<String>,
    ) -> Self {
        Icu {
            expression,
            type_,
            cases,
            source_span,
            expression_placeholder,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TagPlaceholder {
    pub tag: String,
    pub attrs: HashMap<String, String>,
    pub start_name: String,
    pub close_name: String,
    pub children: Vec<Node>,
    pub is_void: bool,
    pub source_span: ParseSourceSpan,
    pub start_source_span: Option<ParseSourceSpan>,
    pub end_source_span: Option<ParseSourceSpan>,
}

impl TagPlaceholder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tag: String,
        attrs: HashMap<String, String>,
        start_name: String,
        close_name: String,
        children: Vec<Node>,
        is_void: bool,
        source_span: ParseSourceSpan,
        start_source_span: Option<ParseSourceSpan>,
        end_source_span: Option<ParseSourceSpan>,
    ) -> Self {
        TagPlaceholder {
            tag,
            attrs,
            start_name,
            close_name,
            children,
            is_void,
            source_span,
            start_source_span,
            end_source_span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Placeholder {
    pub value: String,
    pub name: String,
    pub source_span: ParseSourceSpan,
}

impl Placeholder {
    pub fn new(value: String, name: String, source_span: ParseSourceSpan) -> Self {
        Placeholder {
            value,
            name,
            source_span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IcuPlaceholder {
    pub value: Icu,
    pub name: String,
    pub source_span: ParseSourceSpan,
    /// Used to capture a message computed from a previous processing pass (see `setI18nRefs()`)
    pub previous_message: Option<Box<Message>>,
}

impl IcuPlaceholder {
    pub fn new(value: Icu, name: String, source_span: ParseSourceSpan) -> Self {
        IcuPlaceholder {
            value,
            name,
            source_span,
            previous_message: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockPlaceholder {
    pub name: String,
    pub parameters: Vec<String>,
    pub start_name: String,
    pub close_name: String,
    pub children: Vec<Node>,
    pub source_span: ParseSourceSpan,
    pub start_source_span: Option<ParseSourceSpan>,
    pub end_source_span: Option<ParseSourceSpan>,
}

impl BlockPlaceholder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        parameters: Vec<String>,
        start_name: String,
        close_name: String,
        children: Vec<Node>,
        source_span: ParseSourceSpan,
        start_source_span: Option<ParseSourceSpan>,
        end_source_span: Option<ParseSourceSpan>,
    ) -> Self {
        BlockPlaceholder {
            name,
            parameters,
            start_name,
            close_name,
            children,
            source_span,
            start_source_span,
            end_source_span,
        }
    }
}

/// Each HTML node that is affected by an i18n tag will also have an `i18n` property
/// that is of type `I18nMeta`.
/// This information is either a `Message`, which indicates it is the root of an i18n message,
/// or a `Node`, which indicates it is part of a containing `Message`.
#[derive(Debug, Clone)]
pub enum I18nMeta {
    Message(Message),
    Node(Node),
}

/// Visitor trait for traversing i18n AST
pub trait Visitor {
    fn visit_text(&mut self, text: &Text, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any>;
    fn visit_container(&mut self, container: &Container, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any>;
    fn visit_icu(&mut self, icu: &Icu, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any>;
    fn visit_tag_placeholder(&mut self, ph: &TagPlaceholder, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any>;
    fn visit_placeholder(&mut self, ph: &Placeholder, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any>;
    fn visit_icu_placeholder(&mut self, ph: &IcuPlaceholder, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any>;
    fn visit_block_placeholder(&mut self, ph: &BlockPlaceholder, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any>;
}

/// Clone visitor - clones the AST
pub struct CloneVisitor;

impl Visitor for CloneVisitor {
    fn visit_text(&mut self, text: &Text, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        Box::new(Node::Text(Text::new(text.value.clone(), text.source_span.clone())))
    }

    fn visit_container(&mut self, container: &Container, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        // Note: context cannot be moved in closure, so we pass None for now
        // This needs proper implementation with context handling
        let children = container
            .children
            .iter()
            .map(|n| {
                let result = n.visit(self, None); // TODO: Fix context passing
                *result.downcast::<Node>().unwrap()
            })
            .collect();
        Box::new(Node::Container(Container::new(children, container.source_span.clone())))
    }

    fn visit_icu(&mut self, icu: &Icu, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let mut cases = HashMap::new();
        for (key, node) in &icu.cases {
            let result = node.visit(self, None); // TODO: Fix context passing
            cases.insert(key.clone(), *result.downcast::<Node>().unwrap());
        }
        Box::new(Node::Icu(Icu::new(
            icu.expression.clone(),
            icu.type_.clone(),
            cases,
            icu.source_span.clone(),
            icu.expression_placeholder.clone(),
        )))
    }

    fn visit_tag_placeholder(&mut self, ph: &TagPlaceholder, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let children = ph
            .children
            .iter()
            .map(|n| {
                let result = n.visit(self, None); // TODO: Fix context passing
                *result.downcast::<Node>().unwrap()
            })
            .collect();
        Box::new(Node::TagPlaceholder(TagPlaceholder::new(
            ph.tag.clone(),
            ph.attrs.clone(),
            ph.start_name.clone(),
            ph.close_name.clone(),
            children,
            ph.is_void,
            ph.source_span.clone(),
            ph.start_source_span.clone(),
            ph.end_source_span.clone(),
        )))
    }

    fn visit_placeholder(&mut self, ph: &Placeholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        Box::new(Node::Placeholder(Placeholder::new(
            ph.value.clone(),
            ph.name.clone(),
            ph.source_span.clone(),
        )))
    }

    fn visit_icu_placeholder(&mut self, ph: &IcuPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        Box::new(Node::IcuPlaceholder(IcuPlaceholder::new(
            ph.value.clone(),
            ph.name.clone(),
            ph.source_span.clone(),
        )))
    }

    fn visit_block_placeholder(&mut self, ph: &BlockPlaceholder, context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let children = ph
            .children
            .iter()
            .map(|n| {
                let result = n.visit(self, None); // TODO: Fix context passing
                *result.downcast::<Node>().unwrap()
            })
            .collect();
        Box::new(Node::BlockPlaceholder(BlockPlaceholder::new(
            ph.name.clone(),
            ph.parameters.clone(),
            ph.start_name.clone(),
            ph.close_name.clone(),
            children,
            ph.source_span.clone(),
            ph.start_source_span.clone(),
            ph.end_source_span.clone(),
        )))
    }
}

/// Recurse visitor - visits all nodes recursively
pub struct RecurseVisitor;

impl Visitor for RecurseVisitor {
    fn visit_text(&mut self, _text: &Text, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        Box::new(())
    }

    fn visit_container(&mut self, container: &Container, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        for child in &container.children {
            child.visit(self, None);
        }
        Box::new(())
    }

    fn visit_icu(&mut self, icu: &Icu, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        for node in icu.cases.values() {
            node.visit(self, None);
        }
        Box::new(())
    }

    fn visit_tag_placeholder(&mut self, ph: &TagPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        for child in &ph.children {
            child.visit(self, None);
        }
        Box::new(())
    }

    fn visit_placeholder(&mut self, _ph: &Placeholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        Box::new(())
    }

    fn visit_icu_placeholder(&mut self, _ph: &IcuPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        Box::new(())
    }

    fn visit_block_placeholder(&mut self, ph: &BlockPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        for child in &ph.children {
            child.visit(self, None);
        }
        Box::new(())
    }
}

/// Serialize the message to the Localize backtick string format that would appear in compiled code.
fn serialize_message(message_nodes: &[Node]) -> String {
    let mut visitor = LocalizeMessageStringVisitor;
    message_nodes
        .iter()
        .map(|n| {
            let result = n.visit(&mut visitor, None);
            *result.downcast::<String>().unwrap()
        })
        .collect::<Vec<_>>()
        .join("")
}

struct LocalizeMessageStringVisitor;

impl Visitor for LocalizeMessageStringVisitor {
    fn visit_text(&mut self, text: &Text, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        Box::new(text.value.clone())
    }

    fn visit_container(&mut self, container: &Container, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let result = container
            .children
            .iter()
            .map(|child| {
                let r = child.visit(self, None);
                *r.downcast::<String>().unwrap()
            })
            .collect::<Vec<_>>()
            .join("");
        Box::new(result)
    }

    fn visit_icu(&mut self, icu: &Icu, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let mut str_cases = Vec::new();
        for (k, v) in &icu.cases {
            let case_result = v.visit(self, None);
            let case_str = *case_result.downcast::<String>().unwrap();
            str_cases.push(format!("{} {{{}}}", k, case_str));
        }
        let result = format!(
            "{{${}, {}, {}}}",
            icu.expression_placeholder.as_ref().unwrap_or(&icu.expression),
            icu.type_,
            str_cases.join(" ")
        );
        Box::new(result)
    }

    fn visit_tag_placeholder(&mut self, ph: &TagPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let children = ph
            .children
            .iter()
            .map(|child| {
                let r = child.visit(self, None);
                *r.downcast::<String>().unwrap()
            })
            .collect::<Vec<_>>()
            .join("");
        let result = format!("{{${}}}{}{{${}}}", ph.start_name, children, ph.close_name);
        Box::new(result)
    }

    fn visit_placeholder(&mut self, ph: &Placeholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        Box::new(format!("{{${}}}", ph.name))
    }

    fn visit_icu_placeholder(&mut self, ph: &IcuPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        Box::new(format!("{{${}}}", ph.name))
    }

    fn visit_block_placeholder(&mut self, ph: &BlockPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let children = ph
            .children
            .iter()
            .map(|child| {
                let r = child.visit(self, None);
                *r.downcast::<String>().unwrap()
            })
            .collect::<Vec<_>>()
            .join("");
        let result = format!("{{${}}}{}{{${}}}", ph.start_name, children, ph.close_name);
        Box::new(result)
    }
}

