//! Slot Allocation Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/slot_allocation.ts
//! Assigns data slots for all operations which implement `ConsumesSlotOpTrait`, and propagates the
//! assigned data slots of those operations to any expressions which reference them via
//! `UsesSlotIndexTrait`.

use crate::template::pipeline::ir;
use crate::template::pipeline::src::compilation::{
    ComponentCompilationJob, CompilationUnit,
};
use crate::template::pipeline::src::util::elements::op_kind_has_consumes_slot_trait;
use crate::template::pipeline::ir::traits::ConsumesSlotOpTrait;

/// Assign data slots for all operations which implement `ConsumesSlotOpTrait`, and propagate the
/// assigned data slots of those operations to any expressions which reference them via
/// `UsesSlotIndexTrait`.
///
/// This phase is also responsible for counting the number of slots used for each view (its `decls`)
/// and propagating that number into the `Template` operations which declare embedded views.
pub fn phase(job: &mut ComponentCompilationJob) {
    // Map of all declarations in all views within the component which require an assigned slot index.
    // This map needs to be global (across all views within the component) since it's possible to
    // reference a slot from one view from an expression within another (e.g. local references work
    // this way).
    let mut slot_map: std::collections::HashMap<ir::XrefId, usize> = std::collections::HashMap::new();

    // Process all views in the component and assign slot indexes.
    // First, process root view
    {
        let unit = &mut job.root;
        let mut slot_count = 0;

        for op in unit.create_mut().iter_mut() {
            // Only consider declarations which consume data slots.
            if !op_kind_has_consumes_slot_trait(op.kind()) {
                continue;
            }

            // Get xref before borrowing mutably
            let xref = op.xref();

            // Assign slots to this declaration starting at the current `slotCount`.
            if let Some((handle, num_slots)) = get_slot_handle_and_num_slots_mut(op.as_mut()) {
                handle.slot = Some(slot_count);
                
                // And track its assigned slot in the `slotMap`.
                slot_map.insert(xref, handle.slot.unwrap());
                
                // Each declaration may use more than 1 slot, so increment `slotCount` to reserve the number
                // of slots required.
                slot_count += num_slots;
            }
        }

        // Record the total number of slots used on the view itself.
        unit.decls = Some(slot_count);
    }

    // Process all other views
    for (_, unit) in job.views.iter_mut() {
        let mut slot_count = 0;

        for op in unit.create_mut().iter_mut() {
            // Only consider declarations which consume data slots.
            if !op_kind_has_consumes_slot_trait(op.kind()) {
                continue;
            }

            // Get xref before borrowing mutably
            let xref = op.xref();

            // Assign slots to this declaration starting at the current `slotCount`.
            if let Some((handle, num_slots)) = get_slot_handle_and_num_slots_mut(op.as_mut()) {
                handle.slot = Some(slot_count);
                
                // And track its assigned slot in the `slotMap`.
                slot_map.insert(xref, handle.slot.unwrap());
                
                // Each declaration may use more than 1 slot, so increment `slotCount` to reserve the number
                // of slots required.
                slot_count += num_slots;
            }
        }

        // Record the total number of slots used on the view itself.
        unit.decls = Some(slot_count);
    }

    // After slot assignment, `slotMap` now contains slot assignments for every declaration in the
    // whole template, across all views. Next, look for expressions which implement
    // `UsesSlotIndexExprTrait` and propagate the assigned slot indexes into them.
    // Additionally, this second scan allows us to find `ir.TemplateOp`s which declare views and
    // propagate the number of slots used for each view into the operation which declares it.
    
    // Process root view
    {
        let unit = &mut job.root;
        for op in unit.create_mut().iter_mut() {
            if matches!(
                op.kind(),
                ir::OpKind::Template
                    | ir::OpKind::ConditionalCreate
                    | ir::OpKind::ConditionalBranchCreate
                    | ir::OpKind::RepeaterCreate
            ) {
                // Record the number of slots used by the view this op declares in the
                // operation itself, so it can be emitted later.
                let xref = op.xref();
                if let Some(child_view) = job.views.get(&xref) {
                    let decls = child_view.decls.unwrap_or(0);
                    set_decls_on_op(op.as_mut(), decls);
                }
            }
        }
    }

    // Process all other views
    // Collect xrefs first to avoid borrow conflicts
    let mut decls_to_set: Vec<(ir::XrefId, usize)> = Vec::new();
    for (_, unit) in job.views.iter() {
        for op in unit.create() {
            if matches!(
                op.kind(),
                ir::OpKind::Template
                    | ir::OpKind::ConditionalCreate
                    | ir::OpKind::ConditionalBranchCreate
                    | ir::OpKind::RepeaterCreate
            ) {
                let xref = op.xref();
                if let Some(child_view) = job.views.get(&xref) {
                    let decls = child_view.decls.unwrap_or(0);
                    decls_to_set.push((xref, decls));
                }
            }
        }
    }
    
    // Now set decls on ops
    for (_, unit) in job.views.iter_mut() {
        for op in unit.create_mut().iter_mut() {
            let op_xref = op.xref();
            if let Some((_, decls)) = decls_to_set.iter().find(|(xref, _)| *xref == op_xref) {
                set_decls_on_op(op.as_mut(), *decls);
            }
        }
    }

    // Propagate slot indexes to expressions which implement `UsesSlotIndexExprTrait`
    // Specifically, propagate slot indexes to `ReferenceExpr` and `ContextLetReferenceExpr`
    // expressions based on their `target` XrefId.
    
    // Process root view
    {
        let unit = &mut job.root;
        propagate_slot_indexes_in_unit(unit, &slot_map);
    }
    
    // Process all other views
    for (_, unit) in job.views.iter_mut() {
        propagate_slot_indexes_in_unit(unit, &slot_map);
    }
}

