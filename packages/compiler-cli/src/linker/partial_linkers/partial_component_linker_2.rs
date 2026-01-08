use crate::linker::ast::AstNode;
use crate::linker::ast_value::{AstObject, AstValue};
use crate::linker::metadata_extractor;
use crate::linker::partial_linker::PartialLinker;
use angular_compiler::constant_pool::ConstantPool;
use angular_compiler::core::{ChangeDetectionStrategy, ViewEncapsulation};
use angular_compiler::output::output_ast as o;
use angular_compiler::parse_util::ParseSourceSpan;
use angular_compiler::render3::util::R3Reference;
use angular_compiler::render3::view::api::{
    ChangeDetectionOrExpression, DeclarationListEmitMode, R3ComponentDeferMetadata,
    R3ComponentTemplate, R3DirectiveDependencyMetadata, R3DirectiveMetadata, R3HostMetadata,
    R3InputMetadata, R3LifecycleMetadata, R3PipeDependencyMetadata, R3QueryMetadata,
    R3TemplateDependencyKind, R3TemplateDependencyMetadata,
};
use angular_compiler::render3::view::compiler::compile_component_from_metadata;
use angular_compiler::render3::view::R3ComponentMetadata;
use indexmap::IndexMap;
use std::collections::HashMap;

pub struct PartialComponentLinker2;

impl PartialComponentLinker2 {
    pub fn new() -> Self {
        Self
    }

