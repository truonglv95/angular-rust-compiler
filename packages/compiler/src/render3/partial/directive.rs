//! Render3 Partial Directive Compilation
//!
//! Corresponds to packages/compiler/src/render3/partial/directive.ts
//! Contains directive declaration compilation for partial/linking mode

use indexmap::IndexMap;

use crate::output::output_ast::{
    Expression, LiteralExpr, LiteralValue, LiteralArrayExpr, LiteralMapExpr, LiteralMapEntry,
    ExternalExpr, InvokeFunctionExpr,
};
use crate::render3::r3_identifiers::Identifiers as R3;
use crate::render3::util::{R3CompiledExpression, convert_from_maybe_forward_ref_expression, generate_forward_ref};
use crate::render3::view::api::{R3DirectiveMetadata, R3HostMetadata, R3QueryMetadata};
use crate::render3::view::util::{DefinitionMap, UNSAFE_OBJECT_KEY_NAME_REGEXP, as_literal_string};

/// Helper to create literal expression
fn literal(value: LiteralValue) -> Expression {
    Expression::Literal(LiteralExpr {
        value,
        type_: None,
        source_span: None,
    })
}

/// Helper to create external expression from ExternalReference
fn external_expr(reference: crate::output::output_ast::ExternalReference) -> Expression {
    Expression::External(ExternalExpr {
        value: reference,
        type_: None,
        source_span: None,
    })
}

/// Compile a directive declaration defined by the `R3DirectiveMetadata`.
pub fn compile_declare_directive_from_metadata(
    meta: &R3DirectiveMetadata,
) -> R3CompiledExpression {
    let definition_map = create_directive_definition_map(meta);

    let declare_directive_ref = R3::declare_directive();
    let declare_directive_expr = external_expr(declare_directive_ref);
    
    let expression = Expression::InvokeFn(InvokeFunctionExpr {
        fn_: Box::new(declare_directive_expr),
        args: vec![Expression::LiteralMap(definition_map.to_literal_map())],
        type_: None,
        source_span: None,
        pure: false,
    });

    // TODO: implement create_directive_type
    let type_ = crate::output::output_ast::dynamic_type();

    R3CompiledExpression::new(expression, type_, vec![])
}

