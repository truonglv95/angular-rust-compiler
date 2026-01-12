//! Merge sequential NextContextExpr operations.
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/next_context_merging.ts
//!
//! Merges logically sequential `NextContextExpr` operations.
//! `NextContextExpr` can be referenced repeatedly, "popping" the runtime's context stack each time.
//! When two such expressions appear back-to-back, it's possible to merge them together into a single
//! `NextContextExpr` that steps multiple contexts. This merging is possible if all conditions are met:
//!
//!   * The result of the `NextContextExpr` that's folded into the subsequent one is not stored (that
//!     is, the call is purely side-effectful).
//!   * No operations in between them uses the implicit context.

use crate::output::output_ast::{Expression, Statement};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::expression::{
    as_ir_expression, is_ir_expression, transform_expressions_in_op, VisitorContextFlag,
};
use crate::template::pipeline::ir::ops::create::{
    AnimationListenerOp, AnimationOp, ListenerOp, TwoWayListenerOp,
};
use crate::template::pipeline::ir::ops::shared::StatementOp;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob,
};

pub fn merge_next_context_expressions(job: &mut dyn CompilationJob) {
    eprintln!(
        "[MERGE_CTX_ENTRY] merge_next_context_expressions called, job kind: {:?}",
        job.kind()
    );
    if let Some(component_job) = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        if job.kind() == crate::template::pipeline::src::compilation::CompilationJobKind::Tmpl {
            eprintln!("[MERGE_CTX_ENTRY] Job is Tmpl, processing component");
            Some(&mut *(job_ptr as *mut ComponentCompilationJob))
        } else {
            eprintln!("[MERGE_CTX_ENTRY] Job is NOT Tmpl, skipping");
            None
        }
    } {
        // Process handler ops in create operations
        for op in component_job.root.create_mut().iter_mut() {
            match op.kind() {
                ir::OpKind::Listener => unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let listener_ptr = op_ptr as *mut ListenerOp;
                    merge_next_contexts_in_ops(&mut (*listener_ptr).handler_ops);
                },
                ir::OpKind::TwoWayListener => unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let two_way_listener_ptr = op_ptr as *mut TwoWayListenerOp;
                    merge_next_contexts_in_ops(&mut (*two_way_listener_ptr).handler_ops);
                },
                ir::OpKind::AnimationListener => unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let anim_listener_ptr = op_ptr as *mut AnimationListenerOp;
                    merge_next_contexts_in_ops(&mut (*anim_listener_ptr).handler_ops);
                },
                ir::OpKind::Animation => unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let anim_ptr = op_ptr as *mut AnimationOp;
                    merge_next_contexts_in_ops(&mut (*anim_ptr).handler_ops);
                },
                _ => {}
            }
        }

        for view in component_job.views.values_mut() {
            for op in view.create_mut().iter_mut() {
                match op.kind() {
                    ir::OpKind::Listener => unsafe {
                        let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                        let listener_ptr = op_ptr as *mut ListenerOp;
                        merge_next_contexts_in_ops(&mut (*listener_ptr).handler_ops);
                    },
                    ir::OpKind::TwoWayListener => unsafe {
                        let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                        let two_way_listener_ptr = op_ptr as *mut TwoWayListenerOp;
                        merge_next_contexts_in_ops(&mut (*two_way_listener_ptr).handler_ops);
                    },
                    ir::OpKind::AnimationListener => unsafe {
                        let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                        let anim_listener_ptr = op_ptr as *mut AnimationListenerOp;
                        merge_next_contexts_in_ops(&mut (*anim_listener_ptr).handler_ops);
                    },
                    ir::OpKind::Animation => unsafe {
                        let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                        let anim_ptr = op_ptr as *mut AnimationOp;
                        merge_next_contexts_in_ops(&mut (*anim_ptr).handler_ops);
                    },
                    _ => {}
                }
            }
        }

        // Process update operations in root
        merge_next_contexts_in_ops(component_job.root.update_mut());

        eprintln!("[MERGE_CTX] Views count: {}", component_job.views.len());

        // Process update operations in embedded views
        for (view_xref, view) in component_job.views.iter_mut() {
            eprintln!(
                "[MERGE_CTX] Processing View {:?} with {} ops",
                view_xref,
                view.update().len()
            );
            merge_next_contexts_in_ops(view.update_mut());
        }
    }
}

