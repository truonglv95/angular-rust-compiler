//! Render3 View Compiler
//!
//! Corresponds to packages/compiler/src/render3/view/compiler.ts
//! Contains directive and component compilation logic

use indexmap::IndexMap;
use std::collections::HashMap;

use crate::constant_pool::ConstantPool;
use crate::core::{ChangeDetectionStrategy, ViewEncapsulation};
use crate::directive_matching::CssSelector;
use crate::output::output_ast::{
    ArrowFunctionBody, ArrowFunctionExpr, DynamicImportExpr, Expression, ExternalExpr, FnParam,
    FunctionExpr, InvokeFunctionExpr, LiteralArrayExpr, LiteralExpr, LiteralMapEntry,
    LiteralMapExpr, LiteralValue, ReadPropExpr, ReadVarExpr, ReturnStatement, Statement, Type,
};
use crate::parse_util::{ParseError, ParseSourceSpan};
use crate::render3::r3_identifiers::Identifiers as R3;
use crate::render3::util::{type_with_parameters, R3CompiledExpression};
use crate::shadow_css::ShadowCss;
use crate::template::pipeline::src::emit::emit_host_binding_function;
use crate::template::pipeline::src::ingest::{ingest_host_binding, HostBindingInput};
use crate::template::pipeline::src::phases::run_host;
use crate::template_parser::binding_parser::{BindingParser, ParsedEvent, ParsedProperty};

use super::api::{
    DeclarationListEmitMode, R3ComponentMetadata, R3DeferResolverFunctionMetadata,
    R3DirectiveMetadata, R3TemplateDependencyMetadata,
};
use super::query_generation::{create_content_queries_function, create_view_queries_function};
use super::template::make_binding_parser;
use super::util::{
    conditionally_create_directive_binding_literal, DefinitionMap, InputBindingValue,
};

const COMPONENT_VARIABLE: &str = "%COMP%";
const HOST_ATTR: &str = "_nghost-%COMP%";
const CONTENT_ATTR: &str = "_ngcontent-%COMP%";

/// Helper to create literal expression
fn literal(value: LiteralValue) -> Expression {
    Expression::Literal(LiteralExpr {
        value,
        type_: None,
        source_span: None,
    })
}

/// Helper to create external expression
fn external_expr(reference: crate::output::output_ast::ExternalReference) -> Expression {
    Expression::External(ExternalExpr {
        value: reference,
        type_: None,
        source_span: None,
    })
}

/// Compile a directive for the render3 runtime.
pub fn compile_directive_from_metadata(
    meta: &R3DirectiveMetadata,
    constant_pool: &mut ConstantPool,
    binding_parser: &mut BindingParser,
) -> R3CompiledExpression {
    let mut definition_map = base_directive_fields(meta, constant_pool, binding_parser);
    add_features(&mut definition_map, meta, None);

    let expression = Expression::InvokeFn(InvokeFunctionExpr {
        fn_: Box::new(external_expr(R3::define_directive())),
        args: vec![Expression::LiteralMap(definition_map.to_literal_map())],
        type_: None,
        source_span: None,
        pure: true,
    });

    let type_ = create_directive_type(meta);

    R3CompiledExpression::new(expression, type_, vec![])
}

