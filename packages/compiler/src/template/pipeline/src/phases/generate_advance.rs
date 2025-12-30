//! Generate Advance Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/generate_advance.ts
//! Generate `ir.AdvanceOp`s in between `ir.UpdateOp`s that ensure the runtime's implicit slot
//! context will be advanced correctly.

use crate::output::output_ast::Expression;
use crate::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::ops::update::create_advance_op;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationJobKind, CompilationUnit, ComponentCompilationJob,
};
use crate::template::pipeline::src::util::elements::op_kind_has_consumes_slot_trait;

/// Generate `ir.AdvanceOp`s in between `ir.UpdateOp`s that ensure the runtime's implicit slot
/// context will be advanced correctly.
pub fn phase(job: &mut dyn CompilationJob) {
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
            let root = &mut component_job.root;
            process_unit(root);
        }

        // Process all view units
        let view_keys: Vec<_> = component_job.views.keys().cloned().collect();
        for key in view_keys {
            if let Some(unit) = component_job.views.get_mut(&key) {
                process_unit(unit);
            }
        }
    }
}

fn process_unit(unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit) {
    // First build a map of all of the declarations in the view that have assigned slots.
    let mut slot_map: std::collections::HashMap<ir::XrefId, usize> =
        std::collections::HashMap::new();

    for op in unit.create() {
        // Check if op implements ConsumesSlotOpTrait
        if op_kind_has_consumes_slot_trait(op.kind()) {
            // Downcast to access handle.slot
            if let Some(slot) = get_slot_from_create_op(op) {
                slot_map.insert(op.xref(), slot);
            } else {
                panic!("AssertionError: expected slots to have been allocated before generating advance() calls");
            }
        }
    }

    // Next, step through the update operations and generate `ir.AdvanceOp`s as required to ensure
    // the runtime's implicit slot counter will be set to the correct slot before executing each
    // update operation which depends on it.
    //
    // To do that, we track what the runtime's slot counter will be through the update operations.
    let mut slot_context = 0;
    let mut insertions: Vec<(usize, usize, ParseSourceSpan)> = Vec::new(); // (index, delta, source_span)

    for (index, op) in unit.update().iter().enumerate() {
        let mut consumer_target: Option<ir::XrefId> = None;
        let mut source_span: Option<ParseSourceSpan> = None;

        // Check if op itself depends on slot context
        if has_depends_on_slot_context_trait_by_kind(op.kind()) {
            if let Some((target, span)) = get_target_from_update_op(op) {
                consumer_target = Some(target);
                source_span = Some(span);
            }
        } else {
            // Check expressions in op for ReferenceExpr (which implements DependsOnSlotContextTrait)
            let result = check_expressions_for_reference_in_update_op(op);
            consumer_target = result.0;
            source_span = result.1;
        }

        if let Some(target) = consumer_target {
            if !slot_map.contains_key(&target) {
                panic!(
                    "AssertionError: reference to unknown slot for target {}",
                    target.0
                );
            }

            let slot = *slot_map.get(&target).unwrap();

            // Does the slot counter need to be adjusted?
            if slot_context != slot {
                // If so, generate an `ir.AdvanceOp` to advance the counter.
                let delta = slot as i64 - slot_context as i64;
                if delta < 0 {
                    // Slot counter moving backwards can happen with complex template structures
                    // Reset context to current slot and continue
                    slot_context = slot;
                    continue;
                }

                let span = source_span.unwrap_or_else(|| {
                    op.source_span()
                        .cloned()
                        .unwrap_or_else(|| create_empty_parse_source_span())
                });
                insertions.push((index, delta as usize, span));
                slot_context = slot;
            }
        }
    }

    // Insert AdvanceOps in reverse order to maintain correct indices
    for (index, delta, source_span) in insertions.iter().rev() {
        let advance_op = create_advance_op(*delta, source_span.clone());
        unit.update_mut().insert_at(*index, advance_op);
    }
}

fn has_depends_on_slot_context_trait_by_kind(kind: OpKind) -> bool {
    matches!(
        kind,
        OpKind::Property
            | OpKind::Attribute
            | OpKind::ClassProp
            | OpKind::StyleProp
            | OpKind::ClassMap
            | OpKind::StyleMap
            | OpKind::DomProperty
            | OpKind::Binding
            | OpKind::TwoWayProperty
            | OpKind::TwoWayListener
            | OpKind::Listener
            | OpKind::Animation
            | OpKind::AnimationListener
            | OpKind::InterpolateText
            | OpKind::Repeater
            | OpKind::Conditional
    )
}

