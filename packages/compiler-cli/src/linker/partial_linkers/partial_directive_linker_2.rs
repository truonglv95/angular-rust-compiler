use crate::linker::ast::AstNode;
use crate::linker::ast_value::{AstObject, AstValue};
use crate::linker::partial_linker::PartialLinker;
use angular_compiler::constant_pool::ConstantPool;
use angular_compiler::output::output_ast as o;
use angular_compiler::parse_util::ParseSourceSpan;
use angular_compiler::render3::util::R3Reference;
use angular_compiler::render3::view::api::{
    R3DirectiveMetadata, R3HostMetadata, R3InputMetadata, R3LifecycleMetadata, R3QueryMetadata,
};
use angular_compiler::render3::view::compiler::compile_directive_from_metadata;
use indexmap::IndexMap;
use std::collections::HashMap;

pub struct PartialDirectiveLinker2;

impl PartialDirectiveLinker2 {
    pub fn new() -> Self {
        Self
    }

    fn to_r3_directive_metadata<TExpression: AstNode>(
        &self,
        meta_obj: &AstObject<TExpression>,
    ) -> Result<R3DirectiveMetadata, String> {
        // Essential fields
        let type_expr = meta_obj.get_value("type")?.node;
        let type_str = meta_obj.host.print_node(&type_expr);

        let wrapped_type = o::Expression::ReadVar(o::ReadVarExpr {
            name: type_str,
            type_: None,
            source_span: None,
        });

        let type_ref = R3Reference {
            value: wrapped_type.clone(),
            type_expr: wrapped_type,
        };

        let selector = meta_obj.get_string("selector").ok();

        // Create a dummy source file for source spans
        let dummy_file = std::sync::Arc::new(angular_compiler::parse_util::ParseSourceFile::new(
            "".to_string(),
            "unknown".to_string(),
        ));
        let dummy_span = ParseSourceSpan::new(
            angular_compiler::parse_util::ParseLocation::new(
                std::sync::Arc::clone(&dummy_file),
                0,
                0,
                0,
            ),
            angular_compiler::parse_util::ParseLocation::new(dummy_file, 0, 0, 0),
        );

        // Inputs
        let mut inputs = IndexMap::new();
        if meta_obj.has("inputs") {
            let inputs_obj = meta_obj.get_object("inputs")?;
            for (key, val) in inputs_obj.to_map() {
                let val_ast = AstValue::new(val.clone(), meta_obj.host);
                if val_ast.is_string() {
                    let binding_name = val_ast.get_string()?;
                    inputs.insert(
                        key.clone(),
                        R3InputMetadata {
                            class_property_name: key.clone(),
                            binding_property_name: binding_name,
                            required: false,
                            is_signal: false,
                            transform_function: None,
                        },
                    );
                } else if val_ast.is_array() {
                    let arr = val_ast.get_array()?;
                    if !arr.is_empty() {
                        let binding_name = arr[0].get_string()?;
                        inputs.insert(
                            key.clone(),
                            R3InputMetadata {
                                class_property_name: key.clone(),
                                binding_property_name: binding_name,
                                required: false,
                                is_signal: false,
                                transform_function: None,
                            },
                        );
                    }
                }
            }
        }

        // Outputs
        let mut outputs = IndexMap::new();
        if meta_obj.has("outputs") {
            let outputs_obj = meta_obj.get_object("outputs")?;
            for (key, val) in outputs_obj.to_map() {
                let val_ast = AstValue::new(val.clone(), meta_obj.host);
                if val_ast.is_string() {
                    let binding_name = val_ast.get_string()?;
                    outputs.insert(key.clone(), binding_name);
                }
            }
        }

        // Queries
        let queries = if meta_obj.has("queries") {
            meta_obj
                .get_array("queries")?
                .iter()
                .map(|q| {
                    let q_obj = q.get_object()?;
                    let property_name = q_obj.get_string("propertyName")?;
                    let first = q_obj.get_bool("first").unwrap_or(false);
                    let predicate = if q_obj.has("predicate") {
                        let p = q_obj.get_value("predicate")?;
                        if p.is_string() {
                            angular_compiler::render3::view::api::R3QueryPredicate::Selectors(vec![
                                p.get_string()?,
                            ])
                        } else if p.is_array() {
                            let arr = p.get_array()?;
                            let selectors = arr
                                .iter()
                                .map(|s| s.get_string())
                                .collect::<Result<_, _>>()?;
                            angular_compiler::render3::view::api::R3QueryPredicate::Selectors(
                                selectors,
                            )
                        } else {
                            angular_compiler::render3::view::api::R3QueryPredicate::Selectors(vec![
                                "".to_string(),
                            ])
                        }
                    } else {
                        angular_compiler::render3::view::api::R3QueryPredicate::Selectors(vec![])
                    };

                    Ok(R3QueryMetadata {
                        property_name,
                        first,
                        predicate,
                        descendants: q_obj.get_bool("descendants").unwrap_or(false),
                        emit_distinct_changes_only: q_obj
                            .get_bool("emitDistinctChangesOnly")
                            .unwrap_or(true),
                        read: None, // TODO: handle read token
                        static_: q_obj.get_bool("static").unwrap_or(false),
                        is_signal: q_obj.get_bool("isSignal").unwrap_or(false),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?
        } else {
            vec![]
        };

        let view_queries = vec![]; // TODO: extract from 'viewQueries' property

        // Host
        let host = if meta_obj.has("host") {
            let host_obj = meta_obj.get_object("host")?;
            let mut attributes = HashMap::new();
            let mut listeners = HashMap::new();
            let mut properties = HashMap::new();
            let mut special_attributes =
                angular_compiler::render3::view::api::R3HostSpecialAttributes::default();

            // Handle nested "listeners" object (from ɵɵngDeclareDirective format)
            if host_obj.has("listeners") {
                if let Ok(listeners_obj) = host_obj.get_object("listeners") {
                    for (event_name, handler) in listeners_obj.to_map() {
                        let handler_ast = AstValue::new(handler.clone(), meta_obj.host);
                        if handler_ast.is_string() {
                            listeners.insert(event_name.clone(), handler_ast.get_string()?);
                        }
                    }
                }
            }

            // Handle nested "properties" object (from ɵɵngDeclareDirective format)
            if host_obj.has("properties") {
                if let Ok(properties_obj) = host_obj.get_object("properties") {
                    for (prop_name, binding) in properties_obj.to_map() {
                        let binding_ast = AstValue::new(binding.clone(), meta_obj.host);
                        if binding_ast.is_string() {
                            properties.insert(prop_name.clone(), binding_ast.get_string()?);
                        }
                    }
                }
            }

            // Handle nested "attributes" object (from ɵɵngDeclareDirective format)
            if host_obj.has("attributes") {
                if let Ok(attributes_obj) = host_obj.get_object("attributes") {
                    for (attr_name, attr_value) in attributes_obj.to_map() {
                        let attr_ast = AstValue::new(attr_value.clone(), meta_obj.host);
                        if attr_ast.is_string() {
                            let val_str = attr_ast.get_string()?;
                            attributes.insert(
                                attr_name.clone(),
                                o::Expression::Literal(o::LiteralExpr {
                                    value: o::LiteralValue::String(val_str),
                                    type_: None,
                                    source_span: None,
                                }),
                            );
                        }
                    }
                }
            }

            // Also handle flat format: host: { "(event)": "handler", "[prop]": "binding" }
            for (key, val) in host_obj.to_map() {
                // Skip nested objects we already handled
                if key == "listeners" || key == "properties" || key == "attributes" {
                    continue;
                }

                let val_ast = AstValue::new(val.clone(), meta_obj.host);
                if val_ast.is_string() {
                    let val_str = val_ast.get_string()?;
                    if key.starts_with("(") && key.ends_with(")") {
                        let event_name = &key[1..key.len() - 1];
                        listeners.insert(event_name.to_string(), val_str);
                    } else if key.starts_with("[") && key.ends_with("]") {
                        let prop_name = &key[1..key.len() - 1];
                        properties.insert(prop_name.to_string(), val_str);
                    } else if key == "class" {
                        special_attributes.class_attr = Some(val_str);
                    } else if key == "style" {
                        special_attributes.style_attr = Some(val_str);
                    } else {
                        attributes.insert(
                            key.clone(),
                            o::Expression::Literal(o::LiteralExpr {
                                value: o::LiteralValue::String(val_str),
                                type_: None,
                                source_span: None,
                            }),
                        );
                    }
                }
            }

            R3HostMetadata {
                attributes,
                listeners,
                properties,
                special_attributes,
            }
        } else {
            R3HostMetadata::default()
        };

        let directive = R3DirectiveMetadata {
            name: "Directive".to_string(), // TODO: extract class name
            type_: type_ref,
            type_argument_count: 0,
            type_source_span: dummy_span,
            deps: None,
            selector: selector,
            queries,
            view_queries,
            host,
            lifecycle: R3LifecycleMetadata::default(),
            inputs,
            outputs,
            uses_inheritance: meta_obj.get_bool("usesInheritance").unwrap_or(false),
            export_as: None,
            providers: if meta_obj.has("providers") {
                let providers_node = meta_obj.get_value("providers")?.node;
                let providers_str = meta_obj.host.print_node(&providers_node);
                Some(o::Expression::RawCode(o::RawCodeExpr {
                    code: providers_str,
                    source_span: None,
                }))
            } else {
                None
            },
            is_standalone: meta_obj.get_bool("isStandalone").unwrap_or(false),
            is_signal: meta_obj.get_bool("isSignal").unwrap_or(false),
            host_directives: None,
        };

        Ok(directive)
    }
}

impl<TExpression: AstNode> PartialLinker<TExpression> for PartialDirectiveLinker2 {
    fn link_partial_declaration(
        &self,
        constant_pool: &mut ConstantPool,
        meta_obj: &AstObject<TExpression>,
        _source_url: &str,
        _version: &str,
        _target_name: Option<&str>,
    ) -> o::Expression {
        match self.to_r3_directive_metadata(meta_obj) {
            Ok(meta) => {
                let parser = angular_compiler::expression_parser::parser::Parser::new();
                struct DummySchemaRegistry;
                impl angular_compiler::schema::element_schema_registry::ElementSchemaRegistry
                    for DummySchemaRegistry
                {
                    fn has_property(
                        &self,
                        _tag_name: &str,
                        _prop_name: &str,
                        _schemas: &[angular_compiler::core::SchemaMetadata],
                    ) -> bool {
                        true
                    }
                    fn has_element(
                        &self,
                        _tag_name: &str,
                        _schemas: &[angular_compiler::core::SchemaMetadata],
                    ) -> bool {
                        true
                    }
                    fn security_context(
                        &self,
                        _tag_name: &str,
                        _prop_name: &str,
                        _is_attribute: bool,
                    ) -> angular_compiler::core::SecurityContext {
                        angular_compiler::core::SecurityContext::NONE
                    }
                    fn all_known_element_names(&self) -> Vec<String> {
                        vec![]
                    }
                    fn get_mapped_prop_name(&self, prop_name: &str) -> String {
                        prop_name.to_string()
                    }
                    fn get_default_component_element_name(&self) -> String {
                        "ng-component".to_string()
                    }
                    fn validate_property(
                        &self,
                        _name: &str,
                    ) -> angular_compiler::schema::element_schema_registry::ValidationResult
                    {
                        angular_compiler::schema::element_schema_registry::ValidationResult {
                            error: false,
                            msg: None,
                        }
                    }
                    fn validate_attribute(
                        &self,
                        _name: &str,
                    ) -> angular_compiler::schema::element_schema_registry::ValidationResult
                    {
                        angular_compiler::schema::element_schema_registry::ValidationResult {
                            error: false,
                            msg: None,
                        }
                    }
                    fn normalize_animation_style_property(&self, prop_name: &str) -> String {
                        prop_name.to_string()
                    }
                    fn normalize_animation_style_value(
                        &self,
                        _camel_case_prop: &str,
                        _user_provided_prop: &str,
                        val: &str,
                    ) -> angular_compiler::schema::element_schema_registry::NormalizationResult
                    {
                        angular_compiler::schema::element_schema_registry::NormalizationResult {
                            error: "".to_string(),
                            value: val.to_string(),
                        }
                    }
                }
                let schema_registry = DummySchemaRegistry;

                let mut binding_parser =
                    angular_compiler::template_parser::binding_parser::BindingParser::new(
                        &parser,
                        &schema_registry,
                        vec![],
                    );

                let res =
                    compile_directive_from_metadata(&meta, constant_pool, &mut binding_parser);
                res.expression
            }
            Err(e) => o::Expression::Literal(o::LiteralExpr {
                value: o::LiteralValue::String(format!("Error: {}", e)),
                type_: None,
                source_span: None,
            }),
        }
    }
}
