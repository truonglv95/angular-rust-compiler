//! Const Collection Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/const_collection.ts
//! Converts the semantic attributes of element-like operations into constant array expressions

use crate::core::AttributeMarker;
use crate::output::output_ast::{Expression, LiteralArrayExpr, LiteralExpr, LiteralValue, TaggedTemplateLiteralExpr, TemplateLiteralElement, TemplateLiteral};
use crate::template::pipeline::ir as ir;
use crate::template::pipeline::ir::enums::{OpKind, BindingKind};
use crate::template::pipeline::ir::ops::create::{ExtractedAttributeOp, ElementOrContainerOpBase, ProjectionOp, RepeaterCreateOp};
use crate::template::pipeline::src::compilation::{CompilationJob, ComponentCompilationJob, HostBindingCompilationJob, CompilationUnit, CompilationJobKind};
use std::collections::HashMap;

/// Container for all of the various kinds of attributes which are applied on an element.
#[derive(Clone)]
struct ElementAttributes {
    attributes: Vec<Expression>,
    bindings: Vec<Expression>,
    classes: Vec<Expression>,
    styles: Vec<Expression>,
    template: Vec<Expression>,
    i18n: Vec<Expression>,
    project_as: Option<String>,
    known: HashMap<(BindingKind, String), bool>,
    compatibility: ir::CompatibilityMode,
}

impl ElementAttributes {
    fn new(compatibility: ir::CompatibilityMode) -> Self {
        ElementAttributes {
            attributes: Vec::new(),
            bindings: Vec::new(),
            classes: Vec::new(),
            styles: Vec::new(),
            template: Vec::new(),
            i18n: Vec::new(),
            project_as: None,
            known: HashMap::new(),
            compatibility,
        }
    }

    fn is_known(&mut self, kind: BindingKind, name: &str) -> bool {
        let key = (kind, name.to_string());
        if self.known.contains_key(&key) {
            return true;
        }
        self.known.insert(key.clone(), true);
        false
    }

    fn add(
        &mut self,
        kind: BindingKind,
        name: String,
        value: Option<Expression>,
        namespace: Option<String>,
        trusted_value_fn: Option<Expression>,
    ) {
        // Check for duplicates (except in compatibility mode for some binding kinds)
        let allow_duplicates = self.compatibility == ir::CompatibilityMode::TemplateDefinitionBuilder
            && matches!(kind, BindingKind::Attribute | BindingKind::ClassName | BindingKind::StyleProperty);
        
        if !allow_duplicates && self.is_known(kind, &name) {
            return;
        }

        // Handle ngProjectAs
        if name == "ngProjectAs" {
            if let Some(Expression::Literal(LiteralExpr { value: LiteralValue::String(s), .. })) = value {
                self.project_as = Some(s);
                return; // ngProjectAs is handled separately
            } else {
                panic!("ngProjectAs must have a string literal value");
            }
        }

        // Get the appropriate array for this binding kind
        let array = match kind {
            BindingKind::Property | BindingKind::TwoWayProperty => &mut self.bindings,
            BindingKind::Attribute => &mut self.attributes,
            BindingKind::ClassName => &mut self.classes,
            BindingKind::StyleProperty => &mut self.styles,
            BindingKind::Template => &mut self.template,
            BindingKind::I18n => &mut self.i18n,
            _ => return, // Other kinds not handled here
        };

        // Add namespace and name literals
        if let Some(ns) = namespace {
            array.push(Expression::Literal(LiteralExpr {
                value: LiteralValue::Number(AttributeMarker::NamespaceURI as u8 as f64),
                type_: None,
                source_span: None,
            }));
            array.push(Expression::Literal(LiteralExpr {
                value: LiteralValue::String(ns),
                type_: None,
                source_span: None,
            }));
        }
        array.push(Expression::Literal(LiteralExpr {
            value: LiteralValue::String(name),
            type_: None,
            source_span: None,
        }));

        // Add value for attribute and style properties
        if matches!(kind, BindingKind::Attribute | BindingKind::StyleProperty) {
            if let Some(val) = value {
                if let Some(trusted_fn) = trusted_value_fn {
                    // Use tagged template if trusted value function is provided
                    // Check if value is a string literal
                    if let Expression::Literal(LiteralExpr { value: LiteralValue::String(s), source_span, .. }) = val {
                        let template = TemplateLiteral {
                            elements: vec![TemplateLiteralElement {
                                text: s.clone(),
                                raw_text: s,
                                source_span: source_span.clone(),
                            }],
                            expressions: Vec::new(),
                        };
                        array.push(Expression::TaggedTemplate(TaggedTemplateLiteralExpr {
                            tag: Box::new(trusted_fn),
                            template,
                            type_: None,
                            source_span,
                        }));
                    } else {
                        panic!("AssertionError: extracted attribute value should be string literal");
                    }
                } else {
                    array.push(val);
                }
            } else {
                panic!("Attribute and style element attributes must have a value");
            }
        }
    }
}

