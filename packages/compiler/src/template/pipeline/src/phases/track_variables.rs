//! Track Variables Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/track_variables.ts
//! Inside the `track` expression on a `for` repeater, the `$index` and `$item` variables are
//! ambiently available. In this phase, we find those variable usages, and replace them with the
//! appropriate output read.

use crate::output::output_ast::Expression;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::expression::transform_expressions_in_expression;
use crate::template::pipeline::ir::ops::create::RepeaterCreateOp;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, CompilationUnit, ComponentCompilationJob,
};

/// Inside the `track` expression on a `for` repeater, the `$index` and `$item` variables are
/// ambiently available. In this phase, we find those variable usages, and replace them with the
/// appropriate output read.
pub fn generate_track_variables(job: &mut dyn CompilationJob) {
    let job_kind = job.kind();

    if matches!(
        job_kind,
        CompilationJobKind::Tmpl | CompilationJobKind::Both
    ) {
        let component_job = unsafe {
            let job_ptr = job as *mut dyn CompilationJob;
            let job_ptr = job_ptr as *mut ComponentCompilationJob;
            &mut *job_ptr
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
    for op in unit.create_mut().iter_mut() {
        if op.kind() != OpKind::RepeaterCreate {
            continue;
        }

        unsafe {
            let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
            let repeater_ptr = op_ptr as *mut RepeaterCreateOp;
            let repeater = &mut *repeater_ptr;

            // Transform track expression to replace LexicalReadExpr with appropriate variable reads
            let track_expr = (*repeater.track).clone();
            let dollar_index_names = repeater.var_names.dollar_index.clone();
            let dollar_implicit_name = repeater.var_names.dollar_implicit.clone();

            let transformed = transform_expressions_in_expression(
                track_expr,
                &mut |expr, _flags| {
                    if let Expression::LexicalRead(ref lexical_read) = expr {
                        // Check if this is $index (check if name is in dollar_index vector)
                        if dollar_index_names.iter().any(|n| n == &*lexical_read.name) {
                            return Expression::ReadVar(crate::output::output_ast::ReadVarExpr {
                                name: "$index".to_string(),
                                type_: None,
                                source_span: lexical_read.source_span.clone(),
                            });
                        }

                        // Check if this is $implicit (which becomes $item)
                        if &*lexical_read.name == dollar_implicit_name.as_str() {
                            return Expression::ReadVar(crate::output::output_ast::ReadVarExpr {
                                name: "$item".to_string(),
                                type_: None,
                                source_span: lexical_read.source_span.clone(),
                            });
                        }

                        // For other context variables that are not $index or $implicit,
                        // we leave them as LexicalReadExpr. These may be prohibited context variables
                        // that should be emitted as globals, but the exact handling is TBD.
                        // The TypeScript version also has this TODO with a question mark,
                        // indicating this is a future enhancement.
                    }
                    expr
                },
                ir::VisitorContextFlag::NONE,
            );

            repeater.track = Box::new(transformed);
        }
    }
}
