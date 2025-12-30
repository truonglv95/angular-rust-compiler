//! I18n Parser Module
//!
//! Corresponds to packages/compiler/src/i18n/i18n_parser.ts
//! Converts HTML AST to i18n messages

use crate::expression_parser::parser::Parser as ExpressionParser;
use crate::expression_parser::serializer::serialize;
use crate::i18n::i18n_ast::{
    self as i18n, BlockPlaceholder, Container, Icu, IcuPlaceholder, Message, Placeholder,
    TagPlaceholder, Text as I18nText,
};
use crate::i18n::serializers::placeholder::PlaceholderRegistry;
use crate::ml_parser::ast as html;
use crate::ml_parser::html_tags::get_html_tag_definition;
use crate::ml_parser::tags::TagDefinition;
use crate::ml_parser::tokens::{InterpolationToken, Token};
use crate::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{HashMap, HashSet};

lazy_static! {
    // Match both single and double quotes separately since Rust regex doesn't support backreferences
    // Using regular string with escaped backslashes for regex patterns
    static ref CUSTOM_PH_EXP_SINGLE: Regex = Regex::new("//[\\s\\S]*i18n[\\s\\S]*\\([\\s\\S]*ph[\\s\\S]*=[\\s\\S]*'([\\s\\S]*?)'[\\s\\S]*\\)").unwrap();
    static ref CUSTOM_PH_EXP_DOUBLE: Regex = Regex::new("//[\\s\\S]*i18n[\\s\\S]*\\([\\s\\S]*ph[\\s\\S]*=[\\s\\S]*\"([\\s\\S]*?)\"[\\s\\S]*\\)").unwrap();
}

/// Type for node visitor function
pub type VisitNodeFn = fn(&html::Node, &i18n::Node) -> i18n::Node;

/// Factory for creating i18n messages
pub trait I18nMessageFactory {
    fn create_message(
        &mut self,
        nodes: &[html::Node],
        meaning: Option<&str>,
        description: Option<&str>,
        custom_id: Option<&str>,
        visit_node_fn: Option<VisitNodeFn>,
    ) -> Message;
}

/// Returns a function converting html nodes to an i18n Message
pub fn create_i18n_message_factory(
    container_blocks: HashSet<String>,
    retain_empty_tokens: bool,
    preserve_expression_whitespace: bool,
) -> Box<dyn I18nMessageFactory> {
    Box::new(I18nVisitor::new(
        container_blocks,
        retain_empty_tokens,
        preserve_expression_whitespace,
    ))
}

struct I18nMessageVisitorContext {
    is_icu: bool,
    icu_depth: usize,
    placeholder_registry: PlaceholderRegistry,
    placeholder_to_content: HashMap<String, i18n::MessagePlaceholder>,
    placeholder_to_message: HashMap<String, Box<Message>>,
    visit_node_fn: Option<VisitNodeFn>,
}

fn noop_visit_node_fn(_html: &html::Node, i18n: &i18n::Node) -> i18n::Node {
    i18n.clone()
}

struct I18nVisitor {
    expression_parser: ExpressionParser,
    container_blocks: HashSet<String>,
    retain_empty_tokens: bool,
    preserve_expression_whitespace: bool,
}

impl I18nVisitor {
    fn new(
        container_blocks: HashSet<String>,
        retain_empty_tokens: bool,
        preserve_expression_whitespace: bool,
    ) -> Self {
        I18nVisitor {
            expression_parser: ExpressionParser::new(),
            container_blocks,
            retain_empty_tokens,
            preserve_expression_whitespace,
        }
    }

    fn to_i18n_message(
        &mut self,
        nodes: &[html::Node],
        meaning: &str,
        description: &str,
        custom_id: &str,
        visit_node_fn: Option<VisitNodeFn>,
    ) -> Message {
        let is_icu = nodes.len() == 1 && matches!(nodes[0], html::Node::Expansion(_));

        let mut context = I18nMessageVisitorContext {
            is_icu,
            icu_depth: 0,
            placeholder_registry: PlaceholderRegistry::new(),
            placeholder_to_content: HashMap::new(),
            placeholder_to_message: HashMap::new(),
            visit_node_fn,
        };

        let i18n_nodes: Vec<i18n::Node> = html::visit_all(self, nodes, &mut context)
            .into_iter()
            .filter_map(|result| result.downcast::<i18n::Node>().ok().map(|n| *n))
            .collect();

        Message::new(
            i18n_nodes,
            context.placeholder_to_content,
            context.placeholder_to_message,
            meaning.to_string(),
            description.to_string(),
            custom_id.to_string(),
        )
    }

