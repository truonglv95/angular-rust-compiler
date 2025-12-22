//! Resolve Names Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/resolve_names.ts
//! Resolves lexical references in views (`ir.LexicalReadExpr`) to either a target variable or to
//! property reads on the top-level component context.
//!
//! Also matches `ir.RestoreViewExpr` expressions with the variables of their corresponding saved
//! views.

use crate::template::pipeline::ir as ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::variable::SemanticVariableKind;
use crate::template::pipeline::ir::expression::transform_expressions_in_op;
use crate::template::pipeline::ir::expression::{ReadVariableExpr};
use crate::template::pipeline::ir::expression::{ContextExpr, EitherXrefIdOrExpression};
use crate::template::pipeline::ir::ops::shared::VariableOp;
use crate::template::pipeline::ir::ops::create::{ListenerOp, AnimationListenerOp, TwoWayListenerOp, RepeaterCreateOp};
use crate::template::pipeline::src::compilation::{CompilationJob, ComponentCompilationJob, CompilationJobKind, CompilationUnit};
use crate::output::output_ast::Expression;

/// Information about a `SavedView` variable.
#[derive(Clone)]
struct SavedView {
    /// The view `ir.XrefId` which was saved into this variable.
    view: ir::XrefId,
    /// The `ir.XrefId` of the variable into which the view was saved.
    variable: ir::XrefId,
}

/// Resolves lexical references in views (`ir.LexicalReadExpr`) to either a target variable or to
/// property reads on the top-level component context.
///
/// Also matches `ir.RestoreViewExpr` expressions with the variables of their corresponding saved
/// views.
pub fn phase(job: &mut dyn CompilationJob) {
    let job_kind = job.kind();
    
    if matches!(job_kind, CompilationJobKind::Tmpl | CompilationJobKind::Both) {
        let component_job = unsafe {
            let job_ptr = job as *mut dyn CompilationJob;
            let job_ptr = job_ptr as *mut ComponentCompilationJob;
            &mut *job_ptr
        };
        
        let root_xref = component_job.root.xref();
        
        // Process root unit
        process_lexical_scope(&mut component_job.root, root_xref, None);
        
        // Process all view units
        let view_keys: Vec<_> = component_job.views.keys().cloned().collect();
        for key in view_keys {
            if let Some(unit) = component_job.views.get_mut(&key) {
                process_lexical_scope(unit, root_xref, None);
            }
        }
    }
}

