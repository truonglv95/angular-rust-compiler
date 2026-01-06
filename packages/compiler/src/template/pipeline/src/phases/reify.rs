//! Compiles semantic operations across all views and generates output Statements.
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/reify.ts

use crate::expression_parser::ast::ASTWithSource;
use crate::output::output_ast as o;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::ops::shared::create_statement_op;
use crate::template::pipeline::ir::ops::VariableOp;
use crate::template::pipeline::ir::{CreateOp, UpdateOp};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob, HostBindingCompilationJob,
};
use crate::template::pipeline::src::instruction as ng;

use crate::template::pipeline::ir::handle::XrefId;
use crate::template::pipeline::src::compilation::TemplateCompilationMode;
use std::collections::HashMap;

pub fn reify(job: &mut dyn CompilationJob) {
    let mut view_name_map = HashMap::new();
    for unit in job.units() {
        if let Some(name) = unit.fn_name() {
            view_name_map.insert(unit.xref(), name.to_string());
        }
    }

    // Identify which compilation mode we are in
    let compatibility = job.compatibility() == ir::CompatibilityMode::TemplateDefinitionBuilder;
    let mode = job.mode();

    for unit in job.units_mut() {
        reify_unit(unit, &view_name_map, compatibility, mode);
    }
}

fn reify_unit(
    unit: &mut dyn CompilationUnit,
    view_name_map: &HashMap<XrefId, String>,
    compatibility: bool,
    mode: TemplateCompilationMode,
) {
    reify_create_operations(unit, view_name_map, compatibility, mode);
    reify_update_operations(unit);
}

