//! Generate Variables Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/generate_variables.ts
//! Generate a preamble sequence for each view creation block and listener function which declares
//! any variables that be referenced in other operations in the block.

use crate::output::output_ast::{Expression, ReadVarExpr};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::enums::VariableFlags;
use crate::template::pipeline::ir::expression::{
    ContextExpr, ContextLetReferenceExpr, NextContextExpr, ReferenceExpr,
};
use crate::template::pipeline::ir::handle::SlotHandle;
use crate::template::pipeline::ir::ops::shared::create_variable_op;
use crate::template::pipeline::ir::variable::{
    ContextVariable, IdentifierVariable, SemanticVariable, CTX_REF,
};
use crate::template::pipeline::src::compilation::CompilationJob;
use crate::template::pipeline::src::compilation::{
    CompilationJobKind, CompilationUnit, ComponentCompilationJob,
};
use indexmap::IndexMap;

/// Generate a preamble sequence for each view creation block and listener function which declares
/// any variables that be referenced in other operations in the block.
pub fn phase(job: &mut dyn CompilationJob) {
    let job_kind = job.kind();

    if matches!(
        job_kind,
        CompilationJobKind::Tmpl | CompilationJobKind::Both
    ) {
        let component_job_ptr = {
            let job_ptr = job as *mut dyn CompilationJob;
            job_ptr as *mut ComponentCompilationJob
        };

        unsafe {
            recursively_process_view(
                &mut (*component_job_ptr).root,
                None,
                &mut *component_job_ptr,
            );
        }
    }
}

