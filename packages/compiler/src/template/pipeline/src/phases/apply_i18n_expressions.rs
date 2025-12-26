//! Apply I18n Expressions Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/apply_i18n_expressions.ts
//! Adds apply operations after i18n expressions

use crate::parse_util::ParseSourceSpan;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::handle::SlotHandle;
use crate::template::pipeline::ir::ops::create::I18nContextOp;
use crate::template::pipeline::ir::ops::update::{create_i18n_apply_op, I18nExpressionOp};
use crate::template::pipeline::src::compilation::{CompilationJob, ComponentCompilationJob};

/// Adds apply operations after i18n expressions.
pub fn apply_i18n_expressions(job: &mut dyn CompilationJob) {
    // Downcast to ComponentCompilationJob
    let component_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let job_ptr = job_ptr as *mut ComponentCompilationJob;
        &mut *job_ptr
    };

    // First, collect all I18nContextOps
    let mut i18n_contexts = std::collections::HashMap::new();
    // Collect from root unit
    for op in component_job.root.create.iter() {
        if op.kind() == OpKind::I18nContext {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let ctx_op_ptr = op_ptr as *const I18nContextOp;
                let ctx_op_ref = &*ctx_op_ptr;
                i18n_contexts.insert(ctx_op_ref.xref, ctx_op_ref.clone());
            }
        }
    }
    // Collect from all view units
    for (_, unit) in component_job.views.iter() {
        for op in unit.create.iter() {
            if op.kind() == OpKind::I18nContext {
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let ctx_op_ptr = op_ptr as *const I18nContextOp;
                    let ctx_op_ref = &*ctx_op_ptr;
                    i18n_contexts.insert(ctx_op_ref.xref, ctx_op_ref.clone());
                }
            }
        }
    }

    // Process root unit
    process_unit(&mut component_job.root, &i18n_contexts);

    // Process all view units
    for (_, unit) in component_job.views.iter_mut() {
        process_unit(unit, &i18n_contexts);
    }
}

fn process_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    i18n_contexts: &std::collections::HashMap<ir::XrefId, I18nContextOp>,
) {
    // Collect indices of I18nExpressionOps that need application
    let mut indices_to_insert_after: Vec<(usize, ir::XrefId, SlotHandle, ParseSourceSpan)> =
        Vec::new();

    for (index, op) in unit.update.iter().enumerate() {
        if op.kind() == OpKind::I18nExpression {
            unsafe {
                // op is &Box<dyn UpdateOp>, so we need to dereference it first
                let op_inner = op.as_ref() as *const dyn ir::UpdateOp;
                let expr_op_ptr = op_inner as *const I18nExpressionOp;
                let expr_op_ref = &*expr_op_ptr;

                if needs_application(i18n_contexts, &unit.update, index, expr_op_ref) {
                    // Use the expression op's source span for the apply op
                    let source_span = expr_op_ref.source_span.clone();
                    indices_to_insert_after.push((
                        index,
                        expr_op_ref.i18n_owner,
                        expr_op_ref.handle.clone(),
                        source_span,
                    ));
                }
            }
        }
    }

    // Insert apply ops in reverse order to maintain indices
    for (index, i18n_owner, handle, source_span) in indices_to_insert_after.iter().rev() {
        let apply_op = create_i18n_apply_op(*i18n_owner, handle.clone(), source_span.clone());
        // Insert after the expression op (at index + 1)
        unit.update.insert_at(index + 1, apply_op);
    }
}

/// Checks whether the given expression op needs to be followed with an apply op.
fn needs_application(
    i18n_contexts: &std::collections::HashMap<ir::XrefId, I18nContextOp>,
    update_list: &ir::operations::OpList<Box<dyn ir::UpdateOp + Send + Sync>>,
    current_index: usize,
    op: &I18nExpressionOp,
) -> bool {
    // If the next op is not another expression, we need to apply.
    let next_op = update_list.get(current_index + 1);
    if let Some(next) = next_op {
        if next.kind() != OpKind::I18nExpression {
            return true;
        }

        // Get the next I18nExpressionOp
        unsafe {
            let next_ptr = next.as_ref() as *const dyn ir::UpdateOp;
            let next_expr_op_ptr = next_ptr as *const I18nExpressionOp;
            let next_expr_op = &*next_expr_op_ptr;

            let context = i18n_contexts.get(&op.context);
            let next_context = i18n_contexts.get(&next_expr_op.context);

            if context.is_none() {
                panic!("AssertionError: expected an I18nContextOp to exist for the I18nExpressionOp's context");
            }

            if next_context.is_none() {
                panic!("AssertionError: expected an I18nContextOp to exist for the next I18nExpressionOp's context");
            }

            let context = context.unwrap();
            let next_context = next_context.unwrap();

            // If the next op is an expression targeting a different i18n block (or different element, in the
            // case of i18n attributes), we need to apply.

            // First, handle the case of i18n blocks.
            if let Some(i18n_block) = context.i18n_block {
                // This is a block context. Compare the blocks.
                if next_context.i18n_block != Some(i18n_block) {
                    return true;
                }
                return false;
            }

            // Second, handle the case of i18n attributes.
            if op.i18n_owner != next_expr_op.i18n_owner {
                return true;
            }

            return false;
        }
    }

    // No next op, need to apply
    true
}
