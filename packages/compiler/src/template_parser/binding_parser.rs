//! Binding Parser
//!
//! Corresponds to packages/compiler/src/template_parser/binding_parser.ts

use crate::core::SecurityContext;
use crate::directive_matching::CssSelector;
use crate::expression_parser::ast::{
    AST as ExprAST, LiteralPrimitive, ParseSpan, TemplateBinding, ASTWithSource,
    AbsoluteSourceSpan,
};
use crate::expression_parser::parser::Parser;
use crate::ml_parser::tags::merge_ns_and_name;
use crate::ml_parser::tokens::{Token, InterpolatedAttributeToken};
use crate::ml_parser::entities::NAMED_ENTITIES;
use crate::parse_util::{ParseError, ParseErrorLevel, ParseSourceSpan};
use std::u32;
use crate::schema::element_schema_registry::ElementSchemaRegistry;
use crate::util::{split_at_colon, split_at_period};
use std::collections::HashMap;

const PROPERTY_PARTS_SEPARATOR: char = '.';
const ATTRIBUTE_PREFIX: &str = "attr";
const ANIMATE_PREFIX: &str = "animate";
const CLASS_PREFIX: &str = "class";
const STYLE_PREFIX: &str = "style";
const TEMPLATE_ATTR_PREFIX: &str = "*";
const LEGACY_ANIMATE_PROP_PREFIX: &str = "animate-";

pub type HostProperties = HashMap<String, String>;
pub type HostListeners = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingType {
    Property,
    Attribute,
    Class,
    Style,
    Animation,
    TwoWay,
    LegacyAnimation,
}

#[derive(Debug, Clone)]
pub struct BoundElementProperty {
    pub name: String,
    pub type_: BindingType,
    pub security_context: SecurityContext,
    pub value: ASTWithSource,
    pub unit: Option<String>,
    pub source_span: ParseSourceSpan,
    pub key_span: Option<ParseSourceSpan>,
    pub value_span: Option<ParseSourceSpan>,
}