/// Gathers the declaration fields for a directive into a `DefinitionMap`.
pub fn create_directive_definition_map(meta: &R3DirectiveMetadata) -> DefinitionMap {
    let mut definition_map = DefinitionMap::new();
    let min_version = get_minimum_version_for_partial_output(meta);

    definition_map.set("minVersion", Some(literal(LiteralValue::String(min_version))));
    definition_map.set("version", Some(literal(LiteralValue::String("0.0.0-PLACEHOLDER".to_string()))));

    // e.g. `type: MyDirective`
    definition_map.set("type", Some(meta.type_.value.clone()));

    if meta.is_standalone {
        definition_map.set("isStandalone", Some(literal(LiteralValue::Bool(true))));
    }

    if meta.is_signal {
        definition_map.set("isSignal", Some(literal(LiteralValue::Bool(true))));
    }

    // e.g. `selector: 'some-dir'`
    if let Some(ref selector) = meta.selector {
        definition_map.set("selector", Some(literal(LiteralValue::String(selector.clone()))));
    }

    // inputs
    if needs_new_input_partial_output(meta) {
        if let Some(inputs_expr) = create_inputs_partial_metadata(&meta.inputs) {
            definition_map.set("inputs", Some(inputs_expr));
        }
    } else {
        if let Some(inputs_expr) = legacy_inputs_partial_metadata(&meta.inputs) {
            definition_map.set("inputs", Some(inputs_expr));
        }
    }

    // outputs
    if !meta.outputs.is_empty() {
        let outputs_entries: Vec<LiteralMapEntry> = meta.outputs
            .iter()
            .map(|(key, value)| LiteralMapEntry {
                key: key.clone(),
                value: Box::new(literal(LiteralValue::String(value.clone()))),
                quoted: false,
            })
            .collect();
        definition_map.set("outputs", Some(Expression::LiteralMap(LiteralMapExpr {
            entries: outputs_entries,
            type_: None,
            source_span: None,
        })));
    }

    // host
    if let Some(host_expr) = compile_host_metadata(&meta.host) {
        definition_map.set("host", Some(host_expr));
    }

    // providers
    if let Some(ref providers) = meta.providers {
        definition_map.set("providers", Some(providers.clone()));
    }

    // queries
    if !meta.queries.is_empty() {
        let queries_exprs: Vec<Expression> = meta.queries.iter().map(compile_query).collect();
        definition_map.set("queries", Some(Expression::LiteralArray(LiteralArrayExpr {
            entries: queries_exprs,
            type_: None,
            source_span: None,
        })));
    }

    // viewQueries
    if !meta.view_queries.is_empty() {
        let view_queries_exprs: Vec<Expression> = meta.view_queries.iter().map(compile_query).collect();
        definition_map.set("viewQueries", Some(Expression::LiteralArray(LiteralArrayExpr {
            entries: view_queries_exprs,
            type_: None,
            source_span: None,
        })));
    }

    // exportAs
    if let Some(ref export_as) = meta.export_as {
        let export_as_exprs: Vec<Expression> = export_as
            .iter()
            .map(|s| literal(LiteralValue::String(s.clone())))
            .collect();
        definition_map.set("exportAs", Some(Expression::LiteralArray(LiteralArrayExpr {
            entries: export_as_exprs,
            type_: None,
            source_span: None,
        })));
    }

    if meta.uses_inheritance {
        definition_map.set("usesInheritance", Some(literal(LiteralValue::Bool(true))));
    }

    if meta.lifecycle.uses_on_changes {
        definition_map.set("usesOnChanges", Some(literal(LiteralValue::Bool(true))));
    }

    // hostDirectives
    if let Some(ref host_directives) = meta.host_directives {
        if !host_directives.is_empty() {
            definition_map.set("hostDirectives", Some(create_host_directives(host_directives)));
        }
    }

    // ngImport
    let core_ref = R3::core();
    let ng_import_expr = external_expr(core_ref);
    definition_map.set("ngImport", Some(ng_import_expr));

    definition_map
}

/// Determines the minimum linker version for the partial output.
fn get_minimum_version_for_partial_output(meta: &R3DirectiveMetadata) -> String {
    let mut min_version = "14.0.0".to_string();

    // Check for decorator transform functions
    let has_decorator_transform_functions = meta.inputs.values().any(|input| input.transform_function.is_some());
    if has_decorator_transform_functions {
        min_version = "16.1.0".to_string();
    }

    // If there are signal inputs
    if needs_new_input_partial_output(meta) {
        min_version = "17.1.0".to_string();
    }

    // If there are signal-based queries
    if meta.queries.iter().any(|q| q.is_signal) || meta.view_queries.iter().any(|q| q.is_signal) {
        min_version = "17.2.0".to_string();
    }

    min_version
}

/// Gets whether the directive needs the new input partial output structure.
fn needs_new_input_partial_output(meta: &R3DirectiveMetadata) -> bool {
    meta.inputs.values().any(|input| input.is_signal)
}

/// Compiles the metadata of a single query.
fn compile_query(query: &R3QueryMetadata) -> Expression {
    let mut meta = DefinitionMap::new();
    
    meta.set("propertyName", Some(literal(LiteralValue::String(query.property_name.clone()))));
    
    if query.first {
        meta.set("first", Some(literal(LiteralValue::Bool(true))));
    }

    // predicate
    match &query.predicate {
        crate::render3::view::api::R3QueryPredicate::Selectors(selectors) => {
            let selector_exprs: Vec<Expression> = selectors
                .iter()
                .map(|s| literal(LiteralValue::String(s.clone())))
                .collect();
            meta.set("predicate", Some(Expression::LiteralArray(LiteralArrayExpr {
                entries: selector_exprs,
                type_: None,
                source_span: None,
            })));
        }
        crate::render3::view::api::R3QueryPredicate::Expression(expr) => {
            meta.set("predicate", Some(convert_from_maybe_forward_ref_expression(expr)));
        }
    }

    if !query.emit_distinct_changes_only {
        meta.set("emitDistinctChangesOnly", Some(literal(LiteralValue::Bool(false))));
    }

    if query.descendants {
        meta.set("descendants", Some(literal(LiteralValue::Bool(true))));
    }

    if let Some(ref read) = query.read {
        meta.set("read", Some(read.clone()));
    }

    if query.static_ {
        meta.set("static", Some(literal(LiteralValue::Bool(true))));
    }

    if query.is_signal {
        meta.set("isSignal", Some(literal(LiteralValue::Bool(true))));
    }

    Expression::LiteralMap(meta.to_literal_map())
}

