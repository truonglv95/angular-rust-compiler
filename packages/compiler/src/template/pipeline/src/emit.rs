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
        // eprintln!("DEBUG: [emit] Emitting viewQuery for {} queries", metadata.directive.view_queries.len());
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
        eprintln!(
            "DEBUG: Emitting ngContentSelectors for {}",
            metadata.directive.name
        );
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
                    R3TemplateDependencyMetadata::Directive(dir) => dir.type_.clone(),
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
        match op.kind() {
            ir::OpKind::ElementStart => {
                if let Some(element_op) = op
                    .as_any()
                    .downcast_ref::<ir::ops::create::ElementStartOp>()
                {
                    let index = element_op.base.base.handle.get_slot().unwrap();
                    // Handle tag which might be Option<String>
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
                        fn_: o::import_ref(R3::element_end()),
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
                            fn_: o::import_ref(R3::element()),
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

                    // Build args: slot, templateFn, decls, vars, tag, constIndex, trackFn
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

                    // Tag (optional)
                    if let Some(ref tag) = rep_op.base.tag {
                        args.push(o::Expression::Literal(o::LiteralExpr {
                            value: o::LiteralValue::String(tag.clone()),
                            type_: None,
                            source_span: None,
                        }));
                    }

                    // Const index (optional)
                    if let Some(const_idx) = rep_op.base.base.attributes {
                        args.push(*o::literal(const_idx.as_usize() as f64));
                    }

                    // Track function (optional)
                    if let Some(ref track_fn) = rep_op.track_by_fn {
                        args.push(track_fn.as_ref().clone());
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
                    let mut args = vec![*o::literal(
                        proj_op
                            .handle
                            .get_slot()
                            .expect("Projection slot must be allocated")
                            as f64,
                    )];
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
            ir::OpKind::Property => {
                if let Some(prop_op) = op.as_any().downcast_ref::<ir::ops::update::PropertyOp>() {
                    let expression = match &prop_op.expression {
                        crate::template::pipeline::ir::ops::update::BindingExpression::Expression(e) => {
                            e.clone()
                        }
                        crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(i) => {
                            // Property interpolation needs to be handled
                            // Typically interpolated properties use propertyInterpolate
                            // But here we might receive them as PropertyOp if not specialized?
                            // For simplicity, panic or handle as error if correct op wasn't generated
                            // But actually property binding usually expects Expression. 
                            // If interpolation, it should have been converted to InterpolatePropOp (if exists) or expression
                            panic!("Unexpected interpolation in PropertyOp");
                        }
                    };

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
            ir::OpKind::Attribute => {
                if let Some(attr_op) = op.as_any().downcast_ref::<ir::ops::update::AttributeOp>() {
                    // TODO: attributes can be interpolated (attributeInterpolate)
                    // If expression is interpolation, use attributeInterpolate
                    // For now assume expression

                    let expression = match &attr_op.expression {
                        crate::template::pipeline::ir::ops::update::BindingExpression::Expression(e) => {
                             e.clone()
                        }
                        crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(_) => {
                            panic!("Unexpected interpolation in AttributeOp");
                        }
                     };

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
                    let expression = match &style_op.expression {
                        crate::template::pipeline::ir::ops::update::BindingExpression::Expression(e) => e.clone(),
                         crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(_) => {
                            panic!("Unexpected interpolation in StylePropOp");
                         }
                    };

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
                    let expression = match &class_map_op.expression {
                         crate::template::pipeline::ir::ops::update::BindingExpression::Expression(e) => e.clone(),
                         crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(_) => {
                            // classMapInterpolate?
                             panic!("Unexpected interpolation in ClassMapOp");
                         }
                    };

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
                    let expression = match &style_map_op.expression {
                         crate::template::pipeline::ir::ops::update::BindingExpression::Expression(e) => e.clone(),
                         crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(_) => {
                             panic!("Unexpected interpolation in StyleMapOp");
                         }
                    };

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

    // Generate create block (rf & 1): chained ɵɵviewQuery calls
    if !view_queries.is_empty() {
        // Build chained viewQuery expression
        // i0.ɵɵviewQuery(_c0, 5)(_c1, 5)(_c2, 5)
        let mut chain_expr: Option<o::Expression> = None;

        for query in view_queries {
            let selector = match &query.predicate {
                crate::render3::view::api::R3QueryPredicate::Selectors(selectors) => {
                    selectors.first().cloned().unwrap_or_default()
                }
                _ => String::new(),
            };

            // Create _cN constant reference for the selector string
            // For simplicity, we'll use the selector string directly as a literal array
            let selector_arr = o::Expression::LiteralArray(o::LiteralArrayExpr {
                entries: vec![*o::literal(selector.clone())],
                type_: None,
                source_span: None,
            });

            // Flags: 5 = DescendantsOnly (for ViewChild with descendants=false)
            let flags = if query.first { 5.0 } else { 4.0 };

            if chain_expr.is_none() {
                // First call: i0.ɵɵviewQuery(selector, flags)
                chain_expr = Some(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                    fn_: Box::new(o::Expression::External(o::ExternalExpr {
                        value: o::ExternalReference {
                            module_name: Some("@angular/core".to_string()),
                            name: Some("ɵɵviewQuery".to_string()),
                            runtime: None,
                        },
                        type_: None,
                        source_span: None,
                    })),
                    args: vec![selector_arr, *o::literal(flags)],
                    type_: None,
                    source_span: None,
                    pure: false,
                }));
            } else {
                // Chain call: prev(selector, flags)
                chain_expr = Some(o::Expression::InvokeFn(o::InvokeFunctionExpr {
                    fn_: Box::new(chain_expr.take().unwrap()),
                    args: vec![selector_arr, *o::literal(flags)],
                    type_: None,
                    source_span: None,
                    pure: false,
                }));
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
        // Add: let _t;
        update_stmts.push(o::Statement::DeclareVar(o::DeclareVarStmt {
            name: "_t".to_string(),
            value: None,
            type_: None,
            modifiers: o::StmtModifier::None,
            source_span: None,
        }));

        for query in view_queries {
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
