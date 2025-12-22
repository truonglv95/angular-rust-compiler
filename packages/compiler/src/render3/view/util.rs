//! Render3 View Utilities
//!
//! Corresponds to packages/compiler/src/render3/view/util.ts
//! Contains utility functions for view compilation

use indexmap::IndexMap;

use lazy_static::lazy_static;
use regex::Regex;

use crate::core::InputFlags;
use crate::expression_parser::ast::BindingType;
use crate::ml_parser::tags::split_ns_name;
use crate::output::output_ast::{
    Expression, Statement, LiteralExpr, LiteralValue, LiteralArrayExpr, LiteralMapExpr,
    LiteralMapEntry, DeclareVarStmt, ReadVarExpr,
};
use crate::directive_matching::CssSelector;
use crate::render3::r3_ast as t;

lazy_static! {
    /// Checks whether an object key contains potentially unsafe chars
    pub static ref UNSAFE_OBJECT_KEY_NAME_REGEXP: Regex = Regex::new(r"[-.']").unwrap();
}

/// Name of the temporary to use during data binding
pub const TEMPORARY_NAME: &str = "_t";

/// Name of the context parameter passed into a template function
pub const CONTEXT_NAME: &str = "ctx";

/// Name of the RenderFlag passed into a template function
pub const RENDER_FLAGS: &str = "rf";

/// Creates an allocator for a temporary variable.
pub struct TemporaryAllocator {
    temp: Option<ReadVarExpr>,
    name: String,
    statements: Vec<Statement>,
}

impl TemporaryAllocator {
    pub fn new(name: String) -> Self {
        TemporaryAllocator {
            temp: None,
            name,
            statements: vec![],
        }
    }

    pub fn allocate(&mut self) -> ReadVarExpr {
        if self.temp.is_none() {
            self.statements.push(Statement::DeclareVar(DeclareVarStmt {
                name: TEMPORARY_NAME.to_string(),
                value: None,
                type_: None,
                modifiers: crate::output::output_ast::StmtModifier::None,
                source_span: None,
            }));
            self.temp = Some(ReadVarExpr {
                name: self.name.clone(),
                type_: None,
                source_span: None,
            });
        }
        self.temp.clone().unwrap()
    }

    pub fn get_statements(&self) -> &[Statement] {
        &self.statements
    }
}

/// Helper to create literal expression
fn literal_expr(value: LiteralValue) -> Expression {
    Expression::Literal(LiteralExpr {
        value,
        type_: None,
        source_span: None,
    })
}

/// Converts a value to a literal expression
pub fn as_literal(value: &serde_json::Value) -> Expression {
    match value {
        serde_json::Value::Array(arr) => {
            let entries: Vec<Expression> = arr.iter().map(as_literal).collect();
            Expression::LiteralArray(LiteralArrayExpr {
                entries,
                type_: None,
                source_span: None,
            })
        }
        serde_json::Value::Bool(b) => literal_expr(LiteralValue::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                literal_expr(LiteralValue::Number(i as f64))
            } else if let Some(f) = n.as_f64() {
                literal_expr(LiteralValue::Number(f))
            } else {
                literal_expr(LiteralValue::Null)
            }
        }
        serde_json::Value::String(s) => literal_expr(LiteralValue::String(s.clone())),
        serde_json::Value::Null => literal_expr(LiteralValue::Null),
        serde_json::Value::Object(_) => {
            // For objects, we'd need to convert to LiteralMapExpr
            literal_expr(LiteralValue::Null)
        }
    }
}

/// Converts a string to a literal expression
pub fn as_literal_string(value: &str) -> Expression {
    literal_expr(LiteralValue::String(value.to_string()))
}

/// Input metadata for directive binding
#[derive(Debug, Clone)]
pub struct InputBindingMetadata {
    pub class_property_name: String,
    pub binding_property_name: String,
    pub transform_function: Option<Expression>,
    pub is_signal: bool,
}

/// Serializes inputs and outputs for `defineDirective` and `defineComponent`.
pub fn conditionally_create_directive_binding_literal(
    map: &IndexMap<String, InputBindingValue>,
    for_inputs: bool,
) -> Option<LiteralMapExpr> {
    if map.is_empty() {
        return None;
    }

    let entries: Vec<LiteralMapEntry> = map
        .iter()
        .map(|(key, value)| {
            let expression_value = match value {
                InputBindingValue::Simple(public_name) => {
                    as_literal_string(public_name)
                }
                InputBindingValue::Complex(meta) => {
                    let different_declaring_name = meta.binding_property_name != meta.class_property_name;
                    let has_decorator_input_transform = meta.transform_function.is_some();
                    
                    // Combine flags using bitwise OR on the underlying values
                    let mut flags_value: u32 = InputFlags::None as u32;
                    if meta.is_signal {
                        flags_value |= InputFlags::SignalBased as u32;
                    }
                    if has_decorator_input_transform {
                        flags_value |= InputFlags::HasDecoratorInputTransform as u32;
                    }

                    if for_inputs && (different_declaring_name || has_decorator_input_transform || flags_value != 0) {
                        let mut result = vec![
                            literal_expr(LiteralValue::Number(flags_value as f64)),
                            as_literal_string(&meta.binding_property_name),
                        ];

                        if different_declaring_name || has_decorator_input_transform {
                            result.push(as_literal_string(&meta.class_property_name));

                            if let Some(ref transform) = meta.transform_function {
                                result.push(transform.clone());
                            }
                        }

                        Expression::LiteralArray(LiteralArrayExpr {
                            entries: result,
                            type_: None,
                            source_span: None,
                        })
                    } else {
                        as_literal_string(&meta.binding_property_name)
                    }
                }
            };

            LiteralMapEntry {
                key: key.clone(),
                value: Box::new(expression_value),
                quoted: UNSAFE_OBJECT_KEY_NAME_REGEXP.is_match(key),
            }
        })
        .collect();

    Some(LiteralMapExpr {
        entries,
        type_: None,
        source_span: None,
    })
}