/// Compiles the host metadata.
fn compile_host_metadata(meta: &R3HostMetadata) -> Option<Expression> {
    let mut host_metadata = DefinitionMap::new();

    // attributes
    if !meta.attributes.is_empty() {
        let attr_entries: Vec<LiteralMapEntry> = meta.attributes
            .iter()
            .map(|(key, value)| LiteralMapEntry {
                key: key.clone(),
                value: Box::new(value.clone()),
                quoted: true,
            })
            .collect();
        host_metadata.set("attributes", Some(Expression::LiteralMap(LiteralMapExpr {
            entries: attr_entries,
            type_: None,
            source_span: None,
        })));
    }

    // listeners
    if !meta.listeners.is_empty() {
        let listener_entries: Vec<LiteralMapEntry> = meta.listeners
            .iter()
            .map(|(key, value)| LiteralMapEntry {
                key: key.clone(),
                value: Box::new(literal(LiteralValue::String(value.clone()))),
                quoted: true,
            })
            .collect();
        host_metadata.set("listeners", Some(Expression::LiteralMap(LiteralMapExpr {
            entries: listener_entries,
            type_: None,
            source_span: None,
        })));
    }

    // properties
    if !meta.properties.is_empty() {
        let prop_entries: Vec<LiteralMapEntry> = meta.properties
            .iter()
            .map(|(key, value)| LiteralMapEntry {
                key: key.clone(),
                value: Box::new(literal(LiteralValue::String(value.clone()))),
                quoted: true,
            })
            .collect();
        host_metadata.set("properties", Some(Expression::LiteralMap(LiteralMapExpr {
            entries: prop_entries,
            type_: None,
            source_span: None,
        })));
    }

    // specialAttributes
    if let Some(ref style_attr) = meta.special_attributes.style_attr {
        host_metadata.set("styleAttribute", Some(literal(LiteralValue::String(style_attr.clone()))));
    }
    if let Some(ref class_attr) = meta.special_attributes.class_attr {
        host_metadata.set("classAttribute", Some(literal(LiteralValue::String(class_attr.clone()))));
    }

    if host_metadata.values.is_empty() {
        None
    } else {
        Some(Expression::LiteralMap(host_metadata.to_literal_map()))
    }
}

/// Creates host directives array.
fn create_host_directives(
    host_directives: &[crate::render3::view::api::R3HostDirectiveMetadata],
) -> Expression {
    let expressions: Vec<Expression> = host_directives
        .iter()
        .map(|current| {
            let mut entries = vec![LiteralMapEntry {
                key: "directive".to_string(),
                value: Box::new(if current.is_forward_reference {
                    generate_forward_ref(current.directive.type_expr.clone())
                } else {
                    current.directive.type_expr.clone()
                }),
                quoted: false,
            }];

            if let Some(ref inputs) = current.inputs {
                let inputs_arr: Vec<Expression> = inputs
                    .iter()
                    .flat_map(|(k, v)| vec![
                        literal(LiteralValue::String(k.clone())),
                        literal(LiteralValue::String(v.clone())),
                    ])
                    .collect();
                entries.push(LiteralMapEntry {
                    key: "inputs".to_string(),
                    value: Box::new(Expression::LiteralArray(LiteralArrayExpr {
                        entries: inputs_arr,
                        type_: None,
                        source_span: None,
                    })),
                    quoted: false,
                });
            }

            if let Some(ref outputs) = current.outputs {
                let outputs_arr: Vec<Expression> = outputs
                    .iter()
                    .flat_map(|(k, v)| vec![
                        literal(LiteralValue::String(k.clone())),
                        literal(LiteralValue::String(v.clone())),
                    ])
                    .collect();
                entries.push(LiteralMapEntry {
                    key: "outputs".to_string(),
                    value: Box::new(Expression::LiteralArray(LiteralArrayExpr {
                        entries: outputs_arr,
                        type_: None,
                        source_span: None,
                    })),
                    quoted: false,
                });
            }

            Expression::LiteralMap(LiteralMapExpr {
                entries,
                type_: None,
                source_span: None,
            })
        })
        .collect();

    Expression::LiteralArray(LiteralArrayExpr {
        entries: expressions,
        type_: None,
        source_span: None,
    })
}

