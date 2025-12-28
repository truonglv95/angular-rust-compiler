//! Variable Counting Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/var_counting.ts
//! Counts the number of variable slots used within each view, and stores that on the view itself, as
//! well as propagates it to the `ir.TemplateOp` for embedded views.

use crate::output::output_ast::Expression;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{CompatibilityMode, ExpressionKind, OpKind};
use crate::template::pipeline::ir::expression::{
    is_ir_expression, transform_expressions_in_op, VisitorContextFlag,
};
use crate::template::pipeline::ir::ops::update::{BindingExpression, Interpolation};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, CompilationUnit, ComponentCompilationJob,
};

/// Counts the number of variable slots used within each view, and stores that on the view itself, as
/// well as propagates it to the `ir.TemplateOp` for embedded views.
pub fn phase(job: &mut dyn CompilationJob) {
    let compatibility = job.compatibility();

    // First, count the vars used in each unit, and update the unit-level counter.
    for unit in job.units_mut() {
        process_unit_with_compatibility(unit, compatibility);
    }

    // Propagate var counts to TemplateOp for embedded views (only for ComponentCompilationJob)
    if let Some(component_job) = job.as_any_mut().downcast_mut::<ComponentCompilationJob>() {
        propagate_var_counts_to_embedded_views(component_job);
    }
}

fn process_unit_with_compatibility(
    unit: &mut dyn CompilationUnit,
    compatibility: CompatibilityMode,
) {
    use crate::template::pipeline::src::compilation::CompilationUnit;
    let mut var_count = 0;

    // Count variables on top-level ops first. Don't explore nested expressions just yet.
    for op in unit.create().iter() {
        if check_consumes_vars_trait_create(op) {
            var_count += vars_used_by_op_create(op);
        }
    }

    for op in unit.update().iter() {
        if check_consumes_vars_trait_update(op) {
            let vars_used = vars_used_by_op_update(op);
            var_count += vars_used;
        }
    }

    // Count variables on expressions inside ops. We do this later because some of these expressions
    // might be conditional (e.g. `pipeBinding` inside of a ternary), and we don't want to interfere
    // with indices for top-level binding slots (e.g. `property`).
    for op in unit.create_mut().iter_mut() {
        visit_expressions_for_var_counting(
            op.as_mut(),
            &mut var_count,
            compatibility,
            false, // not compatibility mode pass yet
        );
    }

    for op in unit.update_mut().iter_mut() {
        visit_expressions_for_var_counting(
            op.as_mut(),
            &mut var_count,
            compatibility,
            false, // not compatibility mode pass yet
        );
    }

    // Compatibility mode pass for pure function offsets (as explained above).
    if compatibility == CompatibilityMode::TemplateDefinitionBuilder {
        for op in unit.create_mut().iter_mut() {
            visit_expressions_for_var_counting(
                op.as_mut(),
                &mut var_count,
                compatibility,
                true, // compatibility mode pass for pure functions
            );
        }

        for op in unit.update_mut().iter_mut() {
            visit_expressions_for_var_counting(
                op.as_mut(),
                &mut var_count,
                compatibility,
                true, // compatibility mode pass for pure functions
            );
        }
    }

    unit.set_vars(var_count);
}

fn visit_expressions_for_var_counting(
    op: &mut dyn ir::operations::Op,
    var_count: &mut usize,
    compatibility: CompatibilityMode,
    compatibility_mode_pass: bool,
) {
    transform_expressions_in_op(
        op,
        &mut |expr: Expression, _flag: VisitorContextFlag| -> Expression {
            if !is_ir_expression(&expr) {
                return expr;
            }

            // Check if this is an IR expression that we should process
            let expr_kind = match &expr {
                Expression::PureFunction(_) => Some(ExpressionKind::PureFunctionExpr),
                Expression::PipeBinding(_) => Some(ExpressionKind::PipeBinding),
                Expression::PipeBindingVariadic(_) => Some(ExpressionKind::PipeBindingVariadic),
                Expression::StoreLet(_) => Some(ExpressionKind::StoreLet),
                _ => None,
            };

            // TemplateDefinitionBuilder assigns variable offsets for everything but pure functions
            // first, and then assigns offsets to pure functions lazily. We emulate that behavior by
            // assigning offsets in two passes instead of one, only in compatibility mode.
            if compatibility_mode_pass {
                // In compatibility mode pass, only process PureFunctionExpr
                if expr_kind != Some(ExpressionKind::PureFunctionExpr) {
                    return expr;
                }
            } else {
                // In normal pass, skip PureFunctionExpr if in compatibility mode
                if compatibility == CompatibilityMode::TemplateDefinitionBuilder
                    && expr_kind == Some(ExpressionKind::PureFunctionExpr)
                {
                    return expr;
                }
            }

            let mut modified_expr = expr;

            // Some expressions require knowledge of the number of variable slots consumed.
            if check_uses_var_offset_trait(&modified_expr) {
                set_var_offset(&mut modified_expr, *var_count);
            }

            // Check if expression consumes vars and count them
            if check_consumes_vars_trait_expr(&modified_expr) {
                *var_count += vars_used_by_ir_expression(&modified_expr);
            }

            modified_expr
        },
        VisitorContextFlag::NONE,
    );
}

