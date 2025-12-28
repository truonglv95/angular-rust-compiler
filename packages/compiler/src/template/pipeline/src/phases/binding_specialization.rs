//! Binding Specialization Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/binding_specialization.ts
//! Specializes BindingOp into more specific operation types

use crate::ml_parser::tags::split_ns_name;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{
    AnimationBindingKind, AnimationKind, BindingKind, OpKind,
};
use crate::template::pipeline::ir::ops::create::{
    ContainerOp, ContainerStartOp, ElementOp, ElementOrContainerOpBase, ElementStartOp,
};
use crate::template::pipeline::ir::ops::update::BindingOp;
use crate::template::pipeline::ir::ops::update::{
    create_animation_binding_op, create_attribute_op, create_control_op, create_property_op,
    create_two_way_property_op,
};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, CompilationUnit, ComponentCompilationJob,
    TemplateCompilationMode,
};
use crate::template::pipeline::src::util::attributes::is_aria_attribute;

/// Looks up an element in the given map by xref ID.
fn lookup_element<'a>(
    elements: &'a std::collections::HashMap<ir::XrefId, usize>,
    unit: &'a mut dyn CompilationUnit,
    xref: ir::XrefId,
) -> &'a mut ElementOrContainerOpBase {
    let index = elements
        .get(&xref)
        .expect("All attributes should have an element-like target.");

    // Need to downcast to get mutable access to non_bindable
    // This is complex - we'll use unsafe
    unsafe {
        let op = unit.create_mut().get_mut(*index).unwrap();
        let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;

        // Try ElementStartOp first
        if op.kind() == OpKind::ElementStart {
            let elem_start_ptr = op_ptr as *mut ElementStartOp;
            let elem_start = &mut *elem_start_ptr;
            return &mut elem_start.base.base;
        }

        // Try ElementOp
        if op.kind() == OpKind::Element {
            let elem_ptr = op_ptr as *mut ElementOp;
            let elem = &mut *elem_ptr;
            return &mut elem.base.base;
        }

        // Try ContainerStartOp
        if op.kind() == OpKind::ContainerStart {
            let container_ptr = op_ptr as *mut ContainerStartOp;
            let container = &mut *container_ptr;
            return &mut container.base;
        }

        // Try ContainerOp
        if op.kind() == OpKind::Container {
            let container_ptr = op_ptr as *mut ContainerOp;
            let container = &mut *container_ptr;
            return &mut container.base;
        }

        panic!("Expected element or container op");
    }
}

pub fn specialize_bindings(job: &mut dyn CompilationJob) {
    // Build map of elements - collect indices for all element/container ops
    let mut elements_map: std::collections::HashMap<ir::XrefId, usize> =
        std::collections::HashMap::new();

    // Collect from all units
    for unit in job.units() {
        for (index, op) in unit.create().iter().enumerate() {
            match op.kind() {
                OpKind::ElementStart
                | OpKind::Element
                | OpKind::ContainerStart
                | OpKind::Container => {
                    elements_map.insert(op.xref(), index);
                }
                _ => {}
            }
        }
    }

    let mode = job.mode();
    let kind = job.kind();

    // Process all units
    for unit in job.units_mut() {
        process_unit(unit, mode, kind, &elements_map);
    }
}