    fn visit_element_like(
        &mut self,
        node: &html::Element,
        context: &mut I18nMessageVisitorContext,
    ) -> i18n::Node {
        let children: Vec<i18n::Node> = html::visit_all(self, &node.children, context)
            .into_iter()
            .filter_map(|result| result.downcast::<i18n::Node>().ok().map(|n| *n))
            .collect();

        let mut attrs: HashMap<String, String> = HashMap::new();
        for attr in &node.attrs {
            attrs.insert(attr.name.to_string(), attr.value.to_string());
        }
        for dir in &node.directives {
            for attr in &dir.attrs {
                attrs.insert(attr.name.to_string(), attr.value.to_string());
            }
        }

        let node_name = &node.name;
        let is_void = get_html_tag_definition(node_name).is_void();

        let start_ph_name = context
            .placeholder_registry
            .get_start_tag_placeholder_name(node_name, &attrs, is_void);
        context.placeholder_to_content.insert(
            start_ph_name.clone(),
            i18n::MessagePlaceholder {
                text: node.start_source_span.to_string(),
                source_span: node.start_source_span.clone(),
            },
        );

        let close_ph_name = if is_void {
            String::new()
        } else {
            let name = context
                .placeholder_registry
                .get_close_tag_placeholder_name(node_name);
            context.placeholder_to_content.insert(
                name.clone(),
                i18n::MessagePlaceholder {
                    text: format!("</{}>", node_name),
                    source_span: node
                        .end_source_span
                        .clone()
                        .unwrap_or_else(|| node.source_span.clone()),
                },
            );
            name
        };

        let i18n_node = i18n::Node::TagPlaceholder(TagPlaceholder {
            tag: node_name.to_string(),
            attrs,
            start_name: start_ph_name,
            close_name: close_ph_name,
            children,
            is_void,
            source_span: node.source_span.clone(),
            start_source_span: Some(node.start_source_span.clone()),
            end_source_span: node.end_source_span.clone(),
        });

        if let Some(visit_fn) = context.visit_node_fn {
            visit_fn(&html::Node::Element(node.clone()), &i18n_node)
        } else {
            i18n_node
        }
    }