/// Compile a component for the render3 runtime.
pub fn compile_component_from_metadata(
    meta: &R3ComponentMetadata,
    constant_pool: &mut ConstantPool,
    binding_parser: &mut BindingParser,
) -> R3CompiledExpression {
    eprintln!(
        "[COMPILER_DEBUG] compile_component_from_metadata: {}",
        meta.directive.name
    );
    // eprintln!("DEBUG: compile_component_from_metadata called for {}, inputs len: {}", meta.directive.name, meta.directive.inputs.len());
    // 1. Ingest
    let mut job = crate::template::pipeline::src::ingest::ingest_component(
        meta.directive.name.clone(),
        meta.template.nodes.clone(),
        constant_pool.clone(),
        crate::template::pipeline::src::compilation::TemplateCompilationMode::Full,
        meta.relative_context_file_path.clone(),
        meta.i18n_use_external_ids,
        meta.defer.clone(),
        None, // all_deferrable_deps_fn
        meta.relative_template_path.clone(),
        false, // enable_debug_locations
        meta.change_detection.as_ref().and_then(|cd| match cd {
            super::api::ChangeDetectionOrExpression::Strategy(s) => Some(*s),
            _ => None,
        }),
        meta.declarations.clone(),
    );

    // 2. Run phases
    crate::template::pipeline::src::phases::run(&mut job);
    // NOTE: We do NOT copy job.pool back to constant_pool here because
    // emit_component already includes job.pool.statements in its output.
    // Copying would cause duplicate statements in the final output.

    // 3. Handle Host Bindings if present
    let host_job = if !meta.directive.host.attributes.is_empty()
        || !meta.directive.host.listeners.is_empty()
        || !meta.directive.host.properties.is_empty()
        || meta.directive.host.special_attributes.class_attr.is_some()
        || meta.directive.host.special_attributes.style_attr.is_some()
    {
        let mut attributes = meta.directive.host.attributes.clone();
        if let Some(class_attr) = &meta.directive.host.special_attributes.class_attr {
            attributes.insert(
                "class".to_string(),
                Expression::Literal(LiteralExpr {
                    value: LiteralValue::String(class_attr.clone()),
                    type_: None,
                    source_span: None,
                }),
            );
        }
        if let Some(style_attr) = &meta.directive.host.special_attributes.style_attr {
            attributes.insert(
                "style".to_string(),
                Expression::Literal(LiteralExpr {
                    value: LiteralValue::String(style_attr.clone()),
                    type_: None,
                    source_span: None,
                }),
            );
        }

        let host_input = crate::template::pipeline::src::ingest::HostBindingInput {
            component_name: meta.directive.name.clone(),
            component_selector: meta.directive.selector.clone().unwrap_or_default(),
            properties: meta.directive.host.properties.clone(),
            attributes,
            events: meta.directive.host.listeners.clone(),
        };

        let mut host_job_instance = crate::template::pipeline::src::ingest::ingest_host_binding(
            host_input,
            (*constant_pool).clone(),
        );
        crate::template::pipeline::src::phases::run_host(&mut host_job_instance);
        *constant_pool = host_job_instance.pool.clone();
        Some(host_job_instance)
    } else {
        None
    };

    // 4. Emit
    crate::template::pipeline::src::emit::emit_component(&job, meta, host_job.as_ref())
}

/// Helper to create R3 selector array from CssSelector
fn create_selector_array(selector: &CssSelector) -> Expression {
    let mut entries = vec![];

    // Element
    entries.push(literal(LiteralValue::String(
        selector.element.clone().unwrap_or_default(),
    )));

    // Attributes (including IDs and classes stored as attributes if they are)
    // Note: CssSelector stores IDs in attrs if parsed from #id, and classes in class_names.
    // In R3 selector arrays, attributes are name/value pairs.
    for i in (0..selector.attrs.len()).step_by(2) {
        entries.push(literal(LiteralValue::String(selector.attrs[i].clone())));
        entries.push(literal(LiteralValue::String(selector.attrs[i + 1].clone())));
    }

    // Classes
    for class_name in &selector.class_names {
        // AttributeMarker::Classes = 1
        entries.push(literal(LiteralValue::Number(1.0)));
        entries.push(literal(LiteralValue::String(class_name.clone())));
    }

    Expression::LiteralArray(LiteralArrayExpr {
        entries,
        type_: None,
        source_span: None,
    })
}

