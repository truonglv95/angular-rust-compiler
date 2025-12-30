//! Parse extracted style and class attributes.
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/parse_extracted_styles.ts
//!
//! Parses extracted style and class attributes into separate ExtractedAttributeOps per style or
//! class property.

use crate::core::SecurityContext;
use crate::output::output_ast::{Expression, LiteralExpr, LiteralValue};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{BindingKind, OpKind, TemplateKind};
use crate::template::pipeline::ir::expression::is_string_literal;
use crate::template::pipeline::ir::ops::create::{
    create_extracted_attribute_op, ConditionalBranchCreateOp, ConditionalCreateOp,
    ExtractedAttributeOp, TemplateOp,
};
use crate::template::pipeline::src::compilation::{CompilationJob, ComponentCompilationJob};

// Any changes here should be ported to the Angular Domino fork.
// https://github.com/angular/domino/blob/main/lib/style_parser.js

#[repr(u8)]
enum Char {
    OpenParen = 40,
    CloseParen = 41,
    Colon = 58,
    Semicolon = 59,
    BackSlash = 92,
    QuoteNone = 0, // indicating we are not inside a quote
    QuoteDouble = 34,
    QuoteSingle = 39,
}

/// Parses string representation of a style and converts it into object literal.
///
/// # Arguments
/// * `value` - string representation of style as used in the `style` attribute in HTML.
///   Example: `color: red; height: auto`.
///
/// # Returns
/// An array of style property name and value pairs, e.g. `['color', 'red', 'height', 'auto']`
pub fn parse_style(value: &str) -> Vec<String> {
    // we use a string array here instead of a string map
    // because a string-map is not guaranteed to retain the
    // order of the entries whereas a string array can be
    // constructed in a [key, value, key, value] format.
    let mut styles = Vec::new();

    let mut i = 0;
    let mut paren_depth = 0;
    let mut quote = Char::QuoteNone;
    let mut value_start = 0;
    let mut prop_start = 0;
    let mut current_prop: Option<String> = None;

    let chars: Vec<char> = value.chars().collect();

    while i < chars.len() {
        let token = chars[i] as u8;
        i += 1;

        match token {
            x if x == Char::OpenParen as u8 => {
                paren_depth += 1;
            }
            x if x == Char::CloseParen as u8 => {
                paren_depth -= 1;
            }
            x if x == Char::QuoteSingle as u8 => {
                // valueStart needs to be there since prop values don't
                // have quotes in CSS
                match quote {
                    Char::QuoteNone => {
                        quote = Char::QuoteSingle;
                    }
                    Char::QuoteSingle => {
                        if i == 1 || chars[i - 2] as u8 != Char::BackSlash as u8 {
                            quote = Char::QuoteNone;
                        }
                    }
                    _ => {}
                }
            }
            x if x == Char::QuoteDouble as u8 => {
                // same logic as above
                match quote {
                    Char::QuoteNone => {
                        quote = Char::QuoteDouble;
                    }
                    Char::QuoteDouble => {
                        if i == 1 || chars[i - 2] as u8 != Char::BackSlash as u8 {
                            quote = Char::QuoteNone;
                        }
                    }
                    _ => {}
                }
            }
            x if x == Char::Colon as u8 => {
                if current_prop.is_none() && paren_depth == 0 && matches!(quote, Char::QuoteNone) {
                    // TODO: Do not hyphenate CSS custom property names like: `--intentionallyCamelCase`
                    let prop = value[prop_start..i - 1].trim().to_string();
                    current_prop = Some(hyphenate(&prop));
                    value_start = i;
                }
            }
            x if x == Char::Semicolon as u8 => {
                if current_prop.is_some()
                    && value_start > 0
                    && paren_depth == 0
                    && matches!(quote, Char::QuoteNone)
                {
                    let style_val = value[value_start..i - 1].trim().to_string();
                    styles.push(current_prop.take().unwrap());
                    styles.push(style_val);
                    prop_start = i;
                    value_start = 0;
                }
            }
            _ => {}
        }
    }

    if let Some(prop) = current_prop {
        if value_start > 0 {
            let style_val = value[value_start..].trim().to_string();
            styles.push(prop);
            styles.push(style_val);
        }
    }

    styles
}

pub fn hyphenate(value: &str) -> String {
    // Convert camelCase to kebab-case
    let mut result = String::with_capacity(value.len() + 10);
    let chars: Vec<char> = value.chars().collect();

    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && ch.is_uppercase() && chars[i - 1].is_lowercase() {
            result.push('-');
        }
        result.push(ch.to_ascii_lowercase());
    }

    result
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

