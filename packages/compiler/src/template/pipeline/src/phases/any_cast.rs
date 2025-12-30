//! Any Cast Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/any_cast.ts
//! Removes $any function calls since they have no runtime effects

use crate::output::output_ast::Expression;
use crate::template::pipeline::ir::expression::VisitorContextFlag;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob,
};

/// Find any function calls to `$any`, excluding `this.$any`, and delete them, since they have no
/// runtime effects.
pub fn delete_any_casts(job: &mut dyn CompilationJob) {
    match job.kind() {
        crate::template::pipeline::src::compilation::CompilationJobKind::Tmpl => {
            // Process ComponentCompilationJob
            let component_job = unsafe {
                let job_ptr = job as *mut dyn CompilationJob;
                let job_ptr = job_ptr as *mut ComponentCompilationJob;
                &mut *job_ptr
            };

            // Process root unit
            process_unit(&mut component_job.root);

            // Process all view units
            for (_, unit) in component_job.views.iter_mut() {
                process_unit(unit);
            }
        }
        crate::template::pipeline::src::compilation::CompilationJobKind::Host => {
            // Process HostBindingCompilationJob
            use crate::template::pipeline::src::compilation::HostBindingCompilationJob;
            let host_job = unsafe {
                let job_ptr = job as *mut dyn CompilationJob;
                let job_ptr = job_ptr as *mut HostBindingCompilationJob;
                &mut *job_ptr
            };

            // Process root unit (HostBindingCompilationUnit)
            process_host_unit(&mut host_job.root);
        }
        crate::template::pipeline::src::compilation::CompilationJobKind::Both => {
            // This should not happen in practice
            panic!("CompilationJobKind::Both is not used for individual jobs");
        }
    }
}

fn process_unit(unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit) {
    // Process update ops
    for op in unit.update_mut().iter_mut() {
        transform_expressions_in_op(op, &mut remove_anys, VisitorContextFlag::NONE);
    }

    // Process create ops (some create ops may have expressions too)
    for op in unit.create_mut().iter_mut() {
        transform_expressions_in_create_op(op, &mut remove_anys, VisitorContextFlag::NONE);
    }
}

fn process_host_unit(
    unit: &mut crate::template::pipeline::src::compilation::HostBindingCompilationUnit,
) {
    // Process update ops
    for op in unit.update_mut().iter_mut() {
        transform_expressions_in_op(op, &mut remove_anys, VisitorContextFlag::NONE);
    }

    // Process create ops
    for op in unit.create_mut().iter_mut() {
        transform_expressions_in_create_op(op, &mut remove_anys, VisitorContextFlag::NONE);
    }
}