    fn to_r3_component_metadata<TExpression: AstNode>(
        &self,
        meta_obj: &AstObject<TExpression>,
        source_url: &str,
        target_name: Option<&str>,
        imports: Option<&std::collections::HashMap<String, String>>,
    ) -> Result<R3ComponentMetadata, String> {
        // DEBUG: Trace function entry
        // eprintln!("[Linker2] to_r3_component_metadata called, target_name: {:?}, source: {}", target_name, source_url);

        // DEBUG: Print all metadata keys
        // let keys: Vec<String> = meta_obj.to_map().keys().cloned().collect();
        // eprintln!("[Linker2] Metadata keys: {:?}", keys);

        // Essential fields
        let type_name_str = if let Ok(t) = meta_obj.get_value("type") {
            meta_obj.host.print_node(&t.node)
        } else if let Some(name) = target_name {
            name.to_string()
        } else {
            return Err("Missing 'type' property and no target_name provided".to_string());
        };

        // Create a ReadVarExpr with the string representation
        // This avoids WrappedNodeExpr and 'static requirement
        let wrapped_type = o::Expression::ReadVar(o::ReadVarExpr {
            name: type_name_str.clone(),
            type_: None,
            source_span: None,
        });

        let type_ref = R3Reference {
            value: wrapped_type.clone(),
            type_expr: wrapped_type,
        };

        let selector = meta_obj.get_string("selector").ok();
        let mut template_str = meta_obj.get_string("template").unwrap_or_default();

        if template_str.is_empty() {}

        if template_str.is_empty() && meta_obj.has("templateUrl") {
            let url = meta_obj.get_string("templateUrl")?;
            // Resolve path relative to source_url
            // source_url is likely absolute if from rspack loader
            let source_path = std::path::Path::new(source_url);
            let parent = source_path.parent().unwrap_or(std::path::Path::new("."));
            let template_path = parent.join(url);

            // Read file
            match std::fs::read_to_string(&template_path) {
                Ok(content) => {
                    template_str = content;
                }
                Err(e) => {
                    return Err(format!(
                        "Failed to read template at {:?}: {}",
                        template_path, e
                    ))
                }
            }
        }

        let mut styles = Vec::new();
        if meta_obj.has("styles") {
            if let Ok(arr) = meta_obj.get_array("styles") {
                for entry in arr {
                    if let Ok(s) = entry.get_string() {
                        styles.push(s);
                    }
                }
            }
        }

        // Handle styleUrl (single)
        if meta_obj.has("styleUrl") {
            if let Ok(url) = meta_obj.get_string("styleUrl") {
                let source_path = std::path::Path::new(source_url);
                let parent = source_path.parent().unwrap_or(std::path::Path::new("."));
                let style_path = parent.join(url);
                if let Ok(content) = std::fs::read_to_string(&style_path) {
                    styles.push(content);
                }
            }
        }

        // Handle styleUrls (array)
        if meta_obj.has("styleUrls") {
            if let Ok(urls) = meta_obj.get_array("styleUrls") {
                for entry in urls {
                    if let Ok(url) = entry.get_string() {
                        let source_path = std::path::Path::new(source_url);
                        let parent = source_path.parent().unwrap_or(std::path::Path::new("."));
                        let style_path = parent.join(url);
                        if let Ok(content) = std::fs::read_to_string(&style_path) {
                            styles.push(content);
                        }
                    }
                }
            }
        }

        let encapsulation = if meta_obj.has("encapsulation") {
            let mut idx = 0; // Default ViewEncapsulation.Emulated
            if let Ok(val) = meta_obj.get_number("encapsulation") {
                idx = val as u32;
            } else if let Ok(val) = meta_obj.get_value("encapsulation") {
                let text = val.print();
                if text.contains("ViewEncapsulation.None") {
                    idx = 2;
                } else if text.contains("ViewEncapsulation.ShadowDom") {
                    idx = 3;
                } else if text.contains("ViewEncapsulation.Emulated") {
                    idx = 0;
                } else if text.contains("ViewEncapsulation.Native") {
                    idx = 1;
                }
            }
            match idx {
                0 => ViewEncapsulation::Emulated,
                2 => ViewEncapsulation::None,
                3 => ViewEncapsulation::ShadowDom,
                _ => ViewEncapsulation::Emulated,
            }
        } else {
            ViewEncapsulation::Emulated
        };

        let change_detection = if meta_obj.has("changeDetection") {
            let mut idx = 1; // Default ChangeDetectionStrategy.Default
            if let Ok(val) = meta_obj.get_number("changeDetection") {
                idx = val as u32;
            } else if let Ok(val) = meta_obj.get_value("changeDetection") {
                let text = val.print();
                if text.contains("ChangeDetectionStrategy.OnPush") {
                    idx = 0;
                } else if text.contains("ChangeDetectionStrategy.Default") {
                    idx = 1;
                }
            }
            let strategy = match idx {
                0 => ChangeDetectionStrategy::OnPush,
                1 => ChangeDetectionStrategy::Default,
                _ => ChangeDetectionStrategy::Default,
            };
            Some(ChangeDetectionOrExpression::Strategy(strategy))
        } else {
            None
        };

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

        // Parse template
        let template_opts = angular_compiler::render3::view::template::ParseTemplateOptions {
            preserve_whitespaces: Some(false), // TODO: get from metadata
            ..Default::default()
        };
        let parsed_template = angular_compiler::render3::view::template::parse_template(
            &template_str,
            "template.html", // TODO: get real URL
            template_opts,
        );

        // Inputs
        let mut inputs = IndexMap::new();
        if meta_obj.has("inputs") {
            let inputs_obj = meta_obj.get_object("inputs")?;
            let inputs_map = inputs_obj.to_map();
            let mut keys: Vec<String> = inputs_map.keys().cloned().collect();
            keys.sort();
            for key in keys {
                let val = inputs_map.get(&key).unwrap();
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
                        let mut _alias = binding_name.clone();
                        let mut transform_function = None;

                        // Check for alias/flags at index 1
                        if arr.len() > 1 {
                            if let Ok(s) = arr[1].get_string() {
                                _alias = s;
                            }
                        }

                        // Check for transform at index 2?
                        // Note: Partial Ivy inputs format is typically [publicName, alias]
                        // If transform is present, it might be at index 2.
                        if arr.len() > 2 {
                            let transform_node = &arr[2];
                            // Transform is usually an expression (identifier or arrow fn)
                            // We construct a RawCode expression for it
                            let transform_str = meta_obj.host.print_node(&transform_node.node);
                            transform_function = Some(o::Expression::RawCode(o::RawCodeExpr {
                                code: format!("({})", transform_str),
                                source_span: None,
                            }));
                        }

                        inputs.insert(
                            key.clone(),
                            R3InputMetadata {
                                class_property_name: key.clone(),
                                binding_property_name: binding_name,
                                required: false,
                                is_signal: false,
                                transform_function,
                            },
                        );
                    }
                } else if val_ast.is_object() {
                    let input_obj = val_ast.get_object()?;
                    let class_property_name = input_obj
                        .get_string("classPropertyName")
                        .unwrap_or_else(|_| key.clone());
                    let binding_property_name = input_obj
                        .get_string("publicName")
                        .unwrap_or_else(|_| key.clone());
                    let required = input_obj.get_bool("isRequired").unwrap_or(false);
                    let is_signal = input_obj.get_bool("isSignal").unwrap_or(false);
                    let transform_function = if input_obj.has("transformFunction") {
                        let transform_node = input_obj.get_value("transformFunction")?.node;
                        let transform_str = meta_obj.host.print_node(&transform_node);
                        Some(o::Expression::RawCode(o::RawCodeExpr {
                            code: format!("({})", transform_str),
                            source_span: None,
                        }))
                    } else {
                        None
                    };

                    inputs.insert(
                        key.clone(),
                        R3InputMetadata {
                            class_property_name,
                            binding_property_name,
                            required,
                            is_signal,
                            transform_function,
                        },
                    );
                }
            }
        }

        // Parse propDecorators for Inputs (and potentially Outputs/Queries)
        if meta_obj.has("propDecorators") {
            let props = meta_obj.get_object("propDecorators")?;
            for (prop_name, decorators_val) in props.to_map() {
                let decorators_ast = AstValue::new(decorators_val.clone(), meta_obj.host);
                if let Ok(decorators) = decorators_ast.get_array() {
                    for decorator in decorators {
                        if let Ok(decorator_obj) = decorator.get_object() {
                            if let Ok(type_node) = decorator_obj.get_value("type") {
                                let type_name = meta_obj.host.print_node(&type_node.node);
                                // Check if it's Input
                                if type_name.ends_with("Input") {
                                    let mut binding_name = prop_name.clone();
                                    let mut required = false;
                                    let mut is_signal = false;
                                    let mut transform_function = None;
                                    let mut alias = None;

                                    if let Ok(args) = decorator_obj.get_array("args") {
                                        if !args.is_empty() {
                                            let arg0 = &args[0];
                                            if arg0.is_string() {
                                                alias = Some(arg0.get_string()?);
                                                binding_name = alias.clone().unwrap();
                                            } else if arg0.is_object() {
                                                let config = arg0.get_object()?;
                                                if let Ok(a) = config.get_string("alias") {
                                                    alias = Some(a.clone());
                                                    binding_name = a;
                                                }
                                                required =
                                                    config.get_bool("required").unwrap_or(false);
                                                // isSignal isn't typically in @Input args, mainly transform
                                                if config.has("transform") {
                                                    let transform_node =
                                                        config.get_value("transform")?.node;
                                                    let transform_str =
                                                        meta_obj.host.print_node(&transform_node);
                                                    transform_function = Some(
                                                        o::Expression::RawCode(o::RawCodeExpr {
                                                            code: format!("({})", transform_str),
                                                            source_span: None,
                                                        }),
                                                    );
                                                }
                                            }
                                        }
                                    }

                                    inputs.insert(
                                        prop_name.clone(),
                                        R3InputMetadata {
                                            class_property_name: prop_name.clone(),
                                            binding_property_name: binding_name,
                                            required,
                                            is_signal,
                                            transform_function,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Outputs
        let mut outputs = IndexMap::new();
        if meta_obj.has("outputs") {
            let outputs_obj = meta_obj.get_object("outputs")?;
            let outputs_map = outputs_obj.to_map();
            let mut keys: Vec<String> = outputs_map.keys().cloned().collect();
            keys.sort();
            for key in keys {
                let val = outputs_map.get(&key).unwrap();
                let val_ast = AstValue::new(val.clone(), meta_obj.host);
                if val_ast.is_string() {
                    let binding_name = val_ast.get_string()?;
                    outputs.insert(key.clone(), binding_name);
                }
            }
        }

        // Queries
        let queries = if meta_obj.has("queries") {
            let queries_arr = meta_obj.get_array("queries")?;
            // eprintln!("[Linker] Found {} queries", queries_arr.len());
            queries_arr
                .iter()
                .map(|q| {
                    let q_obj = q.get_object()?;
                    let property_name = q_obj.get_string("propertyName")?;
                    let is_signal = q_obj.get_bool("isSignal").unwrap_or(false);
                    let first = q_obj.get_bool("first").unwrap_or(false);
                    let predicate = if q_obj.has("predicate") {
                        let p = q_obj.get_value("predicate")?;
                        if p.is_string() {
                            angular_compiler::render3::view::api::R3QueryPredicate::Selectors(vec![
                                p.get_string()?,
                            ])
                        } else if p.is_array() {
                            let arr = p.get_array()?;
                            // Check if array contains strings (selectors) or other types
                            let first_is_string = arr.first().map(|f| f.is_string()).unwrap_or(true);
                            if first_is_string {
                                let selectors = arr
                                    .iter()
                                    .map(|s| s.get_string())
                                    .collect::<Result<_, _>>()?;
                                angular_compiler::render3::view::api::R3QueryPredicate::Selectors(
                                    selectors,
                                )
                            } else {
                                // Array of expressions - use first element
                                let expr_str = meta_obj.host.print_node(&p.node);
                                let expr = o::Expression::RawCode(o::RawCodeExpr {
                                    code: expr_str,
                                    source_span: None,
                                });
                                angular_compiler::render3::view::api::R3QueryPredicate::Expression(
                                    angular_compiler::render3::util::MaybeForwardRefExpression {
                                        expression: expr,
                                        forward_ref: angular_compiler::render3::util::ForwardRefHandling::None,
                                    },
                                )
                            }
                        } else {
                            // It's an expression (class reference like MatFormFieldControl)
                            let expr_str = meta_obj.host.print_node(&p.node);
                            let expr = o::Expression::RawCode(o::RawCodeExpr {
                                code: expr_str,
                                source_span: None,
                            });
                            angular_compiler::render3::view::api::R3QueryPredicate::Expression(
                                angular_compiler::render3::util::MaybeForwardRefExpression {
                                    expression: expr,
                                    forward_ref: angular_compiler::render3::util::ForwardRefHandling::None,
                                },
                            )
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

        // ViewQueries - extract from 'viewQueries' property (for @ViewChild/@ViewChildren)
        let view_queries = if meta_obj.has("viewQueries") {
            let view_queries_arr = meta_obj.get_array("viewQueries")?;
            // eprintln!("[Linker] Found {} viewQueries", view_queries_arr.len());
            view_queries_arr
                .iter()
                .map(|q| {
                    let q_obj = q.get_object()?;
                    let property_name = q_obj.get_string("propertyName")?;
                    let is_signal = q_obj.get_bool("isSignal").unwrap_or(false);
                    // DEBUG: Log view query details

                    let first = q_obj.get_bool("first").unwrap_or(false);
                    let predicate = if q_obj.has("predicate") {
                        let p = q_obj.get_value("predicate")?;
                        if p.is_string() {
                            angular_compiler::render3::view::api::R3QueryPredicate::Selectors(vec![
                                p.get_string()?,
                            ])
                        } else if p.is_array() {
                            let arr = p.get_array()?;
                            // Check if array contains strings (selectors) or other types
                            let first_is_string = arr.first().map(|f| f.is_string()).unwrap_or(true);
                            if first_is_string {
                                let selectors = arr
                                    .iter()
                                    .map(|s| s.get_string())
                                    .collect::<Result<_, _>>()?;
                                angular_compiler::render3::view::api::R3QueryPredicate::Selectors(
                                    selectors,
                                )
                            } else {
                                // Array of expressions - use first element
                                let expr_str = meta_obj.host.print_node(&p.node);
                                let expr = o::Expression::RawCode(o::RawCodeExpr {
                                    code: expr_str,
                                    source_span: None,
                                });
                                angular_compiler::render3::view::api::R3QueryPredicate::Expression(
                                    angular_compiler::render3::util::MaybeForwardRefExpression {
                                        expression: expr,
                                        forward_ref: angular_compiler::render3::util::ForwardRefHandling::None,
                                    },
                                )
                            }
                        } else {
                            // It's an expression (class reference)
                            let expr_str = meta_obj.host.print_node(&p.node);
                            let expr = o::Expression::RawCode(o::RawCodeExpr {
                                code: expr_str,
                                source_span: None,
                            });
                            angular_compiler::render3::view::api::R3QueryPredicate::Expression(
                                angular_compiler::render3::util::MaybeForwardRefExpression {
                                    expression: expr,
                                    forward_ref: angular_compiler::render3::util::ForwardRefHandling::None,
                                },
                            )
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

        // Host
        let has_host = meta_obj.has("host");
        if has_host {
            // eprintln!(
            //     "[Linker] Found 'host' key in metadata! Keys: {:?}",
            //     meta_obj.to_map().keys().cloned().collect::<Vec<_>>()
            // );
        }
        let host = if has_host {
            let host_obj = meta_obj.get_object("host")?;

            let mut attributes = HashMap::new();
            let mut listeners = HashMap::new();
            let mut properties = HashMap::new();
            let mut special_attributes =
                angular_compiler::render3::view::api::R3HostSpecialAttributes::default();

            for (key, val) in host_obj.to_map() {
                let val_ast = AstValue::new(val.clone(), meta_obj.host);

                if key == "attributes" && val_ast.is_object() {
                    let obj = val_ast.get_object()?;
                    for (k, v) in obj.to_map() {
                        let v_ast = AstValue::new(v.clone(), meta_obj.host);
                        let v_str = if v_ast.is_string() {
                            v_ast.get_string()?
                        } else {
                            v_ast.print() // For non-string expressions, use print()
                        };
                        attributes.insert(
                            k.clone(),
                            o::Expression::Literal(o::LiteralExpr {
                                value: o::LiteralValue::String(v_str),
                                type_: None,
                                source_span: None,
                            }),
                        );
                    }
                } else if key == "listeners" && val_ast.is_object() {
                    let obj = val_ast.get_object()?;
                    for (k, v) in obj.to_map() {
                        let v_ast = AstValue::new(v.clone(), meta_obj.host);
                        let v_str = if v_ast.is_string() {
                            v_ast.get_string()?
                        } else {
                            v_ast.print()
                        };
                        listeners.insert(k.clone(), v_str);
                    }
                } else if key == "properties" && val_ast.is_object() {
                    let obj = val_ast.get_object()?;
                    for (k, v) in obj.to_map() {
                        let v_ast = AstValue::new(v.clone(), meta_obj.host);
                        let v_str = if v_ast.is_string() {
                            v_ast.get_string()?
                        } else {
                            v_ast.print()
                        };
                        properties.insert(k.clone(), v_str);
                    }
                } else if key == "classAttribute" {
                    let class_val = if val_ast.is_string() {
                        val_ast.get_string()?
                    } else {
                        val_ast.print()
                    };
                    special_attributes.class_attr = Some(class_val);
                } else if key == "styleAttribute" {
                    let style_val = if val_ast.is_string() {
                        val_ast.get_string()?
                    } else {
                        val_ast.print()
                    };
                    special_attributes.style_attr = Some(style_val);
                } else {
                    // Fallback for flat format or other keys
                    let is_str = val_ast.is_string();
                    let val_str = if is_str {
                        val_ast.get_string()?
                    } else {
                        val_ast.print()
                    };

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

        // Dependencies (Directives/Pipes)
        let mut declarations = Vec::new();
        if meta_obj.has("dependencies") {
            if let Ok(deps_arr) = meta_obj.get_array("dependencies") {
                for dep in deps_arr {
                    if let Ok(dep_obj) = dep.get_object() {
                        let kind_str = dep_obj
                            .get_string("kind")
                            .unwrap_or("directive".to_string());
                        let kind = match kind_str.as_str() {
                            "directive" | "component" => R3TemplateDependencyKind::Directive,
                            "pipe" => R3TemplateDependencyKind::Pipe,
                            "ngmodule" => R3TemplateDependencyKind::NgModule,
                            _ => R3TemplateDependencyKind::Directive,
                        };

                        if kind == R3TemplateDependencyKind::NgModule {
                            if let Ok(type_node) = dep_obj.get_value("type") {
                                let type_str = meta_obj.host.print_node(&type_node.node);
                                // Expected format: "alias.ModuleName" (e.g. "i1.MatFormFieldModule")
                                if let Some((alias, module_class)) = type_str.split_once('.') {
                                    if let Some(imports_map) = imports {
                                        if let Some(module_path) = imports_map.get(alias) {
                                            // Resolve exports
                                            if let Some(exported_directives) =
                                                metadata_extractor::get_module_exports(
                                                    module_path,
                                                    module_class,
                                                )
                                            {
                                                for dir in exported_directives {
                                                    // Construct type reference: alias.DirectiveName
                                                    let dir_type_expr =
                                                        o::Expression::ReadProp(o::ReadPropExpr {
                                                            receiver: Box::new(
                                                                o::Expression::ReadVar(
                                                                    o::ReadVarExpr {
                                                                        name: alias.to_string(),
                                                                        type_: None,
                                                                        source_span: None,
                                                                    },
                                                                ),
                                                            ),
                                                            name: dir.name.clone(),
                                                            type_: None,
                                                            source_span: None,
                                                        });

                                                    if dir.is_component {
                                                        // Treat component as directive for dependency purposes
                                                        declarations.push(
                                                            R3TemplateDependencyMetadata::Directive(
                                                                R3DirectiveDependencyMetadata {
                                                                    kind: R3TemplateDependencyKind::Directive,
                                                                    type_: dir_type_expr,
                                                                    selector: dir.selector,
                                                                    inputs: dir.inputs,
                                                                    outputs: dir.outputs,
                                                                    export_as: dir.export_as.clone().map(|s| s.split(',').map(|x| x.to_string()).collect()),
                                                                    is_component: true,
                                                                    source_span: None,
                                                                },
                                                            ),
                                                        );
                                                    } else {
                                                        // Check if it's a pipe (metadata extractor doesn't distinguish nicely yet,
                                                        // but usually directives are directives.
                                                        // TODO: Update metadata extractor to distinguish pipes?
                                                        // For now assume directive.
                                                        declarations.push(
                                                            R3TemplateDependencyMetadata::Directive(
                                                                R3DirectiveDependencyMetadata {
                                                                    kind: R3TemplateDependencyKind::Directive,
                                                                    type_: dir_type_expr,
                                                                    selector: dir.selector,
                                                                    inputs: dir.inputs,
                                                                    outputs: dir.outputs,
                                                                    export_as: dir.export_as.clone().map(|s| s.split(',').map(|x| x.to_string()).collect()),
                                                                    is_component: false,
                                                                    source_span: None,
                                                                },
                                                            ),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            continue;
                        }

                        let selector = dep_obj.get_string("selector").unwrap_or_default();
                        let type_node = dep_obj.get_value("type")?;
                        let type_str = meta_obj.host.print_node(&type_node.node);
                        eprintln!(
                            "[Linker Debug] Dependency type for selector '{}': {}",
                            selector, type_str
                        );
                        let type_expr = o::Expression::ReadVar(o::ReadVarExpr {
                            name: type_str,
                            type_: None,
                            source_span: None,
                        });

                        let mut inputs = Vec::new();
                        if let Ok(inputs_arr) = dep_obj.get_array("inputs") {
                            for input in inputs_arr {
                                if let Ok(s) = input.get_string() {
                                    inputs.push(s);
                                }
                            }
                        }

                        let mut outputs = Vec::new();
                        if let Ok(outputs_arr) = dep_obj.get_array("outputs") {
                            for output in outputs_arr {
                                if let Ok(s) = output.get_string() {
                                    outputs.push(s);
                                }
                            }
                        }

                        let mut export_as = Vec::new();
                        if let Ok(export_as_arr) = dep_obj.get_array("exportAs") {
                            for e in export_as_arr {
                                if let Ok(s) = e.get_string() {
                                    export_as.push(s);
                                }
                            }
                        }

                        if kind == R3TemplateDependencyKind::Pipe {
                            declarations.push(R3TemplateDependencyMetadata::Pipe(
                                R3PipeDependencyMetadata {
                                    kind,
                                    type_: type_expr,
                                    name: selector, // Pipe name is passed in selector field in Partial Ivy
                                    source_span: None,
                                },
                            ));
                        } else {
                            declarations.push(R3TemplateDependencyMetadata::Directive(
                                R3DirectiveDependencyMetadata {
                                    kind,
                                    type_: type_expr,
                                    selector,
                                    inputs,
                                    outputs,
                                    export_as: if export_as.is_empty() {
                                        None
                                    } else {
                                        Some(export_as)
                                    },
                                    is_component: kind_str == "component",
                                    source_span: None,
                                },
                            ));
                        }
                    }
                }
            }
        }

        // Parse exportAs
        let export_as = if meta_obj.has("exportAs") {
            if let Ok(arr) = meta_obj.get_array("exportAs") {
                let exports: Result<Vec<String>, String> =
                    arr.iter().map(|e| e.get_string()).collect();
                exports.ok()
            } else {
                None
            }
        } else {
            None
        };

        // Parse usesInheritance
        let uses_inheritance = meta_obj.get_bool("usesInheritance").unwrap_or(false);

        // Extract providers - this is critical for DI tokens like MAT_FORM_FIELD
        let providers = if meta_obj.has("providers") {
            if let Ok(providers_val) = meta_obj.get_value("providers") {
                // Preserve the providers array as a RawCode expression
                // This keeps token references like MAT_FORM_FIELD intact
                let providers_str = meta_obj.host.print_node(&providers_val.node);
                Some(o::Expression::RawCode(o::RawCodeExpr {
                    code: providers_str,
                    source_span: None,
                }))
            } else {
                None
            }
        } else {
            None
        };

        // Extract simple class name from type_name_str (e.g. "i0.MyComponent" -> "MyComponent")
        let simple_name = type_name_str
            .split('.')
            .last()
            .unwrap_or(&type_name_str)
            .to_string();

        // Construct R3DirectiveMetadata (subset)
        let directive = R3DirectiveMetadata {
            name: simple_name,
            type_: type_ref.clone(),
            type_argument_count: 0,
            type_source_span: dummy_span,
            deps: None,
            selector: selector.clone(),
            queries,
            view_queries,
            host,
            lifecycle: R3LifecycleMetadata::default(),
            inputs,
            outputs,
            uses_inheritance,
            export_as,
            providers,
            is_standalone: meta_obj.get_bool("isStandalone").unwrap_or(false),
            is_signal: meta_obj.get_bool("isSignal").unwrap_or(false),
            host_directives: None,
        };

        // Create a dummy Parser and SchemaRegistry for BindingParser
        // In a real scenario, these should probably be passed in or created with real data
        let _parser = angular_compiler::expression_parser::parser::Parser::new();
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
            ) -> angular_compiler::schema::element_schema_registry::ValidationResult {
                angular_compiler::schema::element_schema_registry::ValidationResult {
                    error: false,
                    msg: None,
                }
            }
            fn validate_attribute(
                &self,
                _name: &str,
            ) -> angular_compiler::schema::element_schema_registry::ValidationResult {
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
        let _schema_registry = DummySchemaRegistry;

        Ok(R3ComponentMetadata {
            directive,
            template: R3ComponentTemplate {
                nodes: parsed_template.nodes,
                ng_content_selectors: parsed_template.ng_content_selectors,
                preserve_whitespaces: false,
            },
            declarations,
            defer: R3ComponentDeferMetadata::PerComponent {
                dependencies_fn: None,
            },
            declaration_list_emit_mode: DeclarationListEmitMode::Closure,
            styles,
            external_styles: None,
            encapsulation,
            animations: None,
            view_providers: None,
            relative_context_file_path: "".to_string(),
            i18n_use_external_ids: false,
            change_detection,
            relative_template_path: None,
            has_directive_dependencies: false,
            raw_imports: None,
        })
    }
}

impl<TExpression: AstNode> PartialLinker<TExpression> for PartialComponentLinker2 {
    fn link_partial_declaration(
        &self,
        constant_pool: &mut ConstantPool,
        meta_obj: &AstObject<TExpression>,
        source_url: &str,
        _version: &str,
        _target_name: Option<&str>,
        imports: Option<&std::collections::HashMap<String, String>>,
        _class_metadata: Option<&AstObject<TExpression>>,
    ) -> o::Expression {
        // TODO: Use source_url to resolve template if needed
        // println!("[LINKER] link_partial_declaration called, source_url: {}", source_url);
        match self.to_r3_component_metadata(meta_obj, source_url, _target_name, imports) {
            Ok(meta) => {
                let _parser = angular_compiler::expression_parser::parser::Parser::new();
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
                let _schema_registry = DummySchemaRegistry;

                let mut binding_parser =
                    angular_compiler::template_parser::binding_parser::BindingParser::new(
                        &_parser,
                        &_schema_registry,
                        vec![],
                    );

                let res =
                    compile_component_from_metadata(&meta, constant_pool, &mut binding_parser);

                if res.statements.is_empty() {
                    res.expression
                } else {
                    let mut stmts = res.statements;
                    stmts.push(o::Statement::Return(o::ReturnStatement {
                        value: Box::new(res.expression),
                        source_span: None,
                    }));

                    o::Expression::InvokeFn(o::InvokeFunctionExpr {
                        fn_: Box::new(o::Expression::ArrowFn(o::ArrowFunctionExpr {
                            params: vec![],
                            body: o::ArrowFunctionBody::Statements(stmts),
                            type_: None,
                            source_span: None,
                        })),
                        args: vec![],
                        type_: None,
                        source_span: None,
                        pure: false,
                    })
                }
            }
            Err(e) => {
                // Return error expression or panic?
                // For now, simple error literal
                o::Expression::Literal(o::LiteralExpr {
                    value: o::LiteralValue::String(format!("Error: {}", e)),
                    type_: None,
                    source_span: None,
                })
            }
        }
    }
}
