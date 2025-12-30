//! Resolve I18n Element Placeholders Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/resolve_i18n_element_placeholders.ts
//!
//! Resolve the element placeholders in i18n messages.

use crate::i18n::i18n_ast::{BlockPlaceholder, TagPlaceholder};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{I18nParamValueFlags, OpKind, TemplateKind};
use crate::template::pipeline::ir::ops::create::{
    ConditionalBranchCreateOp, ConditionalCreateOp, ElementStartOp, I18nContextOp, I18nParamValue,
    I18nParamValueValue, I18nStartOp, ProjectionOp, RepeaterCreateOp, TemplateOp,
};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, ComponentCompilationJob, ViewCompilationUnit,
};

/// Resolve the element placeholders in i18n messages.
pub fn resolve_i18n_element_placeholders(job: &mut dyn CompilationJob) {
    if job.kind() != CompilationJobKind::Tmpl {
        return;
    }

    let component_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let component_job_ptr = job_ptr as *mut ComponentCompilationJob;
        &mut *component_job_ptr
    };

    // Record all of the element and i18n context ops for use later.
    let mut i18n_contexts: std::collections::HashMap<ir::XrefId, I18nContextOp> =
        std::collections::HashMap::new();
    let mut elements: std::collections::HashMap<ir::XrefId, ElementStartOp> =
        std::collections::HashMap::new();

    // Collect from root unit
    for op in component_job.root.create.iter() {
        match op.kind() {
            OpKind::I18nContext => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let ctx_ptr = op_ptr as *const I18nContextOp;
                let ctx = &*ctx_ptr;
                i18n_contexts.insert(ctx.xref, ctx.clone());
            },
            OpKind::ElementStart => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let elem_ptr = op_ptr as *const ElementStartOp;
                let elem = &*elem_ptr;
                elements.insert(elem.base.base.xref, elem.clone());
            },
            _ => {}
        }
    }

    // Collect from all view units
    for (_, unit) in component_job.views.iter() {
        for op in unit.create.iter() {
            match op.kind() {
                OpKind::I18nContext => unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let ctx_ptr = op_ptr as *const I18nContextOp;
                    let ctx = &*ctx_ptr;
                    i18n_contexts.insert(ctx.xref, ctx.clone());
                },
                OpKind::ElementStart => unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let elem_ptr = op_ptr as *const ElementStartOp;
                    let elem = &*elem_ptr;
                    elements.insert(elem.base.base.xref, elem.clone());
                },
                _ => {}
            }
        }
    }

    // Use raw pointer to avoid borrow conflicts
    let job_ptr = component_job as *mut ComponentCompilationJob;
    resolve_placeholders_for_view(
        job_ptr,
        &mut component_job.root,
        &mut i18n_contexts,
        &elements,
        None,
    );
}

