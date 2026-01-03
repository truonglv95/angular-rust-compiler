//! Render3 Query Generation
//!
//! Corresponds to packages/compiler/src/render3/view/query_generation.ts
//! Contains query generation for view and content queries

use crate::constant_pool::ConstantPool;
use crate::core::RenderFlags;
use crate::output::output_ast::{
    BinaryOperator, BinaryOperatorExpr, Expression, ExpressionStatement, ExternalExpr,
    ExternalReference, FnParam, FunctionExpr, IfStmt, InvokeFunctionExpr, LiteralArrayExpr,
    LiteralExpr, LiteralValue, ReadPropExpr, ReadVarExpr, Statement, WritePropExpr, WriteVarExpr,
};
use crate::render3::r3_identifiers::Identifiers as R3;
use crate::render3::util::ForwardRefHandling;

use super::api::R3QueryMetadata;
use super::util::{CONTEXT_NAME, RENDER_FLAGS, TEMPORARY_NAME};

/// A set of flags to be used with Queries.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryFlags {
    /// No flags
    None = 0b0000,
    /// Whether or not the query should descend into children.
    Descendants = 0b0001,
    /// The query can be computed statically.
    IsStatic = 0b0010,
    /// If the `QueryList` should fire change event only if actual change was computed.
    EmitDistinctChangesOnly = 0b0100,
}

/// Translates query flags into `TQueryFlags` type.
fn to_query_flags(query: &R3QueryMetadata) -> u32 {
    let mut flags = QueryFlags::None as u32;

    if query.descendants {
        flags |= QueryFlags::Descendants as u32;
    }
    if query.static_ {
        flags |= QueryFlags::IsStatic as u32;
    }
    if query.emit_distinct_changes_only {
        flags |= QueryFlags::EmitDistinctChangesOnly as u32;
    }

    flags
}

/// Helper to create literal expression
fn literal(value: LiteralValue) -> Expression {
    Expression::Literal(LiteralExpr {
        value,
        type_: None,
        source_span: None,
    })
}

/// Helper to create read var expression
fn read_var(name: &str) -> Expression {
    Expression::ReadVar(ReadVarExpr {
        name: name.to_string(),
        type_: None,
        source_span: None,
    })
}

/// Helper to create external expression
fn external_expr(reference: ExternalReference) -> Expression {
    Expression::External(ExternalExpr {
        value: reference,
        type_: None,
        source_span: None,
    })
}

/// Helper to create invoke function expression
fn invoke_fn(fn_expr: Expression, args: Vec<Expression>) -> Expression {
    Expression::InvokeFn(InvokeFunctionExpr {
        fn_: Box::new(fn_expr),
        args,
        type_: None,
        source_span: None,
        pure: false,
    })
}

/// Get the query predicate expression.
pub fn get_query_predicate(
    query: &R3QueryMetadata,
    _constant_pool: &mut ConstantPool,
) -> Expression {
    match &query.predicate {
        crate::render3::view::api::R3QueryPredicate::Selectors(selectors) => {
            let mut predicates: Vec<Expression> = vec![];
            for selector in selectors {
                for token in selector.split(',') {
                    predicates.push(literal(LiteralValue::String(token.trim().to_string())));
                }
            }
            // TODO: Integrate with ConstantPool once Expression types are unified
            // For now, return the array directly without pooling
            Expression::LiteralArray(LiteralArrayExpr {
                entries: predicates,
                type_: None,
                source_span: None,
            })
        }
        crate::render3::view::api::R3QueryPredicate::Expression(maybe_forward_ref) => {
            match maybe_forward_ref.forward_ref {
                ForwardRefHandling::None | ForwardRefHandling::Unwrapped => {
                    maybe_forward_ref.expression.clone()
                }
                ForwardRefHandling::Wrapped => invoke_fn(
                    external_expr(R3::resolve_forward_ref()),
                    vec![maybe_forward_ref.expression.clone()],
                ),
            }
        }
    }
}

/// Query type function references for signal and non-signal based queries.
#[derive(Debug, Clone)]
pub struct QueryTypeFns {
    pub signal_based: ExternalReference,
    pub non_signal: ExternalReference,
}