/// Process the given `ViewCompilationUnit` and generate preambles for it and any listeners that it declares.
///
/// `parent_scope`: a scope extracted from the parent view which captures any variables which
/// should be inherited by this view. `None` if the current view is the root view.
fn recursively_process_view(
    view: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    parent_scope: Option<Scope>,
    component_job: &mut ComponentCompilationJob,
) {
    // Extract a `Scope` from this view.
    let scope = get_scope_for_view(view, parent_scope);

    // Process create ops and recursively process child views
    // Collect ops first, then process them
    let mut ops_to_process: Vec<(OpKind, ir::XrefId)> = Vec::new();
    for op in view.create() {
        match op.kind() {
            OpKind::ConditionalCreate | OpKind::ConditionalBranchCreate | OpKind::Template => {
                ops_to_process.push((op.kind(), op.xref()));
            }
            OpKind::Projection => {
                ops_to_process.push((OpKind::Projection, op.xref()));
            }
            OpKind::RepeaterCreate => {
                ops_to_process.push((OpKind::RepeaterCreate, op.xref()));
            }
            OpKind::Animation
            | OpKind::AnimationListener
            | OpKind::Listener
            | OpKind::TwoWayListener => {
                ops_to_process.push((op.kind(), op.xref()));
            }
            _ => {}
        }
    }

    // Process each op
    for (kind, xref) in ops_to_process {
        match kind {
            OpKind::ConditionalCreate | OpKind::ConditionalBranchCreate | OpKind::Template => {
                // Descend into child embedded views
                // Use raw pointer to avoid multiple borrows
                let component_job_ptr = component_job as *mut ComponentCompilationJob;
                if let Some(child_view) = unsafe { &mut *component_job_ptr }.views.get_mut(&xref) {
                    recursively_process_view(child_view, Some(scope.clone()), unsafe {
                        &mut *component_job_ptr
                    });
                }
            }
            OpKind::Projection => {
                // Check if there's a fallback view
                let component_job_ptr = component_job as *mut ComponentCompilationJob;
                unsafe {
                    if let Some(projection_op) = find_op_by_xref(view, xref) {
                        let op_ptr = projection_op.as_ref() as *const dyn ir::CreateOp;
                        use crate::template::pipeline::ir::ops::create::ProjectionOp;
                        let proj_ptr = op_ptr as *const ProjectionOp;
                        let proj = &*proj_ptr;
                        if let Some(fallback_view) = proj.fallback_view {
                            if let Some(fallback) =
                                (&mut *component_job_ptr).views.get_mut(&fallback_view)
                            {
                                recursively_process_view(
                                    fallback,
                                    Some(scope.clone()),
                                    &mut *component_job_ptr,
                                );
                            }
                        }
                    }
                }
            }
            OpKind::RepeaterCreate => {
                // Descend into child embedded views
                let component_job_ptr = component_job as *mut ComponentCompilationJob;
                unsafe {
                    if let Some(child_view) = (&mut *component_job_ptr).views.get_mut(&xref) {
                        recursively_process_view(
                            child_view,
                            Some(scope.clone()),
                            &mut *component_job_ptr,
                        );
                    }
                    // Check for empty view and trackByOps
                    if let Some(repeater_op) = find_op_by_xref(view, xref) {
                        let op_ptr = repeater_op.as_ref() as *const dyn ir::CreateOp;
                        use crate::template::pipeline::ir::ops::create::RepeaterCreateOp;
                        let rep_ptr = op_ptr as *const RepeaterCreateOp;
                        let rep = &*rep_ptr;
                        if let Some(empty_view) = rep.empty_view {
                            if let Some(empty) =
                                (&mut *component_job_ptr).views.get_mut(&empty_view)
                            {
                                recursively_process_view(
                                    empty,
                                    Some(scope.clone()),
                                    &mut *component_job_ptr,
                                );
                            }
                        }
                        // Generate variables for trackByOps
                        if rep.track_by_ops.is_some() {
                            let var_ops = generate_variables_in_scope_for_view(
                                view.xref(),
                                &scope,
                                false,
                                &mut *component_job_ptr,
                            );
                            // Prepend to track_by_ops - need mutable access
                            // Find the op again in the mutable list
                            for op_mut in view.create_mut().iter_mut() {
                                if op_mut.xref() == xref {
                                    let op_mut_ptr = op_mut.as_mut() as *mut dyn ir::CreateOp;
                                    let rep_mut_ptr = op_mut_ptr as *mut RepeaterCreateOp;
                                    let rep_mut = &mut *rep_mut_ptr;
                                    if let Some(ref mut track_by_ops) = rep_mut.track_by_ops {
                                        track_by_ops.prepend(var_ops);
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            OpKind::Animation
            | OpKind::AnimationListener
            | OpKind::Listener
            | OpKind::TwoWayListener => {
                // Prepend variables to listener handler functions
                let var_ops =
                    generate_variables_in_scope_for_view(view.xref(), &scope, true, component_job);
                for op in &var_ops {}
                prepend_variables_to_listener(view, xref, kind, var_ops);
            }
            _ => {}
        }
    }

    // Generate variables for this view
    // Generate variables for this view
    let vars = generate_variables_in_scope_for_view(view.xref(), &scope, false, component_job);
    for op in &vars {
        if let Some(var_op) = op
            .as_any()
            .downcast_ref::<ir::ops::shared::VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>>()
        {
        }
    }
    view.update_mut().prepend(vars);

    // Generate variables for listeners in this view
    // Redundant loop removed.
}

/// Helper to find an op by xref
fn find_op_by_xref(
    view: &crate::template::pipeline::src::compilation::ViewCompilationUnit,
    xref: ir::XrefId,
) -> Option<&Box<dyn ir::CreateOp + Send + Sync>> {
    view.create().iter().find(|op| op.xref() == xref)
}

/// Lexical scope of a view, including a reference to its parent view's scope, if any.
#[derive(Clone)]
struct Scope {
    /// `XrefId` of the view to which this scope corresponds.
    view: ir::XrefId,

    view_context_variable: SemanticVariable,

    context_variables: IndexMap<String, SemanticVariable>,

    /// Aliases from the view (cloned for recursive calls, but accessed via scope_view in generate_variables_in_scope_for_view)
    #[allow(dead_code)]
    aliases: Vec<ir::AliasVariable>,

    /// Local references collected from elements within the view.
    references: Vec<Reference>,

    /// `@let` declarations collected from the view.
    let_declarations: Vec<LetDeclaration>,

    /// `Scope` of the parent view, if any.
    parent: Option<Box<Scope>>,
}

/// Information needed about a local reference collected from an element within a view.
#[derive(Clone)]
struct Reference {
    /// Name given to the local reference variable within the template.
    /// (Stored but accessed via variable.identifier in practice)
    #[allow(dead_code)]
    name: String,

    /// `XrefId` of the element-like node which this reference targets.
    target_id: ir::XrefId,

    target_slot: SlotHandle,

    /// A generated offset of this reference among all the references on a specific element.
    offset: usize,

    variable: SemanticVariable,
}

/// Information about `@let` declaration collected from a view.
#[derive(Clone)]
struct LetDeclaration {
    /// `XrefId` of the `@let` declaration that the reference is pointing to.
    target_id: ir::XrefId,

    /// Slot in which the declaration is stored.
    target_slot: SlotHandle,

    /// Variable referring to the declaration.
    variable: IdentifierVariable,
}

/// Process a view and generate a `Scope` representing the variables available for reference within that view.
fn get_scope_for_view(
    view: &crate::template::pipeline::src::compilation::ViewCompilationUnit,
    parent: Option<Scope>,
) -> Scope {
    let mut scope = Scope {
        view: view.xref(),
        view_context_variable: SemanticVariable::Context(ContextVariable::new(view.xref())),
        context_variables: IndexMap::new(),
        aliases: view.aliases.clone(),
        references: Vec::new(),
        let_declarations: Vec::new(),
        parent: parent.map(Box::new),
    };

    // Add context variables
    for (identifier, _value) in &view.context_variables {
        scope.context_variables.insert(
            identifier.clone(),
            SemanticVariable::Identifier(IdentifierVariable::new(identifier.clone(), false)),
        );
    }

    // Collect local references and let declarations from create ops
    for op in view.create() {
        match op.kind() {
            OpKind::ElementStart
            | OpKind::ConditionalCreate
            | OpKind::ConditionalBranchCreate
            | OpKind::Template => {
                // Record available local references from this element
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let local_refs = get_local_refs_from_op(op, op_ptr);
                    for (offset, local_ref) in local_refs.iter().enumerate() {
                        scope.references.push(Reference {
                            name: local_ref.name.clone(),
                            target_id: op.xref(),
                            target_slot: get_slot_from_op(op, op_ptr),
                            offset,
                            variable: SemanticVariable::Identifier(IdentifierVariable::new(
                                local_ref.name.clone(),
                                false,
                            )),
                        });
                    }
                }
            }
            OpKind::DeclareLet => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                use crate::template::pipeline::ir::ops::create::DeclareLetOp;
                let let_ptr = op_ptr as *const DeclareLetOp;
                let let_op = &*let_ptr;
                scope.let_declarations.push(LetDeclaration {
                    target_id: op.xref(),
                    target_slot: let_op.handle.clone(),
                    variable: IdentifierVariable::new(let_op.declared_name.clone(), false),
                });
            },
            _ => {}
        }
    }

    scope
}

/// Get local refs from an op
unsafe fn get_local_refs_from_op(
    op: &Box<dyn ir::CreateOp + Send + Sync>,
    op_ptr: *const dyn ir::CreateOp,
) -> Vec<crate::template::pipeline::ir::ops::create::LocalRef> {
    match op.kind() {
        OpKind::ElementStart => {
            use crate::template::pipeline::ir::ops::create::ElementStartOp;
            let elem_ptr = op_ptr as *const ElementStartOp;
            let elem = &*elem_ptr;
            elem.base.base.local_refs.clone()
        }
        OpKind::ConditionalCreate => {
            use crate::template::pipeline::ir::ops::create::ConditionalCreateOp;
            let cond_ptr = op_ptr as *const ConditionalCreateOp;
            let cond = &*cond_ptr;
            cond.base.base.local_refs.clone()
        }
        OpKind::ConditionalBranchCreate => {
            use crate::template::pipeline::ir::ops::create::ConditionalBranchCreateOp;
            let branch_ptr = op_ptr as *const ConditionalBranchCreateOp;
            let branch = &*branch_ptr;
            branch.base.base.local_refs.clone()
        }
        OpKind::Template => {
            use crate::template::pipeline::ir::ops::create::TemplateOp;
            let template_ptr = op_ptr as *const TemplateOp;
            let template = &*template_ptr;
            template.base.base.local_refs.clone()
        }
        _ => Vec::new(),
    }
}

/// Get slot from an op
unsafe fn get_slot_from_op(
    op: &Box<dyn ir::CreateOp + Send + Sync>,
    op_ptr: *const dyn ir::CreateOp,
) -> SlotHandle {
    match op.kind() {
        OpKind::ElementStart => {
            use crate::template::pipeline::ir::ops::create::ElementStartOp;
            let elem_ptr = op_ptr as *const ElementStartOp;
            let elem = &*elem_ptr;
            elem.base.base.handle.clone()
        }
        OpKind::ConditionalCreate => {
            use crate::template::pipeline::ir::ops::create::ConditionalCreateOp;
            let cond_ptr = op_ptr as *const ConditionalCreateOp;
            let cond = &*cond_ptr;
            cond.base.base.handle.clone()
        }
        OpKind::ConditionalBranchCreate => {
            use crate::template::pipeline::ir::ops::create::ConditionalBranchCreateOp;
            let branch_ptr = op_ptr as *const ConditionalBranchCreateOp;
            let branch = &*branch_ptr;
            branch.base.base.handle.clone()
        }
        OpKind::Template => {
            use crate::template::pipeline::ir::ops::create::TemplateOp;
            let template_ptr = op_ptr as *const TemplateOp;
            let template = &*template_ptr;
            template.base.base.handle.clone()
        }
        _ => SlotHandle::new(),
    }
}

/// Generate declarations for all variables that are in scope for a given view.
///
/// This is a recursive process, as views inherit variables available from their parent view, which
/// itself may have inherited variables, etc.
fn generate_variables_in_scope_for_view(
    view_xref: ir::XrefId,
    scope: &Scope,
    is_callback: bool,
    component_job: &mut ComponentCompilationJob,
) -> Vec<Box<dyn ir::UpdateOp + Send + Sync>> {
    let mut new_ops: Vec<Box<dyn ir::UpdateOp + Send + Sync>> = Vec::new();

    // For callbacks in the SAME view (scope.view == view_xref && is_callback),
    // do NOT create a Context variable here - save_restore_view already prepended
    // a restoreView-based Context variable that should be used instead.
    // Only create NextContext for parent scopes (scope.view != view_xref).
    if scope.view != view_xref {
        // Before generating variables for a parent view, we need to switch to the context of the parent
        // view with a `nextContext` expression. This context switching operation itself declares a
        // variable, because the context of the view may be referenced directly.
        let next_context_xref = component_job.allocate_xref_id();

        let variable_op = create_variable_op::<Box<dyn ir::UpdateOp + Send + Sync>>(
            next_context_xref,
            scope.view_context_variable.clone(),
            Box::new(Expression::NextContext(NextContextExpr::new())),
            VariableFlags::NONE,
        );
        new_ops.push(Box::new(variable_op));
    }

    // Add variables for all context variables available in this scope's view.
    // Get scope_view values first to avoid borrowing issues
    // Sort to ensure consistent order: $implicit first, then $index, then $count, then alphabetical
    let context_var_data: Vec<(String, String)> = {
        if scope.view == component_job.root.xref {
            component_job
                .root
                .context_variables
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            let scope_view = component_job
                .views
                .get(&scope.view)
                .expect("Scope view should exist");
            scope_view
                .context_variables
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        }
    };
    // No manual sorting needed - IndexMap preserves insertion order from ingestion
    // This ensures variables are generated in the exact order they appear in the source template

    for (name, value) in context_var_data {
        let context = Expression::Context(ContextExpr::new(scope.view));
        // We either read the context, or, if the variable is CTX_REF, use the context directly.
        let variable_expr = if value == CTX_REF {
            context.clone()
        } else {
            Expression::ReadProp(crate::output::output_ast::ReadPropExpr {
                receiver: Box::new(context),
                name: value.clone(),
                type_: None,
                source_span: None,
            })
        };

        // Add the variable declaration.
        if let Some(context_var) = scope.context_variables.get(&name) {
            let var_xref = component_job.allocate_xref_id();
            let variable_op = create_variable_op::<Box<dyn ir::UpdateOp + Send + Sync>>(
                var_xref,
                context_var.clone(),
                Box::new(variable_expr),
                VariableFlags::NONE,
            );

            new_ops.push(Box::new(variable_op));
        } else {
        }
    }

    // Add variables for aliases
    // Get aliases first to avoid borrowing issues
    let aliases_data: Vec<ir::AliasVariable> = {
        if scope.view == component_job.root.xref {
            component_job.root.aliases.clone()
        } else {
            let scope_view = component_job
                .views
                .get(&scope.view)
                .expect("Scope view should exist");
            scope_view.aliases.clone()
        }
    };

    for alias in aliases_data {
        let alias_xref = component_job.allocate_xref_id();
        let variable_op = create_variable_op::<Box<dyn ir::UpdateOp + Send + Sync>>(
            alias_xref,
            SemanticVariable::Alias(alias.clone()),
            Box::new(alias.expression.clone()),
            VariableFlags::ALWAYS_INLINE,
        );
        new_ops.push(Box::new(variable_op));
    }

    // Add variables for all local references declared for elements in this scope.
    for ref_item in &scope.references {
        let ref_xref = component_job.allocate_xref_id();
        let ref_expr = Expression::Reference(ReferenceExpr::new(
            ref_item.target_id,
            ref_item.target_slot.clone(),
            ref_item.offset,
        ));
        let variable_op = create_variable_op::<Box<dyn ir::UpdateOp + Send + Sync>>(
            ref_xref,
            ref_item.variable.clone(),
            Box::new(ref_expr),
            VariableFlags::NONE,
        );
        new_ops.push(Box::new(variable_op));
    }

    if scope.view != view_xref || is_callback {
        // Add variables for @let declarations
        for decl in &scope.let_declarations {
            let let_xref = component_job.allocate_xref_id();
            let let_expr = Expression::ContextLetReference(ContextLetReferenceExpr::new(
                decl.target_id,
                decl.target_slot.clone(),
            ));
            let variable_op = create_variable_op::<Box<dyn ir::UpdateOp + Send + Sync>>(
                let_xref,
                SemanticVariable::Identifier(decl.variable.clone()),
                Box::new(let_expr),
                VariableFlags::NONE,
            );
            new_ops.push(Box::new(variable_op));
        }
    }

    // Recursively add variables from parent scope
    if let Some(parent_scope) = &scope.parent {
        let parent_ops =
            generate_variables_in_scope_for_view(view_xref, parent_scope, false, component_job);
        new_ops.extend(parent_ops);
    }

    new_ops
}

/// Prepend variables to listener handler ops
fn prepend_variables_to_listener(
    view: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    xref: ir::XrefId,
    kind: OpKind,
    var_ops: Vec<Box<dyn ir::UpdateOp + Send + Sync>>,
) {
    // Skip if no variables to prepend
    if var_ops.is_empty() {
        return;
    }

    // Find the listener op and prepend variables to its handler_ops
    for op in view.create_mut().iter_mut() {
        if op.xref() == xref {
            match kind {
                OpKind::Listener => {
                    use crate::template::pipeline::ir::ops::create::ListenerOp;
                    if let Some(listener) = op.as_any_mut().downcast_mut::<ListenerOp>() {
                        listener.handler_ops.prepend(var_ops);
                        return;
                    }
                }
                OpKind::TwoWayListener => {
                    use crate::template::pipeline::ir::ops::create::TwoWayListenerOp;
                    if let Some(two_way) = op.as_any_mut().downcast_mut::<TwoWayListenerOp>() {
                        two_way.handler_ops.prepend(var_ops);
                        return;
                    }
                }
                OpKind::Animation => {
                    use crate::template::pipeline::ir::ops::create::AnimationOp;
                    if let Some(anim) = op.as_any_mut().downcast_mut::<AnimationOp>() {
                        anim.handler_ops.prepend(var_ops);
                        return;
                    }
                }
                OpKind::AnimationListener => {
                    use crate::template::pipeline::ir::ops::create::AnimationListenerOp;
                    if let Some(anim_listener) =
                        op.as_any_mut().downcast_mut::<AnimationListenerOp>()
                    {
                        anim_listener.handler_ops.prepend(var_ops);
                        return;
                    }
                }
                _ => {}
            }
        }
    }
}
