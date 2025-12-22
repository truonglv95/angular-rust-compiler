//! Resolve Contexts Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/resolve_contexts.ts
//! Resolves `ir.ContextExpr` expressions (which represent embedded view or component contexts) to
//! either the `ctx` parameter to component functions (for the current view context) or to variables
//! that store those contexts (for contexts accessed via the `nextContext()` instruction).

use crate::template::pipeline::ir as ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::variable::SemanticVariableKind;
use crate::template::pipeline::ir::expression::transform_expressions_in_op;
use crate::template::pipeline::ir::expression::ReadVariableExpr;
use crate::template::pipeline::src::compilation::{CompilationJob, ComponentCompilationJob, CompilationJobKind, CompilationUnit};
use crate::output::output_ast::Expression;
use crate::template::pipeline::ir::ops::create::{ListenerOp, AnimationListenerOp, TwoWayListenerOp, RepeaterCreateOp};
use crate::template::pipeline::ir::ops::shared::VariableOp;

/// Resolves `ir.ContextExpr` expressions (which represent embedded view or component contexts) to
/// either the `ctx` parameter to component functions (for the current view context) or to variables
/// that store those contexts (for contexts accessed via the `nextContext()` instruction).
pub fn phase(job: &mut dyn CompilationJob) {
    let job_kind = job.kind();
    
    if matches!(job_kind, CompilationJobKind::Tmpl | CompilationJobKind::Both) {
        let component_job = unsafe {
            let job_ptr = job as *mut dyn CompilationJob;
            let job_ptr = job_ptr as *mut ComponentCompilationJob;
            &mut *job_ptr
        };
        
        // Process root unit
        let root_xref = component_job.root.xref();
        process_lexical_scope(&mut component_job.root, root_xref);
        
        // Process all view units
        let view_keys: Vec<_> = component_job.views.keys().cloned().collect();
        for key in view_keys {
            if let Some(unit) = component_job.views.get_mut(&key) {
                let unit_xref = unit.xref();
                process_lexical_scope(unit, unit_xref);
            }
        }
    }
}

fn process_lexical_scope(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    root_xref: ir::XrefId,
) {
    // Track the expressions used to access all available contexts within the current view, by the
    // view `ir.XrefId`.
    let mut scope: std::collections::HashMap<ir::XrefId, Expression> = std::collections::HashMap::new();
    
    let unit_xref = unit.xref();
    
    // The current view's context is accessible via the `ctx` parameter.
    scope.insert(unit_xref, Expression::ReadVar(crate::output::output_ast::ReadVarExpr {
        name: "ctx".to_string(),
        type_: None,
        source_span: None,
    }));
    
    // First pass: build scope by processing VariableOps
    for op in unit.create().iter() {
        if op.kind() == OpKind::Variable {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let variable_op_ptr = op_ptr as *const VariableOp<Box<dyn ir::CreateOp + Send + Sync>>;
                let variable_op = &*variable_op_ptr;
                
                if matches!(variable_op.variable.kind(), SemanticVariableKind::Context) {
                    // Get the view from the variable
                    if let ir::SemanticVariable::Context(ctx_var) = &variable_op.variable {
                        scope.insert(ctx_var.view, Expression::ReadVariable(ReadVariableExpr {
                            xref: variable_op.xref,
                            name: None,
                            source_span: None,
                        }));
                    }
                }
            }
        }
    }
    
    for op in unit.update().iter() {
        if op.kind() == OpKind::Variable {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                let variable_op_ptr = op_ptr as *const VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>;
                let variable_op = &*variable_op_ptr;
                
                if matches!(variable_op.variable.kind(), SemanticVariableKind::Context) {
                    if let ir::SemanticVariable::Context(ctx_var) = &variable_op.variable {
                        scope.insert(ctx_var.view, Expression::ReadVariable(ReadVariableExpr {
                            xref: variable_op.xref,
                            name: None,
                            source_span: None,
                        }));
                    }
                }
            }
        }
    }
    
    // Prefer `ctx` of the root view to any variables which happen to contain the root context.
    if unit_xref == root_xref {
        scope.insert(unit_xref, Expression::ReadVar(crate::output::output_ast::ReadVarExpr {
            name: "ctx".to_string(),
            type_: None,
            source_span: None,
        }));
    }
    
    // Second pass: transform ContextExpr using scope
    // Process create ops
    for op in unit.create_mut().iter_mut() {
        unsafe {
            let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
            let op_ptr = op_ptr as *mut dyn ir::Op;
            transform_context_exprs_in_op(&mut *op_ptr, &scope);
        }
    }
    
    // Process update ops
    for op in unit.update_mut().iter_mut() {
        transform_context_exprs_in_op(op.as_mut(), &scope);
    }
    
    // Recursively process nested scopes (listeners, trackByOps)
    for op in unit.create_mut().iter_mut() {
        match op.kind() {
            OpKind::Listener | OpKind::AnimationListener | OpKind::TwoWayListener => {
                process_listener_scope(op, &scope);
            }
            OpKind::RepeaterCreate => {
                process_repeater_scope(op, &scope);
            }
            _ => {}
        }
    }
}

fn transform_context_exprs_in_op(
    op: &mut dyn ir::Op,
    scope: &std::collections::HashMap<ir::XrefId, Expression>,
) {
    transform_expressions_in_op(
        op,
        &mut |expr, _flags| {
            if let Expression::Context(ref ctx_expr) = expr {
                let view_xref = ctx_expr.view;
                if let Some(replacement) = scope.get(&view_xref) {
                    return replacement.clone();
                } else {
                    panic!("No context found for reference to view {:?} from current view", view_xref);
                }
            }
            expr
        },
        ir::VisitorContextFlag::NONE,
    );
}

fn process_listener_scope(
    op: &mut Box<dyn ir::CreateOp + Send + Sync>,
    parent_scope: &std::collections::HashMap<ir::XrefId, Expression>,
) {
    // For listeners, we process handler_ops in their own scope
    // But they can still access parent scope contexts
    match op.kind() {
        OpKind::Listener => {
            unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let listener_ptr = op_ptr as *mut ListenerOp;
                let listener = &mut *listener_ptr;
                
                for handler_op in listener.handler_ops.iter_mut() {
                    transform_context_exprs_in_op(handler_op.as_mut(), parent_scope);
                }
            }
        }
        OpKind::AnimationListener => {
            unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let animation_listener_ptr = op_ptr as *mut AnimationListenerOp;
                let animation_listener = &mut *animation_listener_ptr;
                
                for handler_op in animation_listener.handler_ops.iter_mut() {
                    transform_context_exprs_in_op(handler_op.as_mut(), parent_scope);
                }
            }
        }
        OpKind::TwoWayListener => {
            unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let two_way_listener_ptr = op_ptr as *mut TwoWayListenerOp;
                let two_way_listener = &mut *two_way_listener_ptr;
                
                for handler_op in two_way_listener.handler_ops.iter_mut() {
                    transform_context_exprs_in_op(handler_op.as_mut(), parent_scope);
                }
            }
        }
        _ => {}
    }
}

fn process_repeater_scope(
    op: &mut Box<dyn ir::CreateOp + Send + Sync>,
    parent_scope: &std::collections::HashMap<ir::XrefId, Expression>,
) {
    unsafe {
        let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
        let repeater_ptr = op_ptr as *mut RepeaterCreateOp;
        let repeater = &mut *repeater_ptr;
        
        if let Some(ref mut track_by_ops) = repeater.track_by_ops {
            for track_op in track_by_ops.iter_mut() {
                transform_context_exprs_in_op(track_op.as_mut(), parent_scope);
            }
        }
    }
}
