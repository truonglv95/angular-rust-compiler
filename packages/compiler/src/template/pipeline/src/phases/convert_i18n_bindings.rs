//! Convert I18n Bindings Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/convert_i18n_bindings.ts
//!
//! Some binding instructions in the update block may actually correspond to i18n bindings. In that
//! case, they should be replaced with i18nExp instructions for the dynamic portions.

use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{I18nExpressionFor, I18nParamResolutionTime, OpKind};
use crate::template::pipeline::ir::ops::create::I18nAttributesOp;
use crate::template::pipeline::ir::ops::update::{
    create_i18n_expression_op, AttributeOp, PropertyOp,
};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, ComponentCompilationJob,
};

/// Convert i18n bindings to i18n expression ops.
pub fn convert_i18n_bindings(job: &mut dyn CompilationJob) {
    let job_kind = job.kind();

    if matches!(
        job_kind,
        CompilationJobKind::Tmpl | CompilationJobKind::Both
    ) {
        let component_job = unsafe {
            let job_ptr = job as *mut dyn CompilationJob;
            let component_job_ptr = job_ptr as *mut ComponentCompilationJob;
            &mut *component_job_ptr
        };

        // Process root unit
        process_unit(&mut component_job.root);

        // Process all view units
        for (_, unit) in component_job.views.iter_mut() {
            process_unit(unit);
        }
    }
}

fn process_unit(unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit) {
    // Build map of I18nAttributesOps by target element
    let mut i18n_attributes_by_elem: std::collections::HashMap<ir::XrefId, I18nAttributesOp> =
        std::collections::HashMap::new();

    for op in unit.create.iter() {
        if op.kind() == OpKind::I18nAttributes {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let attrs_ptr = op_ptr as *const I18nAttributesOp;
                let attrs = &*attrs_ptr;
                i18n_attributes_by_elem.insert(attrs.target, attrs.clone());
            }
        }
    }

    // Collect ops to replace
    let mut ops_to_replace: Vec<(usize, Vec<Box<dyn ir::UpdateOp + Send + Sync>>)> = Vec::new();

    for (idx, op) in unit.update.iter().enumerate() {
        match op.kind() {
            OpKind::Property | OpKind::Attribute => {
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                    let i18n_context = match op.kind() {
                        OpKind::Property => {
                            let prop_ptr = op_ptr as *const PropertyOp;
                            let prop = &*prop_ptr;
                            prop.i18n_context
                        }
                        OpKind::Attribute => {
                            let attr_ptr = op_ptr as *const AttributeOp;
                            let attr = &*attr_ptr;
                            attr.i18n_context
                        }
                        _ => None,
                    };

                    if i18n_context.is_none() {
                        continue;
                    }

                    let expression = match op.kind() {
                        OpKind::Property => {
                            let prop_ptr = op_ptr as *const PropertyOp;
                            let prop = &*prop_ptr;
                            prop.expression.clone()
                        }
                        OpKind::Attribute => {
                            let attr_ptr = op_ptr as *const AttributeOp;
                            let attr = &*attr_ptr;
                            attr.expression.clone()
                        }
                        _ => continue,
                    };

                    // Check if expression is Interpolation
                    let interpolation = match &expression {
                        ir::ops::update::BindingExpression::Interpolation(interp) => interp.clone(),
                        _ => continue,
                    };

                    let (target, name, source_span) = match op.kind() {
                        OpKind::Property => {
                            let prop_ptr = op_ptr as *const PropertyOp;
                            let prop = &*prop_ptr;
                            (prop.target, prop.name.clone(), prop.source_span.clone())
                        }
                        OpKind::Attribute => {
                            let attr_ptr = op_ptr as *const AttributeOp;
                            let attr = &*attr_ptr;
                            (attr.target, attr.name.clone(), attr.source_span.clone())
                        }
                        _ => continue,
                    };

                    let i18n_attributes_for_elem = i18n_attributes_by_elem.get(&target);
                    if i18n_attributes_for_elem.is_none() {
                        panic!("AssertionError: An i18n attribute binding instruction requires the owning element to have an I18nAttributes create instruction");
                    }
                    let i18n_attrs = i18n_attributes_for_elem.unwrap();

                    if i18n_attrs.target != target {
                        panic!("AssertionError: Expected i18nAttributes target element to match binding target element");
                    }

                    if interpolation.i18n_placeholders.len() != interpolation.expressions.len() {
                        panic!(
                            "AssertionError: An i18n attribute binding instruction requires the same number of expressions and placeholders, but found {} placeholders and {} expressions",
                            interpolation.i18n_placeholders.len(),
                            interpolation.expressions.len()
                        );
                    }

                    let mut new_ops: Vec<Box<dyn ir::UpdateOp + Send + Sync>> = Vec::new();
                    for (i, expr) in interpolation.expressions.iter().enumerate() {
                        let i18n_expr_op = create_i18n_expression_op(
                            i18n_context.unwrap(),
                            i18n_attrs.target,
                            i18n_attrs.xref,
                            i18n_attrs.handle.clone(),
                            expr.clone(),
                            None, // icu_placeholder
                            Some((&*interpolation.i18n_placeholders[i]).into()),
                            I18nParamResolutionTime::Creation,
                            I18nExpressionFor::I18nAttribute,
                            name.clone(),
                            source_span.clone(),
                        );
                        new_ops.push(i18n_expr_op);
                    }

                    ops_to_replace.push((idx, new_ops));
                }
            }
            _ => {}
        }
    }

    // Replace ops in reverse order to maintain indices
    ops_to_replace.sort_by(|a, b| b.0.cmp(&a.0));
    for (idx, new_ops) in ops_to_replace {
        unit.update.replace_at_with_many(idx, new_ops);
    }
}
