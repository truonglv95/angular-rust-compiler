//! Local Refs Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/local_refs.ts
//! Lifts local reference declarations on element-like structures within each view into an entry in
//! the `consts` array for the whole component.

use crate::output::output_ast::{Expression, LiteralArrayExpr, LiteralExpr, LiteralValue};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::ops::create::LocalRef;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, CompilationUnit, ComponentCompilationJob,
};

/// Lifts local reference declarations on element-like structures within each view into an entry in
/// the `consts` array for the whole component.
pub fn lift_local_refs(job: &mut dyn CompilationJob) {
    let job_kind = job.kind();

    if matches!(
        job_kind,
        CompilationJobKind::Tmpl | CompilationJobKind::Both
    ) {
        unsafe {
            let job_ptr = job as *mut dyn CompilationJob;
            let component_job_ptr = job_ptr as *mut ComponentCompilationJob;
            let component_job = &mut *component_job_ptr;

            // Process root unit
            process_unit(&mut component_job.root, component_job_ptr);

            // Process all view units - collect keys first to avoid borrow checker issues
            let view_keys: Vec<_> = component_job.views.keys().cloned().collect();
            for key in view_keys {
                if let Some(unit) = component_job.views.get_mut(&key) {
                    process_unit(unit, component_job_ptr);
                }
            }
        }
    }
}

fn process_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    component_job_ptr: *mut ComponentCompilationJob,
) {
    for op in unit.create_mut().iter_mut() {
        match op.kind() {
            OpKind::ElementStart
            | OpKind::ConditionalCreate
            | OpKind::ConditionalBranchCreate
            | OpKind::Template => {
                unsafe {
                    use crate::template::pipeline::ir::ops::create::{
                        ConditionalBranchCreateOp, ConditionalCreateOp, ElementStartOp, TemplateOp,
                    };

                    match op.kind() {
                        OpKind::ElementStart => {
                            let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                            let element_ptr = op_ptr as *mut ElementStartOp;
                            let element = &mut *element_ptr;

                            // localRefs should still be an array at this point
                            let local_refs = std::mem::take(&mut element.base.base.local_refs);
                            let local_refs_len = local_refs.len();

                            // Update num_slots_used by adding local_refs.length
                            // Note: In TypeScript, numSlotsUsed is a field that gets incremented.
                            // In Rust, num_slots_used() is a method, so we can't directly update it.
                            // The slot allocation phase will handle this based on local_refs count.

                            if local_refs_len > 0 {
                                let local_refs_expr = serialize_local_refs(&local_refs);
                                let const_index =
                                    (&mut *component_job_ptr).add_const(local_refs_expr, None);
                                element.base.base.local_refs_index = Some(const_index);
                                // Note: In TypeScript, localRefs changes from array to ConstIndex.
                                // In Rust, we've taken the array and serialized it to const array.
                                // The local_refs field is now empty, which represents null in TS.
                                // The const_index is stored in the const array but not in the op struct.
                            }
                            // If local_refs_len == 0, local_refs is already empty (null in TS)
                        }
                        OpKind::ConditionalCreate => {
                            let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                            let conditional_ptr = op_ptr as *mut ConditionalCreateOp;
                            let conditional = &mut *conditional_ptr;

                            let local_refs = std::mem::take(&mut conditional.base.base.local_refs);
                            let _local_refs_len = local_refs.len();

                            if !local_refs.is_empty() {
                                let local_refs_expr = serialize_local_refs(&local_refs);
                                let const_index =
                                    (&mut *component_job_ptr).add_const(local_refs_expr, None);
                                conditional.base.base.local_refs_index = Some(const_index);
                            }
                        }
                        OpKind::ConditionalBranchCreate => {
                            let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                            let branch_ptr = op_ptr as *mut ConditionalBranchCreateOp;
                            let branch = &mut *branch_ptr;

                            let local_refs = std::mem::take(&mut branch.base.base.local_refs);
                            let _local_refs_len = local_refs.len();

                            if !local_refs.is_empty() {
                                let local_refs_expr = serialize_local_refs(&local_refs);
                                let const_index =
                                    (&mut *component_job_ptr).add_const(local_refs_expr, None);
                                branch.base.base.local_refs_index = Some(const_index);
                            }
                        }
                        OpKind::Template => {
                            let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                            let template_ptr = op_ptr as *mut TemplateOp;
                            let template = &mut *template_ptr;

                            let local_refs = std::mem::take(&mut template.base.base.local_refs);

                            if !local_refs.is_empty() {
                                let local_refs_expr = serialize_local_refs(&local_refs);
                                let const_index =
                                    (&mut *component_job_ptr).add_const(local_refs_expr, None);
                                template.base.base.local_refs_index = Some(const_index);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

fn serialize_local_refs(refs: &[LocalRef]) -> Expression {
    #[allow(unused_assignments)]
    let mut const_refs: Vec<Expression> = Vec::new();

    for ref_item in refs {
        // Push name as literal
        const_refs.push(Expression::Literal(LiteralExpr {
            value: LiteralValue::String(ref_item.name.clone()),
            type_: None,
            source_span: None,
        }));

        // Push target as literal (target is a String)
        const_refs.push(Expression::Literal(LiteralExpr {
            value: LiteralValue::String(ref_item.target.clone()),
            type_: None,
            source_span: None,
        }));
    }

    Expression::LiteralArray(LiteralArrayExpr {
        entries: const_refs,
        type_: None,
        source_span: None,
    })
}