/// Propagate slot indexes from slot_map to expressions in a unit
fn propagate_slot_indexes_in_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    slot_map: &std::collections::HashMap<ir::XrefId, usize>,
) {
    use crate::template::pipeline::ir::expression::transform_expressions_in_op;
    use crate::output::output_ast::Expression;
    
    // Process create ops
    for op in unit.create_mut().iter_mut() {
        transform_expressions_in_op(
            &mut **op,
            &mut |mut expr: Expression, _flags| {
                match expr {
                    Expression::Reference(ref mut ref_expr) => {
                        // Propagate slot index from slot_map if target exists
                        if ref_expr.target_slot.slot.is_none() {
                            if let Some(&slot) = slot_map.get(&ref_expr.target) {
                                ref_expr.target_slot.slot = Some(slot);
                            }
                        }
                    }
                    Expression::ContextLetReference(ref mut ctx_let_ref) => {
                        // Propagate slot index from slot_map if target exists
                        if ctx_let_ref.target_slot.slot.is_none() {
                            if let Some(&slot) = slot_map.get(&ctx_let_ref.target) {
                                ctx_let_ref.target_slot.slot = Some(slot);
                            }
                        }
                    }
                    _ => {}
                }
                expr
            },
            ir::VisitorContextFlag::NONE,
        );
    }
    
    // Process update ops
    for op in unit.update_mut().iter_mut() {
        transform_expressions_in_op(
            &mut **op,
            &mut |mut expr: Expression, _flags| {
                match expr {
                    Expression::Reference(ref mut ref_expr) => {
                        // Propagate slot index from slot_map if target exists
                        if ref_expr.target_slot.slot.is_none() {
                            if let Some(&slot) = slot_map.get(&ref_expr.target) {
                                ref_expr.target_slot.slot = Some(slot);
                            }
                        }
                    }
                    Expression::ContextLetReference(ref mut ctx_let_ref) => {
                        // Propagate slot index from slot_map if target exists
                        if ctx_let_ref.target_slot.slot.is_none() {
                            if let Some(&slot) = slot_map.get(&ctx_let_ref.target) {
                                ctx_let_ref.target_slot.slot = Some(slot);
                            }
                        }
                    }
                    _ => {}
                }
                expr
            },
            ir::VisitorContextFlag::NONE,
        );
    }
    
    // Also need to process expressions in nested operations (listeners, repeater trackByOps)
    for op in unit.create_mut().iter_mut() {
        match op.kind() {
            ir::OpKind::Listener => {
                use crate::template::pipeline::ir::ops::create::ListenerOp;
                unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let listener_ptr = op_ptr as *mut ListenerOp;
                    let listener = &mut *listener_ptr;
                    
                    for handler_op in listener.handler_ops.iter_mut() {
                        propagate_slot_indexes_in_nested_ops(handler_op.as_mut(), slot_map);
                    }
                }
            }
            ir::OpKind::TwoWayListener => {
                use crate::template::pipeline::ir::ops::create::TwoWayListenerOp;
                unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let two_way_ptr = op_ptr as *mut TwoWayListenerOp;
                    let two_way = &mut *two_way_ptr;
                    
                    for handler_op in two_way.handler_ops.iter_mut() {
                        propagate_slot_indexes_in_nested_ops(handler_op.as_mut(), slot_map);
                    }
                }
            }
            ir::OpKind::Animation => {
                use crate::template::pipeline::ir::ops::create::AnimationOp;
                unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let anim_ptr = op_ptr as *mut AnimationOp;
                    let anim = &mut *anim_ptr;
                    
                    for handler_op in anim.handler_ops.iter_mut() {
                        propagate_slot_indexes_in_nested_ops(handler_op.as_mut(), slot_map);
                    }
                }
            }
            ir::OpKind::AnimationListener => {
                use crate::template::pipeline::ir::ops::create::AnimationListenerOp;
                unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let anim_listener_ptr = op_ptr as *mut AnimationListenerOp;
                    let anim_listener = &mut *anim_listener_ptr;
                    
                    for handler_op in anim_listener.handler_ops.iter_mut() {
                        propagate_slot_indexes_in_nested_ops(handler_op.as_mut(), slot_map);
                    }
                }
            }
            ir::OpKind::RepeaterCreate => {
                use crate::template::pipeline::ir::ops::create::RepeaterCreateOp;
                unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let repeater_ptr = op_ptr as *mut RepeaterCreateOp;
                    let repeater = &mut *repeater_ptr;
                    
                    if let Some(ref mut track_by_ops) = repeater.track_by_ops {
                        for track_by_op in track_by_ops.iter_mut() {
                            propagate_slot_indexes_in_nested_ops(track_by_op.as_mut(), slot_map);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Propagate slot indexes in nested ops (handler ops, trackByOps)
fn propagate_slot_indexes_in_nested_ops(
    op: &mut dyn ir::UpdateOp,
    slot_map: &std::collections::HashMap<ir::XrefId, usize>,
) {
    use crate::template::pipeline::ir::expression::transform_expressions_in_op;
    use crate::output::output_ast::Expression;
    
    transform_expressions_in_op(
        op,
        &mut |mut expr: Expression, _flags| {
            match expr {
                Expression::Reference(ref mut ref_expr) => {
                    if ref_expr.target_slot.slot.is_none() {
                        if let Some(&slot) = slot_map.get(&ref_expr.target) {
                            ref_expr.target_slot.slot = Some(slot);
                        }
                    }
                }
                Expression::ContextLetReference(ref mut ctx_let_ref) => {
                    if ctx_let_ref.target_slot.slot.is_none() {
                        if let Some(&slot) = slot_map.get(&ctx_let_ref.target) {
                            ctx_let_ref.target_slot.slot = Some(slot);
                        }
                    }
                }
                _ => {}
            }
            expr
        },
        ir::VisitorContextFlag::NONE,
    );
}

/// Get mutable reference to slot handle and num_slots_used for an op that implements ConsumesSlotOpTrait
fn get_slot_handle_and_num_slots_mut(op: &mut dyn ir::Op) -> Option<(&mut ir::SlotHandle, usize)> {
    unsafe {
        let op_ptr = op as *mut dyn ir::Op;
        let kind = op.kind();
        
        match kind {
            ir::OpKind::ElementStart => {
                use crate::template::pipeline::ir::ops::create::ElementStartOp;
                let elem_start_ptr = op_ptr as *mut ElementStartOp;
                let num_slots = (*elem_start_ptr).num_slots_used();
                Some((&mut (*elem_start_ptr).base.base.handle, num_slots))
            }
            ir::OpKind::Element => {
                use crate::template::pipeline::ir::ops::create::ElementOp;
                let elem_ptr = op_ptr as *mut ElementOp;
                let num_slots = (*elem_ptr).num_slots_used();
                Some((&mut (*elem_ptr).base.base.handle, num_slots))
            }
            ir::OpKind::ContainerStart => {
                use crate::template::pipeline::ir::ops::create::ContainerStartOp;
                let container_start_ptr = op_ptr as *mut ContainerStartOp;
                let num_slots = (*container_start_ptr).num_slots_used();
                Some((&mut (*container_start_ptr).base.handle, num_slots))
            }
            ir::OpKind::Container => {
                use crate::template::pipeline::ir::ops::create::ContainerOp;
                let container_ptr = op_ptr as *mut ContainerOp;
                let num_slots = (*container_ptr).num_slots_used();
                Some((&mut (*container_ptr).base.handle, num_slots))
            }
            ir::OpKind::Template => {
                use crate::template::pipeline::ir::ops::create::TemplateOp;
                let template_ptr = op_ptr as *mut TemplateOp;
                let num_slots = (*template_ptr).num_slots_used();
                Some((&mut (*template_ptr).base.base.handle, num_slots))
            }
            ir::OpKind::Text => {
                use crate::template::pipeline::ir::ops::create::TextOp;
                let text_ptr = op_ptr as *mut TextOp;
                let num_slots = (*text_ptr).num_slots_used();
                Some((&mut (*text_ptr).handle, num_slots))
            }
            ir::OpKind::RepeaterCreate => {
                use crate::template::pipeline::ir::ops::create::RepeaterCreateOp;
                let repeater_ptr = op_ptr as *mut RepeaterCreateOp;
                let num_slots = (*repeater_ptr).num_slots_used();
                Some((&mut (*repeater_ptr).base.base.handle, num_slots))
            }
            ir::OpKind::I18nStart => {
                use crate::template::pipeline::ir::ops::create::I18nStartOp;
                let i18n_ptr = op_ptr as *mut I18nStartOp;
                let num_slots = (*i18n_ptr).num_slots_used();
                Some((&mut (*i18n_ptr).base.handle, num_slots))
            }
            ir::OpKind::I18nAttributes => {
                use crate::template::pipeline::ir::ops::create::I18nAttributesOp;
                let i18n_attrs_ptr = op_ptr as *mut I18nAttributesOp;
                let num_slots = (*i18n_attrs_ptr).num_slots_used();
                Some((&mut (*i18n_attrs_ptr).handle, num_slots))
            }
            ir::OpKind::Defer => {
                use crate::template::pipeline::ir::ops::create::DeferOp;
                let defer_ptr = op_ptr as *mut DeferOp;
                let num_slots = (*defer_ptr).num_slots_used();
                Some((&mut (*defer_ptr).handle, num_slots))
            }
            ir::OpKind::Pipe => {
                use crate::template::pipeline::ir::ops::create::PipeOp;
                let pipe_ptr = op_ptr as *mut PipeOp;
                let num_slots = (*pipe_ptr).num_slots_used();
                Some((&mut (*pipe_ptr).handle, num_slots))
            }
            ir::OpKind::DeclareLet => {
                use crate::template::pipeline::ir::ops::create::DeclareLetOp;
                let declare_let_ptr = op_ptr as *mut DeclareLetOp;
                let num_slots = (*declare_let_ptr).num_slots_used();
                Some((&mut (*declare_let_ptr).handle, num_slots))
            }
            ir::OpKind::I18n => {
                use crate::template::pipeline::ir::ops::create::I18nOp;
                let i18n_ptr = op_ptr as *mut I18nOp;
                let num_slots = (*i18n_ptr).num_slots_used();
                Some((&mut (*i18n_ptr).base.handle, num_slots))
            }
            ir::OpKind::Projection => {
                use crate::template::pipeline::ir::ops::create::ProjectionOp;
                let proj_ptr = op_ptr as *mut ProjectionOp;
                let num_slots = (*proj_ptr).num_slots_used();
                Some((&mut (*proj_ptr).handle, num_slots))
            }
            ir::OpKind::ConditionalCreate => {
                use crate::template::pipeline::ir::ops::create::ConditionalCreateOp;
                let cond_ptr = op_ptr as *mut ConditionalCreateOp;
                let num_slots = (*cond_ptr).num_slots_used();
                Some((&mut (*cond_ptr).base.base.handle, num_slots))
            }
            ir::OpKind::ConditionalBranchCreate => {
                use crate::template::pipeline::ir::ops::create::ConditionalBranchCreateOp;
                let branch_ptr = op_ptr as *mut ConditionalBranchCreateOp;
                let num_slots = (*branch_ptr).num_slots_used();
                Some((&mut (*branch_ptr).base.base.handle, num_slots))
            }
            _ => None,
        }
    }
}

/// Set decls on an op that declares an embedded view
fn set_decls_on_op(op: &mut dyn ir::CreateOp, decls: usize) {
    unsafe {
        let op_ptr = op as *mut dyn ir::CreateOp;
        let op_ptr = op_ptr as *mut dyn ir::Op;
        
        match op.kind() {
            ir::OpKind::Template => {
                use crate::template::pipeline::ir::ops::create::TemplateOp;
                let template_ptr = op_ptr as *mut TemplateOp;
                (*template_ptr).decls = Some(decls);
            }
            ir::OpKind::ConditionalCreate => {
                use crate::template::pipeline::ir::ops::create::ConditionalCreateOp;
                let conditional_ptr = op_ptr as *mut ConditionalCreateOp;
                (*conditional_ptr).decls = Some(decls);
            }
            ir::OpKind::ConditionalBranchCreate => {
                use crate::template::pipeline::ir::ops::create::ConditionalBranchCreateOp;
                let branch_ptr = op_ptr as *mut ConditionalBranchCreateOp;
                (*branch_ptr).decls = Some(decls);
            }
            ir::OpKind::RepeaterCreate => {
                use crate::template::pipeline::ir::ops::create::RepeaterCreateOp;
                let repeater_ptr = op_ptr as *mut RepeaterCreateOp;
                (*repeater_ptr).decls = Some(decls);
            }
            _ => {}
        }
    }
}