fn base_directive_fields(
    meta: &R3DirectiveMetadata,
    constant_pool: &mut ConstantPool,
    binding_parser: &mut BindingParser,
) -> DefinitionMap {
    let mut definition_map = DefinitionMap::new();

    // type
    definition_map.set("type", Some(meta.type_.value.clone()));

    // selectors
    if let Some(ref selector_str) = meta.selector {
        if !selector_str.is_empty() {
            if let Ok(selectors) = CssSelector::parse(selector_str) {
                let selector_arr = Expression::LiteralArray(LiteralArrayExpr {
                    entries: selectors.iter().map(|s| create_selector_array(s)).collect(),
                    type_: None,
                    source_span: None,
                });
                definition_map.set("selectors", Some(selector_arr));
            }
        }
    }

    // content queries
    if !meta.queries.is_empty() {
        definition_map.set(
            "contentQueries",
            Some(create_content_queries_function(
                &meta.queries,
                constant_pool,
                Some(&meta.name),
            )),
        );
    }

    // view queries
    if !meta.view_queries.is_empty() {
        definition_map.set(
            "viewQuery",
            Some(create_view_queries_function(
                &meta.view_queries,
                constant_pool,
                Some(&meta.name),
            )),
        );
    }

    // host bindings
    if let Some(host_bindings_fn) = create_host_bindings_function(
        &meta.host,
        &meta.type_source_span,
        binding_parser,
        constant_pool,
        meta.selector.as_deref().unwrap_or(""),
        &meta.name,
        &mut definition_map,
    ) {
        definition_map.set("hostBindings", Some(host_bindings_fn));
    }

    // inputs
    let mut inputs_vec: Vec<(String, InputBindingValue)> = meta
        .inputs
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                InputBindingValue::Complex(super::util::InputBindingMetadata {
                    class_property_name: v.class_property_name.clone(),
                    binding_property_name: v.binding_property_name.clone(),
                    transform_function: v.transform_function.clone(),
                    is_signal: v.is_signal,
                }),
            )
        })
        .collect();
    // Sort by key to ensure deterministic order
    inputs_vec.sort_by(|a, b| a.0.cmp(&b.0));

    let inputs_map: IndexMap<String, InputBindingValue> = inputs_vec.into_iter().collect();

    if let Some(inputs_expr) = conditionally_create_directive_binding_literal(&inputs_map, true) {
        definition_map.set("inputs", Some(Expression::LiteralMap(inputs_expr)));
    }

    // outputs
    let mut outputs_vec: Vec<(String, InputBindingValue)> = meta
        .outputs
        .iter()
        .map(|(k, v)| (k.clone(), InputBindingValue::Simple(v.clone())))
        .collect();
    // Sort by key
    outputs_vec.sort_by(|a, b| a.0.cmp(&b.0));

    let outputs_map: IndexMap<String, InputBindingValue> = outputs_vec.into_iter().collect();

    if let Some(outputs_expr) = conditionally_create_directive_binding_literal(&outputs_map, false)
    {
        definition_map.set("outputs", Some(Expression::LiteralMap(outputs_expr)));
    }

    // exportAs
    if let Some(ref export_as) = meta.export_as {
        let export_exprs: Vec<Expression> = export_as
            .iter()
            .map(|e| literal(LiteralValue::String(e.clone())))
            .collect();
        definition_map.set(
            "exportAs",
            Some(Expression::LiteralArray(LiteralArrayExpr {
                entries: export_exprs,
                type_: None,
                source_span: None,
            })),
        );
    }

    // standalone
    if meta.is_standalone {
        definition_map.set("standalone", Some(literal(LiteralValue::Bool(true))));
    } else {
        definition_map.set("standalone", Some(literal(LiteralValue::Bool(false))));
    }

    // signals
    if meta.is_signal {
        definition_map.set("signals", Some(literal(LiteralValue::Bool(true))));
    }

    definition_map
}

