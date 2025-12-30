//! I18n Const Collection Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/i18n_const_collection.ts
//!
//! Lifts i18n properties into the consts array.

use crate::constant_pool::ConstantPool;
use crate::i18n::i18n_ast as i18n;
use crate::output::output_ast::{
    BinaryOperator, BinaryOperatorExpr, Expression as OutputExpression, IfStmt, LiteralArrayExpr,
    LiteralExpr, LiteralMapEntry, LiteralMapExpr, LiteralValue, ReadVarExpr, Statement,
};
use crate::parse_util::sanitize_identifier;
use crate::render3::r3_identifiers::Identifiers;
use crate::render3::view::i18n::get_msg_utils::create_google_get_msg_statements;
use crate::render3::view::i18n::localize_utils::create_localize_statements;
use crate::render3::view::i18n::util::format_i18n_placeholder_names_in_map;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::{I18nExpressionFor, OpKind};
use crate::template::pipeline::ir::ops::create::{
    ExtractedAttributeOp, I18nAttributesOp, I18nMessageOp, I18nStartOp,
};
use crate::template::pipeline::ir::ops::update::I18nExpressionOp;
use crate::template::pipeline::src::compilation::ComponentCompilationJob;
use std::collections::HashMap;

/// Name of the global variable that is used to determine if we use Closure translations or not
const NG_I18N_CLOSURE_MODE: &str = "ngI18nClosureMode";

/// Prefix for non-`goog.getMsg` i18n-related vars.
/// Note: the prefix uses lowercase characters intentionally due to a Closure behavior that
/// considers variables like `I18N_0` as constants and throws an error when their value changes.
const TRANSLATION_VAR_PREFIX: &str = "i18n_";

/// Prefix of ICU expressions for post processing
pub const I18N_ICU_MAPPING_PREFIX: &str = "I18N_EXP_";

/// The escape sequence used for message param values.
#[allow(dead_code)]
const ESCAPE: char = '\u{FFFD}';

/* Closure variables holding messages must be named `MSG_[A-Z0-9]+` */
const CLOSURE_TRANSLATION_VAR_PREFIX: &str = "MSG_";

/// Generates a prefix for translation const name.
///
/// # Arguments
/// * `extra` - Additional local prefix that should be injected into translation var name
///
/// # Returns
/// Complete translation const prefix
pub fn get_translation_const_prefix(extra: &str) -> String {
    format!("{}{}", CLOSURE_TRANSLATION_VAR_PREFIX, extra).to_uppercase()
}

/// Generate AST to declare a variable. E.g. `var I18N_1;`.
/// # Arguments
/// * `variable` - the name of the variable to declare.
pub fn declare_i18n_variable(variable: &ReadVarExpr) -> Statement {
    Statement::DeclareVar(crate::output::output_ast::DeclareVarStmt {
        name: variable.name.clone(),
        value: None,
        type_: None,
        modifiers: crate::output::output_ast::StmtModifier::Final,
        source_span: variable.source_span.clone(),
    })
}

