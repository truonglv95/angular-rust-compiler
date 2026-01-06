//! Generate projection definitions.
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/generate_projection_def.ts
//!
//! Locate projection slots, populate the each component's `ngContentSelectors` literal field,
//! populate `project` arguments, and generate the required `projectionDef` instruction for the job's
//! root view.

use crate::core::SelectorFlags;
use crate::directive_matching::CssSelector;
use crate::output::output_ast::Expression;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::ops::create::{create_projection_def_op, ProjectionOp};
use crate::template::pipeline::src::compilation::{CompilationJob, ComponentCompilationJob};
use crate::template::pipeline::src::conversion::{literal_or_array_literal, LiteralType};

pub fn generate_projection_defs(job: &mut dyn CompilationJob) {
    if job.kind() != crate::template::pipeline::src::compilation::CompilationJobKind::Tmpl {
        return;
    }

    let component_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let component_job_ptr = job_ptr as *mut ComponentCompilationJob;
        &mut *component_job_ptr
    };

    // TODO: Why does TemplateDefinitionBuilder force a shared constant?
    let share = component_job.compatibility() == ir::CompatibilityMode::TemplateDefinitionBuilder;

    // Collect all selectors from this component, and its nested views. Also, assign each projection a
    // unique ascending projection slot index.
    // IMPORTANT: Process all views in xref order to preserve the template source order.
    // This ensures that ng-content elements inside ng-template get indices matching
    // their position in the source template, not the order views are processed.
    let mut selectors = Vec::new();
    let mut projection_slot_index = 0;

    // Collect all projection ops from all views (root + embedded)
    struct ProjectionInfo {
        view_is_root: bool,
        view_xref: ir::XrefId,
        op_index: usize,
        selector: String,
        source_order: usize,
    }
    let mut all_projections: Vec<ProjectionInfo> = Vec::new();

    // Collect from root view
    for (idx, op) in component_job.root.create.iter().enumerate() {
        if op.kind() == ir::OpKind::Projection {
            if let Some(proj) = op.as_any().downcast_ref::<ProjectionOp>() {
                all_projections.push(ProjectionInfo {
                    view_is_root: true,
                    view_xref: component_job.root.xref,
                    op_index: idx,
                    selector: proj.selector.clone(),
                    source_order: proj.source_order,
                });
            }
        }
    }

    // Collect from embedded views
    for (view_xref, view) in component_job.views.iter() {
        for (idx, op) in view.create.iter().enumerate() {
            if op.kind() == ir::OpKind::Projection {
                if let Some(proj) = op.as_any().downcast_ref::<ProjectionOp>() {
                    all_projections.push(ProjectionInfo {
                        view_is_root: false,
                        view_xref: *view_xref,
                        op_index: idx,
                        selector: proj.selector.clone(),
                        source_order: proj.source_order,
                    });
                }
            }
        }
    }

    // Sort by source_order to preserve template source order
    // source_order is assigned during template ingestion in the order ng-content elements appear
    all_projections.sort_by_key(|p| p.source_order);

    // Assign projection slot indices in sorted order
    for proj_info in &all_projections {
        selectors.push(proj_info.selector.clone());
    }

    // Now apply the indices to the actual ops
    for (slot_index, proj_info) in all_projections.iter().enumerate() {
        if proj_info.view_is_root {
            if let Some(op) = component_job.root.create.iter_mut().nth(proj_info.op_index) {
                if op.kind() == ir::OpKind::Projection {
                    unsafe {
                        let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                        let projection_ptr = op_ptr as *mut ProjectionOp;
                        let projection = &mut *projection_ptr;
                        projection.projection_slot_index = slot_index;
                    }
                }
            }
        } else {
            if let Some(view) = component_job.views.get_mut(&proj_info.view_xref) {
                if let Some(op) = view.create.iter_mut().nth(proj_info.op_index) {
                    if op.kind() == ir::OpKind::Projection {
                        unsafe {
                            let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                            let projection_ptr = op_ptr as *mut ProjectionOp;
                            let projection = &mut *projection_ptr;
                            projection.projection_slot_index = slot_index;
                        }
                    }
                }
            }
        }
    }

    if !selectors.is_empty() {
        // Create the projectionDef array. If we only found a single wildcard selector, then we use the
        // default behavior with no arguments instead.
        let def_expr: Option<Expression> = if selectors.len() > 1 || selectors[0] != "*" {
            // Parse selectors to R3 selector format
            // ProjectionDef = (string | R3CssSelector[])[]
            let def: Vec<LiteralType> = selectors
                .iter()
                .map(|s| {
                    if s == "*" {
                        LiteralType::String(s.clone())
                    } else if let Ok(css_selectors) = CssSelector::parse(s) {
                        // R3CssSelector[]
                        let selectors_list: Vec<LiteralType> = css_selectors
                            .iter()
                            .map(|selector| {
                                let mut vec = Vec::new();
                                // Element name
                                vec.push(LiteralType::String(
                                    selector.element.clone().unwrap_or_default(),
                                ));

                                // Classes
                                for class_name in &selector.class_names {
                                    vec.push(LiteralType::Number(
                                        SelectorFlags::CLASS as u32 as f64,
                                    ));
                                    vec.push(LiteralType::String(class_name.clone()));
                                }

                                // Attributes
                                for i in (0..selector.attrs.len()).step_by(2) {
                                    vec.push(LiteralType::Number(
                                        SelectorFlags::ATTRIBUTE as u32 as f64,
                                    ));
                                    vec.push(LiteralType::String(selector.attrs[i].clone()));
                                    vec.push(LiteralType::String(selector.attrs[i + 1].clone()));
                                }

                                // Not Selectors
                                for not_selector in &selector.not_selectors {
                                    // Element
                                    if let Some(element) = &not_selector.element {
                                        if element != "*" {
                                            vec.push(LiteralType::Number(
                                                (SelectorFlags::NOT as u32
                                                    | SelectorFlags::ELEMENT as u32)
                                                    as f64,
                                            ));
                                            vec.push(LiteralType::String(element.clone()));
                                        }
                                    }

                                    // Classes
                                    for class_name in &not_selector.class_names {
                                        vec.push(LiteralType::Number(
                                            (SelectorFlags::NOT as u32
                                                | SelectorFlags::CLASS as u32)
                                                as f64,
                                        ));
                                        vec.push(LiteralType::String(class_name.clone()));
                                    }

                                    // Attributes
                                    for i in (0..not_selector.attrs.len()).step_by(2) {
                                        vec.push(LiteralType::Number(
                                            (SelectorFlags::NOT as u32
                                                | SelectorFlags::ATTRIBUTE as u32)
                                                as f64,
                                        ));
                                        vec.push(LiteralType::String(
                                            not_selector.attrs[i].clone(),
                                        ));
                                        vec.push(LiteralType::String(
                                            not_selector.attrs[i + 1].clone(),
                                        ));
                                    }
                                }

                                LiteralType::Array(vec)
                            })
                            .collect();

                        LiteralType::Array(selectors_list)
                    } else {
                        // Fallback if parse fails (shouldn't happen with valid selectors)
                        LiteralType::Array(vec![LiteralType::Array(vec![LiteralType::String(
                            s.clone(),
                        )])])
                    }
                })
                .collect();

            let expr = component_job
                .pool
                .get_const_literal(literal_or_array_literal(LiteralType::Array(def)), share);
            Some(expr)
        } else {
            None
        };

        // Create the ngContentSelectors constant
        let selectors_literal: Vec<LiteralType> = selectors
            .iter()
            .map(|s| LiteralType::String(s.clone()))
            .collect();

        component_job.content_selectors = Some(component_job.pool.get_const_literal(
            literal_or_array_literal(LiteralType::Array(selectors_literal)),
            share,
        ));

        // The projection def instruction goes at the beginning of the root view, before any
        // `projection` instructions.
        let def_op = create_projection_def_op(def_expr);
        component_job.root.create.prepend(vec![def_op]);
    }
}