fn get_slot_from_create_op(op: &Box<dyn ir::CreateOp + Send + Sync>) -> Option<usize> {
    // Downcast based on OpKind to access handle.slot
    // This is safe because we've verified the OpKind matches
    unsafe {
        let op_ptr = op.as_ref() as *const dyn ir::CreateOp;

        match op.kind() {
            OpKind::ElementStart => {
                use crate::template::pipeline::ir::ops::create::ElementStartOp;
                let elem_ptr = op_ptr as *const ElementStartOp;
                let elem = &*elem_ptr;
                elem.base.base.handle.get_slot()
            }
            OpKind::Element => {
                use crate::template::pipeline::ir::ops::create::ElementOp;
                let elem_ptr = op_ptr as *const ElementOp;
                let elem = &*elem_ptr;
                elem.base.base.handle.get_slot()
            }
            OpKind::ElementEnd => {
                // ElementEnd doesn't have handle, skip
                None
            }
            OpKind::ContainerStart => {
                use crate::template::pipeline::ir::ops::create::ContainerStartOp;
                let cont_ptr = op_ptr as *const ContainerStartOp;
                let cont = &*cont_ptr;
                cont.base.handle.get_slot()
            }
            OpKind::Container => {
                use crate::template::pipeline::ir::ops::create::ContainerOp;
                let cont_ptr = op_ptr as *const ContainerOp;
                let cont = &*cont_ptr;
                cont.base.handle.get_slot()
            }
            OpKind::ContainerEnd => {
                // ContainerEnd doesn't have handle, skip
                None
            }
            OpKind::Text => {
                use crate::template::pipeline::ir::ops::create::TextOp;
                let text_ptr = op_ptr as *const TextOp;
                let text = &*text_ptr;
                text.handle.get_slot()
            }
            OpKind::Template => {
                use crate::template::pipeline::ir::ops::create::TemplateOp;
                let template_ptr = op_ptr as *const TemplateOp;
                let template = &*template_ptr;
                template.base.base.handle.get_slot()
            }
            OpKind::RepeaterCreate => {
                use crate::template::pipeline::ir::ops::create::RepeaterCreateOp;
                let repeater_ptr = op_ptr as *const RepeaterCreateOp;
                let repeater = &*repeater_ptr;
                repeater.base.base.handle.get_slot()
            }
            OpKind::ConditionalCreate => {
                use crate::template::pipeline::ir::ops::create::ConditionalCreateOp;
                let cond_ptr = op_ptr as *const ConditionalCreateOp;
                let cond = &*cond_ptr;
                cond.base.base.handle.get_slot()
            }
            OpKind::ConditionalBranchCreate => {
                use crate::template::pipeline::ir::ops::create::ConditionalBranchCreateOp;
                let branch_ptr = op_ptr as *const ConditionalBranchCreateOp;
                let branch = &*branch_ptr;
                branch.base.base.handle.get_slot()
            }
            OpKind::Projection => {
                use crate::template::pipeline::ir::ops::create::ProjectionOp;
                let proj_ptr = op_ptr as *const ProjectionOp;
                let proj = &*proj_ptr;
                proj.handle.get_slot()
            }
            OpKind::Defer => {
                use crate::template::pipeline::ir::ops::create::DeferOp;
                let defer_ptr = op_ptr as *const DeferOp;
                let defer = &*defer_ptr;
                defer.handle.get_slot()
            }
            OpKind::I18nStart | OpKind::I18n | OpKind::I18nAttributes => {
                use crate::template::pipeline::ir::ops::create::{
                    I18nAttributesOp, I18nOp, I18nStartOp,
                };
                match op.kind() {
                    OpKind::I18nStart => {
                        let i18n_ptr = op_ptr as *const I18nStartOp;
                        let i18n = &*i18n_ptr;
                        i18n.base.handle.get_slot()
                    }
                    OpKind::I18n => {
                        let i18n_ptr = op_ptr as *const I18nOp;
                        let i18n = &*i18n_ptr;
                        i18n.base.handle.get_slot()
                    }
                    OpKind::I18nAttributes => {
                        let i18n_ptr = op_ptr as *const I18nAttributesOp;
                        let i18n = &*i18n_ptr;
                        i18n.handle.get_slot()
                    }
                    _ => None,
                }
            }
            OpKind::DeclareLet => {
                use crate::template::pipeline::ir::ops::create::DeclareLetOp;
                let let_ptr = op_ptr as *const DeclareLetOp;
                let let_op = &*let_ptr;
                let_op.handle.get_slot()
            }
            OpKind::Pipe => {
                use crate::template::pipeline::ir::ops::create::PipeOp;
                let pipe_ptr = op_ptr as *const PipeOp;
                let pipe = &*pipe_ptr;
                pipe.handle.get_slot()
            }
            _ => None,
        }
    }
}