/// Transform expressions in an UpdateOp
fn transform_expressions_in_op(
    op: &mut Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
    transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
    flags: VisitorContextFlag,
) {
    use crate::template::pipeline::ir::enums::OpKind;
    use crate::template::pipeline::ir::expression::transform_expressions_in_expression;
    use crate::template::pipeline::ir::expression::transform_expressions_in_statement;
    use crate::template::pipeline::ir::ops::shared::StatementOp;
    use crate::template::pipeline::ir::ops::shared::VariableOp;
    use crate::template::pipeline::ir::ops::update::{
        AttributeOp, BindingOp, ClassMapOp, ClassPropOp, ControlOp, I18nExpressionOp,
        InterpolateTextOp, PropertyOp, StyleMapOp, StylePropOp, TwoWayPropertyOp,
    };

    unsafe {
        let op_ptr = op.as_mut() as *mut dyn crate::template::pipeline::ir::operations::UpdateOp;

        match op.kind() {
            OpKind::Binding
            | OpKind::StyleProp
            | OpKind::StyleMap
            | OpKind::ClassProp
            | OpKind::ClassMap => {
                // These ops have BindingExpression (which can be Expression or Interpolation)
                match op.kind() {
                    OpKind::Binding => {
                        let binding_op_ptr = op_ptr as *mut BindingOp;
                        let binding_op = &mut *binding_op_ptr;
                        transform_binding_expression(&mut binding_op.expression, transform, flags);
                    }
                    OpKind::StyleProp => {
                        let style_prop_op_ptr = op_ptr as *mut StylePropOp;
                        let style_prop_op = &mut *style_prop_op_ptr;
                        transform_binding_expression(
                            &mut style_prop_op.expression,
                            transform,
                            flags,
                        );
                    }
                    OpKind::StyleMap => {
                        let style_map_op_ptr = op_ptr as *mut StyleMapOp;
                        let style_map_op = &mut *style_map_op_ptr;
                        transform_binding_expression(
                            &mut style_map_op.expression,
                            transform,
                            flags,
                        );
                    }
                    OpKind::ClassProp => {
                        let class_prop_op_ptr = op_ptr as *mut ClassPropOp;
                        let class_prop_op = &mut *class_prop_op_ptr;
                        let expr = class_prop_op.expression.clone();
                        let transformed = transform(expr, flags);
                        class_prop_op.expression =
                            transform_expressions_in_expression(transformed, transform, flags);
                    }
                    OpKind::ClassMap => {
                        // ClassMapOp uses BindingExpression
                        let class_map_op_ptr = op_ptr as *mut ClassMapOp;
                        let class_map_op = &mut *class_map_op_ptr;
                        transform_binding_expression(
                            &mut class_map_op.expression,
                            transform,
                            flags,
                        );
                    }
                    _ => {}
                }
            }
            OpKind::Property | OpKind::Attribute | OpKind::Control => {
                // These ops have BindingExpression and sanitizer
                match op.kind() {
                    OpKind::Property => {
                        let property_op_ptr = op_ptr as *mut PropertyOp;
                        let property_op = &mut *property_op_ptr;
                        transform_binding_expression(&mut property_op.expression, transform, flags);
                        if let Some(ref mut sanitizer) = property_op.sanitizer {
                            *sanitizer = transform(sanitizer.clone(), flags);
                            *sanitizer = transform_expressions_in_expression(
                                sanitizer.clone(),
                                transform,
                                flags,
                            );
                        }
                    }
                    OpKind::Attribute => {
                        let attribute_op_ptr = op_ptr as *mut AttributeOp;
                        let attribute_op = &mut *attribute_op_ptr;
                        transform_binding_expression(
                            &mut attribute_op.expression,
                            transform,
                            flags,
                        );
                        if let Some(ref mut sanitizer) = attribute_op.sanitizer {
                            *sanitizer = transform(sanitizer.clone(), flags);
                            *sanitizer = transform_expressions_in_expression(
                                sanitizer.clone(),
                                transform,
                                flags,
                            );
                        }
                    }
                    OpKind::Control => {
                        let control_op_ptr = op_ptr as *mut ControlOp;
                        let control_op = &mut *control_op_ptr;
                        transform_binding_expression(&mut control_op.expression, transform, flags);
                        if let Some(ref mut sanitizer) = control_op.sanitizer {
                            *sanitizer = transform(sanitizer.clone(), flags);
                            *sanitizer = transform_expressions_in_expression(
                                sanitizer.clone(),
                                transform,
                                flags,
                            );
                        }
                    }
                    _ => {}
                }
            }
            OpKind::TwoWayProperty => {
                // TwoWayPropertyOp has Expression (not BindingExpression) and sanitizer
                let two_way_op_ptr = op_ptr as *mut TwoWayPropertyOp;
                let two_way_op = &mut *two_way_op_ptr;
                let expr = two_way_op.expression.clone();
                let transformed = transform(expr, flags);
                two_way_op.expression =
                    transform_expressions_in_expression(transformed, transform, flags);
                if let Some(ref mut sanitizer) = two_way_op.sanitizer {
                    let sanitizer_expr = sanitizer.clone();
                    let transformed_sanitizer = transform(sanitizer_expr, flags);
                    *sanitizer = transform_expressions_in_expression(
                        transformed_sanitizer,
                        transform,
                        flags,
                    );
                }
            }
            OpKind::I18nExpression => {
                let i18n_expr_op_ptr = op_ptr as *mut I18nExpressionOp;
                let i18n_expr_op = &mut *i18n_expr_op_ptr;
                let expr = i18n_expr_op.expression.clone();
                let transformed = transform(expr, flags);
                i18n_expr_op.expression =
                    transform_expressions_in_expression(transformed, transform, flags);
            }
            OpKind::InterpolateText => {
                let interpolate_text_op_ptr = op_ptr as *mut InterpolateTextOp;
                let interpolate_text_op = &mut *interpolate_text_op_ptr;
                for expr in &mut interpolate_text_op.interpolation.expressions {
                    *expr = transform(expr.clone(), flags);
                    *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                }
            }
            OpKind::Statement => {
                let stmt_op_ptr = op_ptr
                    as *mut StatementOp<
                        Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
                    >;
                let stmt_op = &mut *stmt_op_ptr;
                transform_expressions_in_statement(&mut stmt_op.statement, transform, flags);
            }
            OpKind::Variable => {
                let variable_op_ptr = op_ptr
                    as *mut VariableOp<
                        Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
                    >;
                let variable_op = &mut *variable_op_ptr;
                let expr = (*variable_op.initializer).clone();
                let transformed = transform(expr, flags);
                variable_op.initializer = Box::new(transform_expressions_in_expression(
                    transformed,
                    transform,
                    flags,
                ));
            }
            OpKind::Repeater => {
                // RepeaterOp has collection expression
                use crate::template::pipeline::ir::ops::update::RepeaterOp;
                let repeater_op_ptr = op_ptr as *mut RepeaterOp;
                let repeater_op = &mut *repeater_op_ptr;
                let expr = repeater_op.collection.clone();
                let transformed = transform(expr, flags);
                repeater_op.collection =
                    transform_expressions_in_expression(transformed, transform, flags);
            }
            _ => {
                // Other op types don't contain expressions or are handled elsewhere
            }
        }
    }
}