fn add_features(
    definition_map: &mut DefinitionMap,
    meta: &R3DirectiveMetadata,
    component_meta: Option<&R3ComponentMetadata>,
) {
    let mut features: Vec<Expression> = vec![];

    // Providers feature
    let providers = &meta.providers;
    let view_providers = component_meta.and_then(|c| c.view_providers.as_ref());

    if providers.is_some() || view_providers.is_some() {
        let mut args = vec![providers.clone().unwrap_or_else(|| {
            Expression::LiteralArray(LiteralArrayExpr {
                entries: vec![],
                type_: None,
                source_span: None,
            })
        })];
        if let Some(vp) = view_providers {
            args.push(vp.clone());
        }
        features.push(Expression::InvokeFn(InvokeFunctionExpr {
            fn_: Box::new(external_expr(R3::providers_feature())),
            args,
            type_: None,
            source_span: None,
            pure: false,
        }));
    }

    // Host directives feature
    if let Some(ref host_directives) = meta.host_directives {
        if !host_directives.is_empty() {
            let arg = create_host_directives_feature_arg(host_directives);
            features.push(Expression::InvokeFn(InvokeFunctionExpr {
                fn_: Box::new(external_expr(R3::host_directives_feature())),
                args: vec![arg],
                type_: None,
                source_span: None,
                pure: false,
            }));
        }
    }

    // Inheritance feature
    if meta.uses_inheritance {
        features.push(external_expr(R3::inherit_definition_feature()));
    }

    // OnChanges feature
    if meta.lifecycle.uses_on_changes {
        features.push(external_expr(R3::ng_on_changes_feature()));
    }

    // External styles feature
    if let Some(component) = component_meta {
        if let Some(ref external_styles) = component.external_styles {
            if !external_styles.is_empty() {
                let style_nodes: Vec<Expression> = external_styles
                    .iter()
                    .map(|s| literal(LiteralValue::String(s.clone())))
                    .collect();
                features.push(Expression::InvokeFn(InvokeFunctionExpr {
                    fn_: Box::new(external_expr(R3::external_styles_feature())),
                    args: vec![Expression::LiteralArray(LiteralArrayExpr {
                        entries: style_nodes,
                        type_: None,
                        source_span: None,
                    })],
                    type_: None,
                    source_span: None,
                    pure: false,
                }));
            }
        }
    }

    if !features.is_empty() {
        definition_map.set(
            "features",
            Some(Expression::LiteralArray(LiteralArrayExpr {
                entries: features,
                type_: None,
                source_span: None,
            })),
        );
    }
}

fn create_host_directives_feature_arg(
    host_directives: &[super::api::R3HostDirectiveMetadata],
) -> Expression {
    let mut expressions: Vec<Expression> = vec![];
    let mut has_forward_ref = false;

    for current in host_directives {
        if current.inputs.is_none() && current.outputs.is_none() {
            expressions.push(current.directive.type_expr.clone());
        } else {
            let mut keys = vec![LiteralMapEntry {
                key: "directive".to_string(),
                value: Box::new(current.directive.type_expr.clone()),
                quoted: false,
            }];

            if let Some(ref inputs) = current.inputs {
                if let Some(inputs_arr) = create_host_directives_mapping_array(inputs) {
                    keys.push(LiteralMapEntry {
                        key: "inputs".to_string(),
                        value: Box::new(Expression::LiteralArray(inputs_arr)),
                        quoted: false,
                    });
                }
            }

            if let Some(ref outputs) = current.outputs {
                if let Some(outputs_arr) = create_host_directives_mapping_array(outputs) {
                    keys.push(LiteralMapEntry {
                        key: "outputs".to_string(),
                        value: Box::new(Expression::LiteralArray(outputs_arr)),
                        quoted: false,
                    });
                }
            }

            expressions.push(Expression::LiteralMap(LiteralMapExpr {
                entries: keys,
                type_: None,
                source_span: None,
            }));
        }

        if current.is_forward_reference {
            has_forward_ref = true;
        }
    }

    if has_forward_ref {
        Expression::Fn(FunctionExpr {
            params: vec![],
            statements: vec![Statement::Return(ReturnStatement {
                value: Box::new(Expression::LiteralArray(LiteralArrayExpr {
                    entries: expressions,
                    type_: None,
                    source_span: None,
                })),
                source_span: None,
            })],
            type_: None,
            source_span: None,
            name: None,
        })
    } else {
        Expression::LiteralArray(LiteralArrayExpr {
            entries: expressions,
            type_: None,
            source_span: None,
        })
    }
}

