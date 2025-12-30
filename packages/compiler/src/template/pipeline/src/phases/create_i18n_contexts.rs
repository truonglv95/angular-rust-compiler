//! Create I18n Contexts Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/create_i18n_contexts.ts
//!
//! Create one helper context op per i18n block (including generate descending blocks).
//!
//! Also, if an ICU exists inside an i18n block that also contains other localizable content (such as
//! string), create an additional helper context op for the ICU.
//!
//! These context ops are later used for generating i18n messages. (Although we generate at least one
//! context op per nested view, we will collect them up the tree later, to generate a top-level
//! message.)

use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{I18nContextKind, OpKind};
use crate::template::pipeline::ir::ops::create::{
    create_i18n_context_op, I18nContextOp, I18nStartOp, IcuStartOp,
};
use crate::template::pipeline::ir::ops::update::{AttributeOp, BindingOp, PropertyOp};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, ComponentCompilationJob,
};

/// Create one helper context op per i18n block.
pub fn create_i18n_contexts(job: &mut dyn CompilationJob) {
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

        // Create i18n context ops for i18n attrs.
        // Use message.id as key since Message doesn't implement Hash/Eq
        let mut attr_context_by_message: std::collections::HashMap<String, ir::XrefId> =
            std::collections::HashMap::new();

        // Process root unit
        process_attrs_for_unit(&mut component_job.root, job, &mut attr_context_by_message);

        // Process all view units
        for (_, unit) in component_job.views.iter_mut() {
            process_attrs_for_unit(unit, job, &mut attr_context_by_message);
        }

        // Create i18n context ops for root i18n blocks.
        let mut block_context_by_i18n_block: std::collections::HashMap<ir::XrefId, I18nContextOp> =
            std::collections::HashMap::new();

        // Process root unit
        process_blocks_for_unit(
            &mut component_job.root,
            job,
            &mut block_context_by_i18n_block,
        );

        // Process all view units
        for (_, unit) in component_job.views.iter_mut() {
            process_blocks_for_unit(unit, job, &mut block_context_by_i18n_block);
        }

        // Assign i18n contexts for child i18n blocks. These don't need their own context, instead they
        // should inherit from their root i18n block.
        // Process root unit
        assign_child_contexts_for_unit(&mut component_job.root, &block_context_by_i18n_block);

        // Process all view units
        for (_, unit) in component_job.views.iter_mut() {
            assign_child_contexts_for_unit(unit, &block_context_by_i18n_block);
        }

        // Create or assign i18n contexts for ICUs.
        // Process root unit
        process_icus_for_unit(
            &mut component_job.root,
            job,
            &mut block_context_by_i18n_block,
        );

        // Process all view units
        for (_, unit) in component_job.views.iter_mut() {
            process_icus_for_unit(unit, job, &mut block_context_by_i18n_block);
        }
    }
}

