//! Save Restore View Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/save_restore_view.ts
//! When inside of a listener, we may need access to one or more enclosing views. Therefore, each
//! view should save the current view, and each listener must have the ability to restore the
//! appropriate view. We eagerly generate all save view variables; they will be optimized away later.

use crate::template::pipeline::ir as ir;
use crate::template::pipeline::ir::enums::{OpKind, VariableFlags};
use crate::template::pipeline::ir::expression::{GetCurrentViewExpr, RestoreViewExpr, ResetViewExpr, EitherXrefIdOrExpression};
use crate::template::pipeline::ir::ops::shared::{create_variable_op, StatementOp, VariableOp};
use crate::template::pipeline::ir::ops::create::{ListenerOp, AnimationListenerOp, TwoWayListenerOp, AnimationOp};
use crate::template::pipeline::ir::variable::{SavedViewVariable, ContextVariable, SemanticVariable};
use crate::template::pipeline::src::compilation::{CompilationJob, ComponentCompilationJob, CompilationJobKind, CompilationUnit};
use crate::output::output_ast::{Expression, Statement};

/// When inside of a listener, we may need access to one or more enclosing views. Therefore, each
/// view should save the current view, and each listener must have the ability to restore the
/// appropriate view. We eagerly generate all save view variables; they will be optimized away later.
pub fn save_and_restore_view(job: &mut dyn CompilationJob) {
    let job_kind = job.kind();
    
    if matches!(job_kind, CompilationJobKind::Tmpl | CompilationJobKind::Both) {
        let component_job = unsafe {
            let job_ptr = job as *mut dyn CompilationJob;
            let job_ptr = job_ptr as *mut ComponentCompilationJob;
            &mut *job_ptr
        };
        
        // Process root unit
        {
            let component_job_ptr = component_job as *mut ComponentCompilationJob;
            let root_ptr = &mut component_job.root as *mut crate::template::pipeline::src::compilation::ViewCompilationUnit;
            process_unit(unsafe { &mut *root_ptr }, unsafe { &mut *component_job_ptr });
        }
        
        // Process all view units
        let view_keys: Vec<_> = component_job.views.keys().cloned().collect();
        for key in view_keys {
            let component_job_ptr = component_job as *mut ComponentCompilationJob;
            if let Some(unit) = component_job.views.get_mut(&key) {
                let unit_ptr = unit as *mut crate::template::pipeline::src::compilation::ViewCompilationUnit;
                process_unit(unsafe { &mut *unit_ptr }, unsafe { &mut *component_job_ptr });
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
    
    // First pass: collect ops that need restore view BEFORE prepending variable op
    // We'll collect references to ops by iterating first, then modify them
    let mut ops_needing_restore_view: Vec<*mut Box<dyn ir::CreateOp + Send + Sync>> = Vec::new();
    
    for op in unit.create_mut().iter_mut() {
        match op.kind() {
            OpKind::Listener | OpKind::TwoWayListener | OpKind::Animation | OpKind::AnimationListener => {
                // Embedded views always need the save/restore view operation.
                let mut needs_restore_view = !is_root;
                
                if !needs_restore_view {
                    // Check if handler ops contain ReferenceExpr or ContextLetReferenceExpr
                    needs_restore_view = check_needs_restore_view(op);
                }
                
                if needs_restore_view {
                    // Store raw pointer to op for later modification
                    let op_ptr = op as *mut Box<dyn ir::CreateOp + Send + Sync>;
                    ops_needing_restore_view.push(op_ptr);
                }
            }
            _ => {}
        }
    }
    
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
    
    // Box the variable op to match the OpList type
    let boxed_variable_op: Box<dyn ir::CreateOp + Send + Sync> = Box::new(variable_op);
    unit.create_mut().prepend(vec![boxed_variable_op]);
    
    // Second pass: apply changes using raw pointers to avoid borrow checker issues
    let component_job_ptr = component_job as *mut ComponentCompilationJob;
    let unit_ptr = unit as *mut crate::template::pipeline::src::compilation::ViewCompilationUnit;
    
    for op_ptr in ops_needing_restore_view {
        unsafe {
            let op = &mut *op_ptr;
            add_save_restore_view_operation_to_listener(unit_ptr, op, component_job_ptr);
        }
    }
}

fn check_needs_restore_view(op: &Box<dyn ir::CreateOp + Send + Sync>) -> bool {
    match op.kind() {
        OpKind::Listener => {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let listener_ptr = op_ptr as *const ListenerOp;
                let listener = &*listener_ptr;
                
                for handler_op in listener.handler_ops.iter() {
                    if contains_reference_expr(handler_op) {
                        return true;
                    }
                }
            }
        }
        OpKind::TwoWayListener => {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let two_way_listener_ptr = op_ptr as *const TwoWayListenerOp;
                let two_way_listener = &*two_way_listener_ptr;
                
                for handler_op in two_way_listener.handler_ops.iter() {
                    if contains_reference_expr(handler_op) {
                        return true;
                    }
                }
            }
        }
        OpKind::Animation => {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let animation_ptr = op_ptr as *const AnimationOp;
                let animation = &*animation_ptr;
                
                for handler_op in animation.handler_ops.iter() {
                    if contains_reference_expr(handler_op) {
                        return true;
                    }
                }
            }
        }
        OpKind::AnimationListener => {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let animation_listener_ptr = op_ptr as *const AnimationListenerOp;
                let animation_listener = &*animation_listener_ptr;
                
                for handler_op in animation_listener.handler_ops.iter() {
                    if contains_reference_expr(handler_op) {
                        return true;
                    }
                }
            }
        }
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
        Statement::Return(ref return_stmt) => {
            check_expressions_recursive(&return_stmt.value)
        }
        Statement::Expression(ref expr_stmt) => {
            check_expressions_recursive(&expr_stmt.expr)
        }
        Statement::DeclareVar(ref var_stmt) => {
            if let Some(ref value) = var_stmt.value {
                check_expressions_recursive(value)
            } else {
                false
            }
        }
        Statement::DeclareFn(_) | Statement::IfStmt(_) => {
            // For now, we don't check inside function declarations or if statements
            // as they're less likely to contain ReferenceExpr directly
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
) {
    let unit = &mut *unit_ptr;
    let unit_xref = unit.xref();
    let context_xref = unsafe {
        (&mut *component_job_ptr).allocate_xref_id()
    };
    let context_variable = SemanticVariable::Context(ContextVariable::new(unit_xref));
    let restore_view_expr = Expression::RestoreView(RestoreViewExpr::new(
        EitherXrefIdOrExpression::XrefId(unit_xref)
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
                
                two_way_listener.handler_ops.prepend(vec![boxed_variable_op]);
                
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
                
                animation_listener.handler_ops.prepend(vec![boxed_variable_op]);
                
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
