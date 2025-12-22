//! Attribute Extraction Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/attribute_extraction.ts
//! Finds all extractable attribute and binding ops, and creates ExtractedAttributeOps for them

use crate::core::SecurityContext;
use crate::template::pipeline::ir as ir;
use crate::template::pipeline::ir::enums::{OpKind, BindingKind};
use crate::template::pipeline::ir::ops::create::{create_extracted_attribute_op, TwoWayListenerOp};
use crate::template::pipeline::ir::ops::update::{AttributeOp, PropertyOp, ControlOp, TwoWayPropertyOp, StylePropOp, ClassPropOp, BindingExpression};
use crate::template::pipeline::ir::ops::create::ListenerOp;
use crate::template::pipeline::src::compilation::{CompilationJob, ComponentCompilationJob, CompilationJobKind, CompilationUnit};
use crate::template::pipeline::src::util::elements::create_op_xref_map;
use crate::output::output_ast::ExpressionTrait;

/// Find all extractable attribute and binding ops, and create ExtractedAttributeOps for them.
/// In cases where no instruction needs to be generated for the attribute or binding, it is removed.
pub fn extract_attributes(job: &mut dyn CompilationJob) {
    let component_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let job_ptr = job_ptr as *mut ComponentCompilationJob;
        &mut *job_ptr
    };
    
    // Process root unit
    process_unit(&mut component_job.root, job, CompilationJobKind::Tmpl);
    
    // Process all view units
    for (_, unit) in component_job.views.iter_mut() {
        process_unit(unit, job, CompilationJobKind::Tmpl);
    }
}

