//! Defer Resolve Targets Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/defer_resolve_targets.ts
//! Some `defer` conditions can reference other elements in the template, using their local reference
//! names. However, the semantics are quite different from the normal local reference system: in
//! particular, we need to look at local reference names in enclosing views. This phase resolves
//! all such references to actual xrefs.

use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::handle::{SlotHandle, XrefId};
use crate::template::pipeline::ir::ops::create::{
    ContainerOp, ContainerStartOp, ElementOp, ElementStartOp,
};
use crate::template::pipeline::ir::ops::create::{
    DeferOnOp, DeferOp, DeferTrigger, ElementOrContainerOpBase,
};
use crate::template::pipeline::src::compilation::{CompilationUnit, ComponentCompilationJob};
use std::collections::HashMap;

/// Scope for tracking local references in a view
struct Scope {
    targets: HashMap<std::sync::Arc<str>, TargetInfo>,
}

struct TargetInfo {
    xref: XrefId,
    slot: SlotHandle,
}

impl Scope {
    fn new() -> Self {
        Scope {
            targets: HashMap::new(),
        }
    }
}

/// Helper function to check if an op is an element or container op
fn is_element_or_container_op(kind: OpKind) -> bool {
    matches!(
        kind,
        OpKind::Element
            | OpKind::ElementStart
            | OpKind::Container
            | OpKind::ContainerStart
            | OpKind::Template
            | OpKind::RepeaterCreate
            | OpKind::ConditionalCreate
            | OpKind::ConditionalBranchCreate
    )
}

/// Helper function to get ElementOrContainerOpBase from an op
unsafe fn get_element_or_container_base(
    op: &dyn ir::CreateOp,
) -> Option<&ElementOrContainerOpBase> {
    match op.kind() {
        OpKind::ElementStart => {
            let op_ptr = op as *const dyn ir::CreateOp;
            let elem_start_ptr = op_ptr as *const ElementStartOp;
            Some(&(*elem_start_ptr).base.base)
        }
        OpKind::Element => {
            let op_ptr = op as *const dyn ir::CreateOp;
            let elem_ptr = op_ptr as *const ElementOp;
            Some(&(*elem_ptr).base.base)
        }
        OpKind::ContainerStart => {
            let op_ptr = op as *const dyn ir::CreateOp;
            let container_ptr = op_ptr as *const ContainerStartOp;
            Some(&(*container_ptr).base)
        }
        OpKind::Container => {
            let op_ptr = op as *const dyn ir::CreateOp;
            let container_ptr = op_ptr as *const ContainerOp;
            Some(&(*container_ptr).base)
        }
        _ => None,
    }
}

/// Some `defer` conditions can reference other elements in the template, using their local reference
/// names. However, the semantics are quite different from the normal local reference system: in
/// particular, we need to look at local reference names in enclosing views. This phase resolves
/// all such references to actual xrefs.
/// Helper to get or create scope for a view
fn get_scope_for_view<'a>(
    view: &'a dyn CompilationUnit,
    scopes: &'a mut HashMap<XrefId, Scope>,
) -> &'a Scope {
    let view_xref = view.xref();

    if !scopes.contains_key(&view_xref) {
        let mut scope = Scope::new();

        for op in view.create().iter() {
            // Add everything that can be referenced
            if !is_element_or_container_op(op.kind()) {
                continue;
            }

            let base = unsafe { get_element_or_container_base(op.as_ref()) };
            if let Some(base) = base {
                // Check if local_refs is still an array (not yet processed)
                if base.local_refs_index.is_some() {
                    panic!("LocalRefs were already processed, but were needed to resolve defer targets.");
                }

                for ref_item in &base.local_refs {
                    if &*ref_item.target != "" {
                        continue;
                    }
                    scope.targets.insert(
                        ref_item.name.clone(),
                        TargetInfo {
                            xref: op.xref(),
                            slot: base.handle.clone(),
                        },
                    );
                }
            }
        }

        scopes.insert(view_xref, scope);
    }

    scopes.get(&view_xref).unwrap()
}

/// Helper to get slot handle from an op
unsafe fn get_slot_handle(op: &dyn ir::CreateOp) -> Option<SlotHandle> {
    match op.kind() {
        OpKind::ElementStart | OpKind::Element => {
            let op_ptr = op as *const dyn ir::CreateOp;
            let elem_ptr = op_ptr as *const ElementStartOp;
            Some((*elem_ptr).base.base.handle.clone())
        }
        OpKind::ContainerStart | OpKind::Container => {
            let op_ptr = op as *const dyn ir::CreateOp;
            let container_ptr = op_ptr as *const ContainerStartOp;
            Some((*container_ptr).base.handle.clone())
        }
        OpKind::Projection => {
            use crate::template::pipeline::ir::ops::create::ProjectionOp;
            let op_ptr = op as *const dyn ir::CreateOp;
            let projection_ptr = op_ptr as *const ProjectionOp;
            Some((*projection_ptr).handle.clone())
        }
        _ => None,
    }
}

