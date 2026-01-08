//! Temporary Variables Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/temporary_variables.ts
//! Find all assignments and usages of temporary variables, which are linked to each other with cross
//! references. Generate names for each cross-reference, and add a `DeclareVarStmt` to initialize
//! them at the beginning of the update block.

use crate::output::output_ast::{Expression, Statement};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::expression::{transform_expressions_in_op, VisitorContextFlag};
use crate::template::pipeline::ir::ops::shared::create_statement_op;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, ComponentCompilationJob,
};
use std::collections::{HashMap, HashSet};

/// Find all assignments and usages of temporary variables, which are linked to each other with cross
/// references. Generate names for each cross-reference, and add a `DeclareVarStmt` to initialize
/// them at the beginning of the update block.
pub fn generate_temporary_variables(job: &mut dyn CompilationJob) {
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
        {
            use crate::template::pipeline::src::compilation::CompilationUnit;
            let create_stmts = generate_temporaries_create(component_job.root.create_mut());
            component_job.root.create_mut().prepend(create_stmts);

            let update_stmts = generate_temporaries_update(component_job.root.update_mut());
            component_job.root.update_mut().prepend(update_stmts);
        }

        // Process all view units
        for (_, unit) in component_job.views.iter_mut() {
            use crate::template::pipeline::src::compilation::CompilationUnit;
            let create_stmts = generate_temporaries_create(unit.create_mut());
            unit.create_mut().prepend(create_stmts);

            let update_stmts = generate_temporaries_update(unit.update_mut());
            unit.update_mut().prepend(update_stmts);
        }
    } else if matches!(job_kind, CompilationJobKind::Host) {
        use crate::template::pipeline::src::compilation::HostBindingCompilationJob;
        let host_job = unsafe {
            let job_ptr = job as *mut dyn CompilationJob;
            let job_ptr = job_ptr as *mut HostBindingCompilationJob;
            &mut *job_ptr
        };

        // Process root unit
        {
            use crate::template::pipeline::src::compilation::CompilationUnit;
            let create_stmts = generate_temporaries_create(host_job.root.create_mut());
            host_job.root.create_mut().prepend(create_stmts);

            let update_stmts = generate_temporaries_update(host_job.root.update_mut());
            host_job.root.update_mut().prepend(update_stmts);
        }
    }
}

fn generate_temporaries_create(
    ops: &mut ir::OpList<Box<dyn ir::CreateOp + Send + Sync>>,
) -> Vec<Box<dyn ir::CreateOp + Send + Sync>> {
    generate_temporaries_impl_create(ops)
}

fn generate_temporaries_update(
    ops: &mut ir::OpList<Box<dyn ir::UpdateOp + Send + Sync>>,
) -> Vec<Box<dyn ir::UpdateOp + Send + Sync>> {
    generate_temporaries_impl_update(ops)
}

// Shared implementation logic - process temp vars in an op
fn process_temp_vars_in_op(
    op: &mut dyn ir::operations::Op,
    op_count: usize,
) -> (Vec<String>, HashMap<ir::XrefId, String>) {
    // Identify the final time each temp var is read.
    // We'll collect all ReadTemporaryExpr xrefs first, then the last one we see is the final read
    let mut read_xrefs: Vec<ir::XrefId> = Vec::new();

    transform_expressions_in_op(
        op,
        &mut |expr: Expression, flag: VisitorContextFlag| {
            if flag.contains(VisitorContextFlag::IN_CHILD_OPERATION) {
                return expr;
            }

            if let Expression::ReadTemporary(ref read_temp) = expr {
                read_xrefs.push(read_temp.xref);
            }
            expr
        },
        VisitorContextFlag::NONE,
    );

    // Build final_reads map: track the last occurrence of each xref
    let mut final_reads: HashSet<ir::XrefId> = HashSet::new();
    let mut seen: HashSet<ir::XrefId> = HashSet::new();
    for xref in read_xrefs.iter().rev() {
        if !seen.contains(xref) {
            final_reads.insert(*xref);
            seen.insert(*xref);
        }
    }

    // Count total reads per xref
    let mut total_read_count: HashMap<ir::XrefId, usize> = HashMap::new();
    for xref in &read_xrefs {
        *total_read_count.entry(*xref).or_insert(0) += 1;
    }

    // Name the temp vars, accounting for the fact that a name can be reused after it has been
    // read for the final time.
    let mut count: usize = 0;
    let mut assigned = HashSet::new();
    let mut defs: HashMap<ir::XrefId, String> = HashMap::new();
    let mut read_counter: HashMap<ir::XrefId, usize> = total_read_count.clone();

    // Note: op is already &mut dyn Op, so we pass it directly
    transform_expressions_in_op(
        op,
        &mut |expr: Expression, flag: VisitorContextFlag| -> Expression {
            if flag.contains(VisitorContextFlag::IN_CHILD_OPERATION) {
                return expr;
            }

            match expr {
                Expression::AssignTemporary(mut assign_temp) => {
                    if !assigned.contains(&assign_temp.xref) {
                        assigned.insert(assign_temp.xref);
                        defs.insert(assign_temp.xref, format!("tmp_{}_{}", op_count, count));
                        count += 1;
                    }
                    if let Some(name) = defs.get(&assign_temp.xref) {
                        assign_temp.name = Some(name.clone());
                    }
                    Expression::AssignTemporary(assign_temp)
                }
                Expression::ReadTemporary(mut read_temp) => {
                    // Decrement read counter for this xref
                    let remaining = read_counter.entry(read_temp.xref).or_insert(0);
                    let is_final_read = *remaining == 1 && final_reads.contains(&read_temp.xref);
                    *remaining = remaining.saturating_sub(1);

                    // If this is the final read, we can release the name
                    if is_final_read {
                        count = count.saturating_sub(1);
                    }

                    // Assign name if not already assigned
                    if let Some(name) = defs.get(&read_temp.xref) {
                        read_temp.name = Some(name.clone());
                    } else {
                        // If name not yet assigned, assign it now
                        if !assigned.contains(&read_temp.xref) {
                            assigned.insert(read_temp.xref);
                            let name = format!("tmp_{}_{}", op_count, count);
                            count += 1;
                            defs.insert(read_temp.xref, name.clone());
                            read_temp.name = Some(name);
                        }
                    }
                    Expression::ReadTemporary(read_temp)
                }
                _ => expr,
            }
        },
        VisitorContextFlag::NONE,
    );

    // Get unique names for declarations
    let unique_names: HashSet<String> = defs.values().cloned().collect();
    (unique_names.into_iter().collect(), defs)
}

