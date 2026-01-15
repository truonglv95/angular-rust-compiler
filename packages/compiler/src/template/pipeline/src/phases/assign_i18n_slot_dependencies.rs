//! Assign I18n Slot Dependencies Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/assign_i18n_slot_dependencies.ts
//! Updates i18n expression ops to target the last slot in their owning i18n block

use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{I18nExpressionFor, OpKind};
use crate::template::pipeline::ir::ops::create::I18nStartOp;
use crate::template::pipeline::ir::ops::update::I18nExpressionOp;
use crate::template::pipeline::ir::traits::DependsOnSlotContextOpTrait;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob,
};

struct BlockState {
    block_xref: ir::XrefId,
    last_slot_consumer: ir::XrefId,
}

/// Updates i18n expression ops to target the last slot in their owning i18n block, and moves them
/// after the last update instruction that depends on that slot.
pub fn assign_i18n_slot_dependencies(job: &mut dyn CompilationJob) {
    let component_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let job_ptr = job_ptr as *mut ComponentCompilationJob;
        &mut *job_ptr
    };

    // Process root unit
    process_unit(&mut component_job.root);

    // Process all view units
    for (_, unit) in component_job.views.iter_mut() {
        process_unit(unit);
    }
}

fn process_unit(unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit) {
    // Track the current position in update list (similar to TypeScript's updateOp pointer)
    let mut update_index = 0;
    let mut i18n_expressions_in_progress: Vec<I18nExpressionOp> = Vec::new();
    let mut state: Option<BlockState> = None;

    // Collect all operations to perform (to avoid borrow conflicts)
    let mut operations: Vec<Operation> = Vec::new();

    // Iterate through create ops
    for create_op in unit.create().iter() {
        if create_op.kind() == OpKind::I18nStart {
            let op_ptr = create_op.as_ref() as *const dyn ir::CreateOp;
            let i18n_start_ptr = op_ptr as *const I18nStartOp;
            let i18n_start = unsafe { &*i18n_start_ptr };
            state = Some(BlockState {
                block_xref: i18n_start.base.xref,
                last_slot_consumer: i18n_start.base.xref,
            });
        } else if create_op.kind() == OpKind::I18nEnd {
            // Insert collected expressions before current update_index
            if let Some(ref st) = state {
                for mut expr_op in i18n_expressions_in_progress.drain(..) {
                    expr_op.target = st.last_slot_consumer;
                    operations.push(Operation::Insert {
                        index: update_index,
                        op: expr_op,
                    });
                }
            }
            state = None;
        }

        // Check if this create op consumes slots
        if has_consumes_slot_op_trait(create_op) {
            if let Some(ref mut st) = state {
                st.last_slot_consumer = create_op.xref();
            }

            // Process update ops that depend on this slot
            while update_index < unit.update().len() {
                let update_op = unit.update().get(update_index);

                if update_op.is_none() {
                    break;
                }

                let update_op = update_op.unwrap();

                // Check if this is an i18n expression that should be moved
                if update_op.kind() == OpKind::I18nExpression {
                    if let Some(ref st) = state {
                        let op_ptr = update_op.as_ref() as *const dyn ir::UpdateOp;
                        let expr_op_ptr = op_ptr as *const I18nExpressionOp;
                        let expr_op = unsafe { &*expr_op_ptr };

                        if expr_op.usage == I18nExpressionFor::I18nText
                            && expr_op.i18n_owner == st.block_xref
                        {
                            // Remove this expression and collect it
                            operations.push(Operation::Remove {
                                index: update_index,
                            });
                            i18n_expressions_in_progress.push(expr_op.clone());
                            // Don't increment update_index - next op is now at this index
                            continue;
                        }
                    }
                }

                // Check if this update op has a different target
                let has_different_target = has_different_slot_target(update_op, create_op.xref());

                if has_different_target {
                    break;
                }

                update_index += 1;
            }
        }
    }

    // Perform all operations in reverse order of index to maintain correctness
    // First, collect all remove operations
    let mut remove_indices: Vec<usize> = operations
        .iter()
        .filter_map(|op| {
            if let Operation::Remove { index } = op {
                Some(*index)
            } else {
                None
            }
        })
        .collect();
    remove_indices.sort();

    // Remove operations in reverse order
    for &idx in remove_indices.iter().rev() {
        unit.update_mut().remove_at(idx);
    }

    // Adjust insertion indices for removals
    let mut adjusted_insertions = Vec::new();
    for op in operations {
        if let Operation::Insert {
            mut index,
            op: expr_op,
        } = op
        {
            // Adjust index for all removals that happened before this insertion
            for &removed_idx in &remove_indices {
                if removed_idx < index {
                    index -= 1;
                }
            }
            adjusted_insertions.push((index, expr_op));
        }
    }

    // Insert expressions in reverse order
    adjusted_insertions.sort_by(|a, b| b.0.cmp(&a.0));
    for (idx, expr_op) in adjusted_insertions {
        unit.update_mut().insert_at(idx, Box::new(expr_op));
    }
}

