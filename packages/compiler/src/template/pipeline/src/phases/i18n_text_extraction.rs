//! I18n Text Extraction Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/i18n_text_extraction.ts
//!
//! Removes text nodes within i18n blocks since they are already hardcoded into the i18n message.
//! Also, replaces interpolations on these text nodes with i18n expressions of the non-text portions,
//! which will be applied later.

use crate::output::output_ast::ExpressionTrait;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{I18nExpressionFor, I18nParamResolutionTime, OpKind};
use crate::template::pipeline::ir::ops::create::{
    create_icu_placeholder_op, I18nStartOp, IcuPlaceholderOp, IcuStartOp, TextOp,
};
use crate::template::pipeline::ir::ops::update::{create_i18n_expression_op, InterpolateTextOp};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, ComponentCompilationJob,
};

/// Removes text nodes within i18n blocks and replaces interpolations with i18n expressions.
pub fn convert_i18n_text(job: &mut dyn CompilationJob) {
    let job_kind = job.kind();

    if matches!(
        job_kind,
        CompilationJobKind::Tmpl | CompilationJobKind::Both
    ) {
        let component_job = unsafe {
            let job_ptr = job as *mut dyn CompilationJob;
            let component_job_ptr = job_ptr as *mut ComponentCompilationJob;
            &mut *component_job_ptr
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
    // Remove all text nodes within i18n blocks, their content is already captured in the i18n
    // message.
    let mut current_i18n: Option<usize> = None;
    let mut current_icu: Option<usize> = None;
    let mut text_node_i18n_blocks: std::collections::HashMap<ir::XrefId, usize> =
        std::collections::HashMap::new();
    let mut text_node_icus: std::collections::HashMap<ir::XrefId, Option<usize>> =
        std::collections::HashMap::new();
    let mut icu_placeholder_by_text: std::collections::HashMap<ir::XrefId, ir::XrefId> =
        std::collections::HashMap::new();
    let mut text_indices_to_remove = Vec::new();
    let mut text_indices_to_replace: Vec<(usize, Box<dyn ir::CreateOp + Send + Sync>)> = Vec::new();

    // First pass: collect text nodes and their i18n/icu context
    for (idx, op) in unit.create.iter().enumerate() {
        match op.kind() {
            OpKind::I18nStart => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let i18n_ptr = op_ptr as *const I18nStartOp;
                let i18n = &*i18n_ptr;

                if i18n.base.context.is_none() {
                    panic!("I18n op should have its context set.");
                }
                current_i18n = Some(idx);
            },
            OpKind::I18nEnd => {
                current_i18n = None;
            }
            OpKind::IcuStart => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let icu_ptr = op_ptr as *const IcuStartOp;
                let icu = &*icu_ptr;

                if icu.context.is_none() {
                    panic!("Icu op should have its context set.");
                }
                current_icu = Some(idx);
            },
            OpKind::IcuEnd => {
                current_icu = None;
            }
            OpKind::Text => {
                if let Some(i18n_idx) = current_i18n {
                    unsafe {
                        let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                        let text_ptr = op_ptr as *const TextOp;
                        let text = &*text_ptr;

                        text_node_i18n_blocks.insert(text.xref, i18n_idx);
                        text_node_icus.insert(text.xref, current_icu);

                        if let Some(ref icu_placeholder) = text.icu_placeholder {
                            // Create an op to represent the ICU placeholder. Initially set its static text to the
                            // value of the text op, though this may be overwritten later if this text op is a
                            // placeholder for an interpolation.
                            let icu_placeholder_op = create_icu_placeholder_op(
                                job.allocate_xref_id(),
                                icu_placeholder.clone().into(),
                                vec![text.initial_value.clone()],
                            );
                            let icu_placeholder_xref = icu_placeholder_op.xref();
                            text_indices_to_replace.push((idx, icu_placeholder_op));
                            icu_placeholder_by_text.insert(text.xref, icu_placeholder_xref);
                        } else {
                            // Otherwise just remove the text op, since its value is already accounted for in the
                            // translated message.
                            text_indices_to_remove.push(idx);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Replace text ops with ICU placeholder ops
    text_indices_to_replace.sort_by(|a, b| b.0.cmp(&a.0));
    for (idx, new_op) in text_indices_to_replace {
        unit.create.replace_at(idx, new_op);
    }

    // Remove text ops
    text_indices_to_remove.sort();
    text_indices_to_remove.reverse();
    for idx in text_indices_to_remove {
        unit.create.remove_at(idx);
    }

    // Update any interpolations to the removed text, and instead represent them as a series of i18n
    // expressions that we then apply.
    let mut interpolate_indices_to_replace: Vec<(usize, Vec<Box<dyn ir::UpdateOp + Send + Sync>>)> =
        Vec::new();
    let mut icu_placeholder_updates: Vec<(ir::XrefId, Vec<String>)> = Vec::new();

    for (idx, op) in unit.update.iter().enumerate() {
        if op.kind() == OpKind::InterpolateText {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                let interpolate_ptr = op_ptr as *const InterpolateTextOp;
                let interpolate = &*interpolate_ptr;

                if let Some(&i18n_idx) = text_node_i18n_blocks.get(&interpolate.target) {
                    let i18n_op_ptr =
                        unit.create.get(i18n_idx).unwrap().as_ref() as *const dyn ir::CreateOp;
                    let i18n_ptr = i18n_op_ptr as *const I18nStartOp;
                    let i18n_op = unsafe { &*i18n_ptr };

                    let icu_idx = text_node_icus.get(&interpolate.target).copied().flatten();
                    let icu_op = if let Some(icu_idx) = icu_idx {
                        let icu_op_ptr =
                            unit.create.get(icu_idx).unwrap().as_ref() as *const dyn ir::CreateOp;
                        let icu_ptr = icu_op_ptr as *const IcuStartOp;
                        Some(unsafe { &*icu_ptr })
                    } else {
                        None
                    };

                    let context_id = if let Some(icu) = icu_op {
                        icu.context
                    } else {
                        i18n_op.base.context
                    };

                    if context_id.is_none() {
                        continue;
                    }

                    let resolution_time = if icu_op.is_some() {
                        I18nParamResolutionTime::Postprocessing
                    } else {
                        I18nParamResolutionTime::Creation
                    };

                    let icu_placeholder_xref =
                        icu_placeholder_by_text.get(&interpolate.target).copied();

                    let mut new_ops: Vec<Box<dyn ir::UpdateOp + Send + Sync>> = Vec::new();
                    for (i, expr) in interpolate.interpolation.expressions.iter().enumerate() {
                        // For now, this i18nExpression depends on the slot context of the enclosing i18n block.
                        // Later, we will modify this, and advance to a different point.
                        let i18n_expr_op = create_i18n_expression_op(
                            context_id.unwrap(),
                            i18n_op.base.xref,
                            i18n_op.base.xref,
                            i18n_op.base.handle.clone(),
                            expr.clone(),
                            icu_placeholder_xref,
                            interpolate
                                .interpolation
                                .i18n_placeholders
                                .get(i)
                                .cloned()
                                .map(|s| s.to_string()),
                            resolution_time,
                            I18nExpressionFor::I18nText,
                            String::new().into(),
                            expr.source_span()
                                .cloned()
                                .unwrap_or_else(|| interpolate.source_span.clone()),
                        );
                        new_ops.push(i18n_expr_op);
                    }

                    interpolate_indices_to_replace.push((idx, new_ops));

                    // If this interpolation is part of an ICU placeholder, add the strings and expressions to
                    // the placeholder.
                    if let Some(icu_placeholder_xref) = icu_placeholder_xref {
                        icu_placeholder_updates.push((
                            icu_placeholder_xref,
                            interpolate
                                .interpolation
                                .strings
                                .iter()
                                .map(|s| s.to_string())
                                .collect(),
                        ));
                    }
                }
            }
        }
    }

    // Replace interpolate ops
    interpolate_indices_to_replace.sort_by(|a, b| b.0.cmp(&a.0));
    for (idx, new_ops) in interpolate_indices_to_replace {
        unit.update.replace_at_with_many(idx, new_ops);
    }

    // Update ICU placeholder strings
    for (icu_placeholder_xref, strings) in icu_placeholder_updates {
        for op in unit.create.iter_mut() {
            if op.kind() == OpKind::IcuPlaceholder && op.xref() == icu_placeholder_xref {
                unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let icu_ph_ptr = op_ptr as *mut IcuPlaceholderOp;
                    let icu_ph = &mut *icu_ph_ptr;
                    icu_ph.strings = strings;
                }
                break;
            }
        }
    }
}
