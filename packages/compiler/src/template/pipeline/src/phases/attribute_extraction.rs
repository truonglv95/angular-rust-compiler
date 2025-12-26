//! Attribute Extraction Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/attribute_extraction.ts
//! Finds all extractable attribute and binding ops, and creates ExtractedAttributeOps for them

use crate::core::SecurityContext;
use crate::output::output_ast::ExpressionTrait;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{BindingKind, OpKind};
use crate::template::pipeline::ir::operations::Op;
use crate::template::pipeline::ir::ops::create::{
    create_extracted_attribute_op, ListenerOp, TwoWayListenerOp,
};
use crate::template::pipeline::ir::ops::update::{
    AttributeOp, BindingExpression, ClassPropOp, ControlOp, PropertyOp, StylePropOp,
    TwoWayPropertyOp,
};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, ComponentCompilationJob, ViewCompilationUnit,
};
use std::collections::{HashMap, HashSet};

/// Find all extractable attribute and binding ops, and create ExtractedAttributeOps for them.
/// In cases where no instruction needs to be generated for the attribute or binding, it is removed.
pub fn extract_attributes(job: &mut dyn CompilationJob) {
    let compatibility = job.compatibility();
    let component_job = job
        .as_any_mut()
        .downcast_mut::<ComponentCompilationJob>()
        .expect("extract_attributes only supports ComponentCompilationJob");

    // Process root unit
    process_unit(
        &mut component_job.root,
        compatibility,
        CompilationJobKind::Tmpl,
    );

    // Process all view units
    for (_, unit) in component_job.views.iter_mut() {
        process_unit(unit, compatibility, CompilationJobKind::Tmpl);
    }
}

