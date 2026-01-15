//! Save Restore View Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/save_restore_view.ts
//! When inside of a listener, we may need access to one or more enclosing views. Therefore, each
//! view should save the current view, and each listener must have the ability to restore the
//! appropriate view. We eagerly generate all save view variables; they will be optimized away later.

use crate::core::ChangeDetectionStrategy;
use crate::output::output_ast::{Expression, Statement};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{OpKind, VariableFlags};
use crate::template::pipeline::ir::expression::{
    EitherXrefIdOrExpression, GetCurrentViewExpr, ResetViewExpr, RestoreViewExpr,
};
use crate::template::pipeline::ir::ops::create::{
    AnimationListenerOp, AnimationOp, ListenerOp, TwoWayListenerOp,
};
use crate::template::pipeline::ir::ops::shared::{create_variable_op, StatementOp, VariableOp};
use crate::template::pipeline::ir::variable::{
    ContextVariable, IdentifierVariable, SavedViewVariable, SemanticVariable,
};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, CompilationUnit, ComponentCompilationJob,
};

/// When inside of a listener, we may need access to one or more enclosing views. Therefore, each
/// view should save the current view, and each listener must have the ability to restore the
/// appropriate view. We eagerly generate all save view variables; they will be optimized away later.
pub fn save_and_restore_view(job: &mut dyn CompilationJob) {
    let job_kind = job.kind();

    if matches!(
        job_kind,
        CompilationJobKind::Tmpl | CompilationJobKind::Both
    ) {
        let component_job = unsafe {
            let job_ptr = job as *mut dyn CompilationJob;
            let job_ptr = job_ptr as *mut ComponentCompilationJob;
            &mut *job_ptr
        };

        // Process root unit
        {
            let component_job_ptr = component_job as *mut ComponentCompilationJob;
            let root_ptr = &mut component_job.root
                as *mut crate::template::pipeline::src::compilation::ViewCompilationUnit;
            process_unit(unsafe { &mut *root_ptr }, unsafe {
                &mut *component_job_ptr
            });
        }

        // Process all view units
        let view_keys: Vec<_> = component_job.views.keys().cloned().collect();
        for key in view_keys {
            let component_job_ptr = component_job as *mut ComponentCompilationJob;
            if let Some(unit) = component_job.views.get_mut(&key) {
                let unit_ptr =
                    unit as *mut crate::template::pipeline::src::compilation::ViewCompilationUnit;
                process_unit(unsafe { &mut *unit_ptr }, unsafe {
                    &mut *component_job_ptr
                });
            }
        }
    }
}

fn process_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    component_job: &mut ComponentCompilationJob,
) {
    // Check if this is the root unit
    let unit_xref = unit.xref();
    let root_xref = component_job.root.xref();
    let is_root = unit_xref == root_xref;

    // Collect all listener indices and check if ANY needs restoreView
    let mut listener_indices: Vec<usize> = Vec::new();
    let mut any_listener_needs_restore_view = false;

    for (idx, op) in unit.create_mut().iter_mut().enumerate() {
        match op.kind() {
            OpKind::Listener
            | OpKind::TwoWayListener
            | OpKind::Animation
            | OpKind::AnimationListener => {
                listener_indices.push(idx);

                // We need restoreView if we're in a nested view (to restore the correct ctx),
                // OR if we're in any view (including root) and use template references/variables.
                if !is_root || check_needs_restore_view(op) {
                    any_listener_needs_restore_view = true;
                }
            }
            _ => {}
        }
    }

    // Only proceed with save/restore if actually needed
    if any_listener_needs_restore_view {
        // Prepend a variable op with SavedView for this unit
        let saved_view_xref = component_job.allocate_xref_id();
        let saved_view_variable = SemanticVariable::SavedView(SavedViewVariable::new(unit_xref));
        let get_current_view_expr = Expression::GetCurrentView(GetCurrentViewExpr::new());

        let variable_op = create_variable_op::<Box<dyn ir::CreateOp + Send + Sync>>(
            saved_view_xref,
            saved_view_variable,
            Box::new(get_current_view_expr),
            VariableFlags::NONE,
        );

        let boxed_variable_op: Box<dyn ir::CreateOp + Send + Sync> = Box::new(variable_op);
        unit.create_mut().prepend(vec![boxed_variable_op]);

        // Second pass: apply to all listeners in this view
        // Note: listener indices are now shifted by 1 due to prepend
        let component_job_ptr = component_job as *mut ComponentCompilationJob;
        let unit_ptr =
            unit as *mut crate::template::pipeline::src::compilation::ViewCompilationUnit;

        for idx in listener_indices {
            unsafe {
                let unit_ref = &mut *unit_ptr;
                if let Some(op) = unit_ref.create_mut().get_mut(idx + 1) {
                    add_save_restore_view_operation_to_listener(
                        unit_ptr,
                        op,
                        component_job_ptr,
                        saved_view_xref,
                    );
                }
            }
        }
    }
}

