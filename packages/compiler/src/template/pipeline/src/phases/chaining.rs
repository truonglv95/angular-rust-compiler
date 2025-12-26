//! Chaining Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/chaining.ts
//! Converts sequential calls to chainable instructions into chain calls

use crate::output::output_ast::{Expression, ExternalReference, Statement};
use crate::render3::r3_identifiers::Identifiers;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::ops::shared::StatementOp;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob,
};
use std::collections::HashMap;
use std::hash::Hash;

const MAX_CHAIN_LENGTH: usize = 256;

/// Helper to create a key from ExternalReference for HashMap
/// Uses (module_name, name) tuple, ignoring runtime
#[derive(Hash, PartialEq, Eq)]
struct ExternalRefKey(Option<String>, Option<String>);

impl ExternalRefKey {
    fn from_ref(r: &ExternalReference) -> Self {
        ExternalRefKey(r.module_name.clone(), r.name.clone())
    }
}

/// Build CHAIN_COMPATIBILITY map
fn build_chain_compatibility_map() -> HashMap<ExternalRefKey, ExternalReference> {
    let mut map = HashMap::new();

    // All chainable instructions map to themselves
    let instructions = vec![
        Identifiers::aria_property(),
        Identifiers::attribute(),
        Identifiers::class_prop(),
        Identifiers::element(),
        Identifiers::element_container(),
        Identifiers::element_container_end(),
        Identifiers::element_container_start(),
        Identifiers::element_end(),
        Identifiers::element_start(),
        Identifiers::dom_property(),
        Identifiers::i18n_exp(),
        Identifiers::listener(),
        Identifiers::property(),
        Identifiers::style_prop(),
        Identifiers::synthetic_host_listener(),
        Identifiers::synthetic_host_property(),
        Identifiers::template_create(),
        Identifiers::two_way_property(),
        Identifiers::two_way_listener(),
        Identifiers::declare_let(),
        Identifiers::conditional_create(),
        Identifiers::conditional_branch_create(),
        Identifiers::dom_element(),
        Identifiers::dom_element_start(),
        Identifiers::dom_element_end(),
        Identifiers::dom_element_container(),
        Identifiers::dom_element_container_start(),
        Identifiers::dom_element_container_end(),
        Identifiers::dom_listener(),
        Identifiers::dom_template(),
        Identifiers::animation_enter(),
        Identifiers::animation_leave(),
        Identifiers::animation_enter_listener(),
        Identifiers::animation_leave_listener(),
    ];

    for ref_ in instructions {
        let key = ExternalRefKey::from_ref(&ref_);
        map.insert(key, ref_.clone());
    }

    map
}

/// Structure representing an in-progress chain
struct Chain {
    /// Index of the statement which holds the entire chain
    op_index: usize,
    /// The expression representing the whole current chained call
    expression: Expression,
    /// The instruction that is being chained
    instruction: ExternalReference,
    /// The number of instructions that have been collected into this chain
    length: usize,
}

/// Post-process a reified view compilation and convert sequential calls to chainable instructions
/// into chain calls.
pub fn chain(job: &mut dyn CompilationJob) {
    let component_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let job_ptr = job_ptr as *mut ComponentCompilationJob;
        &mut *job_ptr
    };

    let chain_compat = build_chain_compatibility_map();

    // Process root unit
    chain_operations_in_list_create(&mut component_job.root.create_mut(), &chain_compat);
    chain_operations_in_list_update(&mut component_job.root.update_mut(), &chain_compat);

    // Process all view units
    for (_, unit) in component_job.views.iter_mut() {
        chain_operations_in_list_create(&mut unit.create_mut(), &chain_compat);
        chain_operations_in_list_update(&mut unit.update_mut(), &chain_compat);
    }
}

fn chain_operations_in_list_create(
    op_list: &mut ir::operations::OpList<Box<dyn ir::CreateOp + Send + Sync>>,
    chain_compat: &HashMap<ExternalRefKey, ExternalReference>,
) {
    chain_operations_impl_create(op_list, chain_compat);
}