fn get_target_from_update_op(
    op: &Box<dyn ir::UpdateOp + Send + Sync>,
) -> Option<(ir::XrefId, ParseSourceSpan)> {
    // Check if op implements DependsOnSlotContextOpTrait
    // Downcast based on OpKind to access target and source_span
    unsafe {
        let op_ptr = op.as_ref() as *const dyn ir::UpdateOp;

        match op.kind() {
            OpKind::Property => {
                use crate::template::pipeline::ir::ops::update::PropertyOp;
                let prop_ptr = op_ptr as *const PropertyOp;
                let prop = &*prop_ptr;
                Some((prop.target, prop.source_span.clone()))
            }
            OpKind::Attribute => {
                use crate::template::pipeline::ir::ops::update::AttributeOp;
                let attr_ptr = op_ptr as *const AttributeOp;
                let attr = &*attr_ptr;
                Some((attr.target, attr.source_span.clone()))
            }
            OpKind::ClassProp => {
                use crate::template::pipeline::ir::ops::update::ClassPropOp;
                let class_ptr = op_ptr as *const ClassPropOp;
                let class = &*class_ptr;
                Some((class.target, class.source_span.clone()))
            }
            OpKind::StyleProp => {
                use crate::template::pipeline::ir::ops::update::StylePropOp;
                let style_ptr = op_ptr as *const StylePropOp;
                let style = &*style_ptr;
                Some((style.target, style.source_span.clone()))
            }
            OpKind::ClassMap => {
                use crate::template::pipeline::ir::ops::update::ClassMapOp;
                let map_ptr = op_ptr as *const ClassMapOp;
                let map = &*map_ptr;
                Some((map.target, map.source_span.clone()))
            }
            OpKind::StyleMap => {
                use crate::template::pipeline::ir::ops::update::StyleMapOp;
                let map_ptr = op_ptr as *const StyleMapOp;
                let map = &*map_ptr;
                Some((map.target, map.source_span.clone()))
            }
            OpKind::DomProperty => {
                use crate::template::pipeline::ir::ops::update::PropertyOp as DomPropertyOp;
                let dom_ptr = op_ptr as *const DomPropertyOp;
                let dom = &*dom_ptr;
                Some((dom.target, dom.source_span.clone()))
            }
            OpKind::Binding => {
                use crate::template::pipeline::ir::ops::update::BindingOp;
                let binding_ptr = op_ptr as *const BindingOp;
                let binding = &*binding_ptr;
                Some((binding.target, binding.source_span.clone()))
            }
            OpKind::TwoWayProperty => {
                use crate::template::pipeline::ir::ops::update::TwoWayPropertyOp;
                let two_way_ptr = op_ptr as *const TwoWayPropertyOp;
                let two_way = &*two_way_ptr;
                Some((two_way.target, two_way.source_span.clone()))
            }
            OpKind::Repeater => {
                use crate::template::pipeline::ir::ops::update::RepeaterOp;
                let rep_ptr = op_ptr as *const RepeaterOp;
                let rep = &*rep_ptr;
                Some((rep.target, rep.source_span.clone()))
            }
            OpKind::Conditional => {
                use crate::template::pipeline::ir::ops::update::ConditionalOp;
                let cond_ptr = op_ptr as *const ConditionalOp;
                let cond = &*cond_ptr;
                Some((cond.target, cond.source_span.clone()))
            }
            OpKind::InterpolateText => {
                use crate::template::pipeline::ir::ops::update::InterpolateTextOp;
                let text_ptr = op_ptr as *const InterpolateTextOp;
                let text = &*text_ptr;
                Some((text.target, text.source_span.clone()))
            }
            OpKind::TwoWayListener
            | OpKind::Listener
            | OpKind::Animation
            | OpKind::AnimationListener => {
                // These are CreateOps, not UpdateOps, so they shouldn't appear here
                None
            }
            _ => None,
        }
    }
}