/// Creates a mapping array from input/output mapping.
pub fn create_host_directives_mapping_array(
    mapping: &HashMap<String, String>,
) -> Option<LiteralArrayExpr> {
    let mut elements: Vec<Expression> = vec![];

    for (public_name, aliased_name) in mapping {
        elements.push(literal(LiteralValue::String(public_name.clone())));
        elements.push(literal(LiteralValue::String(aliased_name.clone())));
    }

    if elements.is_empty() {
        None
    } else {
        Some(LiteralArrayExpr {
            entries: elements,
            type_: None,
            source_span: None,
        })
    }
}

/// Creates the type specification for a directive.
pub fn create_directive_type(meta: &R3DirectiveMetadata) -> Type {
    let type_params = create_base_directive_type_params(meta);
    // TODO: Add remaining type params
    Type::Expression(crate::output::output_ast::ExpressionType {
        value: Box::new(external_expr(R3::directive_declaration())),
        modifiers: crate::output::output_ast::TypeModifier::None,
        type_params: Some(type_params),
    })
}

/// Creates the type specification for a component.
pub fn create_component_type(meta: &R3ComponentMetadata) -> Type {
    let type_params = create_base_directive_type_params(&meta.directive);
    // TODO: Add remaining type params
    Type::Expression(crate::output::output_ast::ExpressionType {
        value: Box::new(external_expr(R3::component_declaration())),
        modifiers: crate::output::output_ast::TypeModifier::None,
        type_params: Some(type_params),
    })
}

fn create_base_directive_type_params(meta: &R3DirectiveMetadata) -> Vec<Type> {
    let selector_for_type = meta.selector.as_ref().map(|s| s.replace('\n', ""));

    vec![
        type_with_parameters(meta.type_.type_expr.clone(), meta.type_argument_count),
        selector_for_type.map_or(
            crate::output::output_ast::none_type(),
            |_| crate::output::output_ast::string_type(), // TODO: Create literal type from string
        ),
        // TODO: Add remaining type params
    ]
}

fn compile_declaration_list(list: Expression, mode: DeclarationListEmitMode) -> Expression {
    match mode {
        DeclarationListEmitMode::Direct => list,
        DeclarationListEmitMode::Closure => Expression::ArrowFn(ArrowFunctionExpr {
            params: vec![],
            body: ArrowFunctionBody::Expression(Box::new(list)),
            type_: None,
            source_span: None,
        }),
        DeclarationListEmitMode::ClosureResolved => {
            // list.prop('map').callFn([o.importExpr(R3.resolveForwardRef)])
            let resolved_list = Expression::InvokeFn(InvokeFunctionExpr {
                fn_: Box::new(Expression::ReadProp(ReadPropExpr {
                    receiver: Box::new(list),
                    name: "map".to_string(),
                    type_: None,
                    source_span: None,
                })),
                args: vec![external_expr(R3::resolve_forward_ref())],
                type_: None,
                source_span: None,
                pure: false,
            });
            Expression::ArrowFn(ArrowFunctionExpr {
                params: vec![],
                body: ArrowFunctionBody::Expression(Box::new(resolved_list)),
                type_: None,
                source_span: None,
            })
        }
        DeclarationListEmitMode::RuntimeResolved => {
            panic!("Unsupported with an array of pre-resolved dependencies")
        }
    }
}

pub fn compile_styles(styles: &[String], selector: &str, host_selector: &str) -> Vec<String> {
    let shadow_css = ShadowCss::new();
    styles
        .iter()
        .map(|style| shadow_css.shim_css_text(style, selector, host_selector))
        .collect()
}

/// Encapsulates a CSS stylesheet with emulated view encapsulation.
pub fn encapsulate_style(style: &str, component_identifier: Option<&str>) -> String {
    let shadow_css = ShadowCss::new();
    let selector = component_identifier
        .map(|id| CONTENT_ATTR.replace(COMPONENT_VARIABLE, id))
        .unwrap_or_else(|| CONTENT_ATTR.to_string());
    let host_selector = component_identifier
        .map(|id| HOST_ATTR.replace(COMPONENT_VARIABLE, id))
        .unwrap_or_else(|| HOST_ATTR.to_string());
    shadow_css.shim_css_text(style, &selector, &host_selector)
}