fn process_unit(
    unit: &mut ViewCompilationUnit,
    compatibility: ir::CompatibilityMode,
    job_kind: CompilationJobKind,
) {
    // Map to collect extracted attributes per target element
    let mut extracted_attributes: HashMap<ir::XrefId, Vec<Box<dyn ir::CreateOp + Send + Sync>>> =
        HashMap::new();

    // Temporarily take update list to avoid borrow conflicts
    let mut update_list = std::mem::replace(&mut unit.update, ir::OpList::new());
    let mut update_ops_to_remove = HashSet::new();

    // Identify which XrefIds belong to templates
    let mut template_xrefs = HashSet::new();
    let create_list = std::mem::replace(&mut unit.create, ir::OpList::new());
    for op in create_list.iter() {
        if op.kind() == OpKind::Template {
            if let Some(xref) = get_xref_from_create_op(op.as_ref()) {
                template_xrefs.insert(xref);
            }
        }
    }

    // Process update ops
    for (index, op_ref) in update_list.iter().enumerate() {
        if let Some(attr_op) = op_ref.as_any().downcast_ref::<AttributeOp>() {
            // Skip if expression is an interpolation
            if matches!(attr_op.expression, BindingExpression::Interpolation(_)) {
                continue;
            }

            let is_constant = match &attr_op.expression {
                BindingExpression::Expression(expr) => expr.is_constant(),
                BindingExpression::Interpolation(_) => false,
            };

            let mut extractable = attr_op.is_text_attribute || is_constant;

            if compatibility == ir::CompatibilityMode::TemplateDefinitionBuilder {
                extractable = extractable && attr_op.is_text_attribute;
            }

            if extractable {
                let is_template = template_xrefs.contains(&attr_op.target);

                let binding_kind = if attr_op.name == "class" {
                    if is_template {
                        BindingKind::Attribute
                    } else {
                        BindingKind::ClassName
                    }
                } else if attr_op.name == "style" {
                    BindingKind::StyleProperty
                } else if attr_op.is_structural_template_attribute {
                    BindingKind::Template
                } else {
                    BindingKind::Attribute
                };

                let expression = match &attr_op.expression {
                    BindingExpression::Expression(expr) => Some(expr.clone()),
                    BindingExpression::Interpolation(_) => None,
                };

                if binding_kind == BindingKind::ClassName {
                    if let Some(ref expr) = expression {
                        if let crate::output::output_ast::Expression::Literal(lit) = expr {
                            if let crate::output::output_ast::LiteralValue::String(class_value) =
                                &lit.value
                            {
                                for class_name in class_value.split_whitespace() {
                                    let extracted_attr_op = create_extracted_attribute_op(
                                        attr_op.target,
                                        BindingKind::ClassName,
                                        attr_op.namespace.clone(),
                                        class_name.to_string(),
                                        None,
                                        attr_op.i18n_context,
                                        attr_op.i18n_message.clone(),
                                        attr_op.security_context.clone(),
                                        attr_op.source_span().cloned(),
                                    );
                                    extracted_attributes
                                        .entry(attr_op.target)
                                        .or_default()
                                        .push(extracted_attr_op);
                                }
                                update_ops_to_remove.insert(index);
                                continue;
                            }
                        }
                    }
                }

                let extracted_attr_op = create_extracted_attribute_op(
                    attr_op.target,
                    binding_kind,
                    attr_op.namespace.clone(),
                    attr_op.name.clone(),
                    expression,
                    attr_op.i18n_context,
                    attr_op.i18n_message.clone(),
                    attr_op.security_context.clone(),
                    attr_op.source_span().cloned(),
                );
                extracted_attributes
                    .entry(attr_op.target)
                    .or_default()
                    .push(extracted_attr_op);
                update_ops_to_remove.insert(index);
            }
        } else if let Some(prop_op) = op_ref.as_any().downcast_ref::<PropertyOp>() {
            if prop_op.binding_kind != BindingKind::LegacyAnimation
                && prop_op.binding_kind != BindingKind::Animation
            {
                let binding_kind =
                    if prop_op.i18n_message.is_some() && prop_op.template_kind.is_none() {
                        BindingKind::I18n
                    } else if prop_op.is_structural_template_attribute {
                        BindingKind::Template
                    } else {
                        BindingKind::Property
                    };

                let extracted_attr_op = create_extracted_attribute_op(
                    prop_op.target,
                    binding_kind,
                    None,
                    prop_op.name.clone(),
                    None,
                    prop_op.i18n_context,
                    None,
                    prop_op.security_context.clone(),
                    prop_op.source_span().cloned(),
                );
                extracted_attributes
                    .entry(prop_op.target)
                    .or_default()
                    .push(extracted_attr_op);
            }
        } else if let Some(control_op) = op_ref.as_any().downcast_ref::<ControlOp>() {
            let extracted_attr_op = create_extracted_attribute_op(
                control_op.target,
                BindingKind::Property,
                None,
                "field".to_string(),
                None,
                None,
                None,
                control_op.security_context.clone(),
                control_op.source_span().cloned(),
            );
            extracted_attributes
                .entry(control_op.target)
                .or_default()
                .push(extracted_attr_op);
        } else if let Some(twoway_op) = op_ref.as_any().downcast_ref::<TwoWayPropertyOp>() {
            let extracted_attr_op = create_extracted_attribute_op(
                twoway_op.target,
                BindingKind::TwoWayProperty,
                None,
                twoway_op.name.clone(),
                None,
                None,
                None,
                twoway_op.security_context.clone(),
                twoway_op.source_span().cloned(),
            );
            extracted_attributes
                .entry(twoway_op.target)
                .or_default()
                .push(extracted_attr_op);
        } else if let Some(listener_op) = op_ref.as_any().downcast_ref::<ListenerOp>() {
            if !listener_op.is_legacy_animation_listener {
                let extracted_attr_op = create_extracted_attribute_op(
                    listener_op.target,
                    BindingKind::Property,
                    None,
                    listener_op.name.clone(),
                    None,
                    None,
                    None,
                    vec![SecurityContext::NONE],
                    listener_op.source_span().cloned(),
                );

                if job_kind == CompilationJobKind::Host {
                    if compatibility != ir::CompatibilityMode::TemplateDefinitionBuilder {
                        extracted_attributes
                            .entry(listener_op.target)
                            .or_default()
                            .push(extracted_attr_op);
                    }
                } else {
                    extracted_attributes
                        .entry(listener_op.target)
                        .or_default()
                        .push(extracted_attr_op);
                }
            }
        } else if compatibility == ir::CompatibilityMode::TemplateDefinitionBuilder {
            if let Some(style_op) = op_ref.as_any().downcast_ref::<StylePropOp>() {
                if matches!(&style_op.expression, BindingExpression::Expression(ref e) if matches!(e, crate::output::output_ast::Expression::Empty(_)))
                {
                    let extracted_attr_op = create_extracted_attribute_op(
                        style_op.target,
                        BindingKind::Property,
                        None,
                        style_op.name.clone(),
                        None,
                        None,
                        None,
                        vec![SecurityContext::STYLE],
                        style_op.source_span().cloned(),
                    );
                    extracted_attributes
                        .entry(style_op.target)
                        .or_default()
                        .push(extracted_attr_op);
                }
            } else if let Some(class_op) = op_ref.as_any().downcast_ref::<ClassPropOp>() {
                if matches!(
                    &class_op.expression,
                    crate::output::output_ast::Expression::Empty(_)
                ) {
                    let extracted_attr_op = create_extracted_attribute_op(
                        class_op.target,
                        BindingKind::Property,
                        None,
                        class_op.name.clone(),
                        None,
                        None,
                        None,
                        vec![SecurityContext::STYLE],
                        class_op.source_span().cloned(),
                    );
                    extracted_attributes
                        .entry(class_op.target)
                        .or_default()
                        .push(extracted_attr_op);
                }
            }
        }
    }

    // Now collect from create ops
    for op_ref in create_list.iter() {
        // Extract regular event listeners (e.g., (click)="handler()")
        if let Some(listener_op) = op_ref.as_any().downcast_ref::<ListenerOp>() {
            if !listener_op.is_legacy_animation_listener {
                let extracted_attr_op = create_extracted_attribute_op(
                    listener_op.target,
                    BindingKind::Property,
                    None,
                    listener_op.name.clone(),
                    None,
                    None,
                    None,
                    vec![SecurityContext::NONE],
                    listener_op.source_span().cloned(),
                );
                extracted_attributes
                    .entry(listener_op.target)
                    .or_default()
                    .push(extracted_attr_op);
            }
        }
        // Extract two-way listener bindings
        if let Some(twoway_listener) = op_ref.as_any().downcast_ref::<TwoWayListenerOp>() {
            let extracted_attr_op = create_extracted_attribute_op(
                twoway_listener.target,
                BindingKind::Property,
                None,
                twoway_listener.name.clone(),
                None,
                None,
                None,
                vec![SecurityContext::NONE],
                twoway_listener.source_span().cloned(),
            );
            extracted_attributes
                .entry(twoway_listener.target)
                .or_default()
                .push(extracted_attr_op);
        }
    }

    // Remove processed update ops
    for index in (0..update_list.len()).rev() {
        if update_ops_to_remove.contains(&index) {
            update_list.remove_at(index);
        }
    }

    // Put back update_list
    unit.update = update_list;

    // Now insert extracted attributes into create list
    let mut final_create = ir::OpList::new();
    for op in create_list {
        let xref_opt = get_xref_from_create_op(op.as_ref());
        final_create.push(op);

        if let Some(xref) = xref_opt {
            if let Some(attrs) = extracted_attributes.remove(&xref) {
                for attr in attrs {
                    final_create.push(attr);
                }
            }
        }
    }

    // Any remaining attributes (e.g. for host bindings or if xrefs weren't found)
    if job_kind == CompilationJobKind::Host {
        for (_, attrs) in extracted_attributes {
            for attr in attrs {
                final_create.push(attr);
            }
        }
    }

    unit.create = final_create;
}

