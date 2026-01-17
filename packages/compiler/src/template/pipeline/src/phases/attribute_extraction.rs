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
    CompilationJob, CompilationJobKind, CompilationUnit, ComponentCompilationJob,
};
use std::collections::{HashMap, HashSet};

/// Find all extractable attribute and binding ops, and create ExtractedAttributeOps for them.
/// In cases where no instruction needs to be generated for the attribute or binding, it is removed.
pub fn extract_attributes(job: &mut dyn CompilationJob) {
    let compatibility = job.compatibility();
    let kind = job.kind();

    // First pass: Build cross-unit mapping of conditional template XrefId -> first child element XrefId
    // This needs to happen across all units since ConditionalCreate is in root view
    // but first child elements are in embedded views
    let mut conditional_to_first_child: HashMap<ir::XrefId, ir::XrefId> = HashMap::new();

    // Collect all conditional template XrefIds from all units
    let mut conditional_xrefs: HashSet<ir::XrefId> = HashSet::new();
    for unit in job.units() {
        for op in unit.create().iter() {
            if op.kind() == OpKind::ConditionalCreate
                || op.kind() == OpKind::ConditionalBranchCreate
            {
                if let Some(xref) = get_xref_from_create_op(op.as_ref()) {
                    conditional_xrefs.insert(xref);
                }
            }
        }
    }

    // For each conditional, find its first child element (which is in the embedded view with same xref)
    // The conditional's xref matches the embedded view's xref, and the first element in that view is the first child
    for unit in job.units() {
        let unit_xref = unit.xref();
        if conditional_xrefs.contains(&unit_xref) {
            // This unit is the embedded view for a conditional template
            // Find the first element op
            for op in unit.create().iter() {
                if op.kind() == OpKind::ElementStart || op.kind() == OpKind::Element {
                    if let Some(child_xref) = get_xref_from_create_op(op.as_ref()) {
                        conditional_to_first_child.insert(unit_xref, child_xref);
                        break; // Only first element
                    }
                }
            }
        }
    }

    // Build reverse map
    let first_child_to_conditional: HashMap<ir::XrefId, ir::XrefId> = conditional_to_first_child
        .iter()
        .map(|(cond, child)| (*child, *cond))
        .collect();

    // Collect conditional template extracted attributes separately
    // These will be inserted into the root unit after processing
    let mut conditional_extracted_attrs: HashMap<
        ir::XrefId,
        Vec<Box<dyn ir::CreateOp + Send + Sync>>,
    > = HashMap::new();

    // Process all units with the cross-unit mapping
    for unit in job.units_mut() {
        let attrs = process_unit(
            unit,
            compatibility,
            kind,
            &first_child_to_conditional,
            &conditional_xrefs,
        );
        // Merge conditional attrs from this unit
        for (xref, attr_list) in attrs {
            conditional_extracted_attrs
                .entry(xref)
                .or_default()
                .extend(attr_list);
        }
    }

    // Now insert conditional template extracted attrs into the root unit
    // We need to find the ConditionalCreate ops and insert attrs after them
    if !conditional_extracted_attrs.is_empty() {
        let root = job.root_mut();
        let create_list = std::mem::replace(root.create_mut(), ir::OpList::new());
        let mut final_create = ir::OpList::new();

        for op in create_list {
            let xref_opt = get_xref_from_create_op(op.as_ref());
            let op_kind = op.kind();
            final_create.push(op);

            // If this is a conditional op and we have extracted attrs for it, insert them
            if let Some(xref) = xref_opt {
                if op_kind == OpKind::ConditionalCreate
                    || op_kind == OpKind::ConditionalBranchCreate
                {
                    if let Some(attrs) = conditional_extracted_attrs.remove(&xref) {
                        for attr in attrs {
                            final_create.push(attr);
                        }
                    }
                }
            }
        }

        *root.create_mut() = final_create;
    }
}

