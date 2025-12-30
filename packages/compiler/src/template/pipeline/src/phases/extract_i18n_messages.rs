//! Extract I18n Messages Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/extract_i18n_messages.ts
//!
//! Formats the param maps on extracted message ops into a maps of `Expression` objects that can be
//! used in the final output.

use crate::output::output_ast::{Expression as OutputExpression, LiteralExpr, LiteralValue};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{I18nParamValueFlags, OpKind};
use crate::template::pipeline::ir::ops::create::{
    I18nContextOp, I18nMessageOp, I18nStartOp, IcuPlaceholderOp, IcuStartOp,
};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, ComponentCompilationJob,
};

/// The escape sequence used indicate message param values.
const ESCAPE: char = '\u{FFFD}';

/// Marker used to indicate an element tag.
const ELEMENT_MARKER: char = '#';

/// Marker used to indicate a template tag.
const TEMPLATE_MARKER: char = '*';

/// Marker used to indicate closing of an element or template tag.
const TAG_CLOSE_MARKER: char = '/';

/// Marker used to indicate the sub-template context.
const CONTEXT_MARKER: char = ':';

/// Marker used to indicate the start of a list of values.
const LIST_START_MARKER: char = '[';

/// Marker used to indicate the end of a list of values.
const LIST_END_MARKER: char = ']';

/// Delimiter used to separate multiple values in a list.
const LIST_DELIMITER: char = '|';