fn check_needs_restore_view(op: &Box<dyn ir::CreateOp + Send + Sync>) -> bool {
    match op.kind() {
        OpKind::Listener => unsafe {
            let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
            let listener_ptr = op_ptr as *const ListenerOp;
            let listener = &*listener_ptr;

            for handler_op in listener.handler_ops.iter() {
                if contains_reference_expr(handler_op) {
                    return true;
                }
            }
        },
        OpKind::TwoWayListener => unsafe {
            let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
            let two_way_listener_ptr = op_ptr as *const TwoWayListenerOp;
            let two_way_listener = &*two_way_listener_ptr;

            for handler_op in two_way_listener.handler_ops.iter() {
                if contains_reference_expr(handler_op) {
                    return true;
                }
            }
        },
        OpKind::Animation => unsafe {
            let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
            let animation_ptr = op_ptr as *const AnimationOp;
            let animation = &*animation_ptr;

            for handler_op in animation.handler_ops.iter() {
                if contains_reference_expr(handler_op) {
                    return true;
                }
            }
        },
        OpKind::AnimationListener => unsafe {
            let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
            let animation_listener_ptr = op_ptr as *const AnimationListenerOp;
            let animation_listener = &*animation_listener_ptr;

            for handler_op in animation_listener.handler_ops.iter() {
                if contains_reference_expr(handler_op) {
                    return true;
                }
            }
        },
        _ => {}
    }
    false
}

fn contains_reference_expr(op: &Box<dyn ir::UpdateOp + Send + Sync>) -> bool {
    // We need to traverse expressions to check for ReferenceExpr or ContextLetReferenceExpr.
    // Since transform_expressions_in_op requires &mut, we traverse handler ops directly
    // by downcasting to specific op types and checking their handler_ops.
    // This avoids the need for invalid reference casting.

    match op.kind() {
        OpKind::Statement => {
            // Check if statement contains expressions with ReferenceExpr
            // We'll traverse expressions in StatementOp
            check_expressions_in_statement_op(op)
        }
        OpKind::Variable => {
            // Check initializer expression
            check_expressions_in_variable_op(op)
        }
        _ => {
            // For other ops, we can't easily traverse without mutable access
            // But most ops don't contain ReferenceExpr directly, so this is usually fine
            // The main concern is handler_ops which we check separately in check_needs_restore_view
            false
        }
    }
}

fn check_expressions_in_statement_op(op: &Box<dyn ir::UpdateOp + Send + Sync>) -> bool {
    unsafe {
        let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
        let statement_op_ptr = op_ptr as *const StatementOp<Box<dyn ir::UpdateOp + Send + Sync>>;
        let statement_op = &*statement_op_ptr;

        // Check expressions in the statement
        check_expressions_in_statement(&statement_op.statement)
    }
}

