//! XML Helper Module
//!
//! Corresponds to packages/compiler/src/i18n/serializers/xml_helper.ts
//! Helper functions and types for XML serialization

use std::collections::HashMap;

/// Visitor trait for XML nodes
pub trait IVisitor {
    fn visit_tag(&mut self, tag: &Tag) -> String;
    fn visit_text(&mut self, text: &Text) -> String;
    fn visit_declaration(&mut self, decl: &Declaration) -> String;
    fn visit_doctype(&mut self, doctype: &Doctype) -> String;
}

struct Visitor;

impl IVisitor for Visitor {
    fn visit_tag(&mut self, tag: &Tag) -> String {
        let str_attrs = self.serialize_attributes(&tag.attrs);

        if tag.children.is_empty() {
            return format!("<{}{}/>", tag.name, str_attrs);
        }

        let str_children: Vec<String> = tag
            .children
            .iter()
            .map(|node| node.visit(self))
            .collect();

        format!(
            "<{}{}>{}</{}>",
            tag.name,
            str_attrs,
            str_children.join(""),
            tag.name
        )
    }

    fn visit_text(&mut self, text: &Text) -> String {
        text.value.clone()
    }

    fn visit_declaration(&mut self, decl: &Declaration) -> String {
        let str_attrs = self.serialize_attributes(&decl.attrs);
        format!("<?xml{} ?>", str_attrs)
    }

    fn visit_doctype(&mut self, doctype: &Doctype) -> String {
        format!("<!DOCTYPE {} [\n{}\n]>", doctype.root_tag, doctype.dtd)
    }
}

impl Visitor {
    fn serialize_attributes(&self, attrs: &HashMap<String, String>) -> String {
        if attrs.is_empty() {
            return String::new();
        }

        let mut keys: Vec<_> = attrs.keys().collect();
        keys.sort();

        let str_attrs: String = keys
            .iter()
            .map(|name| format!("{}=\"{}\"", name, attrs[*name]))
            .collect::<Vec<_>>()
            .join(" ");

        format!(" {}", str_attrs)
    }
}

/// Serialize XML nodes to string
pub fn serialize(nodes: &[Box<dyn Node>]) -> String {
    let mut visitor = Visitor;
    nodes
        .iter()
        .map(|node| node.visit(&mut visitor))
        .collect::<Vec<_>>()
        .join("")
}

/// Base trait for all XML nodes
pub trait Node: std::fmt::Debug {
    fn visit(&self, visitor: &mut dyn IVisitor) -> String;
    fn clone_box(&self) -> Box<dyn Node>;
}

/// XML Declaration node
#[derive(Debug, Clone)]
pub struct Declaration {
    pub attrs: HashMap<String, String>,
}

impl Declaration {
    pub fn new(unescaped_attrs: HashMap<String, String>) -> Self {
        let mut attrs = HashMap::new();
        for (k, v) in unescaped_attrs {
            attrs.insert(k, escape_xml(&v));
        }
        Declaration { attrs }
    }
}

impl Node for Declaration {
    fn visit(&self, visitor: &mut dyn IVisitor) -> String {
        visitor.visit_declaration(self)
    }

    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(self.clone())
    }
}

/// XML Doctype node
#[derive(Debug, Clone)]
pub struct Doctype {
    pub root_tag: String,
    pub dtd: String,
}

impl Doctype {
    pub fn new(root_tag: String, dtd: String) -> Self {
        Doctype { root_tag, dtd }
    }
}

impl Node for Doctype {
    fn visit(&self, visitor: &mut dyn IVisitor) -> String {
        visitor.visit_doctype(self)
    }

    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(self.clone())
    }
}

/// XML Tag node
#[derive(Debug)]
pub struct Tag {
    pub name: String,
    pub attrs: HashMap<String, String>,
    pub children: Vec<Box<dyn Node>>,
}

impl Clone for Tag {
    fn clone(&self) -> Self {
        // Note: Cannot clone Box<dyn Node> as Node trait doesn't require Clone
        // This is a limitation that needs to be addressed
        // For now, we create a new Tag with empty children
        Tag {
            name: self.name.clone(),
            attrs: self.attrs.clone(),
            children: Vec::new(), // TODO: Implement proper cloning when Node trait supports it
        }
    }
}

impl Tag {
    pub fn new(
        name: String,
        unescaped_attrs: HashMap<String, String>,
        children: Vec<Box<dyn Node>>,
    ) -> Self {
        let mut attrs = HashMap::new();
        for (k, v) in unescaped_attrs {
            attrs.insert(k, escape_xml(&v));
        }
        Tag {
            name,
            attrs,
            children,
        }
    }
}

impl Node for Tag {
    fn visit(&self, visitor: &mut dyn IVisitor) -> String {
        visitor.visit_tag(self)
    }

    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(Tag {
            name: self.name.clone(),
            attrs: self.attrs.clone(),
            children: self.children.iter().map(|c| c.clone_box()).collect(),
        })
    }
}

/// XML Text node
#[derive(Debug, Clone)]
pub struct Text {
    pub value: String,
}

impl Text {
    pub fn new(unescaped_value: String) -> Self {
        Text {
            value: escape_xml(&unescaped_value),
        }
    }
}

impl Node for Text {
    fn visit(&self, visitor: &mut dyn IVisitor) -> String {
        visitor.visit_text(self)
    }

    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(self.clone())
    }
}

/// XML Carriage Return node (for formatting)
#[derive(Debug, Clone)]
pub struct CR {
    value: String,
}

impl CR {
    pub fn new(ws: usize) -> Self {
        CR {
            value: format!("\n{}", " ".repeat(ws)),
        }
    }
}

impl Node for CR {
    fn visit(&self, visitor: &mut dyn IVisitor) -> String {
        visitor.visit_text(&Text {
            value: self.value.clone(),
        })
    }

    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(self.clone())
    }
}

/// Escape special XML characters
pub fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

