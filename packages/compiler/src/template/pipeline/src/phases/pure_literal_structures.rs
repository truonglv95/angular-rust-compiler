//! Generate pure literal structures (arrays/maps) using PureFunctionExpr.
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/pure_literal_structures.ts

use crate::output::output_ast as o;
use crate::template::pipeline::ir;
use crate::template::pipeline::src::compilation::{CompilationJob, ComponentCompilationJob, CompilationUnit};
use crate::output::output_ast::ExpressionTrait;

pub fn phase(job: &mut dyn CompilationJob) {
    println!("DEBUG: pure_literal_structures phase START. Job kind: {:?}", job.kind());
    fn process_expr(expr: o::Expression) -> o::Expression {
        match expr {
            o::Expression::LiteralArray(ref arr) => {
                println!("DEBUG: pure_literal_structures visiting LiteralArray with {} entries", arr.entries.len());
                if let Some(transformed) = transform_literal_array(arr) {
                    println!("DEBUG: LiteralArray transformed to PureFunction");
                    return transformed;
                } else {
                    println!("DEBUG: LiteralArray NOT transformed");
                }
            },
            o::Expression::LiteralMap(ref map) => {
                if let Some(transformed) = transform_literal_map(map) {
                    return transformed;
                }
            },
            _ => {}
        }
        expr
    }

    if let Some(component_job) = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        if job.kind() == crate::template::pipeline::src::compilation::CompilationJobKind::Tmpl {
            Some(&mut *(job_ptr as *mut ComponentCompilationJob))
        } else {
            None
        }
    } {
        // Helper to process a unit
        let mut process_unit = |unit: &mut dyn CompilationUnit| {
            for op in unit.update_mut().iter_mut() {
                 ir::transform_expressions_in_op(op.as_mut(), &mut |expr: o::Expression, flags: ir::VisitorContextFlag| {
                     if flags.contains(ir::VisitorContextFlag::IN_CHILD_OPERATION) {
                         return expr;
                     }
                     process_expr(expr)
                 }, ir::VisitorContextFlag::NONE);
            }
        };

        // Process root view
        process_unit(&mut component_job.root);

        // Process nested views
        for unit in component_job.views.values_mut() {
            process_unit(unit);
        }
    }
}

fn transform_literal_array(arr: &o::LiteralArrayExpr) -> Option<o::Expression> {
    let mut derived_entries = Vec::new();
    let mut args = Vec::new();
    
    for entry in &arr.entries {
        if entry.is_constant() {
            derived_entries.push(entry.clone());
        } else {
            let idx = args.len();
            args.push(entry.clone());
            // PureFunctionParameterExpr
            let param = ir::expression::PureFunctionParameterExpr::new(idx);
            derived_entries.push(o::Expression::PureFunctionParameter(param));
        }
    }

    // If args is empty, we still want to create a pure function (pureFunction0)
    // for constant arrays to ensure proper change detection and slot allocation.
    
    let literal_expr = o::literal_arr(derived_entries);
    
    let pure_fn = ir::expression::PureFunctionExpr::new(
        Some(literal_expr),
        args.into_iter().collect(),
    );
        
    Some(o::Expression::PureFunction(pure_fn))
}

fn transform_literal_map(map: &o::LiteralMapExpr) -> Option<o::Expression> {
    let mut derived_entries = Vec::new();
    let mut args = Vec::new();
    
    for entry in &map.entries {
        if entry.value.is_constant() {
            derived_entries.push(entry.clone());
        } else {
             let idx = args.len();
             args.push(entry.value.clone());
             
             let param = ir::expression::PureFunctionParameterExpr::new(idx);
             let param_expr: o::Expression = o::Expression::PureFunctionParameter(param);
             
             derived_entries.push(o::LiteralMapEntry {
                 key: entry.key.clone(),
                 value: Box::new(param_expr),
                 quoted: entry.quoted,
             });
        }
    }


    
    let literal_expr = o::literal_map(derived_entries);
    
    let pure_fn = ir::expression::PureFunctionExpr::new(
        Some(literal_expr),
        args.into_iter().map(|b| *b).collect(),
    );
        
    Some(o::Expression::PureFunction(pure_fn))
}