/// Converts the semantic attributes of element-like operations into constant array expressions.
pub fn collect_element_consts(job: &mut dyn CompilationJob) {
    // Collect all extracted attributes
    let mut all_element_attributes: HashMap<ir::XrefId, ElementAttributes> = HashMap::new();
    
    // Collect ExtractedAttributeOps from all units
    let component_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let job_ptr = job_ptr as *mut ComponentCompilationJob;
        &mut *job_ptr
    };
    
    let compatibility = job.compatibility();
    
    // Collect ExtractedAttributeOps and remove them
    let mut units_to_process: Vec<*mut crate::template::pipeline::src::compilation::ViewCompilationUnit> = Vec::new();
    units_to_process.push(&mut component_job.root as *mut _);
    for (_, unit) in component_job.views.iter_mut() {
        units_to_process.push(unit as *mut _);
    }
    
    // First pass: collect ExtractedAttributeOps
    for unit_ptr in &units_to_process {
        let unit = unsafe { &mut **unit_ptr };
        let mut indices_to_remove: Vec<usize> = Vec::new();
        
        for (index, op) in unit.create().iter().enumerate() {
            if op.kind() == OpKind::ExtractedAttribute {
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let extracted_attr_ptr = op_ptr as *const ExtractedAttributeOp;
                    let extracted_attr = &*extracted_attr_ptr;
                    
                    let attributes = all_element_attributes.entry(extracted_attr.target)
                        .or_insert_with(|| ElementAttributes::new(compatibility));
                    
                    attributes.add(
                        extracted_attr.binding_kind,
                        extracted_attr.name.clone(),
                        extracted_attr.expression.clone(),
                        extracted_attr.namespace.clone(),
                        extracted_attr.trusted_value_fn.clone(),
                    );
                    
                    indices_to_remove.push(index);
                }
            }
        }
        
        // Remove ExtractedAttributeOps (iterate in reverse)
        for index in indices_to_remove.iter().rev() {
            unit.create_mut().remove_at(*index);
        }
    }
    
    // Serialize the extracted attributes into the const array
    // Handle ComponentCompilationJob
    let job_kind = job.kind();
    if matches!(job_kind, CompilationJobKind::Tmpl | CompilationJobKind::Both) {
        let component_job = unsafe {
            let job_ptr = job as *mut dyn CompilationJob;
            let job_ptr = job_ptr as *mut ComponentCompilationJob;
            &mut *job_ptr
        };
        // Process all units to assign attributes
        process_component_job(component_job, &all_element_attributes);
    } else {
        // Handle HostBindingCompilationJob
        let host_job = unsafe {
            let job_ptr = job as *mut dyn CompilationJob;
            let job_ptr = job_ptr as *mut HostBindingCompilationJob;
            &mut *job_ptr
        };
        
        for (xref, attributes) in all_element_attributes.iter() {
            if *xref != host_job.root.xref {
                panic!("An attribute would be const collected into the host binding's template function, but is not associated with the root xref.");
            }
            let attr_array = serialize_attributes(attributes.clone());
            if !attr_array.entries.is_empty() {
                host_job.root.attributes = Some(Expression::LiteralArray(attr_array));
            }
        }
    }
}