fn process_attrs_for_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    job: &mut dyn CompilationJob,
    attr_context_by_message: &mut std::collections::HashMap<String, ir::XrefId>,
) {
    use crate::parse_util::ParseSourceSpan;

    // Collect indices and message IDs first
    let mut binding_indices: Vec<(
        usize,
        String,
        crate::i18n::i18n_ast::Message,
        ParseSourceSpan,
    )> = Vec::new();
    let mut property_indices: Vec<(
        usize,
        String,
        crate::i18n::i18n_ast::Message,
        ParseSourceSpan,
    )> = Vec::new();
    let mut attribute_indices: Vec<(
        usize,
        String,
        crate::i18n::i18n_ast::Message,
        ParseSourceSpan,
    )> = Vec::new();

    // Process update ops - collect first
    for (idx, op) in unit.update.iter().enumerate() {
        match op.kind() {
            OpKind::Binding => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                let binding_ptr = op_ptr as *const BindingOp;
                let binding = &*binding_ptr;

                if let Some(ref i18n_message) = binding.i18n_message {
                    binding_indices.push((
                        idx,
                        i18n_message.id.clone(),
                        i18n_message.clone(),
                        binding.source_span.clone(),
                    ));
                }
            },
            OpKind::Property => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                let prop_ptr = op_ptr as *const PropertyOp;
                let prop = &*prop_ptr;

                if let Some(ref i18n_message) = prop.i18n_message {
                    property_indices.push((
                        idx,
                        i18n_message.id.clone(),
                        i18n_message.clone(),
                        prop.source_span.clone(),
                    ));
                }
            },
            OpKind::Attribute => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                let attr_ptr = op_ptr as *const AttributeOp;
                let attr = &*attr_ptr;

                if let Some(ref i18n_message) = attr.i18n_message {
                    attribute_indices.push((
                        idx,
                        i18n_message.id.clone(),
                        i18n_message.clone(),
                        attr.source_span.clone(),
                    ));
                }
            },
            _ => {}
        }
    }

    // Create contexts and update ops
    for (idx, message_id, i18n_message, source_span) in binding_indices {
        if !attr_context_by_message.contains_key(&message_id) {
            let i18n_context = create_i18n_context_op(
                I18nContextKind::Attr,
                job.allocate_xref_id(),
                None,
                i18n_message.clone().into(),
                source_span.clone(),
            );
            let context_xref = i18n_context.xref();
            unit.create.push(i18n_context);
            attr_context_by_message.insert(message_id.clone(), context_xref);
        }

        // Set i18n_context on the binding op
        unsafe {
            let op_mut_ptr = unit.update.get_mut(idx).unwrap().as_mut() as *mut dyn ir::UpdateOp;
            let binding_mut_ptr = op_mut_ptr as *mut BindingOp;
            let binding_mut = &mut *binding_mut_ptr;
            binding_mut.i18n_context = attr_context_by_message.get(&message_id).copied();
        }
    }

    for (idx, message_id, i18n_message, source_span) in property_indices {
        if !attr_context_by_message.contains_key(&message_id) {
            let i18n_context = create_i18n_context_op(
                I18nContextKind::Attr,
                job.allocate_xref_id(),
                None,
                i18n_message.clone().into(),
                source_span.clone(),
            );
            let context_xref = i18n_context.xref();
            unit.create.push(i18n_context);
            attr_context_by_message.insert(message_id.clone(), context_xref);
        }

        unsafe {
            let op_mut_ptr = unit.update.get_mut(idx).unwrap().as_mut() as *mut dyn ir::UpdateOp;
            let prop_mut_ptr = op_mut_ptr as *mut PropertyOp;
            let prop_mut = &mut *prop_mut_ptr;
            prop_mut.i18n_context = attr_context_by_message.get(&message_id).copied();
        }
    }

    for (idx, message_id, i18n_message, source_span) in attribute_indices {
        if !attr_context_by_message.contains_key(&message_id) {
            let i18n_context = create_i18n_context_op(
                I18nContextKind::Attr,
                job.allocate_xref_id(),
                None,
                i18n_message.clone().into(),
                source_span.clone(),
            );
            let context_xref = i18n_context.xref();
            unit.create.push(i18n_context);
            attr_context_by_message.insert(message_id.clone(), context_xref);
        }

        unsafe {
            let op_mut_ptr = unit.update.get_mut(idx).unwrap().as_mut() as *mut dyn ir::UpdateOp;
            let attr_mut_ptr = op_mut_ptr as *mut AttributeOp;
            let attr_mut = &mut *attr_mut_ptr;
            attr_mut.i18n_context = attr_context_by_message.get(&message_id).copied();
        }
    }
}

fn process_blocks_for_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    job: &mut dyn CompilationJob,
    block_context_by_i18n_block: &mut std::collections::HashMap<ir::XrefId, I18nContextOp>,
) {
    // Collect root i18n ops first
    let mut root_i18n_ops: Vec<(
        usize,
        ir::XrefId,
        crate::i18n::i18n_ast::Message,
        crate::parse_util::ParseSourceSpan,
    )> = Vec::new();

    for (idx, op) in unit.create.iter().enumerate() {
        if op.kind() == OpKind::I18nStart {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let i18n_ptr = op_ptr as *const I18nStartOp;
                let i18n = &*i18n_ptr;

                if i18n.base.xref == i18n.base.root {
                    let default_source_span = crate::parse_util::ParseSourceSpan::new(
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
                    );

                    root_i18n_ops.push((
                        idx,
                        i18n.base.xref,
                        i18n.base.message.clone(),
                        i18n.source_span.clone().unwrap_or(default_source_span),
                    ));
                }
            }
        }
    }

    // Process root i18n ops
    for (idx, i18n_xref, message, source_span) in root_i18n_ops {
        let context_op = create_i18n_context_op(
            I18nContextKind::RootI18n,
            job.allocate_xref_id(),
            Some(i18n_xref),
            message.clone().into(),
            source_span,
        );
        let context_xref = context_op.xref();

        // Extract context_op fields before pushing
        let context_op_clone = unsafe {
            let op_ptr = &context_op as *const Box<dyn ir::CreateOp + Send + Sync>;
            let ctx_ptr = op_ptr as *const I18nContextOp;
            (*ctx_ptr).clone()
        };

        // Push context op
        unit.create.push(context_op);

        // Update I18nStartOp context
        unsafe {
            let op_mut_ptr = unit.create.get_mut(idx).unwrap().as_mut() as *mut dyn ir::CreateOp;
            let i18n_mut_ptr = op_mut_ptr as *mut I18nStartOp;
            let i18n_mut = &mut *i18n_mut_ptr;
            i18n_mut.base.context = Some(context_xref);
        }

        // Store in map
        block_context_by_i18n_block.insert(i18n_xref, context_op_clone);
    }
}