/// Create a query creation call expression.
pub fn create_query_create_call(
    query: &R3QueryMetadata,
    constant_pool: &mut ConstantPool,
    query_type_fns: &QueryTypeFns,
    prepend_params: Option<Vec<Expression>>,
) -> Expression {
    let mut parameters: Vec<Expression> = vec![];

    if let Some(prepend) = prepend_params {
        parameters.extend(prepend);
    }

    if query.is_signal {
        let ctx_prop = Expression::ReadProp(ReadPropExpr {
            receiver: Box::new(read_var(CONTEXT_NAME)),
            name: query.property_name.clone(),
            type_: None,
            source_span: None,
        });
        parameters.push(ctx_prop);
    }

    parameters.push(get_query_predicate(query, constant_pool));
    parameters.push(literal(LiteralValue::Number(to_query_flags(query) as f64)));

    if let Some(ref read) = query.read {
        parameters.push(read.clone());
    }

    let query_create_fn = if query.is_signal {
        &query_type_fns.signal_based
    } else {
        &query_type_fns.non_signal
    };

    invoke_fn(external_expr(query_create_fn.clone()), parameters)
}

/// Create a render flag check if statement.
fn render_flag_check_if_stmt(flags: RenderFlags, statements: Vec<Statement>) -> Statement {
    let rf_var = read_var(RENDER_FLAGS);
    let flags_literal = literal(LiteralValue::Number(flags as u32 as f64));
    let condition = Expression::BinaryOp(BinaryOperatorExpr {
        operator: BinaryOperator::BitwiseAnd,
        lhs: Box::new(rf_var),
        rhs: Box::new(flags_literal),
        type_: None,
        source_span: None,
    });

    Statement::IfStmt(IfStmt {
        condition: Box::new(condition),
        true_case: statements,
        false_case: vec![],
        source_span: None,
    })
}

/// Helper to create expression statement
fn expr_stmt(expr: Expression) -> Statement {
    Statement::Expression(ExpressionStatement {
        expr: Box::new(expr),
        source_span: None,
    })
}

/// Define and update any view queries.
pub fn create_view_queries_function(
    view_queries: &[R3QueryMetadata],
    constant_pool: &mut ConstantPool,
    name: Option<&str>,
) -> Expression {
    let mut create_statements: Vec<Statement> = vec![];
    let mut update_statements: Vec<Statement> = vec![];

    for query in view_queries {
        // Creation call
        let query_definition_call = create_query_create_call(
            query,
            constant_pool,
            &QueryTypeFns {
                signal_based: R3::view_query_signal(),
                non_signal: R3::view_query(),
            },
            None,
        );
        create_statements.push(expr_stmt(query_definition_call));

        // Signal queries update lazily
        if query.is_signal {
            update_statements.push(expr_stmt(invoke_fn(
                external_expr(R3::query_advance()),
                vec![],
            )));
            continue;
        }

        // Non-signal query update
        let temporary = read_var(TEMPORARY_NAME);
        let get_query_list = invoke_fn(external_expr(R3::load_query()), vec![]);

        let write_tmp = Expression::WriteVar(WriteVarExpr {
            name: TEMPORARY_NAME.to_string(),
            value: Box::new(get_query_list),
            type_: None,
            source_span: None,
        });

        let refresh = invoke_fn(external_expr(R3::query_refresh()), vec![write_tmp]);

        let update_value: Expression = if query.first {
            Expression::ReadProp(ReadPropExpr {
                receiver: Box::new(temporary.clone()),
                name: "first".to_string(),
                type_: None,
                source_span: None,
            })
        } else {
            temporary.clone()
        };

        let update_directive = Expression::WriteProp(WritePropExpr {
            receiver: Box::new(read_var(CONTEXT_NAME)),
            name: query.property_name.clone(),
            value: Box::new(update_value),
            type_: None,
            source_span: None,
        });

        let and_expr = Expression::BinaryOp(BinaryOperatorExpr {
            operator: BinaryOperator::And,
            lhs: Box::new(refresh),
            rhs: Box::new(update_directive),
            type_: None,
            source_span: None,
        });

        update_statements.push(expr_stmt(and_expr));
    }

    let view_query_fn_name = name.map(|n| format!("{}_Query", n));

    // Add `var _t;` declaration if there are non-signal queries
    let has_non_signal_queries = view_queries.iter().any(|q| !q.is_signal);
    let mut statements = vec![];
    if has_non_signal_queries {
        statements.push(Statement::DeclareVar(
            crate::output::output_ast::DeclareVarStmt {
                name: TEMPORARY_NAME.to_string(),
                value: None,
                type_: None,
                modifiers: crate::output::output_ast::StmtModifier::None,
                source_span: None,
            },
        ));
    }
    statements.push(render_flag_check_if_stmt(
        RenderFlags::Create,
        create_statements,
    ));
    statements.push(render_flag_check_if_stmt(
        RenderFlags::Update,
        update_statements,
    ));

    Expression::Fn(FunctionExpr {
        params: vec![
            FnParam {
                name: RENDER_FLAGS.to_string(),
                type_: None,
            },
            FnParam {
                name: CONTEXT_NAME.to_string(),
                type_: None,
            },
        ],
        statements,
        type_: None,
        source_span: None,
        name: view_query_fn_name,
    })
}

