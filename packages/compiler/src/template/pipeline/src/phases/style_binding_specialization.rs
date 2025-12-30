//! Style Binding Specialization Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/style_binding_specialization.ts
//! Transforms special-case bindings with 'style' or 'class' in their names. Must run before the
//! main binding specialization pass.

use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{BindingKind, OpKind};
use crate::template::pipeline::ir::ops::update::BindingOp;
use crate::template::pipeline::ir::ops::update::{
    create_class_map_op, create_class_prop_op, create_style_map_op, create_style_prop_op,
    BindingExpression,
};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob,
};

/// Transforms special-case bindings with 'style' or 'class' in their names. Must run before the
/// main binding specialization pass.
pub fn specialize_style_bindings(job: &mut dyn CompilationJob) {
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
    // Collect BindingOps to replace
    let mut ops_to_replace: Vec<(usize, BindingOp)> = Vec::new();

    // First pass: collect all BindingOps
    for (index, op) in unit.update().iter().enumerate() {
        if op.kind() == OpKind::Binding {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                let binding_op_ptr = op_ptr as *const BindingOp;
                let binding_op = &*binding_op_ptr;
                ops_to_replace.push((index, binding_op.clone()));
            }
        }
    }

    // Second pass: replace ops (iterate in reverse to maintain indices)
    for (index, binding_op) in ops_to_replace.iter().rev() {
        let replacement: Option<Box<dyn ir::UpdateOp + Send + Sync>> = match binding_op.binding_kind
        {
            BindingKind::ClassName => {
                // ClassName binding - expression must be Expression, not Interpolation
                match &binding_op.expression {
                    BindingExpression::Expression(expr) => Some(create_class_prop_op(
                        binding_op.target,
                        binding_op.name.clone(),
                        expr.clone(),
                        binding_op.source_span.clone(),
                    )),
                    BindingExpression::Interpolation(_) => {
                        panic!("Unexpected interpolation in ClassName binding");
                    }
                }
            }
            BindingKind::StyleProperty => {
                // StyleProperty binding - can be Expression or Interpolation
                Some(create_style_prop_op(
                    binding_op.target,
                    binding_op.name.clone(),
                    binding_op.expression.clone(),
                    binding_op.unit.clone(),
                    binding_op.source_span.clone(),
                ))
            }
            BindingKind::Property | BindingKind::Template => {
                // Property or Template binding - check if name is 'style' or 'class'
                if &*binding_op.name == "style" {
                    Some(create_style_map_op(
                        binding_op.target,
                        binding_op.expression.clone(),
                        binding_op.source_span.clone(),
                    ))
                } else if &*binding_op.name == "class" {
                    Some(create_class_map_op(
                        binding_op.target,
                        binding_op.expression.clone(),
                        binding_op.source_span.clone(),
                    ))
                } else {
                    // Not a style/class binding, skip
                    None
                }
            }
            _ => {
                // Other binding kinds are not handled by this phase
                None
            }
        };

        if let Some(new_op) = replacement {
            unit.update_mut().replace_at(*index, new_op);
        }
        // If replacement is None, leave the op as is (will be handled by binding_specialization)
    }
}