fn chain_operations_in_list_update(
    op_list: &mut ir::operations::OpList<Box<dyn ir::UpdateOp + Send + Sync>>,
    chain_compat: &HashMap<ExternalRefKey, ExternalReference>,
) {
    chain_operations_impl_update(op_list, chain_compat);
}

fn chain_operations_impl_create(
    op_list: &mut ir::operations::OpList<Box<dyn ir::CreateOp + Send + Sync>>,
    chain_compat: &HashMap<ExternalRefKey, ExternalReference>,
) {
    // Use the macro but need to handle the type properly
    // For now, we'll inline the logic since macro with generic types is complex
    let mut chain: Option<Chain> = None;
    let mut indices_to_remove: Vec<usize> = Vec::new();
    let mut updates: HashMap<usize, Expression> = HashMap::new();

    for (index, op) in op_list.iter().enumerate() {
        println!("Op[{}]: Kind={:?}", index, op.kind());
        if op.kind() == OpKind::Text {
            if let Some(text_op) = op.as_any().downcast_ref::<ir::ops::create::TextOp>() {
                if text_op.handle.get_slot().is_none() {
                    continue;
                }
            }
        }

        if op.kind() != OpKind::Statement {
            chain = None;
            continue;
        }

        let statement_op = unsafe {
            let op_ptr = op.as_ref() as *const dyn ir::operations::Op;
            let stmt_op_ptr = op_ptr as *const StatementOp<Box<dyn ir::CreateOp + Send + Sync>>;
            &*stmt_op_ptr
        };

        let expr_stmt = match statement_op.statement.as_ref() {
            Statement::Expression(expr_stmt) => expr_stmt,
            _ => {
                chain = None;
                continue;
            }
        };

        let invoke_expr = match expr_stmt.expr.as_ref() {
            Expression::InvokeFn(invoke) => invoke,
            _ => {
                chain = None;
                continue;
            }
        };

        let external_expr = match invoke_expr.fn_.as_ref() {
            Expression::External(ext) => ext,
            _ => {
                chain = None;
                continue;
            }
        };

        let instruction = &external_expr.value;
        let instruction_key = ExternalRefKey::from_ref(instruction);

        if !chain_compat.contains_key(&instruction_key) {
            chain = None;
            continue;
        }

        if let Some(ref mut chain_state) = chain {
            let chain_key = ExternalRefKey::from_ref(&chain_state.instruction);
            let compatible_instruction = chain_compat.get(&chain_key);
            let compatible_key = compatible_instruction.map(|r| ExternalRefKey::from_ref(r));
            if compatible_key.as_ref() == Some(&instruction_key)
                && chain_state.length < MAX_CHAIN_LENGTH
            {
                let new_expr = chain_state.expression.call_fn(
                    invoke_expr.args.clone(),
                    invoke_expr.source_span.clone(),
                    Some(invoke_expr.pure),
                );

                chain_state.expression = *new_expr;
                chain_state.length += 1;

                indices_to_remove.push(index);
                updates.insert(chain_state.op_index, chain_state.expression.clone());
            } else {
                chain = Some(Chain {
                    op_index: index,
                    expression: Expression::InvokeFn(invoke_expr.clone()),
                    instruction: instruction.clone(),
                    length: 1,
                });
            }
        } else {
            chain = Some(Chain {
                op_index: index,
                expression: Expression::InvokeFn(invoke_expr.clone()),
                instruction: instruction.clone(),
                length: 1,
            });
        }
    }

    // Apply updates
    // println!("Applying updates: count {}, Removing indices: count {}", updates.len(), indices_to_remove.len());
    for (index, expr) in updates {
        if let Some(op) = op_list.get_mut(index) {
            if let Some(stmt_op) = op
                .as_any_mut()
                .downcast_mut::<StatementOp<Box<dyn ir::CreateOp + Send + Sync>>>()
            {
                if let Statement::Expression(ref mut expr_stmt) = *stmt_op.statement {
                    expr_stmt.expr = Box::new(expr);
                }
            }
        }
    }

    // println!("Indices to remove: {:?}", indices_to_remove);
    for index in indices_to_remove.iter().rev() {
        op_list.remove_at(*index);
    }
}