/// Check if a CreateOp implements ConsumesVarsTrait
fn check_consumes_vars_trait_create(op: &Box<dyn ir::CreateOp + Send + Sync>) -> bool {
    matches!(op.kind(), OpKind::RepeaterCreate)
}

/// Check if an UpdateOp implements ConsumesVarsTrait
fn check_consumes_vars_trait_update(op: &Box<dyn ir::UpdateOp + Send + Sync>) -> bool {
    // Check based on OpKind
    matches!(
        op.kind(),
        OpKind::Attribute
            | OpKind::Property
            | OpKind::DomProperty
            | OpKind::Control
            | OpKind::TwoWayProperty
            | OpKind::StyleProp
            | OpKind::ClassProp
            | OpKind::StyleMap
            | OpKind::ClassMap
            | OpKind::InterpolateText
            | OpKind::I18nExpression
            | OpKind::Conditional
            | OpKind::DeferWhen
            | OpKind::StoreLet
    )
}

/// Check if an expression implements ConsumesVarsTrait
fn check_consumes_vars_trait_expr(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::PureFunction(_)
            | Expression::PipeBinding(_)
            | Expression::PipeBindingVariadic(_)
            | Expression::StoreLet(_)
    )
}

/// Check if an expression implements UsesVarOffsetTrait
fn check_uses_var_offset_trait(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::PureFunction(_)
            | Expression::PipeBinding(_)
            | Expression::PipeBindingVariadic(_)
    )
}

/// Set var_offset on an expression
fn set_var_offset(expr: &mut Expression, offset: usize) {
    match expr {
        Expression::PureFunction(pure_fn) => {
            pure_fn.var_offset = Some(offset);
        }
        Expression::PipeBinding(pipe) => {
            pipe.var_offset = Some(offset);
        }
        Expression::PipeBindingVariadic(pipe_var) => {
            pipe_var.var_offset = Some(offset);
        }
        _ => {}
    }
}

/// Different operations that implement `ir.UsesVarsTrait` use different numbers of variables, so
/// count the variables used by any particular `op`.
fn vars_used_by_op_create(op: &Box<dyn ir::CreateOp + Send + Sync>) -> usize {
    match op.kind() {
        OpKind::RepeaterCreate => {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                use crate::template::pipeline::ir::ops::create::RepeaterCreateOp;
                let rep_ptr = op_ptr as *const RepeaterCreateOp;
                let rep = &*rep_ptr;
                // Repeaters may require an extra variable binding slot, if they have an empty view, for the
                // empty block tracking.
                if rep.empty_view.is_some() {
                    1
                } else {
                    0
                }
            }
        }
        _ => 0,
    }
}

fn vars_used_by_op_update(op: &Box<dyn ir::UpdateOp + Send + Sync>) -> usize {
    unsafe {
        let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;

        match op.kind() {
            OpKind::Attribute => {
                use crate::template::pipeline::ir::ops::update::AttributeOp;
                let attr_ptr = op_ptr as *const AttributeOp;
                let attr = &*attr_ptr;
                // All of these bindings use 1 variable slot, plus 1 slot for every interpolated expression,
                // if any.
                let mut slots = 1;
                if let BindingExpression::Interpolation(ref interp) = attr.expression {
                    if !is_singleton_interpolation(interp) {
                        slots += interp.expressions.len();
                    }
                }
                slots
            }
            OpKind::Property | OpKind::DomProperty => {
                use crate::template::pipeline::ir::ops::update::PropertyOp;
                let prop_ptr = if op.kind() == OpKind::Property {
                    op_ptr as *const PropertyOp
                } else {
                    op_ptr as *const PropertyOp // DomProperty uses same struct
                };
                let prop = &*prop_ptr;
                let mut slots = 1;
                // We need to assign a slot even for singleton interpolations, because the
                // runtime needs to store both the raw value and the stringified one.
                if let BindingExpression::Interpolation(ref interp) = prop.expression {
                    slots += interp.expressions.len();
                }
                slots
            }
            OpKind::Control => {
                // 1 for the [field] binding itself.
                // 1 for the control bindings object containing bound field states properties.
                2
            }
            OpKind::TwoWayProperty => {
                // Two-way properties can only have expressions so they only need one variable slot.
                1
            }
            OpKind::StyleProp => {
                use crate::template::pipeline::ir::ops::update::StylePropOp;
                let style_ptr = op_ptr as *const StylePropOp;
                let style = &*style_ptr;
                let mut slots = 2;
                if let BindingExpression::Interpolation(ref interp) = style.expression {
                    slots += interp.expressions.len();
                }
                slots
            }
            OpKind::ClassProp => {
                // ClassProp uses Expression, not BindingExpression
                // For now, assume 2 slots (standard for class bindings)
                // TODO: Check if expression is an interpolation and add extra slots if needed
                2
            }
            OpKind::StyleMap => {
                use crate::template::pipeline::ir::ops::update::StyleMapOp;
                let style_map_ptr = op_ptr as *const StyleMapOp;
                let style_map = &*style_map_ptr;
                let mut slots = 2;
                if let BindingExpression::Interpolation(ref interp) = style_map.expression {
                    slots += interp.expressions.len();
                }
                slots
            }
            OpKind::ClassMap => {
                use crate::template::pipeline::ir::ops::update::ClassMapOp;
                let class_map_ptr = op_ptr as *const ClassMapOp;
                let class_map = &*class_map_ptr;
                let mut slots = 2;
                if let BindingExpression::Interpolation(ref interp) = class_map.expression {
                    slots += interp.expressions.len();
                }
                slots
            }
            OpKind::InterpolateText => {
                use crate::template::pipeline::ir::ops::update::InterpolateTextOp;
                let interp_text_ptr = op_ptr as *const InterpolateTextOp;
                let interp_text = &*interp_text_ptr;
                // `ir.InterpolateTextOp`s use a variable slot for each dynamic expression.
                interp_text.interpolation.expressions.len()
            }
            OpKind::I18nExpression | OpKind::Conditional | OpKind::DeferWhen | OpKind::StoreLet => {
                1
            }
            _ => 0,
        }
    }
}

