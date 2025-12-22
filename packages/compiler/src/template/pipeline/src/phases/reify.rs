//! Compiles semantic operations across all views and generates output Statements.
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/reify.ts

use crate::output::output_ast as o;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::operations::{CreateOp, UpdateOp};
use crate::template::pipeline::src::compilation::{
    CompilationJob, ComponentCompilationJob, CompilationUnit,
};
use crate::template::pipeline::src::instruction as ng;

pub fn reify(job: &mut dyn CompilationJob) {
    if let Some(component_job) = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        if job.kind() == crate::template::pipeline::src::compilation::CompilationJobKind::Tmpl {
             Some(&mut *(job_ptr as *mut ComponentCompilationJob))
        } else {
            None
        }
    } {
        reify_unit(&mut component_job.root);
        for unit in component_job.views.values_mut() {
            reify_unit(unit);
        }
    }
}

fn reify_unit(unit: &mut dyn CompilationUnit) {
    reify_create_operations(unit);
    reify_update_operations(unit);
}

fn reify_create_operations(unit: &mut dyn CompilationUnit) {
    for op in unit.create_mut().iter_mut() {
        ir::transform_expressions_in_op(op.as_mut(), &mut reify_ir_expression, ir::VisitorContextFlag::NONE);
        
        let new_op: Option<Box<dyn CreateOp + Send + Sync>> = match op.kind() {
            ir::OpKind::Text => {
                if let Some(text_op) = op.as_any().downcast_ref::<ir::ops::create::TextOp>() {
                   if let Some(slot) = text_op.handle.slot {
                        let stmt = ng::text(slot as i32, text_op.initial_value.clone(), text_op.source_span.clone());
                        Some(Box::new(ir::ops::shared::create_statement_op::<Box<dyn CreateOp + Send + Sync>>(Box::new(stmt))))
                   } else { None }
                } else { None }
            }
            ir::OpKind::Element | ir::OpKind::ElementStart => {
                if let Some(el_op) = op.as_any().downcast_ref::<ir::ops::create::ElementStartOp>() {
                   if let Some(slot) = el_op.base.base.handle.slot {
                       let const_index = el_op.base.base.attributes.map(|idx| idx.as_usize() as i32);
                       let local_ref_index = el_op.base.base.local_refs_index.map(|idx| idx.as_usize() as i32);
                       let tag = el_op.base.tag.clone().unwrap_or_default();
                       let stmt = ng::element_start(slot as i32, tag, const_index, local_ref_index, el_op.base.base.start_source_span.clone());
                       Some(Box::new(ir::ops::shared::create_statement_op::<Box<dyn CreateOp + Send + Sync>>(Box::new(stmt))))
                   } else { None }
                } else if let Some(el_op) = op.as_any().downcast_ref::<ir::ops::create::ElementOp>() {
                   if let Some(slot) = el_op.base.base.handle.slot {
                       let const_index = el_op.base.base.attributes.map(|idx| idx.as_usize() as i32);
                       let local_ref_index = el_op.base.base.local_refs_index.map(|idx| idx.as_usize() as i32);
                       let tag = el_op.base.tag.clone().unwrap_or_default();
                       let stmt = ng::element(slot as i32, tag, const_index, local_ref_index, el_op.base.base.start_source_span.clone());
                       Some(Box::new(ir::ops::shared::create_statement_op::<Box<dyn CreateOp + Send + Sync>>(Box::new(stmt))))
                   } else { None }
                } else {
                    None
                }
            }
            ir::OpKind::ElementEnd => {
                 let stmt = ng::element_end(op.source_span().cloned());
                 Some(Box::new(ir::ops::shared::create_statement_op::<Box<dyn CreateOp + Send + Sync>>(Box::new(stmt))))
            }
            ir::OpKind::Pipe => {
                 if let Some(pipe_op) = op.as_any().downcast_ref::<ir::ops::create::CreatePipeOp>() {
                     if let Some(slot) = pipe_op.handle.slot {
                         let stmt = ng::pipe(slot as i32, pipe_op.name.clone());
                          Some(Box::new(ir::ops::shared::create_statement_op::<Box<dyn CreateOp + Send + Sync>>(Box::new(stmt))))
                     } else { None }
                 } else { None }
            }
             ir::OpKind::DisableBindings => {
                 let stmt = ng::disable_bindings();
                 Some(Box::new(ir::ops::shared::create_statement_op::<Box<dyn CreateOp + Send + Sync>>(Box::new(stmt))))
             },
             ir::OpKind::EnableBindings => {
                 let stmt = ng::enable_bindings();
                 Some(Box::new(ir::ops::shared::create_statement_op::<Box<dyn CreateOp + Send + Sync>>(Box::new(stmt))))
             },
            _ => None
        };
        
        if let Some(new) = new_op {
             *op = new;
        }
    }
}