/// Host bindings structure.
#[derive(Debug, Clone, Default)]
pub struct ParsedHostBindings {
    pub attributes: HashMap<String, Expression>,
    pub listeners: HashMap<String, String>,
    pub properties: HashMap<String, String>,
    pub special_attributes: HostSpecialAttributes,
}

#[derive(Debug, Clone, Default)]
pub struct HostSpecialAttributes {
    pub style_attr: Option<String>,
    pub class_attr: Option<String>,
}

/// Parse host bindings from a host object.
pub fn parse_host_bindings(host: &HashMap<String, String>) -> ParsedHostBindings {
    let mut attributes: HashMap<String, Expression> = HashMap::new();
    let mut listeners: HashMap<String, String> = HashMap::new();
    let mut properties: HashMap<String, String> = HashMap::new();
    let mut special_attributes = HostSpecialAttributes::default();

    let host_regex = regex::Regex::new(r"^(?:\[([^\]]+)\])|(?:\(([^\)]+)\))$").unwrap();

    for (key, value) in host {
        if let Some(caps) = host_regex.captures(key) {
            if let Some(binding) = caps.get(1) {
                properties.insert(binding.as_str().to_string(), value.clone());
            } else if let Some(event) = caps.get(2) {
                listeners.insert(event.as_str().to_string(), value.clone());
            }
        } else {
            match key.as_str() {
                "class" => {
                    special_attributes.class_attr = Some(value.clone());
                }
                "style" => {
                    special_attributes.style_attr = Some(value.clone());
                }
                _ => {
                    attributes.insert(key.clone(), literal(LiteralValue::String(value.clone())));
                }
            }
        }
    }

    ParsedHostBindings {
        attributes,
        listeners,
        properties,
        special_attributes,
    }
}

/// Verify host bindings and return errors.
pub fn verify_host_bindings(
    bindings: &ParsedHostBindings,
    source_span: &ParseSourceSpan,
) -> Vec<ParseError> {
    let mut binding_parser = make_binding_parser(false);
    binding_parser.create_directive_host_event_asts(&bindings.listeners, source_span);
    binding_parser.create_bound_host_properties(&bindings.properties, source_span);
    binding_parser.errors.clone()
}

/// Create a host bindings function or return None if one is not necessary.
/// Corresponds to createHostBindingsFunction in TypeScript compiler.ts
fn create_host_bindings_function(
    host_bindings_metadata: &super::api::R3HostMetadata,
    type_source_span: &ParseSourceSpan,
    binding_parser: &mut BindingParser,
    constant_pool: &mut ConstantPool,
    selector: &str,
    name: &str,
    definition_map: &mut DefinitionMap,
) -> Option<Expression> {
    // The parser for host bindings treats class and style attributes specially -- they are
    // extracted into these separate fields. This is not the case for templates, so the compiler can
    // actually already handle these special attributes internally. Therefore, we just drop them
    // into the attributes map.
    let mut attributes = host_bindings_metadata.attributes.clone();
    if let Some(ref style_attr) = host_bindings_metadata.special_attributes.style_attr {
        attributes.insert(
            "style".to_string(),
            literal(LiteralValue::String(style_attr.clone())),
        );
    }
    if let Some(ref class_attr) = host_bindings_metadata.special_attributes.class_attr {
        attributes.insert(
            "class".to_string(),
            literal(LiteralValue::String(class_attr.clone())),
        );
    }

    // Create host job
    let mut host_job = ingest_host_binding(
        HostBindingInput {
            component_name: name.to_string(),
            component_selector: selector.to_string(),
            properties: host_bindings_metadata.properties.clone(),
            attributes,
            events: host_bindings_metadata.listeners.clone(),
        },
        constant_pool.clone(), // ingest_host_binding takes ownership, we clone to preserve original
    );

    // Transform the host job through the pipeline phases
    run_host(&mut host_job);

    // Set hostAttrs in definition map
    if let Some(ref attrs) = host_job.root.attributes {
        definition_map.set("hostAttrs", Some(attrs.clone()));
    }

    // Set hostVars in definition map - CRITICAL for class/style bindings
    let host_vars = host_job.root.vars.unwrap_or(0);
    if host_vars > 0 {
        definition_map.set(
            "hostVars",
            Some(Expression::Literal(LiteralExpr {
                value: LiteralValue::Number(host_vars as f64),
                type_: None,
                source_span: None,
            })),
        );
    }

    // Emit the host binding function
    emit_host_binding_function(&host_job)
}