/// Define and update any content queries.
pub fn create_content_queries_function(
    queries: &[R3QueryMetadata],
    constant_pool: &mut ConstantPool,
    name: Option<&str>,
) -> Expression {
    let mut create_statements: Vec<Statement> = vec![];
    let mut update_statements: Vec<Statement> = vec![];

    for query in queries {
        // Creation call with dirIndex prepended
        let query_definition_call = create_query_create_call(
            query,
            constant_pool,
            &QueryTypeFns {
                signal_based: R3::content_query_signal(),
                non_signal: R3::content_query(),
            },
            Some(vec![read_var("dirIndex")]),
        );
        create_statements.push(expr_stmt(query_definition_call));

        // Signal queries update lazily
        if query.is_signal {
            update_statements.push(expr_stmt(invoke_fn(
                external_expr(R3::query_advance()),
                vec![],
            )));
            continue;
        }

        // Non-signal query update
        let temporary = read_var(TEMPORARY_NAME);
        let get_query_list = invoke_fn(external_expr(R3::load_query()), vec![]);

        let write_tmp = Expression::WriteVar(WriteVarExpr {
            name: TEMPORARY_NAME.to_string(),
            value: Box::new(get_query_list),
            type_: None,
            source_span: None,
        });

        let refresh = invoke_fn(external_expr(R3::query_refresh()), vec![write_tmp]);

        let update_value: Expression = if query.first {
            Expression::ReadProp(ReadPropExpr {
                receiver: Box::new(temporary.clone()),
                name: "first".to_string(),
                type_: None,
                source_span: None,
            })
        } else {
            temporary.clone()
        };

        let update_directive = Expression::WriteProp(WritePropExpr {
            receiver: Box::new(read_var(CONTEXT_NAME)),
            name: query.property_name.clone(),
            value: Box::new(update_value),
            type_: None,
            source_span: None,
        });

        let and_expr = Expression::BinaryOp(BinaryOperatorExpr {
            operator: BinaryOperator::And,
            lhs: Box::new(refresh),
            rhs: Box::new(update_directive),
            type_: None,
            source_span: None,
        });

        update_statements.push(expr_stmt(and_expr));
    }

    let content_queries_fn_name = name.map(|n| format!("{}_ContentQueries", n));

    // Add `var _t;` declaration if there are non-signal queries
    let has_non_signal_queries = queries.iter().any(|q| !q.is_signal);
    let mut statements = vec![];
    if has_non_signal_queries {
        statements.push(Statement::DeclareVar(
            crate::output::output_ast::DeclareVarStmt {
                name: TEMPORARY_NAME.to_string(),
                value: None,
                type_: None,
                modifiers: crate::output::output_ast::StmtModifier::None,
                source_span: None,
            },
        ));
    }
    statements.push(render_flag_check_if_stmt(
        RenderFlags::Create,
        create_statements,
    ));
    statements.push(render_flag_check_if_stmt(
        RenderFlags::Update,
        update_statements,
    ));

    Expression::Fn(FunctionExpr {
        params: vec![
            FnParam {
                name: RENDER_FLAGS.to_string(),
                type_: None,
            },
            FnParam {
                name: CONTEXT_NAME.to_string(),
                type_: None,
            },
            FnParam {
                name: "dirIndex".to_string(),
                type_: None,
            },
        ],
        statements,
        type_: None,
        source_span: None,
        name: content_queries_fn_name,
    })
}
