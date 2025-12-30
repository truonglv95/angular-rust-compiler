//! Deduplicate Text Bindings Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/deduplicate_text_bindings.ts
//! Deduplicate text bindings, e.g. <div class="cls1" class="cls2">

use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::ops::update::BindingOp;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob,
};
use std::collections::{HashMap, HashSet};

/// Deduplicate text bindings, e.g. <div class="cls1" class="cls2">
pub fn deduplicate_text_bindings(job: &mut dyn CompilationJob) {
    let component_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let job_ptr = job_ptr as *mut ComponentCompilationJob;
        &mut *job_ptr
    };

    let mut seen: HashMap<ir::XrefId, HashSet<std::sync::Arc<str>>> = HashMap::new();

    // Process root unit
    process_unit(&mut component_job.root, job, &mut seen);

    // Process all view units
    for (_, unit) in component_job.views.iter_mut() {
        process_unit(unit, job, &mut seen);
    }
}

fn process_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    job: &dyn CompilationJob,
    seen: &mut HashMap<ir::XrefId, HashSet<std::sync::Arc<str>>>,
) {
    let mut ops_to_remove: Vec<usize> = Vec::new();

    // Iterate in reverse order (same as TypeScript unit.update.reversed())
    for (index, op) in unit.update().iter().enumerate().rev() {
        if op.kind() == OpKind::Binding {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                let binding_op = &*(op_ptr as *const BindingOp);

                if binding_op.is_text_attribute {
                    let seen_for_element =
                        seen.entry(binding_op.target).or_insert_with(HashSet::new);

                    if seen_for_element.contains(&binding_op.name) {
                        if job.compatibility() == ir::CompatibilityMode::TemplateDefinitionBuilder {
                            // For most duplicated attributes, TemplateDefinitionBuilder lists all of the values in
                            // the consts array. However, for style and class attributes it only keeps the last one.
                            // We replicate that behavior here since it has actual consequences for apps with
                            // duplicate class or style attrs.
                            if &*binding_op.name == "style" || &*binding_op.name == "class" {
                                ops_to_remove.push(index);
                            }
                        }
                        // TODO: Determine the correct behavior. It would probably make sense to merge multiple
                        // style and class attributes. Alternatively we could just throw an error, as HTML
                        // doesn't permit duplicate attributes.
                    }

                    seen_for_element.insert(binding_op.name.clone());
                }
            }
        }
    }

    // Remove ops in reverse order (since we collected in reverse)
    for index in ops_to_remove {
        unit.update_mut().remove_at(index);
    }
}