/// Formats the param maps on extracted message ops into a maps of `Expression` objects.
pub fn extract_i18n_messages(job: &mut dyn CompilationJob) {
    if job.kind() != CompilationJobKind::Tmpl {
        return;
    }

    let component_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let component_job_ptr = job_ptr as *mut ComponentCompilationJob;
        &mut *component_job_ptr
    };

    // Create an i18n message for each context.
    let mut i18n_messages_by_context: std::collections::HashMap<ir::XrefId, I18nMessageOp> =
        std::collections::HashMap::new();
    let mut i18n_blocks: std::collections::HashMap<ir::XrefId, I18nStartOp> =
        std::collections::HashMap::new();
    let mut i18n_contexts: std::collections::HashMap<ir::XrefId, I18nContextOp> =
        std::collections::HashMap::new();

    // Collect I18nContext and I18nStart ops
    // Process root unit - collect first
    let mut root_message_ops: Vec<(I18nMessageOp, I18nContextOp)> = Vec::new();
    for op in component_job.root.create.iter() {
        match op.kind() {
            OpKind::I18nContext => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let ctx_ptr = op_ptr as *const I18nContextOp;
                let ctx = &*ctx_ptr;

                let i18n_message_op = create_i18n_message(job, ctx.clone(), None);
                root_message_ops.push((i18n_message_op.clone(), ctx.clone()));
                i18n_messages_by_context.insert(ctx.xref, i18n_message_op);
                i18n_contexts.insert(ctx.xref, ctx.clone());
            },
            OpKind::I18nStart => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let i18n_ptr = op_ptr as *const I18nStartOp;
                let i18n = &*i18n_ptr;
                i18n_blocks.insert(i18n.base.xref, i18n.clone());
            },
            _ => {}
        }
    }

    // Push message ops to root unit
    for (message_op, _) in root_message_ops {
        component_job.root.create.push(Box::new(message_op));
    }

    // Process all view units
    for (_, unit) in component_job.views.iter() {
        for op in unit.create.iter() {
            match op.kind() {
                OpKind::I18nContext => {
                    unsafe {
                        let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                        let ctx_ptr = op_ptr as *const I18nContextOp;
                        let ctx = &*ctx_ptr;

                        let i18n_message_op = create_i18n_message(job, ctx.clone(), None);
                        // We'll add this to the unit later
                        i18n_messages_by_context.insert(ctx.xref, i18n_message_op);
                        i18n_contexts.insert(ctx.xref, ctx.clone());
                    }
                }
                OpKind::I18nStart => unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let i18n_ptr = op_ptr as *const I18nStartOp;
                    let i18n = &*i18n_ptr;
                    i18n_blocks.insert(i18n.base.xref, i18n.clone());
                },
                _ => {}
            }
        }
    }

    // Add I18nMessageOps to units - collect first
    let mut view_message_ops: Vec<(ir::XrefId, I18nMessageOp)> = Vec::new();
    for (_, unit) in component_job.views.iter() {
        for op in unit.create.iter() {
            if op.kind() == OpKind::I18nContext {
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let ctx_ptr = op_ptr as *const I18nContextOp;
                    let ctx = &*ctx_ptr;

                    if let Some(message_op) = i18n_messages_by_context.get(&ctx.xref) {
                        view_message_ops.push((ctx.xref, message_op.clone()));
                    }
                }
            }
        }
    }

    // Push message ops to view units - collect context xrefs first
    let mut view_context_xrefs: Vec<ir::XrefId> = Vec::new();
    for (_, unit) in component_job.views.iter() {
        for op in unit.create.iter() {
            if op.kind() == OpKind::I18nContext {
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let ctx_ptr = op_ptr as *const I18nContextOp;
                    let ctx = &*ctx_ptr;
                    view_context_xrefs.push(ctx.xref);
                }
            }
        }
    }

    // Push message ops
    for (_, unit) in component_job.views.iter_mut() {
        for ctx_xref in &view_context_xrefs {
            if let Some((_, message_op)) =
                view_message_ops.iter().find(|(xref, _)| *xref == *ctx_xref)
            {
                unit.create.push(Box::new(message_op.clone()));
            }
        }
    }

    // Associate sub-messages for ICUs with their root message. At this point we can also remove the
    // ICU start/end ops, as they are no longer needed.
    let mut current_icu: Option<usize> = None;
    let mut icu_indices_to_remove = Vec::new();
    let mut icu_placeholder_indices_to_remove = Vec::new();

    // Process root unit
    process_icus_for_unit(
        &mut component_job.root,
        &i18n_blocks,
        &i18n_contexts,
        &mut i18n_messages_by_context,
        &mut current_icu,
        &mut icu_indices_to_remove,
        &mut icu_placeholder_indices_to_remove,
    );

    // Process all view units
    for (_, unit) in component_job.views.iter_mut() {
        process_icus_for_unit(
            unit,
            &i18n_blocks,
            &i18n_contexts,
            &mut i18n_messages_by_context,
            &mut current_icu,
            &mut icu_indices_to_remove,
            &mut icu_placeholder_indices_to_remove,
        );
    }

    // Remove ICU ops
    // Process root unit
    icu_indices_to_remove.sort();
    icu_indices_to_remove.reverse();
    for idx in &icu_indices_to_remove {
        component_job.root.create.remove_at(*idx);
    }
    icu_placeholder_indices_to_remove.sort();
    icu_placeholder_indices_to_remove.reverse();
    for idx in &icu_placeholder_indices_to_remove {
        component_job.root.create.remove_at(*idx);
    }

    // Process all view units
    for (_, unit) in component_job.views.iter_mut() {
        // Note: We need to track indices per unit, but for simplicity, we'll process each unit separately
        let mut unit_icu_indices = Vec::new();
        let mut unit_icu_placeholder_indices = Vec::new();

        for (idx, op) in unit.create.iter().enumerate() {
            match op.kind() {
                OpKind::IcuStart | OpKind::IcuEnd => {
                    unit_icu_indices.push(idx);
                }
                OpKind::IcuPlaceholder => {
                    unit_icu_placeholder_indices.push(idx);
                }
                _ => {}
            }
        }

        unit_icu_indices.sort();
        unit_icu_indices.reverse();
        for idx in unit_icu_indices {
            unit.create.remove_at(idx);
        }

        unit_icu_placeholder_indices.sort();
        unit_icu_placeholder_indices.reverse();
        for idx in unit_icu_placeholder_indices {
            unit.create.remove_at(idx);
        }
    }
}