/// Compiles the dependency resolver function for a defer block.
pub fn compile_defer_resolver_function(meta: &R3DeferResolverFunctionMetadata) -> Expression {
    let mut dep_expressions: Vec<Expression> = vec![];

    match meta {
        R3DeferResolverFunctionMetadata::PerBlock { dependencies } => {
            for dep in dependencies {
                if dep.is_deferrable {
                    // Dynamic import with callback
                    let inner_fn = Expression::ArrowFn(ArrowFunctionExpr {
                        params: vec![FnParam {
                            name: "m".to_string(),
                            type_: Some(crate::output::output_ast::dynamic_type()),
                        }],
                        body: ArrowFunctionBody::Expression(Box::new(Expression::ReadProp(
                            ReadPropExpr {
                                receiver: Box::new(Expression::ReadVar(ReadVarExpr {
                                    name: "m".to_string(),
                                    type_: None,
                                    source_span: None,
                                })),
                                name: if dep.is_default_import {
                                    "default".to_string()
                                } else {
                                    dep.symbol_name.clone()
                                },
                                type_: None,
                                source_span: None,
                            },
                        ))),
                        type_: None,
                        source_span: None,
                    });

                    if let Some(ref import_path) = dep.import_path {
                        let dynamic_import = Expression::DynamicImport(DynamicImportExpr {
                            url: import_path.clone(),
                            source_span: None,
                        });
                        let then_call = Expression::InvokeFn(InvokeFunctionExpr {
                            fn_: Box::new(Expression::ReadProp(ReadPropExpr {
                                receiver: Box::new(dynamic_import),
                                name: "then".to_string(),
                                type_: None,
                                source_span: None,
                            })),
                            args: vec![inner_fn],
                            type_: None,
                            source_span: None,
                            pure: false,
                        });
                        dep_expressions.push(then_call);
                    }
                } else {
                    dep_expressions.push(dep.type_reference.clone());
                }
            }
        }
        R3DeferResolverFunctionMetadata::PerComponent { dependencies } => {
            for dep in dependencies {
                let inner_fn = Expression::ArrowFn(ArrowFunctionExpr {
                    params: vec![FnParam {
                        name: "m".to_string(),
                        type_: Some(crate::output::output_ast::dynamic_type()),
                    }],
                    body: ArrowFunctionBody::Expression(Box::new(Expression::ReadProp(
                        ReadPropExpr {
                            receiver: Box::new(Expression::ReadVar(ReadVarExpr {
                                name: "m".to_string(),
                                type_: None,
                                source_span: None,
                            })),
                            name: if dep.is_default_import {
                                "default".to_string()
                            } else {
                                dep.symbol_name.clone()
                            },
                            type_: None,
                            source_span: None,
                        },
                    ))),
                    type_: None,
                    source_span: None,
                });

                let dynamic_import = Expression::DynamicImport(DynamicImportExpr {
                    url: dep.import_path.clone(),
                    source_span: None,
                });
                let then_call = Expression::InvokeFn(InvokeFunctionExpr {
                    fn_: Box::new(Expression::ReadProp(ReadPropExpr {
                        receiver: Box::new(dynamic_import),
                        name: "then".to_string(),
                        type_: None,
                        source_span: None,
                    })),
                    args: vec![inner_fn],
                    type_: None,
                    source_span: None,
                    pure: false,
                });
                dep_expressions.push(then_call);
            }
        }
    }

    Expression::ArrowFn(ArrowFunctionExpr {
        params: vec![],
        body: ArrowFunctionBody::Expression(Box::new(Expression::LiteralArray(LiteralArrayExpr {
            entries: dep_expressions,
            type_: None,
            source_span: None,
        }))),
        type_: None,
        source_span: None,
    })
}