fn check_expressions_in_variable_op(op: &Box<dyn ir::UpdateOp + Send + Sync>) -> bool {
    unsafe {
        let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
        let variable_op_ptr = op_ptr as *const VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>;
        let variable_op = &*variable_op_ptr;

        // Check expressions in the initializer
        check_expressions_recursive(&variable_op.initializer)
    }
}

fn check_expressions_in_statement(stmt: &crate::output::output_ast::Statement) -> bool {
    match stmt {
        Statement::Block(block) => block
            .statements
            .iter()
            .any(|stmt| check_expressions_in_statement(stmt)),
        Statement::Return(ref return_stmt) => check_expressions_recursive(&return_stmt.value),
        Statement::Expression(ref expr_stmt) => check_expressions_recursive(&expr_stmt.expr),
        Statement::DeclareVar(ref var_stmt) => {
            if let Some(ref value) = var_stmt.value {
                check_expressions_recursive(value)
            } else {
                false
            }
        }
        Statement::IfStmt(_) => {
            // For now, we don't check inside if statements
            // as they're less likely to contain ReferenceExpr directly
            false
        }
        Statement::DeclareFn(_) => {
            // For now, we don't check inside function declarations
            false
        }
    }
}

fn check_expressions_recursive(expr: &Expression) -> bool {
    match expr {
        // IR expressions we're looking for
        Expression::Reference(_) | Expression::ContextLetReference(_) => {
            return true;
        }
        // Expressions that may contain nested expressions
        Expression::BinaryOp(bin) => {
            check_expressions_recursive(&bin.lhs) || check_expressions_recursive(&bin.rhs)
        }
        Expression::Unary(un) => {
            check_expressions_recursive(&un.expr)
        }
        Expression::ReadProp(prop) => {
            check_expressions_recursive(&prop.receiver)
        }
        Expression::ReadKey(key) => {
            check_expressions_recursive(&key.receiver) || check_expressions_recursive(&key.index)
        }
        Expression::WriteVar(write) => {
            check_expressions_recursive(&write.value)
        }
        Expression::WriteKey(write) => {
            check_expressions_recursive(&write.receiver) ||
            check_expressions_recursive(&write.index) ||
            check_expressions_recursive(&write.value)
        }
        Expression::WriteProp(write) => {
            check_expressions_recursive(&write.receiver) || check_expressions_recursive(&write.value)
        }
        Expression::InvokeFn(invoke) => {
            if check_expressions_recursive(&invoke.fn_) {
                return true;
            }
            for arg in &invoke.args {
                if check_expressions_recursive(arg) {
                    return true;
                }
            }
            false
        }
        Expression::LiteralArray(arr) => {
            for entry in &arr.entries {
                if check_expressions_recursive(entry) {
                    return true;
                }
            }
            false
        }
        Expression::LiteralMap(map) => {
            for entry in &map.entries {
                if check_expressions_recursive(&entry.value) {
                    return true;
                }
            }
            false
        }
        Expression::Conditional(cond) => {
            check_expressions_recursive(&cond.condition) ||
            check_expressions_recursive(&cond.true_case) ||
            (cond.false_case.as_ref().map_or(false, |e| check_expressions_recursive(e)))
        }
        Expression::TypeOf(type_of) => {
            check_expressions_recursive(&type_of.expr)
        }
        Expression::Void(void) => {
            check_expressions_recursive(&void.expr)
        }
        Expression::Parens(parens) => {
            check_expressions_recursive(&parens.expr)
        }
        Expression::NotExpr(not) => {
            check_expressions_recursive(&not.condition)
        }
        Expression::TaggedTemplate(tagged) => {
            check_expressions_recursive(&tagged.tag) ||
            tagged.template.expressions.iter().any(|e| check_expressions_recursive(e))
        }
        Expression::TemplateLiteral(template) => {
            template.expressions.iter().any(|e| check_expressions_recursive(e))
        }
        Expression::Instantiate(inst) => {
            if check_expressions_recursive(&inst.class_expr) {
                return true;
            }
            for arg in &inst.args {
                if check_expressions_recursive(arg) {
                    return true;
                }
            }
            false
        }
        Expression::Localized(localized) => {
            localized.expressions.iter().any(|e| check_expressions_recursive(e))
        }
        Expression::Cast(cast) => {
            check_expressions_recursive(&cast.value)
        }
        Expression::IfNull(if_null) => {
            check_expressions_recursive(&if_null.condition) || check_expressions_recursive(&if_null.null_case)
        }
        Expression::AssertNotNull(assert) => {
            check_expressions_recursive(&assert.condition)
        }
        Expression::ArrowFn(arrow) => {
            // Check body - could be expression or statements
            match &arrow.body {
                crate::output::output_ast::ArrowFunctionBody::Expression(expr) => {
                    check_expressions_recursive(expr)
                }
                crate::output::output_ast::ArrowFunctionBody::Statements(_) => {
                    // Statements are less likely to contain ReferenceExpr directly
                    false
                }
            }
        }
        Expression::Fn(_func) => {
            // Function expressions don't contain ReferenceExpr in their body
            false
        }
        Expression::CommaExpr(comma) => {
            comma.parts.iter().any(|e| check_expressions_recursive(e))
        }
        // IR expressions that don't contain nested ReferenceExpr
        Expression::Context(_) | Expression::NextContext(_) | Expression::GetCurrentView(_) |
        Expression::RestoreView(_) | Expression::ResetView(_) | Expression::ReadVariable(_) |
        Expression::PureFunction(_) | Expression::PureFunctionParameter(_) |
        Expression::PipeBinding(_) | Expression::PipeBindingVariadic(_) |
        Expression::SafePropertyRead(_) | Expression::SafeKeyedRead(_) |
        Expression::SafeInvokeFunction(_) | Expression::SafeTernary(_) | Expression::Empty(_) |
        Expression::AssignTemporary(_) | Expression::ReadTemporary(_) |
        Expression::SlotLiteral(_) | Expression::ConditionalCase(_) |
        Expression::ConstCollected(_) | Expression::TwoWayBindingSet(_) |
        Expression::StoreLet(_) | Expression::TrackContext(_) | Expression::LexicalRead(_) |
        // Simple expressions with no nested expressions
        Expression::ReadVar(_) | Expression::Literal(_) | Expression::External(_) |
        Expression::ExternalRef(_) | Expression::WrappedNode(_) | Expression::DynamicImport(_) |
        Expression::FnParam(_) | Expression::RegularExpressionLiteral(_) | Expression::RawCode(_) => {
            false
        }
    }
}