fn process_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    job: &dyn CompilationJob,
    job_kind: CompilationJobKind,
) {
    let elements = create_op_xref_map(unit);
    
    // Process create ops for TwoWayListener
    process_create_ops(unit, job_kind, &elements);
    
    // Collect ops to modify - need to iterate and collect indices first
    let mut ops_to_process: Vec<(usize, OpKind)> = Vec::new();
    
    // Collect update ops (immutable borrow)
    {
        let update_list = unit.update();
        for (index, op) in update_list.iter().enumerate() {
            ops_to_process.push((index, op.kind()));
        }
    }
    
    // Process each op - need to handle mutable borrows carefully
    for (index, op_kind) in ops_to_process.iter().rev() {
        match op_kind {
            OpKind::Attribute => {
                // Extract all needed info first (immutable borrow)
                let attr_data = {
                    let update_list = unit.update();
                    let op = update_list.get(*index);
                    op.map(|op_ref| {
                        unsafe {
                            let op_ptr = op_ref.as_ref() as *const dyn ir::UpdateOp;
                            let attr_op_ptr = op_ptr as *const AttributeOp;
                            let attr_op = &*attr_op_ptr;
                            (attr_op.target, attr_op.expression.clone(), attr_op.is_text_attribute,
                             attr_op.is_structural_template_attribute, attr_op.namespace.clone(),
                             attr_op.name.clone(), attr_op.i18n_context, attr_op.i18n_message.clone(),
                             attr_op.security_context.clone())
                        }
                    })
                };
                
                if let Some((target, expr, is_text_attr, is_structural, namespace, name, 
                           i18n_ctx, i18n_msg, sec_ctx)) = attr_data {
                    // Skip if expression is an interpolation
                    if matches!(expr, BindingExpression::Interpolation(_)) {
                        continue;
                    }
                    
                    let is_constant = match &expr {
                        BindingExpression::Expression(expr) => expr.is_constant(),
                        BindingExpression::Interpolation(_) => false,
                    };
                    
                    let mut extractable = is_text_attr || is_constant;
                    
                    if job.compatibility() == ir::CompatibilityMode::TemplateDefinitionBuilder {
                        extractable = extractable && is_text_attr;
                    }
                    
                    if extractable {
                        // Determine binding kind - for structural templates, only directive-specific
                        // attrs (like ngFor, ngForOf) should use Template kind. Static attrs like
                        // class/style should keep their proper kind so they appear before the
                        // AttributeMarker::Template marker in the consts array.
                        let binding_kind = if name == "class" {
                            // Class attribute should use Attribute binding kind to match TSC output
                            // TSC emits ["class", "value"] instead of [1, "value"] for static classes
                            BindingKind::Attribute
                        } else if name == "style" {
                            BindingKind::StyleProperty
                        } else if is_structural {
                            // Only structural directive-specific attrs (not class/style) use Template
                            BindingKind::Template
                        } else {
                            BindingKind::Attribute
                        };
                        
                        let expression = match &expr {
                            BindingExpression::Expression(expr) => Some(expr.clone()),
                            BindingExpression::Interpolation(_) => None,
                        };
                        
                        // For class and style attributes, extract value from expression and use as name
                        if binding_kind == BindingKind::ClassName {
                            // Extract class value from expression
                            if let Some(ref expr) = expression {
                                if let crate::output::output_ast::Expression::Literal(lit) = expr {
                                    if let crate::output::output_ast::LiteralValue::String(class_value) = &lit.value {
                                        // Split multiple classes (e.g., "main content" -> ["main", "content"])
                                        for class_name in class_value.split_whitespace() {
                                            let extracted_attr_op = create_extracted_attribute_op(
                                                target,
                                                BindingKind::ClassName,
                                                namespace.clone(),
                                                class_name.to_string(), // Use class value as name!
                                                None, // No expression needed for class names
                                                i18n_ctx,
                                                i18n_msg.clone(),
                                                sec_ctx.clone(),
                                            );
                                            
                                            if job_kind == CompilationJobKind::Host {
                                                unit.create_mut().push(extracted_attr_op);
                                            } else {
                                                let element_index = elements.get(&target)
                                                    .expect("All attributes should have an element-like target.");
                                                unit.create_mut().insert_at(*element_index, extracted_attr_op);
                                            }
                                        }
                                        // Remove the original op
                                        unit.update_mut().remove_at(*index);
                                        continue;
                                    }
                                }
                            }
                        }
                        
                        let extracted_attr_op = create_extracted_attribute_op(
                            target,
                            binding_kind,
                            namespace,
                            name,
                            expression,
                            i18n_ctx,
                            i18n_msg,
                            sec_ctx,
                        );
                        
                        // Now use mutable borrow
                        if job_kind == CompilationJobKind::Host {
                            unit.create_mut().push(extracted_attr_op);
                        } else {
                            let element_index = elements.get(&target)
                                .expect("All attributes should have an element-like target.");
                            unit.create_mut().insert_at(*element_index, extracted_attr_op);
                        }
                        
                        // Remove the original op
                        unit.update_mut().remove_at(*index);
                    }
                }
                continue; // Skip to next iteration
            }
            OpKind::Property => {
                let op = unit.update().get(*index);
                if op.is_none() {
                    continue;
                }
                let op = op.unwrap();
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                    let prop_op_ptr = op_ptr as *const PropertyOp;
                    let prop_op = &*prop_op_ptr;
                    
                    if prop_op.binding_kind != BindingKind::LegacyAnimation 
                        && prop_op.binding_kind != BindingKind::Animation {
                        let binding_kind = if prop_op.i18n_message.is_some() && prop_op.template_kind.is_none() {
                            BindingKind::I18n
                        } else if prop_op.is_structural_template_attribute {
                            BindingKind::Template
                        } else {
                            BindingKind::Property
                        };
                        
                        let element_index = elements.get(&prop_op.target)
                            .expect("All attributes should have an element-like target.");
                        let extracted_attr_op = create_extracted_attribute_op(
                            prop_op.target,
                            binding_kind,
                            None, // namespace
                            prop_op.name.clone(),
                            None, // expression
                            prop_op.i18n_context,
                            None, // i18n_message - deliberately null
                            prop_op.security_context.clone(),
                        );
                        
                        unit.create_mut().insert_at(*element_index, extracted_attr_op);
                    }
                }
            }
            OpKind::Control => {
                let op = unit.update().get(*index);
                if op.is_none() {
                    continue;
                }
                let op = op.unwrap();
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                    let control_op_ptr = op_ptr as *const ControlOp;
                    let control_op = &*control_op_ptr;
                    
                    let element_index = elements.get(&control_op.target)
                        .expect("All attributes should have an element-like target.");
                    let extracted_attr_op = create_extracted_attribute_op(
                        control_op.target,
                        BindingKind::Property,
                        None, // namespace
                        "field".to_string(),
                        None, // expression
                        None, // i18n_context
                        None, // i18n_message
                        control_op.security_context.clone(),
                    );
                    
                    unit.create_mut().insert_at(*element_index, extracted_attr_op);
                }
            }
            OpKind::TwoWayProperty => {
                let op = unit.update().get(*index);
                if op.is_none() {
                    continue;
                }
                let op = op.unwrap();
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                    let twoway_op_ptr = op_ptr as *const TwoWayPropertyOp;
                    let twoway_op = &*twoway_op_ptr;
                    
                    let element_index = elements.get(&twoway_op.target)
                        .expect("All attributes should have an element-like target.");
                    let extracted_attr_op = create_extracted_attribute_op(
                        twoway_op.target,
                        BindingKind::TwoWayProperty,
                        None, // namespace
                        twoway_op.name.clone(),
                        None, // expression
                        None, // i18n_context
                        None, // i18n_message
                        twoway_op.security_context.clone(),
                    );
                    
                    unit.create_mut().insert_at(*element_index, extracted_attr_op);
                }
            }
            OpKind::StyleProp | OpKind::ClassProp => {
                // Check compatibility mode for empty expressions
                if job.compatibility() == ir::CompatibilityMode::TemplateDefinitionBuilder {
                    let op = unit.update().get(*index);
                    if op.is_none() {
                        continue;
                    }
                    let op = op.unwrap();
                    unsafe {
                        let is_empty = match op_kind {
                            OpKind::StyleProp => {
                                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                                let style_op_ptr = op_ptr as *const StylePropOp;
                                let style_op = &*style_op_ptr;
                                matches!(&style_op.expression, BindingExpression::Expression(ref e) if matches!(e, crate::output::output_ast::Expression::Empty(_)))
                            }
                            OpKind::ClassProp => {
                                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                                let class_op_ptr = op_ptr as *const ClassPropOp;
                                let class_op = &*class_op_ptr;
                                matches!(&class_op.expression, crate::output::output_ast::Expression::Empty(_))
                            }
                            _ => false,
                        };
                        
                        if is_empty {
                            let target = op.xref();
                            let element_index = elements.get(&target)
                                .expect("All attributes should have an element-like target.");
                            
                            let name = match op_kind {
                            OpKind::StyleProp => {
                                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                                let style_op_ptr = op_ptr as *const StylePropOp;
                                let style_op = &*style_op_ptr;
                                style_op.name.clone()
                            }
                            OpKind::ClassProp => {
                                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                                let class_op_ptr = op_ptr as *const ClassPropOp;
                                let class_op = &*class_op_ptr;
                                class_op.name.clone()
                            }
                                _ => unreachable!(),
                            };
                            
                            let extracted_attr_op = create_extracted_attribute_op(
                                target,
                                BindingKind::Property,
                                None, // namespace
                                name,
                                None, // expression
                                None, // i18n_context
                                None, // i18n_message
                                vec![SecurityContext::STYLE],
                            );
                            
                            unit.create_mut().insert_at(*element_index, extracted_attr_op);
                        }
                    }
                }
            }
            OpKind::Listener => {
                let op = unit.update().get(*index);
                if op.is_none() {
                    continue;
                }
                let op = op.unwrap();
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                    let listener_op_ptr = op_ptr as *const ListenerOp;
                    let listener_op = &*listener_op_ptr;
                    
                    if !listener_op.is_legacy_animation_listener {
                        let extracted_attr_op = create_extracted_attribute_op(
                            listener_op.target,
                            BindingKind::Property,
                            None, // namespace
                            listener_op.name.clone(),
                            None, // expression
                            None, // i18n_context
                            None, // i18n_message
                            vec![SecurityContext::NONE],
                        );
                        
                        if job_kind == CompilationJobKind::Host {
                            if job.compatibility() == ir::CompatibilityMode::TemplateDefinitionBuilder {
                                // TemplateDefinitionBuilder does not extract listener bindings to the const array
                                continue;
                            }
                            unit.create_mut().push(extracted_attr_op);
                        } else {
                            let element_index = elements.get(&listener_op.target)
                                .expect("All attributes should have an element-like target.");
                            unit.create_mut().insert_at(*element_index, extracted_attr_op);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Process create ops for TwoWayListener extraction
fn process_create_ops(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    job_kind: CompilationJobKind,
    elements: &std::collections::HashMap<ir::XrefId, usize>,
) {
    // Collect create ops indices for TwoWayListener
    let mut create_indices_to_process: Vec<usize> = Vec::new();
    
    // Collect indices (immutable borrow)
    {
        let create_list = unit.create();
        for (index, op) in create_list.iter().enumerate() {
            if op.kind() == OpKind::TwoWayListener {
                create_indices_to_process.push(index);
            }
        }
    }
    
    // Process in reverse order to maintain indices
    for index in create_indices_to_process.iter().rev() {
        let op = unit.create().get(*index);
        if op.is_none() {
            continue;
        }
        let op = op.unwrap();
        
        unsafe {
            let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
            let twoway_listener_ptr = op_ptr as *const TwoWayListenerOp;
            let twoway_listener = &*twoway_listener_ptr;
            
            // Two-way listeners aren't supported in host bindings
            if job_kind == CompilationJobKind::Host {
                continue;
            }
            
            let extracted_attr_op = create_extracted_attribute_op(
                twoway_listener.target,
                BindingKind::Property,
                None, // namespace
                twoway_listener.name.clone(),
                None, // expression
                None, // i18n_context
                None, // i18n_message
                vec![SecurityContext::NONE],
            );
            
            let element_index = elements.get(&twoway_listener.target)
                .expect("All attributes should have an element-like target.");
            unit.create_mut().insert_at(*element_index, extracted_attr_op);
        }
    }
}

