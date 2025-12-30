//! Remove Illegal Let References Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/remove_illegal_let_references.ts
//! It's not allowed to access a `@let` declaration before it has been defined. This is enforced
//! already via template type checking, however it can trip some of the assertions in the pipeline.
//! This phase detects illegal forward references and replaces them with `undefined`.

use crate::output::output_ast::{Expression, ExpressionTrait};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::expression::transform_expressions_in_op;
use crate::template::pipeline::ir::ops::shared::VariableOp;
use crate::template::pipeline::ir::variable::{SemanticVariable, SemanticVariableKind};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, CompilationUnit, ComponentCompilationJob,
};

/// It's not allowed to access a `@let` declaration before it has been defined. This phase detects
/// illegal forward references and replaces them with `undefined`.
pub fn remove_illegal_let_references(job: &mut dyn CompilationJob) {
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
    // Collect @let variable names with their indices
    let mut let_vars: Vec<(usize, String)> = Vec::new();

    // First pass: collect @let variable declarations
    for (var_index, op) in unit.update().iter().enumerate() {
        if op.kind() != OpKind::Variable {
            continue;
        }

        unsafe {
            let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
            let variable_op_ptr = op_ptr as *const VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>;
            let variable_op = &*variable_op_ptr;

            // Check if variable is Identifier and initializer is StoreLetExpr
            if variable_op.variable.kind() != SemanticVariableKind::Identifier {
                continue;
            }

            if let SemanticVariable::Identifier(identifier_var) = &variable_op.variable {
                if let Expression::StoreLet(_) = &*variable_op.initializer {
                    let_vars.push((var_index, identifier_var.identifier.clone()));
                }
            }
        }
    }

    // Second pass: for each @let variable, transform forward references
    for (var_index, let_name) in let_vars {
        // Iterate backwards from var_index to find forward references
        for prev_index in (0..var_index).rev() {
            // Get mutable reference to the op at prev_index
            if let Some(prev_op_mut) = unit.update_mut().get_mut(prev_index) {
                unsafe {
                    let prev_op_mut_ptr = prev_op_mut.as_mut() as *mut dyn ir::UpdateOp;

                    transform_expressions_in_op(
                        &mut *prev_op_mut_ptr,
                        &mut |expr, _flags| {
                            if let Expression::LexicalRead(lexical_read) = &expr {
                                if &*lexical_read.name == let_name.as_str() {
                                    // Replace with undefined literal (use null as equivalent to undefined)
                                    return Expression::Literal(
                                        crate::output::output_ast::LiteralExpr {
                                            value: crate::output::output_ast::LiteralValue::Null,
                                            type_: None,
                                            source_span: expr.source_span().cloned(),
                                        },
                                    );
                                }
                            }
                            expr
                        },
                        ir::VisitorContextFlag::NONE,
                    );
                }
            }
        }
    }
}
