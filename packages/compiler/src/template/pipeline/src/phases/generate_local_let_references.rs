//! Generate Local Let References Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/generate_local_let_references.ts
//! Replaces the `storeLet` ops with variables that can be used to reference the value within the same view.

use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::expression::StoreLetExpr;
use crate::template::pipeline::ir::ops::shared::create_variable_op;
use crate::template::pipeline::ir::ops::update::StoreLetOp;
use crate::template::pipeline::ir::variable::{IdentifierVariable, SemanticVariable};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, CompilationUnit, ComponentCompilationJob,
};

/// Replaces the `storeLet` ops with variables that can be used to reference the value within the same view.
pub fn generate_local_let_references(job: &mut dyn CompilationJob) {
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
        process_unit(&mut component_job.root, job);

        // Process all view units
        for (_, unit) in component_job.views.iter_mut() {
            process_unit(unit, job);
        }
    }
}

fn process_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    job: &mut dyn CompilationJob,
) {
    // Store replacement data instead of ops
    #[derive(Debug)]
    struct ReplacementData {
        index: usize,
        xref: ir::XrefId,
        variable: SemanticVariable,
        store_let_expr: crate::output::output_ast::Expression,
    }

    let mut replacements: Vec<ReplacementData> = Vec::new();

    // First pass: collect StoreLetOp replacement data
    for (index, op) in unit.update().iter().enumerate() {
        if op.kind() != OpKind::StoreLet {
            continue;
        }

        unsafe {
            let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
            let store_let_ptr = op_ptr as *const StoreLetOp;
            let store_let = &*store_let_ptr;

            // Create IdentifierVariable
            let variable = SemanticVariable::Identifier(IdentifierVariable {
                identifier: store_let.declared_name.to_string(),
                local: true,
                name: None,
            });

            // Create StoreLetExpr
            let store_let_expr =
                crate::output::output_ast::Expression::StoreLet(StoreLetExpr::new(
                    store_let.target,
                    Box::new(store_let.value.clone()),
                    store_let.source_span.clone(),
                ));

            replacements.push(ReplacementData {
                index,
                xref: job.allocate_xref_id(),
                variable,
                store_let_expr,
            });
        }
    }

    // Second pass: apply replacements (in reverse order to maintain indices)
    for replacement_data in replacements.iter().rev() {
        // Create VariableOp
        let variable_op = create_variable_op::<Box<dyn ir::UpdateOp + Send + Sync>>(
            replacement_data.xref,
            replacement_data.variable.clone(),
            Box::new(replacement_data.store_let_expr.clone()),
            ir::VariableFlags::NONE,
        );

        // Wrap in Box<dyn UpdateOp>
        let boxed_op = Box::new(variable_op) as Box<dyn ir::UpdateOp + Send + Sync>;
        unit.update_mut()
            .replace_at(replacement_data.index, boxed_op);
    }
}