/// Generates partial output metadata for inputs.
fn create_inputs_partial_metadata(
    inputs: &IndexMap<String, crate::render3::view::api::R3InputMetadata>,
) -> Option<Expression> {
    if inputs.is_empty() {
        return None;
    }

    let entries: Vec<LiteralMapEntry> = inputs
        .iter()
        .map(|(declared_name, value)| {
            let inner_entries = vec![
                LiteralMapEntry {
                    key: "classPropertyName".to_string(),
                    value: Box::new(as_literal_string(&value.class_property_name)),
                    quoted: false,
                },
                LiteralMapEntry {
                    key: "publicName".to_string(),
                    value: Box::new(as_literal_string(&value.binding_property_name)),
                    quoted: false,
                },
                LiteralMapEntry {
                    key: "isSignal".to_string(),
                    value: Box::new(literal(LiteralValue::Bool(value.is_signal))),
                    quoted: false,
                },
                LiteralMapEntry {
                    key: "isRequired".to_string(),
                    value: Box::new(literal(LiteralValue::Bool(value.required))),
                    quoted: false,
                },
                LiteralMapEntry {
                    key: "transformFunction".to_string(),
                    value: Box::new(value.transform_function.clone().unwrap_or_else(|| literal(LiteralValue::Null))),
                    quoted: false,
                },
            ];

            LiteralMapEntry {
                key: declared_name.clone(),
                value: Box::new(Expression::LiteralMap(LiteralMapExpr {
                    entries: inner_entries,
                    type_: None,
                    source_span: None,
                })),
                quoted: UNSAFE_OBJECT_KEY_NAME_REGEXP.is_match(declared_name),
            }
        })
        .collect();

    Some(Expression::LiteralMap(LiteralMapExpr {
        entries,
        type_: None,
        source_span: None,
    }))
}

/// Legacy partial output for inputs.
fn legacy_inputs_partial_metadata(
    inputs: &IndexMap<String, crate::render3::view::api::R3InputMetadata>,
) -> Option<Expression> {
    if inputs.is_empty() {
        return None;
    }

    let entries: Vec<LiteralMapEntry> = inputs
        .iter()
        .map(|(declared_name, value)| {
            let public_name = &value.binding_property_name;
            let different_declaring_name = public_name != declared_name;

            let result: Expression = if different_declaring_name || value.transform_function.is_some() {
                let mut values = vec![
                    as_literal_string(public_name),
                    as_literal_string(declared_name),
                ];
                if let Some(ref transform) = value.transform_function {
                    values.push(transform.clone());
                }
                Expression::LiteralArray(LiteralArrayExpr {
                    entries: values,
                    type_: None,
                    source_span: None,
                })
            } else {
                as_literal_string(public_name)
            };

            LiteralMapEntry {
                key: declared_name.clone(),
                value: Box::new(result),
                quoted: UNSAFE_OBJECT_KEY_NAME_REGEXP.is_match(declared_name),
            }
        })
        .collect();

    Some(Expression::LiteralMap(LiteralMapExpr {
        entries,
        type_: None,
        source_span: None,
    }))
}