pub fn resolve_defer_target_names(job: &mut ComponentCompilationJob) {
    let mut scopes: HashMap<XrefId, Scope> = HashMap::new();

    // First pass: collect all DeferOps and store their info
    let mut defer_ops_info: HashMap<XrefId, (Option<XrefId>, Option<XrefId>)> = HashMap::new(); // defer xref -> (main_view, placeholder_view)

    // Collect from root unit
    for op in job.root.create().iter() {
        if op.kind() == OpKind::Defer {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let defer_ptr = op_ptr as *const DeferOp;
                let defer = &*defer_ptr;
                defer_ops_info.insert(defer.xref, (Some(defer.main_view), defer.placeholder_view));
            }
        }
    }

    // Collect from all view units
    for (_, unit) in job.views.iter() {
        for op in unit.create().iter() {
            if op.kind() == OpKind::Defer {
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let defer_ptr = op_ptr as *const DeferOp;
                    let defer = &*defer_ptr;
                    defer_ops_info
                        .insert(defer.xref, (Some(defer.main_view), defer.placeholder_view));
                }
            }
        }
    }

    // Second pass: resolve triggers for DeferOnOps
    // Process root unit
    {
        let job_ptr = job as *mut ComponentCompilationJob;
        for op in job.root.create_mut().iter_mut() {
            if op.kind() == OpKind::DeferOn {
                unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let defer_on_ptr = op_ptr as *mut DeferOnOp;
                    let defer_on = &mut *defer_on_ptr;

                    // Get the associated DeferOp info
                    let (main_view, placeholder_view_from_defer) = defer_ops_info
                        .get(&defer_on.defer)
                        .expect("DeferOp not found for DeferOnOp");

                    let placeholder_view =
                        if defer_on.modifier == ir::enums::DeferOpModifierKind::Hydrate {
                            *main_view
                        } else {
                            *placeholder_view_from_defer
                        };

                    // Resolve trigger - use immutable reference via raw pointer
                    let job_ref: &ComponentCompilationJob = &*job_ptr;
                    let root_ptr = &(*job_ptr).root
                        as *const crate::template::pipeline::src::compilation::ViewCompilationUnit;
                    let root_ref: &dyn CompilationUnit = &*root_ptr as &dyn CompilationUnit;
                    resolve_trigger_inner(
                        defer_on,
                        placeholder_view,
                        job_ref,
                        &mut scopes,
                        root_ref,
                    );
                }
            }
        }
    }

    // Process all view units
    {
        let job_ptr = job as *mut ComponentCompilationJob;
        for (_, unit) in job.views.iter_mut() {
            // Save unit pointer before mutating ops
            let unit_ptr =
                unit as *const crate::template::pipeline::src::compilation::ViewCompilationUnit;
            for op in unit.create_mut().iter_mut() {
                if op.kind() == OpKind::DeferOn {
                    unsafe {
                        let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                        let defer_on_ptr = op_ptr as *mut DeferOnOp;
                        let defer_on = &mut *defer_on_ptr;

                        // Get the associated DeferOp info
                        let (main_view, placeholder_view_from_defer) = defer_ops_info
                            .get(&defer_on.defer)
                            .expect("DeferOp not found for DeferOnOp");

                        let placeholder_view =
                            if defer_on.modifier == ir::enums::DeferOpModifierKind::Hydrate {
                                *main_view
                            } else {
                                *placeholder_view_from_defer
                            };

                        // Resolve trigger - use immutable references via raw pointers
                        let job_ref: &ComponentCompilationJob = &*job_ptr;
                        let unit_ref: &dyn CompilationUnit = &*unit_ptr as &dyn CompilationUnit;
                        resolve_trigger_inner(
                            defer_on,
                            placeholder_view,
                            job_ref,
                            &mut scopes,
                            unit_ref,
                        );
                    }
                }
            }
        }
    }
}

