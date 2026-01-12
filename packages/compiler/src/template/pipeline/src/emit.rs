//! Emit Module
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/emit.ts

use crate::core::ViewEncapsulation;
use crate::directive_matching::CssSelector;
use crate::output::output_ast as o;
use crate::output::output_ast::Expression;
use crate::render3::r3_identifiers::Identifiers as R3;
use crate::render3::util::R3CompiledExpression;
use crate::render3::view::api::{R3ComponentMetadata, R3TemplateDependencyMetadata};
use crate::render3::view::compiler::compile_styles;
use crate::render3::view::util::{
    conditionally_create_directive_binding_literal, InputBindingValue,
};
use crate::template::pipeline::ir;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob, HostBindingCompilationJob,
};
use crate::template::pipeline::src::instruction as ng;
use indexmap::IndexMap;

/// Helper to create R3 selector array from CssSelector
/// Format: ["button", "mat-button", ""] for button[mat-button]
fn create_selector_array(selector: &CssSelector) -> o::Expression {
    let mut entries = vec![];

    // Element
    entries.push(*o::literal(selector.element.clone().unwrap_or_default()));

    // Attributes (stored in pairs: [name, value, name, value, ...])
    for i in (0..selector.attrs.len()).step_by(2) {
        entries.push(*o::literal(selector.attrs[i].clone()));
        if i + 1 < selector.attrs.len() {
            entries.push(*o::literal(selector.attrs[i + 1].clone()));
        } else {
            entries.push(*o::literal(String::new()));
        }
    }

    // Classes - not needed for R3 selector array format (classes are handled differently)
    // R3 selector array format: [element, attr1, value1, attr2, value2, ...]
    // Classes are not included in this format

    o::Expression::LiteralArray(o::LiteralArrayExpr {
        entries,
        type_: None,
        source_span: None,
    })
}

/// Emits a view unit as a FunctionExpr.
/// Corresponds to emitView in TypeScript emit.ts
fn emit_view(job: &ComponentCompilationJob, unit: &dyn CompilationUnit) -> o::Expression {
    let fn_name = unit.fn_name().unwrap_or("template").to_string();

    let mut body = vec![];

    // Create Block (rf & 1)
    let create_ops: Vec<&dyn ir::Op> = unit
        .create()
        .iter()
        .map(|op| op.as_ref() as &dyn ir::Op)
        .collect();
    if !create_ops.is_empty() {
        let create_stmts = emit_ops(job, create_ops);
        if !create_stmts.is_empty() {
            body.push(o::Statement::IfStmt(o::IfStmt {
                condition: Box::new(o::Expression::BinaryOp(o::BinaryOperatorExpr {
                    operator: o::BinaryOperator::BitwiseAnd,
                    lhs: Box::new(*o::variable("rf")),
                    rhs: Box::new(*o::literal(1.0)),
                    type_: None,
                    source_span: None,
                })),
                true_case: create_stmts,
                false_case: vec![],
                source_span: None,
            }));
        }
    }

    // Update Block (rf & 2)
    let update_ops: Vec<&dyn ir::Op> = unit
        .update()
        .iter()
        .map(|op| op.as_ref() as &dyn ir::Op)
        .collect();
    if !update_ops.is_empty() {
        let update_stmts = emit_ops(job, update_ops);
        if !update_stmts.is_empty() {
            body.push(o::Statement::IfStmt(o::IfStmt {
                condition: Box::new(o::Expression::BinaryOp(o::BinaryOperatorExpr {
                    operator: o::BinaryOperator::BitwiseAnd,
                    lhs: Box::new(*o::variable("rf")),
                    rhs: Box::new(*o::literal(2.0)),
                    type_: None,
                    source_span: None,
                })),
                true_case: update_stmts,
                false_case: vec![],
                source_span: None,
            }));
        }
    }

    // Return FunctionExpr instead of DeclareFunctionStmt
    o::Expression::Fn(o::FunctionExpr {
        name: Some(fn_name),
        params: vec![
            o::FnParam {
                name: "rf".to_string(),
                type_: None,
            },
            o::FnParam {
                name: "ctx".to_string(),
                type_: None,
            },
        ],
        statements: body,
        type_: None,
        source_span: None,
    })
}