fn process_unit(
    unit: &mut dyn CompilationUnit,
    mode: TemplateCompilationMode,
    kind: CompilationJobKind,
    elements_map: &std::collections::HashMap<ir::XrefId, usize>,
) {
    // Collect BindingOps to replace
    let mut ops_to_replace: Vec<(usize, BindingOp)> = Vec::new();

    // First pass: collect all BindingOps
    for (index, op) in unit.update().iter().enumerate() {
        if op.kind() == OpKind::Binding {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                let binding_op_ptr = op_ptr as *const BindingOp;
                let binding_op = &*binding_op_ptr;
                ops_to_replace.push((index, binding_op.clone()));
            }
        }
    }

    // Second pass: replace ops (iterate in reverse to maintain indices)
    for (index, binding_op) in ops_to_replace.iter().rev() {
        let replacement = match binding_op.binding_kind {
            BindingKind::Attribute => {
                if binding_op.name == "ngNonBindable" {
                    // Set non_bindable flag on element and remove op
                    if elements_map.contains_key(&binding_op.target) {
                        let base = lookup_element(elements_map, unit, binding_op.target);
                        base.non_bindable = true;
                    }
                    None // Remove op
                } else if binding_op.name.starts_with("animate.") {
                    // Convert to AnimationBindingOp
                    let animation_kind = if binding_op.name == "animate.enter" {
                        AnimationKind::Enter
                    } else {
                        AnimationKind::Leave
                    };
                    Some(create_animation_binding_op(
                        binding_op.name.clone(),
                        binding_op.target,
                        animation_kind,
                        binding_op.expression.clone(),
                        binding_op.security_context.clone(),
                        binding_op.source_span.clone(),
                        AnimationBindingKind::String,
                    ))
                } else {
                    // Convert to AttributeOp
                    let (namespace_opt, name) = split_ns_name(&binding_op.name, false)
                        .unwrap_or((None, binding_op.name.clone()));

                    Some(create_attribute_op(
                        binding_op.target,
                        namespace_opt,
                        name,
                        binding_op.expression.clone(),
                        binding_op.security_context.clone(),
                        binding_op.is_text_attribute,
                        binding_op.is_structural_template_attribute,
                        binding_op.template_kind,
                        binding_op.i18n_message.clone(),
                        binding_op.source_span.clone(),
                    ))
                }
            }
            BindingKind::Animation => {
                let animation_kind = if binding_op.name == "animate.enter" {
                    AnimationKind::Enter
                } else {
                    AnimationKind::Leave
                };
                Some(create_animation_binding_op(
                    binding_op.name.clone(),
                    binding_op.target,
                    animation_kind,
                    binding_op.expression.clone(),
                    binding_op.security_context.clone(),
                    binding_op.source_span.clone(),
                    AnimationBindingKind::Value,
                ))
            }
            BindingKind::Property | BindingKind::LegacyAnimation => {
                if binding_op.name.starts_with("style.") {
                    let style_prop_name = binding_op.name[6..].to_string();
                    Some(
                        crate::template::pipeline::ir::ops::update::create_style_prop_op(
                            binding_op.target,
                            style_prop_name,
                            binding_op.expression.clone(),
                            binding_op.unit.clone(),
                            binding_op.source_span.clone(),
                        ),
                    )
                } else if binding_op.name.starts_with("class.") {
                    let class_prop_name = binding_op.name[6..].to_string();
                    // ClassPropOp expects Expression, convert Interpolation if needed
                    let expression = match &binding_op.expression {
                        crate::template::pipeline::ir::ops::update::BindingExpression::Expression(expr) => expr.clone(),
                        crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(interp) => {
                            // Fallback for interpolation in class binding (should be rare for class.prop)
                             crate::output::output_ast::Expression::Literal(crate::output::output_ast::LiteralExpr {
                                value: crate::output::output_ast::LiteralValue::String(interp.strings.join("")),
                                type_: None,
                                source_span: None,
                            })
                        }
                    };
                    Some(
                        crate::template::pipeline::ir::ops::update::create_class_prop_op(
                            binding_op.target,
                            class_prop_name,
                            expression,
                            binding_op.source_span.clone(),
                        ),
                    )
                } else {
                    // Convert property binding to appropriate op
                    if mode == TemplateCompilationMode::DomOnly
                        && is_aria_attribute(&binding_op.name)
                    {
                        // Convert ARIA property to attribute in DomOnly mode
                        Some(create_attribute_op(
                            binding_op.target,
                            None, // namespace
                            binding_op.name.clone(),
                            binding_op.expression.clone(),
                            binding_op.security_context.clone(),
                            false, // is_text_attribute
                            binding_op.is_structural_template_attribute,
                            binding_op.template_kind,
                            binding_op.i18n_message.clone(),
                            binding_op.source_span.clone(),
                        ))
                    } else if kind == CompilationJobKind::Host {
                        // For host bindings, create PropertyOp (DomPropertyOp doesn't exist, use PropertyOp)
                        Some(create_property_op(
                            binding_op.target,
                            binding_op.name.clone(),
                            binding_op.expression.clone(),
                            binding_op.binding_kind,
                            binding_op.security_context.clone(),
                            binding_op.is_structural_template_attribute,
                            binding_op.template_kind,
                            binding_op.i18n_context,
                            binding_op.i18n_message.clone(),
                            binding_op.source_span.clone(),
                        ))
                    } else if binding_op.name == "field" {
                        // Convert to ControlOp
                        Some(create_control_op(binding_op))
                    } else {
                        // Convert to PropertyOp
                        Some(create_property_op(
                            binding_op.target,
                            binding_op.name.clone(),
                            binding_op.expression.clone(),
                            binding_op.binding_kind,
                            binding_op.security_context.clone(),
                            binding_op.is_structural_template_attribute,
                            binding_op.template_kind,
                            binding_op.i18n_context,
                            binding_op.i18n_message.clone(),
                            binding_op.source_span.clone(),
                        ))
                    }
                }
            }
            BindingKind::TwoWayProperty => {
                // TwoWayProperty - expression must be Expression, not Interpolation
                let expression = match &binding_op.expression {
                    crate::template::pipeline::ir::ops::update::BindingExpression::Expression(expr) => expr.clone(),
                    crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(_) => {
                        panic!("Expected value of two-way property binding \"{}\" to be an expression", binding_op.name);
                    }
                };
                Some(create_two_way_property_op(
                    binding_op.target,
                    binding_op.name.clone(),
                    expression,
                    binding_op.security_context.clone(),
                    binding_op.is_structural_template_attribute,
                    binding_op.template_kind,
                    binding_op.i18n_context,
                    binding_op.i18n_message.clone(),
                    binding_op.source_span.clone(),
                ))
            }
            BindingKind::Template => {
                // Template bindings are handled separately, don't replace here
                None
            }
            BindingKind::I18n => {
                // I18n bindings are handled separately, don't replace here
                None
            }
            BindingKind::ClassName => {
                // Convert to ClassPropOp
                // ClassName bindings expect Expression, not Interpolation
                let expression = match &binding_op.expression {
                    crate::template::pipeline::ir::ops::update::BindingExpression::Expression(expr) => expr.clone(),
                    crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(interp) => {
                        // Convert interpolation to a string concatenation expression if needed
                        // For now, use Literal(String) as fallback
                        crate::output::output_ast::Expression::Literal(crate::output::output_ast::LiteralExpr {
                            value: crate::output::output_ast::LiteralValue::String(interp.strings.join("")),
                            type_: None,
                            source_span: None,
                        })
                    }
                };
                Some(
                    crate::template::pipeline::ir::ops::update::create_class_prop_op(
                        binding_op.target,
                        binding_op.name.clone(),
                        expression,
                        binding_op.source_span.clone(),
                    ),
                )
            }
            BindingKind::StyleProperty => {
                // Convert to StylePropOp
                Some(
                    crate::template::pipeline::ir::ops::update::create_style_prop_op(
                        binding_op.target,
                        binding_op.name.clone(),
                        binding_op.expression.clone(),
                        binding_op.unit.clone(),
                        binding_op.source_span.clone(),
                    ),
                )
            }
        };

        if let Some(new_op) = replacement {
            unit.update_mut().replace_at(*index, new_op);
        } else {
            // Remove the op
            unit.update_mut().remove_at(*index);
        }
    }
}
