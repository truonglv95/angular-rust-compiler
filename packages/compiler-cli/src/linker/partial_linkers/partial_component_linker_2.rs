use crate::linker::ast::AstNode;
use crate::linker::ast_value::{AstObject, AstValue};
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
    ) -> Result<R3ComponentMetadata, String> {
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
            name: type_name_str,
            type_: None,
            source_span: None,
        });

        let type_ref = R3Reference {
            value: wrapped_type.clone(),
            type_expr: wrapped_type,
        };

        let selector = meta_obj.get_string("selector").ok();
        let mut template_str = meta_obj.get_string("template").unwrap_or_default();

        if template_str.is_empty() {
            println!(
                "DEBUG: Checking for templateUrl. has_url={}, keys={:?}",
                meta_obj.has("templateUrl"),
                meta_obj.to_map().keys()
            );
        }

        if template_str.is_empty() && meta_obj.has("templateUrl") {
            let url = meta_obj.get_string("templateUrl")?;
            println!("DEBUG: Found templateUrl: {}", url);
            // Resolve path relative to source_url
            // source_url is likely absolute if from rspack loader
            let source_path = std::path::Path::new(source_url);
            let parent = source_path.parent().unwrap_or(std::path::Path::new("."));
            let template_path = parent.join(url);

            // Read file
            match std::fs::read_to_string(&template_path) {
                Ok(content) => {
                    println!(
                        "DEBUG: Read template from {:?}, content length: {}",
                        template_path,
                        content.len()
                    );
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
            // Assuming it sends an index
            let idx = meta_obj.get_number("encapsulation")? as u32;
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
            let idx = meta_obj.get_number("changeDetection")? as u32;
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
        let dummy_file = angular_compiler::parse_util::ParseSourceFile::new(
            "".to_string(),
            "unknown".to_string(),
        );
        let dummy_span = ParseSourceSpan::new(
            angular_compiler::parse_util::ParseLocation::new(dummy_file.clone(), 0, 0, 0),
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
                        // TODO: handle flags/transform if present in 2nd element
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

        let view_queries = vec![]; // TODO: extract from 'viewQueries' property, similar structure

        // Host
        let host = if meta_obj.has("host") {
            let host_obj = meta_obj.get_object("host")?;
            let mut attributes = HashMap::new();
            let mut listeners = HashMap::new();
            let mut properties = HashMap::new();
            let mut special_attributes =
                angular_compiler::render3::view::api::R3HostSpecialAttributes::default();

            for (key, val) in host_obj.to_map() {
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
                            _ => R3TemplateDependencyKind::Directive,
                        };

                        let type_node = dep_obj.get_value("type")?;
                        let type_expr = o::Expression::ReadVar(o::ReadVarExpr {
                            name: meta_obj.host.print_node(&type_node.node),
                            type_: None,
                            source_span: None,
                        });

                        let selector = dep_obj.get_string("selector").unwrap_or_default();

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

        // Construct R3DirectiveMetadata (subset)
        let directive = R3DirectiveMetadata {
            name: "Component".to_string(), // TODO: extract class name
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
            uses_inheritance: false,
            export_as: None,
            providers: None,
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
            declaration_list_emit_mode: DeclarationListEmitMode::Direct,
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
        target_name: Option<&str>,
    ) -> o::Expression {
        // TODO: Use source_url to resolve template if needed
        match self.to_r3_component_metadata(meta_obj, source_url, target_name) {
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

                let binding_parser =
                    angular_compiler::template_parser::binding_parser::BindingParser::new(
                        &_parser,
                        &_schema_registry,
                        vec![],
                    );

                let res = compile_component_from_metadata(&meta, constant_pool, &binding_parser);
                res.expression
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