    fn visit_text_with_interpolation(
        &mut self,
        tokens: &[Token],
        source_span: &ParseSourceSpan,
        context: &mut I18nMessageVisitorContext,
        previous_i18n: Option<&i18n::I18nMeta>,
    ) -> i18n::Node {
        let mut nodes: Vec<i18n::Node> = Vec::new();
        let mut has_interpolation = false;

        for token in tokens {
            match token {
                Token::Interpolation(interp_token) => {
                    has_interpolation = true;
                    let parts = &interp_token.parts;
                    if parts.len() >= 3 {
                        let expression = &parts[1];
                        let base_name = extract_placeholder_name(expression)
                            .unwrap_or_else(|| "INTERPOLATION".to_string());
                        let ph_name = context
                            .placeholder_registry
                            .get_placeholder_name(&base_name, expression);

                        if self.preserve_expression_whitespace {
                            context.placeholder_to_content.insert(
                                ph_name.clone(),
                                i18n::MessagePlaceholder {
                                    text: parts.join(""),
                                    source_span: interp_token.source_span.clone(),
                                },
                            );
                            nodes.push(i18n::Node::Placeholder(Placeholder {
                                value: expression.to_string(),
                                name: ph_name,
                                source_span: interp_token.source_span.clone(),
                            }));
                        } else {
                            let normalized = self.normalize_expression(interp_token);
                            context.placeholder_to_content.insert(
                                ph_name.clone(),
                                i18n::MessagePlaceholder {
                                    text: format!("{}{}{}", parts[0], normalized, parts[2]),
                                    source_span: interp_token.source_span.clone(),
                                },
                            );
                            nodes.push(i18n::Node::Placeholder(Placeholder {
                                value: normalized,
                                name: ph_name,
                                source_span: interp_token.source_span.clone(),
                            }));
                        }
                    }
                }
                Token::AttrValueInterpolation(attr_interp_token) => {
                    has_interpolation = true;
                    let parts = &attr_interp_token.parts;
                    if parts.len() >= 3 {
                        let expression = &parts[1];
                        let base_name = extract_placeholder_name(expression)
                            .unwrap_or_else(|| "INTERPOLATION".to_string());
                        let ph_name = context
                            .placeholder_registry
                            .get_placeholder_name(&base_name, expression);

                        if self.preserve_expression_whitespace {
                            context.placeholder_to_content.insert(
                                ph_name.clone(),
                                i18n::MessagePlaceholder {
                                    text: parts.join(""),
                                    source_span: attr_interp_token.source_span.clone(),
                                },
                            );
                            nodes.push(i18n::Node::Placeholder(Placeholder {
                                value: expression.to_string(),
                                name: ph_name,
                                source_span: attr_interp_token.source_span.clone(),
                            }));
                        } else {
                            // For AttributeValueInterpolationToken, create a temporary InterpolationToken for normalization
                            let temp_token = InterpolationToken {
                                parts: parts.clone(),
                                source_span: attr_interp_token.source_span.clone(),
                            };
                            let normalized = self.normalize_expression(&temp_token);
                            context.placeholder_to_content.insert(
                                ph_name.clone(),
                                i18n::MessagePlaceholder {
                                    text: format!("{}{}{}", parts[0], normalized, parts[2]),
                                    source_span: attr_interp_token.source_span.clone(),
                                },
                            );
                            nodes.push(i18n::Node::Placeholder(Placeholder {
                                value: normalized,
                                name: ph_name,
                                source_span: attr_interp_token.source_span.clone(),
                            }));
                        }
                    }
                }
                _ => {
                    // Text or encoded entity token
                    let text_value = match token {
                        Token::Text(text_token) => {
                            text_token.parts.get(0).cloned().unwrap_or_default()
                        }
                        Token::EncodedEntity(entity_token) => {
                            entity_token.parts.get(0).cloned().unwrap_or_default()
                        }
                        _ => String::new().into(),
                    };

                    if !text_value.is_empty() || self.retain_empty_tokens {
                        if let Some(last) = nodes.last_mut() {
                            if let i18n::Node::Text(ref mut text_node) = last {
                                text_node.value.push_str(&text_value);
                                let token_span = match token {
                                    Token::Text(t) => &t.source_span,
                                    Token::EncodedEntity(e) => &e.source_span,
                                    _ => &text_node.source_span,
                                };
                                text_node.source_span = ParseSourceSpan::new(
                                    text_node.source_span.start.clone(),
                                    token_span.end.clone(),
                                );
                                continue;
                            }
                        }
                        let span = match token {
                            Token::Text(t) => t.source_span.clone(),
                            Token::EncodedEntity(e) => e.source_span.clone(),
                            _ => ParseSourceSpan::new(
                                ParseLocation::from_source(String::new(), String::new(), 0, 0, 0),
                                ParseLocation::from_source(String::new(), String::new(), 0, 0, 0),
                            ),
                        };
                        nodes.push(i18n::Node::Text(I18nText::new(
                            text_value.to_string(),
                            span.clone(),
                        )));
                    } else if self.retain_empty_tokens {
                        let span = match token {
                            Token::Text(t) => t.source_span.clone(),
                            Token::EncodedEntity(e) => e.source_span.clone(),
                            _ => ParseSourceSpan::new(
                                ParseLocation::from_source(String::new(), String::new(), 0, 0, 0),
                                ParseLocation::from_source(String::new(), String::new(), 0, 0, 0),
                            ),
                        };
                        nodes.push(i18n::Node::Text(I18nText::new(
                            text_value.to_string(),
                            span,
                        )));
                    }
                }
            }
        }

        if has_interpolation {
            reuse_previous_source_spans(&mut nodes, previous_i18n);
            i18n::Node::Container(Container {
                children: nodes,
                source_span: source_span.clone(),
            })
        } else {
            nodes.into_iter().next().unwrap_or_else(|| {
                i18n::Node::Text(I18nText::new(String::new(), source_span.clone()))
            })
        }
    }

    fn normalize_expression(&self, token: &crate::ml_parser::tokens::InterpolationToken) -> String {
        let expression = &token.parts[1];
        match self
            .expression_parser
            .parse_binding(expression, token.source_span.start.offset)
        {
            Ok(ast) => serialize(&ast),
            Err(_) => expression.to_string(),
        }
    }
}

impl I18nMessageFactory for I18nVisitor {
    fn create_message(
        &mut self,
        nodes: &[html::Node],
        meaning: Option<&str>,
        description: Option<&str>,
        custom_id: Option<&str>,
        visit_node_fn: Option<VisitNodeFn>,
    ) -> Message {
        self.to_i18n_message(
            nodes,
            meaning.unwrap_or(""),
            description.unwrap_or(""),
            custom_id.unwrap_or(""),
            visit_node_fn,
        )
    }
}