/// Helper function to extract xref from different create op types
unsafe fn get_xref_from_create_op(op_ptr: *const dyn ir::CreateOp, kind: OpKind) -> Option<ir::XrefId> {
    match kind {
        OpKind::ElementStart => {
            let el_op = &*(op_ptr as *const ir::ops::create::ElementStartOp);
            Some(el_op.base.base.xref)
        }
        OpKind::Element => {
            let el_op = &*(op_ptr as *const ir::ops::create::ElementOp);
            Some(el_op.base.base.xref)
        }
        OpKind::Template => {
            let tmpl_op = &*(op_ptr as *const ir::ops::create::TemplateOp);
            Some(tmpl_op.base.base.xref)
        }
        OpKind::ContainerStart => {
            let c_op = &*(op_ptr as *const ir::ops::create::ContainerStartOp);
            Some(c_op.base.xref)
        }
        OpKind::Container => {
            let c_op = &*(op_ptr as *const ir::ops::create::ContainerOp);
            Some(c_op.base.xref)
        }
        OpKind::RepeaterCreate => {
            let r_op = &*(op_ptr as *const ir::ops::create::RepeaterCreateOp);
            Some(r_op.base.base.xref)
        }
        OpKind::ConditionalCreate => {
            let c_op = &*(op_ptr as *const ir::ops::create::ConditionalCreateOp);
            Some(c_op.base.base.xref)
        }
        OpKind::ConditionalBranchCreate => {
            let c_op = &*(op_ptr as *const ir::ops::create::ConditionalBranchCreateOp);
            Some(c_op.base.base.xref)
        }
        OpKind::Projection => {
            let p_op = &*(op_ptr as *const ir::ops::create::ProjectionOp);
            Some(p_op.xref)
        }
        _ => None,
    }
}

fn process_component_job(
    job: &mut ComponentCompilationJob,
    all_element_attributes: &HashMap<ir::XrefId, ElementAttributes>,
) {
    // Pass 2: Collect all elements that need const indices in sequential order
    let mut ordered_elements = Vec::new();

    // Helper to collect elements from a view
    let mut collect_elements = |create_list: &ir::OpList<Box<dyn ir::CreateOp + Send + Sync>>| {
        for op in create_list.iter() {
            let kind = op.kind();
            if kind == ir::OpKind::ElementStart || kind == ir::OpKind::Element || 
               kind == ir::OpKind::Template || kind == ir::OpKind::RepeaterCreate ||
               kind == ir::OpKind::ConditionalCreate || kind == ir::OpKind::Projection {
                
                let xref_opt = unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    get_xref_from_create_op(op_ptr, kind)
                };
                if let Some(xref) = xref_opt {
                    ordered_elements.push((xref, kind));
                }
            }
        }
    };

    // Process Root View
    collect_elements(&job.root.create);

    // Process embedded views in Xref order
    let mut view_ids: Vec<_> = job.views.keys().cloned().collect();
    view_ids.sort();
    for view_id in view_ids {
        if let Some(view) = job.views.get(&view_id) {
            collect_elements(&view.create);
        }
    }

    // Pass 3: Assign const indices in the collected order
    let mut const_indices = HashMap::new();
    let mut projection_attributes = HashMap::new();

    for (xref, kind) in ordered_elements {
        if !const_indices.contains_key(&xref) {
            if let Some(attributes) = all_element_attributes.get(&xref) {
                let attr_array = serialize_attributes(attributes.clone());
                if kind == ir::OpKind::Projection {
                    projection_attributes.insert(xref, attr_array);
                } else {
                    if !attr_array.entries.is_empty() {
                        let const_idx = job.add_const(Expression::LiteralArray(attr_array), None);
                        const_indices.insert(xref, Some(const_idx));
                    } else {
                        const_indices.insert(xref, None);
                    }
                }
            } else {
                const_indices.insert(xref, None);
            }
        }
    }
    
    // Final pass: assign const indices to elements
    process_unit_for_component(&mut job.root, &const_indices, &projection_attributes);
    let view_keys: Vec<_> = job.views.keys().cloned().collect();
    for key in view_keys {
        if let Some(unit) = job.views.get_mut(&key) {
            process_unit_for_component(unit, &const_indices, &projection_attributes);
        }
    }
}





