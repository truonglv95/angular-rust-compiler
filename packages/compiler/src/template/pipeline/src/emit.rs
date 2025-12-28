//! Emit Module
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/emit.ts

use crate::core::ViewEncapsulation;
use crate::output::output_ast as o;
use crate::output::output_ast::Expression;
use crate::render3::r3_identifiers::Identifiers as R3;
use crate::render3::util::R3CompiledExpression;
use crate::render3::view::api::{R3ComponentMetadata, R3TemplateDependencyMetadata};
use crate::render3::view::compiler::compile_styles;
use crate::template::pipeline::ir;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob, HostBindingCompilationJob,
};
use crate::template::pipeline::src::instruction as ng;

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
    println!(
        "DEBUG: emit_component for {} - views: {}, mode: {:?}",
        job.component_name,
        job.views.len(),
        job.mode
    );
    for (id, view) in &job.views {
        println!(
            "DEBUG: View {:?} has {} create ops, {} update ops",
            id,
            view.create.len(),
            view.update.len()
        );
        for op in &view.create {
            println!("DEBUG: Create Op: {:?}", op.kind());
            // println!("DEBUG: Create Op Tag: {:?}", op.tag); // If tag exists
        }
    }
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

    let selectors_expr = if let Some(s) = &metadata.directive.selector {
        let inner = o::Expression::LiteralArray(o::LiteralArrayExpr {
            entries: vec![*o::literal(s.clone())],
            type_: None,
            source_span: None,
        });
        o::Expression::LiteralArray(o::LiteralArrayExpr {
            entries: vec![inner],
            type_: None,
            source_span: None,
        })
    } else {
        *o::literal(o::LiteralValue::Null)
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
        o::LiteralMapEntry {
            key: "decls".into(),
            value: Box::new(*o::literal(decls as f64)),
            quoted: false,
        },
        o::LiteralMapEntry {
            key: "vars".into(),
            value: Box::new(*o::literal(vars as f64)),
            quoted: false,
        },
    ];

    // hostBindings, hostVars, hostAttrs
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

        // hostAttrs
        if let Some(host_attrs) = &host_job.root.attributes {
            definition_entries.push(o::LiteralMapEntry {
                key: "hostAttrs".into(),
                value: Box::new(host_attrs.clone()),
                quoted: false,
            });
        }
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
    if !metadata.directive.inputs.is_empty() {
        let mut input_entries = vec![];
        for (prop_name, input) in &metadata.directive.inputs {
            let value = if input.is_signal {
                // Signal input: [1, binding_name]
                // 1 = InputFlags.SignalBased
                o::Expression::LiteralArray(o::LiteralArrayExpr {
                    entries: vec![
                        *o::literal(1.0),
                        *o::literal(input.binding_property_name.clone()),
                    ],
                    type_: None,
                    source_span: None,
                })
            } else {
                *o::literal(input.binding_property_name.clone())
            };

            input_entries.push(o::LiteralMapEntry {
                key: prop_name.clone(),
                value: Box::new(value),
                quoted: false,
            });
        }

        definition_entries.push(o::LiteralMapEntry {
            key: "inputs".into(),
            value: Box::new(o::Expression::LiteralMap(o::LiteralMapExpr {
                entries: input_entries,
                type_: None,
                source_span: None,
            })),
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

    // Add dependencies if any
    if !metadata.declarations.is_empty() {
        let dep_exprs: Vec<o::Expression> = metadata
            .declarations
            .iter()
            .map(|decl| match decl {
                R3TemplateDependencyMetadata::Directive(dir) => dir.type_.clone(),
                R3TemplateDependencyMetadata::Pipe(pipe) => pipe.type_.clone(),
                R3TemplateDependencyMetadata::NgModule(module) => module.type_.clone(),
            })
            .collect();

        if !dep_exprs.is_empty() {
            definition_entries.push(o::LiteralMapEntry {
                key: "dependencies".into(),
                value: Box::new(o::Expression::LiteralArray(o::LiteralArrayExpr {
                    entries: dep_exprs,
                    type_: None,
                    source_span: None,
                })),
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

pub fn emit_ops(job: &ComponentCompilationJob, ops: Vec<&dyn ir::Op>) -> Vec<o::Statement> {
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
                    let tag = element_op.base.tag.clone().unwrap_or("div".to_string());
                    println!("DEBUG: Emit visiting ElementStart with tag '{}'", tag);

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
                    let tag = element_op.base.tag.clone().unwrap_or("div".to_string());

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
                    let view = if view_xref == job.root.xref {
                        &job.root
                    } else {
                        job.views.get(&view_xref).expect("Template view not found")
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
                    let view = if view_xref == job.root.xref {
                        &job.root
                    } else {
                        job.views.get(&view_xref).expect("Template view not found")
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

                    stmts.push(ng::template(
                        slot,
                        *o::variable(fn_name),
                        decls,
                        vars,
                        tag,
                        const_index,
                        local_ref_index,
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
                    let view = if view_xref == job.root.xref {
                        &job.root
                    } else {
                        job.views.get(&view_xref).expect("Template view not found")
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

                    stmts.push(ng::template(
                        slot,
                        *o::variable(fn_name),
                        decls,
                        vars,
                        tag,
                        const_index,
                        local_ref_index,
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
                            args.push(*o::literal(interpolation.strings[idx].clone()));
                            args.push(expr.clone());
                        }
                        // Add last string (NGTSC always includes it even if empty, except for textInterpolate(v))
                        let last_string =
                            interpolation.strings[interpolation.strings.len() - 1].clone();
                        if !last_string.is_empty() {
                            args.push(*o::literal(last_string));
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
                    let mut args = vec![*o::literal(event_name), handler_fn];

                    // Add event target if present (e.g., "document:click")
                    if let Some(ref event_target) = listener_op.event_target {
                        args.push(*o::literal(event_target.clone()));
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
                        event_name, handler_fn, false, // default preventDefault to false
                        None,
                    ));
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
                let mut args = vec![*o::literal(event_name), handler_fn];

                if let Some(ref event_target) = listener_op.event_target {
                    args.push(*o::literal(event_target.clone()));
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

    let mut update_stmts = vec![];
    for op in &job.root.update {
        if op.kind() != ir::OpKind::Statement {
            panic!(
                "AssertionError: expected all update ops to have been compiled, but got {:?}",
                op.kind()
            );
        }
        if let Some(stmt_op) = op
            .as_any()
            .downcast_ref::<StatementOp<Box<dyn ir::UpdateOp + Send + Sync>>>()
        {
            update_stmts.push(*stmt_op.statement.clone());
        }
    }

    if create_stmts.is_empty() && update_stmts.is_empty() {
        return None;
    }

    // Generate rf block conditions
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
        statements: body,
        type_: None,
        source_span: None,
    }))
}
