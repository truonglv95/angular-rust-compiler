//! Render3 View Compiler
//!
//! Corresponds to packages/compiler/src/render3/view/compiler.ts
//! Contains directive and component compilation logic

use std::collections::HashMap;
use indexmap::IndexMap;

use crate::constant_pool::ConstantPool;
use crate::core::{ViewEncapsulation, ChangeDetectionStrategy};
use crate::directive_matching::CssSelector;
use crate::output::output_ast::{
    Expression, Type, Statement, LiteralExpr, LiteralValue, LiteralArrayExpr,
    LiteralMapExpr, LiteralMapEntry, ExternalExpr, ArrowFunctionExpr, ArrowFunctionBody,
    FunctionExpr, FnParam, InvokeFunctionExpr, ReadVarExpr, ReadPropExpr,
    ReturnStatement, DynamicImportExpr,
};
use crate::parse_util::{ParseError, ParseSourceSpan};
use crate::render3::r3_identifiers::Identifiers as R3;
use crate::render3::util::{R3CompiledExpression, type_with_parameters};
use crate::shadow_css::ShadowCss;
use crate::template_parser::binding_parser::BindingParser;

use super::api::{
    DeclarationListEmitMode, R3ComponentMetadata,
    R3DeferResolverFunctionMetadata, R3DirectiveMetadata, R3TemplateDependencyMetadata,
};
use super::query_generation::{create_view_queries_function, create_content_queries_function};
use super::template::make_binding_parser;
use super::util::{DefinitionMap, conditionally_create_directive_binding_literal, InputBindingValue};

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
    binding_parser: &BindingParser,
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
    binding_parser: &BindingParser,
) -> R3CompiledExpression {
    let mut definition_map = base_directive_fields(&meta.directive, constant_pool, binding_parser);
    add_features(&mut definition_map, &meta.directive, Some(meta));
    
    // Parse selector
    if let Some(ref selector) = meta.directive.selector {
        if let Ok(selectors) = CssSelector::parse(selector) {
            if let Some(first_selector) = selectors.first() {
                // CssSelector.attrs is Vec<String> with pairs [name, value, name, value, ...]
                if !first_selector.attrs.is_empty() {
                    let attr_exprs: Vec<Expression> = first_selector.attrs
                        .chunks(2)
                        .flat_map(|chunk| {
                            vec![
                                literal(LiteralValue::String(chunk[0].clone())),
                                if chunk.len() > 1 && !chunk[1].is_empty() {
                                    literal(LiteralValue::String(chunk[1].clone()))
                                } else {
                                    literal(LiteralValue::Null)
                                }
                            ]
                        })
                        .collect();
                    // TODO: Fix constant_pool.get_const_literal() type conversion
                    let attr_array = Expression::LiteralArray(LiteralArrayExpr {
                        entries: attr_exprs,
                        type_: None,
                        source_span: None,
                    });
                    definition_map.set("attrs", Some(attr_array));
                }
            }
        }
    }

    // TODO: Implement full template compilation using pipeline
    // For now, set placeholder values
    definition_map.set("decls", Some(literal(LiteralValue::Number(0.0))));
    definition_map.set("vars", Some(literal(LiteralValue::Number(0.0))));
    
    // ng-content selectors
    if !meta.template.ng_content_selectors.is_empty() {
        let selectors: Vec<Expression> = meta.template.ng_content_selectors
            .iter()
            .map(|s| literal(LiteralValue::String(s.clone())))
            .collect();
        definition_map.set("ngContentSelectors", Some(Expression::LiteralArray(LiteralArrayExpr {
            entries: selectors,
            type_: None,
            source_span: None,
        })));
    }

    // declarations/dependencies
    if meta.declaration_list_emit_mode != DeclarationListEmitMode::RuntimeResolved
        && !meta.declarations.is_empty()
    {
        let decl_types: Vec<Expression> = meta.declarations.iter().map(|d| {
            match d {
                R3TemplateDependencyMetadata::Directive(dir) => dir.type_.clone(),
                R3TemplateDependencyMetadata::Pipe(pipe) => pipe.type_.clone(),
                R3TemplateDependencyMetadata::NgModule(ng_module) => ng_module.type_.clone(),
            }
        }).collect();
        let list = Expression::LiteralArray(LiteralArrayExpr {
            entries: decl_types,
            type_: None,
            source_span: None,
        });
        definition_map.set("dependencies", Some(compile_declaration_list(list, meta.declaration_list_emit_mode)));
    } else if meta.declaration_list_emit_mode == DeclarationListEmitMode::RuntimeResolved {
        let mut args = vec![meta.directive.type_.value.clone()];
        if let Some(ref raw_imports) = meta.raw_imports {
            args.push(raw_imports.clone());
        }
        let deps_call = Expression::InvokeFn(InvokeFunctionExpr {
            fn_: Box::new(external_expr(R3::get_component_deps_factory())),
            args,
            type_: None,
            source_span: None,
            pure: false,
        });
        definition_map.set("dependencies", Some(deps_call));
    }

    // Styles
    let mut has_styles = meta.external_styles.as_ref().map_or(false, |s| !s.is_empty());
    if !meta.styles.is_empty() {
        let style_values = if meta.encapsulation == ViewEncapsulation::Emulated {
            compile_styles(&meta.styles, CONTENT_ATTR, HOST_ATTR)
        } else {
            meta.styles.clone()
        };
        
        // TODO: Fix constant_pool.get_const_literal() type conversion
        let style_nodes: Vec<Expression> = style_values
            .iter()
            .filter(|s| !s.trim().is_empty())
            .map(|style| literal(LiteralValue::String(style.clone())))
            .collect();
        
        if !style_nodes.is_empty() {
            has_styles = true;
            definition_map.set("styles", Some(Expression::LiteralArray(LiteralArrayExpr {
                entries: style_nodes,
                type_: None,
                source_span: None,
            })));
        }
    }

    // Encapsulation
    let mut encapsulation = meta.encapsulation;
    if !has_styles && encapsulation == ViewEncapsulation::Emulated {
        encapsulation = ViewEncapsulation::None;
    }
    if encapsulation != ViewEncapsulation::Emulated {
        definition_map.set("encapsulation", Some(literal(LiteralValue::Number(encapsulation as u32 as f64))));
    }

    // Animations
    if let Some(ref animations) = meta.animations {
        let data_map = Expression::LiteralMap(LiteralMapExpr {
            entries: vec![
                LiteralMapEntry {
                    key: "animation".to_string(),
                    value: Box::new(animations.clone()),
                    quoted: false,
                },
            ],
            type_: None,
            source_span: None,
        });
        definition_map.set("data", Some(data_map));
    }

    // Change detection
    if let Some(ref change_detection) = meta.change_detection {
        match change_detection {
            super::api::ChangeDetectionOrExpression::Strategy(strategy) => {
                if *strategy != ChangeDetectionStrategy::Default {
                    definition_map.set("changeDetection", Some(literal(LiteralValue::Number(*strategy as u32 as f64))));
                }
            }
            super::api::ChangeDetectionOrExpression::Expression(expr) => {
                definition_map.set("changeDetection", Some(expr.clone()));
            }
        }
    }

    let expression = Expression::InvokeFn(InvokeFunctionExpr {
        fn_: Box::new(external_expr(R3::define_component())),
        args: vec![Expression::LiteralMap(definition_map.to_literal_map())],
        type_: None,
        source_span: None,
        pure: true,
    });

    let type_ = create_component_type(meta);

    R3CompiledExpression::new(expression, type_, vec![])
}