impl html::Visitor for I18nVisitor {
    fn visit_element(
        &mut self,
        element: &html::Element,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        let ctx = context.downcast_mut::<I18nMessageVisitorContext>().unwrap();
        Some(Box::new(self.visit_element_like(element, ctx)))
    }

    fn visit_component(
        &mut self,
        component: &html::Component,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        // First, visit children (this needs mutable access to context)
        let children: Vec<i18n::Node> = html::visit_all(self, &component.children, context)
            .into_iter()
            .filter_map(|result| result.downcast::<i18n::Node>().ok().map(|n| *n))
            .collect();

        // Now we can safely borrow context as ctx
        let ctx = context.downcast_mut::<I18nMessageVisitorContext>().unwrap();
        let mut attrs: HashMap<String, String> = HashMap::new();
        for attr in &component.attrs {
            attrs.insert(attr.name.to_string(), attr.value.to_string());
        }
        for dir in &component.directives {
            for attr in &dir.attrs {
                attrs.insert(attr.name.to_string(), attr.value.to_string());
            }
        }

        let node_name = &component.full_name;
        let is_void = component
            .tag_name
            .as_ref()
            .map(|tn| get_html_tag_definition(tn).is_void())
            .unwrap_or(false);

        let start_ph_name = ctx
            .placeholder_registry
            .get_start_tag_placeholder_name(node_name, &attrs, is_void);
        ctx.placeholder_to_content.insert(
            start_ph_name.clone(),
            i18n::MessagePlaceholder {
                text: component.start_source_span.to_string(),
                source_span: component.start_source_span.clone(),
            },
        );

        let close_ph_name = if is_void {
            String::new()
        } else {
            let name = ctx
                .placeholder_registry
                .get_close_tag_placeholder_name(node_name);
            ctx.placeholder_to_content.insert(
                name.clone(),
                i18n::MessagePlaceholder {
                    text: format!("</{}>", node_name),
                    source_span: component
                        .end_source_span
                        .clone()
                        .unwrap_or_else(|| component.source_span.clone()),
                },
            );
            name
        };

        let i18n_node = i18n::Node::TagPlaceholder(TagPlaceholder {
            tag: node_name.to_string(),
            attrs,
            start_name: start_ph_name,
            close_name: close_ph_name,
            children,
            is_void,
            source_span: component.source_span.clone(),
            start_source_span: Some(component.start_source_span.clone()),
            end_source_span: component.end_source_span.clone(),
        });

        let result = if let Some(visit_fn) = ctx.visit_node_fn {
            visit_fn(&html::Node::Component(component.clone()), &i18n_node)
        } else {
            i18n_node
        };

        Some(Box::new(result))
    }

    fn visit_attribute(
        &mut self,
        attribute: &html::Attribute,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        let ctx = context.downcast_mut::<I18nMessageVisitorContext>().unwrap();
        let node = if attribute.value_tokens.is_none()
            || attribute.value_tokens.as_ref().unwrap().len() == 1
        {
            i18n::Node::Text(I18nText::new(
                attribute.value.to_string(),
                attribute
                    .value_span
                    .clone()
                    .unwrap_or_else(|| attribute.source_span.clone()),
            ))
        } else {
            let tokens = attribute.value_tokens.as_ref().unwrap();
            let token_vec: Vec<Token> = tokens.iter().cloned().collect();
            self.visit_text_with_interpolation(
                &token_vec,
                &attribute
                    .value_span
                    .clone()
                    .unwrap_or_else(|| attribute.source_span.clone()),
                ctx,
                attribute.i18n.as_ref(),
            )
        };

        let result = if let Some(visit_fn) = ctx.visit_node_fn {
            visit_fn(&html::Node::Attribute(attribute.clone()), &node)
        } else {
            node
        };

        Some(Box::new(result))
    }

    fn visit_text(
        &mut self,
        text: &html::Text,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        let ctx = context.downcast_mut::<I18nMessageVisitorContext>().unwrap();
        let node = if text.tokens.len() == 1 {
            i18n::Node::Text(I18nText::new(
                text.value.to_string(),
                text.source_span.clone(),
            ))
        } else {
            let token_vec: Vec<Token> = text.tokens.iter().cloned().collect();
            self.visit_text_with_interpolation(
                &token_vec,
                &text.source_span,
                ctx,
                text.i18n.as_ref(),
            )
        };

        let result = if let Some(visit_fn) = ctx.visit_node_fn {
            visit_fn(&html::Node::Text(text.clone()), &node)
        } else {
            node
        };

        Some(Box::new(result))
    }