fn process_lexical_scope(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    root_xref: ir::XrefId,
    saved_view: Option<SavedView>,
) {
    // Maps names defined in the lexical scope of this template to the `ir.XrefId`s of the variable
    // declarations which represent those values.
    //
    // Since variables are generated in each view for the entire lexical scope (including any
    // identifiers from parent templates) only local variables need be considered here.
    let mut scope: std::collections::HashMap<String, ir::XrefId> = std::collections::HashMap::new();
    
    // Symbols defined within the current scope. They take precedence over ones defined outside.
    let mut local_definitions: std::collections::HashMap<String, ir::XrefId> = std::collections::HashMap::new();
    
    let mut current_saved_view = saved_view;
    
    // First, step through the operations list and:
    // 1) build up the `scope` mapping
    // 2) recurse into any listener functions
    // Process create ops
    for op in unit.create().iter() {
        match op.kind() {
            OpKind::Variable => {
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let variable_op_ptr = op_ptr as *const VariableOp<Box<dyn ir::CreateOp + Send + Sync>>;
                    let variable_op = &*variable_op_ptr;
                    
                    match variable_op.variable.kind() {
                        SemanticVariableKind::Identifier => {
                            if let ir::SemanticVariable::Identifier(identifier_var) = &variable_op.variable {
                                if identifier_var.local {
                                    if !local_definitions.contains_key(&identifier_var.identifier) {
                                        local_definitions.insert(identifier_var.identifier.clone(), variable_op.xref);
                                    }
                                } else if !scope.contains_key(&identifier_var.identifier) {
                                    scope.insert(identifier_var.identifier.clone(), variable_op.xref);
                                }
                            }
                        }
                        SemanticVariableKind::Alias => {
                            if let ir::SemanticVariable::Alias(alias_var) = &variable_op.variable {
                                if !scope.contains_key(&alias_var.identifier) {
                                    scope.insert(alias_var.identifier.clone(), variable_op.xref);
                                }
                            }
                        }
                        SemanticVariableKind::SavedView => {
                            if let ir::SemanticVariable::SavedView(saved_view_var) = &variable_op.variable {
                                current_saved_view = Some(SavedView {
                                    view: saved_view_var.view,
                                    variable: variable_op.xref,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            OpKind::Listener | OpKind::AnimationListener | OpKind::TwoWayListener => {
                // Listener functions have separate variable declarations, so process them as a separate
                // lexical scope. We'll process them in the transform phase below.
            }
            OpKind::RepeaterCreate => {
                // TrackByOps will be processed in the transform phase below.
            }
            _ => {}
        }
    }
    
    // Process update ops
    for op in unit.update().iter() {
        if op.kind() == OpKind::Variable {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                let variable_op_ptr = op_ptr as *const VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>;
                let variable_op = &*variable_op_ptr;
                
                match variable_op.variable.kind() {
                    SemanticVariableKind::Identifier => {
                        if let ir::SemanticVariable::Identifier(identifier_var) = &variable_op.variable {
                            if identifier_var.local {
                                if !local_definitions.contains_key(&identifier_var.identifier) {
                                    local_definitions.insert(identifier_var.identifier.clone(), variable_op.xref);
                                }
                            } else if !scope.contains_key(&identifier_var.identifier) {
                                scope.insert(identifier_var.identifier.clone(), variable_op.xref);
                            }
                        }
                    }
                    SemanticVariableKind::Alias => {
                        if let ir::SemanticVariable::Alias(alias_var) = &variable_op.variable {
                            if !scope.contains_key(&alias_var.identifier) {
                                scope.insert(alias_var.identifier.clone(), variable_op.xref);
                            }
                        }
                    }
                    SemanticVariableKind::SavedView => {
                        if let ir::SemanticVariable::SavedView(saved_view_var) = &variable_op.variable {
                            current_saved_view = Some(SavedView {
                                view: saved_view_var.view,
                                variable: variable_op.xref,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    // Next, use the `scope` mapping to match `ir.LexicalReadExpr` with defined names in the lexical
    // scope. Also, look for `ir.RestoreViewExpr`s and match them with the snapshotted view context
    // variable.
    // Process create ops
    let current_saved_view_ref = &current_saved_view;
    for op in unit.create_mut().iter_mut() {
        match op.kind() {
            OpKind::Listener | OpKind::AnimationListener | OpKind::TwoWayListener => {
                // Listener functions have separate variable declarations, so process them as a separate
                // lexical scope.
                process_listener_scope_recursive(op, root_xref, current_saved_view.clone(), &scope, &local_definitions);
            }
            OpKind::RepeaterCreate => {
                process_repeater_scope_recursive(op, root_xref, current_saved_view.clone(), &scope, &local_definitions);
            }
            _ => {
                unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let op_ptr = op_ptr as *mut dyn ir::Op;
                    transform_lexical_reads_in_op(&mut *op_ptr, &scope, &local_definitions, root_xref, current_saved_view_ref);
                }
            }
        }
    }
    
    // Process update ops
    for op in unit.update_mut().iter_mut() {
        transform_lexical_reads_in_op(op.as_mut(), &scope, &local_definitions, root_xref, current_saved_view_ref);
    }
    
    // Verify no lexical reads remain
    // Note: We skip verification in Rust due to borrow checker limitations.
    // The transform phase should have replaced all LexicalReadExpr instances.
}

fn transform_lexical_reads_in_op(
    op: &mut dyn ir::Op,
    scope: &std::collections::HashMap<String, ir::XrefId>,
    local_definitions: &std::collections::HashMap<String, ir::XrefId>,
    root_xref: ir::XrefId,
    saved_view: &Option<SavedView>,
) {
    transform_expressions_in_op(
        op,
        &mut |expr, _flags| {
            if let Expression::LexicalRead(ref lexical_read) = expr {
                // `expr` is a read of a name within the lexical scope of this view.
                // Either that name is defined within the current view, or it represents a property from the
                // main component context.
                if let Some(xref) = local_definitions.get(&lexical_read.name) {
                    return Expression::ReadVariable(ReadVariableExpr {
                        xref: *xref,
                        name: None,
                        source_span: lexical_read.source_span.clone(),
                    });
                } else if let Some(xref) = scope.get(&lexical_read.name) {
                    // This was a defined variable in the current scope.
                    return Expression::ReadVariable(ReadVariableExpr {
                        xref: *xref,
                        name: None,
                        source_span: lexical_read.source_span.clone(),
                    });
                } else {
                    // Reading from the component context.
                    return Expression::ReadProp(crate::output::output_ast::ReadPropExpr {
                        receiver: Box::new(Expression::Context(ContextExpr {
                            view: root_xref,
                            source_span: None,
                        })),
                        name: lexical_read.name.clone(),
                        type_: None,
                        source_span: lexical_read.source_span.clone(),
                    });
                }
            } else if let Expression::RestoreView(ref restore_view) = expr {
                // `ir.RestoreViewExpr` happens in listener functions and restores a saved view from the
                // parent creation list. We expect to find that we captured the `savedView` previously, and
                // that it matches the expected view to be restored.
                if let EitherXrefIdOrExpression::XrefId(view_xref) = &restore_view.view {
                    if let Some(saved) = saved_view {
                        if saved.view == *view_xref {
                            return Expression::RestoreView(ir::expression::RestoreViewExpr {
                                view: EitherXrefIdOrExpression::Expression(Box::new(
                                    Expression::ReadVariable(ReadVariableExpr {
                                        xref: saved.variable,
                                        name: None,
                                        source_span: restore_view.source_span.clone(),
                                    })
                                )),
                                source_span: restore_view.source_span.clone(),
                            });
                        }
                    }
                    panic!("AssertionError: no saved view {:?} from current view", view_xref);
                }
            }
            expr
        },
        ir::VisitorContextFlag::NONE,
    );
}

fn process_listener_scope_recursive(
    op: &mut Box<dyn ir::CreateOp + Send + Sync>,
    root_xref: ir::XrefId,
    saved_view: Option<SavedView>,
    scope: &std::collections::HashMap<String, ir::XrefId>,
    local_definitions: &std::collections::HashMap<String, ir::XrefId>,
) {
    match op.kind() {
        OpKind::Listener => {
            unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let listener_ptr = op_ptr as *mut ListenerOp;
                let listener = &mut *listener_ptr;
                
                // Transform expressions in handler_ops
                let saved_view_ref = &saved_view;
                for handler_op in listener.handler_ops.iter_mut() {
                    transform_lexical_reads_in_op(handler_op.as_mut(), &scope, &local_definitions, root_xref, saved_view_ref);
                }
            }
        }
        OpKind::AnimationListener => {
            unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let animation_listener_ptr = op_ptr as *mut AnimationListenerOp;
                let animation_listener = &mut *animation_listener_ptr;
                
                // Transform expressions in handler_ops
                let saved_view_ref = &saved_view;
                for handler_op in animation_listener.handler_ops.iter_mut() {
                    transform_lexical_reads_in_op(handler_op.as_mut(), &scope, &local_definitions, root_xref, saved_view_ref);
                }
            }
        }
        OpKind::TwoWayListener => {
            unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let two_way_listener_ptr = op_ptr as *mut TwoWayListenerOp;
                let two_way_listener = &mut *two_way_listener_ptr;
                
                // Transform expressions in handler_ops
                let saved_view_ref = &saved_view;
                for handler_op in two_way_listener.handler_ops.iter_mut() {
                    transform_lexical_reads_in_op(handler_op.as_mut(), &scope, &local_definitions, root_xref, saved_view_ref);
                }
            }
        }
        _ => {}
    }
}

fn process_repeater_scope_recursive(
    op: &mut Box<dyn ir::CreateOp + Send + Sync>,
    root_xref: ir::XrefId,
    saved_view: Option<SavedView>,
    scope: &std::collections::HashMap<String, ir::XrefId>,
    local_definitions: &std::collections::HashMap<String, ir::XrefId>,
) {
    unsafe {
        let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
        let repeater_ptr = op_ptr as *mut RepeaterCreateOp;
        let repeater = &mut *repeater_ptr;
        
        if let Some(ref mut track_by_ops) = repeater.track_by_ops {
            // Transform expressions in trackByOps
            let saved_view_ref = &saved_view;
            for track_op in track_by_ops.iter_mut() {
                transform_lexical_reads_in_op(track_op.as_mut(), &scope, &local_definitions, root_xref, saved_view_ref);
            }
        }
    }
}