/// Extract string value from a string literal expression
fn extract_string_literal_value(expr: &Expression) -> Option<&str> {
    match expr {
        Expression::Literal(lit) => {
            if let LiteralValue::String(ref s) = lit.value {
                Some(s.as_str())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Check if target element is a structural template
fn is_structural_template(target: &dyn ir::CreateOp) -> bool {
    match target.kind() {
        OpKind::Template => unsafe {
            let op_ptr = target as *const dyn ir::CreateOp;
            let template_ptr = op_ptr as *const TemplateOp;
            let template = &*template_ptr;
            template.template_kind == TemplateKind::Structural
        },
        OpKind::ConditionalCreate => unsafe {
            let op_ptr = target as *const dyn ir::CreateOp;
            let cond_create_ptr = op_ptr as *const ConditionalCreateOp;
            let cond_create = &*cond_create_ptr;
            cond_create.template_kind == TemplateKind::Structural
        },
        OpKind::ConditionalBranchCreate => unsafe {
            let op_ptr = target as *const dyn ir::CreateOp;
            let cond_branch_ptr = op_ptr as *const ConditionalBranchCreateOp;
            let cond_branch = &*cond_branch_ptr;
            cond_branch.template_kind == TemplateKind::Structural
        },
        _ => false,
    }
}

/// Parses extracted style and class attributes into separate ExtractedAttributeOps per style or
/// class property.
pub fn parse_extracted_styles(job: &mut dyn CompilationJob) {
    if job.kind() != crate::template::pipeline::src::compilation::CompilationJobKind::Tmpl {
        return;
    }

    let component_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let component_job_ptr = job_ptr as *mut ComponentCompilationJob;
        &mut *component_job_ptr
    };

    // Build map of elements by xref ID (using indices since we can't easily store trait object refs)
    let mut root_element_indices: std::collections::HashMap<ir::XrefId, usize> =
        std::collections::HashMap::new();

    // Collect element indices from root view
    for (idx, op) in component_job.root.create.iter().enumerate() {
        if is_element_or_container_op(op.kind()) {
            root_element_indices.insert(op.xref(), idx);
        }
    }

    // Build maps for embedded views
    let mut view_element_indices: std::collections::HashMap<
        ir::XrefId,
        std::collections::HashMap<ir::XrefId, usize>,
    > = std::collections::HashMap::new();
    for (view_xref, view) in component_job.views.iter() {
        let mut element_map = std::collections::HashMap::new();
        for (idx, op) in view.create.iter().enumerate() {
            if is_element_or_container_op(op.kind()) {
                element_map.insert(op.xref(), idx);
            }
        }
        view_element_indices.insert(*view_xref, element_map);
    }

    // Process root view
    let mut ops_to_remove = Vec::new();
    let mut ops_to_insert: Vec<(usize, Vec<Box<dyn ir::CreateOp + Send + Sync>>)> = Vec::new();

    for (idx, op) in component_job.root.create.iter().enumerate() {
        if op.kind() == OpKind::ExtractedAttribute {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let extracted_ptr = op_ptr as *const ExtractedAttributeOp;
                let extracted = &*extracted_ptr;

                if extracted.binding_kind == BindingKind::Attribute {
                    if let Some(ref expr) = extracted.expression {
                        if is_string_literal(expr) {
                            // Find target element using map
                            if let Some(&target_idx) = root_element_indices.get(&extracted.target) {
                                let target_op = component_job.root.create.get(target_idx).unwrap();
                                // Skip structural templates
                                if is_structural_template(target_op.as_ref()) {
                                    continue;
                                }

                                if &*extracted.name == "style" {
                                    // Get string literal value using helper
                                    if let Some(style_str) = extract_string_literal_value(expr) {
                                        let parsed_styles = parse_style(style_str);
                                        let mut new_ops = Vec::new();

                                        for i in (0..parsed_styles.len()).step_by(2) {
                                            if i + 1 < parsed_styles.len() {
                                                let prop_name = parsed_styles[i].clone();
                                                let prop_value = parsed_styles[i + 1].clone();

                                                let new_op = create_extracted_attribute_op(
                                                    extracted.target,
                                                    BindingKind::StyleProperty,
                                                    None, // namespace
                                                    prop_name.into(),
                                                    Some(Expression::Literal(LiteralExpr {
                                                        value: LiteralValue::String(prop_value),
                                                        type_: None,
                                                        source_span: None,
                                                    })),
                                                    None, // i18n_context
                                                    None, // i18n_message
                                                    vec![SecurityContext::STYLE],
                                                    None, // source_span
                                                );
                                                new_ops.push(new_op);
                                            }
                                        }

                                        if !new_ops.is_empty() {
                                            ops_to_insert.push((idx, new_ops));
                                            ops_to_remove.push(idx);
                                        }
                                    }
                                } else if &*extracted.name == "class" {
                                    // Get string literal value using helper
                                    if let Some(class_str) = extract_string_literal_value(expr) {
                                        let parsed_classes: Vec<&str> =
                                            class_str.trim().split_whitespace().collect();
                                        let mut new_ops = Vec::new();

                                        for class_name in parsed_classes {
                                            if !class_name.is_empty() {
                                                let new_op = create_extracted_attribute_op(
                                                    extracted.target,
                                                    BindingKind::ClassName,
                                                    None, // namespace
                                                    class_name.into(),
                                                    None, // expression
                                                    None, // i18n_context
                                                    None, // i18n_message
                                                    vec![SecurityContext::NONE],
                                                    None, // source_span
                                                );
                                                new_ops.push(new_op);
                                            }
                                        }

                                        if !new_ops.is_empty() {
                                            ops_to_insert.push((idx, new_ops));
                                            ops_to_remove.push(idx);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Apply changes: insert new ops and remove old ones
    // Insert in reverse order to maintain indices
    ops_to_insert.sort_by(|a, b| b.0.cmp(&a.0));
    for (idx, new_ops) in ops_to_insert {
        for new_op in new_ops.into_iter() {
            component_job.root.create.insert_at(idx, new_op);
        }
    }

    // Remove in reverse order
    ops_to_remove.sort();
    ops_to_remove.reverse();
    for idx in ops_to_remove {
        component_job.root.create.remove_at(idx);
    }

    // Process embedded views
    for (view_xref, view) in component_job.views.iter_mut() {
        let mut ops_to_remove = Vec::new();
        let mut ops_to_insert: Vec<(usize, Vec<Box<dyn ir::CreateOp + Send + Sync>>)> = Vec::new();

        for (idx, op) in view.create.iter().enumerate() {
            if op.kind() == OpKind::ExtractedAttribute {
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let extracted_ptr = op_ptr as *const ExtractedAttributeOp;
                    let extracted = &*extracted_ptr;

                    if extracted.binding_kind == BindingKind::Attribute {
                        if let Some(ref expr) = extracted.expression {
                            if is_string_literal(expr) {
                                // Find target element using map
                                if let Some(element_map) = view_element_indices.get(view_xref) {
                                    if let Some(&target_idx) = element_map.get(&extracted.target) {
                                        let target_op = view.create.get(target_idx).unwrap();

                                        // Skip structural templates
                                        if is_structural_template(target_op.as_ref()) {
                                            continue;
                                        }

                                        if &*extracted.name == "style" {
                                            // Get string literal value using helper
                                            if let Some(style_str) =
                                                extract_string_literal_value(expr)
                                            {
                                                let parsed_styles = parse_style(style_str);
                                                let mut new_ops = Vec::new();

                                                for i in (0..parsed_styles.len()).step_by(2) {
                                                    if i + 1 < parsed_styles.len() {
                                                        let prop_name = parsed_styles[i].clone();
                                                        let prop_value =
                                                            parsed_styles[i + 1].clone();

                                                        let new_op = create_extracted_attribute_op(
                                                            extracted.target,
                                                            BindingKind::StyleProperty,
                                                            None,
                                                            prop_name.into(),
                                                            Some(Expression::Literal(
                                                                LiteralExpr {
                                                                    value: LiteralValue::String(
                                                                        prop_value,
                                                                    ),
                                                                    type_: None,
                                                                    source_span: None,
                                                                },
                                                            )),
                                                            None,
                                                            None,
                                                            vec![SecurityContext::STYLE],
                                                            None, // source_span
                                                        );
                                                        new_ops.push(new_op);
                                                    }
                                                }

                                                if !new_ops.is_empty() {
                                                    ops_to_insert.push((idx, new_ops));
                                                    ops_to_remove.push(idx);
                                                }
                                            }
                                        } else if &*extracted.name == "class" {
                                            // Get string literal value using helper
                                            if let Some(class_str) =
                                                extract_string_literal_value(expr)
                                            {
                                                let parsed_classes: Vec<&str> =
                                                    class_str.trim().split_whitespace().collect();
                                                let mut new_ops = Vec::new();

                                                for class_name in parsed_classes {
                                                    if !class_name.is_empty() {
                                                        let new_op = create_extracted_attribute_op(
                                                            extracted.target,
                                                            BindingKind::ClassName,
                                                            None,
                                                            class_name.into(),
                                                            None,
                                                            None,
                                                            None,
                                                            vec![SecurityContext::NONE],
                                                            None, // source_span
                                                        );
                                                        new_ops.push(new_op);
                                                    }
                                                }

                                                if !new_ops.is_empty() {
                                                    ops_to_insert.push((idx, new_ops));
                                                    ops_to_remove.push(idx);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Apply changes
        ops_to_insert.sort_by(|a, b| b.0.cmp(&a.0));
        for (idx, new_ops) in ops_to_insert {
            for new_op in new_ops.into_iter() {
                view.create.insert_at(idx, new_op);
            }
        }

        ops_to_remove.sort();
        ops_to_remove.reverse();
        for idx in ops_to_remove {
            view.create.remove_at(idx);
        }
    }
}