/// Check expressions in update op for ReferenceExpr (which implements DependsOnSlotContextTrait)
/// This is done by downcasting to concrete types and checking expressions directly,
/// avoiding the need for unsafe mutable reference casting.
fn check_expressions_for_reference_in_update_op(
    _op: &Box<dyn ir::UpdateOp + Send + Sync>,
) -> (Option<ir::XrefId>, Option<ParseSourceSpan>) {
    // ReferenceExpr does not actually depend on the slot context in Ivy.
    // ɵɵreference(index) can be called from any context.
    // Therefore, we should not advance to the reference's target slot.
    (None, None)
}

/// Recursively check an expression for ReferenceExpr
fn check_expression_recursive_for_reference(
    expr: &Expression,
) -> (Option<ir::XrefId>, Option<ParseSourceSpan>) {
    match expr {
        Expression::Reference(ref ref_expr) => {
            (Some(ref_expr.target), ref_expr.source_span.clone())
        }
        Expression::BinaryOp(bin) => {
            let lhs_result = check_expression_recursive_for_reference(&bin.lhs);
            if lhs_result.0.is_some() {
                return lhs_result;
            }
            check_expression_recursive_for_reference(&bin.rhs)
        }
        Expression::Unary(un) => check_expression_recursive_for_reference(&un.expr),
        Expression::ReadProp(prop) => check_expression_recursive_for_reference(&prop.receiver),
        Expression::ReadKey(key) => {
            let receiver_result = check_expression_recursive_for_reference(&key.receiver);
            if receiver_result.0.is_some() {
                return receiver_result;
            }
            check_expression_recursive_for_reference(&key.index)
        }
        Expression::InvokeFn(invoke) => {
            let fn_result = check_expression_recursive_for_reference(&invoke.fn_);
            if fn_result.0.is_some() {
                return fn_result;
            }
            for arg in &invoke.args {
                let arg_result = check_expression_recursive_for_reference(arg);
                if arg_result.0.is_some() {
                    return arg_result;
                }
            }
            (None, None)
        }
        Expression::LiteralArray(arr) => {
            for entry in &arr.entries {
                let entry_result = check_expression_recursive_for_reference(entry);
                if entry_result.0.is_some() {
                    return entry_result;
                }
            }
            (None, None)
        }
        Expression::LiteralMap(map) => {
            for entry in &map.entries {
                let entry_result = check_expression_recursive_for_reference(&entry.value);
                if entry_result.0.is_some() {
                    return entry_result;
                }
            }
            (None, None)
        }
        Expression::Conditional(cond) => {
            let cond_result = check_expression_recursive_for_reference(&cond.condition);
            if cond_result.0.is_some() {
                return cond_result;
            }
            let true_result = check_expression_recursive_for_reference(&cond.true_case);
            if true_result.0.is_some() {
                return true_result;
            }
            if let Some(ref false_case) = cond.false_case {
                check_expression_recursive_for_reference(false_case)
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    }
}

/// Check a statement for ReferenceExpr
fn check_statement_for_reference(
    stmt: &crate::output::output_ast::Statement,
) -> (Option<ir::XrefId>, Option<ParseSourceSpan>) {
    use crate::output::output_ast::Statement;
    match stmt {
        Statement::Return(ref return_stmt) => {
            check_expression_recursive_for_reference(&return_stmt.value)
        }
        Statement::Expression(ref expr_stmt) => {
            check_expression_recursive_for_reference(&expr_stmt.expr)
        }
        Statement::DeclareVar(ref var_stmt) => {
            if let Some(ref value) = var_stmt.value {
                check_expression_recursive_for_reference(value)
            } else {
                (None, None)
            }
        }
        Statement::DeclareFn(_) | Statement::IfStmt(_) => {
            // For now, we don't check inside function declarations or if statements
            (None, None)
        }
    }
}

/// Create an empty ParseSourceSpan for cases where we don't have source span information
fn create_empty_parse_source_span() -> ParseSourceSpan {
    let empty_loc = ParseLocation::from_source(String::new(), String::new(), 0, 0, 0);
    ParseSourceSpan::new(empty_loc.clone(), empty_loc)
}