pub fn vars_used_by_ir_expression(expr: &Expression) -> usize {
    match expr {
        Expression::PureFunction(pure_fn) => 1 + pure_fn.args.len(),
        Expression::PipeBinding(pipe) => 1 + pipe.args.len(),
        Expression::PipeBindingVariadic(pipe_var) => 1 + pipe_var.num_args,
        Expression::StoreLet(_) => 1,
        _ => 0,
    }
}

fn is_singleton_interpolation(interp: &Interpolation) -> bool {
    if interp.expressions.len() != 1 || interp.strings.len() != 2 {
        return false;
    }
    if interp.strings[0] != "" || interp.strings[1] != "" {
        return false;
    }
    true
}

/// Propagate var counts for each view to the `ir.TemplateOp` which declares that view (if the view is
/// an embedded view).
fn propagate_var_counts_to_embedded_views(component_job: &mut ComponentCompilationJob) {
    // First collect all var counts we need
    let mut var_counts: std::collections::HashMap<ir::XrefId, usize> =
        std::collections::HashMap::new();

    for (xref, unit) in component_job.views.iter() {
        if let Some(vars) = unit.vars() {
            var_counts.insert(*xref, vars);
        }
    }

    // Then update ops with the var counts
    for op in component_job.root.create_mut().iter_mut() {
        propagate_var_count_for_op_with_map(op, &var_counts);
    }

    // Process all view units
    for (_, unit) in component_job.views.iter_mut() {
        for op in unit.create_mut().iter_mut() {
            propagate_var_count_for_op_with_map(op, &var_counts);
        }
    }
}

fn propagate_var_count_for_op_with_map(
    op: &mut Box<dyn ir::CreateOp + Send + Sync>,
    var_counts: &std::collections::HashMap<ir::XrefId, usize>,
) {
    let xref = op.xref();
    // Get vars from map
    let vars = match op.kind() {
        OpKind::Template
        | OpKind::RepeaterCreate
        | OpKind::ConditionalCreate
        | OpKind::ConditionalBranchCreate => var_counts.get(&xref).copied(),
        _ => None,
    };

    if let Some(vars) = vars {
        unsafe {
            let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
            match op.kind() {
                OpKind::Template => {
                    use crate::template::pipeline::ir::ops::create::TemplateOp;
                    let template_ptr = op_ptr as *mut TemplateOp;
                    let template = &mut *template_ptr;
                    template.vars = Some(vars);
                }
                OpKind::RepeaterCreate => {
                    use crate::template::pipeline::ir::ops::create::RepeaterCreateOp;
                    let rep_ptr = op_ptr as *mut RepeaterCreateOp;
                    let rep = &mut *rep_ptr;
                    rep.vars = Some(vars);
                }
                OpKind::ConditionalCreate => {
                    use crate::template::pipeline::ir::ops::create::ConditionalCreateOp;
                    let cond_ptr = op_ptr as *mut ConditionalCreateOp;
                    let cond = &mut *cond_ptr;
                    cond.vars = Some(vars);
                }
                OpKind::ConditionalBranchCreate => {
                    use crate::template::pipeline::ir::ops::create::ConditionalBranchCreateOp;
                    let branch_ptr = op_ptr as *mut ConditionalBranchCreateOp;
                    let branch = &mut *branch_ptr;
                    branch.vars = Some(vars);
                }
                _ => {}
            }
        }
    }
}
