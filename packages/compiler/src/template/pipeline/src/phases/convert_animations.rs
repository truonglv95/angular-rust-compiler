//! Convert Animations Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/convert_animations.ts
//! Converts AnimationBindingOp to AnimationOp or AnimationStringOp and moves them to create ops

use crate::output::output_ast::{ExpressionTrait, ReturnStatement, Statement};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{AnimationBindingKind, AnimationKind, OpKind};
use crate::template::pipeline::ir::ops::create::{create_animation_op, create_animation_string_op};
use crate::template::pipeline::ir::ops::shared::create_statement_op;
use crate::template::pipeline::ir::ops::update::AnimationBindingOp;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, CompilationUnit, ComponentCompilationJob,
};
use std::collections::HashMap;

/// Converts AnimationBindingOp to AnimationOp or AnimationStringOp and moves them to create ops
pub fn convert_animations(job: &mut dyn CompilationJob) {
    let component_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let job_ptr = job_ptr as *mut ComponentCompilationJob;
        &mut *job_ptr
    };

    // Process root unit (build unit-specific element map)
    {
        let root_elements_map = build_elements_map(&component_job.root);
        process_unit(&mut component_job.root, job, &root_elements_map);
    }

    // Process all view units (build unit-specific element maps)
    let view_keys: Vec<_> = component_job.views.keys().cloned().collect();
    for key in view_keys {
        if let Some(unit) = component_job.views.get_mut(&key) {
            let unit_elements_map = build_elements_map(unit);
            process_unit(unit, job, &unit_elements_map);
        }
    }
}

/// Build a map of elements by xref for a specific unit
fn build_elements_map(unit: &dyn CompilationUnit) -> HashMap<ir::XrefId, usize> {
    let mut elements_map: HashMap<ir::XrefId, usize> = HashMap::new();

    for (index, op) in unit.create().iter().enumerate() {
        match op.kind() {
            OpKind::ElementStart | OpKind::Element | OpKind::ContainerStart | OpKind::Container => {
                elements_map.insert(op.xref(), index);
            }
            _ => {}
        }
    }

    elements_map
}

fn process_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    job: &dyn CompilationJob,
    elements_map: &HashMap<ir::XrefId, usize>,
) {
    // Collect AnimationBindingOps to convert
    let mut ops_to_convert: Vec<(usize, AnimationBindingOp)> = Vec::new();

    // First pass: collect all AnimationBindingOps
    for (index, op) in unit.update().iter().enumerate() {
        if op.kind() == OpKind::AnimationBinding {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                let anim_binding_op_ptr = op_ptr as *const AnimationBindingOp;
                let anim_binding_op = &*anim_binding_op_ptr;
                ops_to_convert.push((index, anim_binding_op.clone()));
            }
        }
    }

    // Second pass: convert and insert/create ops (iterate in reverse to maintain indices)
    for (index, anim_binding_op) in ops_to_convert.iter().rev() {
        let create_animation_op = get_animation_op(anim_binding_op);

        if job.kind() == CompilationJobKind::Host {
            // For host bindings, just push to create
            unit.create_mut().push(create_animation_op);
        } else {
            // Insert after the target element
            if let Some(element_index) = elements_map.get(&anim_binding_op.target) {
                let insert_index = *element_index + 1; // Insert after element
                if insert_index <= unit.create().len() {
                    unit.create_mut()
                        .insert_at(insert_index, create_animation_op);
                } else {
                    // If insert_index is beyond bounds, just push
                    unit.create_mut().push(create_animation_op);
                }
            }
        }

        // Remove the AnimationBindingOp from update list
        unit.update_mut().remove_at(*index);
    }
}

fn get_animation_op(op: &AnimationBindingOp) -> Box<dyn ir::CreateOp + Send + Sync> {
    if op.animation_binding_kind == AnimationBindingKind::String {
        // Simple string case
        create_animation_string_op(
            op.name.clone(),
            op.target,
            if &*op.name == "animate.enter" {
                AnimationKind::Enter
            } else {
                AnimationKind::Leave
            },
            op.expression.clone(),
            op.security_context.clone(),
            op.source_span.clone(),
        )
    } else {
        // Value case - wrap expression in ReturnStatement
        let expression = match &op.expression {
            ir::ops::update::BindingExpression::Expression(expr) => expr.clone(),
            ir::ops::update::BindingExpression::Interpolation(_) => {
                panic!("AnimationBindingOp with VALUE kind should have Expression binding");
            }
        };

        let return_stmt = Statement::Return(ReturnStatement {
            value: Box::new(expression.clone()),
            source_span: expression.source_span().cloned(),
        });

        let mut handler_ops = ir::operations::OpList::new();
        let stmt_op =
            create_statement_op::<Box<dyn ir::UpdateOp + Send + Sync>>(Box::new(return_stmt));
        // Convert StatementOp to Box<dyn UpdateOp>
        handler_ops.push(Box::new(stmt_op) as Box<dyn ir::UpdateOp + Send + Sync>);

        create_animation_op(
            op.name.clone(),
            op.target,
            if &*op.name == "animate.enter" {
                AnimationKind::Enter
            } else {
                AnimationKind::Leave
            },
            handler_ops,
            op.source_span.clone(),
        )
    }
}
