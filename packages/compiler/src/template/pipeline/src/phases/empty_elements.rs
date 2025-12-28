//! Empty Elements Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/empty_elements.ts
//! Replace sequences of mergable instructions (e.g. `ElementStart` and `ElementEnd`) with a
//! consolidated instruction (e.g. `Element`).

use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::ops::create::{
    ContainerOp, ContainerStartOp, ElementOp, ElementOpBase, ElementStartOp, I18nOp, I18nStartOp,
};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, CompilationUnit, ComponentCompilationJob,
};

/// Op kinds that should not prevent merging of start/end ops.
const IGNORED_OP_KINDS: &[OpKind] = &[OpKind::Pipe];

/// Replace sequences of mergable instructions (e.g. `ElementStart` and `ElementEnd`) with a
/// consolidated instruction (e.g. `Element`).
pub fn collapse_empty_instructions(job: &mut dyn CompilationJob) {
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
        process_unit(&mut component_job.root);

        // Process all view units
        for (_, unit) in component_job.views.iter_mut() {
            process_unit(unit);
        }
    }
}

fn process_unit(unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit) {
    let mut indices_to_remove: Vec<usize> = Vec::new();

    // Store replacement data (index and data to create merged op)
    #[derive(Debug)]
    enum ReplacementData {
        Element {
            tag: Option<String>,
            namespace: crate::template::pipeline::ir::enums::Namespace,
            base: crate::template::pipeline::ir::ops::create::ElementOrContainerOpBase,
            i18n_placeholder: Option<crate::i18n::i18n_ast::TagPlaceholder>,
            has_directives: bool,
        },
        Container {
            base: crate::template::pipeline::ir::ops::create::ElementOrContainerOpBase,
        },
        I18n {
            base: crate::template::pipeline::ir::ops::create::I18nOpBase,
            source_span: Option<crate::parse_util::ParseSourceSpan>,
        },
    }
    let mut replacements: Vec<(usize, ReplacementData)> = Vec::new();

    // Iterate through ops to find end ops that can be merged
    for (index, op) in unit.create().iter().enumerate() {
        let (start_kind, _merged_kind) = match op.kind() {
            OpKind::ElementEnd => (OpKind::ElementStart, OpKind::Element),
            OpKind::ContainerEnd => (OpKind::ContainerStart, OpKind::Container),
            OpKind::I18nEnd => (OpKind::I18nStart, OpKind::I18n),
            _ => continue,
        };

        // Find previous non-ignored op
        let mut prev_index = index;
        while prev_index > 0 {
            prev_index -= 1;
            let prev_op = unit.create().get(prev_index);
            if let Some(prev_op) = prev_op {
                if IGNORED_OP_KINDS.contains(&prev_op.kind()) {
                    continue;
                }

                // Check if previous op is the corresponding start op
                if prev_op.kind() == start_kind && prev_op.xref() == op.xref() {
                    // Store replacement data
                    let replacement_data = unsafe {
                        match start_kind {
                            OpKind::ElementStart => {
                                let op_ptr = prev_op.as_ref() as *const dyn ir::CreateOp;
                                let elem_start_ptr = op_ptr as *const ElementStartOp;
                                let elem_start = &*elem_start_ptr;
                                ReplacementData::Element {
                                    tag: elem_start.base.tag.clone(),
                                    namespace: elem_start.base.namespace,
                                    base: elem_start.base.base.clone(),
                                    i18n_placeholder: elem_start.i18n_placeholder.clone(),
                                    has_directives: elem_start.base.has_directives,
                                }
                            }
                            OpKind::ContainerStart => {
                                let op_ptr = prev_op.as_ref() as *const dyn ir::CreateOp;
                                let container_start_ptr = op_ptr as *const ContainerStartOp;
                                let container_start = &*container_start_ptr;
                                ReplacementData::Container {
                                    base: container_start.base.clone(),
                                }
                            }
                            OpKind::I18nStart => {
                                let op_ptr = prev_op.as_ref() as *const dyn ir::CreateOp;
                                let i18n_start_ptr = op_ptr as *const I18nStartOp;
                                let i18n_start = &*i18n_start_ptr;
                                ReplacementData::I18n {
                                    base: i18n_start.base.clone(),
                                    source_span: i18n_start.source_span.clone(),
                                }
                            }
                            _ => continue,
                        }
                    };
                    replacements.push((prev_index, replacement_data));
                    indices_to_remove.push(index);
                    break;
                } else {
                    // Not the matching start op, can't merge
                    break;
                }
            } else {
                break;
            }
        }
    }

    // Sort indices_to_remove to adjust replacement indices correctly
    indices_to_remove.sort();

    // Remove end ops first (in reverse order)
    for index in indices_to_remove.iter().rev() {
        unit.create_mut().remove_at(*index);
    }

    // Then apply replacements (in reverse order, after adjusting indices)
    for (index, replacement_data) in replacements.iter().rev() {
        // Adjust index based on removals that came before it
        let removals_before = indices_to_remove.iter().filter(|&&r| r < *index).count();
        let adjusted_index = *index - removals_before;

        let merged_op = match replacement_data {
            ReplacementData::Element {
                tag,
                namespace,
                base,
                i18n_placeholder,
                has_directives,
            } => Box::new(ElementOp {
                base: ElementOpBase {
                    base: base.clone(),
                    tag: tag.clone(),
                    namespace: *namespace,
                    has_directives: *has_directives,
                },
                i18n_placeholder: i18n_placeholder.clone(),
            }) as Box<dyn ir::CreateOp + Send + Sync>,
            ReplacementData::Container { base } => {
                Box::new(ContainerOp { base: base.clone() }) as Box<dyn ir::CreateOp + Send + Sync>
            }
            ReplacementData::I18n { base, source_span } => Box::new(I18nOp {
                base: base.clone(),
                source_span: source_span.clone(),
            })
                as Box<dyn ir::CreateOp + Send + Sync>,
        };
        unit.create_mut().replace_at(adjusted_index, merged_op);
    }
}