fn process_unit_for_component(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    const_indices: &HashMap<ir::XrefId, Option<ir::ConstIndex>>,
    projection_attributes: &HashMap<ir::XrefId, LiteralArrayExpr>,
) {
    for op in unit.create_mut().iter_mut() {
        // Handle Projection ops
        if op.kind() == OpKind::Projection {
            unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let projection_ptr = op_ptr as *mut ProjectionOp;
                let projection = &mut *projection_ptr;
                
                if let Some(attr_array) = projection_attributes.get(&projection.xref) {
                    projection.attributes = Some(Box::new(Expression::LiteralArray(attr_array.clone())));
                }
            }
        } else if is_element_or_container_op(op.kind()) {
            // Handle ElementOrContainerOps - must cast to specific types because of nested struct hierarchy
            match op.kind() {
                OpKind::ElementStart => {
                    if let Some(el_op) = op.as_any_mut().downcast_mut::<ir::ops::create::ElementStartOp>() {
                        if let Some(idx) = const_indices.get(&el_op.base.base.xref).copied().flatten() {
                            el_op.base.base.attributes = Some(idx);
                        }
                    }
                }
                OpKind::Element => {
                    if let Some(el_op) = op.as_any_mut().downcast_mut::<ir::ops::create::ElementOp>() {
                        if let Some(idx) = const_indices.get(&el_op.base.base.xref).copied().flatten() {
                            el_op.base.base.attributes = Some(idx);
                        }
                    }
                }
                OpKind::Template => {
                    if let Some(tmpl_op) = op.as_any_mut().downcast_mut::<ir::ops::create::TemplateOp>() {
                        if let Some(idx) = const_indices.get(&tmpl_op.base.base.xref).copied().flatten() {
                            tmpl_op.base.base.attributes = Some(idx);
                        }
                    }
                }
                OpKind::ContainerStart => {
                    if let Some(c_op) = op.as_any_mut().downcast_mut::<ir::ops::create::ContainerStartOp>() {
                        if let Some(idx) = const_indices.get(&c_op.base.xref).copied().flatten() {
                            c_op.base.attributes = Some(idx);
                        }
                    }
                }
                OpKind::Container => {
                    if let Some(c_op) = op.as_any_mut().downcast_mut::<ir::ops::create::ContainerOp>() {
                        if let Some(idx) = const_indices.get(&c_op.base.xref).copied().flatten() {
                            c_op.base.attributes = Some(idx);
                        }
                    }
                }
                OpKind::RepeaterCreate => {
                    if let Some(r_op) = op.as_any_mut().downcast_mut::<ir::ops::create::RepeaterCreateOp>() {
                        if let Some(idx) = const_indices.get(&r_op.base.base.xref).copied().flatten() {
                            r_op.base.base.attributes = Some(idx);
                        }
                        if let Some(empty_view) = r_op.empty_view {
                            r_op.empty_attributes = const_indices.get(&empty_view).copied().flatten();
                        }
                    }
                }
                OpKind::ConditionalCreate => {
                    if let Some(c_op) = op.as_any_mut().downcast_mut::<ir::ops::create::ConditionalCreateOp>() {
                        if let Some(idx) = const_indices.get(&c_op.base.base.xref).copied().flatten() {
                            c_op.base.base.attributes = Some(idx);
                        }
                    }
                }
                OpKind::ConditionalBranchCreate => {
                    if let Some(c_op) = op.as_any_mut().downcast_mut::<ir::ops::create::ConditionalBranchCreateOp>() {
                        if let Some(idx) = const_indices.get(&c_op.base.base.xref).copied().flatten() {
                            c_op.base.base.attributes = Some(idx);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Check if an op kind is an ElementOrContainerOp
fn is_element_or_container_op(kind: OpKind) -> bool {
    matches!(
        kind,
        OpKind::Element
            | OpKind::ElementStart
            | OpKind::Container
            | OpKind::ContainerStart
            | OpKind::Template
            | OpKind::RepeaterCreate
            | OpKind::ConditionalCreate
            | OpKind::ConditionalBranchCreate
    )
}


/// Serialize ElementAttributes into a LiteralArrayExpr
fn serialize_attributes(attrs: ElementAttributes) -> LiteralArrayExpr {
    let mut attr_array = attrs.attributes;

    // Add projectAs if present
    if let Some(project_as) = attrs.project_as {
        // Parse selector to R3 selector
        // TODO: Implement parse_selector_to_r3_selector properly
        // For now, use a simplified approach - parse selector string to R3 format
        // In TypeScript: parseSelectorToR3Selector returns R3CssSelectorList
        // We'll create a placeholder literal array for now
        attr_array.push(Expression::Literal(LiteralExpr {
            value: LiteralValue::Number(AttributeMarker::ProjectAs as u8 as f64),
            type_: None,
            source_span: None,
        }));
        // Parse selector - for now use a simplified representation
        // The selector is a string that needs to be parsed into an R3 selector
        // R3 selector is an array of arrays: [[element, attr1, attr2, ...], ...]
        // For simple selectors like "div", it becomes [["", "div", ""]]
        // For now, we'll create a basic structure
        let selector_parts: Vec<&str> = project_as.split(',').map(|s| s.trim()).collect();
        let mut selector_array = Vec::new();
        for part in selector_parts {
            // Simple parsing: if it starts with a dot, it's a class; if it starts with #, it's an id
            // Otherwise, it's an element name
            let mut selector_item = vec![
                Expression::Literal(LiteralExpr {
                    value: LiteralValue::String("".to_string()),
                    type_: None,
                    source_span: None,
                })
            ];
            
            if part.starts_with('.') {
                // Class selector
                selector_item.push(Expression::Literal(LiteralExpr {
                    value: LiteralValue::String("".to_string()),
                    type_: None,
                    source_span: None,
                }));
                selector_item.push(Expression::Literal(LiteralExpr {
                    value: LiteralValue::String(part[1..].to_string()),
                    type_: None,
                    source_span: None,
                }));
            } else if part.starts_with('#') {
                // ID selector
                selector_item.push(Expression::Literal(LiteralExpr {
                    value: LiteralValue::String(part[1..].to_string()),
                    type_: None,
                    source_span: None,
                }));
                selector_item.push(Expression::Literal(LiteralExpr {
                    value: LiteralValue::String("".to_string()),
                    type_: None,
                    source_span: None,
                }));
            } else {
                // Element selector
                selector_item.push(Expression::Literal(LiteralExpr {
                    value: LiteralValue::String(part.to_string()),
                    type_: None,
                    source_span: None,
                }));
                selector_item.push(Expression::Literal(LiteralExpr {
                    value: LiteralValue::String("".to_string()),
                    type_: None,
                    source_span: None,
                }));
            }
            
            selector_array.push(Expression::LiteralArray(LiteralArrayExpr {
                entries: selector_item,
                type_: None,
                source_span: None,
            }));
        }
        
        attr_array.push(Expression::LiteralArray(LiteralArrayExpr {
            entries: selector_array,
            type_: None,
            source_span: None,
        }));
    }

    // Add classes marker and classes
    if !attrs.classes.is_empty() {
        attr_array.push(Expression::Literal(LiteralExpr {
            value: LiteralValue::Number(AttributeMarker::Classes as u8 as f64),
            type_: None,
            source_span: None,
        }));
        attr_array.extend(attrs.classes);
    }

    // Add styles marker and styles
    if !attrs.styles.is_empty() {
        attr_array.push(Expression::Literal(LiteralExpr {
            value: LiteralValue::Number(AttributeMarker::Styles as u8 as f64),
            type_: None,
            source_span: None,
        }));
        attr_array.extend(attrs.styles);
    }

    // Add bindings marker and bindings
    if !attrs.bindings.is_empty() {
        attr_array.push(Expression::Literal(LiteralExpr {
            value: LiteralValue::Number(AttributeMarker::Bindings as u8 as f64),
            type_: None,
            source_span: None,
        }));
        attr_array.extend(attrs.bindings);
    }

    // Add template marker and template
    if !attrs.template.is_empty() {
        attr_array.push(Expression::Literal(LiteralExpr {
            value: LiteralValue::Number(AttributeMarker::Template as u8 as f64),
            type_: None,
            source_span: None,
        }));
        attr_array.extend(attrs.template);
    }

    // Add i18n marker and i18n
    if !attrs.i18n.is_empty() {
        attr_array.push(Expression::Literal(LiteralExpr {
            value: LiteralValue::Number(AttributeMarker::I18n as u8 as f64),
            type_: None,
            source_span: None,
        }));
        attr_array.extend(attrs.i18n);
    }

    LiteralArrayExpr {
        entries: attr_array,
        type_: None,
        source_span: None,
    }
}