fn reify_update_operations(unit: &mut dyn CompilationUnit) {
     for op in unit.update_mut().iter_mut() {
        ir::transform_expressions_in_op(op.as_mut(), &mut reify_ir_expression, ir::VisitorContextFlag::NONE);
        
        let new_op: Option<Box<dyn UpdateOp + Send + Sync>> = match op.kind() {
            ir::OpKind::Advance => {
                 if let Some(adv) = op.as_any().downcast_ref::<ir::ops::update::AdvanceOp>() {
                     let stmt = ng::advance(adv.delta as i32, adv.source_span.clone());
                     Some(Box::new(ir::ops::shared::create_statement_op::<Box<dyn UpdateOp + Send + Sync>>(Box::new(stmt))))
                 } else { None }
            }
            ir::OpKind::Property => {
                 if let Some(prop) = op.as_any().downcast_ref::<ir::ops::update::PropertyOp>() {
                     if let ir::ops::update::BindingExpression::Expression(expr) = &prop.expression {
                         let stmt = ng::property(prop.name.clone(), expr.clone(), prop.sanitizer.clone(), prop.source_span.clone());
                         Some(Box::new(ir::ops::shared::create_statement_op::<Box<dyn UpdateOp + Send + Sync>>(Box::new(stmt))))
                     } else { None }
                 } else { None }
            }
            ir::OpKind::Variable => {
                use crate::template::pipeline::ir::ops::shared::VariableOp;
                use crate::template::pipeline::ir::SemanticVariable;
                if let Some(var_op) = op.as_any().downcast_ref::<VariableOp<Box<dyn UpdateOp + Send + Sync>>>() {
                    // Get variable name from SemanticVariable
                    let var_name = match &var_op.variable {
                        SemanticVariable::Identifier(ident_var) => ident_var.name.clone(),
                        SemanticVariable::Context(ctx_var) => ctx_var.name.clone(),
                        SemanticVariable::Alias(_) => None, // Aliases are always inlined, shouldn't reach here
                        SemanticVariable::SavedView(_) => None, // SavedView variables don't need declarations here
                    };
                    
                    if let Some(name) = var_name {
                        // Reify initializer expression before using it
                        let reified_initializer = reify_ir_expression(*var_op.initializer.clone(), ir::VisitorContextFlag::NONE);
                        
                        // Convert VariableOp to DeclareVarStmt
                        let stmt = o::Statement::DeclareVar(o::DeclareVarStmt {
                            name,
                            value: Some(Box::new(reified_initializer)),
                            type_: None,
                            modifiers: o::StmtModifier::Final,
                            source_span: op.source_span().cloned(),
                        });
                        Some(Box::new(ir::ops::shared::create_statement_op::<Box<dyn UpdateOp + Send + Sync>>(Box::new(stmt))))
                    } else {
                        // Variable name not yet assigned - this should not happen after naming phase
                        None
                    }
                } else { None }
            }
             _ => None
        };

        if let Some(new) = new_op {
            *op = new;
        }
    }
}