fn base_directive_fields(
    meta: &R3DirectiveMetadata,
    constant_pool: &mut ConstantPool,
    _binding_parser: &BindingParser,
) -> DefinitionMap {
    let mut definition_map = DefinitionMap::new();
    
    // type
    definition_map.set("type", Some(meta.type_.value.clone()));

    // selectors
    if let Some(ref selector) = meta.selector {
        // Parse selector into R3 format
        // TODO: Implement parse_selector_to_r3_selector in core module
        if !selector.is_empty() {
            // For now, just store the raw selector string in a nested array format
            let selector_arr = Expression::LiteralArray(LiteralArrayExpr {
                entries: vec![Expression::LiteralArray(LiteralArrayExpr {
                    entries: vec![
                        literal(LiteralValue::String("".to_string())),
                        literal(LiteralValue::String(selector.clone())),
                        literal(LiteralValue::String("".to_string())),
                    ],
                    type_: None,
                    source_span: None,
                })],
                type_: None,
                source_span: None,
            });
            definition_map.set("selectors", Some(selector_arr));
        }
    }

    // content queries
    if !meta.queries.is_empty() {
        definition_map.set("contentQueries", Some(
            create_content_queries_function(&meta.queries, constant_pool, Some(&meta.name))
        ));
    }

    // view queries
    if !meta.view_queries.is_empty() {
        definition_map.set("viewQuery", Some(
            create_view_queries_function(&meta.view_queries, constant_pool, Some(&meta.name))
        ));
    }

    // host bindings
    // TODO: Implement createHostBindingsFunction

    // inputs
    let inputs_map: IndexMap<String, InputBindingValue> = meta.inputs
        .iter()
        .map(|(k, v)| {
            (k.clone(), InputBindingValue::Complex(super::util::InputBindingMetadata {
                class_property_name: v.class_property_name.clone(),
                binding_property_name: v.binding_property_name.clone(),
                transform_function: v.transform_function.clone(),
                is_signal: v.is_signal,
            }))
        })
        .collect();
    if let Some(inputs_expr) = conditionally_create_directive_binding_literal(&inputs_map, true) {
        definition_map.set("inputs", Some(Expression::LiteralMap(inputs_expr)));
    }

    // outputs  
    let outputs_map: IndexMap<String, InputBindingValue> = meta.outputs
        .iter()
        .map(|(k, v)| (k.clone(), InputBindingValue::Simple(v.clone())))
        .collect();
    if let Some(outputs_expr) = conditionally_create_directive_binding_literal(&outputs_map, false) {
        definition_map.set("outputs", Some(Expression::LiteralMap(outputs_expr)));
    }

    // exportAs
    if let Some(ref export_as) = meta.export_as {
        let export_exprs: Vec<Expression> = export_as
            .iter()
            .map(|e| literal(LiteralValue::String(e.clone())))
            .collect();
        definition_map.set("exportAs", Some(Expression::LiteralArray(LiteralArrayExpr {
            entries: export_exprs,
            type_: None,
            source_span: None,
        })));
    }

    // standalone
    if !meta.is_standalone {
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
        let mut args = vec![
            providers.clone().unwrap_or_else(|| Expression::LiteralArray(LiteralArrayExpr {
                entries: vec![],
                type_: None,
                source_span: None,
            }))
        ];
        if let Some(vp) = view_providers {
            args.push(vp.clone());
        }
        features.push(
            Expression::InvokeFn(InvokeFunctionExpr {
                fn_: Box::new(external_expr(R3::providers_feature())),
                args,
                type_: None,
                source_span: None,
                pure: false,
            })
        );
    }

    // Host directives feature
    if let Some(ref host_directives) = meta.host_directives {
        if !host_directives.is_empty() {
            let arg = create_host_directives_feature_arg(host_directives);
            features.push(
                Expression::InvokeFn(InvokeFunctionExpr {
                    fn_: Box::new(external_expr(R3::host_directives_feature())),
                    args: vec![arg],
                    type_: None,
                    source_span: None,
                    pure: false,
                })
            );
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
                features.push(
                    Expression::InvokeFn(InvokeFunctionExpr {
                        fn_: Box::new(external_expr(R3::external_styles_feature())),
                        args: vec![Expression::LiteralArray(LiteralArrayExpr {
                            entries: style_nodes,
                            type_: None,
                            source_span: None,
                        })],
                        type_: None,
                        source_span: None,
                        pure: false,
                    })
                );
            }
        }
    }

    if !features.is_empty() {
        definition_map.set("features", Some(Expression::LiteralArray(LiteralArrayExpr {
            entries: features,
            type_: None,
            source_span: None,
        })));
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
            let mut keys = vec![
                LiteralMapEntry {
                    key: "directive".to_string(),
                    value: Box::new(current.directive.type_expr.clone()),
                    quoted: false,
                },
            ];

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
    let selector_for_type = meta.selector.as_ref()
        .map(|s| s.replace('\n', ""));
    
    vec![
        type_with_parameters(meta.type_.type_expr.clone(), meta.type_argument_count),
        selector_for_type.map_or(
            crate::output::output_ast::none_type(),
            |_| crate::output::output_ast::string_type() // TODO: Create literal type from string
        ),
        // TODO: Add remaining type params
    ]
}