enum Operation {
    Remove { index: usize },
    Insert { index: usize, op: I18nExpressionOp },
}

/// Check if an op implements ConsumesSlotOpTrait based on its kind
fn has_consumes_slot_op_trait(op: &Box<dyn ir::CreateOp + Send + Sync>) -> bool {
    use crate::template::pipeline::src::util::elements::op_kind_has_consumes_slot_trait;
    op_kind_has_consumes_slot_trait(op.kind())
}

/// Check if an update op depends on a different slot target
fn has_different_slot_target(
    update_op: &Box<dyn ir::UpdateOp + Send + Sync>,
    expected_target: ir::XrefId,
) -> bool {
    // Check if the op implements DependsOnSlotContextOpTrait
    unsafe {
        let op_ptr = update_op.as_ref() as *const dyn ir::UpdateOp;

        // First, check if op itself implements DependsOnSlotContextOpTrait
        match update_op.kind() {
            OpKind::Conditional => {
                use crate::template::pipeline::ir::ops::update::ConditionalOp;
                let cond_op_ptr = op_ptr as *const ConditionalOp;
                let cond_op = &*cond_op_ptr;
                if cond_op.target() != expected_target {
                    return true;
                }
            }
            OpKind::Repeater => {
                use crate::template::pipeline::ir::ops::update::RepeaterOp;
                let rep_op_ptr = op_ptr as *const RepeaterOp;
                let rep_op = &*rep_op_ptr;
                if rep_op.target() != expected_target {
                    return true;
                }
            }
            OpKind::Property => {
                use crate::template::pipeline::ir::ops::update::PropertyOp;
                let prop_op_ptr = op_ptr as *const PropertyOp;
                let prop_op = &*prop_op_ptr;
                if prop_op.target() != expected_target {
                    return true;
                }
            }
            OpKind::Attribute => {
                use crate::template::pipeline::ir::ops::update::AttributeOp;
                let attr_op_ptr = op_ptr as *const AttributeOp;
                let attr_op = &*attr_op_ptr;
                if attr_op.target() != expected_target {
                    return true;
                }
            }
            OpKind::TwoWayProperty => {
                use crate::template::pipeline::ir::ops::update::TwoWayPropertyOp;
                let two_way_op_ptr = op_ptr as *const TwoWayPropertyOp;
                let two_way_op = &*two_way_op_ptr;
                if two_way_op.target() != expected_target {
                    return true;
                }
            }
            OpKind::Control => {
                use crate::template::pipeline::ir::ops::update::ControlOp;
                let ctrl_op_ptr = op_ptr as *const ControlOp;
                let ctrl_op = &*ctrl_op_ptr;
                if ctrl_op.target() != expected_target {
                    return true;
                }
            }
            OpKind::InterpolateText => {
                use crate::template::pipeline::ir::ops::update::InterpolateTextOp;
                let interp_op_ptr = op_ptr as *const InterpolateTextOp;
                let interp_op = &*interp_op_ptr;
                if interp_op.target() != expected_target {
                    return true;
                }
            }
            OpKind::StoreLet => {
                use crate::template::pipeline::ir::ops::update::StoreLetOp;
                let store_let_op_ptr = op_ptr as *const StoreLetOp;
                let store_let_op = &*store_let_op_ptr;
                if store_let_op.target() != expected_target {
                    return true;
                }
            }
            OpKind::Statement => {
                use crate::template::pipeline::ir::ops::shared::StatementOp;
                let stmt_op_ptr = op_ptr as *const StatementOp<Box<dyn ir::UpdateOp + Send + Sync>>;
                let stmt_op = &*stmt_op_ptr;

                // Visit expressions in the statement
                if check_expressions_in_statement(&stmt_op.statement, expected_target) {
                    return true;
                }
            }
            OpKind::Variable => {
                use crate::template::pipeline::ir::ops::shared::VariableOp;
                let variable_op_ptr =
                    op_ptr as *const VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>;
                let variable_op = &*variable_op_ptr;

                // Visit expressions in the initializer
                if check_expressions_in_expression(&variable_op.initializer, expected_target) {
                    return true;
                }
            }
            _ => {
                // Other op types don't depend on slot context
            }
        }
    }

    false
}