/// Emits a component definition from a compilation job.
pub fn emit_component(
    job: &ComponentCompilationJob,
    metadata: &R3ComponentMetadata,
    host_job: Option<&HostBindingCompilationJob>,
) -> R3CompiledExpression {
    let mut statements = vec![];
    statements.extend(job.pool.statements.clone());

    // Emit child views as DeclareFunctionStmt (hoisted to top)
    // Only root template is inlined in defineComponent
    for unit in job.units() {
        // Skip root view - it will be inlined
        if unit.xref() == job.root.xref {
            continue;
        }

        let fn_name = unit
            .fn_name()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "template".to_string());
        let view_fn = emit_view(job, unit);

        // Convert FunctionExpr to DeclareFunctionStmt for child views
        if let o::Expression::Fn(fn_expr) = view_fn {
            statements.push(o::Statement::DeclareFn(o::DeclareFunctionStmt {
                name: fn_name,
                params: fn_expr.params,
                statements: fn_expr.statements,
                type_: fn_expr.type_,
                modifiers: o::StmtModifier::None,
                source_span: fn_expr.source_span,
            }));
        }
    }

    // Emit root template as inline FunctionExpr
    eprintln!(
        "[EMIT_DEBUG] emit_component: job.root create ops: {}, update ops: {}",
        job.root.create().len(),
        job.root.update().len()
    );
    let template_fn = emit_view(job, &job.root);

    // Construct data for defineComponent
    let decls = job.root.decls.unwrap_or(0);
    let vars = job.root.vars.unwrap_or(0);
    let type_expr = metadata.directive.type_.value.clone();

    // Parse selector string into CssSelector and emit as R3 selector array format
    // Format: [["button", "mat-button", ""], ["a", "mat-button", ""]] for "button[mat-button], a[mat-button]"
    let selectors_expr = if let Some(selector_str) = &metadata.directive.selector {
        if !selector_str.is_empty() {
            if let Ok(selectors) = CssSelector::parse(selector_str) {
                // Create array of selector arrays
                let selector_arrays: Vec<o::Expression> =
                    selectors.iter().map(|s| create_selector_array(s)).collect();
                o::Expression::LiteralArray(o::LiteralArrayExpr {
                    entries: selector_arrays,
                    type_: None,
                    source_span: None,
                })
            } else {
                // Fallback: emit as string if parsing fails
                let inner = o::Expression::LiteralArray(o::LiteralArrayExpr {
                    entries: vec![*o::literal(selector_str.clone())],
                    type_: None,
                    source_span: None,
                });
                o::Expression::LiteralArray(o::LiteralArrayExpr {
                    entries: vec![inner],
                    type_: None,
                    source_span: None,
                })
            }
        } else {
            *o::literal(o::LiteralValue::Null)
        }
    } else {
        *o::literal(o::LiteralValue::Null)
    };

    // Extract attrs from first selector (if any) - corresponds to ngtsc compiler.ts line 197-212
    // This is optional and only included if the first selector of a component specifies attributes.
    let attrs_expr = if let Some(selector_str) = &metadata.directive.selector {
        if !selector_str.is_empty() {
            if let Ok(selectors) = CssSelector::parse(selector_str) {
                if let Some(first_selector) = selectors.first() {
                    // getAttrs() returns: ['class', classNames.join(' '), ...attrs]
                    let mut attrs: Vec<o::Expression> = vec![];
                    if !first_selector.class_names.is_empty() {
                        attrs.push(*o::literal("class".to_string()));
                        attrs.push(*o::literal(first_selector.class_names.join(" ")));
                    }
                    // Add attribute pairs from selector
                    for i in (0..first_selector.attrs.len()).step_by(2) {
                        attrs.push(*o::literal(first_selector.attrs[i].clone()));
                        if i + 1 < first_selector.attrs.len() {
                            attrs.push(*o::literal(first_selector.attrs[i + 1].clone()));
                        } else {
                            attrs.push(*o::literal(String::new()));
                        }
                    }
                    if !attrs.is_empty() {
                        Some(o::Expression::LiteralArray(o::LiteralArrayExpr {
                            entries: attrs,
                            type_: None,
                            source_span: None,
                        }))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let mut definition_entries = vec![
        o::LiteralMapEntry {
            key: "type".into(),
            value: Box::new(type_expr),
            quoted: false,
        },
        o::LiteralMapEntry {
            key: "selectors".into(),
            value: Box::new(selectors_expr),
            quoted: false,
        },
    ];

    // Add attrs if present (from selector attributes)
    if let Some(attrs) = attrs_expr {
        definition_entries.push(o::LiteralMapEntry {
            key: "attrs".into(),
            value: Box::new(attrs),
            quoted: false,
        });
    }

    // hostBindings, hostVars, hostAttrs - must be before features to match ngtsc order
    // In ngtsc, hostAttrs is set in baseDirectiveFields (line 525), before addFeatures (line 190)
    if let Some(host_job) = host_job {
        // hostBindings
        if let Some(host_fn) = emit_host_binding_function(host_job) {
            definition_entries.push(o::LiteralMapEntry {
                key: "hostBindings".into(),
                value: Box::new(host_fn),
                quoted: false,
            });
        }

        // hostVars
        let host_vars = host_job.root.vars.unwrap_or(0);
        if host_vars > 0 {
            definition_entries.push(o::LiteralMapEntry {
                key: "hostVars".into(),
                value: Box::new(*o::literal(host_vars as f64)),
                quoted: false,
            });
        }

        // hostAttrs - always emit (even if null or empty) to match ngtsc behavior
        // This is critical for InheritDefinitionFeature to merge correctly from base class
        // In ngtsc, hostAttrs is always set: definitionMap.set('hostAttrs', hostJob.root.attributes);
        // After const_collection phase, hostAttrs should always be Some (even if empty array)
        let host_attrs_expr = if let Some(host_attrs) = &host_job.root.attributes {
            host_attrs.clone()
        } else {
            // Emit empty array if no attributes (matches ngtsc behavior when hostAttrs is empty)
            // This ensures InheritDefinitionFeature can merge correctly
            o::Expression::LiteralArray(o::LiteralArrayExpr {
                entries: vec![],
                type_: None,
                source_span: None,
            })
        };
        definition_entries.push(o::LiteralMapEntry {
            key: "hostAttrs".into(),
            value: Box::new(host_attrs_expr),
            quoted: false,
        });
    }

    // Add features after hostAttrs - matches ngtsc order
    // In ngtsc, addFeatures is called after baseDirectiveFields (line 190), which includes hostAttrs
    let mut features: Vec<o::Expression> = vec![];

    // ProvidersFeature - critical for DI tokens like MAT_FORM_FIELD
    // Must be added BEFORE InheritDefinitionFeature to match ngtsc order
    if let Some(ref providers) = metadata.directive.providers {
        // Emit ɵɵProvidersFeature(providers) or ɵɵProvidersFeature(providers, viewProviders)
        let mut args = vec![providers.clone()];
        if let Some(ref view_providers) = metadata.view_providers {
            args.push(view_providers.clone());
        }
        features.push(o::Expression::InvokeFn(o::InvokeFunctionExpr {
            fn_: o::import_ref(R3::providers_feature()),
            args,
            type_: None,
            source_span: None,
            pure: false,
        }));
    }

    // InheritDefinitionFeature - added when component uses inheritance
    if metadata.directive.uses_inheritance {
        features.push(*o::import_ref(R3::inherit_definition_feature()));
    }

    // Add features to definition if any
    if !features.is_empty() {
        definition_entries.push(o::LiteralMapEntry {
            key: "features".into(),
            value: Box::new(o::Expression::LiteralArray(o::LiteralArrayExpr {
                entries: features,
                type_: None,
                source_span: None,
            })),
            quoted: false,
        });
    }

    definition_entries.push(o::LiteralMapEntry {
        key: "decls".into(),
        value: Box::new(*o::literal(decls as f64)),
        quoted: false,
    });
    definition_entries.push(o::LiteralMapEntry {
        key: "vars".into(),
        value: Box::new(*o::literal(vars as f64)),
        quoted: false,
    });

    // viewQuery function for @ViewChild/@ViewChildren
    if !metadata.directive.view_queries.is_empty() {
        // eprintln!("DEBUG: [emit] Emitting viewQuery for {} queries: {}", metadata.directive.name, metadata.directive.view_queries.len());
        let view_query_fn =
            emit_view_query_function(&metadata.directive.view_queries, &metadata.directive.name);
        definition_entries.push(o::LiteralMapEntry {
            key: "viewQuery".into(),
            value: Box::new(view_query_fn),
            quoted: false,
        });
    }

    // contentQueries function for @ContentChild/@ContentChildren
    if !metadata.directive.queries.is_empty() {
        eprintln!(
            "[EMIT] Generating content queries for component: {} ({} queries)",
            metadata.directive.name,
            metadata.directive.queries.len()
        );

        let mut constant_pool = crate::constant_pool::ConstantPool::new(false);
        let content_queries_fn =
            crate::render3::view::query_generation::create_content_queries_function(
                &metadata.directive.queries,
                &mut constant_pool,
                Some(&metadata.directive.name),
            );
        definition_entries.push(o::LiteralMapEntry {
            key: "contentQueries".into(),
            value: Box::new(content_queries_fn),
            quoted: false,
        });
    }

    // consts - collected element attributes from const_collection phase
    definition_entries.push(o::LiteralMapEntry {
        key: "consts".into(),
        value: Box::new(o::Expression::LiteralArray(o::LiteralArrayExpr {
            entries: job.consts.iter().cloned().collect(),
            type_: None,
            source_span: None,
        })),
        quoted: false,
    });

    definition_entries.push(o::LiteralMapEntry {
        key: "template".into(),
        value: Box::new(template_fn),
        quoted: false,
    });
    definition_entries.push(o::LiteralMapEntry {
        key: "standalone".into(),
        value: Box::new(*o::literal(metadata.directive.is_standalone)),
        quoted: false,
    });

    // styles - shim CSS with [_ngcontent-%COMP%] selectors when Emulated encapsulation
    definition_entries.push(o::LiteralMapEntry {
        key: "styles".into(),
        value: Box::new({
            // Shim styles for Emulated encapsulation (default)
            let shimmed_styles = match metadata.encapsulation {
                ViewEncapsulation::Emulated => {
                    // Transform styles with [_ngcontent-%COMP%] and [_nghost-%COMP%] selectors
                    compile_styles(&metadata.styles, "_ngcontent-%COMP%", "_nghost-%COMP%")
                }
                _ => metadata.styles.clone(),
            };
            o::Expression::LiteralArray(o::LiteralArrayExpr {
                entries: shimmed_styles
                    .iter()
                    .map(|s| *o::literal(s.clone()))
                    .collect(),
                type_: None,
                source_span: None,
            })
        }),
        quoted: false,
    });

    // Optimize encapsulation: when no styles and encapsulation is Emulated, use None
    let effective_encapsulation =
        if metadata.styles.is_empty() && metadata.encapsulation == ViewEncapsulation::Emulated {
            ViewEncapsulation::None
        } else {
            metadata.encapsulation
        };

    definition_entries.push(o::LiteralMapEntry {
        key: "encapsulation".into(),
        value: Box::new(*o::literal(match effective_encapsulation {
            ViewEncapsulation::Emulated => 0.0,
            ViewEncapsulation::None => 2.0,
            ViewEncapsulation::ShadowDom => 3.0,
            ViewEncapsulation::IsolatedShadowDom => 4.0,
        })),
        quoted: false,
    });

    if let Some(content_selectors) = &job.content_selectors {
        // eprintln!(
        //     "DEBUG: Emitting ngContentSelectors for {}",
        //     metadata.directive.name
        // );
        definition_entries.push(o::LiteralMapEntry {
            key: "ngContentSelectors".into(),
            value: Box::new(content_selectors.clone()),
            quoted: false,
        });
    }

    if let Some(export_as) = &metadata.directive.export_as {
        definition_entries.push(o::LiteralMapEntry {
            key: "exportAs".into(),
            value: Box::new(o::Expression::LiteralArray(o::LiteralArrayExpr {
                entries: export_as.iter().map(|s| *o::literal(s.clone())).collect(),
                type_: None,
                source_span: None,
            })),
            quoted: false,
        });
    }

    // Add changeDetection if set (OnPush = 0)
    if let Some(ref change_detection) = metadata.change_detection {
        match change_detection {
            crate::render3::view::api::ChangeDetectionOrExpression::Strategy(strategy) => {
                let value = match strategy {
                    crate::core::ChangeDetectionStrategy::OnPush => 0,
                    crate::core::ChangeDetectionStrategy::Default => 1,
                };
                definition_entries.push(o::LiteralMapEntry {
                    key: "changeDetection".into(),
                    value: Box::new(*o::literal(value as f64)),
                    quoted: false,
                });
            }
            crate::render3::view::api::ChangeDetectionOrExpression::Expression(expr) => {
                definition_entries.push(o::LiteralMapEntry {
                    key: "changeDetection".into(),
                    value: Box::new(expr.clone()),
                    quoted: false,
                });
            }
        }
    }

    // Add inputs
    let inputs_map: IndexMap<String, InputBindingValue> = metadata
        .directive
        .inputs
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                InputBindingValue::Complex(crate::render3::view::util::InputBindingMetadata {
                    class_property_name: v.class_property_name.clone(),
                    binding_property_name: v.binding_property_name.clone(),
                    transform_function: v.transform_function.clone(),
                    is_signal: v.is_signal,
                }),
            )
        })
        .collect();

    // eprintln!("DEBUG: [emit] Component {} has {} inputs", metadata.directive.name, metadata.directive.inputs.len());
    if let Some(inputs_expr) = conditionally_create_directive_binding_literal(&inputs_map, true) {
        // eprintln!("DEBUG: [emit] Emitting inputs map with {} entries", inputs_map.len());
        definition_entries.push(o::LiteralMapEntry {
            key: "inputs".into(),
            value: Box::new(o::Expression::LiteralMap(inputs_expr)),
            quoted: false,
        });
    }

    // Add outputs
    if !metadata.directive.outputs.is_empty() {
        let mut output_entries = vec![];
        for (prop_name, binding_name) in &metadata.directive.outputs {
            output_entries.push(o::LiteralMapEntry {
                key: prop_name.clone(),
                value: Box::new(*o::literal(binding_name.clone())),
                quoted: false,
            });
        }
        definition_entries.push(o::LiteralMapEntry {
            key: "outputs".into(),
            value: Box::new(o::Expression::LiteralMap(o::LiteralMapExpr {
                entries: output_entries,
                type_: None,
                source_span: None,
            })),
            quoted: false,
        });
    }

    // Add dependencies if any - wrap in closure for deferred evaluation
    if !metadata.declarations.is_empty() {
        let mut dep_exprs: Vec<o::Expression> = vec![];

        for (i, decl) in metadata.declarations.iter().enumerate() {
            let is_used = job.used_dependencies.contains(&i);
            let is_module = matches!(decl, R3TemplateDependencyMetadata::NgModule(_));

            if is_used || is_module {
                let expr = match decl {
                    R3TemplateDependencyMetadata::Directive(dir) => {
                        eprintln!(
                            "[EMIT] Adding used directive {} to dependencies: {}",
                            i, dir.selector
                        );
                        dir.type_.clone()
                    }
                    R3TemplateDependencyMetadata::Pipe(pipe) => pipe.type_.clone(),
                    R3TemplateDependencyMetadata::NgModule(module) => module.type_.clone(),
                };
                dep_exprs.push(expr);
            }
        }

        if !dep_exprs.is_empty() {
            let deps_array = o::Expression::LiteralArray(o::LiteralArrayExpr {
                entries: dep_exprs,
                type_: None,
                source_span: None,
            });

            let deps_value = match metadata.declaration_list_emit_mode {
                crate::render3::view::api::DeclarationListEmitMode::Direct => deps_array,
                crate::render3::view::api::DeclarationListEmitMode::Closure
                | crate::render3::view::api::DeclarationListEmitMode::ClosureResolved => {
                    o::Expression::ArrowFn(o::ArrowFunctionExpr {
                        params: vec![],
                        body: o::ArrowFunctionBody::Expression(Box::new(deps_array)),
                        type_: None,
                        source_span: None,
                    })
                }
                crate::render3::view::api::DeclarationListEmitMode::RuntimeResolved => {
                    // RuntimeResolved usually implies closure too in AOT context, or different handling.
                    // For now treat as closure or todo.
                    o::Expression::ArrowFn(o::ArrowFunctionExpr {
                        params: vec![],
                        body: o::ArrowFunctionBody::Expression(Box::new(deps_array)),
                        type_: None,
                        source_span: None,
                    })
                }
            };

            definition_entries.push(o::LiteralMapEntry {
                key: "dependencies".into(),
                value: Box::new(deps_value),
                quoted: false,
            });
        }
    }

    let definition = o::Expression::LiteralMap(o::LiteralMapExpr {
        entries: definition_entries,
        type_: None,
        source_span: None,
    });

    let expr = o::import_ref(R3::define_component()).call_fn(vec![definition], None, None);

    R3CompiledExpression::new(*expr, o::dynamic_type(), statements)
}

pub fn emit_ops(
    job: &dyn crate::template::pipeline::src::compilation::CompilationJob,
    ops: Vec<&dyn ir::Op>,
) -> Vec<o::Statement> {
    let mut stmts = vec![];

    for op in ops {
        eprintln!("[EMIT_DEBUG_OP] OpKind: {:?}", op.kind());
        match op.kind() {
            ir::OpKind::ElementStart => {
                if let Some(element_op) = op
                    .as_any()
                    .downcast_ref::<ir::ops::create::ElementStartOp>()
                {
                    let index = element_op.base.base.handle.get_slot().unwrap();
                    // Handle tag which might be Option<String>
                    let mut tag = element_op.base.tag.clone().unwrap_or("div".to_string());

                    if tag == "mat-label" {
                        eprintln!("[EMIT] mat-label index={}", index);
                    }
                    // eprintln!("[EMIT_ALL] tag={} index={}", tag, index);

                    // Strip namespace prefix if present (e.g., ":svg:svg" -> "svg")
                    // When namespace op is present, tag name should not have prefix
                    if tag.starts_with(':') {
                        if let Ok((_, stripped_name)) =
                            crate::ml_parser::tags::split_ns_name(&tag, false)
                        {
                            tag = stripped_name;
                        }
                    }

                    // Build args: slot, tag, [constsIndex]
                    let mut args = vec![*o::literal(index as f64), *o::literal(tag.clone())];

                    // Add consts index if element has attributes (event bindings, etc.)
                    if let Some(consts_index) = element_op.base.base.attributes {
                        args.push(*o::literal(consts_index.0 as f64));
                    }

                    // FORCE domElementStart
                    let instruction = R3::dom_element_start();

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(instruction),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }

            ir::OpKind::ElementEnd => {
                stmts.push(o::Statement::Expression(o::ExpressionStatement {
                    expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                        fn_: o::import_ref(R3::dom_element_end()),
                        args: vec![],
                        type_: None,
                        source_span: None,
                        pure: false,
                    })),
                    source_span: None,
                }));
            }

            // Handle merged Element (self-closing empty elements)
            ir::OpKind::Element => {
                if let Some(element_op) = op.as_any().downcast_ref::<ir::ops::create::ElementOp>() {
                    let index = element_op.base.base.handle.get_slot().unwrap();
                    let mut tag = element_op.base.tag.clone().unwrap_or("div".to_string());

                    // Strip namespace prefix if present (e.g., ":svg:svg" -> "svg")
                    // When namespace op is present, tag name should not have prefix
                    if tag.starts_with(':') {
                        if let Ok((_, stripped_name)) =
                            crate::ml_parser::tags::split_ns_name(&tag, false)
                        {
                            tag = stripped_name;
                        }
                    }

                    // Build args: slot, tag, [constsIndex]
                    let mut args = vec![*o::literal(index as f64), *o::literal(tag)];

                    // Add consts index if element has attributes
                    if let Some(consts_index) = element_op.base.base.attributes {
                        args.push(*o::literal(consts_index.0 as f64));
                    }

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(R3::dom_element()),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }

            // Handle empty container (OpKind::Container)
            ir::OpKind::Container => {
                eprintln!("[EMIT_DEBUG] OpKind::Container found. Debug: {:?}", op);
                if let Some(container_op) =
                    op.as_any().downcast_ref::<ir::ops::create::ContainerOp>()
                {
                    eprintln!(
                        "[EMIT] Handling Container op for slot {}",
                        container_op.base.handle.get_slot().unwrap()
                    );
                    let index = container_op.base.handle.get_slot().unwrap();
                    let mut args = vec![*o::literal(index as f64)];

                    // Add attrsIndex if present
                    if let Some(const_idx) = container_op.base.attributes {
                        args.push(*o::literal(const_idx.as_usize() as f64));
                    } else if container_op.base.local_refs_index.is_some() {
                        args.push(*o::literal(o::LiteralValue::Null));
                    }

                    // Add localRefsIndex if present
                    if let Some(local_refs_idx) = container_op.base.local_refs_index {
                        args.push(*o::literal(local_refs_idx.as_usize() as f64));
                    }

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(R3::element_container()),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                } else {
                    eprintln!("[EMIT_ERROR] Failed to downcast OpKind::Container to ir::ops::create::ContainerOp");
                }
            }

            // Handle element container (ng-container)
            ir::OpKind::ContainerStart => {
                if let Some(container_op) = op
                    .as_any()
                    .downcast_ref::<ir::ops::create::ContainerStartOp>()
                {
                    eprintln!(
                        "[EMIT] Handling ContainerStart op for slot {}",
                        container_op.base.handle.get_slot().unwrap()
                    );
                    let index = container_op.base.handle.get_slot().unwrap();
                    let mut args = vec![*o::literal(index as f64)];

                    // Add attrsIndex if present
                    if let Some(const_idx) = container_op.base.attributes {
                        args.push(*o::literal(const_idx.as_usize() as f64));
                    } else if container_op.base.local_refs_index.is_some() {
                        args.push(*o::literal(o::LiteralValue::Null));
                    }

                    // Add localRefsIndex if present
                    if let Some(local_refs_idx) = container_op.base.local_refs_index {
                        args.push(*o::literal(local_refs_idx.as_usize() as f64));
                    }

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(R3::element_container_start()),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }

            ir::OpKind::ContainerEnd => {
                stmts.push(o::Statement::Expression(o::ExpressionStatement {
                    expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                        fn_: o::import_ref(R3::element_container_end()),
                        args: vec![],
                        type_: None,
                        source_span: None,
                        pure: false,
                    })),
                    source_span: None,
                }));
            }

            // Handle template (ng-template) declaration
            ir::OpKind::Template => {
                if let Some(template_op) = op.as_any().downcast_ref::<ir::ops::create::TemplateOp>()
                {
                    let slot = template_op.base.base.handle.get_slot().unwrap() as i32;
                    let view_xref = template_op.base.base.xref;

                    // Get the template function reference
                    let view = if view_xref == job.root_xref() {
                        job.root()
                    } else {
                        get_unit(job, view_xref).expect("Template view not found")
                    };
                    let fn_name = view
                        .fn_name()
                        .expect("Template function name not assigned")
                        .to_string();

                    let decls = template_op.decls.unwrap_or(0) as i32;
                    let vars = template_op.vars.unwrap_or(0) as i32;

                    // Build args: slot, templateFn, decls, vars, [tag], [attrsIndex], [localRefsIndex]
                    let mut args: Vec<o::Expression> = vec![
                        *o::literal(slot as f64),
                        o::Expression::ReadVar(o::ReadVarExpr {
                            name: fn_name.clone(),
                            type_: None,
                            source_span: None,
                        }),
                        *o::literal(decls as f64),
                        *o::literal(vars as f64),
                    ];

                    let instruction = match template_op.template_kind {
                        ir::enums::TemplateKind::NgTemplate => {
                            // For ng-template with directives (e.g., [ngTemplateOutlet]),
                            // use ɵɵtemplate which triggers directive matching via declareDirectiveHostTemplate.
                            // For ng-template without directives, use ɵɵdomTemplate which calls
                            // declareNoDirectiveHostTemplate (faster, no directive matching).
                            args.push(*o::literal("ng-template".to_string()));

                            // Add attrsIndex if present
                            if let Some(const_idx) = template_op.base.base.attributes {
                                args.push(*o::literal(const_idx.as_usize() as f64));
                            } else if template_op.base.base.local_refs_index.is_some() {
                                args.push(*o::literal(o::LiteralValue::Null));
                            }

                            // Add localRefsIndex if present
                            if let Some(local_refs_idx) = template_op.base.base.local_refs_index {
                                args.push(*o::literal(local_refs_idx.as_usize() as f64));
                            }

                            // Use ɵɵtemplate when directives are present, ɵɵdomTemplate otherwise
                            if template_op.base.has_directives {
                                R3::template_create() // ɵɵtemplate - triggers directive matching
                            } else {
                                R3::dom_template() // ɵɵdomTemplate - no directive matching
                            }
                        }
                        ir::enums::TemplateKind::Structural | ir::enums::TemplateKind::Block => {
                            // Add tag if present (for structural directives like *ngIf)
                            if let Some(ref tag) = template_op.base.tag {
                                args.push(*o::literal(tag.clone()));
                            } else {
                                // For structural templates without tag, use null (represented as absent argument)
                                // But we need placeholder if we have attrsIndex or localRefsIndex
                                if template_op.base.base.attributes.is_some()
                                    || template_op.base.base.local_refs_index.is_some()
                                {
                                    args.push(*o::literal(o::LiteralValue::Null));
                                }
                            }

                            // Add attrsIndex if present
                            if let Some(const_idx) = template_op.base.base.attributes {
                                args.push(*o::literal(const_idx.as_usize() as f64));
                            } else if template_op.base.base.local_refs_index.is_some() {
                                // Need placeholder for localRefsIndex
                                args.push(*o::literal(o::LiteralValue::Null));
                            }

                            // Add localRefsIndex if present
                            if let Some(local_refs_idx) = template_op.base.base.local_refs_index {
                                args.push(*o::literal(local_refs_idx.as_usize() as f64));
                            }

                            R3::template_create()
                        }
                    };

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(instruction),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }

            ir::OpKind::Text => {
                if let Some(text_op) = op.as_any().downcast_ref::<ir::ops::create::TextOp>() {
                    let index = text_op.handle.get_slot().unwrap(); // Access field
                    let content = &text_op.initial_value;
                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(R3::text()),
                            args: vec![*o::literal(index as f64), *o::literal(content.clone())], // Deref Box
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::RepeaterCreate => {
                if let Some(rep_op) = op
                    .as_any()
                    .downcast_ref::<ir::ops::create::RepeaterCreateOp>()
                {
                    let index = rep_op.base.base.handle.get_slot().unwrap();

                    // Build args: slot, templateFn, decls, vars, trackFn, [constIndex], [localRefsIndex]
                    let mut args: Vec<o::Expression> = vec![*o::literal(index as f64)];

                    // Template function reference - get from referenced view
                    let view_xref = rep_op.base.base.xref;
                    let view = if view_xref == job.root_xref() {
                        job.root()
                    } else {
                        get_unit(job, view_xref).expect("Template view not found")
                    };
                    let fn_name = view
                        .fn_name()
                        .expect("Template function name not assigned")
                        .to_string();

                    args.push(o::Expression::ReadVar(o::ReadVarExpr {
                        name: fn_name,
                        type_: None,
                        source_span: None,
                    }));

                    // Decls
                    if let Some(decls) = rep_op.decls {
                        args.push(*o::literal(decls as f64));
                    } else {
                        args.push(*o::literal(0.0));
                    }

                    // Vars
                    if let Some(vars) = rep_op.vars {
                        args.push(*o::literal(vars as f64));
                    } else {
                        args.push(*o::literal(0.0));
                    }

                    // Tag (Arg 4 - optional)
                    if let Some(ref tag) = rep_op.base.tag {
                        args.push(o::Expression::Literal(o::LiteralExpr {
                            value: o::LiteralValue::String(tag.clone()),
                            type_: None,
                            source_span: None,
                        }));
                    } else {
                        args.push(*o::literal(o::LiteralValue::Null));
                    }

                    // Const index / Attrs index (Arg 5 - optional)
                    if let Some(const_idx) = rep_op.base.base.attributes {
                        args.push(*o::literal(const_idx.as_usize() as f64));
                    } else {
                        args.push(*o::literal(o::LiteralValue::Null));
                    }

                    // Track function (Arg 6 - Required for control flow)
                    if let Some(ref track_fn) = rep_op.track_by_fn {
                        println!("[Emit Debug] Repeater track_fn IS SOME");
                        // Implement trackBy function: (index, item) => track_expr
                        let params = vec![
                            o::FnParam {
                                name: "$index".to_string(),
                                type_: None,
                            },
                            o::FnParam {
                                name: rep_op.var_names.dollar_implicit.clone(),
                                type_: None,
                            },
                        ];

                        let arrow_fn = o::Expression::ArrowFn(o::ArrowFunctionExpr {
                            params,
                            body: o::ArrowFunctionBody::Expression(track_fn.clone()),
                            type_: None,
                            source_span: None,
                        });

                        args.push(arrow_fn);
                    } else {
                        println!("[Emit Debug] Repeater track_fn IS NONE");
                        args.push(*o::literal(o::LiteralValue::Null));
                    }

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(R3::repeater_create()),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }

            ir::OpKind::ConditionalCreate => {
                if let Some(cond_op) = op
                    .as_any()
                    .downcast_ref::<ir::ops::create::ConditionalCreateOp>()
                {
                    let slot = cond_op
                        .base
                        .base
                        .handle
                        .get_slot()
                        .expect("Expected a slot") as i32;
                    let view_xref = cond_op.base.base.xref;
                    let view = if view_xref == job.root_xref() {
                        job.root()
                    } else {
                        get_unit(job, view_xref).expect("Template view not found")
                    };
                    let fn_name = view
                        .fn_name()
                        .expect("Template function name not assigned")
                        .to_string();

                    let decls = cond_op.decls.unwrap_or(0);
                    let vars = cond_op.vars.unwrap_or(0);
                    let tag = cond_op.base.tag.clone();
                    let const_index = cond_op
                        .base
                        .base
                        .attributes
                        .map(|idx| idx.as_usize() as i32);
                    let local_ref_index = cond_op
                        .base
                        .base
                        .local_refs_index
                        .map(|idx| idx.as_usize() as i32);

                    stmts.push(ng::conditional_create(
                        slot,
                        *o::variable(fn_name),
                        decls,
                        vars,
                        tag,
                        const_index,
                        cond_op.base.base.start_source_span.clone(),
                    ));
                }
            }
            ir::OpKind::ConditionalBranchCreate => {
                if let Some(branch_op) = op
                    .as_any()
                    .downcast_ref::<ir::ops::create::ConditionalBranchCreateOp>()
                {
                    let slot = branch_op
                        .base
                        .base
                        .handle
                        .get_slot()
                        .expect("Expected a slot") as i32;
                    let view_xref = branch_op.base.base.xref;
                    let view = if view_xref == job.root_xref() {
                        job.root()
                    } else {
                        get_unit(job, view_xref).expect("Template view not found")
                    };
                    let fn_name = view
                        .fn_name()
                        .expect("Template function name not assigned")
                        .to_string();

                    let decls = branch_op.decls.unwrap_or(0);
                    let vars = branch_op.vars.unwrap_or(0);
                    let tag = branch_op.base.tag.clone();
                    let const_index = branch_op
                        .base
                        .base
                        .attributes
                        .map(|idx| idx.as_usize() as i32);
                    let local_ref_index = branch_op
                        .base
                        .base
                        .local_refs_index
                        .map(|idx| idx.as_usize() as i32);

                    stmts.push(ng::conditional_create(
                        slot,
                        *o::variable(fn_name),
                        decls,
                        vars,
                        tag,
                        const_index,
                        branch_op.base.base.start_source_span.clone(),
                    ));
                }
            }
            ir::OpKind::Repeater => {
                if let Some(rep_op) = op.as_any().downcast_ref::<ir::ops::update::RepeaterOp>() {
                    let collection = rep_op.collection.clone();
                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(R3::repeater()),
                            args: vec![collection], // No deref needed
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::Conditional => {
                if let Some(cond_op) = op.as_any().downcast_ref::<ir::ops::update::ConditionalOp>()
                {
                    // The processed field contains the ternary expression like:
                    // ctx.isLoading ? 12 : ctx.hasError ? 13 : 14
                    if let Some(ref processed) = cond_op.processed {
                        let mut args = vec![processed.clone()];
                        // If there's a context value, add it as well
                        if let Some(ref ctx_value) = cond_op.context_value {
                            args.push(ctx_value.clone());
                        }
                        stmts.push(o::Statement::Expression(o::ExpressionStatement {
                            expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                                fn_: o::import_ref(R3::conditional()),
                                args,
                                type_: None,
                                source_span: None,
                                pure: false,
                            })),
                            source_span: None,
                        }));
                    }
                }
            }
            ir::OpKind::Projection => {
                if let Some(proj_op) = op.as_any().downcast_ref::<ir::ops::create::ProjectionOp>() {
                    let slot = proj_op
                        .handle
                        .get_slot()
                        .expect("Projection slot must be allocated");
                    eprintln!("[EMIT] Projection instruction at slot: {}", slot);

                    let mut args = vec![*o::literal(slot as f64)];
                    if proj_op.projection_slot_index > 0 {
                        args.push(*o::literal(proj_op.projection_slot_index as f64));
                    }
                    if let Some(const_idx) = proj_op.attributes.as_ref() {
                        // TODO: Support projection attributes (e.g. for fallback view)
                        // For now, we only support basic projection
                        // If attributes exist, we might need to handle them similar to directives
                    }
                    // Fallback view handling (optional)
                    if let Some(fallback_view_xref) = proj_op.fallback_view {
                        let fallback_view = if fallback_view_xref == job.root_xref() {
                            job.root()
                        } else {
                            get_unit(job, fallback_view_xref).expect("Fallback view not found")
                        };
                        let fn_name = fallback_view
                            .fn_name()
                            .expect("Fallback view function name not assigned")
                            .to_string();
                        // Fallback view not fully implemented yet in args, mimicking ngtsc might require more complex logic
                        // But typically it's just projection(slot, selector, attrs)
                    }

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(R3::projection()),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::ProjectionDef => {
                if let Some(proj_def_op) = op
                    .as_any()
                    .downcast_ref::<ir::ops::create::ProjectionDefOp>()
                {
                    let args = if let Some(def) = &proj_def_op.def {
                        vec![def.clone()]
                    } else {
                        vec![]
                    };
                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(R3::projection_def()),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::Advance => {
                if let Some(adv_op) = op.as_any().downcast_ref::<ir::ops::update::AdvanceOp>() {
                    let delta = adv_op.delta;
                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(R3::advance()),
                            args: vec![*o::literal(delta as f64)], // Deref Box
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::InterpolateText => {
                if let Some(interp_op) = op
                    .as_any()
                    .downcast_ref::<ir::ops::update::InterpolateTextOp>()
                {
                    let interpolation = &interp_op.interpolation;

                    // Collate interpolation args: interleave strings and expressions
                    // Special case: if 1 expression and both strings are empty, use textInterpolate with just expression
                    let interpolation_args: Vec<Expression> = if interpolation.expressions.len()
                        == 1
                        && interpolation.strings.len() == 2
                        && interpolation.strings[0].is_empty()
                        && interpolation.strings[1].is_empty()
                    {
                        // Special case: single expression with empty strings -> use textInterpolate(expr)
                        vec![interpolation.expressions[0].clone()]
                    } else {
                        // Normal case: interleave strings and expressions
                        // Format: [strings[0], expr[0], strings[1], expr[1], ..., strings[n]]
                        let mut args = vec![];
                        for (idx, expr) in interpolation.expressions.iter().enumerate() {
                            args.push(*o::literal(interpolation.strings[idx].to_string()));
                            args.push(expr.clone());
                        }
                        // Add last string (NGTSC always includes it even if empty, except for textInterpolate(v))
                        let last_string =
                            interpolation.strings[interpolation.strings.len() - 1].clone();
                        if !last_string.is_empty() {
                            args.push(*o::literal(last_string.to_string()));
                        }
                        args
                    };

                    // Choose function based on number of args
                    // TEXT_INTERPOLATE_CONFIG mapping
                    // For special case (1 arg), n = 0 -> textInterpolate
                    // For others, n is the number of expressions, which is len / 2
                    let n = interpolation_args.len() / 2;

                    let fn_ref = match n {
                        0 => R3::text_interpolate(),
                        1 => R3::text_interpolate1(),
                        2 => R3::text_interpolate2(),
                        3 => R3::text_interpolate3(),
                        4 => R3::text_interpolate4(),
                        5 => R3::text_interpolate5(),
                        6 => R3::text_interpolate6(),
                        7 => R3::text_interpolate7(),
                        8 => R3::text_interpolate8(),
                        _ => R3::text_interpolate_v(), // Use variadic version for > 8
                    };

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(fn_ref),
                            args: interpolation_args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::Namespace => {
                if let Some(ns_op) = op.as_any().downcast_ref::<ir::ops::create::NamespaceOp>() {
                    let fn_val = match ns_op.active {
                        ir::enums::Namespace::HTML => R3::namespace_html(),
                        ir::enums::Namespace::SVG => R3::namespace_svg(),
                        ir::enums::Namespace::Math => R3::namespace_math_ml(),
                    };

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(fn_val),
                            args: vec![],
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::Statement => {
                if let Some(stmt_op) =
                    op.as_any().downcast_ref::<ir::ops::shared::StatementOp<
                        Box<dyn ir::operations::CreateOp + Send + Sync>,
                    >>()
                {
                    stmts.push(*stmt_op.statement.clone());
                } else if let Some(stmt_op) =
                    op.as_any().downcast_ref::<ir::ops::shared::StatementOp<
                        Box<dyn ir::operations::UpdateOp + Send + Sync>,
                    >>()
                {
                    stmts.push(*stmt_op.statement.clone());
                }
            }
            ir::OpKind::Listener => {
                if let Some(listener_op) = op.as_any().downcast_ref::<ir::ops::create::ListenerOp>()
                {
                    // Emit ɵɵlistener('eventName', function handlerFn() { return handler; })
                    let event_name = listener_op.name.clone();

                    // Build handler function body from handler_ops
                    let handler_stmts = emit_ops(
                        job,
                        listener_op
                            .handler_ops
                            .iter()
                            .map(|op| op.as_ref() as &dyn ir::Op)
                            .collect(),
                    );

                    // Create handler function
                    let handler_fn_name = listener_op.handler_fn_name.clone();
                    let mut params = vec![];
                    if listener_op.consumes_dollar_event {
                        params.push(o::FnParam {
                            name: "$event".to_string(),
                            type_: None,
                        });
                    }
                    let handler_fn = o::Expression::Fn(o::FunctionExpr {
                        name: handler_fn_name,
                        params,
                        statements: handler_stmts,
                        type_: None,
                        source_span: None,
                    });

                    // Build args: eventName, handlerFn
                    let mut args = vec![*o::literal(event_name.to_string()), handler_fn];

                    // Add event target if present (e.g., "document:click")
                    if let Some(ref event_target) = listener_op.event_target {
                        args.push(*o::literal(event_target.to_string()));
                    }

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(R3::listener()),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::TwoWayListener => {
                if let Some(listener_op) = op
                    .as_any()
                    .downcast_ref::<ir::ops::create::TwoWayListenerOp>()
                {
                    // Emit ɵɵtwoWayListener('eventName', function handlerFn($event) { return handler; })
                    let event_name = listener_op.name.clone();

                    // Build handler function body from handler_ops
                    let handler_stmts = emit_ops(
                        job,
                        listener_op
                            .handler_ops
                            .iter()
                            .map(|op| op.as_ref() as &dyn ir::Op)
                            .collect(),
                    );

                    // Create handler function
                    let handler_fn_name = listener_op.handler_fn_name.clone();
                    let mut params = vec![];

                    // Two-way listeners always consume $event
                    params.push(o::FnParam {
                        name: "$event".to_string(),
                        type_: None,
                    });

                    let handler_fn = o::Expression::Fn(o::FunctionExpr {
                        name: handler_fn_name,
                        params,
                        statements: handler_stmts,
                        type_: None,
                        source_span: None,
                    });

                    stmts.push(ng::two_way_listener(
                        event_name, handler_fn, false, // default prevent_default to false
                        None,
                    ));
                }
            }
            ir::OpKind::Variable => {
                // Handle variable declarations (e.g. NextContext)
                // We need to support both CreateOp and UpdateOp variants of VariableOp
                if let Some(var_op) = op.as_any().downcast_ref::<ir::ops::shared::VariableOp<Box<dyn ir::CreateOp + Send + Sync>>>() {
                     let initializer = convert_binding_expression_to_expression(&ir::ops::update::BindingExpression::Expression(*var_op.initializer.clone()));
                     let name = var_op.variable.name().unwrap_or_else(|| {
                         // Fallback structure if name is missing? Usually variables have names.
                         // For generated variables like context restores, they have names.
                         panic!("VariableOp (Create) must have a name");
                     });

                     stmts.push(o::Statement::DeclareVar(o::DeclareVarStmt {
                        name: name.to_string(),
                        value: Some(Box::new(initializer)),
                        type_: None,
                        modifiers: o::StmtModifier::Final,
                        source_span: None,
                     }));
                } else if let Some(var_op) = op.as_any().downcast_ref::<ir::ops::shared::VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>>() {
                     let initializer = convert_binding_expression_to_expression(&ir::ops::update::BindingExpression::Expression(*var_op.initializer.clone()));
                     let name = var_op.variable.name().unwrap_or_else(|| {
                         panic!("VariableOp (Update) must have a name");
                     });

                     stmts.push(o::Statement::DeclareVar(o::DeclareVarStmt {
                        name: name.to_string(),
                        value: Some(Box::new(initializer)),
                        type_: None,
                        modifiers: o::StmtModifier::Final,
                        source_span: None,
                     }));
                }
            }
            ir::OpKind::Property => {
                if let Some(prop_op) = op.as_any().downcast_ref::<ir::ops::update::PropertyOp>() {
                    let expression = convert_binding_expression_to_expression(&prop_op.expression);

                    let instruction = R3::property();
                    let sanitizer = prop_op.sanitizer.clone();

                    let mut args = vec![*o::literal(prop_op.name.to_string()), expression];

                    if let Some(sanitizer_fn) = sanitizer {
                        args.push(sanitizer_fn);
                    }

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(instruction),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::DomProperty => {
                if let Some(prop_op) = op.as_any().downcast_ref::<ir::ops::update::DomPropertyOp>()
                {
                    let expression = convert_binding_expression_to_expression(&prop_op.expression);

                    let instruction = R3::dom_property();
                    let sanitizer = prop_op.sanitizer.clone();

                    let mut args = vec![*o::literal(prop_op.name.to_string()), expression];

                    if let Some(sanitizer_fn) = sanitizer {
                        args.push(sanitizer_fn);
                    }

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(instruction),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::Attribute => {
                if let Some(attr_op) = op.as_any().downcast_ref::<ir::ops::update::AttributeOp>() {
                    let expression = convert_binding_expression_to_expression(&attr_op.expression);

                    let instruction = R3::attribute();
                    let sanitizer = attr_op.sanitizer.clone();

                    let mut args = vec![*o::literal(attr_op.name.to_string()), expression];

                    if let Some(sanitizer_fn) = sanitizer {
                        args.push(sanitizer_fn);
                    }

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(instruction),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::ClassProp => {
                if let Some(class_op) = op.as_any().downcast_ref::<ir::ops::update::ClassPropOp>() {
                    let instruction = R3::class_prop();
                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(instruction),
                            args: vec![
                                *o::literal(class_op.name.to_string()),
                                class_op.expression.clone(),
                            ],
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::StyleProp => {
                if let Some(style_op) = op.as_any().downcast_ref::<ir::ops::update::StylePropOp>() {
                    let expression = convert_binding_expression_to_expression(&style_op.expression);

                    let instruction = R3::style_prop();
                    let mut args = vec![*o::literal(style_op.name.to_string()), expression];

                    if let Some(unit) = &style_op.unit {
                        args.push(*o::literal(unit.clone()));
                    }

                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(instruction),
                            args,
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::ClassMap => {
                if let Some(class_map_op) =
                    op.as_any().downcast_ref::<ir::ops::update::ClassMapOp>()
                {
                    let expression =
                        convert_binding_expression_to_expression(&class_map_op.expression);

                    let instruction = R3::class_map();
                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(instruction),
                            args: vec![expression],
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            ir::OpKind::StyleMap => {
                if let Some(style_map_op) =
                    op.as_any().downcast_ref::<ir::ops::update::StyleMapOp>()
                {
                    let expression =
                        convert_binding_expression_to_expression(&style_map_op.expression);

                    let instruction = R3::style_map();
                    stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                            fn_: o::import_ref(instruction),
                            args: vec![expression],
                            type_: None,
                            source_span: None,
                            pure: false,
                        })),
                        source_span: None,
                    }));
                }
            }
            _ => {
                // Ignore other ops for now
            }
        }
    }
    stmts
}

/// Emits a host binding function from a host binding compilation job.
/// Corresponds to emitHostBindingFunction in TypeScript emit.ts
pub fn emit_host_binding_function(job: &HostBindingCompilationJob) -> Option<o::Expression> {
    let fn_name = job
        .root
        .fn_name
        .clone()
        .expect("host binding function is unnamed");

    use crate::template::pipeline::ir::ops::shared::StatementOp;

    let mut create_stmts = vec![];
    for op in &job.root.create {
        if op.kind() == ir::OpKind::Statement {
            if let Some(stmt_op) = op
                .as_any()
                .downcast_ref::<StatementOp<Box<dyn ir::CreateOp + Send + Sync>>>()
            {
                create_stmts.push(*stmt_op.statement.clone());
            }
        } else if op.kind() == ir::OpKind::Listener {
            if let Some(listener_op) =
                op.as_any()
                    .downcast_ref::<crate::template::pipeline::ir::ops::create::ListenerOp>()
            {
                let event_name = listener_op.name.clone();

                // Build handler function body
                let mut handler_stmts = vec![];
                for handler_op in &listener_op.handler_ops {
                    if let Some(stmt_op) = handler_op
                        .as_any()
                        .downcast_ref::<StatementOp<Box<dyn ir::UpdateOp + Send + Sync>>>()
                    {
                        handler_stmts.push(*stmt_op.statement.clone());
                    } else {
                        // Fallback or panic if handler op is not a statement
                        panic!(
                            "Expected StatementOp in host listener handler, got {:?}",
                            handler_op.kind()
                        );
                    }
                }

                // Create handler function
                let handler_fn_name = listener_op.handler_fn_name.clone();
                let mut params = vec![];
                if listener_op.consumes_dollar_event {
                    params.push(o::FnParam {
                        name: "$event".to_string(),
                        type_: None,
                    });
                }
                let handler_fn = o::Expression::Fn(o::FunctionExpr {
                    name: handler_fn_name,
                    params,
                    statements: handler_stmts,
                    type_: None,
                    source_span: None,
                });

                // Build args: eventName, handlerFn
                let mut args = vec![*o::literal(event_name.to_string()), handler_fn];

                if let Some(ref event_target) = listener_op.event_target {
                    args.push(*o::literal(event_target.to_string()));
                }

                create_stmts.push(o::Statement::Expression(o::ExpressionStatement {
                    expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                        fn_: o::import_ref(R3::listener()),
                        args,
                        type_: None,
                        source_span: None,
                        pure: false,
                    })),
                    source_span: None,
                }));
            }
        } else {
            panic!(
                "AssertionError: expected all create ops to have been compiled, but got {:?}",
                op.kind()
            );
        }
    }

    // Generate update statements using emit_ops
    let update_ops: Vec<&dyn ir::Op> = job
        .root
        .update
        .iter()
        .map(|op| op.as_ref() as &dyn ir::Op)
        .collect();
    let update_stmts = emit_ops(job, update_ops);

    if create_stmts.is_empty() && update_stmts.is_empty() {
        return None;
    }

    let fn_name = format!("{}_HostBindings", job.component_name);

    Some(o::Expression::Fn(o::FunctionExpr {
        name: Some(fn_name),
        params: vec![
            o::FnParam {
                name: "rf".to_string(),
                type_: None,
            },
            o::FnParam {
                name: "ctx".to_string(),
                type_: None,
            },
        ],
        statements: vec![
            // if (rf & 1) { ...create... }
            o::Statement::IfStmt(o::IfStmt {
                condition: Box::new(o::Expression::BinaryOp(o::BinaryOperatorExpr {
                    operator: o::BinaryOperator::BitwiseAnd,
                    lhs: Box::new(o::Expression::ReadVar(o::ReadVarExpr {
                        name: "rf".to_string(),
                        type_: None,
                        source_span: None,
                    })),
                    rhs: Box::new(*o::literal(1.0)),
                    type_: None,
                    source_span: None,
                })),
                true_case: create_stmts,
                false_case: vec![],
                source_span: None,
            }),
            // if (rf & 2) { ...update... }
            o::Statement::IfStmt(o::IfStmt {
                condition: Box::new(o::Expression::BinaryOp(o::BinaryOperatorExpr {
                    operator: o::BinaryOperator::BitwiseAnd,
                    lhs: Box::new(o::Expression::ReadVar(o::ReadVarExpr {
                        name: "rf".to_string(),
                        type_: None,
                        source_span: None,
                    })),
                    rhs: Box::new(*o::literal(2.0)),
                    type_: None,
                    source_span: None,
                })),
                true_case: update_stmts,
                false_case: vec![],
                source_span: None,
            }),
        ],
        type_: None,
        source_span: None,
    }))
}

/// Emits a viewQuery function for @ViewChild/@ViewChildren decorators.
/// Generates code like:
/// ```js
/// function ComponentName_Query(rf, ctx) {
///   if (rf & 1) {
///     i0.ɵɵviewQuery(_c0, 5)(_c1, 5)(_c2, 5);
///   }
///   if (rf & 2) {
///     let _t;
///     i0.ɵɵqueryRefresh((_t = i0.ɵɵloadQuery())) && (ctx.checkbox = _t.first);
///     i0.ɵɵqueryRefresh((_t = i0.ɵɵloadQuery())) && (ctx.input = _t.first);
///     i0.ɵɵqueryRefresh((_t = i0.ɵɵloadQuery())) && (ctx.label = _t.first);
///   }
/// }
/// ```
fn emit_view_query_function(
    view_queries: &[crate::render3::view::api::R3QueryMetadata],
    component_name: &str,
) -> o::Expression {
    let fn_name = format!("{}_Query", component_name);
    let mut create_stmts = vec![];
    let mut update_stmts = vec![];

    // Generate create block (rf & 1)
    if !view_queries.is_empty() {
        let mut chain_expr: Option<o::Expression> = None;

        for query in view_queries {
            if query.is_signal {
                // Signal based queries cannot be chained with viewQuery, flush existing chain
                if let Some(expr) = chain_expr.take() {
                    create_stmts.push(o::Statement::Expression(o::ExpressionStatement {
                        expr: Box::new(expr),
                        source_span: None,
                    }));
                }

                // Emit ɵɵviewQuerySignal(ctx.prop, selector, flags)
                let selector = match &query.predicate {
                    crate::render3::view::api::R3QueryPredicate::Selectors(selectors) => {
                        selectors.first().cloned().unwrap_or_default()
                    }
                    _ => String::new(),
                };

                let selector_arr = o::Expression::LiteralArray(o::LiteralArrayExpr {
                    entries: vec![*o::literal(selector.clone())],
                    type_: None,
                    source_span: None,
                });

                // Flags: 1 = Descendants, 2 = Static, 4 = EmitDistinctChangesOnly
                let mut flags = 0.0;
                if query.descendants {
                    flags += 1.0;
                }
                if query.static_ {
                    flags += 2.0;
                }
                if query.emit_distinct_changes_only {
                    flags += 4.0;
                }

                let mut args = vec![
                    o::Expression::ReadProp(o::ReadPropExpr {
                        receiver: Box::new(*o::variable("ctx")),
                        name: query.property_name.clone(),
                        type_: None,
                        source_span: None,
                    }),
                    selector_arr,
                    *o::literal(flags),
                ];

                if let Some(ref read) = query.read {
                    args.push(read.clone());
                }

                create_stmts.push(o::Statement::Expression(o::ExpressionStatement {
                    expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                        fn_: o::import_ref(R3::view_query_signal()),
                        args,
                        type_: None,
                        source_span: None,
                        pure: false,
                    })),
                    source_span: None,
                }));
            } else {
                // Non-signal based queries can be chained
                let predicate_expr = match &query.predicate {
                    crate::render3::view::api::R3QueryPredicate::Selectors(selectors) => {
                        o::Expression::LiteralArray(o::LiteralArrayExpr {
                            entries: selectors.iter().map(|s| *o::literal(s.clone())).collect(),
                            type_: None,
                            source_span: None,
                        })
                    }
                    crate::render3::view::api::R3QueryPredicate::Expression(expr) => {
                        crate::render3::util::convert_from_maybe_forward_ref_expression(expr)
                    }
                };

                // Flags: 1 = Descendants, 2 = Static, 4 = EmitDistinctChangesOnly
                let mut flags = 0.0;
                if query.descendants {
                    flags += 1.0;
                }
                if query.static_ {
                    flags += 2.0;
                }
                if query.emit_distinct_changes_only {
                    flags += 4.0;
                }

                let mut args = vec![predicate_expr, *o::literal(flags)];
                if let Some(ref read) = query.read {
                    args.push(read.clone());
                }

                if chain_expr.is_none() {
                    // First call: i0.ɵɵviewQuery(selector, flags)
                    chain_expr = Some(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                        fn_: o::import_ref(R3::view_query()),
                        args,
                        type_: None,
                        source_span: None,
                        pure: false,
                    }));
                } else {
                    // Chain call: prev(selector, flags)
                    chain_expr = Some(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                        fn_: Box::new(chain_expr.take().unwrap()),
                        args,
                        type_: None,
                        source_span: None,
                        pure: false,
                    }));
                }
            }
        }

        if let Some(expr) = chain_expr {
            create_stmts.push(o::Statement::Expression(o::ExpressionStatement {
                expr: Box::new(expr),
                source_span: None,
            }));
        }
    }

    // Generate update block (rf & 2): let _t; + queryRefresh calls
    if !view_queries.is_empty() {
        let has_non_signal_queries = view_queries.iter().any(|q| !q.is_signal);

        // Add: let _t; only if needed
        if has_non_signal_queries {
            update_stmts.push(o::Statement::DeclareVar(o::DeclareVarStmt {
                name: "_t".to_string(),
                value: None,
                type_: None,
                modifiers: o::StmtModifier::None,
                source_span: None,
            }));
        }

        for query in view_queries {
            if query.is_signal {
                // i0.ɵɵqueryAdvance();
                update_stmts.push(o::Statement::Expression(o::ExpressionStatement {
                    expr: Box::new(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                        fn_: o::import_ref(R3::query_advance()),
                        args: vec![],
                        type_: None,
                        source_span: None,
                        pure: false,
                    })),
                    source_span: None,
                }));
            } else {
                // i0.ɵɵqueryRefresh((_t = i0.ɵɵloadQuery())) && (ctx.propertyName = _t.first);

                // loadQuery call
                let load_query = o::Expression::InvokeFn(o::InvokeFunctionExpr {
                    fn_: Box::new(o::Expression::External(o::ExternalExpr {
                        value: o::ExternalReference {
                            module_name: Some("@angular/core".to_string()),
                            name: Some("ɵɵloadQuery".to_string()),
                            runtime: None,
                        },
                        type_: None,
                        source_span: None,
                    })),
                    args: vec![],
                    type_: None,
                    source_span: None,
                    pure: false,
                });

                // _t = i0.ɵɵloadQuery()
                let assign_t = o::Expression::BinaryOp(o::BinaryOperatorExpr {
                    operator: o::BinaryOperator::Assign,
                    lhs: Box::new(*o::variable("_t")),
                    rhs: Box::new(load_query),
                    type_: None,
                    source_span: None,
                });

                // Wrap in parentheses (represented as-is in output)
                let wrapped_assign = assign_t;

                // queryRefresh((_t = loadQuery()))
                let query_refresh = o::Expression::InvokeFn(o::InvokeFunctionExpr {
                    fn_: Box::new(o::Expression::External(o::ExternalExpr {
                        value: o::ExternalReference {
                            module_name: Some("@angular/core".to_string()),
                            name: Some("ɵɵqueryRefresh".to_string()),
                            runtime: None,
                        },
                        type_: None,
                        source_span: None,
                    })),
                    args: vec![wrapped_assign],
                    type_: None,
                    source_span: None,
                    pure: false,
                });

                // ctx.propertyName = _t.first (or _t for ViewChildren)
                let result_access = if query.first {
                    // _t.first
                    o::Expression::ReadProp(o::ReadPropExpr {
                        receiver: Box::new(*o::variable("_t")),
                        name: "first".to_string(),
                        type_: None,
                        source_span: None,
                    })
                } else {
                    // _t (entire query list)
                    *o::variable("_t")
                };

                // ctx.propertyName = ...
                let ctx_prop_assign = o::Expression::BinaryOp(o::BinaryOperatorExpr {
                    operator: o::BinaryOperator::Assign,
                    lhs: Box::new(o::Expression::ReadProp(o::ReadPropExpr {
                        receiver: Box::new(*o::variable("ctx")),
                        name: query.property_name.clone(),
                        type_: None,
                        source_span: None,
                    })),
                    rhs: Box::new(result_access),
                    type_: None,
                    source_span: None,
                });

                // Wrap assignment in parens to ensure correct precedence: a && (b = c)
                let wrapped_assign = o::Expression::Parens(o::ParenthesizedExpr {
                    expr: Box::new(ctx_prop_assign),
                    type_: None,
                    source_span: None,
                });

                // queryRefresh(...) && (ctx.prop = _t.first)
                let and_expr = o::Expression::BinaryOp(o::BinaryOperatorExpr {
                    operator: o::BinaryOperator::And,
                    lhs: Box::new(query_refresh),
                    rhs: Box::new(wrapped_assign),
                    type_: None,
                    source_span: None,
                });

                update_stmts.push(o::Statement::Expression(o::ExpressionStatement {
                    expr: Box::new(and_expr),
                    source_span: None,
                }));
            }
        }
    }

    // Build function body with if (rf & 1) and if (rf & 2) blocks
    let mut body = vec![];

    if !create_stmts.is_empty() {
        body.push(o::Statement::IfStmt(o::IfStmt {
            condition: Box::new(o::Expression::BinaryOp(o::BinaryOperatorExpr {
                operator: o::BinaryOperator::BitwiseAnd,
                lhs: Box::new(*o::variable("rf")),
                rhs: Box::new(*o::literal(1.0)),
                type_: None,
                source_span: None,
            })),
            true_case: create_stmts,
            false_case: vec![],
            source_span: None,
        }));
    }

    if !update_stmts.is_empty() {
        body.push(o::Statement::IfStmt(o::IfStmt {
            condition: Box::new(o::Expression::BinaryOp(o::BinaryOperatorExpr {
                operator: o::BinaryOperator::BitwiseAnd,
                lhs: Box::new(*o::variable("rf")),
                rhs: Box::new(*o::literal(2.0)),
                type_: None,
                source_span: None,
            })),
            true_case: update_stmts,
            false_case: vec![],
            source_span: None,
        }));
    }

    o::Expression::Fn(o::FunctionExpr {
        name: Some(fn_name),
        params: vec![
            o::FnParam {
                name: "rf".to_string(),
                type_: None,
            },
            o::FnParam {
                name: "ctx".to_string(),
                type_: None,
            },
        ],
        statements: body,
        type_: None,
        source_span: None,
    })
}

fn get_unit<'a>(
    job: &'a dyn crate::template::pipeline::src::compilation::CompilationJob,
    xref: crate::template::pipeline::ir::XrefId,
) -> Option<&'a dyn crate::template::pipeline::src::compilation::CompilationUnit> {
    job.units().find(|u| u.xref() == xref)
}

/// Helper to convert BindingExpression to Expression (handling interpolation desugaring)
fn convert_binding_expression_to_expression(
    binding_expression: &crate::template::pipeline::ir::ops::update::BindingExpression,
) -> o::Expression {
    match binding_expression {
        crate::template::pipeline::ir::ops::update::BindingExpression::Expression(e) => {
            // Check for NextContext specifically
            if let o::Expression::NextContext(next_ctx) = e {
                // Emit ɵɵnextContext(steps)
                let instruction = R3::next_context();
                let steps = next_ctx.steps;
                let mut args = vec![];

                // If steps is 1, we can omit the argument if we want strict parity with ngtsc
                // But ngtsc often emits ɵɵnextContext() for 1, or ɵɵnextContext(1)
                // Let's emit it explicitly for clarity, or check ngtsc defaults.
                // Generally ɵɵnextContext() defaults to 1.
                if steps != 1 {
                    args.push(*o::literal(steps as f64));
                }

                return o::Expression::InvokeFn(o::InvokeFunctionExpr {
                    fn_: o::import_ref(instruction),
                    args,
                    type_: None,
                    source_span: None,
                    pure: false,
                });
            }

            e.clone()
        }
        crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(i) => {
            emit_interpolation_expression(i)
        }
    }
}

/// Emit an interpolation as an expression (e.g. ɵɵinterpolate1(p1, p2))
fn emit_interpolation_expression(
    interpolation: &crate::template::pipeline::ir::ops::update::Interpolation,
) -> o::Expression {
    // Collate arguments: string, expr, string, expr, ..., string
    let strings = &interpolation.strings;
    let exprs = &interpolation.expressions;

    // Special case: if we have 1 expression and empty surrounding strings, return the expression directly
    if exprs.len() == 1
        && strings.len() >= 2
        && strings[0].as_ref() == ""
        && strings[1].as_ref() == ""
    {
        return exprs[0].clone();
    }

    let mut args = vec![];
    for (i, expr) in exprs.iter().enumerate() {
        if i < strings.len() {
            args.push(*o::literal(strings[i].as_ref().to_string()));
        } else {
            // Should not happen if strings.len() == exprs.len() + 1
            args.push(*o::literal("".to_string()));
        }
        args.push(expr.clone());
    }
    // Add last string
    if strings.len() > exprs.len() {
        args.push(*o::literal(strings[exprs.len()].as_ref().to_string()));
    }

    // Determine instruction based on number of expressions
    let instruction = match exprs.len() {
        1 => R3::interpolate1(),
        2 => R3::interpolate2(),
        3 => R3::interpolate3(),
        4 => R3::interpolate4(),
        5 => R3::interpolate5(),
        6 => R3::interpolate6(),
        7 => R3::interpolate7(),
        8 => R3::interpolate8(),
        _ => R3::interpolate_v(), // Or interpolateV? Check identifiers.
    };

    // For interpolateV, the arguments are [literals_array] + expressions?
    // Wait, let's check ngtsc instruction.ts again.
    // const TEXT_INTERPOLATE_CONFIG ... variable: Identifiers.textInterpolateV
    // mapping: (n) => (n-1)/2. which means number of expressions.

    // And callVariadicInstructionExpr:
    // for interpolateV (variable pattern):
    // return o.importExpr(config.variable).callFn([...baseArgs, o.literalArr(interpolationArgs), ...extraArgs]);

    // interpolationArgs as constructed by collateInterpolationArgs contains both strings and exprs interleaved.
    // [s0, e0, s1, e1, ..., sN]

    // So for interpolateV, we pass [ [s0, e0, s1... sN] ] (as array literal).

    // However, looking at R3 identifiers, there is `interpolate_v` and `text_interpolate_v`.

    if exprs.len() > 8 {
        // For larger interpolations, we use interpolateV which takes an array of the mix
        o::Expression::InvokeFn(o::InvokeFunctionExpr {
            fn_: o::import_ref(instruction),
            args: vec![o::Expression::LiteralArray(o::LiteralArrayExpr {
                entries: args,
                type_: None,
                source_span: None,
            })],
            type_: None,
            source_span: None,
            pure: false,
        })
    } else {
        o::Expression::InvokeFn(o::InvokeFunctionExpr {
            fn_: o::import_ref(instruction),
            args,
            type_: None,
            source_span: None,
            pure: false,
        })
    }
}