/// Resolve trigger for a DeferOnOp
fn resolve_trigger_inner(
    defer_on_op: &mut DeferOnOp,
    placeholder_view: Option<XrefId>,
    job: &ComponentCompilationJob,
    scopes: &mut HashMap<XrefId, Scope>,
    defer_owner_view: &dyn CompilationUnit,
) {
    match &mut defer_on_op.trigger {
        DeferTrigger::Idle
        | DeferTrigger::Never
        | DeferTrigger::Immediate
        | DeferTrigger::Timer { .. } => {
            return;
        }
        DeferTrigger::Hover {
            target_name,
            target_xref,
            target_view,
            target_slot,
            target_slot_view_steps,
        }
        | DeferTrigger::Interaction {
            target_name,
            target_xref,
            target_view,
            target_slot,
            target_slot_view_steps,
        }
        | DeferTrigger::Viewport {
            target_name,
            target_xref,
            target_view,
            target_slot,
            target_slot_view_steps,
            ..
        } => {
            // Handle null target name (default to first element in placeholder block)
            if target_name.is_none() {
                let placeholder = placeholder_view
                    .expect("defer on trigger with no target name must have a placeholder block");

                let placeholder_unit = job
                    .views
                    .get(&placeholder)
                    .expect("AssertionError: could not find placeholder view for defer on trigger");

                // Find first element/container/projection op in placeholder view
                for op in placeholder_unit.create().iter() {
                    if op.kind() == OpKind::Projection
                        || (op_kind_has_consumes_slot_trait(op.kind())
                            && is_element_or_container_op(op.kind()))
                    {
                        // This op can be referenced
                        *target_xref = Some(op.xref());
                        *target_view = Some(placeholder);
                        *target_slot_view_steps = Some(0); // Use 0 as sentinel for placeholder (-1 in TS)

                        // Get slot handle
                        if let Some(slot) = unsafe { get_slot_handle(op.as_ref()) } {
                            *target_slot = Some(slot);
                        }
                        return;
                    }
                }
                return;
            }

            // Search for target name in views (starting from placeholder or defer owner view)
            let target_name_str = target_name.as_ref().unwrap();
            let mut current_view_xref: Option<XrefId> = placeholder_view;
            let mut step: isize = if placeholder_view.is_some() { -1 } else { 0 }; // -1 for placeholder

            loop {
                let view = if let Some(xref) = current_view_xref {
                    if xref == defer_owner_view.xref() {
                        Some(defer_owner_view)
                    } else {
                        job.views.get(&xref).map(|v| v as &dyn CompilationUnit)
                    }
                } else {
                    if step == 0 {
                        Some(defer_owner_view)
                    } else {
                        None
                    }
                };

                if let Some(view) = view {
                    let scope = get_scope_for_view(view, scopes);

                    if let Some(target_info) = scope.targets.get(target_name_str.as_str()) {
                        *target_xref = Some(target_info.xref);
                        *target_view = Some(view.xref());
                        *target_slot_view_steps = Some(if step == -1 { 0 } else { step as usize });
                        *target_slot = Some(target_info.slot.clone());
                        return;
                    }

                    // Move to parent view
                    if view.xref() == defer_owner_view.xref() {
                        // Access parent through ViewCompilationUnit's parent field
                        unsafe {
                            let view_ptr = view as *const dyn CompilationUnit;
                            let view_unit_ptr = view_ptr as *const crate::template::pipeline::src::compilation::ViewCompilationUnit;
                            current_view_xref = (*view_unit_ptr).parent;
                        }
                    } else {
                        // It's from job.views, so access parent directly
                        unsafe {
                            let view_ptr = view as *const dyn CompilationUnit;
                            let view_unit_ptr = view_ptr as *const crate::template::pipeline::src::compilation::ViewCompilationUnit;
                            current_view_xref = (*view_unit_ptr).parent;
                        }
                    }

                    if current_view_xref.is_none() {
                        break;
                    }
                    step += 1;
                } else {
                    break;
                }
            }
        }
    }
}

/// Helper function to check if an op kind has ConsumesSlotOpTrait
fn op_kind_has_consumes_slot_trait(kind: OpKind) -> bool {
    matches!(
        kind,
        OpKind::ElementStart
            | OpKind::Element
            | OpKind::ContainerStart
            | OpKind::Container
            | OpKind::Template
            | OpKind::Text
            | OpKind::I18nStart
            | OpKind::I18n
            | OpKind::I18nAttributes
            | OpKind::RepeaterCreate
            | OpKind::ConditionalCreate
            | OpKind::ConditionalBranchCreate
            | OpKind::Projection
            | OpKind::Defer
            | OpKind::DeclareLet
            | OpKind::Pipe
    )
}