fn reify_create_operations(
    unit: &mut dyn CompilationUnit,
    view_name_map: &HashMap<XrefId, String>,
    _compatibility: bool,
    mode: TemplateCompilationMode,
) {
    for op in unit.create_mut().iter_mut() {
        ir::transform_expressions_in_op(
            op.as_mut(),
            &mut reify_ir_expression,
            ir::VisitorContextFlag::NONE,
        );

        let new_op: Option<Box<dyn CreateOp + Send + Sync>> = match op.kind() {
            ir::OpKind::Pipe => {
                if let Some(pipe_op) = op.as_any().downcast_ref::<ir::ops::create::PipeOp>() {
                    let slot = pipe_op.handle.get_slot().expect("Expected a slot") as i32;
                    let name = pipe_op.name.clone();
                    let stmt = ng::pipe(slot, name);
                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn CreateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
                }
            }
            ir::OpKind::Namespace => {
                if let Some(template_op) = op.as_any().downcast_ref::<ir::ops::create::TemplateOp>()
                {
                    let slot = template_op
                        .base
                        .base
                        .handle
                        .get_slot()
                        .expect("Expected a slot") as i32;
                    let view_xref = template_op.base.base.xref;
                    let fn_name = view_name_map
                        .get(&view_xref)
                        .expect("Template function name not assigned");

                    let decls = template_op.decls.unwrap_or(0);
                    let vars = template_op.vars.unwrap_or(0);
                    let tag = template_op.base.tag.clone();
                    let const_index = template_op
                        .base
                        .base
                        .attributes
                        .map(|idx| idx.as_usize() as i32);
                    let local_ref_index = template_op
                        .base
                        .base
                        .local_refs_index
                        .map(|idx| idx.as_usize() as i32);

                    let stmt = ng::template(
                        slot,
                        *o::variable(fn_name),
                        decls,
                        vars,
                        tag,
                        const_index,
                        local_ref_index,
                        template_op.base.base.start_source_span.clone(),
                    );
                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn CreateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
                }
            }
            ir::OpKind::Template => {
                if let Some(template_op) = op.as_any().downcast_ref::<ir::ops::create::TemplateOp>()
                {
                    let slot = template_op
                        .base
                        .base
                        .handle
                        .get_slot()
                        .expect("Expected a slot") as i32;
                    let view_xref = template_op.base.base.xref;
                    let fn_name = view_name_map
                        .get(&view_xref)
                        .expect("Template function name not assigned");

                    let decls = template_op.decls.unwrap_or(0);
                    let vars = template_op.vars.unwrap_or(0);
                    let const_index = template_op
                        .base
                        .base
                        .attributes
                        .map(|idx| idx.as_usize() as i32);
                    let local_ref_index = template_op
                        .base
                        .base
                        .local_refs_index
                        .map(|idx| idx.as_usize() as i32);

                    let stmt = if template_op.template_kind == ir::TemplateKind::NgTemplate {
                        let tag = "ng-template".to_string();
                        // Use ng::template (ɵɵtemplate) when the ng-template has matched directives
                        // (e.g., [ngTemplateOutlet]) OR has local refs (which need templateRefExtractor).
                        // This ensures proper TemplateRef creation and directive matching.
                        if template_op.base.has_directives || local_ref_index.is_some() {
                            ng::template(
                                slot,
                                *o::variable(fn_name),
                                decls,
                                vars,
                                Some(tag),
                                const_index,
                                local_ref_index,
                                template_op.base.base.start_source_span.clone(),
                            )
                        } else {
                            ng::dom_template(
                                slot,
                                *o::variable(fn_name),
                                decls,
                                vars,
                                tag,
                                const_index,
                                local_ref_index,
                                template_op.base.base.start_source_span.clone(),
                            )
                        }
                    } else {
                        let tag = template_op.base.tag.clone().filter(|t| t != "ng-template");
                        ng::template(
                            slot,
                            *o::variable(fn_name),
                            decls,
                            vars,
                            tag,
                            const_index,
                            local_ref_index,
                            template_op.base.base.start_source_span.clone(),
                        )
                    };
                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn CreateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
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
                    let fn_name = view_name_map.get(&view_xref).unwrap_or_else(|| {
                        eprintln!("Available views (named): {:?}", view_name_map.keys());
                        panic!(
                            "Template function name not assigned for view {:?}",
                            view_xref
                        )
                    });

                    let decls = cond_op.decls.unwrap_or(0);
                    let vars = cond_op.vars.unwrap_or(0);
                    let tag = cond_op.base.tag.clone();
                    let const_index = cond_op
                        .base
                        .base
                        .attributes
                        .map(|idx| idx.as_usize() as i32);

                    let stmt = ng::conditional_create(
                        slot,
                        *o::variable(fn_name),
                        decls,
                        vars,
                        tag,
                        const_index,
                        cond_op.base.base.start_source_span.clone(),
                    );
                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn CreateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
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
                    let fn_name = view_name_map
                        .get(&view_xref)
                        .expect("Template function name not assigned");

                    let decls = branch_op.decls.unwrap_or(0);
                    let vars = branch_op.vars.unwrap_or(0);
                    let tag = branch_op.base.tag.clone();
                    let const_index = branch_op
                        .base
                        .base
                        .attributes
                        .map(|idx| idx.as_usize() as i32);

                    let stmt = ng::conditional_create(
                        slot,
                        *o::variable(fn_name),
                        decls,
                        vars,
                        tag,
                        const_index,
                        branch_op.base.base.start_source_span.clone(),
                    );
                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn CreateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
                }
            }
            ir::OpKind::Text => {
                if let Some(text_op) = op.as_any().downcast_ref::<ir::ops::create::TextOp>() {
                    if let Some(slot) = text_op.handle.get_slot() {
                        let stmt = ng::text(
                            slot as i32,
                            text_op.initial_value.clone(),
                            text_op.source_span.clone(),
                        );
                        Some(Box::new(ir::ops::shared::create_statement_op::<
                            Box<dyn CreateOp + Send + Sync>,
                        >(Box::new(stmt))))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            ir::OpKind::Element | ir::OpKind::ElementStart => {
                if let Some(el_op) = op
                    .as_any()
                    .downcast_ref::<ir::ops::create::ElementStartOp>()
                {
                    if let Some(slot) = el_op.base.base.handle.get_slot() {
                        let const_index =
                            el_op.base.base.attributes.map(|idx| idx.as_usize() as i32);
                        let local_ref_index = el_op
                            .base
                            .base
                            .local_refs_index
                            .map(|idx| idx.as_usize() as i32);
                        let tag = el_op.base.tag.clone().unwrap_or_default();
                        // Use domElementStart in DomOnly mode (no directive dependencies)
                        let stmt = if mode == TemplateCompilationMode::DomOnly {
                            ng::dom_element_start(
                                slot as i32,
                                tag,
                                const_index,
                                local_ref_index,
                                el_op.base.base.start_source_span.clone(),
                            )
                        } else {
                            ng::element_start(
                                slot as i32,
                                tag,
                                const_index,
                                local_ref_index,
                                el_op.base.base.start_source_span.clone(),
                            )
                        };
                        Some(Box::new(ir::ops::shared::create_statement_op::<
                            Box<dyn CreateOp + Send + Sync>,
                        >(Box::new(stmt))))
                    } else {
                        None
                    }
                } else if let Some(el_op) = op.as_any().downcast_ref::<ir::ops::create::ElementOp>()
                {
                    if let Some(slot) = el_op.base.base.handle.get_slot() {
                        let const_index =
                            el_op.base.base.attributes.map(|idx| idx.as_usize() as i32);
                        let local_ref_index = el_op
                            .base
                            .base
                            .local_refs_index
                            .map(|idx| idx.as_usize() as i32);
                        let tag = el_op.base.tag.clone().unwrap_or_default();
                        let stmt = ng::element(
                            slot as i32,
                            tag,
                            const_index,
                            local_ref_index,
                            el_op.base.base.start_source_span.clone(),
                        );
                        Some(Box::new(ir::ops::shared::create_statement_op::<
                            Box<dyn CreateOp + Send + Sync>,
                        >(Box::new(stmt))))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            ir::OpKind::ElementEnd => {
                let stmt = if mode == TemplateCompilationMode::DomOnly {
                    ng::dom_element_end(op.source_span().cloned())
                } else {
                    ng::element_end(op.source_span().cloned())
                };
                Some(Box::new(ir::ops::shared::create_statement_op::<
                    Box<dyn CreateOp + Send + Sync>,
                >(Box::new(stmt))))
            }

            ir::OpKind::DisableBindings => {
                let stmt = ng::disable_bindings();
                Some(Box::new(ir::ops::shared::create_statement_op::<
                    Box<dyn CreateOp + Send + Sync>,
                >(Box::new(stmt))))
            }
            ir::OpKind::EnableBindings => {
                let stmt = ng::enable_bindings();
                Some(Box::new(ir::ops::shared::create_statement_op::<
                    Box<dyn CreateOp + Send + Sync>,
                >(Box::new(stmt))))
            }
            ir::OpKind::Variable => {
                if let Some(var_op) = op
                    .as_any()
                    .downcast_ref::<ir::ops::shared::VariableOp<Box<dyn CreateOp + Send + Sync>>>()
                {
                    // Use the name from the naming phase if available, otherwise fall back to XrefId
                    let name = var_op
                        .variable
                        .name()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("_r{}", var_op.xref.0));
                    let value = reify_ir_expression(
                        *var_op.initializer.clone(),
                        ir::VisitorContextFlag::NONE,
                    );
                    let stmt = o::Statement::DeclareVar(o::DeclareVarStmt {
                        name,
                        value: Some(Box::new(value)),
                        type_: None,
                        modifiers: o::StmtModifier::Final,
                        source_span: None,
                    });
                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn CreateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
                }
            }

            ir::OpKind::Listener => {
                // eprintln!("[reify] Found ListenerOp");
                if let Some(listener_op) = op
                    .as_any_mut()
                    .downcast_mut::<ir::ops::create::ListenerOp>()
                {
                    // Reify handler operations
                    let mut handler_stmts = vec![];
                    // eprintln!("[reify] Handler ops count: {}", listener_op.handler_ops.len());

                    for handler_op in &mut listener_op.handler_ops {
                        // First, transform IR expressions
                        ir::transform_expressions_in_op(
                            handler_op.as_mut(),
                            &mut reify_ir_expression,
                            ir::VisitorContextFlag::NONE,
                        );

                        // Handle VariableOp -> DeclareVarStmt conversion
                        if handler_op.kind() == ir::OpKind::Variable {
                            use crate::template::pipeline::ir::ops::shared::VariableOp;
                            use crate::template::pipeline::ir::SemanticVariable;

                            if let Some(var_op) = handler_op
                                .as_any()
                                .downcast_ref::<VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>>()
                            {
                                let var_name = match &var_op.variable {
                                    SemanticVariable::Identifier(ident_var) => {
                                        ident_var.name.clone()
                                    }
                                    SemanticVariable::Context(ctx_var) => ctx_var.name.clone(),
                                    SemanticVariable::Alias(_) => None,
                                    SemanticVariable::SavedView(_) => None,
                                };

                                let reified_initializer = reify_ir_expression(
                                    *var_op.initializer.clone(),
                                    ir::VisitorContextFlag::NONE,
                                );

                                let stmt = if let Some(name) = var_name {
                                    // Normal case: emit as DeclareVarStmt
                                    o::Statement::DeclareVar(o::DeclareVarStmt {
                                        name: name.clone(),
                                        value: Some(Box::new(reified_initializer)),
                                        type_: None,
                                        modifiers: o::StmtModifier::Final,
                                        source_span: None,
                                    })
                                } else if let SemanticVariable::Identifier(ident_var) =
                                    &var_op.variable
                                {
                                    // Identifier case: use identifier name as fallback if no unique name assigned
                                    o::Statement::DeclareVar(o::DeclareVarStmt {
                                        name: ident_var.identifier.clone(),
                                        value: Some(Box::new(reified_initializer)),
                                        type_: None,
                                        modifiers: o::StmtModifier::Final,
                                        source_span: None,
                                    })
                                } else {
                                    // No variable name: emit as expression statement
                                    o::Statement::Expression(o::ExpressionStatement {
                                        expr: Box::new(reified_initializer),
                                        source_span: None,
                                    })
                                };

                                let new_op: Box<dyn ir::UpdateOp + Send + Sync> =
                                    Box::new(ir::ops::shared::create_statement_op::<
                                        Box<dyn ir::UpdateOp + Send + Sync>,
                                    >(Box::new(stmt)));
                                *handler_op = new_op;
                            }
                        }

                        // Collect statements
                        if let Some(stmt_op) = handler_op
                            .as_any()
                            .downcast_ref::<ir::ops::shared::StatementOp<Box<dyn ir::UpdateOp + Send + Sync>>>()
                        {
                            handler_stmts.push(*stmt_op.statement.clone());
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

                    // Create listener statement
                    let stmt = ng::listener(
                        listener_op.name.to_string(),
                        handler_fn,
                        listener_op.event_target.clone(),
                        listener_op.source_span.clone().into(),
                    );

                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn CreateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
                }
            }
            ir::OpKind::TwoWayListener => {
                if let Some(listener_op) = op
                    .as_any_mut()
                    .downcast_mut::<ir::ops::create::TwoWayListenerOp>()
                {
                    // Reify handler operations
                    let mut handler_stmts = vec![];

                    for handler_op in &mut listener_op.handler_ops {
                        // First, transform IR expressions
                        ir::transform_expressions_in_op(
                            handler_op.as_mut(),
                            &mut reify_ir_expression,
                            ir::VisitorContextFlag::NONE,
                        );

                        // Handle VariableOp -> DeclareVarStmt conversion
                        if handler_op.kind() == ir::OpKind::Variable {
                            use crate::template::pipeline::ir::ops::shared::VariableOp;
                            use crate::template::pipeline::ir::SemanticVariable;

                            if let Some(var_op) = handler_op
                                .as_any()
                                .downcast_ref::<VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>>()
                            {
                                let var_name = match &var_op.variable {
                                    SemanticVariable::Identifier(ident_var) => {
                                        ident_var.name.clone()
                                    }
                                    SemanticVariable::Context(ctx_var) => ctx_var.name.clone(),
                                    SemanticVariable::Alias(_) => None,
                                    SemanticVariable::SavedView(_) => None,
                                };

                                let reified_initializer = reify_ir_expression(
                                    *var_op.initializer.clone(),
                                    ir::VisitorContextFlag::NONE,
                                );

                                let stmt = if let Some(name) = var_name {
                                    // Normal case: emit as DeclareVarStmt
                                    o::Statement::DeclareVar(o::DeclareVarStmt {
                                        name: name.clone(),
                                        value: Some(Box::new(reified_initializer)),
                                        type_: None,
                                        modifiers: o::StmtModifier::Final,
                                        source_span: None,
                                    })
                                } else {
                                    // No variable name: emit as expression statement
                                    o::Statement::Expression(o::ExpressionStatement {
                                        expr: Box::new(reified_initializer),
                                        source_span: None,
                                    })
                                };

                                let new_op: Box<dyn ir::UpdateOp + Send + Sync> =
                                    Box::new(ir::ops::shared::create_statement_op::<
                                        Box<dyn ir::UpdateOp + Send + Sync>,
                                    >(Box::new(stmt)));
                                *handler_op = new_op;
                            }
                        }

                        // Collect statements
                        if let Some(stmt_op) = handler_op
                            .as_any()
                            .downcast_ref::<ir::ops::shared::StatementOp<Box<dyn ir::UpdateOp + Send + Sync>>>()
                        {
                            handler_stmts.push(*stmt_op.statement.clone());
                        }
                    }

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

                    // Create listener statement
                    let stmt = ng::two_way_listener(
                        listener_op.name.to_string(),
                        handler_fn,
                        false, // Default prevent_default
                        listener_op.source_span.clone().into(),
                    );

                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn CreateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(new) = new_op {
            *op = new;
        }
    }
}

fn reify_update_operations(unit: &mut dyn CompilationUnit) {
    for op in unit.update_mut().iter_mut() {
        ir::transform_expressions_in_op(
            op.as_mut(),
            &mut reify_ir_expression,
            ir::VisitorContextFlag::NONE,
        );

        let new_op: Option<Box<dyn UpdateOp + Send + Sync>> = match op.kind() {
            ir::OpKind::Advance => {
                if let Some(adv) = op.as_any().downcast_ref::<ir::ops::update::AdvanceOp>() {
                    let stmt = ng::advance(adv.delta as i32, adv.source_span.clone());
                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn UpdateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
                }
            }
            ir::OpKind::Property => {
                if let Some(prop) = op.as_any().downcast_ref::<ir::ops::update::PropertyOp>() {
                    if let ir::ops::update::BindingExpression::Expression(expr) = &prop.expression {
                        let stmt = ng::property(
                            prop.name.clone(),
                            expr.clone(),
                            prop.sanitizer.clone(),
                            prop.source_span.clone(),
                        );
                        Some(Box::new(ir::ops::shared::create_statement_op::<
                            Box<dyn UpdateOp + Send + Sync>,
                        >(Box::new(stmt))))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            ir::OpKind::TwoWayProperty => {
                if let Some(prop) = op
                    .as_any()
                    .downcast_ref::<ir::ops::update::TwoWayPropertyOp>()
                {
                    let stmt = ng::two_way_property(
                        prop.name.clone(),
                        prop.expression.clone(),
                        prop.sanitizer.clone(),
                        prop.source_span.clone(),
                    );
                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn UpdateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
                }
            }
            ir::OpKind::Variable => {
                use crate::template::pipeline::ir::ops::shared::VariableOp;
                use crate::template::pipeline::ir::SemanticVariable;
                if let Some(var_op) = op
                    .as_any()
                    .downcast_ref::<VariableOp<Box<dyn UpdateOp + Send + Sync>>>()
                {
                    // Get variable name from SemanticVariable
                    let var_name = match &var_op.variable {
                        SemanticVariable::Identifier(ident_var) => ident_var.name.clone(),
                        SemanticVariable::Context(ctx_var) => ctx_var.name.clone(),
                        SemanticVariable::Alias(_) => None, // Aliases are always inlined, shouldn't reach here
                        SemanticVariable::SavedView(_) => None, // SavedView variables don't need declarations here
                    };

                    if let Some(name) = var_name {
                        // Reify initializer expression before using it
                        let reified_initializer = reify_ir_expression(
                            *var_op.initializer.clone(),
                            ir::VisitorContextFlag::NONE,
                        );

                        // Convert VariableOp to DeclareVarStmt
                        let stmt = o::Statement::DeclareVar(o::DeclareVarStmt {
                            name,
                            value: Some(Box::new(reified_initializer)),
                            type_: None,
                            modifiers: o::StmtModifier::Final,
                            source_span: op.source_span().cloned(),
                        });
                        Some(Box::new(ir::ops::shared::create_statement_op::<
                            Box<dyn UpdateOp + Send + Sync>,
                        >(Box::new(stmt))))
                    } else {
                        // Variable name not yet assigned - this should not happen after naming phase
                        None
                    }
                } else {
                    None
                }
            }
            ir::OpKind::ClassProp => {
                if let Some(class_op) = op.as_any().downcast_ref::<ir::ops::update::ClassPropOp>() {
                    // Reify the expression before using it
                    let reified_expr = reify_ir_expression(
                        class_op.expression.clone(),
                        ir::VisitorContextFlag::NONE,
                    );
                    let stmt = ng::class_prop(
                        class_op.name.clone(),
                        reified_expr,
                        Some(class_op.source_span.clone()),
                    );
                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn UpdateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
                }
            }
            ir::OpKind::StyleProp => {
                if let Some(style_op) = op.as_any().downcast_ref::<ir::ops::update::StylePropOp>() {
                    let expression = match &style_op.expression {
                        ir::ops::update::BindingExpression::Expression(expr) => {
                            // Reify the expression

                            reify_ir_expression(expr.clone(), ir::VisitorContextFlag::NONE)
                        }
                        ir::ops::update::BindingExpression::Interpolation(interp) => {
                            // Convert interpolation to string for now
                            o::Expression::Literal(o::LiteralExpr {
                                value: o::LiteralValue::String(interp.strings.join("")),
                                type_: None,
                                source_span: None,
                            })
                        }
                    };
                    let stmt = ng::style_prop(
                        style_op.name.clone(),
                        expression,
                        style_op.unit.clone(),
                        Some(style_op.source_span.clone()),
                    );
                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn UpdateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
                }
            }
            ir::OpKind::Attribute => {
                if let Some(attr) = op.as_any().downcast_ref::<ir::ops::update::AttributeOp>() {
                    let expression = match &attr.expression {
                        ir::ops::update::BindingExpression::Expression(expr) => {
                            // Reify the expression
                            reify_ir_expression(expr.clone(), ir::VisitorContextFlag::NONE)
                        }
                        ir::ops::update::BindingExpression::Interpolation(interp) => {
                            // Convert interpolation to string for now - similar to StyleProp
                            o::Expression::Literal(o::LiteralExpr {
                                value: o::LiteralValue::String(interp.strings.join("")),
                                type_: None,
                                source_span: None,
                            })
                        }
                    };
                    let stmt = ng::attribute(
                        attr.name.clone(),
                        expression,
                        attr.sanitizer.clone(),
                        attr.source_span.clone(),
                    );
                    Some(Box::new(ir::ops::shared::create_statement_op::<
                        Box<dyn UpdateOp + Send + Sync>,
                    >(Box::new(stmt))))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(new) = new_op {
            *op = new;
        }
    }
}

fn reify_ir_expression(expr: o::Expression, flags: ir::VisitorContextFlag) -> o::Expression {
    match expr {
        o::Expression::LiteralArray(arr) => {
            return o::Expression::LiteralArray(o::LiteralArrayExpr {
                entries: arr
                    .entries
                    .into_iter()
                    .map(|e| reify_ir_expression(e, flags))
                    .collect(),
                type_: arr.type_,
                source_span: arr.source_span,
            });
        }
        o::Expression::LiteralMap(map) => {
            return o::Expression::LiteralMap(o::LiteralMapExpr {
                entries: map
                    .entries
                    .into_iter()
                    .map(|e| o::LiteralMapEntry {
                        key: e.key,
                        value: Box::new(reify_ir_expression(*e.value, flags)),
                        quoted: e.quoted,
                    })
                    .collect(),
                type_: map.type_,
                source_span: map.source_span,
            });
        }
        o::Expression::BinaryOp(op) => {
            return o::Expression::BinaryOp(o::BinaryOperatorExpr {
                lhs: Box::new(reify_ir_expression(*op.lhs, flags)),
                rhs: Box::new(reify_ir_expression(*op.rhs, flags)),
                operator: op.operator,
                type_: op.type_,
                source_span: op.source_span,
            });
        }
        o::Expression::InvokeFn(func) => {
            return o::Expression::InvokeFn(o::InvokeFunctionExpr {
                fn_: Box::new(reify_ir_expression(*func.fn_, flags)),
                args: func
                    .args
                    .into_iter()
                    .map(|a| reify_ir_expression(a, flags))
                    .collect(),
                type_: func.type_,
                source_span: func.source_span,
                pure: func.pure,
            });
        }
        o::Expression::ReadProp(prop) => {
            return o::Expression::ReadProp(o::ReadPropExpr {
                receiver: Box::new(reify_ir_expression(*prop.receiver, flags)),
                name: prop.name,
                type_: prop.type_,
                source_span: prop.source_span,
            });
        }
        o::Expression::Conditional(cond) => {
            return o::Expression::Conditional(o::ConditionalExpr {
                condition: Box::new(reify_ir_expression(*cond.condition, flags)),
                true_case: Box::new(reify_ir_expression(*cond.true_case, flags)),
                false_case: cond
                    .false_case
                    .map(|f| Box::new(reify_ir_expression(*f, flags))),
                type_: cond.type_,
                source_span: cond.source_span,
            });
        }
        o::Expression::NotExpr(not) => {
            return o::Expression::NotExpr(o::NotExpr {
                condition: Box::new(reify_ir_expression(*not.condition, flags)),
                source_span: not.source_span,
            });
        }
        o::Expression::Parens(paren) => {
            return o::Expression::Parens(o::ParenthesizedExpr {
                expr: Box::new(reify_ir_expression(*paren.expr, flags)),
                type_: paren.type_,
                source_span: paren.source_span,
            });
        }
        _ => {}
    }

    if !ir::is_ir_expression(&expr) {
        return expr;
    }

    match &expr {
        o::Expression::PureFunctionParameter(param) => o::Expression::ReadVar(o::ReadVarExpr {
            name: format!("_p{}", param.index),
            type_: None,
            source_span: param.source_span.clone(),
        }),
        o::Expression::PureFunction(pf) => {
            let func = if let Some(fn_) = &pf.fn_ {
                reify_ir_expression(*fn_.clone(), flags)
            } else if let Some(body) = &pf.body {
                let params: Vec<o::FnParam> = (0..pf.args.len())
                    .map(|i| o::FnParam {
                        name: format!("_p{}", i),
                        type_: None,
                    })
                    .collect();

                let reified_body = reify_ir_expression(*body.clone(), flags);

                o::Expression::ArrowFn(o::ArrowFunctionExpr {
                    params,
                    body: o::ArrowFunctionBody::Expression(Box::new(reified_body)),
                    type_: None,
                    source_span: None,
                })
            } else {
                *o::literal(o::LiteralValue::Null)
            };

            let args: Vec<o::Expression> = pf
                .args
                .iter()
                .map(|arg| reify_ir_expression(arg.clone(), flags))
                .collect();
            let slot = pf.var_offset.unwrap_or(0) as i32;
            ng::pure_function(slot, func, args)
        }
        o::Expression::ReadVariable(var) => {
            // Get name from variable, or generate fallback if missing
            let name = var.name.clone().unwrap_or_else(|| {
                // Fallback: generate variable name from xref
                format!("_r{}", var.xref.0)
            });
            o::Expression::ReadVar(o::ReadVarExpr {
                name,
                type_: None,
                source_span: var.source_span.clone(),
            })
        }
        o::Expression::ReadVar(expr) => o::Expression::ReadVar(expr.clone()),
        o::Expression::NextContext(next_ctx) => {
            // Reify NextContextExpr to ng.nextContext(steps) expression
            // If steps == 1, call with empty args, otherwise pass steps as argument
            let args = if next_ctx.steps == 1 {
                vec![]
            } else {
                vec![*o::literal(next_ctx.steps as f64)]
            };
            *o::import_ref(crate::render3::r3_identifiers::Identifiers::next_context()).call_fn(
                args,
                next_ctx.source_span.clone(),
                None,
            )
        }
        o::Expression::PipeBinding(pipe) => {
            // Reify PipeBindingExpr to ɵɵpipeBind call
            let reified_args: Vec<o::Expression> = pipe
                .args
                .iter()
                .map(|arg| reify_ir_expression(arg.clone(), flags))
                .collect();

            let pipe_slot = pipe.target_slot.get_slot().unwrap_or(0) as i32;
            let var_offset = pipe.var_offset.unwrap_or(0) as i32;
            ng::pipe_bind(pipe_slot, var_offset, reified_args)
        }
        o::Expression::PipeBindingVariadic(pipe) => {
            // Reify PipeBindingVariadicExpr to ɵɵpipeBindV call
            let reified_args = reify_ir_expression(*pipe.args.clone(), flags);
            let pipe_slot = pipe.target_slot.get_slot().unwrap_or(0) as i32;
            let var_offset = pipe.var_offset.unwrap_or(0) as i32;

            // For variadic, wrap in array and call pipeBindV
            let args_array = vec![reified_args];
            ng::pipe_bind(pipe_slot, var_offset, args_array)
        }
        o::Expression::Reference(ref_expr) => {
            // Reify ReferenceExpr to ɵɵreference(slot + 1 + offset) expression
            // The +1 is because the reference is stored in the slot AFTER the element/template
            // that declares it. For example, if a template is at slot 16, its template ref
            // is at slot 17 (slot + 1).
            let slot = ref_expr.target_slot.get_slot().unwrap_or(0) as i32;
            let offset = ref_expr.offset as i32;
            ng::reference(slot + 1 + offset)
        }
        o::Expression::ContextLetReference(let_ref) => {
            // Reify ContextLetReferenceExpr to ɵɵstoreLet(slot) expression
            // This reads the stored @let value
            let slot = let_ref.target_slot.get_slot().unwrap_or(0) as i32;
            ng::reference(slot)
        }
        o::Expression::GetCurrentView(_) => *o::import_ref(
            crate::render3::r3_identifiers::Identifiers::get_current_view(),
        )
        .call_fn(vec![], None, None),
        o::Expression::RestoreView(restore_view) => {
            let view_arg = match &restore_view.view {
                ir::expression::EitherXrefIdOrExpression::XrefId(xref) => {
                    // If it's an xref, we need to read the variable associated with it?
                    // Or is it a variable read?
                    // Typically RestoreView takes a view instance.
                    // If we have an xref, it usually means a variable holding the view.
                    // We should generate a ReadVar expression for that xref.
                    o::Expression::ReadVar(o::ReadVarExpr {
                        name: format!("_r{}", xref.0), // Fallback naming, should ideally resolve name
                        type_: None,
                        source_span: None,
                    })
                }
                ir::expression::EitherXrefIdOrExpression::Expression(expr) => {
                    reify_ir_expression(*expr.clone(), flags)
                }
            };
            *o::import_ref(crate::render3::r3_identifiers::Identifiers::restore_view()).call_fn(
                vec![view_arg],
                None,
                None,
            )
        }
        o::Expression::ResetView(reset_view) => {
            let expr_arg = reify_ir_expression(*reset_view.expr.clone(), flags);
            *o::import_ref(crate::render3::r3_identifiers::Identifiers::reset_view()).call_fn(
                vec![expr_arg],
                None,
                None,
            )
        }
        o::Expression::Context(ctx) => {
            // Reify ContextExpr to ReadVariable (which resolves to _rX)
            reify_ir_expression(
                o::Expression::ReadVariable(ir::expression::ReadVariableExpr {
                    xref: ctx.view,
                    name: None,
                    source_span: ctx.source_span.clone(),
                }),
                flags,
            )
        }
        o::Expression::SlotLiteral(slot_lit) => {
            // Reify SlotLiteralExpr to a numeric literal representing the slot index
            let slot_value = slot_lit.slot.get_slot().unwrap_or(0);
            *o::literal(slot_value as f64)
        }
        o::Expression::AssignTemporary(temp) => {
            let name = temp
                .name
                .clone()
                .unwrap_or_else(|| format!("_t{}", temp.xref.0));
            o::Expression::WriteVar(o::WriteVarExpr {
                name,
                value: Box::new(reify_ir_expression(*temp.expr.clone(), flags)),
                type_: None,
                source_span: temp.source_span.clone(),
            })
        }
        o::Expression::ReadTemporary(temp) => {
            let name = temp
                .name
                .clone()
                .unwrap_or_else(|| format!("_t{}", temp.xref.0));
            o::Expression::ReadVar(o::ReadVarExpr {
                name,
                type_: None,
                source_span: temp.source_span.clone(),
            })
        }
        _ => expr,
    }
}
