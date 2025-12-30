//! Resolve Dollar Event Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/resolve_dollar_event.ts
//! Any variable inside a listener with the name `$event` will be transformed into a output lexical
//! read immediately, and does not participate in any of the normal logic for handling variables.

use crate::output::output_ast::Expression;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::expression::transform_expressions_in_op;
use crate::template::pipeline::ir::ops::create::{
    AnimationListenerOp, ListenerOp, TwoWayListenerOp,
};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, CompilationUnit, ComponentCompilationJob,
};

/// Any variable inside a listener with the name `$event` will be transformed into a output lexical
/// read immediately, and does not participate in any of the normal logic for handling variables.
pub fn resolve_dollar_event(job: &mut dyn CompilationJob) {
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
        transform_dollar_event_in_unit(&mut component_job.root);

        // Process all view units
        for (_, unit) in component_job.views.iter_mut() {
            transform_dollar_event_in_unit(unit);
        }
    }
}

fn transform_dollar_event_in_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
) {
    // Transform in create ops (listeners)
    for op in unit.create_mut().iter_mut() {
        match op.kind() {
            OpKind::Listener | OpKind::AnimationListener | OpKind::TwoWayListener => {
                unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;

                    match op.kind() {
                        OpKind::Listener => {
                            let listener_ptr = op_ptr as *mut ListenerOp;
                            let listener = &mut *listener_ptr;

                            // Transform expressions in handler_ops
                            // The transformExpressionsInOp with InChildOperation flag in TypeScript
                            // means we only transform expressions in child operations (handler_ops)
                            for handler_op in listener.handler_ops.iter_mut() {
                                transform_expressions_in_op(
                                    handler_op.as_mut(),
                                    &mut |expr, _flags| {
                                        if let Expression::LexicalRead(ref lexical_read) = expr {
                                            if &*lexical_read.name == "$event" {
                                                listener.consumes_dollar_event = true;
                                                return Expression::ReadVar(
                                                    crate::output::output_ast::ReadVarExpr {
                                                        name: "$event".to_string(),
                                                        type_: None,
                                                        source_span: lexical_read
                                                            .source_span
                                                            .clone(),
                                                    },
                                                );
                                            }
                                        }
                                        expr
                                    },
                                    ir::VisitorContextFlag::NONE,
                                );
                            }
                        }
                        OpKind::AnimationListener => {
                            let animation_listener_ptr = op_ptr as *mut AnimationListenerOp;
                            let animation_listener = &mut *animation_listener_ptr;

                            // Transform expressions in handler_ops
                            for handler_op in animation_listener.handler_ops.iter_mut() {
                                transform_expressions_in_op(
                                    handler_op.as_mut(),
                                    &mut |expr, _flags| {
                                        if let Expression::LexicalRead(ref lexical_read) = expr {
                                            if &*lexical_read.name == "$event" {
                                                animation_listener.consumes_dollar_event = true;
                                                return Expression::ReadVar(
                                                    crate::output::output_ast::ReadVarExpr {
                                                        name: "$event".to_string(),
                                                        type_: None,
                                                        source_span: lexical_read
                                                            .source_span
                                                            .clone(),
                                                    },
                                                );
                                            }
                                        }
                                        expr
                                    },
                                    ir::VisitorContextFlag::NONE,
                                );
                            }
                        }
                        OpKind::TwoWayListener => {
                            let two_way_listener_ptr = op_ptr as *mut TwoWayListenerOp;
                            let two_way_listener = &mut *two_way_listener_ptr;

                            // Transform expressions in handler_ops
                            // Note: Two-way listeners always consume `$event` so they omit this field
                            for handler_op in two_way_listener.handler_ops.iter_mut() {
                                transform_expressions_in_op(
                                    handler_op.as_mut(),
                                    &mut |expr, _flags| {
                                        if let Expression::LexicalRead(ref lexical_read) = expr {
                                            if &*lexical_read.name == "$event" {
                                                return Expression::ReadVar(
                                                    crate::output::output_ast::ReadVarExpr {
                                                        name: "$event".to_string(),
                                                        type_: None,
                                                        source_span: lexical_read
                                                            .source_span
                                                            .clone(),
                                                    },
                                                );
                                            }
                                        }
                                        expr
                                    },
                                    ir::VisitorContextFlag::NONE,
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
