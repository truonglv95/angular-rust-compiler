//! Conditionals Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/conditionals.ts
//! Collapses the various conditions of conditional ops (if, switch) into a single test expression

use crate::output::output_ast::{
    BinaryOperator, BinaryOperatorExpr, ConditionalExpr, Expression, LiteralExpr, LiteralValue,
};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::expression::{
    AssignTemporaryExpr, ReadTemporaryExpr, SlotLiteralExpr,
};
use crate::template::pipeline::ir::ops::update::ConditionalOp;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob,
};

/// Collapse the various conditions of conditional ops (if, switch) into a single test expression.
pub fn generate_conditional_expressions(job: &mut dyn CompilationJob) {
    let component_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let job_ptr = job_ptr as *mut ComponentCompilationJob;
        &mut *job_ptr
    };

    // Process root unit
    process_unit(&mut component_job.root, job);

    // Process all view units
    for (_, unit) in component_job.views.iter_mut() {
        process_unit(unit, job);
    }
}

fn process_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    job: &mut dyn CompilationJob,
) {
    for op in unit.update_mut().iter_mut() {
        if op.kind() != OpKind::Conditional {
            continue;
        }

        // Downcast to ConditionalOp
        unsafe {
            let op_ptr = op.as_mut() as *mut dyn ir::UpdateOp;
            let conditional_op_ptr = op_ptr as *mut ConditionalOp;
            let conditional_op = &mut *conditional_op_ptr;

            let mut test: Expression;

            // Any case with a `null` condition is `default`. If one exists, default to it instead.
            let default_case_index = conditional_op
                .conditions
                .iter()
                .position(|cond| cond.expr.is_none());

            if let Some(default_index) = default_case_index {
                let default_case = conditional_op.conditions.remove(default_index);
                test = Expression::SlotLiteral(SlotLiteralExpr::new(default_case.target_slot));
            } else {
                // By default, a switch evaluates to `-1`, causing no template to be displayed.
                test = Expression::Literal(LiteralExpr {
                    value: LiteralValue::Number(-1.0),
                    type_: None,
                    source_span: None,
                });
            }

            // Switch expressions assign their main test to a temporary, to avoid re-executing it.
            let tmp: Option<(AssignTemporaryExpr, ir::XrefId)> =
                conditional_op.test.as_ref().map(|test_expr| {
                    let xref = job.allocate_xref_id();
                    let assign_tmp = AssignTemporaryExpr {
                        name: None,
                        expr: Box::new(test_expr.clone()),
                        xref,
                        source_span: None,
                    };
                    (assign_tmp, xref)
                });

            let mut case_expression_temporary_xref: Option<ir::XrefId> = None;

            // For each remaining condition, test whether the temporary satisfies the check. (If no temp is
            // present, just check each expression directly.)
            for i in (0..conditional_op.conditions.len()).rev() {
                let conditional_case = &mut conditional_op.conditions[i];

                if conditional_case.expr.is_none() {
                    continue;
                }

                let case_expr = if let Some((ref tmp_expr, tmp_xref)) = tmp {
                    // Use temporary for comparison
                    let use_tmp = if i == 0 {
                        Expression::AssignTemporary(tmp_expr.clone())
                    } else {
                        Expression::ReadTemporary(ReadTemporaryExpr::new(tmp_xref))
                    };

                    // Create binary comparison: use_tmp === conditional_case.expr
                    let case_expr_val = conditional_case.expr.as_ref().unwrap().clone();
                    Expression::BinaryOp(BinaryOperatorExpr {
                        operator: BinaryOperator::Identical,
                        lhs: Box::new(use_tmp),
                        rhs: case_expr_val,
                        type_: None,
                        source_span: None,
                    })
                } else if conditional_case.alias.is_some() {
                    // Since we can only pass one variable into the conditional instruction,
                    // reuse the same variable to store the result of the expressions.
                    if case_expression_temporary_xref.is_none() {
                        case_expression_temporary_xref = Some(job.allocate_xref_id());
                    }
                    let case_expr_val = conditional_case.expr.as_ref().unwrap().clone();
                    let case_tmp_xref = case_expression_temporary_xref.unwrap();
                    let assign_tmp = AssignTemporaryExpr {
                        name: None,
                        expr: case_expr_val,
                        xref: case_tmp_xref,
                        source_span: None,
                    };
                    conditional_op.context_value = Some(Expression::ReadTemporary(
                        ReadTemporaryExpr::new(case_tmp_xref),
                    ));
                    Expression::AssignTemporary(assign_tmp)
                } else {
                    // Use expression directly
                    *conditional_case.expr.as_ref().unwrap().clone()
                };

                // Build conditional: case_expr ? slot : test
                let slot_expr = Expression::SlotLiteral(SlotLiteralExpr::new(
                    conditional_case.target_slot.clone(),
                ));
                test = Expression::Conditional(ConditionalExpr {
                    condition: Box::new(case_expr),
                    true_case: Box::new(slot_expr),
                    false_case: Some(Box::new(test)),
                    type_: None,
                    source_span: None,
                });
            }

            // Save the resulting aggregate expression
            conditional_op.processed = Some(test);

            // Clear the original conditions array, since we no longer need it, and don't want it to
            // affect subsequent phases (e.g. pipe creation).
            conditional_op.conditions.clear();
        }
    }
}
