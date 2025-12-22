//! Extract pure functions into shared constants.
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/pure_function_extraction.ts
//!
//! This phase transforms PureFunctionExpr by:
//! 1. Taking the body expression
//! 2. Creating a shared constant definition (PureFunctionConstant)
//! 3. Requesting a shared constant from the pool (hoisting it to _c0, _c1, etc.)
//! 4. Replacing the body with the reference to the shared constant
//!
//! Now that unsafe code in variable_optimization is fixed, we can safely use the constant pool.

use crate::constant_pool::SharedConstantDefinition;
use crate::output::output_ast as o;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::IRExpression;
use crate::template::pipeline::src::compilation::ComponentCompilationJob;

struct PureFunctionConstant {
    num_args: usize,
}

impl SharedConstantDefinition for PureFunctionConstant {
    fn key_of(&self, expr: &o::Expression) -> String {
        // Include num_args in key to differentiate functions with same body but different arg counts (unlikely but possible)
        format!("pure_fn_{} args_{:?}", self.num_args, expr)
    }

    fn to_shared_constant_declaration(&self, name: String, expr: o::Expression) -> o::Statement {
        // Create parameters a0, a1, ...
        let mut fn_params = Vec::new();
        for idx in 0..self.num_args {
            fn_params.push(o::FnParam {
                name: format!("a{}", idx),
                type_: None,
            });
        }
        
        // Return expression: replace PureFunctionParameterExpr with variable reads (a0, a1...)
        let return_expr = ir::transform_expressions_in_expression(
            expr,
            &mut |e, _f| {
                if let Some(ir_e) = ir::as_ir_expression(&e) {
                    if let IRExpression::PureFunctionParameter(param) = ir_e {
                        return *o::variable(format!("a{}", param.index));
                    }
                }
                e.clone()
            },
            ir::VisitorContextFlag::NONE,
        );

        // Create arrow function: (a0, a1) => return_expr
        let arrow_fn = o::arrow_fn(
            fn_params,
            o::ArrowFunctionBody::Expression(Box::new(return_expr)),
            None,
        );
        
        // Declare var: const _c0 = (a0, a1) => ...
        o::Statement::DeclareVar(o::DeclareVarStmt {
            name,
            value: Some(Box::new(*arrow_fn)),
            type_: None,
            modifiers: o::StmtModifier::Final,
            source_span: None,
        })
    }
}

pub fn phase(job: &mut ComponentCompilationJob) {
    extract_pure_functions(job);
}

pub fn extract_pure_functions(job: &mut ComponentCompilationJob) {
    // Split borrow of job to access pool and units simultaneously
    let pool = &mut job.pool;
    
    // Process root view
    process_view(pool, &mut job.root);
    
    // Process child views
    for view in job.views.values_mut() {
        process_view(pool, view);
    }
}

fn process_view(pool: &mut crate::constant_pool::ConstantPool, view: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit) {
    // Transform create ops
    for op in view.create.iter_mut() {
        ir::transform_expressions_in_op(
            op.as_mut(),
            &mut |expr, flags| transform_pure_function(pool, expr, flags),
            ir::VisitorContextFlag::NONE
        );
    }
    
    // Transform update ops
    for op in view.update.iter_mut() {
        ir::transform_expressions_in_op(
            op.as_mut(),
            &mut |expr, flags| transform_pure_function(pool, expr, flags),
            ir::VisitorContextFlag::NONE
        );
    }
}

fn transform_pure_function(
    pool: &mut crate::constant_pool::ConstantPool, 
    expr: o::Expression, 
    _flags: ir::VisitorContextFlag
) -> o::Expression {
    if let o::Expression::PureFunction(mut pure_fn) = expr {
        if let Some(body) = pure_fn.body.take() {
            if pure_fn.fn_.is_none() {
                let constant_def = Box::new(PureFunctionConstant {
                    num_args: pure_fn.args.len(),
                });
                
                // Hoist to constant pool!
                // pool.get_shared_constant will use to_shared_constant_declaration to create the arrow function
                // and return a reference to the variable (e.g. _c0)
                let hoisted_ref = pool.get_shared_constant(constant_def, *body);
                
                pure_fn.fn_ = Some(Box::new(hoisted_ref));
            }
        }
        return o::Expression::PureFunction(pure_fn);
    }
    expr
}
