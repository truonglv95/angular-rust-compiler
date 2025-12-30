//! Helper functions for host binding ingestion
//!
//! These functions are used to process host bindings (properties, attributes, events)
//! for components and directives.

use crate::core::SecurityContext;
use crate::output::output_ast::{Expression, ExpressionTrait};
use crate::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::ops::update::BindingExpression;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, HostBindingCompilationJob,
};
use crate::template::pipeline::src::conversion::convert_ast;
use crate::template::pipeline::src::ingest::uses_dollar_event;
use crate::template_parser::binding_parser::{ParsedEvent, ParsedProperty};
use std::sync::Arc;

/// Ingest a DOM property binding for host bindings
pub fn ingest_dom_property(
    job: &mut HostBindingCompilationJob,
    property: ParsedProperty,
    binding_kind: ir::BindingKind,
    security_contexts: Vec<SecurityContext>,
) {
    use crate::expression_parser::ast::AST as ExprAST;

    let root_xref = job.root_mut().xref();

    // Convert expression - handle interpolation if present
    let expression = match property.expression.ast.as_ref() {
        ExprAST::Interpolation(interp) => {
            // Convert interpolation to IR Interpolation
            let exprs: Vec<Expression> = interp
                .expressions
                .iter()
                .map(|expr| convert_ast(expr, job, root_xref, None, Some(&property.source_span)))
                .collect();

            // Create Interpolation expression
            BindingExpression::Interpolation(ir::ops::update::Interpolation {
                strings: interp
                    .strings
                    .iter()
                    .map(|s| Arc::from(s.clone()))
                    .collect(),
                expressions: exprs,
                i18n_placeholders: vec![], // Host bindings don't have i18n placeholders
            })
        }
        ast => {
            // Convert regular AST to Expression
            BindingExpression::Expression(convert_ast(
                ast,
                job,
                root_xref,
                None,
                Some(&property.source_span),
            ))
        }
    };

    // Create binding op
    let binding_op = ir::ops::update::create_binding_op(
        job.root_mut().xref(),
        binding_kind,
        property.name.into(),
        expression,
        None, // unit - encoded in name for host bindings
        security_contexts,
        false, // is_text_attr
        false, // is_structural_template_attribute
        None,  // template_kind
        None,  // i18n_message - host bindings don't handle i18n
        property.source_span,
    );

    job.root_mut().update_mut().push(binding_op);
}

/// Ingest a host attribute binding
pub fn ingest_host_attribute(
    job: &mut HostBindingCompilationJob,
    name: String,
    value: Expression,
    security_contexts: Vec<SecurityContext>,
) {
    // Host attributes should always be extracted to const hostAttrs
    // Create binding op with is_text_attr = true
    let binding_op = ir::ops::update::create_binding_op(
        job.root_mut().xref(),
        ir::BindingKind::Attribute,
        name.into(),
        BindingExpression::Expression(value.clone()),
        None, // unit
        security_contexts,
        true,  // is_text_attr - always true for host attributes
        false, // is_structural_template_attribute
        None,  // template_kind
        None,  // i18n_message
        value.source_span().cloned().unwrap_or_else(|| {
            ParseSourceSpan::new(
                ParseLocation::from_source(String::new(), String::new(), 0, 0, 0),
                ParseLocation::from_source(String::new(), String::new(), 0, 0, 0),
            )
        }),
    );

    job.root_mut().update_mut().push(binding_op);
}

/// Ingest a host event binding
pub fn ingest_host_event(job: &mut HostBindingCompilationJob, event: ParsedEvent) {
    use crate::template::pipeline::ir::handle::SlotHandle;
    use crate::template_parser::binding_parser::ParsedEventType;

    // Create handler ops
    let handler_ops = make_listener_handler_ops(job, &event.handler.ast, &event.handler_span);

    // Handle different event types similar to TypeScript implementation
    if event.type_ == ParsedEventType::Animation {
        // Determine animation kind based on event name
        let animation_kind = if event.name.ends_with("enter") {
            ir::enums::AnimationKind::Enter
        } else {
            ir::enums::AnimationKind::Leave
        };

        // Create animation listener op
        let animation_listener_op = ir::ops::create::create_animation_listener_op(
            job.root_mut().xref(),
            job.root_mut().xref(), // element
            SlotHandle::new(),
            event.name.into(),
            None, // tag - host listeners don't have tags
            handler_ops,
            animation_kind,
            event.target_or_phase.clone(), // event_target (target_or_phase for Animation)
            true,                          // host_listener
            event.source_span,
        );
        job.root_mut().create_mut().push(animation_listener_op);
    } else {
        // For Regular, TwoWay, and LegacyAnimation events
        // TypeScript handles them differently - for Regular and LegacyAnimation use create_listener_op
        // TwoWay is handled separately
        let (phase, target) = if event.type_ == ParsedEventType::LegacyAnimation {
            // For LegacyAnimation, target_or_phase contains the phase
            (event.target_or_phase.clone(), None)
        } else {
            // For Regular and TwoWay, target_or_phase is the target
            (None, event.target_or_phase.clone())
        };

        // For now, handle Regular and LegacyAnimation the same way
        // TwoWay events would need special handling (not shown in TypeScript host event code)
        // Note: uses_dollar_event is defined in ingest.rs - we can access it or inline logic here
        // For simplicity, assume dollar event is used for now (can be optimized later)
        let consumes_dollar_event = uses_dollar_event(&event.handler.ast);
        let listener_op = ir::ops::create::create_listener_op(
            job.root_mut().xref(),
            job.root_mut().xref(), // element
            SlotHandle::new(),
            event.name.into(),
            None, // tag
            handler_ops,
            phase,  // legacy_animation_phase (only for LegacyAnimation)
            target, // event_target (for Regular events)
            true,   // host_listener
            event.source_span,
            consumes_dollar_event,
        );
        job.root_mut().create_mut().push(listener_op);
    }
}

/// Helper function to convert event handler AST into UpdateOps for host bindings
fn make_listener_handler_ops(
    job: &mut HostBindingCompilationJob,
    handler: &crate::expression_parser::ast::AST,
    handler_span: &ParseSourceSpan,
) -> crate::template::pipeline::ir::operations::OpList<
    Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
> {
    use crate::output::output_ast::{ReturnStatement, Statement};
    use crate::template::pipeline::ir::ops::shared::create_statement_op;

    let mut handler_ops: crate::template::pipeline::ir::operations::OpList<
        Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
    > = crate::template::pipeline::ir::operations::OpList::new();

    // Convert handler AST to Expression
    let root_xref = job.root_mut().xref();
    let handler_expr = convert_ast(handler, job, root_xref, None, Some(handler_span));

    // Extract return expression if present
    // For simplicity, treat the handler as a return statement
    let return_stmt = ReturnStatement {
        value: Box::new(handler_expr),
        source_span: Some(handler_span.clone()),
    };
    let stmt = Statement::Return(return_stmt);
    let stmt_op = create_statement_op::<
        Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
    >(Box::new(stmt));
    handler_ops.push(Box::new(stmt_op));

    handler_ops
}