fn process_unit(
    unit: &mut dyn CompilationUnit,
    compatibility: ir::CompatibilityMode,
    job_kind: CompilationJobKind,
    first_child_to_conditional: &HashMap<ir::XrefId, ir::XrefId>,
    conditional_xrefs: &HashSet<ir::XrefId>,
) -> HashMap<ir::XrefId, Vec<Box<dyn ir::CreateOp + Send + Sync>>> {
    // Map to collect extracted attributes per target element
    let mut extracted_attributes: HashMap<ir::XrefId, Vec<Box<dyn ir::CreateOp + Send + Sync>>> =
        HashMap::new();

    // Map to collect extracted attributes for conditional templates (to be returned)
    let mut conditional_attrs: HashMap<ir::XrefId, Vec<Box<dyn ir::CreateOp + Send + Sync>>> =
        HashMap::new();

    // Temporarily take update list to avoid borrow conflicts
    let mut update_list = std::mem::replace(unit.update_mut(), ir::OpList::new());
    let mut update_ops_to_remove = HashSet::new();

    // Identify which XrefIds belong to templates and collect tag names
    let mut template_xrefs = HashSet::new();
    let mut xref_to_tag = HashMap::new();
    let create_list = std::mem::replace(unit.create_mut(), ir::OpList::new());

    // First pass: collect template xrefs and tag names
    for op in create_list.iter() {
        if op.kind() == OpKind::Template {
            if let Some(templ_op) = op.as_any().downcast_ref::<ir::ops::create::TemplateOp>() {
                template_xrefs.insert(templ_op.base.base.xref);
                if let Some(tag) = &templ_op.base.tag {
                    xref_to_tag.insert(templ_op.base.base.xref, tag.clone());
                }
            }
        } else if op.kind() == OpKind::Element {
            if let Some(el_op) = op.as_any().downcast_ref::<ir::ops::create::ElementOp>() {
                if let Some(tag) = &el_op.base.tag {
                    xref_to_tag.insert(el_op.base.base.xref, tag.clone());
                }
            }
        } else if op.kind() == OpKind::ElementStart {
            if let Some(el_op) = op
                .as_any()
                .downcast_ref::<ir::ops::create::ElementStartOp>()
            {
                if let Some(tag) = &el_op.base.tag {
                    xref_to_tag.insert(el_op.base.base.xref, tag.clone());
                }
            }
        }
    }

    // Process create ops FIRST to extract listeners before properties
    // NGTSC outputs listeners before properties in the consts bindings section
    for op_ref in create_list.iter() {
        if job_kind == CompilationJobKind::Host {
            continue;
        }

        // Extract regular event listeners (e.g., (click)="handler()")
        if let Some(listener_op) = op_ref.as_any().downcast_ref::<ListenerOp>() {
            if !listener_op.is_legacy_animation_listener {
                let extracted_attr_op = create_extracted_attribute_op(
                    listener_op.element,
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
                    .entry(listener_op.element)
                    .or_default()
                    .push(extracted_attr_op);
            }
        }
        // Extract two-way listener bindings
        if let Some(twoway_listener) = op_ref.as_any().downcast_ref::<TwoWayListenerOp>() {
            let extracted_attr_op = create_extracted_attribute_op(
                twoway_listener.element,
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
                .entry(twoway_listener.element)
                .or_default()
                .push(extracted_attr_op);
        }
    }

    // Process update ops - extract properties AFTER listeners
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

                let binding_kind = if attr_op.name.as_ref() == "class" {
                    if is_template {
                        BindingKind::Attribute
                    } else {
                        BindingKind::ClassName
                    }
                } else if attr_op.name.as_ref() == "style" {
                    BindingKind::StyleProperty
                } else if attr_op.is_structural_template_attribute {
                    // Check if target is ng-template, if so force Property kind
                    if xref_to_tag.get(&attr_op.target).map(|s| s.as_str()) == Some("ng-template") {
                        BindingKind::Property
                    } else {
                        BindingKind::Template
                    }
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
                                        class_name.into(),
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

                                    // Also propagate class to parent conditional template
                                    if let Some(cond_xref) =
                                        first_child_to_conditional.get(&attr_op.target)
                                    {
                                        let cond_extracted = create_extracted_attribute_op(
                                            *cond_xref,
                                            BindingKind::ClassName,
                                            attr_op.namespace.clone(),
                                            class_name.into(),
                                            None,
                                            attr_op.i18n_context,
                                            attr_op.i18n_message.clone(),
                                            attr_op.security_context.clone(),
                                            attr_op.source_span().cloned(),
                                        );
                                        // Use conditional_attrs for insertion into root unit
                                        conditional_attrs
                                            .entry(*cond_xref)
                                            .or_default()
                                            .push(cond_extracted);
                                    }
                                }
                                update_ops_to_remove.insert(index);
                                continue;
                            }
                        }
                    }
                } else if binding_kind == BindingKind::StyleProperty {
                    if let Some(ref expr) = expression {
                        if let crate::output::output_ast::Expression::Literal(lit) = expr {
                            if let crate::output::output_ast::LiteralValue::String(style_value) =
                                &lit.value
                            {
                                // Parse style string: "key: value; key2: value2"
                                for style_decl in style_value.split(';') {
                                    let style_decl = style_decl.trim();
                                    if style_decl.is_empty() {
                                        continue;
                                    }

                                    if let Some((name, value)) = style_decl.split_once(':') {
                                        let style_name = name.trim();
                                        let style_val = value.trim();

                                        if !style_name.is_empty() {
                                            let extracted_attr_op = create_extracted_attribute_op(
                                                attr_op.target,
                                                BindingKind::StyleProperty,
                                                attr_op.namespace.clone(),
                                                style_name.into(),
                                                Some(crate::output::output_ast::Expression::Literal(
                                                    crate::output::output_ast::LiteralExpr {
                                                        value: crate::output::output_ast::LiteralValue::String(style_val.to_string()),
                                                        type_: None,
                                                        source_span: None,
                                                    }
                                                )),
                                                attr_op.i18n_context,
                                                attr_op.i18n_message.clone(),
                                                attr_op.security_context.clone(),
                                                attr_op.source_span().cloned(),
                                            );

                                            extracted_attributes
                                                .entry(attr_op.target)
                                                .or_default()
                                                .push(extracted_attr_op);

                                            // Also propagate style to parent conditional template
                                            if let Some(cond_xref) =
                                                first_child_to_conditional.get(&attr_op.target)
                                            {
                                                let cond_extracted = create_extracted_attribute_op(
                                                    *cond_xref,
                                                    BindingKind::StyleProperty,
                                                    attr_op.namespace.clone(),
                                                    style_name.into(),
                                                    Some(crate::output::output_ast::Expression::Literal(
                                                        crate::output::output_ast::LiteralExpr {
                                                            value: crate::output::output_ast::LiteralValue::String(style_val.to_string()),
                                                            type_: None,
                                                            source_span: None,
                                                        }
                                                    )),
                                                    attr_op.i18n_context,
                                                    attr_op.i18n_message.clone(),
                                                    attr_op.security_context.clone(),
                                                    attr_op.source_span().cloned(),
                                                );
                                                // Use conditional_attrs for insertion into root unit
                                                conditional_attrs
                                                    .entry(*cond_xref)
                                                    .or_default()
                                                    .push(cond_extracted);
                                            }
                                        }
                                    }
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
            let binding_kind = if prop_op.i18n_message.is_some() && prop_op.template_kind.is_none()
            {
                BindingKind::I18n
            } else if prop_op.is_structural_template_attribute {
                // Check if target is ng-template, if so force Property kind
                let tag = xref_to_tag.get(&prop_op.target).map(|s| s.as_str());
                eprintln!("[ATTR_EXTRACT] Checking Structural PropertyOp for target {:?}, resolved tag: {:?}", prop_op.target, tag);

                if tag == Some("ng-template") {
                    eprintln!("[ATTR_EXTRACT] Decision: Property (tag is ng-template)");
                    BindingKind::Property
                } else {
                    eprintln!("[ATTR_EXTRACT] Decision: Template (tag is {:?})", tag);
                    BindingKind::Template
                }
            } else {
                BindingKind::Property
            };

            if (job_kind != CompilationJobKind::Host || binding_kind == BindingKind::I18n)
                && prop_op.binding_kind != BindingKind::LegacyAnimation
                && prop_op.binding_kind != BindingKind::Animation
            {
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

                // Also propagate binding to parent conditional template if this is a first child element
                if let Some(cond_xref) = first_child_to_conditional.get(&prop_op.target) {
                    let cond_extracted = create_extracted_attribute_op(
                        *cond_xref,
                        binding_kind,
                        None,
                        prop_op.name.clone(),
                        None,
                        prop_op.i18n_context,
                        None,
                        prop_op.security_context.clone(),
                        prop_op.source_span().cloned(),
                    );
                    // Use conditional_attrs since these will be inserted into root unit later
                    conditional_attrs
                        .entry(*cond_xref)
                        .or_default()
                        .push(cond_extracted);
                }
            }
        } else if let Some(control_op) = op_ref.as_any().downcast_ref::<ControlOp>() {
            if job_kind != CompilationJobKind::Host {
                let extracted_attr_op = create_extracted_attribute_op(
                    control_op.target,
                    BindingKind::Property,
                    None,
                    "field".into(),
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
            }
        } else if let Some(twoway_op) = op_ref.as_any().downcast_ref::<TwoWayPropertyOp>() {
            if job_kind != CompilationJobKind::Host {
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

        // Always extract StylePropOp bindings to parent conditional templates
        if let Some(style_op) = op_ref.as_any().downcast_ref::<StylePropOp>() {
            if let Some(cond_xref) = first_child_to_conditional.get(&style_op.target) {
                let cond_extracted = create_extracted_attribute_op(
                    *cond_xref,
                    BindingKind::Property,
                    None,
                    style_op.name.clone(),
                    None,
                    None,
                    None,
                    vec![SecurityContext::STYLE],
                    style_op.source_span().cloned(),
                );
                conditional_attrs
                    .entry(*cond_xref)
                    .or_default()
                    .push(cond_extracted);
            }
        }

        // Always extract ClassPropOp bindings to parent conditional templates
        if let Some(class_op) = op_ref.as_any().downcast_ref::<ClassPropOp>() {
            if let Some(cond_xref) = first_child_to_conditional.get(&class_op.target) {
                let cond_extracted = create_extracted_attribute_op(
                    *cond_xref,
                    BindingKind::Property,
                    None,
                    class_op.name.clone(),
                    None,
                    None,
                    None,
                    vec![SecurityContext::NONE],
                    class_op.source_span().cloned(),
                );
                conditional_attrs
                    .entry(*cond_xref)
                    .or_default()
                    .push(cond_extracted);
            }
        }
    }

    // Remove processed update ops
    for index in (0..update_list.len()).rev() {
        if update_ops_to_remove.contains(&index) {
            update_list.remove_at(index);
        }
    }

    // Put back update_list
    *unit.update_mut() = update_list;

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

    *unit.create_mut() = final_create;

    // Return conditional template extracted attributes for insertion into root unit
    conditional_attrs
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