fn merge_next_contexts_in_ops(ops: &mut ir::OpList<Box<dyn ir::UpdateOp + Send + Sync>>) {
    let mut indices_to_remove = Vec::new();
    let mut candidate_info = Vec::new();

    eprintln!(
        "[MERGE_CTX] Starting merge_next_contexts_in_ops with {} ops",
        ops.len()
    );

    // First pass: collect candidate operations (StatementOp with NextContextExpr)
    for (idx, op) in ops.iter().enumerate() {
        eprintln!("[MERGE_CTX] Checking op {} kind: {:?}", idx, op.kind());

        unsafe {
            let op_ptr = op.as_ref() as *const dyn ir::Op;

            if op.kind() == ir::OpKind::Statement {
                // Use the type matching the op list: StatementOp<Box<dyn ir::UpdateOp...>>
                let stmt_op_ptr = op_ptr as *const StatementOp<Box<dyn ir::UpdateOp + Send + Sync>>;
                let stmt_op = &*stmt_op_ptr;

                if let Statement::Expression(ref expr_stmt) = *stmt_op.statement {
                    if let Some(ir_expr) = as_ir_expression(&expr_stmt.expr) {
                        if let ir::IRExpression::NextContext(ref next_ctx) = ir_expr {
                            eprintln!(
                                "[MERGE_CTX] Found NextContext (Statement) at idx {} steps: {}",
                                idx, next_ctx.steps
                            );
                            candidate_info.push((idx, next_ctx.steps));
                        }
                    }
                }
            } else if op.kind() == ir::OpKind::Variable {
                // Use the type matching the op list: VariableOp<Box<dyn ir::UpdateOp...>>
                let var_op_ptr =
                    op_ptr as *const ir::ops::VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>;
                let var_op = &*var_op_ptr;

                if let Some(ir_expr) = as_ir_expression(&var_op.initializer) {
                    match ir_expr {
                        ir::IRExpression::NextContext(ref next_ctx) => {
                            eprintln!(
                                "[MERGE_CTX] Found NextContext (Variable) at idx {} steps: {}",
                                idx, next_ctx.steps
                            );
                            candidate_info.push((idx, next_ctx.steps));
                        }
                        _ => {
                            eprintln!("[MERGE_CTX] Variable op idx {} has IR expr but not NextContext: {:?}", idx, ir_expr);
                        }
                    }
                } else {
                    eprintln!(
                        "[MERGE_CTX] Variable op idx {} initializer is NOT an IR expr. It is: {:?}",
                        idx, var_op.initializer
                    );
                }
            }
        }
    }

    // Second pass: try to merge each candidate with subsequent operations
    for (op_idx, _) in candidate_info {
        if indices_to_remove.contains(&op_idx) {
            continue; // Already merged
        }

        let is_variable = ops.get(op_idx).unwrap().kind() == ir::OpKind::Variable;

        if is_variable {
            // "Absorption" strategy for Variables:
            // A Variable cannot be removed, so we must suck subsequent statements INTO the variable.
            // We can absorb multiple subsequent statements until we hit a blocker or another Variable.
            let mut absorbed_steps = 0;

            for candidate_idx in (op_idx + 1)..ops.len() {
                if indices_to_remove.contains(&candidate_idx) {
                    continue;
                }

                let candidate_op_mut = ops.get_mut(candidate_idx).unwrap();

                // Stop if Candidate is not a Statement (cannot absorb variables or other things)
                if candidate_op_mut.kind() != ir::OpKind::Statement {
                    break;
                }

                let mut has_blocking = false;
                let mut found_next_context_steps = 0;

                transform_expressions_in_op(
                    candidate_op_mut.as_mut(),
                    &mut |expr: Expression, flags| {
                        if flags.contains(VisitorContextFlag::IN_CHILD_OPERATION) {
                            has_blocking = true;
                            return expr;
                        }
                        if is_ir_expression(&expr) {
                            if let Some(ir_expr) = as_ir_expression(&expr) {
                                match ir_expr {
                                    ir::IRExpression::NextContext(ref next_ctx) => {
                                        found_next_context_steps = next_ctx.steps;
                                    }
                                    ir::IRExpression::GetCurrentView(_)
                                    | ir::IRExpression::Reference(_)
                                    | ir::IRExpression::ContextLetReference(_) => {
                                        has_blocking = true;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        expr
                    },
                    VisitorContextFlag::NONE,
                );

                if has_blocking {
                    break;
                }

                if found_next_context_steps > 0 {
                    // Absorb this statement
                    absorbed_steps += found_next_context_steps;
                    indices_to_remove.push(candidate_idx);
                } else {
                    // Statement without NextContext - implies gap or other instruction.
                    // If strict contiguous (no gaps allowed), break.
                    // If gaps allowed, we'd need checks. Original code checked blocking.
                    // Assuming unrelated statement acts as blocker or ignored?
                    // Original code: "If has_blocking... break".
                    // But if it's benign statement?
                    // Original code iterated until it found target.
                    // But for absorption, order matters. We absorb strictly following?
                    // We'll break for safety if we see non-NC statement to avoid reordering issues.
                    // Actually, if it's not NextContext, we treat it like a gap. Break or Skip?
                    // Safer to Break.
                    break;
                }
            }

            if absorbed_steps > 0 {
                // Update the VariableOp
                let op_mut = ops.get_mut(op_idx).unwrap();
                transform_expressions_in_op(
                    op_mut.as_mut(),
                    &mut |mut expr: Expression, flags| {
                        if flags.contains(VisitorContextFlag::IN_CHILD_OPERATION) {
                            return expr;
                        }
                        if let Expression::NextContext(ref mut next_ctx) = expr {
                            next_ctx.steps += absorbed_steps;
                        }
                        expr
                    },
                    VisitorContextFlag::NONE,
                );
            }
        } else {
            // "Forward Merge" strategy for Statements (Existing Logic):
            // Merge current statement INTO the next target. Remove current.
            let mut found_merge_target: Option<usize> = None;
            let mut can_merge = true;

            // Re-fetch current steps because previous merges might have updated this op
            let current_merge_steps = get_next_context_steps(ops.get(op_idx).unwrap()).unwrap_or(0);
            if current_merge_steps == 0 {
                continue;
            }

            for candidate_idx in (op_idx + 1)..ops.len() {
                if !can_merge {
                    break;
                }
                if indices_to_remove.contains(&candidate_idx) {
                    continue;
                }

                let candidate_op_mut = ops.get_mut(candidate_idx).unwrap();
                let mut has_blocking = false;
                let mut has_next_context = false;

                transform_expressions_in_op(
                    candidate_op_mut.as_mut(),
                    &mut |expr: Expression, flags| {
                        if flags.contains(VisitorContextFlag::IN_CHILD_OPERATION) {
                            has_blocking = true;
                            return expr;
                        }
                        if is_ir_expression(&expr) {
                            if let Some(ir_expr) = as_ir_expression(&expr) {
                                match ir_expr {
                                    ir::IRExpression::NextContext(_) => {
                                        has_next_context = true;
                                    }
                                    ir::IRExpression::GetCurrentView(_)
                                    | ir::IRExpression::Reference(_)
                                    | ir::IRExpression::ContextLetReference(_) => {
                                        has_blocking = true;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        expr
                    },
                    VisitorContextFlag::NONE,
                );

                if has_blocking {
                    can_merge = false;
                    break;
                }

                if has_next_context {
                    // Merge INTO candidate
                    transform_expressions_in_op(
                        candidate_op_mut.as_mut(),
                        &mut |mut expr: Expression, flags| {
                            if flags.contains(VisitorContextFlag::IN_CHILD_OPERATION) {
                                return expr;
                            }
                            if let Expression::NextContext(ref mut next_ctx) = expr {
                                next_ctx.steps += current_merge_steps;
                                found_merge_target = Some(candidate_idx);
                            }
                            expr
                        },
                        VisitorContextFlag::NONE,
                    );
                    if found_merge_target.is_some() {
                        break;
                    }
                }
            }

            if found_merge_target.is_some() {
                indices_to_remove.push(op_idx);
            }
        }
    }

    // Remove merged operations (in reverse order to maintain indices)
    indices_to_remove.sort();
    indices_to_remove.reverse();
    for idx in indices_to_remove {
        ops.remove_at(idx);
    }
}

fn get_next_context_steps(op: &Box<dyn ir::UpdateOp + Send + Sync>) -> Option<usize> {
    unsafe {
        let op_ptr = op.as_ref() as *const dyn ir::Op;

        if op.kind() == ir::OpKind::Statement {
            let stmt_op_ptr = op_ptr as *const StatementOp<Box<dyn ir::UpdateOp + Send + Sync>>;
            let stmt_op = &*stmt_op_ptr;

            if let Statement::Expression(ref expr_stmt) = *stmt_op.statement {
                if let Some(ir_expr) = as_ir_expression(&expr_stmt.expr) {
                    if let ir::IRExpression::NextContext(ref next_ctx) = ir_expr {
                        return Some(next_ctx.steps);
                    }
                }
            }
        } else if op.kind() == ir::OpKind::Variable {
            let var_op_ptr =
                op_ptr as *const ir::ops::VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>;
            let var_op = &*var_op_ptr;

            if let Some(ir_expr) = as_ir_expression(&var_op.initializer) {
                if let ir::IRExpression::NextContext(ref next_ctx) = ir_expr {
                    return Some(next_ctx.steps);
                }
            }
        }
    }
    None
}