/// Helper function to handle template kind ops (ConditionalCreate, ConditionalBranchCreate, Template)
fn handle_template_kind_op(
    job: *mut ComponentCompilationJob,
    unit: &mut ViewCompilationUnit,
    op: &Box<dyn ir::CreateOp + Send + Sync>,
    op_kind: OpKind,
    op_xref: ir::XrefId,
    idx: usize,
    current_ops: &mut Option<(usize, usize)>,
    i18n_contexts: &mut std::collections::HashMap<ir::XrefId, I18nContextOp>,
    elements: &std::collections::HashMap<ir::XrefId, ElementStartOp>,
    pending_structural_directive: Option<usize>,
) {
    unsafe {
        let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
        let view_xref = op_xref;

        let job_ref = &mut *job;
        if let Some(view) = job_ref.views.get_mut(&view_xref) {
            let has_i18n_placeholder = match op_kind {
                OpKind::Template => {
                    let template_ptr = op_ptr as *const TemplateOp;
                    let template = &*template_ptr;
                    template.i18n_placeholder.is_some()
                }
                OpKind::ConditionalCreate => {
                    let cond_ptr = op_ptr as *const ConditionalCreateOp;
                    let cond = &*cond_ptr;
                    cond.i18n_placeholder.is_some()
                }
                OpKind::ConditionalBranchCreate => {
                    let branch_ptr = op_ptr as *const ConditionalBranchCreateOp;
                    let branch = &*branch_ptr;
                    branch.i18n_placeholder.is_some()
                }
                _ => false,
            };

            if !has_i18n_placeholder {
                // If there is no i18n placeholder, just recurse into the view in case it contains i18n blocks.
                resolve_placeholders_for_view(job, view, i18n_contexts, elements, None);
            } else {
                if let Some((i18n_idx, ctx_idx)) = *current_ops {
                    let i18n_block = unit.create.get(i18n_idx).unwrap();

                    let (i18n_placeholder, template_kind, slot) = unsafe {
                        let (i18n_placeholder, template_kind, slot) = match op_kind {
                            OpKind::Template => {
                                let template_ptr = op_ptr as *const TemplateOp;
                                let template = &*template_ptr;
                                (
                                    template.i18n_placeholder.as_ref(),
                                    template.template_kind,
                                    template.base.base.handle.get_slot().unwrap_or(0),
                                )
                            }
                            OpKind::ConditionalCreate => {
                                let cond_ptr = op_ptr as *const ConditionalCreateOp;
                                let cond = &*cond_ptr;
                                (
                                    cond.i18n_placeholder.as_ref(),
                                    cond.template_kind,
                                    cond.base.base.handle.get_slot().unwrap_or(0),
                                )
                            }
                            OpKind::ConditionalBranchCreate => {
                                let branch_ptr = op_ptr as *const ConditionalBranchCreateOp;
                                let branch = &*branch_ptr;
                                (
                                    branch.i18n_placeholder.as_ref(),
                                    branch.template_kind,
                                    branch.base.base.handle.get_slot().unwrap_or(0),
                                )
                            }
                            _ => (None, TemplateKind::Structural, 0),
                        };
                        (i18n_placeholder, template_kind, slot)
                    };

                    if template_kind == TemplateKind::Structural {
                        // If this is a structural directive template, don't record anything yet. Instead pass
                        // the current template as a pending structural directive to be recorded when we find
                        // the element, content, or template it belongs to.
                        resolve_placeholders_for_view(
                            job,
                            view,
                            i18n_contexts,
                            elements,
                            Some(idx),
                        );
                    } else {
                        // If this is some other kind of template, we can record its start, recurse into its
                        // view, and then record its end.
                        if let Some(ph) = i18n_placeholder {
                            // Use TagPlaceholder or BlockPlaceholder - need to handle both
                            let ph_ref: &dyn std::any::Any = ph;
                            let start_name = if let Some(tag_ph) =
                                ph_ref.downcast_ref::<TagPlaceholder>()
                            {
                                tag_ph.start_name.clone()
                            } else if let Some(block_ph) = ph_ref.downcast_ref::<BlockPlaceholder>()
                            {
                                block_ph.start_name.clone()
                            } else {
                                String::new()
                            };

                            let close_name = if let Some(tag_ph) =
                                ph_ref.downcast_ref::<TagPlaceholder>()
                            {
                                tag_ph.close_name.clone()
                            } else if let Some(block_ph) = ph_ref.downcast_ref::<BlockPlaceholder>()
                            {
                                block_ph.close_name.clone()
                            } else {
                                String::new()
                            };

                            // For now, treat as TagPlaceholder
                            let tag_ph = TagPlaceholder::new(
                                String::new(),
                                std::collections::HashMap::new(),
                                start_name,
                                close_name,
                                Vec::new(),
                                false,
                                crate::parse_util::ParseSourceSpan::new(
                                    crate::parse_util::ParseLocation::from_source(
                                        String::new(),
                                        String::new(),
                                        0,
                                        0,
                                        0,
                                    ),
                                    crate::parse_util::ParseLocation::from_source(
                                        String::new(),
                                        String::new(),
                                        0,
                                        0,
                                        0,
                                    ),
                                ),
                                None,
                                None,
                            );

                            let i18n_block_typed = unsafe {
                                let i18n_block_ptr = i18n_block.as_ref() as *const dyn ir::CreateOp;
                                &*(i18n_block_ptr as *const I18nStartOp)
                            };

                            // Record start
                            {
                                let i18n_context_mut =
                                    i18n_contexts.values_mut().nth(ctx_idx).unwrap();
                                record_template_start(
                                    job,
                                    view,
                                    slot,
                                    &tag_ph,
                                    i18n_context_mut,
                                    i18n_block_typed,
                                    pending_structural_directive.and_then(|pd_idx| {
                                        unit.create
                                            .get(pd_idx)
                                            .map(|pd_op| pd_op.as_any() as *const dyn std::any::Any)
                                    }),
                                );
                            }

                            resolve_placeholders_for_view(job, view, i18n_contexts, elements, None);

                            // Record close
                            {
                                let i18n_context_mut =
                                    i18n_contexts.values_mut().nth(ctx_idx).unwrap();
                                record_template_close(
                                    job,
                                    view,
                                    slot,
                                    &tag_ph,
                                    i18n_context_mut,
                                    i18n_block_typed,
                                    pending_structural_directive.and_then(|pd_idx| {
                                        unit.create
                                            .get(pd_idx)
                                            .map(|pd_op| pd_op.as_any() as *const dyn std::any::Any)
                                    }),
                                );
                            }
                        }
                    }
                } else {
                    panic!("i18n tag placeholder should only occur inside an i18n block");
                }
            }
        }
    }
}