    fn visit_comment(
        &mut self,
        _comment: &html::Comment,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None
    }

    fn visit_expansion(
        &mut self,
        expansion: &html::Expansion,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        // First, update icu_depth
        {
            let ctx = context.downcast_mut::<I18nMessageVisitorContext>().unwrap();
            ctx.icu_depth += 1;
        }

        // Visit all cases (this needs mutable access to context)
        let mut i18n_icu_cases: HashMap<String, i18n::Node> = HashMap::new();
        for case in &expansion.cases {
            let case_nodes: Vec<i18n::Node> = html::visit_all(self, &case.expression, context)
                .into_iter()
                .filter_map(|result| result.downcast::<i18n::Node>().ok().map(|n| *n))
                .collect();
            i18n_icu_cases.insert(
                case.value.to_string(),
                i18n::Node::Container(Container {
                    children: case_nodes,
                    source_span: case.exp_source_span.clone(),
                }),
            );
        }

        // Now we can safely borrow context as ctx
        let ctx = context.downcast_mut::<I18nMessageVisitorContext>().unwrap();
        ctx.icu_depth -= 1;

        let i18n_icu = Icu {
            expression: expansion.switch_value.to_string(),
            type_: expansion.expansion_type.to_string(),
            cases: i18n_icu_cases.clone(),
            source_span: expansion.source_span.clone(),
            expression_placeholder: None,
        };

        if ctx.is_icu || ctx.icu_depth > 0 {
            let exp_ph = ctx
                .placeholder_registry
                .get_unique_placeholder(&format!("VAR_{}", expansion.expansion_type));
            let mut i18n_icu_with_ph = i18n_icu.clone();
            i18n_icu_with_ph.expression_placeholder = Some(exp_ph.clone());
            ctx.placeholder_to_content.insert(
                exp_ph.clone(),
                i18n::MessagePlaceholder {
                    text: expansion.switch_value.to_string(),
                    source_span: expansion.switch_value_source_span.clone(),
                },
            );

            let result = if let Some(visit_fn) = ctx.visit_node_fn {
                visit_fn(
                    &html::Node::Expansion(expansion.clone()),
                    &i18n::Node::Icu(i18n_icu_with_ph),
                )
            } else {
                i18n::Node::Icu(i18n_icu_with_ph)
            };

            Some(Box::new(result))
        } else {
            let ph_name = ctx
                .placeholder_registry
                .get_placeholder_name("ICU", &expansion.source_span.to_string());
            let sub_message = self.to_i18n_message(
                &[html::Node::Expansion(expansion.clone())],
                "",
                "",
                "",
                None,
            );
            ctx.placeholder_to_message
                .insert(ph_name.clone(), Box::new(sub_message));

            let icu_placeholder = IcuPlaceholder {
                value: i18n_icu,
                name: ph_name,
                source_span: expansion.source_span.clone(),
                previous_message: None,
            };

            let result = if let Some(visit_fn) = ctx.visit_node_fn {
                visit_fn(
                    &html::Node::Expansion(expansion.clone()),
                    &i18n::Node::IcuPlaceholder(icu_placeholder.clone()),
                )
            } else {
                i18n::Node::IcuPlaceholder(icu_placeholder)
            };

            Some(Box::new(result))
        }
    }

    fn visit_expansion_case(
        &mut self,
        _expansion_case: &html::ExpansionCase,
        _context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        None // Handled in visit_expansion
    }