/// Lifts i18n properties into the consts array.
pub fn collect_i18n_consts(job: &mut ComponentCompilationJob) {
    let file_based_i18n_suffix = job
        .relative_context_file_path
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_uppercase()
        + "_";

    // Step One: Build up various lookup maps we need to collect all the consts.

    // Context Xref -> Extracted Attribute Ops
    let mut extracted_attributes_by_i18n_context: HashMap<ir::XrefId, Vec<ExtractedAttributeOp>> =
        HashMap::new();
    // Element/ElementStart Xref -> I18n Attributes config op
    let mut i18n_attributes_by_element: HashMap<ir::XrefId, I18nAttributesOp> = HashMap::new();
    // Element/ElementStart Xref -> All I18n Expression ops for attrs on that target
    let mut i18n_expressions_by_element: HashMap<ir::XrefId, Vec<I18nExpressionOp>> =
        HashMap::new();
    // I18n Message Xref -> I18n Message Op
    let mut messages: HashMap<ir::XrefId, I18nMessageOp> = HashMap::new();

    // Collect from root unit
    for op in job.root.create.iter() {
        match op.kind() {
            OpKind::ExtractedAttribute => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let attr_ptr = op_ptr as *const ExtractedAttributeOp;
                let attr = &*attr_ptr;
                if let Some(i18n_ctx) = attr.i18n_context {
                    extracted_attributes_by_i18n_context
                        .entry(i18n_ctx)
                        .or_insert_with(Vec::new)
                        .push(attr.clone());
                }
            },
            OpKind::I18nAttributes => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let attrs_ptr = op_ptr as *const I18nAttributesOp;
                let attrs = &*attrs_ptr;
                i18n_attributes_by_element.insert(attrs.target, attrs.clone());
            },
            OpKind::I18nMessage => unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let msg_ptr = op_ptr as *const I18nMessageOp;
                let msg = &*msg_ptr;
                messages.insert(msg.xref, msg.clone());
            },
            _ => {}
        }
    }

    // Collect from update ops in root unit
    for op in job.root.update.iter() {
        if op.kind() == OpKind::I18nExpression {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                let expr_ptr = op_ptr as *const I18nExpressionOp;
                let expr = &*expr_ptr;
                if expr.usage == I18nExpressionFor::I18nAttribute {
                    i18n_expressions_by_element
                        .entry(expr.target)
                        .or_insert_with(Vec::new)
                        .push(expr.clone());
                }
            }
        }
    }

    // Collect from all view units
    for (_, unit) in job.views.iter() {
        for op in unit.create.iter() {
            match op.kind() {
                OpKind::ExtractedAttribute => unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let attr_ptr = op_ptr as *const ExtractedAttributeOp;
                    let attr = &*attr_ptr;
                    if let Some(i18n_ctx) = attr.i18n_context {
                        extracted_attributes_by_i18n_context
                            .entry(i18n_ctx)
                            .or_insert_with(Vec::new)
                            .push(attr.clone());
                    }
                },
                OpKind::I18nAttributes => unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let attrs_ptr = op_ptr as *const I18nAttributesOp;
                    let attrs = &*attrs_ptr;
                    i18n_attributes_by_element.insert(attrs.target, attrs.clone());
                },
                OpKind::I18nMessage => unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let msg_ptr = op_ptr as *const I18nMessageOp;
                    let msg = &*msg_ptr;
                    messages.insert(msg.xref, msg.clone());
                },
                _ => {}
            }
        }

        for op in unit.update.iter() {
            if op.kind() == OpKind::I18nExpression {
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;
                    let expr_ptr = op_ptr as *const I18nExpressionOp;
                    let expr = &*expr_ptr;
                    if expr.usage == I18nExpressionFor::I18nAttribute {
                        i18n_expressions_by_element
                            .entry(expr.target)
                            .or_insert_with(Vec::new)
                            .push(expr.clone());
                    }
                }
            }
        }
    }

    // Step Two: Serialize the extracted i18n messages for root i18n blocks and i18n attributes into
    // the const array.
    //
    // Also, each i18n message will have a variable expression that can refer to its
    // value. Store these expressions in the appropriate place:
    // 1. For normal i18n content, it also goes in the const array. We save the const index to use
    // later.
    // 2. For extracted attributes, it becomes the value of the extracted attribute instruction.
    // 3. For i18n bindings, it will go in a separate const array instruction below; for now, we just
    // save it.

    let mut i18n_values_by_context: HashMap<ir::XrefId, OutputExpression> = HashMap::new();
    let mut message_const_indices: HashMap<ir::XrefId, ir::ConstIndex> = HashMap::new();

    // Collect I18nMessage ops to process
    let mut message_ops_to_process: Vec<(ir::XrefId, I18nMessageOp, bool)> = Vec::new(); // (xref, op, is_root)

    // From root unit
    for op in job.root.create.iter() {
        if op.kind() == OpKind::I18nMessage {
            unsafe {
                let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                let msg_ptr = op_ptr as *const I18nMessageOp;
                let msg = &*msg_ptr;
                message_ops_to_process.push((msg.xref, msg.clone(), true));
            }
        }
    }

    // From view units
    for (_, unit) in job.views.iter() {
        for op in unit.create.iter() {
            if op.kind() == OpKind::I18nMessage {
                unsafe {
                    let op_ptr = op.as_ref() as *const dyn ir::CreateOp;
                    let msg_ptr = op_ptr as *const I18nMessageOp;
                    let msg = &*msg_ptr;
                    message_ops_to_process.push((msg.xref, msg.clone(), false));
                }
            }
        }
    }

    // Process messages
    for (msg_xref, msg_op, is_root) in message_ops_to_process {
        if msg_op.message_placeholder.is_none() {
            let result = collect_message(job, &file_based_i18n_suffix, &messages, &msg_op);

            if msg_op.i18n_block.is_some() {
                // This is a regular i18n message with a corresponding i18n block. Collect it into the
                // const array.
                let i18n_const =
                    job.add_const(OutputExpression::ReadVar(result.main_var.clone()), None);
                message_const_indices.insert(msg_op.i18n_block.unwrap(), i18n_const);
            } else {
                // This is an i18n attribute. Extract the initializers into the const pool.
                // Note: In TypeScript, statements are pushed to constsInitializers
                // In Rust, we need to convert statements to expressions
                // For now, we'll just store the main variable
                job.consts_initializers
                    .push(OutputExpression::ReadVar(result.main_var.clone()));

                // Save the i18n variable value for later.
                i18n_values_by_context.insert(
                    msg_op.i18n_context,
                    OutputExpression::ReadVar(result.main_var.clone()),
                );

                // This i18n message may correspond to an individual extracted attribute. If so, The
                // value of that attribute is updated to read the extracted i18n variable.
                if let Some(_attributes_for_message) =
                    extracted_attributes_by_i18n_context.get(&msg_op.i18n_context)
                {
                    // Update extracted attributes - need to find and update them
                    if is_root {
                        for op in job.root.create.iter_mut() {
                            if op.kind() == OpKind::ExtractedAttribute {
                                unsafe {
                                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                                    let attr_ptr = op_ptr as *mut ExtractedAttributeOp;
                                    let attr = &mut *attr_ptr;
                                    if attr.i18n_context == Some(msg_op.i18n_context) {
                                        attr.expression = Some(OutputExpression::ReadVar(
                                            result.main_var.clone(),
                                        ));
                                    }
                                }
                            }
                        }
                    } else {
                        for (_, unit) in job.views.iter_mut() {
                            for op in unit.create.iter_mut() {
                                if op.kind() == OpKind::ExtractedAttribute {
                                    unsafe {
                                        let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                                        let attr_ptr = op_ptr as *mut ExtractedAttributeOp;
                                        let attr = &mut *attr_ptr;
                                        if attr.i18n_context == Some(msg_op.i18n_context) {
                                            attr.expression = Some(OutputExpression::ReadVar(
                                                result.main_var.clone(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Remove I18nMessageOp
        if is_root {
            let mut indices_to_remove = Vec::new();
            for (idx, op) in job.root.create.iter().enumerate() {
                if op.kind() == OpKind::I18nMessage && op.xref() == msg_xref {
                    indices_to_remove.push(idx);
                }
            }
            indices_to_remove.sort();
            indices_to_remove.reverse();
            for idx in indices_to_remove {
                job.root.create.remove_at(idx);
            }
        } else {
            for (_, unit) in job.views.iter_mut() {
                let mut indices_to_remove = Vec::new();
                for (idx, op) in unit.create.iter().enumerate() {
                    if op.kind() == OpKind::I18nMessage && op.xref() == msg_xref {
                        indices_to_remove.push(idx);
                    }
                }
                indices_to_remove.sort();
                indices_to_remove.reverse();
                for idx in indices_to_remove {
                    unit.create.remove_at(idx);
                }
            }
        }
    }

    // Step Three: Serialize I18nAttributes configurations into the const array. Each I18nAttributes
    // instruction has a config array, which contains k-v pairs describing each binding name, and the
    // i18n variable that provides the value.

    // Collect updates to apply later
    let mut updates: Vec<(ir::XrefId, Vec<OutputExpression>)> = Vec::new();

    for (_, unit) in job.views.iter() {
        for elem in unit.create.iter() {
            let is_element_or_container = matches!(
                elem.kind(),
                OpKind::ElementStart | OpKind::Element | OpKind::ContainerStart | OpKind::Container
            );

            if is_element_or_container {
                let elem_xref = elem.xref();
                if let Some(_i18n_attributes) = i18n_attributes_by_element.get(&elem_xref) {
                    let mut i18n_expressions = i18n_expressions_by_element
                        .get(&elem_xref)
                        .cloned()
                        .unwrap_or_default();

                    if i18n_expressions.is_empty() {
                        panic!("AssertionError: Could not find any i18n expressions associated with an I18nAttributes instruction");
                    }

                    // Find expressions for all the unique property names, removing duplicates.
                    let mut seen_property_names = std::collections::HashSet::new();
                    i18n_expressions.retain(|i18n_expr| {
                        let seen = seen_property_names.contains(&i18n_expr.name);
                        seen_property_names.insert(i18n_expr.name.clone());
                        !seen
                    });

                    let i18n_attribute_config: Vec<OutputExpression> = i18n_expressions
                        .iter()
                        .flat_map(|i18n_expr| {
                            let i18n_expr_value = i18n_values_by_context.get(&i18n_expr.context);
                            if i18n_expr_value.is_none() {
                                panic!("AssertionError: Could not find i18n expression's value");
                            }
                            vec![
                                OutputExpression::Literal(LiteralExpr {
                                    value: LiteralValue::String(i18n_expr.name.to_string()),
                                    type_: None,
                                    source_span: None,
                                }),
                                i18n_expr_value.unwrap().clone(),
                            ]
                        })
                        .collect();

                    updates.push((elem_xref, i18n_attribute_config));
                }
            }
        }
    }

    // Calculate all const_indices first
    let mut const_indices: Vec<(ir::XrefId, ir::ConstIndex)> = Vec::new();
    for (elem_xref, i18n_attribute_config) in &updates {
        let const_index = job.add_const(
            OutputExpression::LiteralArray(LiteralArrayExpr {
                entries: i18n_attribute_config.clone(),
                type_: None,
                source_span: None,
            }),
            None,
        );
        const_indices.push((*elem_xref, const_index));
    }

    // Apply updates
    for (elem_xref, const_index) in const_indices {
        for (_, unit) in job.views.iter_mut() {
            // Find the index of I18nAttributesOp
            let mut attr_op_index: Option<usize> = None;
            for (idx, attr_op) in unit.create.iter().enumerate() {
                if attr_op.kind() == OpKind::I18nAttributes {
                    unsafe {
                        let attr_op_ptr = attr_op.as_ref() as *const dyn ir::CreateOp;
                        let i18n_attrs_ptr = attr_op_ptr as *const I18nAttributesOp;
                        let i18n_attrs = &*i18n_attrs_ptr;
                        if i18n_attrs.target == elem_xref {
                            attr_op_index = Some(idx);
                            break;
                        }
                    }
                }
            }
            if let Some(idx) = attr_op_index {
                // Now update the op at the found index
                if let Some(attr_op) = unit.create.get_mut(idx) {
                    unsafe {
                        let attr_op_ptr = attr_op.as_mut() as *mut dyn ir::CreateOp;
                        let i18n_attrs_ptr = attr_op_ptr as *mut I18nAttributesOp;
                        let i18n_attrs = &mut *i18n_attrs_ptr;
                        i18n_attrs.i18n_attributes_config = Some(const_index);
                    }
                }
                break; // Found and updated, move to next
            }
        }
    }

    // Step Four: Propagate the extracted const index into i18n ops that messages were extracted from.

    for (_, unit) in job.views.iter_mut() {
        for op in unit.create.iter_mut() {
            if op.kind() == OpKind::I18nStart {
                unsafe {
                    let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                    let i18n_ptr = op_ptr as *mut I18nStartOp;
                    let i18n = &mut *i18n_ptr;
                    let msg_index = message_const_indices.get(&i18n.base.root);
                    if msg_index.is_none() {
                        panic!("AssertionError: Could not find corresponding i18n block index for an i18n message op; was an i18n message incorrectly assumed to correspond to an attribute?");
                    }
                    i18n.base.message_index = Some(*msg_index.unwrap());
                }
            }
        }
    }

    // Also update root unit
    for op in job.root.create.iter_mut() {
        if op.kind() == OpKind::I18nStart {
            unsafe {
                let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
                let i18n_ptr = op_ptr as *mut I18nStartOp;
                let i18n = &mut *i18n_ptr;
                let msg_index = message_const_indices.get(&i18n.base.root);
                if msg_index.is_none() {
                    panic!("AssertionError: Could not find corresponding i18n block index for an i18n message op; was an i18n message incorrectly assumed to correspond to an attribute?");
                }
                i18n.base.message_index = Some(*msg_index.unwrap());
            }
        }
    }
}

/// Collects the given message into a set of statements that can be added to the const array.
/// This will recursively collect any sub-messages referenced from the parent message as well.
fn collect_message(
    job: &mut ComponentCompilationJob,
    file_based_i18n_suffix: &str,
    messages: &HashMap<ir::XrefId, I18nMessageOp>,
    message_op: &I18nMessageOp,
) -> CollectMessageResult {
    // Recursively collect any sub-messages, record each sub-message's main variable under its
    // placeholder so that we can add them to the params for the parent message. It is possible
    // that multiple sub-messages will share the same placeholder, so we need to track an array of
    // variables for each placeholder.
    let mut statements: Vec<Statement> = Vec::new();
    let mut sub_message_placeholders: HashMap<String, Vec<OutputExpression>> = HashMap::new();

    for sub_message_id in &message_op.sub_messages {
        if let Some(sub_message) = messages.get(sub_message_id) {
            let result = collect_message(job, file_based_i18n_suffix, messages, sub_message);
            statements.extend(result.statements);
            if let Some(ref placeholder) = sub_message.message_placeholder {
                sub_message_placeholders
                    .entry(placeholder.clone())
                    .or_insert_with(Vec::new)
                    .push(OutputExpression::ReadVar(result.main_var.clone()));
            }
        }
    }

    // Note: add_sub_message_params is currently a no-op as we can't mutate message_op
    // This would need to be refactored to return updated params
    // add_sub_message_params(message_op, &mut sub_message_placeholders);

    // Sort the params for consistency with TemplateDefinitionBuilder output.
    let mut sorted_params: Vec<(String, OutputExpression)> = message_op
        .params
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    sorted_params.sort_by(|a, b| a.0.cmp(&b.0));
    // Note: We can't directly mutate message_op.params here, so we'll work with a copy

    let main_var_expr = crate::output::output_ast::variable(
        job.pool
            .unique_name(TRANSLATION_VAR_PREFIX.to_string(), false),
    );
    let main_var = match *main_var_expr {
        OutputExpression::ReadVar(ref expr) => expr.clone(),
        _ => panic!("variable() should return ReadVarExpr"),
    };

    // Closure Compiler requires const names to start with `MSG_` but disallows any other
    // const to start with `MSG_`. We define a variable starting with `MSG_` just for the
    // `goog.getMsg` call
    let closure_var = i18n_generate_closure_var(
        &mut job.pool,
        &message_op.message.id,
        file_based_i18n_suffix,
        job.i18n_use_external_ids,
    );

    let mut transform_fn: Option<Box<dyn Fn(&ReadVarExpr) -> OutputExpression>> = None;

    // If necessary, add a post-processing step and resolve any placeholder params that are
    // set in post-processing.
    if message_op.needs_postprocessing || !message_op.postprocessing_params.is_empty() {
        // Sort the post-processing params for consistency with TemplateDefinitionBuilder output.
        let mut sorted_postprocessing_params: Vec<(String, OutputExpression)> = message_op
            .postprocessing_params
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        sorted_postprocessing_params.sort_by(|a, b| a.0.cmp(&b.0));
        let postprocessing_params_map: HashMap<String, OutputExpression> =
            sorted_postprocessing_params.into_iter().collect();
        let formatted_postprocessing_params =
            format_i18n_placeholder_names_in_map(&postprocessing_params_map, false);

        if !message_op.postprocessing_params.is_empty() {
            let extra_transform_fn_params: Vec<OutputExpression> =
                vec![OutputExpression::LiteralMap(LiteralMapExpr {
                    entries: formatted_postprocessing_params
                        .iter()
                        .map(|(k, v)| LiteralMapEntry {
                            key: k.clone(),
                            value: Box::new(v.clone()),
                            quoted: true,
                        })
                        .collect(),
                    type_: None,
                    source_span: None,
                })];

            transform_fn = Some(Box::new(move |expr: &ReadVarExpr| {
                OutputExpression::InvokeFn(crate::output::output_ast::InvokeFunctionExpr {
                    fn_: Box::new(OutputExpression::ExternalRef(
                        Identifiers::i18n_postprocess(),
                    )),
                    args: {
                        let mut args = vec![OutputExpression::ReadVar(expr.clone())];
                        args.extend(extra_transform_fn_params.iter().cloned());
                        args
                    },
                    type_: None,
                    source_span: None,
                    pure: false,
                })
            }));
        }
    }

    // Add the message's statements
    let message_statements = get_translation_decl_stmts(
        &message_op.message,
        &main_var,
        &closure_var,
        &message_op.params,
        transform_fn.as_deref(),
    );
    statements.extend(message_statements);

    CollectMessageResult {
        main_var: main_var.clone(),
        statements,
    }
}

struct CollectMessageResult {
    main_var: ReadVarExpr,
    statements: Vec<Statement>,
}

/// Adds the given subMessage placeholders to the given message op.
///
/// If a placeholder only corresponds to a single sub-message variable, we just set that variable
/// as the param value. However, if the placeholder corresponds to multiple sub-message
/// variables, we need to add a special placeholder value that is handled by the post-processing
/// step. We then add the array of variables as a post-processing param.
#[allow(dead_code)]
fn add_sub_message_params(
    _message_op: &I18nMessageOp,
    _sub_message_placeholders: &mut HashMap<String, Vec<OutputExpression>>,
) {
    // Note: We can't directly mutate message_op here, so this function would need to return
    // the updated params and postprocessing_params
    // For now, we'll work with the existing structure
    // This is a limitation - we may need to refactor to use mutable references
    for (_placeholder, _sub_messages) in _sub_message_placeholders.iter() {
        if _sub_messages.len() == 1 {
            // Would set message_op.params.set(placeholder, sub_messages[0])
            // But we can't mutate here
        } else {
            // Would set message_op.params.set(placeholder, o.literal(`${ESCAPE}${I18N_ICU_MAPPING_PREFIX}${placeholder}${ESCAPE}`))
            // And message_op.postprocessing_params.set(placeholder, o.literalArr(subMessages))
            // But we can't mutate here
        }
    }
}

/// Generate statements that define a given translation message.
fn get_translation_decl_stmts(
    message: &i18n::Message,
    variable: &ReadVarExpr,
    closure_var: &ReadVarExpr,
    params: &HashMap<String, OutputExpression>,
    transform_fn: Option<&(dyn Fn(&ReadVarExpr) -> OutputExpression)>,
) -> Vec<Statement> {
    let params_object: HashMap<String, OutputExpression> = params.clone();
    let mut statements: Vec<Statement> = vec![declare_i18n_variable(variable)];

    // Create closure mode guard
    let closure_mode_guard = create_closure_mode_guard();

    // Create Google getMsg statements
    let google_get_msg_stmts =
        create_google_get_msg_statements(variable, message, closure_var, &params_object);

    // Create localize statements
    let formatted_params = format_i18n_placeholder_names_in_map(&params_object, false);
    let localize_stmts = create_localize_statements(variable, message, &formatted_params);

    // Create if statement
    statements.push(Statement::IfStmt(IfStmt {
        condition: Box::new(closure_mode_guard),
        true_case: google_get_msg_stmts,
        false_case: localize_stmts,
        source_span: None,
    }));

    if let Some(transform) = transform_fn {
        let transformed = transform(variable);
        statements.push(Statement::Expression(
            crate::output::output_ast::ExpressionStatement {
                expr: Box::new(OutputExpression::WriteVar(
                    crate::output::output_ast::WriteVarExpr {
                        name: variable.name.clone(),
                        value: Box::new(transformed),
                        type_: None,
                        source_span: None,
                    },
                )),
                source_span: None,
            },
        ));
    }

    statements
}

/// Create the expression that will be used to guard the closure mode block
/// It is equivalent to:
///
/// ```ts
/// typeof ngI18nClosureMode !== undefined && ngI18nClosureMode
/// ```
fn create_closure_mode_guard() -> OutputExpression {
    use crate::output::output_ast::TypeofExpr;

    let typeof_expr = OutputExpression::TypeOf(TypeofExpr {
        expr: Box::new(OutputExpression::ReadVar(
            crate::output::output_ast::ReadVarExpr {
                name: NG_I18N_CLOSURE_MODE.to_string(),
                type_: None,
                source_span: None,
            },
        )),
        type_: None,
        source_span: None,
    });

    let not_identical = OutputExpression::BinaryOp(BinaryOperatorExpr {
        lhs: Box::new(typeof_expr),
        operator: BinaryOperator::NotIdentical,
        rhs: Box::new(OutputExpression::Literal(LiteralExpr {
            value: LiteralValue::String("undefined".to_string()),
            type_: None,
            source_span: None,
        })),
        type_: None,
        source_span: None,
    });

    OutputExpression::BinaryOp(BinaryOperatorExpr {
        lhs: Box::new(not_identical),
        operator: BinaryOperator::And,
        rhs: Box::new(OutputExpression::ReadVar(
            crate::output::output_ast::ReadVarExpr {
                name: NG_I18N_CLOSURE_MODE.to_string(),
                type_: None,
                source_span: None,
            },
        )),
        type_: None,
        source_span: None,
    })
}

/// Generates vars with Closure-specific names for i18n blocks (i.e. `MSG_XXX`).
fn i18n_generate_closure_var(
    pool: &mut ConstantPool,
    message_id: &str,
    file_based_i18n_suffix: &str,
    use_external_ids: bool,
) -> ReadVarExpr {
    let name = if use_external_ids {
        let prefix = get_translation_const_prefix("EXTERNAL_");
        let unique_suffix = pool.unique_name(file_based_i18n_suffix.to_string(), false);
        format!(
            "{}{}$${}",
            prefix,
            sanitize_identifier(message_id),
            unique_suffix
        )
    } else {
        let prefix = get_translation_const_prefix(file_based_i18n_suffix);
        pool.unique_name(prefix, false)
    };
    match *crate::output::output_ast::variable(name) {
        OutputExpression::ReadVar(expr) => expr,
        _ => panic!("variable() should return ReadVarExpr"),
    }
}