/// Transform expressions in a CreateOp
fn transform_expressions_in_create_op(
    op: &mut Box<dyn crate::template::pipeline::ir::operations::CreateOp + Send + Sync>,
    transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
    flags: VisitorContextFlag,
) {
    use crate::template::pipeline::ir::enums::OpKind;
    use crate::template::pipeline::ir::expression::transform_expressions_in_expression;
    use crate::template::pipeline::ir::ops::create::{
        DeferOp, ExtractedAttributeOp, ProjectionDefOp, RepeaterCreateOp,
    };

    unsafe {
        let op_ptr = op.as_mut() as *mut dyn crate::template::pipeline::ir::operations::CreateOp;

        match op.kind() {
            OpKind::Listener | OpKind::TwoWayListener | OpKind::AnimationListener => {
                use crate::template::pipeline::ir::ops::create::{
                    AnimationListenerOp, ListenerOp, TwoWayListenerOp,
                };
                match op.kind() {
                    OpKind::Listener => {
                        let listener_op_ptr = op_ptr as *mut ListenerOp;
                        let listener_op = &mut *listener_op_ptr;
                        for handler_op in listener_op.handler_ops.iter_mut() {
                            transform_expressions_in_op(handler_op, transform, flags);
                        }
                    }
                    OpKind::TwoWayListener => {
                        let two_way_op_ptr = op_ptr as *mut TwoWayListenerOp;
                        let two_way_op = &mut *two_way_op_ptr;
                        for handler_op in two_way_op.handler_ops.iter_mut() {
                            transform_expressions_in_op(handler_op, transform, flags);
                        }
                    }
                    OpKind::AnimationListener => {
                        let anim_op_ptr = op_ptr as *mut AnimationListenerOp;
                        let anim_op = &mut *anim_op_ptr;
                        for handler_op in anim_op.handler_ops.iter_mut() {
                            transform_expressions_in_op(handler_op, transform, flags);
                        }
                    }
                    _ => {}
                }
            }
            OpKind::RepeaterCreate => {
                let repeater_create_op_ptr = op_ptr as *mut RepeaterCreateOp;
                let repeater_create_op = &mut *repeater_create_op_ptr;

                // Transform track expression if track_by_ops is None
                if repeater_create_op.track_by_ops.is_none() {
                    let track_expr = (*repeater_create_op.track).clone();
                    let transformed = transform(track_expr, flags);
                    repeater_create_op.track = Box::new(transform_expressions_in_expression(
                        transformed,
                        transform,
                        flags,
                    ));
                } else {
                    // Transform expressions in track_by_ops
                    if let Some(ref mut track_by_ops) = repeater_create_op.track_by_ops {
                        for inner_op in track_by_ops.iter_mut() {
                            transform_expressions_in_op(inner_op, transform, flags);
                        }
                    }
                }

                // Transform track_by_fn if present
                if let Some(ref mut track_by_fn) = repeater_create_op.track_by_fn {
                    let fn_expr = (**track_by_fn).clone();
                    let transformed = transform(fn_expr, flags);
                    *track_by_fn = Box::new(transform_expressions_in_expression(
                        transformed,
                        transform,
                        flags,
                    ));
                }
            }
            OpKind::ExtractedAttribute => {
                let extracted_attr_op_ptr = op_ptr as *mut ExtractedAttributeOp;
                let extracted_attr_op = &mut *extracted_attr_op_ptr;

                // Transform expression if present
                if let Some(ref mut expr) = extracted_attr_op.expression {
                    let expr_clone = (*expr).clone();
                    let transformed = transform(expr_clone, flags);
                    *expr = transform_expressions_in_expression(transformed, transform, flags);
                }

                // Transform trusted_value_fn if present
                if let Some(ref mut trusted_fn) = extracted_attr_op.trusted_value_fn {
                    let fn_expr = (*trusted_fn).clone();
                    let transformed = transform(fn_expr, flags);
                    *trusted_fn =
                        transform_expressions_in_expression(transformed, transform, flags);
                }
            }
            OpKind::ProjectionDef => {
                let projection_def_op_ptr = op_ptr as *mut ProjectionDefOp;
                let projection_def_op = &mut *projection_def_op_ptr;

                // Transform def expression if present
                if let Some(ref mut def) = projection_def_op.def {
                    let def_expr = (*def).clone();
                    let transformed = transform(def_expr, flags);
                    *def = transform_expressions_in_expression(transformed, transform, flags);
                }
            }
            OpKind::Defer => {
                let defer_op_ptr = op_ptr as *mut DeferOp;
                let defer_op = &mut *defer_op_ptr;

                // Transform loading_config if present
                if let Some(ref mut loading_config) = defer_op.loading_config {
                    let expr = loading_config.clone();
                    let transformed = transform(expr, flags);
                    *loading_config =
                        transform_expressions_in_expression(transformed, transform, flags);
                }

                // Transform placeholder_config if present
                if let Some(ref mut placeholder_config) = defer_op.placeholder_config {
                    let expr = placeholder_config.clone();
                    let transformed = transform(expr, flags);
                    *placeholder_config =
                        transform_expressions_in_expression(transformed, transform, flags);
                }

                // Transform resolver_fn if present
                if let Some(ref mut resolver_fn) = defer_op.resolver_fn {
                    let expr = resolver_fn.clone();
                    let transformed = transform(expr, flags);
                    *resolver_fn =
                        transform_expressions_in_expression(transformed, transform, flags);
                }

                // Transform own_resolver_fn if present
                if let Some(ref mut own_resolver_fn) = defer_op.own_resolver_fn {
                    let expr = own_resolver_fn.clone();
                    let transformed = transform(expr, flags);
                    *own_resolver_fn =
                        transform_expressions_in_expression(transformed, transform, flags);
                }
            }
            _ => {
                // Other create op types don't contain expressions or are handled elsewhere
            }
        }
    }
}

