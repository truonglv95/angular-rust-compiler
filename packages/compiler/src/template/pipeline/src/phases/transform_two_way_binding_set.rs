//! Transform Two Way Binding Set Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/transform_two_way_binding_set.ts
//! Transforms a `TwoWayBindingSet` expression into an expression that either
//! sets a value through the `twoWayBindingSet` instruction or falls back to setting
//! the value directly.

use crate::output::output_ast::Expression;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::expression::transform_expressions_in_op;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob,
};
use crate::template::pipeline::src::instruction::two_way_binding_set;

/// Transforms a `TwoWayBindingSet` expression into an expression that either
/// sets a value through the `twoWayBindingSet` instruction or falls back to setting
/// the value directly.
pub fn transform_two_way_binding_set(job: &mut dyn CompilationJob) {
    for unit in job.units_mut() {
        process_unit(unit);
    }
}

fn process_unit(unit: &mut dyn crate::template::pipeline::src::compilation::CompilationUnit) {
    for op in unit.create_mut().iter_mut() {
        if op.kind() == OpKind::TwoWayListener {
            unsafe {
                use crate::template::pipeline::ir::ops::create::TwoWayListenerOp;
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let two_way_listener_ptr = op_ptr as *mut TwoWayListenerOp;
                let two_way_listener = &mut *two_way_listener_ptr;

                for handler_op in two_way_listener.handler_ops.iter_mut() {
                    transform_expressions_in_op(
                        handler_op.as_mut(),
                        &mut |expr: Expression, _flags| {
                            // Check if this is a TwoWayBindingSetExpr
                            if let Expression::TwoWayBindingSet(two_way_expr) = &expr {
                                eprintln!("DEBUG: transform_two_way_binding_set found TwoWayBindingSetExpr");
                                let target = two_way_expr.target.clone();
                                let value = two_way_expr.value.clone();
                                let source_span = two_way_expr.source_span.clone();

                                eprintln!(
                                    "DEBUG: target type = {:?}",
                                    std::mem::discriminant(&*target)
                                );

                                // Transform based on target type
                                match &*target {
                                    Expression::ReadProp(_) | Expression::ReadKey(_) => {
                                        eprintln!("DEBUG: matched ReadProp/ReadKey, creating twoWayBindingSet");
                                        // For ReadPropExpr or ReadKeyExpr, create:
                                        // twoWayBindingSet(target, value) || (target = value)
                                        let two_way_set =
                                            two_way_binding_set(target.clone(), value.clone());
                                        let assign = target.set(value, source_span.clone());
                                        // Wrap assignment in parens to produce valid JS: a || (b = c)
                                        let parens_assign = Box::new(Expression::Parens(
                                            crate::output::output_ast::ParenthesizedExpr {
                                                expr: assign,
                                                source_span: source_span.clone(),
                                                type_: None,
                                            },
                                        ));
                                        *two_way_set.or(parens_assign, source_span)
                                    }
                                    Expression::ReadVariable(_) => {
                                        eprintln!("DEBUG: matched ReadVariable");
                                        // For ReadVariableExpr (local template variable),
                                        // only emit the twoWayBindingSet since the fallback
                                        // would be attempting to write into a constant.
                                        *two_way_binding_set(target.clone(), value.clone())
                                    }
                                    _ => {
                                        eprintln!("DEBUG: unsupported target type: {:?}", target);
                                        panic!("Unsupported expression in two-way action binding.");
                                    }
                                }
                            } else {
                                expr
                            }
                        },
                        ir::VisitorContextFlag::IN_CHILD_OPERATION,
                    );
                }
            }
        }
    }
}