fn generate_temporaries_impl_create(
    ops: &mut ir::OpList<Box<dyn ir::CreateOp + Send + Sync>>,
) -> Vec<Box<dyn ir::CreateOp + Send + Sync>> {
    let mut op_count = 0;
    let mut generated_statements: Vec<Box<dyn ir::CreateOp + Send + Sync>> = Vec::new();

    // For each op, search for any variables that are assigned or read. For each variable, generate a
    // name and produce a `DeclareVarStmt` to the beginning of the block.
    for op in ops.iter_mut() {
        let (unique_names, _defs) = process_temp_vars_in_op(&mut **op, op_count);

        // Add declarations for the temp vars.
        for name in unique_names {
            let stmt = Statement::DeclareVar(crate::output::output_ast::DeclareVarStmt {
                name,
                value: None,
                type_: None,
                modifiers: crate::output::output_ast::StmtModifier::None,
                source_span: None,
            });

            // Create StatementOp as CreateOp
            let stmt_op =
                create_statement_op::<Box<dyn ir::CreateOp + Send + Sync>>(Box::new(stmt));
            generated_statements.push(Box::new(stmt_op));
        }

        op_count += 1;

        // Recursively process handler ops for listeners and trackByOps for repeaters
        match op.kind() {
            OpKind::Listener
            | OpKind::Animation
            | OpKind::AnimationListener
            | OpKind::TwoWayListener => unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::operations::Op;
                match op.kind() {
                    OpKind::Listener => {
                        use crate::template::pipeline::ir::ops::create::ListenerOp;
                        let listener_ptr = op_ptr as *mut ListenerOp;
                        let listener = &mut *listener_ptr;
                        let handler_stmts = generate_temporaries_update(&mut listener.handler_ops);
                        listener.handler_ops.prepend(handler_stmts);
                    }
                    OpKind::TwoWayListener => {
                        use crate::template::pipeline::ir::ops::create::TwoWayListenerOp;
                        let two_way_ptr = op_ptr as *mut TwoWayListenerOp;
                        let two_way = &mut *two_way_ptr;
                        let handler_stmts = generate_temporaries_update(&mut two_way.handler_ops);
                        two_way.handler_ops.prepend(handler_stmts);
                    }
                    OpKind::Animation => {
                        use crate::template::pipeline::ir::ops::create::AnimationOp;
                        let anim_ptr = op_ptr as *mut AnimationOp;
                        let anim = &mut *anim_ptr;
                        let handler_stmts = generate_temporaries_update(&mut anim.handler_ops);
                        anim.handler_ops.prepend(handler_stmts);
                    }
                    OpKind::AnimationListener => {
                        use crate::template::pipeline::ir::ops::create::AnimationListenerOp;
                        let anim_listener_ptr = op_ptr as *mut AnimationListenerOp;
                        let anim_listener = &mut *anim_listener_ptr;
                        let handler_stmts =
                            generate_temporaries_update(&mut anim_listener.handler_ops);
                        anim_listener.handler_ops.prepend(handler_stmts);
                    }
                    _ => {}
                }
            },
            OpKind::RepeaterCreate => unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::operations::Op;
                use crate::template::pipeline::ir::ops::create::RepeaterCreateOp;
                let rep_ptr = op_ptr as *mut RepeaterCreateOp;
                let rep = &mut *rep_ptr;
                if let Some(ref mut track_by_ops) = rep.track_by_ops {
                    let track_by_stmts = generate_temporaries_update(track_by_ops);
                    track_by_ops.prepend(track_by_stmts);
                }
            },
            _ => {}
        }
    }

    generated_statements
}

fn generate_temporaries_impl_update(
    ops: &mut ir::OpList<Box<dyn ir::UpdateOp + Send + Sync>>,
) -> Vec<Box<dyn ir::UpdateOp + Send + Sync>> {
    let mut op_count = 0;
    let mut generated_statements: Vec<Box<dyn ir::UpdateOp + Send + Sync>> = Vec::new();

    // For each op, search for any variables that are assigned or read. For each variable, generate a
    // name and produce a `DeclareVarStmt` to the beginning of the block.
    for op in ops.iter_mut() {
        let (unique_names, _defs) = process_temp_vars_in_op(&mut **op, op_count);

        // Add declarations for the temp vars.
        for name in unique_names {
            let stmt = Statement::DeclareVar(crate::output::output_ast::DeclareVarStmt {
                name,
                value: None,
                type_: None,
                modifiers: crate::output::output_ast::StmtModifier::None,
                source_span: None,
            });

            // Create StatementOp as UpdateOp
            let stmt_op =
                create_statement_op::<Box<dyn ir::UpdateOp + Send + Sync>>(Box::new(stmt));
            generated_statements.push(Box::new(stmt_op));
        }

        op_count += 1;
    }

    generated_statements
}