impl BoundElementProperty {
    pub fn new(
        name: String,
        type_: BindingType,
        security_context: SecurityContext,
        value: ASTWithSource,
        unit: Option<String>,
        source_span: ParseSourceSpan,
        key_span: Option<ParseSourceSpan>,
        value_span: Option<ParseSourceSpan>,
    ) -> Self {
        BoundElementProperty {
            name,
            type_,
            security_context,
            value,
            unit,
            source_span,
            key_span,
            value_span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedPropertyType {
    LiteralAttr,
    Animation,
    LegacyAnimation,
    TwoWay,
    Default,
}

#[derive(Debug, Clone)]
pub struct ParsedProperty {
    pub name: String,
    pub expression: ASTWithSource,
    pub type_: ParsedPropertyType,
    pub source_span: ParseSourceSpan,
    pub key_span: Option<ParseSourceSpan>,
    pub value_span: Option<ParseSourceSpan>,
    pub is_literal: bool,
    pub is_animation: bool,
    pub is_legacy_animation: bool,
}

impl ParsedProperty {
    pub fn new(
        name: String,
        expression: ASTWithSource,
        type_: ParsedPropertyType,
        source_span: ParseSourceSpan,
        key_span: Option<ParseSourceSpan>,
        value_span: Option<ParseSourceSpan>,
    ) -> Self {
        let is_literal = type_ == ParsedPropertyType::LiteralAttr;
        let is_animation = type_ == ParsedPropertyType::Animation;
        let is_legacy_animation = type_ == ParsedPropertyType::LegacyAnimation;
        ParsedProperty {
            name,
            expression,
            type_,
            source_span,
            key_span,
            value_span,
            is_literal,
            is_animation,
            is_legacy_animation,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedVariable {
    pub name: String,
    pub value: String,
    pub source_span: ParseSourceSpan,
    pub key_span: ParseSourceSpan,
    pub value_span: Option<ParseSourceSpan>,
}

impl ParsedVariable {
    pub fn new(
        name: String,
        value: String,
        source_span: ParseSourceSpan,
        key_span: ParseSourceSpan,
        value_span: Option<ParseSourceSpan>,
    ) -> Self {
        ParsedVariable {
            name,
            value,
            source_span,
            key_span,
            value_span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedEventType {
    Regular,
    Animation,
    TwoWay,
    LegacyAnimation,
}

#[derive(Debug, Clone)]
pub struct ParsedEvent {
    pub name: String,
    pub target_or_phase: Option<String>,
    pub type_: ParsedEventType,
    pub handler: ASTWithSource,
    pub source_span: ParseSourceSpan,
    pub handler_span: ParseSourceSpan,
    pub key_span: Option<ParseSourceSpan>,
}

impl ParsedEvent {
    pub fn new(
        name: String,
        target_or_phase: Option<String>,
        type_: ParsedEventType,
        handler: ASTWithSource,
        source_span: ParseSourceSpan,
        handler_span: ParseSourceSpan,
        key_span: Option<ParseSourceSpan>,
    ) -> Self {
        ParsedEvent {
            name,
            target_or_phase,
            type_,
            handler,
            source_span,
            handler_span,
            key_span,
        }
    }
}

pub struct BindingParser<'a> {
    expr_parser: &'a Parser,
    schema_registry: &'a dyn ElementSchemaRegistry,
    pub errors: Vec<ParseError>,
}

impl<'a> BindingParser<'a> {
    pub fn new(
        expr_parser: &'a Parser,
        schema_registry: &'a dyn ElementSchemaRegistry,
        errors: Vec<ParseError>,
    ) -> Self {
        BindingParser {
            expr_parser,
            schema_registry,
            errors,
        }
    }

    pub fn create_bound_host_properties(
        &mut self,
        properties: &HostProperties,
        source_span: &ParseSourceSpan,
    ) -> Option<Vec<ParsedProperty>> {
        let mut bound_props: Vec<ParsedProperty> = Vec::new();

        for (prop_name, expression) in properties {
            self.parse_property_binding(
                prop_name,
                expression,
                true,
                false,
                source_span.clone(),
                source_span.start.offset,
                None,
                &mut Vec::new(),
                &mut bound_props,
                source_span.clone(),
            );
        }
        Some(bound_props)
    }

    pub fn create_directive_host_event_asts(
        &mut self,
        host_listeners: &HostListeners,
        source_span: &ParseSourceSpan,
    ) -> Option<Vec<ParsedEvent>> {
        let mut target_events: Vec<ParsedEvent> = Vec::new();
        
        for (prop_name, expression) in host_listeners {
            self.parse_event(
                prop_name,
                expression,
                false,
                source_span.clone(),
                source_span.clone(),
                &mut Vec::new(),
                &mut target_events,
                Some(source_span.clone()),
            );
        }
        Some(target_events)
    }

    pub fn parse_interpolation(
        &mut self,
        value: &str,
        source_span: &ParseSourceSpan,
        interpolated_tokens: Option<Vec<InterpolatedAttributeToken>>,
    ) -> ASTWithSource {
        if let Some(tokens) = interpolated_tokens {
            let mut strings = Vec::new();
            let mut expressions = Vec::new();
            let mut current_string = String::new();

            for token in tokens {
                match token {
                    Token::Text(t) => {
                         current_string.push_str(&decode_entities(&t.parts.join("")));
                    },
                    Token::AttrValueText(t) => {
                         current_string.push_str(&decode_entities(&t.parts.join("")));
                    },
                    Token::EncodedEntity(t) => {
                         current_string.push_str(&decode_entities(&t.parts[0]));
                    },
                    Token::Interpolation(t) => {
                        strings.push(current_string);
                        current_string = String::new();
                        
                        let expr_string = if t.parts.len() > 1 { &t.parts[1] } else { "" };
                        let prefix_len = if t.parts.len() > 0 { t.parts[0].len() } else { 0 };
                        let start_location = t.source_span.start.move_by(prefix_len as i32);
                        let absolute_offset = start_location.offset;
                        
                        match self.expr_parser.parse_binding(expr_string, absolute_offset) {
                            Ok(ast) => {
                                expressions.push(ASTWithSource::new(
                                    Box::new(ast),
                                    Some(expr_string.to_string()),
                                    start_location.to_string(),
                                    absolute_offset,
                                    vec![]
                                ));
                            },
                            Err(e) => {
                                self._report_error(&e.to_string(), &t.source_span, ParseErrorLevel::Error);
                                let err_ast = self._wrap_literal_primitive("ERROR", &t.source_span, absolute_offset);
                                expressions.push(ASTWithSource::new(
                                     Box::new(err_ast),
                                     Some(expr_string.to_string()),
                                     start_location.to_string(),
                                     absolute_offset,
                                     vec![]
                                 ));
                            }
                        }
                    },
                    Token::AttrValueInterpolation(t) => {
                        strings.push(current_string);
                        current_string = String::new();
                        
                        let expr_string = if t.parts.len() > 1 { &t.parts[1] } else { "" };
                        let prefix_len = if t.parts.len() > 0 { t.parts[0].len() } else { 0 };
                        let start_location = t.source_span.start.move_by(prefix_len as i32);
                        let absolute_offset = start_location.offset;
                        
                        match self.expr_parser.parse_binding(expr_string, absolute_offset) {
                            Ok(ast) => {
                                expressions.push(ASTWithSource::new(
                                    Box::new(ast),
                                    Some(expr_string.to_string()),
                                    start_location.to_string(),
                                    absolute_offset,
                                    vec![]
                                ));
                            },
                            Err(e) => {
                                self._report_error(&e.to_string(), &t.source_span, ParseErrorLevel::Error);
                                let err_ast = self._wrap_literal_primitive("ERROR", &t.source_span, absolute_offset);
                                expressions.push(ASTWithSource::new(
                                     Box::new(err_ast),
                                     Some(expr_string.to_string()),
                                     start_location.to_string(),
                                     absolute_offset,
                                     vec![]
                                 ));
                            }
                        }
                    },
                    _ => {}
                }
            }
            strings.push(current_string);
            
            let expr_asts: Vec<Box<ExprAST>> = expressions.into_iter().map(|e| e.ast).collect();
            
            let span = ParseSpan::new(0, source_span.end.offset - source_span.start.offset);
            let abs_source_span = AbsoluteSourceSpan::new(source_span.start.offset, source_span.end.offset);

            let interpolation = crate::expression_parser::ast::Interpolation {
                span,
                source_span: abs_source_span,
                strings,
                expressions: expr_asts,
            };
            
            ASTWithSource::new(
                    Box::new(ExprAST::Interpolation(interpolation)),
                    Some(value.to_string()),
                    source_span.start.to_string(),
                    source_span.start.offset,
                    vec![],
            )
        } else {
             // Fallback to original logic
             let absolute_offset = source_span.start.offset;
             match self.expr_parser.parse_interpolation(value, absolute_offset) {
                Ok(interpolation) => {
                     ASTWithSource::new(
                        Box::new(ExprAST::Interpolation(interpolation)),
                        Some(value.to_string()),
                        source_span.start.to_string(),
                        absolute_offset,
                        vec![],
                    )
                },
                Err(e) => {
                     self._report_error(&e.to_string(), source_span, ParseErrorLevel::Error);
                     let err_ast = self._wrap_literal_primitive("ERROR", source_span, absolute_offset);
                     ASTWithSource::new(
                        Box::new(err_ast),
                         Some(value.to_string()),
                         source_span.start.to_string(),
                         absolute_offset,
                         vec![],
                     )
                }
            }
        }
    }

    pub fn parse_interpolation_expression(
        &mut self,
        expression: &str,
        source_span: &ParseSourceSpan,
    ) -> ASTWithSource {
        let absolute_offset = source_span.start.offset;
        match self.expr_parser.parse_binding(expression, absolute_offset) {
            Ok(ast) => {
                 ASTWithSource::new(
                    Box::new(ast),
                    Some(expression.to_string()),
                    source_span.start.to_string(),
                    absolute_offset,
                    vec![],
                )
            },
            Err(e) => {
                self._report_error(&e.to_string(), source_span, ParseErrorLevel::Error);
                 let err_ast = self._wrap_literal_primitive("ERROR", source_span, absolute_offset);
                 ASTWithSource::new(
                    Box::new(err_ast),
                     Some(expression.to_string()),
                     source_span.start.to_string(),
                     absolute_offset,
                     vec![],
                 )
            }
        }
    }

    pub fn parse_inline_template_binding(
        &mut self,
        tpl_key: &str,
        tpl_value: &str,
        source_span: &ParseSourceSpan,
        absolute_value_offset: usize,
        target_matchable_attrs: &mut Vec<Vec<String>>,
        target_props: &mut Vec<ParsedProperty>,
        target_vars: &mut Vec<ParsedVariable>,
        is_ivy_ast: bool,
    ) {
        let absolute_key_offset = source_span.start.offset + TEMPLATE_ATTR_PREFIX.len();
        let bindings = self._parse_template_bindings(
            tpl_key,
            tpl_value,
            source_span,
            absolute_key_offset,
            absolute_value_offset,
        );

        let mut key_bound = false;
        for binding in bindings {
            match binding {
                TemplateBinding::Variable(var_binding) => {
                     let binding_span = move_parse_source_span(source_span, &var_binding.span);
                     let key = var_binding.key.source.clone();
                     let key_span = move_parse_source_span(source_span, &var_binding.key.span);
                     
                     let value = if let Some(_) = &var_binding.value {
                         "$implicit".to_string() 
                     } else {
                         "$implicit".to_string()
                     };
                     
                     let value_span = var_binding.value.as_ref().map(|v| move_parse_source_span(source_span, &v.source_span()));

                     target_vars.push(ParsedVariable::new(key, value, binding_span, key_span, value_span));
                },
                TemplateBinding::Expression(expr_binding) => {
                     let mut binding_span = move_parse_source_span(source_span, &expr_binding.span);
                     let key = expr_binding.key.source.clone();
                     let mut key_span = move_parse_source_span(source_span, &expr_binding.key.span);
                     
                     if key == tpl_key {
                         key_bound = true;
                         // If synthesized binding (empty span), fix it to cover directive name
                         if key_span.start == key_span.end {
                             let start = source_span.start.move_by(1); // Skip *
                             let end = start.move_by(tpl_key.len() as i32);
                             key_span = ParseSourceSpan::new(start, end).with_details(source_span.details.clone().unwrap_or_default());
                             binding_span = key_span.clone();
                         }
                     }

                     if let Some(value_ast) = expr_binding.value {
                         let src_span = if is_ivy_ast { binding_span.clone() } else { source_span.clone() };
                         let value_span = move_parse_source_span(source_span, &value_ast.source_span());
                         
                         let ast_with_source = ASTWithSource::new(
                             value_ast,
                             Some("".to_string()),
                             src_span.start.to_string(),
                             value_span.start.offset,
                             vec![]
                         );
                         
                         self.parse_property_ast(
                             &key,
                             ast_with_source,
                             false,
                             &src_span,
                             &key_span,
                             Some(&value_span),
                             target_matchable_attrs,
                             target_props
                         );
                     } else {
                         target_matchable_attrs.push(vec![key.clone(), "".to_string()]);
                         self.parse_literal_attr(
                             &key,
                             None,
                             key_span.clone(),
                             absolute_value_offset,
                             None,
                             target_matchable_attrs,
                             target_props,
                             key_span.clone()
                         );
                     }
                }
            }
        }

        if !key_bound {
             // If the template key was not bound, add it as a literal attribute
             let start = source_span.start.move_by(1);
             let end = start.move_by(tpl_key.len() as i32);
             let name_span = ParseSourceSpan::new(start, end).with_details(source_span.details.clone().unwrap_or_default());
             
             target_matchable_attrs.push(vec![tpl_key.to_string(), "".to_string()]);
             self.parse_literal_attr(
                 tpl_key,
                 None,
                 name_span.clone(),
                 absolute_value_offset,
                 None,
                 target_matchable_attrs,
                 target_props,
                 name_span 
             );
        }
    }

    fn _parse_template_bindings(
        &mut self,
        tpl_key: &str,
        tpl_value: &str,
        source_span: &ParseSourceSpan,
        _absolute_key_offset: usize,
        absolute_value_offset: usize,
    ) -> Vec<TemplateBinding> {
         let result = self.expr_parser.parse_template_bindings(tpl_value, Some(tpl_key), absolute_value_offset);
         
         for error in result.errors {
             self.errors.push(ParseError::new(source_span.clone(), error.msg)); 
         }
         
         result.template_bindings
    }

    pub fn parse_literal_attr(
        &mut self,
        name: &str,
        value: Option<&str>,
        source_span: ParseSourceSpan,
        absolute_offset: usize,
        value_span: Option<ParseSourceSpan>,
        target_matchable_attrs: &mut Vec<Vec<String>>,
        target_props: &mut Vec<ParsedProperty>,
        key_span: ParseSourceSpan,
    ) {
        let mut name = name.to_string();
        let mut key_span = key_span;
        
        if is_legacy_animation_label(&name) {
             name = name[1..].to_string();
             let new_start = key_span.start.move_by(1);
             key_span = ParseSourceSpan::new(new_start, key_span.end);
             
             if value.is_some() {
                 self._report_error(
                     "Assigning animation triggers via @prop=\"exp\" attributes with an expression is invalid. Use property bindings (e.g. [@prop]=\"exp\") or use an attribute without a value (e.g. @prop) instead.", 
                     &source_span, 
                     ParseErrorLevel::Error
                );
             }
             
             self._parse_legacy_animation(
                 &name,
                 value.unwrap_or(""),
                 &source_span,
                 absolute_offset,
                 &key_span,
                 value_span.as_ref(),
                 target_matchable_attrs,
                 target_props
             );
        } else {
             target_props.push(ParsedProperty::new(
                 name,
                 ASTWithSource::new(
                     Box::new(self._wrap_literal_primitive(value.unwrap_or(""), &source_span, absolute_offset)),
                     Some("".to_string()),
                     source_span.start.to_string(),
                     absolute_offset,
                     vec![]
                 ),
                 ParsedPropertyType::LiteralAttr,
                 source_span,
                 Some(key_span),
                 value_span,
             ));
        }
    }

    pub fn parse_property_binding(
        &mut self,
        name: &str,
        expression: &str,
        is_host: bool,
        is_part_of_assignment_binding: bool,
        source_span: ParseSourceSpan,
        absolute_offset: usize,
        value_span: Option<ParseSourceSpan>,
        target_matchable_attrs: &mut Vec<Vec<String>>,
        target_props: &mut Vec<ParsedProperty>,
        key_span: ParseSourceSpan,
    ) {
        if name.is_empty() {
            self._report_error("Property name is missing in binding", &source_span, ParseErrorLevel::Error);
        }

        let mut name = name.to_string();
        let mut is_legacy_animation_prop = false;
        let mut key_span = key_span;

        if name.starts_with(LEGACY_ANIMATE_PROP_PREFIX) {
            is_legacy_animation_prop = true;
            name = name[LEGACY_ANIMATE_PROP_PREFIX.len()..].to_string();
             let new_start = key_span.start.move_by(LEGACY_ANIMATE_PROP_PREFIX.len() as i32);
             key_span = ParseSourceSpan::new(new_start, key_span.end);
        } else if is_legacy_animation_label(&name) {
            is_legacy_animation_prop = true;
            name = name[1..].to_string();
             let new_start = key_span.start.move_by(1);
             key_span = ParseSourceSpan::new(new_start, key_span.end);
        }

        if is_legacy_animation_prop {
            self._parse_legacy_animation(
                &name,
                expression,
                &source_span,
                absolute_offset,
                &key_span,
                value_span.as_ref(),
                target_matchable_attrs,
                target_props,
            );
        } else if name.starts_with(&format!("{}{}", ANIMATE_PREFIX, PROPERTY_PARTS_SEPARATOR)) {
            let binding_ast = self.parse_binding(expression, is_host, value_span.clone().unwrap_or(source_span.clone()), absolute_offset);
            self._parse_animation(
                &name,
                &binding_ast,
                &source_span,
                &key_span,
                value_span.as_ref(),
                target_matchable_attrs,
                target_props,
            );
        } else {
             let binding_ast = self.parse_binding(expression, is_host, value_span.clone().unwrap_or(source_span.clone()), absolute_offset);
             self.parse_property_ast(
                &name,
                binding_ast,
                is_part_of_assignment_binding,
                &source_span,
                &key_span,
                value_span.as_ref(),
                target_matchable_attrs,
                target_props,
            );
        }
    }

    pub fn parse_binding(
        &mut self,
        value: &str,
        _is_host_binding: bool,
        source_span: ParseSourceSpan,
        absolute_offset: usize,
    ) -> ASTWithSource {
         let result = self.expr_parser.parse_binding(value, absolute_offset);
         
         match result {
             Ok(ast) => {
                 ASTWithSource::new(
                    Box::new(ast),
                    Some(value.to_string()),
                    source_span.start.to_string(),
                    absolute_offset,
                    vec![],
                )
             },
             Err(e) => {
                 self._report_error(&e.to_string(), &source_span, ParseErrorLevel::Error);
                 let err_ast = self._wrap_literal_primitive("ERROR", &source_span, absolute_offset);
                 ASTWithSource::new(
                    Box::new(err_ast),
                     Some(value.to_string()),
                     source_span.start.to_string(),
                     absolute_offset,
                     vec![],
                 )
             }
         }
    }
    
    pub fn create_bound_element_property(
        &mut self,
        element_selector: Option<&str>,
        bound_prop: &ParsedProperty,
        skip_validation: bool,
        map_property_name: bool,
    ) -> BoundElementProperty {
        if bound_prop.is_legacy_animation {
            return BoundElementProperty::new(
                bound_prop.name.clone(),
                BindingType::LegacyAnimation,
                SecurityContext::NONE,
                bound_prop.expression.clone(),
                None,
                bound_prop.source_span.clone(),
                bound_prop.key_span.clone(),
                bound_prop.value_span.clone(),
            );
        }

        let mut unit: Option<String> = None;
        let mut binding_type: BindingType = BindingType::Property;
        let mut bound_property_name: Option<String> = None;
        let parts: Vec<&str> = bound_prop.name.split(PROPERTY_PARTS_SEPARATOR).collect();
        let mut security_contexts: Vec<SecurityContext> = Vec::new();

        if parts.len() > 1 {
            if parts[0] == ATTRIBUTE_PREFIX {
                let name_part = parts[1..].join(&PROPERTY_PARTS_SEPARATOR.to_string());
                if !skip_validation {
                    self._validate_property_or_attribute_name(&name_part, &bound_prop.source_span, true);
                }
                security_contexts = calc_possible_security_contexts(
                    self.schema_registry,
                    element_selector,
                    &name_part,
                    true,
                );

                let merged_name = if let Some(ns_separator_idx) = name_part.find(':') {
                    let ns = &name_part[0..ns_separator_idx];
                    let n = &name_part[ns_separator_idx + 1..];
                    merge_ns_and_name(Some(ns), n)
                } else {
                    name_part
                };
                bound_property_name = Some(merged_name);

                binding_type = BindingType::Attribute;
            } else if parts[0] == CLASS_PREFIX {
                bound_property_name = Some(parts[1].to_string());
                binding_type = BindingType::Class;
                security_contexts = vec![SecurityContext::NONE];
            } else if parts[0] == STYLE_PREFIX {
                unit = if parts.len() > 2 { Some(parts[2].to_string()) } else { None };
                bound_property_name = Some(parts[1].to_string());
                binding_type = BindingType::Style;
                security_contexts = vec![SecurityContext::STYLE];
            } else if parts[0] == ANIMATE_PREFIX {
                bound_property_name = Some(bound_prop.name.clone());
                binding_type = BindingType::Animation;
                security_contexts = vec![SecurityContext::NONE];
            }
        }

        if bound_property_name.is_none() {
            let mapped_prop_name = self.schema_registry.get_mapped_prop_name(&bound_prop.name);
            security_contexts = calc_possible_security_contexts(
                self.schema_registry,
                element_selector,
                &mapped_prop_name,
                false,
            );
            binding_type = if bound_prop.type_ == ParsedPropertyType::TwoWay {
                BindingType::TwoWay
            } else {
                BindingType::Property
            };
            if !skip_validation {
                self._validate_property_or_attribute_name(&mapped_prop_name, &bound_prop.source_span, false);
            }
            bound_property_name = Some(if map_property_name { mapped_prop_name } else { bound_prop.name.clone() });
        }

        BoundElementProperty::new(
            bound_property_name.unwrap(),
            binding_type,
            security_contexts.first().cloned().unwrap_or(SecurityContext::NONE),
            bound_prop.expression.clone(),
            unit,
            bound_prop.source_span.clone(),
            bound_prop.key_span.clone(),
            bound_prop.value_span.clone(),
        )
    }

    fn _validate_property_or_attribute_name(
        &mut self,
        prop_name: &str,
        source_span: &ParseSourceSpan,
        is_attr: bool,
    ) {
        let report = if is_attr {
            self.schema_registry.validate_attribute(prop_name)
        } else {
            self.schema_registry.validate_property(prop_name)
        };
        
        if report.error {
             self._report_error(&report.msg.unwrap_or_default(), source_span, ParseErrorLevel::Error);
        }
    }

    pub fn parse_property_ast(
        &mut self,
        name: &str,
        ast: ASTWithSource,
        is_part_of_assignment_binding: bool,
        source_span: &ParseSourceSpan,
        key_span: &ParseSourceSpan,
        value_span: Option<&ParseSourceSpan>,
        target_matchable_attrs: &mut Vec<Vec<String>>,
        target_props: &mut Vec<ParsedProperty>,
    ) {
        target_matchable_attrs.push(vec![name.to_string(), ast.source.clone().unwrap_or_default()]);
        target_props.push(ParsedProperty::new(
            name.to_string(),
            ast,
            if is_part_of_assignment_binding { ParsedPropertyType::TwoWay } else { ParsedPropertyType::Default },
            source_span.clone(),
            Some(key_span.clone()),
            value_span.cloned(),
        ));
    }

    fn _parse_animation(
        &mut self,
        name: &str,
        ast: &ASTWithSource,
        source_span: &ParseSourceSpan,
        key_span: &ParseSourceSpan,
        value_span: Option<&ParseSourceSpan>,
        target_matchable_attrs: &mut Vec<Vec<String>>,
        target_props: &mut Vec<ParsedProperty>,
    ) {
        target_matchable_attrs.push(vec![name.to_string(), ast.source.clone().unwrap_or_default()]);
        target_props.push(ParsedProperty::new(
            name.to_string(),
            ast.clone(),
            ParsedPropertyType::Animation,
            source_span.clone(),
            Some(key_span.clone()),
            value_span.cloned(),
        ));
    }
    
    fn _parse_legacy_animation(
        &mut self,
        name: &str,
        expression: &str,
        source_span: &ParseSourceSpan,
        absolute_offset: usize,
        key_span: &ParseSourceSpan,
        value_span: Option<&ParseSourceSpan>,
        target_matchable_attrs: &mut Vec<Vec<String>>,
        target_props: &mut Vec<ParsedProperty>,
    ) {
        if name.is_empty() {
             self._report_error("Animation trigger is missing", source_span, ParseErrorLevel::Error);
        }
        
        let expression_val = if expression.is_empty() { "undefined" } else { expression };
        
        let ast = self.parse_binding(
            expression_val,
            false,
            value_span.cloned().unwrap_or(source_span.clone()),
            absolute_offset
        );
        
        target_matchable_attrs.push(vec![name.to_string(), ast.source.clone().unwrap_or_default()]);
        target_props.push(ParsedProperty::new(
            name.to_string(),
            ast,
            ParsedPropertyType::LegacyAnimation,
            source_span.clone(),
            Some(key_span.clone()),
            value_span.cloned(),
        ));
    }

    pub fn parse_event(
        &mut self,
        name: &str,
        expression: &str,
        is_assignment_event: bool,
        source_span: ParseSourceSpan,
        handler_span: ParseSourceSpan,
        target_matchable_attrs: &mut Vec<Vec<String>>,
        target_events: &mut Vec<ParsedEvent>,
        key_span: Option<ParseSourceSpan>,
    ) {
        if name.is_empty() {
             self._report_error("Event name is missing in binding", &source_span, ParseErrorLevel::Error);
        }
        
        let mut name = name.to_string();
        let mut key_span = key_span;

        if is_legacy_animation_label(&name) {
            name = name[1..].to_string();
             if let Some(ref mut k_span) = key_span {
                 let new_start = k_span.start.move_by(1);
                 let end = k_span.end.clone();
                 *k_span = ParseSourceSpan::new(new_start, end);
             }
             
             self._parse_legacy_animation_event(
                 &name,
                 expression,
                 &source_span,
                 &handler_span,
                 target_events,
                 key_span.as_ref(),
             );
        } else {
             self._parse_regular_event(
                 &name,
                 expression,
                 is_assignment_event,
                 &source_span,
                 &handler_span,
                 target_matchable_attrs,
                 target_events,
                 key_span.as_ref(),
             );
        }
    }
    
    fn _parse_regular_event(
        &mut self,
        name: &str,
        expression: &str,
        is_assignment_event: bool,
        source_span: &ParseSourceSpan,
        handler_span: &ParseSourceSpan,
        target_matchable_attrs: &mut Vec<Vec<String>>,
        target_events: &mut Vec<ParsedEvent>,
        key_span: Option<&ParseSourceSpan>,
    ) {
         let (event_name, target) = parse_event_listener_name(name);
         let prev_error_count = self.errors.len();
         let ast = self._parse_action(expression, handler_span);
         let is_valid = self.errors.len() == prev_error_count;
         
         target_matchable_attrs.push(vec![name.to_string(), ast.source.clone().unwrap_or_default()]);
         
         if is_assignment_event && is_valid && !self._is_allowed_assignment_event(&ast.ast) {
              self._report_error("Unsupported expression in a two-way binding", source_span, ParseErrorLevel::Error);
         }
         
         let mut event_type = ParsedEventType::Regular;
         if is_assignment_event {
             event_type = ParsedEventType::TwoWay;
         }
         if name.starts_with(&format!("{}{}", ANIMATE_PREFIX, PROPERTY_PARTS_SEPARATOR)) {
             event_type = ParsedEventType::Animation;
         }
         
         target_events.push(ParsedEvent::new(
             event_name,
             target,
             event_type,
             ast,
             source_span.clone(),
             handler_span.clone(),
             key_span.cloned(),
         ));
    }

    fn _parse_action(&mut self, value: &str, source_span: &ParseSourceSpan) -> ASTWithSource {
         let absolute_offset = source_span.start.offset;
         match self.expr_parser.parse_action(value, absolute_offset) {
             Ok(ast) => {
                 // Check if empty expression
                 if let ExprAST::EmptyExpr(_) = ast {
                      self._report_error("Empty expressions are not allowed", source_span, ParseErrorLevel::Error);
                      let err_ast = self._wrap_literal_primitive("ERROR", source_span, absolute_offset);
                      return ASTWithSource::new(Box::new(err_ast), Some(value.to_string()), source_span.start.to_string(), absolute_offset, vec![]);
                 }
                 ASTWithSource::new(Box::new(ast), Some(value.to_string()), source_span.start.to_string(), absolute_offset, vec![])
             },
             Err(e) => {
                 self._report_error(&e.to_string(), source_span, ParseErrorLevel::Error);
                 let err_ast = self._wrap_literal_primitive("ERROR", source_span, absolute_offset);
                 ASTWithSource::new(Box::new(err_ast), Some(value.to_string()), source_span.start.to_string(), absolute_offset, vec![])
             }
         }
    }
    
    fn _parse_legacy_animation_event(
        &mut self,
        name: &str,
        expression: &str,
        source_span: &ParseSourceSpan,
        handler_span: &ParseSourceSpan,
        target_events: &mut Vec<ParsedEvent>,
        key_span: Option<&ParseSourceSpan>,
    ) {
        let (event_name, phase) = parse_legacy_animation_event_name(name);
        let ast = self._parse_action(expression, handler_span);
        target_events.push(ParsedEvent::new(
            event_name.clone(),
            phase.clone(),
            ParsedEventType::LegacyAnimation,
            ast,
            source_span.clone(),
            handler_span.clone(),
            key_span.cloned(),
        ));
        
        if event_name.is_empty() {
             self._report_error("Animation event name is missing in binding", source_span, ParseErrorLevel::Error);
        }
        
        if let Some(p) = phase {
             if p != "start" && p != "done" {
                  self._report_error(&format!("The provided animation output phase value \"{}\" for \"@{}\" is not supported (use start or done)", p, event_name), source_span, ParseErrorLevel::Error);
             }
        } else {
             self._report_error(&format!("The animation trigger output event (@{}) is missing its phase value name (start or done are currently supported)", event_name), source_span, ParseErrorLevel::Error);
        }
    }

    fn _is_allowed_assignment_event(&self, ast: &ExprAST) -> bool {
        match ast {
            ExprAST::NonNullAssert(n) => self._is_allowed_assignment_event(&n.expression),
            ExprAST::Call(c) => {
                 if c.args.len() == 1 {
                     if let ExprAST::PropertyRead(p) = &*c.receiver {
                         if p.name == "$any" {
                             if let ExprAST::ImplicitReceiver(_) = &*p.receiver {
                                  return self._is_allowed_assignment_event(&c.args[0]);
                             }
                         }
                     }
                 }
                 false
            },
            ExprAST::PropertyRead(p) => !has_recursive_safe_receiver(&p.receiver),
            ExprAST::KeyedRead(k) => !has_recursive_safe_receiver(&k.receiver),
            _ => false
        }
    }

    fn _report_error(&mut self, message: &str, source_span: &ParseSourceSpan, level: ParseErrorLevel) {
        self.errors.push(ParseError {
            span: source_span.clone(),
            msg: message.to_string(),
            level,
        });
    }
    
    fn _wrap_literal_primitive(&self, value: &str, source_span: &ParseSourceSpan, _absolute_offset: usize) -> ExprAST {
         let span = ParseSpan::new(0, value.len()); 
         let abs_src_span = AbsoluteSourceSpan::new(source_span.start.offset, source_span.end.offset);
         
         ExprAST::LiteralPrimitive(LiteralPrimitive::string(span, abs_src_span, value.to_string()))
    }
}

fn has_recursive_safe_receiver(ast: &ExprAST) -> bool {
    match ast {
        ExprAST::SafePropertyRead(_) | ExprAST::SafeKeyedRead(_) => true,
        ExprAST::ParenthesizedExpression(p) => has_recursive_safe_receiver(&p.expression),
        ExprAST::PropertyRead(p) => has_recursive_safe_receiver(&p.receiver),
        ExprAST::KeyedRead(k) => has_recursive_safe_receiver(&k.receiver),
        ExprAST::Call(c) => has_recursive_safe_receiver(&c.receiver),
        _ => false,
    }
}

fn is_legacy_animation_label(name: &str) -> bool {
    name.starts_with('@')
}

fn parse_event_listener_name(raw_name: &str) -> (String, Option<String>) {
    let parts = split_at_colon(raw_name, &[None, Some(raw_name)]);
    (parts[1].clone().unwrap(), parts[0].clone())
}

fn parse_legacy_animation_event_name(raw_name: &str) -> (String, Option<String>) {
    let parts = split_at_period(raw_name, &[Some(raw_name), None]);
    (parts[0].clone().unwrap(), parts[1].clone().map(|s| s.to_lowercase()))
}

fn move_parse_source_span(source_span: &ParseSourceSpan, absolute_span: &AbsoluteSourceSpan) -> ParseSourceSpan {
    let start_diff = absolute_span.start as isize - source_span.start.offset as isize;
    let end_diff = absolute_span.end as isize - source_span.end.offset as isize;
    
    ParseSourceSpan::new(
        source_span.start.move_by(start_diff as i32),
        source_span.end.move_by(end_diff as i32),
    ).with_details(source_span.details.clone().unwrap_or_default())
}

pub fn calc_possible_security_contexts(
    registry: &dyn ElementSchemaRegistry,
    selector: Option<&str>,
    prop_name: &str,
    is_attribute: bool,
) -> Vec<SecurityContext> {
    let mut ctxs: Vec<SecurityContext> = Vec::new();
    
    if let Some(sel) = selector {
        if let Ok(selectors) = CssSelector::parse(sel) {
            for css_selector in selectors {
                 let element_names = if let Some(el) = &css_selector.element {
                     vec![el.clone()]
                 } else {
                     registry.all_known_element_names()
                 };
                 
                 for el_name in element_names {
                      ctxs.push(registry.security_context(&el_name, prop_name, is_attribute));
                 }
            }
        }
    } else {
         for el_name in registry.all_known_element_names() {
             ctxs.push(registry.security_context(&el_name, prop_name, is_attribute));
         }
    }
    
    if ctxs.is_empty() {
        vec![SecurityContext::NONE]
    } else {
        ctxs.sort();
        ctxs.dedup();
        ctxs
    }
}

fn decode_entities(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '&' {
             let mut entity_buf = String::new();
             let mut is_hex = false;
             let mut is_decimal = false;
             
             if chars.peek() == Some(&'#') {
                 entity_buf.push(chars.next().unwrap());
                 if chars.peek() == Some(&'x') || chars.peek() == Some(&'X') {
                     entity_buf.push(chars.next().unwrap());
                     is_hex = true;
                 } else {
                     is_decimal = true;
                 }
             }
             
             let mut consumed_semicolon = false;
             while let Some(&next_ch) = chars.peek() {
                 if next_ch == ';' {
                     chars.next(); 
                     consumed_semicolon = true;
                     break;
                 }
                 if !next_ch.is_alphanumeric() && next_ch != '#' {
                     break;
                 }
                 entity_buf.push(chars.next().unwrap());
             }
             
             let decoded = if is_hex {
                 let code_str = &entity_buf[2..]; // skip #x
                 u32::from_str_radix(code_str, 16)
                    .ok()
                    .and_then(std::char::from_u32)
                    .map(|c| c.to_string())
             } else if is_decimal {
                 let code_str = &entity_buf[1..]; // skip #
                 u32::from_str_radix(code_str, 10)
                    .ok()
                    .and_then(std::char::from_u32)
                    .map(|c| c.to_string())
             } else {
                 NAMED_ENTITIES.get(entity_buf.as_str()).map(|s| s.to_string())
             };
             
             if let Some(dec) = decoded {
                 result.push_str(&dec);
             } else {
                 result.push('&');
                 result.push_str(&entity_buf);
                 if consumed_semicolon {
                     result.push(';');
                 }
             }
             
        } else {
            result.push(ch);
        }
    }
    result
}
