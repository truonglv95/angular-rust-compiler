//! Abstract JavaScript Emitter Module
//!
//! Corresponds to packages/compiler/src/output/abstract_js_emitter.ts
//! JavaScript-specific emitter functionality

use crate::output::abstract_emitter::{AbstractEmitterVisitor, EmitterVisitorContext, escape_identifier};
use crate::output::output_ast as o;
use crate::output::output_ast::ExpressionTrait;
use std::any::Any;

/// Template object polyfill for tagged templates
const MAKE_TEMPLATE_OBJECT_POLYFILL: &str =
    "(this&&this.__makeTemplateObject||function(e,t){return Object.defineProperty?Object.defineProperty(e,\"raw\",{value:t}):e.raw=t,e})";

/// Abstract JavaScript emitter visitor
pub struct AbstractJsEmitterVisitor {
    base: AbstractEmitterVisitor,
}

impl AbstractJsEmitterVisitor {
    pub fn new() -> Self {
        AbstractJsEmitterVisitor {
            base: AbstractEmitterVisitor::new(false),
        }
    }

    // TODO: Implement JavaScript-specific visitor methods:
    // - visit_wrapped_node_expr (should throw error)
    // - visit_declare_var_stmt (use 'var' keyword)
    // - visit_tagged_template_literal_expr
    // - visit_template_literal_expr
    // - visit_template_literal_element_expr
    // - visit_function_expr
    // - visit_arrow_function_expr
    // - visit_declare_function_stmt
    // - visit_localized_string
    // etc.

    pub fn visit_params(&self, params: &[o::FnParam], ctx: &mut EmitterVisitorContext) {
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                ctx.print(None, ", ", false);
            }
            let param_name = escape_identifier(&param.name, false);
            ctx.print(None, &param_name, false);
        }
    }

    pub fn visit_all_statements(&mut self, statements: &[o::Statement], ctx: &mut EmitterVisitorContext) {
        let context: &mut dyn Any = ctx;
        for statement in statements {
            statement.visit_statement(self, context);
        }
    }
}

impl o::ExpressionVisitor for AbstractJsEmitterVisitor {
    fn visit_read_var_expr(&mut self, expr: &o::ReadVarExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_read_var_expr(expr, context)
    }

    fn visit_write_var_expr(&mut self, expr: &o::WriteVarExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_write_var_expr(expr, context)
    }

    fn visit_write_key_expr(&mut self, expr: &o::WriteKeyExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_write_key_expr(expr, context)
    }

    fn visit_write_prop_expr(&mut self, expr: &o::WritePropExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_write_prop_expr(expr, context)
    }

    fn visit_invoke_function_expr(&mut self, expr: &o::InvokeFunctionExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_invoke_function_expr(expr, context)
    }