/// Value for input binding - either simple string or complex metadata
#[derive(Debug, Clone)]
pub enum InputBindingValue {
    Simple(String),
    Complex(InputBindingMetadata),
}

/// A representation for an object literal used during codegen of definition objects.
#[derive(Debug, Clone, Default)]
pub struct DefinitionMap {
    pub values: Vec<DefinitionMapEntry>,
}

#[derive(Debug, Clone)]
pub struct DefinitionMapEntry {
    pub key: String,
    pub quoted: bool,
    pub value: Expression,
}

impl DefinitionMap {
    pub fn new() -> Self {
        DefinitionMap { values: vec![] }
    }

    pub fn set(&mut self, key: &str, value: Option<Expression>) {
        if let Some(val) = value {
            if let Some(existing) = self.values.iter_mut().find(|v| v.key == key) {
                existing.value = val;
            } else {
                self.values.push(DefinitionMapEntry {
                    key: key.to_string(),
                    value: val,
                    quoted: false,
                });
            }
        }
    }

    pub fn to_literal_map(&self) -> LiteralMapExpr {
        let entries: Vec<LiteralMapEntry> = self
            .values
            .iter()
            .map(|entry| LiteralMapEntry {
                key: entry.key.clone(),
                value: Box::new(entry.value.clone()),
                quoted: entry.quoted,
            })
            .collect();
        LiteralMapExpr {
            entries,
            type_: None,
            source_span: None,
        }
    }
}

/// Creates a `CssSelector` from an AST node.
pub fn create_css_selector_from_node(node: &t::R3Node) -> Option<CssSelector> {
    let (element_name, attributes, inputs, outputs) = match node {
        t::R3Node::Element(el) => (
            el.name.clone(),
            &el.attributes,
            &el.inputs,
            &el.outputs,
        ),
        t::R3Node::Template(tpl) => (
            tpl.tag_name.clone().unwrap_or_else(|| "ng-template".to_string()),
            &tpl.attributes,
            &tpl.inputs,
            &tpl.outputs,
        ),
        _ => return None,
    };

    let mut css_selector = CssSelector::new();
    // split_ns_name returns Result<(Option<String>, String), String>
    if let Ok((_, element_name_no_ns)) = split_ns_name(&element_name, false) {
        css_selector.set_element(&element_name_no_ns);
    }

    // Add attributes
    for attr in attributes {
        if let Ok((_, name_no_ns)) = split_ns_name(&attr.name, false) {
            css_selector.add_attribute(&name_no_ns, &attr.value);
            
            if attr.name.to_lowercase() == "class" {
                let classes: Vec<&str> = attr.value.trim().split_whitespace().collect();
                for class_name in classes {
                    css_selector.add_class_name(class_name);
                }
            }
        }
    }

    // Add inputs as empty attributes for matching
    for input in inputs {
        if input.type_ == BindingType::Property || input.type_ == BindingType::TwoWay {
            css_selector.add_attribute(&input.name, "");
        }
    }

    // Add outputs as empty attributes for matching
    for output in outputs {
        css_selector.add_attribute(&output.name, "");
    }

    // For Templates, also include template_attrs (from structural directives)
    if let t::R3Node::Template(tpl) = node {
        for attr in &tpl.template_attrs {
            match attr {
                t::TemplateAttr::Text(text_attr) => {
                    if let Ok((_, name_no_ns)) = split_ns_name(&text_attr.name, false) {
                        css_selector.add_attribute(&name_no_ns, &text_attr.value);
                    }
                }
                t::TemplateAttr::Bound(bound_attr) => {
                     // Bound attributes (e.g. from *ngIf="value") usually match as input [ngIf]
                     // But strictly speaking, the attribute name itself should be present.
                     // Synthesized structural directives often appear as Text attributes (e.g. *f -> f="")
                     // But if it is bound, we add the name.
                     if let Ok((_, name_no_ns)) = split_ns_name(&bound_attr.name, false) {
                        css_selector.add_attribute(&name_no_ns, "");
                     }
                }
            }
        }
    }

    Some(css_selector)
}