    fn visit_block(
        &mut self,
        block: &html::Block,
        context: &mut dyn std::any::Any,
    ) -> Option<Box<dyn std::any::Any>> {
        if block.name.as_ref() == "switch" {
            let children: Vec<i18n::Node> = html::visit_all(self, &block.children, context)
                .into_iter()
                .filter_map(|result| result.downcast::<i18n::Node>().ok().map(|n| *n))
                .collect();
            return Some(Box::new(i18n::Node::Container(Container {
                children,
                source_span: block.source_span.clone(),
            })));
        }

        // First, visit children (this needs mutable access to context)
        let children: Vec<i18n::Node> = html::visit_all(self, &block.children, context)
            .into_iter()
            .filter_map(|result| result.downcast::<i18n::Node>().ok().map(|n| *n))
            .collect();

        // Now we can safely borrow context as ctx
        let ctx = context.downcast_mut::<I18nMessageVisitorContext>().unwrap();

        let parameters: Vec<String> = block
            .parameters
            .iter()
            .map(|param| param.expression.to_string())
            .collect();

        let start_ph_name = ctx
            .placeholder_registry
            .get_start_block_placeholder_name(&block.name, &parameters);
        let close_ph_name = ctx
            .placeholder_registry
            .get_close_block_placeholder_name(&block.name);

        ctx.placeholder_to_content.insert(
            start_ph_name.clone(),
            i18n::MessagePlaceholder {
                text: block.start_source_span.to_string(),
                source_span: block.start_source_span.clone(),
            },
        );

        ctx.placeholder_to_content.insert(
            close_ph_name.clone(),
            i18n::MessagePlaceholder {
                text: block
                    .end_source_span
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "}".to_string()),
                source_span: block
                    .end_source_span
                    .clone()
                    .unwrap_or_else(|| block.source_span.clone()),
            },
        );

        let block_placeholder = BlockPlaceholder {
            name: block.name.to_string(),
            parameters,
            start_name: start_ph_name,
            close_name: close_ph_name,
            children,
            source_span: block.source_span.clone(),
            start_source_span: Some(block.start_source_span.clone()),
            end_source_span: block.end_source_span.clone(),
        };

        let result = if let Some(visit_fn) = ctx.visit_node_fn {
            visit_fn(
                &html::Node::Block(block.clone()),
                &i18n::Node::BlockPlaceholder(block_placeholder.clone()),
            )
        } else {
            i18n::Node::BlockPlaceholder(block_placeholder)
        };

        Some(Box::new(result))
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

fn reuse_previous_source_spans(nodes: &mut [i18n::Node], previous_i18n: Option<&i18n::I18nMeta>) {
    if let Some(i18n_meta) = previous_i18n {
        match i18n_meta {
            i18n::I18nMeta::Message(msg) => {
                assert_single_container_message(msg);
                if let Some(i18n::Node::Container(container)) = msg.nodes.first() {
                    assert_equivalent_nodes(&container.children, nodes);
                    for (i, node) in nodes.iter_mut().enumerate() {
                        if let Some(prev_node) = container.children.get(i) {
                            *node = prev_node.clone();
                        }
                    }
                }
            }
            i18n::I18nMeta::Node(i18n::Node::Container(container)) => {
                assert_equivalent_nodes(&container.children, nodes);
                for (i, node) in nodes.iter_mut().enumerate() {
                    if let Some(prev_node) = container.children.get(i) {
                        *node = prev_node.clone();
                    }
                }
            }
            _ => {}
        }
    }
}

fn assert_single_container_message(message: &Message) {
    if message.nodes.len() != 1 {
        panic!("Unexpected previous i18n message - expected it to consist of only a single `Container` node.");
    }
    if !matches!(message.nodes[0], i18n::Node::Container(_)) {
        panic!("Unexpected previous i18n message - expected it to consist of only a single `Container` node.");
    }
}

fn assert_equivalent_nodes(previous_nodes: &[i18n::Node], nodes: &[i18n::Node]) {
    if previous_nodes.len() != nodes.len() {
        panic!(
            "The number of i18n message children changed between first and second pass.\nFirst pass ({} tokens):\n{}\nSecond pass ({} tokens):\n{}",
            previous_nodes.len(),
            previous_nodes.iter().map(|n| format!("\"{}\"", n.source_span().to_string())).collect::<Vec<_>>().join("\n"),
            nodes.len(),
            nodes.iter().map(|n| format!("\"{}\"", n.source_span().to_string())).collect::<Vec<_>>().join("\n")
        );
    }
    for (i, (prev, curr)) in previous_nodes.iter().zip(nodes.iter()).enumerate() {
        if std::mem::discriminant(prev) != std::mem::discriminant(curr) {
            panic!("The types of the i18n message children changed between first and second pass at index {}.", i);
        }
    }
}

fn extract_placeholder_name(input: &str) -> Option<String> {
    // Try single quotes first
    if let Some(captures) = CUSTOM_PH_EXP_SINGLE.captures(input) {
        if let Some(name) = captures.get(1) {
            return Some(name.as_str().to_string());
        }
    }
    // Try double quotes
    if let Some(captures) = CUSTOM_PH_EXP_DOUBLE.captures(input) {
        if let Some(name) = captures.get(1) {
            return Some(name.as_str().to_string());
        }
    }
    None
}