fn process_icus_for_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    i18n_blocks: &std::collections::HashMap<ir::XrefId, I18nStartOp>,
    i18n_contexts: &std::collections::HashMap<ir::XrefId, I18nContextOp>,
    i18n_messages_by_context: &mut std::collections::HashMap<ir::XrefId, I18nMessageOp>,
    current_icu: &mut Option<usize>,
    icu_indices_to_remove: &mut Vec<usize>,
    icu_placeholder_indices_to_remove: &mut Vec<usize>,
) {
    for (idx, op) in unit.create.iter().enumerate() {
        match op.kind() {
            OpKind::IcuStart => {
                *current_icu = Some(idx);
                icu_indices_to_remove.push(idx);

                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let icu_ptr = op_ptr as *const IcuStartOp;
                    let icu = &*icu_ptr;

                    // Skip any contexts not associated with an ICU.
                    if let Some(context_id) = icu.context {
                        if let Some(icu_context) = i18n_contexts.get(&context_id) {
                            if icu_context.context_kind != ir::enums::I18nContextKind::Icu {
                                continue;
                            }

                            // Skip ICUs that share a context with their i18n message. These represent root-level
                            // ICUs, not sub-messages.
                            if let Some(i18n_block_id) = icu_context.i18n_block {
                                if let Some(i18n_block) = i18n_blocks.get(&i18n_block_id) {
                                    if i18n_block.base.context == Some(context_id) {
                                        continue;
                                    }

                                    // Find the root message and push this ICUs message as a sub-message.
                                    if let Some(root_i18n_block) =
                                        i18n_blocks.get(&i18n_block.base.root)
                                    {
                                        if let Some(root_context_id) = root_i18n_block.base.context
                                        {
                                            // Collect sub_message xref first to avoid double mutable borrow
                                            let sub_message_xref = i18n_messages_by_context
                                                .get(&context_id)
                                                .map(|m| m.xref);

                                            if let Some(root_message) =
                                                i18n_messages_by_context.get_mut(&root_context_id)
                                            {
                                                if let Some(sub_xref) = sub_message_xref {
                                                    // Set message placeholder if available
                                                    // Note: IcuStartOp doesn't have message_placeholder field directly
                                                    // We'll need to find it from the message
                                                    root_message.sub_messages.push(sub_xref);
                                                }
                                            } else {
                                                panic!("AssertionError: ICU sub-message should belong to a root message.");
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            OpKind::IcuEnd => {
                *current_icu = None;
                icu_indices_to_remove.push(idx);
            }
            OpKind::IcuPlaceholder => {
                // Add ICU placeholders to the message, then remove the ICU placeholder ops.
                if let Some(icu_idx) = *current_icu {
                    unsafe {
                        let icu_op_ptr =
                            unit.create.get(icu_idx).unwrap().as_ref() as *const dyn ir::CreateOp;
                        let icu_ptr = icu_op_ptr as *const IcuStartOp;
                        let icu = &*icu_ptr;

                        if icu.context.is_none() {
                            panic!("AssertionError: Unexpected ICU placeholder outside of i18n context");
                        }

                        let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                        let icu_ph_ptr = op_ptr as *const IcuPlaceholderOp;
                        let icu_ph = &*icu_ph_ptr;

                        if let Some(msg) = i18n_messages_by_context.get_mut(&icu.context.unwrap()) {
                            let formatted = format_icu_placeholder(icu_ph);
                            msg.postprocessing_params.insert(
                                icu_ph.name.to_string(),
                                OutputExpression::Literal(LiteralExpr {
                                    value: LiteralValue::String(formatted),
                                    type_: None,
                                    source_span: None,
                                }),
                            );
                        }
                    }
                }
                icu_placeholder_indices_to_remove.push(idx);
            }
            _ => {}
        }
    }
}

/// Create an i18n message op from an i18n context op.
fn create_i18n_message(
    job: &mut dyn CompilationJob,
    context: I18nContextOp,
    message_placeholder: Option<String>,
) -> I18nMessageOp {
    let formatted_params = format_params(&context.params);
    let formatted_postprocessing_params = format_params(&context.postprocessing_params);
    let needs_postprocessing =
        context.params.values().any(|v| v.len() > 1) || !context.postprocessing_params.is_empty();

    I18nMessageOp::new(
        job.allocate_xref_id(),
        context.xref,
        context.i18n_block,
        context.message,
        message_placeholder,
        formatted_params,
        formatted_postprocessing_params,
        needs_postprocessing,
    )
}

/// Formats an ICU placeholder into a single string with expression placeholders.
fn format_icu_placeholder(op: &IcuPlaceholderOp) -> String {
    if op.strings.len() != op.expression_placeholders.len() + 1 {
        panic!(
            "AssertionError: Invalid ICU placeholder with {} strings and {} expressions",
            op.strings.len(),
            op.expression_placeholders.len()
        );
    }
    let values: Vec<String> = op
        .expression_placeholders
        .iter()
        .map(format_value)
        .collect();
    let mut result = String::new();
    for (i, str_val) in op.strings.iter().enumerate() {
        result.push_str(str_val);
        if i < values.len() {
            result.push_str(&values[i]);
        }
    }
    result
}

/// Formats a map of `I18nParamValue[]` values into a map of `Expression` values.
fn format_params(
    params: &std::collections::HashMap<String, Vec<ir::ops::create::I18nParamValue>>,
) -> std::collections::HashMap<String, OutputExpression> {
    let mut formatted_params = std::collections::HashMap::new();
    for (placeholder, placeholder_values) in params {
        let serialized_values = format_param_values(placeholder_values);
        if let Some(serialized) = serialized_values {
            formatted_params.insert(
                placeholder.clone(),
                OutputExpression::Literal(LiteralExpr {
                    value: LiteralValue::String(serialized),
                    type_: None,
                    source_span: None,
                }),
            );
        }
    }
    formatted_params
}

/// Formats an `I18nParamValue[]` into a string (or None for empty array).
fn format_param_values(values: &[ir::ops::create::I18nParamValue]) -> Option<String> {
    if values.is_empty() {
        return None;
    }
    let serialized_values: Vec<String> = values.iter().map(format_value).collect();
    Some(if serialized_values.len() == 1 {
        serialized_values[0].clone()
    } else {
        format!(
            "{}{}{}",
            LIST_START_MARKER,
            serialized_values.join(&LIST_DELIMITER.to_string()),
            LIST_END_MARKER
        )
    })
}

/// Formats a single `I18nParamValue` into a string
fn format_value(value: &ir::ops::create::I18nParamValue) -> String {
    use crate::template::pipeline::ir::ops::create::I18nParamValueValue;

    // Element tags with a structural directive use a special form that concatenates the element and
    // template values.
    if value.flags.contains(I18nParamValueFlags::ELEMENT_TAG)
        && value.flags.contains(I18nParamValueFlags::TEMPLATE_TAG)
    {
        if let I18nParamValueValue::Compound { element, template } = &value.value {
            let mut element_value = value.clone();
            element_value.value = I18nParamValueValue::Number(*element);
            element_value
                .flags
                .remove(I18nParamValueFlags::TEMPLATE_TAG);
            let element_str = format_value(&element_value);

            let mut template_value = value.clone();
            template_value.value = I18nParamValueValue::Number(*template);
            template_value
                .flags
                .remove(I18nParamValueFlags::ELEMENT_TAG);
            let template_str = format_value(&template_value);

            // TODO(mmalerba): This is likely a bug in TemplateDefinitionBuilder, we should not need to
            // record the template value twice. For now I'm re-implementing the behavior here to keep the
            // output consistent with TemplateDefinitionBuilder.
            if value.flags.contains(I18nParamValueFlags::OPEN_TAG)
                && value.flags.contains(I18nParamValueFlags::CLOSE_TAG)
            {
                return format!("{}{}{}", template_str, element_str, template_str);
            }
            // To match the TemplateDefinitionBuilder output, flip the order depending on whether the
            // values represent a closing or opening tag (or both).
            if value.flags.contains(I18nParamValueFlags::CLOSE_TAG) {
                return format!("{}{}", element_str, template_str);
            } else {
                return format!("{}{}", template_str, element_str);
            }
        } else {
            panic!(
                "AssertionError: Expected i18n param value to have an element and template slot"
            );
        }
    }

    // Self-closing tags use a special form that concatenates the start and close tag values.
    if value.flags.contains(I18nParamValueFlags::OPEN_TAG)
        && value.flags.contains(I18nParamValueFlags::CLOSE_TAG)
    {
        let mut start_value = value.clone();
        start_value.flags.remove(I18nParamValueFlags::CLOSE_TAG);
        let start_str = format_value(&start_value);

        let mut close_value = value.clone();
        close_value.flags.remove(I18nParamValueFlags::OPEN_TAG);
        let close_str = format_value(&close_value);

        return format!("{}{}", start_str, close_str);
    }

    // If there are no special flags, just return the raw value.
    if value.flags == I18nParamValueFlags::NONE {
        return match &value.value {
            I18nParamValueValue::String(s) => s.clone(),
            I18nParamValueValue::Number(n) => n.to_string(),
            I18nParamValueValue::Compound { .. } => String::new(),
        };
    }

    // Encode the remaining flags as part of the value.
    let tag_marker = if value.flags.contains(I18nParamValueFlags::ELEMENT_TAG) {
        ELEMENT_MARKER
    } else if value.flags.contains(I18nParamValueFlags::TEMPLATE_TAG) {
        TEMPLATE_MARKER
    } else {
        '\0'
    };

    let close_marker = if tag_marker != '\0' && value.flags.contains(I18nParamValueFlags::CLOSE_TAG)
    {
        TAG_CLOSE_MARKER
    } else {
        '\0'
    };

    let context = if let Some(sub_template_index) = value.sub_template_index {
        format!("{}{}", CONTEXT_MARKER, sub_template_index)
    } else {
        String::new()
    };

    let value_str = match &value.value {
        I18nParamValueValue::String(s) => s.clone(),
        I18nParamValueValue::Number(n) => n.to_string(),
        I18nParamValueValue::Compound { .. } => String::new(),
    };

    format!(
        "{}{}{}{}{}{}",
        ESCAPE, close_marker, tag_marker, value_str, context, ESCAPE
    )
}