/// Check if any expression in a statement depends on a different slot target
fn check_expressions_in_statement(
    stmt: &crate::output::output_ast::Statement,
    expected_target: ir::XrefId,
) -> bool {
    use crate::output::output_ast::Statement;

    match stmt {
        Statement::Block(block) => block
            .statements
            .iter()
            .any(|stmt| check_expressions_in_statement(stmt, expected_target)),
        Statement::Expression(expr_stmt) => {
            check_expressions_in_expression(&expr_stmt.expr, expected_target)
        }
        Statement::Return(return_stmt) => {
            check_expressions_in_expression(&return_stmt.value, expected_target)
        }
        Statement::DeclareVar(declare_var) => {
            if let Some(ref value) = declare_var.value {
                check_expressions_in_expression(value, expected_target)
            } else {
                false
            }
        }
        Statement::IfStmt(if_stmt) => {
            if check_expressions_in_expression(&if_stmt.condition, expected_target) {
                return true;
            }
            for case_stmt in &if_stmt.true_case {
                if check_expressions_in_statement(case_stmt, expected_target) {
                    return true;
                }
            }
            for case_stmt in &if_stmt.false_case {
                if check_expressions_in_statement(case_stmt, expected_target) {
                    return true;
                }
            }
            false
        }
        Statement::DeclareFn(declare_fn) => {
            for stmt in &declare_fn.statements {
                if check_expressions_in_statement(stmt, expected_target) {
                    return true;
                }
            }
            false
        }
    }
}

/// Check if an expression or any nested expression depends on a different slot target
fn check_expressions_in_expression(
    expr: &crate::output::output_ast::Expression,
    expected_target: ir::XrefId,
) -> bool {
    use crate::output::output_ast::Expression as OutputExpr;
    use crate::template::pipeline::ir::traits::DependsOnSlotContextOpTrait;

    // Check if this expression itself implements DependsOnSlotContextOpTrait
    // Currently, only StoreLetExpr implements this trait among IR expressions
    match expr {
        OutputExpr::StoreLet(store_let_expr) => {
            if store_let_expr.target() != expected_target {
                return true;
            }
        }
        _ => {}
    }

    // Recursively check nested expressions
    match expr {
        OutputExpr::BinaryOp(bin) => {
            check_expressions_in_expression(&bin.lhs, expected_target)
                || check_expressions_in_expression(&bin.rhs, expected_target)
        }
        OutputExpr::Unary(un) => check_expressions_in_expression(&un.expr, expected_target),
        OutputExpr::ReadProp(prop) => {
            check_expressions_in_expression(&prop.receiver, expected_target)
        }
        OutputExpr::ReadKey(key) => {
            check_expressions_in_expression(&key.receiver, expected_target)
                || check_expressions_in_expression(&key.index, expected_target)
        }
        OutputExpr::InvokeFn(invoke) => {
            if check_expressions_in_expression(&invoke.fn_, expected_target) {
                return true;
            }
            for arg in &invoke.args {
                if check_expressions_in_expression(arg, expected_target) {
                    return true;
                }
            }
            false
        }
        OutputExpr::LiteralArray(arr) => {
            for entry in &arr.entries {
                if check_expressions_in_expression(entry, expected_target) {
                    return true;
                }
            }
            false
        }
        OutputExpr::LiteralMap(map) => {
            for entry in &map.entries {
                if check_expressions_in_expression(&entry.value, expected_target) {
                    return true;
                }
            }
            false
        }
        OutputExpr::Conditional(cond) => {
            check_expressions_in_expression(&cond.condition, expected_target)
                || check_expressions_in_expression(&cond.true_case, expected_target)
                || cond
                    .false_case
                    .as_ref()
                    .map(|e| check_expressions_in_expression(e, expected_target))
                    .unwrap_or(false)
        }
        OutputExpr::PipeBinding(pipe) => {
            // PipeBinding has args, check them
            for arg in &pipe.args {
                if check_expressions_in_expression(arg, expected_target) {
                    return true;
                }
            }
            false
        }
        OutputExpr::TemplateLiteral(template) => {
            for expr in &template.expressions {
                if check_expressions_in_expression(expr, expected_target) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}