/// Helper to transform a BindingExpression
fn transform_binding_expression(
    binding_expr: &mut crate::template::pipeline::ir::ops::update::BindingExpression,
    transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
    flags: VisitorContextFlag,
) {
    use crate::template::pipeline::ir::expression::transform_expressions_in_expression;
    use crate::template::pipeline::ir::ops::update::BindingExpression;

    match binding_expr {
        BindingExpression::Expression(expr) => {
            *expr = transform(expr.clone(), flags);
            *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
        }
        BindingExpression::Interpolation(interp) => {
            for expr in &mut interp.expressions {
                *expr = transform(expr.clone(), flags);
                *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
            }
        }
    }
}

/// Remove $any function calls - replace them with their argument
fn remove_anys(e: Expression, _flags: VisitorContextFlag) -> Expression {
    // Check if this is an InvokeFunctionExpr
    if let Expression::InvokeFn(invoke) = &e {
        // Check if fn is a LexicalRead with name '$any'
        if let Expression::LexicalRead(lexical_read) = invoke.fn_.as_ref() {
            if &*lexical_read.name == "$any" {
                // Check that there's exactly one argument
                if invoke.args.len() != 1 {
                    panic!("The $any builtin function expects exactly one argument.");
                }
                // Return the argument - the caller will recursively transform it
                return invoke.args[0].clone();
            }
        }

        // Also check if fn is a ReadProp with name '$any' (e.g., ctx.$any(...))
        if let Expression::ReadProp(read_prop) = invoke.fn_.as_ref() {
            if read_prop.name == "$any" {
                // Check that there's exactly one argument
                if invoke.args.len() != 1 {
                    panic!("The $any builtin function expects exactly one argument.");
                }
                // Return the argument - the caller will recursively transform it
                return invoke.args[0].clone();
            }
        }
    }

    // Return the expression unchanged - the caller handles recursive traversal
    e
}