// Helper to get xref from op
fn get_xref_from_create_op(op: &dyn ir::CreateOp) -> Option<ir::XrefId> {
    match op.kind() {
        OpKind::ElementStart => op
            .as_any()
            .downcast_ref::<ir::ops::create::ElementStartOp>()
            .map(|el| el.base.base.xref),
        OpKind::Element => op
            .as_any()
            .downcast_ref::<ir::ops::create::ElementOp>()
            .map(|el| el.base.base.xref),
        OpKind::Template => op
            .as_any()
            .downcast_ref::<ir::ops::create::TemplateOp>()
            .map(|tmpl| tmpl.base.base.xref),
        OpKind::ContainerStart => op
            .as_any()
            .downcast_ref::<ir::ops::create::ContainerStartOp>()
            .map(|c| c.base.xref),
        OpKind::Container => op
            .as_any()
            .downcast_ref::<ir::ops::create::ContainerOp>()
            .map(|c| c.base.xref),
        OpKind::RepeaterCreate => op
            .as_any()
            .downcast_ref::<ir::ops::create::RepeaterCreateOp>()
            .map(|r| r.base.base.xref),
        OpKind::ConditionalCreate => op
            .as_any()
            .downcast_ref::<ir::ops::create::ConditionalCreateOp>()
            .map(|c| c.base.base.xref),
        OpKind::ConditionalBranchCreate => op
            .as_any()
            .downcast_ref::<ir::ops::create::ConditionalBranchCreateOp>()
            .map(|c| c.base.base.xref),
        OpKind::Projection => op
            .as_any()
            .downcast_ref::<ir::ops::create::ProjectionOp>()
            .map(|p| p.xref),
        _ => None,
    }
}