/// Recursively resolves element and template tag placeholders in the given view.
fn resolve_placeholders_for_view(
    job: *mut ComponentCompilationJob,
    unit: &mut ViewCompilationUnit,
    i18n_contexts: &mut std::collections::HashMap<ir::XrefId, I18nContextOp>,
    elements: &std::collections::HashMap<ir::XrefId, ElementStartOp>,
    pending_structural_directive: Option<usize>, // Index of pending structural directive op
) {
    // Track the current i18n op and corresponding i18n context op as we step through the creation IR.
    let mut current_ops: Option<(usize, usize)> = None; // (i18n_block_idx, i18n_context_idx)
    let mut pending_structural_directive_closes: std::collections::HashMap<ir::XrefId, usize> =
        std::collections::HashMap::new();

    // Collect ops to process to avoid borrow conflicts
    let mut ops_to_process: Vec<(usize, OpKind, ir::XrefId)> = Vec::new();
    for (idx, op) in unit.create.iter().enumerate() {
        ops_to_process.push((idx, op.kind(), op.xref()));
    }

    for (idx, op_kind, op_xref) in ops_to_process {
        match op_kind {
            OpKind::I18nStart => {
                let op = unit.create.get(idx).unwrap();
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let i18n_ptr = op_ptr as *const I18nStartOp;
                    let i18n = &*i18n_ptr;

                    if i18n.base.context.is_none() {
                        panic!("Could not find i18n context for i18n op");
                    }

                    // Find context index
                    let context_xref = i18n.base.context.unwrap();
                    let context_idx = i18n_contexts
                        .keys()
                        .enumerate()
                        .find(|(_, &xref)| xref == context_xref)
                        .map(|(i, _)| i);

                    if let Some(ctx_idx) = context_idx {
                        current_ops = Some((idx, ctx_idx));
                    }
                }
            }
            OpKind::I18nEnd => {
                current_ops = None;
            }
            OpKind::ElementStart => {
                let op = unit.create.get(idx).unwrap();
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let elem_ptr = op_ptr as *const ElementStartOp;
                    let elem = &*elem_ptr;

                    if elem.i18n_placeholder.is_some() {
                        if let Some((i18n_idx, ctx_idx)) = current_ops {
                            let i18n_block = unit.create.get(i18n_idx).unwrap();
                            let _i18n_context = i18n_contexts.values().nth(ctx_idx).unwrap();

                            unsafe {
                                let i18n_block_ptr = i18n_block.as_ref() as *const dyn ir::CreateOp;
                                let i18n_block_typed = &*(i18n_block_ptr as *const I18nStartOp);
                                let i18n_context_mut =
                                    i18n_contexts.values_mut().nth(ctx_idx).unwrap();

                                record_element_start(
                                    elem,
                                    i18n_context_mut,
                                    i18n_block_typed,
                                    pending_structural_directive.and_then(|pd_idx| {
                                        unit.create
                                            .get(pd_idx)
                                            .map(|pd_op| pd_op.as_any() as *const dyn std::any::Any)
                                    }),
                                );

                                // If there is a separate close tag placeholder for this element, save the pending
                                // structural directive so we can pass it to the closing tag as well.
                                if let Some(ref ph) = elem.i18n_placeholder {
                                    if !ph.close_name.is_empty()
                                        && pending_structural_directive.is_some()
                                    {
                                        pending_structural_directive_closes.insert(
                                            elem.base.base.xref,
                                            pending_structural_directive.unwrap(),
                                        );
                                    }
                                }
                            }
                        } else {
                            panic!("i18n tag placeholder should only occur inside an i18n block");
                        }
                    }
                }
            }
            OpKind::ElementEnd => {
                let _op = unit.create.get(idx).unwrap();
                unsafe {
                    let elem_end_xref = op_xref;
                    if let Some(start_op) = elements.get(&elem_end_xref) {
                        if start_op.i18n_placeholder.is_some() {
                            if let Some((i18n_idx, ctx_idx)) = current_ops {
                                let i18n_block = unit.create.get(i18n_idx).unwrap();
                                let _i18n_context = i18n_contexts.values().nth(ctx_idx).unwrap();

                                unsafe {
                                    let i18n_block_ptr =
                                        i18n_block.as_ref() as *const dyn ir::CreateOp;
                                    let i18n_block_typed = &*(i18n_block_ptr as *const I18nStartOp);
                                    let i18n_context_mut =
                                        i18n_contexts.values_mut().nth(ctx_idx).unwrap();

                                    let pending_close = pending_structural_directive_closes
                                        .get(&elem_end_xref)
                                        .and_then(|pd_idx| {
                                            unit.create.get(*pd_idx).map(|pd_op| {
                                                pd_op.as_any() as *const dyn std::any::Any
                                            })
                                        });

                                    record_element_close(
                                        start_op,
                                        i18n_context_mut,
                                        i18n_block_typed,
                                        pending_close,
                                    );
                                    pending_structural_directive_closes.remove(&elem_end_xref);
                                }
                            } else {
                                panic!("AssertionError: i18n tag placeholder should only occur inside an i18n block");
                            }
                        }
                    }
                }
            }
            OpKind::Projection => {
                let op = unit.create.get(idx).unwrap();
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let proj_ptr = op_ptr as *const ProjectionOp;
                    let proj = &*proj_ptr;

                    if proj.i18n_placeholder.is_some() {
                        if let Some((i18n_idx, ctx_idx)) = current_ops {
                            let i18n_block = unit.create.get(i18n_idx).unwrap();
                            let _i18n_context = i18n_contexts.values().nth(ctx_idx).unwrap();

                            unsafe {
                                let i18n_block_ptr = i18n_block.as_ref() as *const dyn ir::CreateOp;
                                let i18n_block_typed = &*(i18n_block_ptr as *const I18nStartOp);
                                let i18n_context_mut =
                                    i18n_contexts.values_mut().nth(ctx_idx).unwrap();

                                record_element_start(
                                    proj,
                                    i18n_context_mut,
                                    i18n_block_typed,
                                    pending_structural_directive.and_then(|pd_idx| {
                                        unit.create
                                            .get(pd_idx)
                                            .map(|pd_op| pd_op.as_any() as *const dyn std::any::Any)
                                    }),
                                );

                                record_element_close(
                                    proj,
                                    i18n_context_mut,
                                    i18n_block_typed,
                                    pending_structural_directive.and_then(|pd_idx| {
                                        unit.create
                                            .get(pd_idx)
                                            .map(|pd_op| pd_op.as_any() as *const dyn std::any::Any)
                                    }),
                                );
                            }
                        } else {
                            panic!("i18n tag placeholder should only occur inside an i18n block");
                        }
                    }

                    if let Some(fallback_view_xref) = proj.fallback_view {
                        unsafe {
                            let job_ref = &mut *job;
                            if let Some(fallback_view) = job_ref.views.get_mut(&fallback_view_xref)
                            {
                                if proj.fallback_view_i18n_placeholder.is_none() {
                                    resolve_placeholders_for_view(
                                        job,
                                        fallback_view,
                                        i18n_contexts,
                                        elements,
                                        None,
                                    );
                                } else {
                                    if let Some((i18n_idx, ctx_idx)) = current_ops {
                                        let i18n_block = unit.create.get(i18n_idx).unwrap();

                                        let i18n_block_typed = unsafe {
                                            let i18n_block_ptr =
                                                i18n_block.as_ref() as *const dyn ir::CreateOp;
                                            &*(i18n_block_ptr as *const I18nStartOp)
                                        };

                                        let ph =
                                            proj.fallback_view_i18n_placeholder.as_ref().unwrap();
                                        let slot = proj.handle.get_slot().unwrap_or(0);

                                        // Record start
                                        {
                                            let i18n_context_mut =
                                                i18n_contexts.values_mut().nth(ctx_idx).unwrap();
                                            record_template_start(
                                                job,
                                                fallback_view,
                                                slot,
                                                ph,
                                                i18n_context_mut,
                                                i18n_block_typed,
                                                pending_structural_directive.and_then(|pd_idx| {
                                                    unit.create.get(pd_idx).map(|pd_op| {
                                                        pd_op.as_any() as *const dyn std::any::Any
                                                    })
                                                }),
                                            );
                                        }

                                        resolve_placeholders_for_view(
                                            job,
                                            fallback_view,
                                            i18n_contexts,
                                            elements,
                                            None,
                                        );

                                        // Record close
                                        {
                                            let i18n_context_mut =
                                                i18n_contexts.values_mut().nth(ctx_idx).unwrap();
                                            record_template_close(
                                                job,
                                                fallback_view,
                                                slot,
                                                ph,
                                                i18n_context_mut,
                                                i18n_block_typed,
                                                pending_structural_directive.and_then(|pd_idx| {
                                                    unit.create.get(pd_idx).map(|pd_op| {
                                                        pd_op.as_any() as *const dyn std::any::Any
                                                    })
                                                }),
                                            );
                                        }
                                    } else {
                                        panic!("i18n tag placeholder should only occur inside an i18n block");
                                    }
                                }
                            }
                        }
                    }
                }
            }
            OpKind::ConditionalCreate | OpKind::ConditionalBranchCreate | OpKind::Template => {
                // Get op reference and convert to raw pointer to avoid borrow conflicts
                let op_ref = unit.create.get(idx).unwrap();
                let op_ptr = op_ref.as_ref() as *const dyn ir::CreateOp;
                // Now we can borrow unit mutably
                let op = unsafe { &*(op_ptr as *const Box<dyn ir::CreateOp + Send + Sync>) };
                handle_template_kind_op(
                    job,
                    unit,
                    op,
                    op_kind,
                    op_xref,
                    idx,
                    &mut current_ops,
                    i18n_contexts,
                    elements,
                    pending_structural_directive,
                );
            }
            OpKind::RepeaterCreate => {
                let op = unit.create.get(idx).unwrap();
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let repeater_ptr = op_ptr as *const RepeaterCreateOp;
                    let repeater = &*repeater_ptr;

                    if pending_structural_directive.is_some() {
                        panic!("AssertionError: Unexpected structural directive associated with @for block");
                    }

                    // RepeaterCreate has 3 slots: the first is for the op itself, the second is for the @for
                    // template and the (optional) third is for the @empty template.
                    let for_slot = repeater.base.base.handle.get_slot().unwrap_or(0) + 1;
                    let for_view_xref = repeater.base.base.xref;

                    let job_ref = &mut *job;
                    if let Some(for_view) = job_ref.views.get_mut(&for_view_xref) {
                        // First record all of the placeholders for the @for template.
                        if repeater.i18n_placeholder.is_none() {
                            // If there is no i18n placeholder, just recurse into the view in case it contains i18n blocks.
                            resolve_placeholders_for_view(
                                job,
                                for_view,
                                i18n_contexts,
                                elements,
                                None,
                            );
                        } else {
                            if let Some((i18n_idx, ctx_idx)) = current_ops {
                                let i18n_block = unit.create.get(i18n_idx).unwrap();

                                let i18n_block_typed = unsafe {
                                    let i18n_block_ptr =
                                        i18n_block.as_ref() as *const dyn ir::CreateOp;
                                    &*(i18n_block_ptr as *const I18nStartOp)
                                };

                                if let Some(ref ph) = repeater.i18n_placeholder {
                                    // Record start
                                    {
                                        let i18n_context_mut =
                                            i18n_contexts.values_mut().nth(ctx_idx).unwrap();
                                        record_template_start(
                                            job,
                                            for_view,
                                            for_slot,
                                            ph,
                                            i18n_context_mut,
                                            i18n_block_typed,
                                            None,
                                        );
                                    }

                                    resolve_placeholders_for_view(
                                        job,
                                        for_view,
                                        i18n_contexts,
                                        elements,
                                        None,
                                    );

                                    // Record close
                                    {
                                        let i18n_context_mut =
                                            i18n_contexts.values_mut().nth(ctx_idx).unwrap();
                                        record_template_close(
                                            job,
                                            for_view,
                                            for_slot,
                                            ph,
                                            i18n_context_mut,
                                            i18n_block_typed,
                                            None,
                                        );
                                    }
                                }
                            } else {
                                panic!(
                                    "i18n tag placeholder should only occur inside an i18n block"
                                );
                            }
                        }
                    }

                    // Then if there's an @empty template, add its placeholders as well.
                    if let Some(empty_view_xref) = repeater.empty_view {
                        // RepeaterCreate has 3 slots: the first is for the op itself, the second is for the @for
                        // template and the (optional) third is for the @empty template.
                        let empty_slot = repeater.base.base.handle.get_slot().unwrap_or(0) + 2;

                        let job_ref = &mut *job;
                        if let Some(empty_view) = job_ref.views.get_mut(&empty_view_xref) {
                            if repeater.empty_i18n_placeholder.is_none() {
                                // If there is no i18n placeholder, just recurse into the view in case it contains i18n blocks.
                                resolve_placeholders_for_view(
                                    job,
                                    empty_view,
                                    i18n_contexts,
                                    elements,
                                    None,
                                );
                            } else {
                                if let Some((i18n_idx, ctx_idx)) = current_ops {
                                    let i18n_block = unit.create.get(i18n_idx).unwrap();

                                    let i18n_block_typed = unsafe {
                                        let i18n_block_ptr =
                                            i18n_block.as_ref() as *const dyn ir::CreateOp;
                                        &*(i18n_block_ptr as *const I18nStartOp)
                                    };

                                    if let Some(ref ph) = repeater.empty_i18n_placeholder {
                                        // Record start
                                        {
                                            let i18n_context_mut =
                                                i18n_contexts.values_mut().nth(ctx_idx).unwrap();
                                            record_template_start(
                                                job,
                                                empty_view,
                                                empty_slot,
                                                ph,
                                                i18n_context_mut,
                                                i18n_block_typed,
                                                None,
                                            );
                                        }

                                        resolve_placeholders_for_view(
                                            job,
                                            empty_view,
                                            i18n_contexts,
                                            elements,
                                            None,
                                        );

                                        // Record close
                                        {
                                            let i18n_context_mut =
                                                i18n_contexts.values_mut().nth(ctx_idx).unwrap();
                                            record_template_close(
                                                job,
                                                empty_view,
                                                empty_slot,
                                                ph,
                                                i18n_context_mut,
                                                i18n_block_typed,
                                                None,
                                            );
                                        }
                                    }
                                } else {
                                    panic!("i18n tag placeholder should only occur inside an i18n block");
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Records an i18n param value for the start of an element.
fn record_element_start(
    op: &dyn std::any::Any,
    i18n_context: &mut I18nContextOp,
    i18n_block: &I18nStartOp,
    structural_directive: Option<*const dyn std::any::Any>,
) {
    let (start_name, close_name, slot) = if let Some(elem) = op.downcast_ref::<ElementStartOp>() {
        if let Some(ref ph) = elem.i18n_placeholder {
            (
                ph.start_name.clone(),
                ph.close_name.clone(),
                elem.base.base.handle.get_slot().unwrap_or(0),
            )
        } else {
            return;
        }
    } else if let Some(proj) = op.downcast_ref::<ProjectionOp>() {
        if let Some(ref ph) = proj.i18n_placeholder {
            (
                ph.start_name.clone(),
                ph.close_name.clone(),
                proj.handle.get_slot().unwrap_or(0),
            )
        } else {
            return;
        }
    } else {
        return;
    };

    let mut flags = I18nParamValueFlags::ELEMENT_TAG | I18nParamValueFlags::OPEN_TAG;
    let mut value: I18nParamValueValue = I18nParamValueValue::Number(slot);

    // If the element is associated with a structural directive, start it as well.
    if let Some(struct_dir_ptr) = structural_directive {
        unsafe {
            flags |= I18nParamValueFlags::TEMPLATE_TAG;
            let struct_dir = &*struct_dir_ptr;
            let template_slot = if let Some(template) = struct_dir.downcast_ref::<TemplateOp>() {
                template.base.base.handle.get_slot().unwrap_or(0)
            } else if let Some(cond) = struct_dir.downcast_ref::<ConditionalCreateOp>() {
                cond.base.base.handle.get_slot().unwrap_or(0)
            } else if let Some(branch) = struct_dir.downcast_ref::<ConditionalBranchCreateOp>() {
                branch.base.base.handle.get_slot().unwrap_or(0)
            } else {
                return;
            };
            value = I18nParamValueValue::Compound {
                element: slot,
                template: template_slot,
            };
        }
    }

    // For self-closing tags, there is no close tag placeholder. Instead, the start tag
    // placeholder accounts for the start and close of the element.
    if close_name.is_empty() {
        flags |= I18nParamValueFlags::CLOSE_TAG;
    }

    add_param(
        &mut i18n_context.params,
        start_name,
        value,
        i18n_block.base.sub_template_index,
        flags,
    );
}

/// Records an i18n param value for the closing of an element.
fn record_element_close(
    op: &dyn std::any::Any,
    i18n_context: &mut I18nContextOp,
    i18n_block: &I18nStartOp,
    structural_directive: Option<*const dyn std::any::Any>,
) {
    let (close_name, slot) = if let Some(elem) = op.downcast_ref::<ElementStartOp>() {
        if let Some(ref ph) = elem.i18n_placeholder {
            (
                ph.close_name.clone(),
                elem.base.base.handle.get_slot().unwrap_or(0),
            )
        } else {
            return;
        }
    } else if let Some(proj) = op.downcast_ref::<ProjectionOp>() {
        if let Some(ref ph) = proj.i18n_placeholder {
            (ph.close_name.clone(), proj.handle.get_slot().unwrap_or(0))
        } else {
            return;
        }
    } else {
        return;
    };

    // Self-closing tags don't have a closing tag placeholder, instead the element closing is
    // recorded via an additional flag on the element start value.
    if !close_name.is_empty() {
        let mut flags = I18nParamValueFlags::ELEMENT_TAG | I18nParamValueFlags::CLOSE_TAG;
        let mut value: I18nParamValueValue = I18nParamValueValue::Number(slot);

        // If the element is associated with a structural directive, close it as well.
        if let Some(struct_dir_ptr) = structural_directive {
            unsafe {
                flags |= I18nParamValueFlags::TEMPLATE_TAG;
                let struct_dir = &*struct_dir_ptr;
                let template_slot = if let Some(template) = struct_dir.downcast_ref::<TemplateOp>()
                {
                    template.base.base.handle.get_slot().unwrap_or(0)
                } else if let Some(cond) = struct_dir.downcast_ref::<ConditionalCreateOp>() {
                    cond.base.base.handle.get_slot().unwrap_or(0)
                } else if let Some(branch) = struct_dir.downcast_ref::<ConditionalBranchCreateOp>()
                {
                    branch.base.base.handle.get_slot().unwrap_or(0)
                } else {
                    return;
                };
                value = I18nParamValueValue::Compound {
                    element: slot,
                    template: template_slot,
                };
            }
        }

        add_param(
            &mut i18n_context.params,
            close_name,
            value,
            i18n_block.base.sub_template_index,
            flags,
        );
    }
}

/// Records an i18n param value for the start of a template.
fn record_template_start(
    job: *mut ComponentCompilationJob,
    view: &ViewCompilationUnit,
    slot: usize,
    i18n_placeholder: &dyn std::any::Any,
    i18n_context: &mut I18nContextOp,
    i18n_block: &I18nStartOp,
    structural_directive: Option<*const dyn std::any::Any>,
) {
    let (start_name, close_name) =
        if let Some(tag_ph) = i18n_placeholder.downcast_ref::<TagPlaceholder>() {
            (tag_ph.start_name.clone(), tag_ph.close_name.clone())
        } else if let Some(block_ph) = i18n_placeholder.downcast_ref::<BlockPlaceholder>() {
            (block_ph.start_name.clone(), block_ph.close_name.clone())
        } else {
            return;
        };

    let mut flags = I18nParamValueFlags::TEMPLATE_TAG | I18nParamValueFlags::OPEN_TAG;

    // For self-closing tags, there is no close tag placeholder. Instead, the start tag
    // placeholder accounts for the start and close of the element.
    if close_name.is_empty() {
        flags |= I18nParamValueFlags::CLOSE_TAG;
    }

    // If the template is associated with a structural directive, record the structural directive's
    // start first. Since this template must be in the structural directive's view, we can just
    // directly use the current i18n block's sub-template index.
    if let Some(struct_dir_ptr) = structural_directive {
        unsafe {
            let struct_dir = &*struct_dir_ptr;
            let template_slot = if let Some(template) = struct_dir.downcast_ref::<TemplateOp>() {
                template.base.base.handle.get_slot().unwrap_or(0)
            } else if let Some(cond) = struct_dir.downcast_ref::<ConditionalCreateOp>() {
                cond.base.base.handle.get_slot().unwrap_or(0)
            } else if let Some(branch) = struct_dir.downcast_ref::<ConditionalBranchCreateOp>() {
                branch.base.base.handle.get_slot().unwrap_or(0)
            } else {
                return;
            };

            add_param(
                &mut i18n_context.params,
                start_name.clone(),
                I18nParamValueValue::Number(template_slot),
                i18n_block.base.sub_template_index,
                flags,
            );
        }
    }

    // Record the start of the template. For the sub-template index, pass the index for the template's
    // view, rather than the current i18n block's index.
    unsafe {
        add_param(
            &mut i18n_context.params,
            start_name,
            I18nParamValueValue::Number(slot),
            get_sub_template_index_for_template_tag(&*job, i18n_block, view),
            flags,
        );
    }
}

/// Records an i18n param value for the closing of a template.
fn record_template_close(
    job: *mut ComponentCompilationJob,
    view: &ViewCompilationUnit,
    slot: usize,
    i18n_placeholder: &dyn std::any::Any,
    i18n_context: &mut I18nContextOp,
    i18n_block: &I18nStartOp,
    structural_directive: Option<*const dyn std::any::Any>,
) {
    let close_name = if let Some(tag_ph) = i18n_placeholder.downcast_ref::<TagPlaceholder>() {
        tag_ph.close_name.clone()
    } else if let Some(block_ph) = i18n_placeholder.downcast_ref::<BlockPlaceholder>() {
        block_ph.close_name.clone()
    } else {
        return;
    };

    let flags = I18nParamValueFlags::TEMPLATE_TAG | I18nParamValueFlags::CLOSE_TAG;

    // Self-closing tags don't have a closing tag placeholder, instead the template's closing is
    // recorded via an additional flag on the template start value.
    if !close_name.is_empty() {
        // Record the closing of the template. For the sub-template index, pass the index for the
        // template's view, rather than the current i18n block's index.
        unsafe {
            add_param(
                &mut i18n_context.params,
                close_name.clone(),
                I18nParamValueValue::Number(slot),
                get_sub_template_index_for_template_tag(&*job, i18n_block, view),
                flags,
            );
        }

        // If the template is associated with a structural directive, record the structural directive's
        // closing after. Since this template must be in the structural directive's view, we can just
        // directly use the current i18n block's sub-template index.
        if let Some(struct_dir_ptr) = structural_directive {
            unsafe {
                let struct_dir = &*struct_dir_ptr;
                let template_slot = if let Some(template) = struct_dir.downcast_ref::<TemplateOp>()
                {
                    template.base.base.handle.get_slot().unwrap_or(0)
                } else if let Some(cond) = struct_dir.downcast_ref::<ConditionalCreateOp>() {
                    cond.base.base.handle.get_slot().unwrap_or(0)
                } else if let Some(branch) = struct_dir.downcast_ref::<ConditionalBranchCreateOp>()
                {
                    branch.base.base.handle.get_slot().unwrap_or(0)
                } else {
                    return;
                };

                add_param(
                    &mut i18n_context.params,
                    close_name,
                    I18nParamValueValue::Number(template_slot),
                    i18n_block.base.sub_template_index,
                    flags,
                );
            }
        }
    }
}

/// Get the subTemplateIndex for the given template op. For template ops, use the subTemplateIndex of
/// the child i18n block inside the template.
fn get_sub_template_index_for_template_tag(
    _job: &ComponentCompilationJob,
    i18n_op: &I18nStartOp,
    view: &ViewCompilationUnit,
) -> Option<usize> {
    for op in view.create.iter() {
        if op.kind() == OpKind::I18nStart {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let i18n_ptr = op_ptr as *const I18nStartOp;
                let i18n = &*i18n_ptr;
                return i18n.base.sub_template_index;
            }
        }
    }
    i18n_op.base.sub_template_index
}

/// Add a param value to the given params map.
fn add_param(
    params: &mut std::collections::HashMap<String, Vec<I18nParamValue>>,
    placeholder: String,
    value: I18nParamValueValue,
    sub_template_index: Option<usize>,
    flags: I18nParamValueFlags,
) {
    let values = params.entry(placeholder).or_insert_with(Vec::new);
    values.push(I18nParamValue {
        value,
        sub_template_index,
        flags,
    });
}