fn assign_child_contexts_for_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    block_context_by_i18n_block: &std::collections::HashMap<ir::XrefId, I18nContextOp>,
) {
    // Collect indices first to avoid borrow conflicts
    let mut indices_to_update: Vec<(usize, ir::XrefId)> = Vec::new();

    for (idx, op) in unit.create.iter().enumerate() {
        if op.kind() == OpKind::I18nStart {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let i18n_ptr = op_ptr as *const I18nStartOp;
                let i18n = &*i18n_ptr;

                if i18n.base.xref != i18n.base.root {
                    if let Some(root_context) = block_context_by_i18n_block.get(&i18n.base.root) {
                        indices_to_update.push((idx, root_context.xref));
                    } else {
                        panic!("AssertionError: Root i18n block i18n context should have been created.");
                    }
                }
            }
        }
    }

    // Update contexts
    for (idx, context_xref) in indices_to_update {
        unsafe {
            let op_mut_ptr = unit.create.get_mut(idx).unwrap().as_mut() as *mut dyn ir::CreateOp;
            let i18n_mut_ptr = op_mut_ptr as *mut I18nStartOp;
            let i18n_mut = &mut *i18n_mut_ptr;
            i18n_mut.base.context = Some(context_xref);
        }
    }
}

fn process_icus_for_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    job: &mut dyn CompilationJob,
    block_context_by_i18n_block: &mut std::collections::HashMap<ir::XrefId, I18nContextOp>,
) {
    // Collect ICU operations to process
    let mut current_i18n_op: Option<usize> = None;
    let mut icu_ops_to_process: Vec<(usize, usize, bool)> = Vec::new(); // (icu_idx, i18n_idx, is_sub_message)
    let mut icu_ops_to_update_context: Vec<usize> = Vec::new(); // icu_idx that need context update

    // First pass: collect ICU operations
    for (idx, op) in unit.create.iter().enumerate() {
        match op.kind() {
            OpKind::I18nStart => {
                current_i18n_op = Some(idx);
            }
            OpKind::I18nEnd => {
                current_i18n_op = None;
            }
            OpKind::IcuStart => {
                if let Some(i18n_idx) = current_i18n_op {
                    unsafe {
                        let i18n_op_ptr =
                            unit.create.get(i18n_idx).unwrap().as_ref() as *const dyn ir::CreateOp;
                        let i18n_ptr = i18n_op_ptr as *const I18nStartOp;
                        let i18n = &*i18n_ptr;

                        let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                        let icu_ptr = op_ptr as *const IcuStartOp;
                        let icu = &*icu_ptr;

                        let is_sub_message = icu.message.id != i18n.base.message.id;
                        icu_ops_to_process.push((idx, i18n_idx, is_sub_message));

                        if !is_sub_message {
                            icu_ops_to_update_context.push(idx);
                        }
                    }
                } else {
                    panic!("AssertionError: Unexpected ICU outside of an i18n block.");
                }
            }
            _ => {}
        }
    }

    // Second pass: process ICU operations
    for (icu_idx, i18n_idx, is_sub_message) in icu_ops_to_process {
        unsafe {
            let i18n_op_ptr =
                unit.create.get(i18n_idx).unwrap().as_ref() as *const dyn ir::CreateOp;
            let i18n_ptr = i18n_op_ptr as *const I18nStartOp;
            let i18n = &*i18n_ptr;

            let op_ptr = unit.create.get(icu_idx).unwrap().as_ref() as *const dyn ir::CreateOp;
            let icu_ptr = op_ptr as *const IcuStartOp;
            let icu = &*icu_ptr;

            if is_sub_message {
                // This ICU is a sub-message inside its parent i18n block message. We need to give it
                // its own context.
                let context_op = create_i18n_context_op(
                    I18nContextKind::Icu,
                    job.allocate_xref_id(),
                    Some(i18n.base.root),
                    icu.message.clone().into(),
                    icu.source_span.clone(),
                );
                let context_xref = context_op.xref();
                unit.create.push(context_op);

                // Set context on IcuStartOp
                let op_mut_ptr =
                    unit.create.get_mut(icu_idx).unwrap().as_mut() as *mut dyn ir::CreateOp;
                let icu_mut_ptr = op_mut_ptr as *mut IcuStartOp;
                let icu_mut = &mut *icu_mut_ptr;
                icu_mut.context = Some(context_xref);
            }
        }
    }

    // Third pass: update ICU contexts and context kinds
    // Collect context xrefs to update
    let mut context_updates: Vec<(ir::XrefId, ir::XrefId)> = Vec::new(); // (icu_xref, i18n_xref)
    let mut context_kind_updates: Vec<ir::XrefId> = Vec::new(); // i18n_xrefs that need context kind update

    for icu_idx in &icu_ops_to_update_context {
        let i18n_idx = {
            let mut found_idx = None;
            let mut current_i18n: Option<usize> = None;
            for (idx, op) in unit.create.iter().enumerate() {
                if idx == *icu_idx {
                    found_idx = current_i18n;
                    break;
                }
                match op.kind() {
                    OpKind::I18nStart => current_i18n = Some(idx),
                    OpKind::I18nEnd => current_i18n = None,
                    _ => {}
                }
            }
            found_idx.unwrap()
        };

        unsafe {
            let i18n_op_ptr =
                unit.create.get(i18n_idx).unwrap().as_ref() as *const dyn ir::CreateOp;
            let i18n_ptr = i18n_op_ptr as *const I18nStartOp;
            let i18n = &*i18n_ptr;

            let icu_op_ptr = unit.create.get(*icu_idx).unwrap().as_ref() as *const dyn ir::CreateOp;
            let icu_ptr = icu_op_ptr as *const IcuStartOp;
            let icu = &*icu_ptr;

            context_updates.push((icu.xref, i18n.base.xref));
            context_kind_updates.push(i18n.base.xref);
        }
    }

    // Collect i18n contexts first
    let mut i18n_contexts: std::collections::HashMap<ir::XrefId, Option<ir::XrefId>> =
        std::collections::HashMap::new();
    for op in unit.create.iter() {
        if op.kind() == OpKind::I18nStart {
            let i18n_xref = op.xref();
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let i18n_ptr = op_ptr as *const I18nStartOp;
                let i18n = &*i18n_ptr;
                i18n_contexts.insert(i18n_xref, i18n.base.context);
            }
        }
    }

    // Update ICU contexts
    for (icu_xref, i18n_xref) in context_updates {
        for op in unit.create.iter_mut() {
            if op.kind() == OpKind::IcuStart && op.xref() == icu_xref {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let icu_mut_ptr = op_ptr as *mut IcuStartOp;
                let icu_mut = unsafe { &mut *icu_mut_ptr };
                icu_mut.context = *i18n_contexts.get(&i18n_xref).unwrap_or(&None);
                break;
            }
        }
    }

    // Update context kinds
    for i18n_xref in context_kind_updates {
        // Update in HashMap
        if let Some(context_op) = block_context_by_i18n_block.get_mut(&i18n_xref) {
            context_op.context_kind = I18nContextKind::Icu;
        }

        // Update in unit
        for ctx_idx in 0..unit.create.len() {
            if let Some(ctx_op) = unit.create.get(ctx_idx) {
                if ctx_op.kind() == OpKind::I18nContext {
                    unsafe {
                        let ctx_op_ptr = ctx_op.as_ref() as *const dyn ir::CreateOp;
                        let ctx_ptr = ctx_op_ptr as *const I18nContextOp;
                        let ctx = &*ctx_ptr;

                        if let Some(context_op) = block_context_by_i18n_block.get(&i18n_xref) {
                            if ctx.xref == context_op.xref {
                                let ctx_op_mut_ptr = unit.create.get_mut(ctx_idx).unwrap().as_mut()
                                    as *mut dyn ir::CreateOp;
                                let ctx_mut_ptr = ctx_op_mut_ptr as *mut I18nContextOp;
                                let ctx_mut = &mut *ctx_mut_ptr;
                                ctx_mut.context_kind = I18nContextKind::Icu;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}
