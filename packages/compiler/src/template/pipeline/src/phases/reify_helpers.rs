
fn create_restore_view_stmt(rv: &o::RestoreViewExpr) -> Box<dyn ir::UpdateOp + Send + Sync> {
    use crate::template::pipeline::src::phases::reify::reify_ir_expression;
    use crate::output::output_ast as o;
    use crate::template::pipeline::ir;
    
    // Reify the view argument
    let view_arg = match &rv.view {
        ir::expression::EitherXrefIdOrExpression::XrefId(xref) => {
            o::Expression::ReadVar(o::ReadVarExpr {
                name: format!("_r{}", xref.0),
                type_: None,
                source_span: None,
            })
        }
        ir::expression::EitherXrefIdOrExpression::Expression(expr) => {
            reify_ir_expression(*expr.clone(), ir::VisitorContextFlag::NONE)
        }
    };
    
    // Create RestoreView generic call
    let restore_call = *o::import_ref(crate::render3::r3_identifiers::Identifiers::restore_view()).call_fn(
        vec![view_arg],
        None,
        None,
    );

    // Replace with StatementOp(ExpressionStatement)
    let stmt = o::Statement::Expression(o::ExpressionStatement {
        expr: Box::new(restore_call),
        source_span: None,
    });
    
    Box::new(ir::ops::shared::create_statement_op::<
        Box<dyn ir::UpdateOp + Send + Sync>,
    >(Box::new(stmt)))
}

fn create_var_decl_update(var_op: &crate::template::pipeline::ir::ops::shared::VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>) -> Option<Box<dyn ir::UpdateOp + Send + Sync>> {
    use crate::template::pipeline::src::phases::reify::reify_ir_expression;
    use crate::output::output_ast as o;
    use crate::template::pipeline::ir;
    use crate::template::pipeline::ir::SemanticVariable;

    let var_name = match &var_op.variable {
        SemanticVariable::Identifier(ident_var) => {
            ident_var.name.clone()
        }
        SemanticVariable::Context(ctx_var) => ctx_var.name.clone(),
        SemanticVariable::Alias(_) => None,
        SemanticVariable::SavedView(_) => None,
    };

    if let Some(name) = var_name {
        let reified_initializer = reify_ir_expression(
            *var_op.initializer.clone(),
            ir::VisitorContextFlag::NONE,
        );

        let stmt = o::Statement::DeclareVar(o::DeclareVarStmt {
            name: name.clone(),
            value: Some(Box::new(reified_initializer)),
            type_: None,
            modifiers: o::StmtModifier::Final,
            source_span: None,
        });

        Some(Box::new(ir::ops::shared::create_statement_op::<
            Box<dyn ir::UpdateOp + Send + Sync>,
        >(Box::new(stmt))))
    } else {
        None
    }
}

fn create_var_decl_create(var_op: &crate::template::pipeline::ir::ops::shared::VariableOp<Box<dyn ir::CreateOp + Send + Sync>>) -> Option<Box<dyn ir::UpdateOp + Send + Sync>> {
    use crate::template::pipeline::src::phases::reify::reify_ir_expression;
    use crate::output::output_ast as o;
    use crate::template::pipeline::ir;
    use crate::template::pipeline::ir::SemanticVariable;

    let var_name = match &var_op.variable {
        SemanticVariable::Identifier(ident_var) => {
            ident_var.name.clone()
        }
        SemanticVariable::Context(ctx_var) => ctx_var.name.clone(),
        SemanticVariable::Alias(_) => None,
        SemanticVariable::SavedView(_) => None,
    };

    if let Some(name) = var_name {
        let reified_initializer = reify_ir_expression(
            *var_op.initializer.clone(),
            ir::VisitorContextFlag::NONE,
        );

        let stmt = o::Statement::DeclareVar(o::DeclareVarStmt {
            name: name.clone(),
            value: Some(Box::new(reified_initializer)),
            type_: None,
            modifiers: o::StmtModifier::Final,
            source_span: None,
        });

        Some(Box::new(ir::ops::shared::create_statement_op::<
            Box<dyn ir::UpdateOp + Send + Sync>,
        >(Box::new(stmt))))
    } else {
        None
    }
}