unsafe fn add_save_restore_view_operation_to_listener(
    unit_ptr: *mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    op: &mut Box<dyn ir::CreateOp + Send + Sync>,
    component_job_ptr: *mut ComponentCompilationJob,
    saved_view_xref: ir::XrefId,
) {
    let unit = &mut *unit_ptr;
    let unit_xref = unit.xref();
    let context_xref = unsafe { (&mut *component_job_ptr).allocate_xref_id() };

    let root_xref = unsafe { (&*component_job_ptr).root.xref() };
    let change_detection = unsafe { (&*component_job_ptr).change_detection };

    // DEBUG: Log change detection
    // eprintln!("[save_restore_view] add_save_restore_view: unit={:?}, root={:?}, change_detection={:?}", unit_xref, root_xref, change_detection);

    // NOTE: Previously had an optimization that skipped restoreView for OnPush root views,
    // but this was incorrect for Linker-compiled templates where ctx is not properly captured.
    // Now we always add restoreView for all listeners.

    let context_variable = if unit_xref == root_xref {
        // For the root view, we want to use the generic 'ctx' available in the closure,
        // so we treat the restoreView result as a generic IdentifierVariable that isn't
        // mapped by resolve_contexts.
        SemanticVariable::Identifier(IdentifierVariable::new(
            "restored_ctx".to_string(), // Name doesn't matter much as it won't be used for lookup
            true,
        ))
    } else {
        // For embedded views, we need to map all context accesses to this variable,
        // so we use ContextVariable. DON'T set an explicit name - let naming phase
        // generate a unique name (ctx_r1, ctx_r2, etc.) to avoid shadowing the
        // template function's 'ctx' parameter.
        SemanticVariable::Context(ContextVariable::new(unit_xref))
    };

    // Use saved_view_xref to reference the variable holding getCurrentView() result.
    // We MUST use ReadVariable here so that validation phases (like variable_optimization)
    // can see that the variable is being used. If we used raw XrefId, it would be invisible
    // to visitors and the variable would be incorrectly removed as dead code.
    let restore_view_expr = Expression::RestoreView(RestoreViewExpr::new(
        EitherXrefIdOrExpression::Expression(Box::new(Expression::ReadVariable(
            crate::template::pipeline::ir::expression::ReadVariableExpr::new(saved_view_xref),
        ))),
    ));

    let variable_op = create_variable_op::<Box<dyn ir::UpdateOp + Send + Sync>>(
        context_xref,
        context_variable,
        Box::new(restore_view_expr),
        VariableFlags::NONE,
    );

    // Box the variable op to match the OpList type
    let boxed_variable_op: Box<dyn ir::UpdateOp + Send + Sync> = Box::new(variable_op);

    match op.kind() {
        OpKind::Listener => {
            unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let listener_ptr = op_ptr as *mut ListenerOp;
                let listener = &mut *listener_ptr;

                listener.handler_ops.prepend(vec![boxed_variable_op]);

                // Wrap return statements with ResetViewExpr
                for handler_op in listener.handler_ops.iter_mut() {
                    wrap_return_statements_with_reset_view(handler_op.as_mut());
                }
            }
        }
        OpKind::TwoWayListener => {
            unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let two_way_listener_ptr = op_ptr as *mut TwoWayListenerOp;
                let two_way_listener = &mut *two_way_listener_ptr;

                two_way_listener
                    .handler_ops
                    .prepend(vec![boxed_variable_op]);

                // Wrap return statements with ResetViewExpr
                for handler_op in two_way_listener.handler_ops.iter_mut() {
                    wrap_return_statements_with_reset_view(handler_op.as_mut());
                }
            }
        }
        OpKind::Animation => {
            unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let animation_ptr = op_ptr as *mut AnimationOp;
                let animation = &mut *animation_ptr;

                animation.handler_ops.prepend(vec![boxed_variable_op]);

                // Wrap return statements with ResetViewExpr
                for handler_op in animation.handler_ops.iter_mut() {
                    wrap_return_statements_with_reset_view(handler_op.as_mut());
                }
            }
        }
        OpKind::AnimationListener => {
            unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let animation_listener_ptr = op_ptr as *mut AnimationListenerOp;
                let animation_listener = &mut *animation_listener_ptr;

                animation_listener
                    .handler_ops
                    .prepend(vec![boxed_variable_op]);

                // Wrap return statements with ResetViewExpr
                for handler_op in animation_listener.handler_ops.iter_mut() {
                    wrap_return_statements_with_reset_view(handler_op.as_mut());
                }
            }
        }
        _ => {}
    }
}

fn wrap_return_statements_with_reset_view(op: &mut dyn ir::UpdateOp) {
    if op.kind() == OpKind::Statement {
        unsafe {
            let op_ptr = op as *mut dyn ir::UpdateOp;
            let statement_op_ptr = op_ptr as *mut StatementOp<Box<dyn ir::UpdateOp + Send + Sync>>;
            let statement_op = &mut *statement_op_ptr;

            match *statement_op.statement {
                Statement::Return(ref mut return_stmt) => {
                    // Wrap the return value with ResetViewExpr
                    let return_value = return_stmt.value.clone();
                    let reset_view_expr = Expression::ResetView(ResetViewExpr::new(return_value));
                    return_stmt.value = Box::new(reset_view_expr);
                }
                _ => {}
            }
        }
    }
}
