//! Ng Container Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/ng_container.ts
//! Replace an `Element` or `ElementStart` whose tag is `ng-container` with a specific op.

use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::ops::create::{
    ContainerEndOp, ContainerStartOp, ElementEndOp, ElementStartOp,
};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob,
};
use std::collections::HashSet;

const CONTAINER_TAG: &str = "ng-container";

/// Replace an `Element` or `ElementStart` whose tag is `ng-container` with a specific op.
pub fn generate_ng_container_ops(job: &mut dyn CompilationJob) {
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
    let mut updated_element_xrefs: HashSet<ir::XrefId> = HashSet::new();
    let mut start_replacements: Vec<(usize, ContainerStartOp)> = Vec::new();
    let mut end_replacements: Vec<(usize, ContainerEndOp)> = Vec::new();

    // First pass: collect ElementStart ops to convert
    for (index, op) in unit.create().iter().enumerate() {
        if op.kind() == OpKind::ElementStart {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let elem_start_ptr = op_ptr as *const ElementStartOp;
                let elem_start = &*elem_start_ptr;

                if elem_start.base.tag.as_deref() == Some(CONTAINER_TAG) {
                    if let Some(slot) = elem_start.base.base.handle.clone().get_slot() {
                        // Already slotted, nothing to do
                        return;
                    }
                    let xref = elem_start.base.base.xref;
                    let start_source_span = elem_start.base.base.start_source_span.clone();
                    let whole_source_span = elem_start.base.base.whole_source_span.clone();

                    // Create ContainerStartOp to replace ElementStartOp
                    let mut container_start =
                        ContainerStartOp::new(xref, start_source_span, whole_source_span);
                    // Copy attributes and local_refs from ElementStartOp
                    container_start.base.attributes = elem_start.base.base.attributes;
                    container_start.base.local_refs = elem_start.base.base.local_refs.clone();
                    container_start.base.non_bindable = elem_start.base.base.non_bindable;
                    container_start.base.handle = elem_start.base.base.handle.clone();

                    start_replacements.push((index, container_start));
                    updated_element_xrefs.insert(xref);
                }
            }
        }
    }

    // Apply start replacements in reverse order
    for (index, container_start) in start_replacements.iter().rev() {
        let new_op = Box::new(container_start.clone()) as Box<dyn ir::CreateOp + Send + Sync>;
        unit.create_mut().replace_at(*index, new_op);
    }

    // Second pass: convert ElementEnd to ContainerEnd
    for (index, op) in unit.create().iter().enumerate() {
        if op.kind() == OpKind::ElementEnd {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let elem_end_ptr = op_ptr as *const ElementEndOp;
                let elem_end = &*elem_end_ptr;

                if updated_element_xrefs.contains(&elem_end.xref) {
                    // This `ElementEnd` is associated with an `ElementStart` we already transmuted
                    // Note: ElementEndOp.source_span is Option<ParseSourceSpan>, but ContainerEndOp needs ParseSourceSpan
                    // We need to provide a default source span if None
                    if let Some(source_span) = elem_end.source_span.clone() {
                        let container_end = ContainerEndOp::new(elem_end.xref, source_span);
                        end_replacements.push((index, container_end));
                    } else {
                        // If no source span, create a default one (same as ElementEndOp would have)
                        let default_source_span = crate::parse_util::ParseSourceSpan::new(
                            crate::parse_util::ParseLocation::from_source(
                                "".to_string(),
                                "".to_string(),
                                0,
                                0,
                                0,
                            ),
                            crate::parse_util::ParseLocation::from_source(
                                "".to_string(),
                                "".to_string(),
                                0,
                                0,
                                0,
                            ),
                        );
                        let container_end = ContainerEndOp::new(elem_end.xref, default_source_span);
                        end_replacements.push((index, container_end));
                    }
                }
            }
        }
    }

    // Apply end replacements in reverse order
    for (index, container_end) in end_replacements.iter().rev() {
        let new_op = Box::new(container_end.clone()) as Box<dyn ir::CreateOp + Send + Sync>;
        unit.create_mut().replace_at(*index, new_op);
    }
}