fn compile_declaration_list(list: Expression, mode: DeclarationListEmitMode) -> Expression {
    match mode {
        DeclarationListEmitMode::Direct => list,
        DeclarationListEmitMode::Closure => {
            Expression::ArrowFn(ArrowFunctionExpr {
                params: vec![],
                body: ArrowFunctionBody::Expression(Box::new(list)),
                type_: None,
                source_span: None,
            })
        }
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

/// Compiles the dependency resolver function for a defer block.
pub fn compile_defer_resolver_function(
    meta: &R3DeferResolverFunctionMetadata,
) -> Expression {
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
                        body: ArrowFunctionBody::Expression(Box::new(
                            Expression::ReadProp(ReadPropExpr {
                                receiver: Box::new(Expression::ReadVar(ReadVarExpr {
                                    name: "m".to_string(),
                                    type_: None,
                                    source_span: None,
                                })),
                                name: if dep.is_default_import { "default".to_string() } else { dep.symbol_name.clone() },
                                type_: None,
                                source_span: None,
                            })
                        )),
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
                    body: ArrowFunctionBody::Expression(Box::new(
                        Expression::ReadProp(ReadPropExpr {
                            receiver: Box::new(Expression::ReadVar(ReadVarExpr {
                                name: "m".to_string(),
                                type_: None,
                                source_span: None,
                            })),
                            name: if dep.is_default_import { "default".to_string() } else { dep.symbol_name.clone() },
                            type_: None,
                            source_span: None,
                        })
                    )),
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