fn chain_operations_impl_update(
    op_list: &mut ir::operations::OpList<Box<dyn ir::UpdateOp + Send + Sync>>,
    chain_compat: &HashMap<ExternalRefKey, ExternalReference>,
) {
    // Same logic as create but for UpdateOp
    let mut chain: Option<Chain> = None;
    let mut indices_to_remove: Vec<usize> = Vec::new();
    let mut updates: HashMap<usize, Expression> = HashMap::new();

    for (index, op) in op_list.iter().enumerate() {
        if op.kind() != OpKind::Statement {
            chain = None;
            continue;
        }

        let statement_op = unsafe {
            let op_ptr = op.as_ref() as *const dyn ir::operations::Op;
            let stmt_op_ptr = op_ptr as *const StatementOp<Box<dyn ir::UpdateOp + Send + Sync>>;
            &*stmt_op_ptr
        };

        let expr_stmt = match statement_op.statement.as_ref() {
            Statement::Expression(expr_stmt) => expr_stmt,
            _ => {
                chain = None;
                continue;
            }
        };

        let invoke_expr = match expr_stmt.expr.as_ref() {
            Expression::InvokeFn(invoke) => invoke,
            _ => {
                chain = None;
                continue;
            }
        };

        let external_expr = match invoke_expr.fn_.as_ref() {
            Expression::External(ext) => ext,
            _ => {
                chain = None;
                continue;
            }
        };

        let instruction = &external_expr.value;
        let instruction_key = ExternalRefKey::from_ref(instruction);

        // println!("Checking instruction: {:?}", instruction_key.1);

        if !chain_compat.contains_key(&instruction_key) {
            // println!("Not compatible: {:?}", instruction_key.1);
            chain = None;
            continue;
        }

        if let Some(ref mut chain_state) = chain {
            let chain_key = ExternalRefKey::from_ref(&chain_state.instruction);
            let compatible_instruction = chain_compat.get(&chain_key);
            let compatible_key = compatible_instruction.map(|r| ExternalRefKey::from_ref(r));

            // println!("Chain state: {:?}, Compatible: {:?}, Current: {:?}", chain_key.1, compatible_key.as_ref().map(|k| &k.1), instruction_key.1);

            if compatible_key.as_ref() == Some(&instruction_key)
                && chain_state.length < MAX_CHAIN_LENGTH
            {
                let new_expr = chain_state.expression.call_fn(
                    invoke_expr.args.clone(),
                    invoke_expr.source_span.clone(),
                    Some(invoke_expr.pure),
                );

                chain_state.expression = *new_expr;
                chain_state.length += 1;

                indices_to_remove.push(index);
                updates.insert(chain_state.op_index, chain_state.expression.clone());
                println!("Extended chain for {:?}", instruction_key.1);
            } else {
                println!(
                    "Broken chain. Expected {:?} but got {:?}",
                    compatible_key.as_ref().map(|k| &k.1),
                    instruction_key.1
                );
                chain = Some(Chain {
                    op_index: index,
                    expression: Expression::InvokeFn(invoke_expr.clone()),
                    instruction: instruction.clone(),
                    length: 1,
                });
            }
        } else {
            // println!("Started new chain for {:?}", instruction_key.1);
            chain = Some(Chain {
                op_index: index,
                expression: Expression::InvokeFn(invoke_expr.clone()),
                instruction: instruction.clone(),
                length: 1,
            });
        }
    }

    // Apply updates
    for (index, expr) in updates {
        if let Some(op) = op_list.get_mut(index) {
            if let Some(stmt_op) = op
                .as_any_mut()
                .downcast_mut::<StatementOp<Box<dyn ir::UpdateOp + Send + Sync>>>()
            {
                if let Statement::Expression(ref mut expr_stmt) = *stmt_op.statement {
                    expr_stmt.expr = Box::new(expr);
                }
            }
        }
    }

    for index in indices_to_remove.iter().rev() {
        op_list.remove_at(*index);
    }
}