fn reify_ir_expression(expr: o::Expression, flags: ir::VisitorContextFlag) -> o::Expression {
    match expr {
        o::Expression::LiteralArray(arr) => {
             return o::Expression::LiteralArray(o::LiteralArrayExpr {
                entries: arr.entries.into_iter().map(|e| reify_ir_expression(e, flags)).collect(),
                type_: arr.type_,
                source_span: arr.source_span,
            });
        }
        o::Expression::LiteralMap(map) => {
             return o::Expression::LiteralMap(o::LiteralMapExpr {
                entries: map.entries.into_iter().map(|e| o::LiteralMapEntry {
                    key: e.key,
                    value: Box::new(reify_ir_expression(*e.value, flags)),
                    quoted: e.quoted,
                }).collect(),
                type_: map.type_,
                source_span: map.source_span,
            });
        }
        o::Expression::BinaryOp(op) => {
             return o::Expression::BinaryOp(o::BinaryOperatorExpr {
                lhs: Box::new(reify_ir_expression(*op.lhs, flags)),
                rhs: Box::new(reify_ir_expression(*op.rhs, flags)),
                operator: op.operator,
                type_: op.type_,
                source_span: op.source_span,
            });
        }
        o::Expression::InvokeFn(func) => {
             return o::Expression::InvokeFn(o::InvokeFunctionExpr {
                fn_: Box::new(reify_ir_expression(*func.fn_, flags)),
                args: func.args.into_iter().map(|a| reify_ir_expression(a, flags)).collect(),
                type_: func.type_,
                source_span: func.source_span,
                pure: func.pure,
            });
        }
        o::Expression::ReadProp(prop) => {
            return o::Expression::ReadProp(o::ReadPropExpr {
                receiver: Box::new(reify_ir_expression(*prop.receiver, flags)),
                name: prop.name,
                type_: prop.type_,
                source_span: prop.source_span,
            });
        }
        o::Expression::Conditional(cond) => {
            return o::Expression::Conditional(o::ConditionalExpr {
                condition: Box::new(reify_ir_expression(*cond.condition, flags)),
                true_case: Box::new(reify_ir_expression(*cond.true_case, flags)),
                false_case: cond.false_case.map(|f| Box::new(reify_ir_expression(*f, flags))),
                type_: cond.type_,
                source_span: cond.source_span,
            });
        }
        o::Expression::NotExpr(not) => {
             return o::Expression::NotExpr(o::NotExpr {
                condition: Box::new(reify_ir_expression(*not.condition, flags)),
                source_span: not.source_span,
            });
        }
        o::Expression::Parens(paren) => {
             return o::Expression::Parens(o::ParenthesizedExpr {
                expr: Box::new(reify_ir_expression(*paren.expr, flags)),
                type_: paren.type_,
                source_span: paren.source_span,
            });
        }
        _ => {}
    }

    if !ir::is_ir_expression(&expr) {
        return expr;
    }
    
    match &expr {
        o::Expression::PureFunctionParameter(param) => {
             o::Expression::ReadVar(o::ReadVarExpr {
                 name: format!("_p{}", param.index),
                 type_: None,
                 source_span: param.source_span.clone(),
             })
        }
        o::Expression::PureFunction(pf) => {
            let func = if let Some(fn_) = &pf.fn_ {
                reify_ir_expression(*fn_.clone(), flags)
            } else if let Some(body) = &pf.body {
                 let params: Vec<o::FnParam> = (0..pf.args.len()).map(|i| o::FnParam {
                    name: format!("_p{}", i),
                    type_: None,
                }).collect();
                
                 let reified_body = reify_ir_expression(*body.clone(), flags);
                 
                 o::Expression::ArrowFn(o::ArrowFunctionExpr {
                    params,
                    body: o::ArrowFunctionBody::Expression(Box::new(reified_body)),
                    type_: None,
                    source_span: None,
                })
            } else {
                *o::literal(o::LiteralValue::Null)
            };
            
            let args: Vec<o::Expression> = pf.args.iter().map(|arg| reify_ir_expression(arg.clone(), flags)).collect();
            let slot = pf.var_offset.unwrap_or(0) as i32;
            ng::pure_function(slot, func, args)
        }
        o::Expression::ReadVariable(var) => {
             if let Some(name) = &var.name {
                 o::Expression::ReadVar(o::ReadVarExpr {
                     name: name.clone(),
                     type_: None,
                     source_span: var.source_span.clone(),
                 })
             } else {
                 println!("PANIC: ReadVariableExpr without name! Xref: {:?}", var.xref);
                 panic!("ReadVariableExpr without name: {:?}", var);
             }
        }
        o::Expression::ReadVar(expr) => {
            o::Expression::ReadVar(expr.clone())
        }
        o::Expression::NextContext(next_ctx) => {
            // Reify NextContextExpr to ng.nextContext(steps) expression
            // If steps == 1, call with empty args, otherwise pass steps as argument
            let args = if next_ctx.steps == 1 {
                vec![]
            } else {
                vec![*o::literal(next_ctx.steps as f64)]
            };
            *o::import_ref(crate::render3::r3_identifiers::Identifiers::next_context())
                .call_fn(args, next_ctx.source_span.clone(), None)
        }
        _ => expr
    }
}