    fn visit_tagged_template_expr(&mut self, expr: &o::TaggedTemplateLiteralExpr, context: &mut dyn Any) -> Box<dyn Any> {
        // Tagged template: tag`template`
        expr.tag.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(None, "`", false);
        }
        // Visit template elements and expressions
        for (i, element) in expr.template.elements.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                ctx.print(None, &element.text, false);
            }
            if i < expr.template.expressions.len() {
                {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.print(None, "${", false);
                }
                expr.template.expressions[i].visit_expression(self, context);
                {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.print(None, "}", false);
                }
            }
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(None, "`", false);
        }
        Box::new(())
    }

    fn visit_instantiate_expr(&mut self, expr: &o::InstantiateExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_instantiate_expr(expr, context)
    }

    fn visit_literal_expr(&mut self, expr: &o::LiteralExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_literal_expr(expr, context)
    }

    fn visit_localized_string(&mut self, expr: &o::LocalizedString, context: &mut dyn Any) -> Box<dyn Any> {
        // TODO: Implement localized string for JavaScript
        self.base.visit_localized_string(expr, context)
    }

    fn visit_external_expr(&mut self, expr: &o::ExternalExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_external_expr(expr, context)
    }

    fn visit_binary_operator_expr(&mut self, expr: &o::BinaryOperatorExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_binary_operator_expr(expr, context)
    }

    fn visit_read_prop_expr(&mut self, expr: &o::ReadPropExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_read_prop_expr(expr, context)
    }

    fn visit_read_key_expr(&mut self, expr: &o::ReadKeyExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_read_key_expr(expr, context)
    }

    fn visit_conditional_expr(&mut self, expr: &o::ConditionalExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_conditional_expr(expr, context)
    }

    fn visit_unary_operator_expr(&mut self, expr: &o::UnaryOperatorExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_unary_operator_expr(expr, context)
    }

    fn visit_function_expr(&mut self, expr: &o::FunctionExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_function_expr(expr, context)
    }

    fn visit_arrow_function_expr(&mut self, expr: &o::ArrowFunctionExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_arrow_function_expr(expr, context)
    }

    fn visit_literal_array_expr(&mut self, expr: &o::LiteralArrayExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_literal_array_expr(expr, context)
    }

    fn visit_literal_map_expr(&mut self, expr: &o::LiteralMapExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_literal_map_expr(expr, context)
    }

    fn visit_comma_expr(&mut self, expr: &o::CommaExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_comma_expr(expr, context)
    }

    fn visit_typeof_expr(&mut self, expr: &o::TypeofExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_typeof_expr(expr, context)
    }

    fn visit_void_expr(&mut self, expr: &o::VoidExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_void_expr(expr, context)
    }

    fn visit_not_expr(&mut self, expr: &o::NotExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_not_expr(expr, context)
    }

    fn visit_if_null_expr(&mut self, expr: &o::IfNullExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_if_null_expr(expr, context)
    }

    fn visit_assert_not_null_expr(&mut self, expr: &o::AssertNotNullExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_assert_not_null_expr(expr, context)
    }

    fn visit_cast_expr(&mut self, expr: &o::CastExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_cast_expr(expr, context)
    }

    fn visit_dynamic_import_expr(&mut self, expr: &o::DynamicImportExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_dynamic_import_expr(expr, context)
    }

    fn visit_template_literal_expr(&mut self, expr: &o::TemplateLiteralExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_template_literal_expr(expr, context)
    }

    fn visit_wrapped_node_expr(&mut self, _expr: &o::WrappedNodeExpr, _context: &mut dyn Any) -> Box<dyn Any> {
        // WrappedNodeExpr should throw error in JavaScript emitter
        panic!("Cannot emit a wrapped node expression as JavaScript code");
    }
}

impl o::StatementVisitor for AbstractJsEmitterVisitor {
    fn visit_declare_var_stmt(&mut self, stmt: &o::DeclareVarStmt, context: &mut dyn Any) -> Box<dyn Any> {
        // Use 'var' keyword for JavaScript (base uses 'var' too, but we override for clarity)
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(stmt), "var ", false);
            let name = escape_identifier(&stmt.name, false);
            ctx.print(Some(stmt), &name, false);
            if let Some(_value) = &stmt.value {
                ctx.print(Some(stmt), " = ", false);
            }
        }
        if let Some(value) = &stmt.value {
            value.as_ref().visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.println(Some(stmt), ";");
        }
        Box::new(())
    }

    fn visit_declare_function_stmt(&mut self, stmt: &o::DeclareFunctionStmt, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_declare_function_stmt(stmt, context)
    }

    fn visit_expression_stmt(&mut self, stmt: &o::ExpressionStatement, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_expression_stmt(stmt, context)
    }

    fn visit_return_stmt(&mut self, stmt: &o::ReturnStatement, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_return_stmt(stmt, context)
    }

    fn visit_if_stmt(&mut self, stmt: &o::IfStmt, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_if_stmt(stmt, context)
    }
}

impl Default for AbstractJsEmitterVisitor {
    fn default() -> Self {
        Self::new()
    }
}





