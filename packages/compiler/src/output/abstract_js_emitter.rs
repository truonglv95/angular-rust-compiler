//! Abstract JavaScript Emitter Module
//!
//! Corresponds to packages/compiler/src/output/abstract_js_emitter.ts
//! JavaScript-specific emitter functionality

use crate::output::abstract_emitter::{
    escape_identifier, AbstractEmitterVisitor, EmitterVisitorContext, BINARY_OPERATORS,
};
use crate::output::output_ast as o;
use crate::output::output_ast::ExpressionTrait;
use std::any::Any;
use std::collections::HashMap;

/// Template object polyfill for tagged templates
#[allow(dead_code)]
const MAKE_TEMPLATE_OBJECT_POLYFILL: &str =
    "(this&&this.__makeTemplateObject||function(e,t){return Object.defineProperty?Object.defineProperty(e,\"raw\",{value:t}):e.raw=t,e})";

#[allow(dead_code)]
const SINGLE_QUOTE_ESCAPE_STRING_RE: &str = r"'|\\|\n|\r|\$";

/// Abstract JavaScript emitter visitor
pub struct AbstractJsEmitterVisitor {
    base: AbstractEmitterVisitor,
    imports: HashMap<String, String>,
}

impl AbstractJsEmitterVisitor {
    pub fn new() -> Self {
        AbstractJsEmitterVisitor {
            base: AbstractEmitterVisitor::new(false),
            imports: HashMap::new(),
        }
    }

    pub fn with_imports(imports: HashMap<String, String>) -> Self {
        AbstractJsEmitterVisitor {
            base: AbstractEmitterVisitor::new(false),
            imports,
        }
    }

    pub fn visit_all_statements(&mut self, stmts: &[o::Statement], ctx: &mut dyn Any) {
        for stmt in stmts {
            stmt.visit_statement(self, ctx);
        }
    }

    // ... (rest of impl)

    // ...
}

impl o::ExpressionVisitor for AbstractJsEmitterVisitor {
    fn visit_raw_code_expr(
        &mut self,
        expr: &o::RawCodeExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_raw_code_expr(expr, context)
    }

    fn visit_read_var_expr(
        &mut self,
        expr: &o::ReadVarExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_read_var_expr(expr, context)
    }

    fn visit_write_var_expr(
        &mut self,
        expr: &o::WriteVarExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            let name = escape_identifier(&expr.name, false, false);
            ctx.print(Some(expr), &name, false);
            ctx.print(Some(expr), " = ", false);
        }
        expr.value.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_write_key_expr(
        &mut self,
        expr: &o::WriteKeyExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        expr.receiver.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "[", false);
        }
        expr.index.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "] = ", false);
        }
        expr.value.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_write_prop_expr(
        &mut self,
        expr: &o::WritePropExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        expr.receiver.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ".", false);
            let name = escape_identifier(&expr.name, false, false);
            ctx.print(Some(expr), &name, false);
            ctx.print(Some(expr), " = ", false);
        }
        expr.value.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_invoke_function_expr(
        &mut self,
        expr: &o::InvokeFunctionExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        // Wrap function expressions that need parentheses due to operator precedence:
        // - ArrowFn and Fn: (function(){})() or (()=>{})()
        // - BinaryOp: (a || b)() - critical for inherited factory caching pattern
        // - Conditional: (a ? b : c)()
        match &*expr.fn_ {
            o::Expression::ArrowFn(_)
            | o::Expression::Fn(_)
            | o::Expression::BinaryOp(_)
            | o::Expression::Conditional(_) => {
                {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.print(Some(expr), "(", false);
                }
                expr.fn_.as_ref().visit_expression(self, context);
                {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.print(Some(expr), ")", false);
                }
            }
            _ => {
                expr.fn_.as_ref().visit_expression(self, context);
            }
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "(", false);
        }
        for (i, arg) in expr.args.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
            }
            arg.visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ")", false);
        }
        Box::new(())
    }

    fn visit_tagged_template_expr(
        &mut self,
        expr: &o::TaggedTemplateLiteralExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
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

    fn visit_instantiate_expr(
        &mut self,
        expr: &o::InstantiateExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "new ", false);
        }
        // Wrap class expression in parentheses if it's a binary expression or conditional
        // to ensure correct operator precedence: `new (a || b)()` not `new a || b()`
        let needs_parens = matches!(
            expr.class_expr.as_ref(),
            o::Expression::BinaryOp(_) | o::Expression::Conditional(_)
        );
        if needs_parens {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "(", false);
        }
        expr.class_expr.as_ref().visit_expression(self, context);
        if needs_parens {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ")", false);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "(", false);
        }
        for (i, arg) in expr.args.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
            }
            arg.visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ")", false);
        }
        Box::new(())
    }

    fn visit_literal_expr(&mut self, expr: &o::LiteralExpr, context: &mut dyn Any) -> Box<dyn Any> {
        self.base.visit_literal_expr(expr, context)
    }

    fn visit_localized_string(
        &mut self,
        expr: &o::LocalizedString,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_localized_string(expr, context)
    }

    fn visit_external_expr(
        &mut self,
        expr: &o::ExternalExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        let ref_expr = &expr.value;

        // Handle common Angular imports aliasing and configured imports
        if let Some(module_name) = &ref_expr.module_name {
            if module_name == "@angular/core" {
                ctx.print(Some(expr), "i0.", false);
            } else if let Some(alias) = self.imports.get(module_name) {
                ctx.print(Some(expr), alias, false);
                ctx.print(Some(expr), ".", false);
            } else {
                ctx.print(Some(expr), module_name, false);
                ctx.print(Some(expr), ".", false);
            }
        }

        if let Some(name) = &ref_expr.name {
            ctx.print(Some(expr), name, false);
        }
        Box::new(())
    }

    fn visit_binary_operator_expr(
        &mut self,
        expr: &o::BinaryOperatorExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        expr.lhs.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            if let Some(op_str) = BINARY_OPERATORS.get(&expr.operator) {
                ctx.print(Some(expr), " ", false);
                ctx.print(Some(expr), op_str, false);
                ctx.print(Some(expr), " ", false);
            }
        }
        expr.rhs.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_read_prop_expr(
        &mut self,
        expr: &o::ReadPropExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        expr.receiver.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ".", false);
            let name = escape_identifier(&expr.name, false, false);
            ctx.print(Some(expr), &name, false);
        }
        Box::new(())
    }

    fn visit_read_key_expr(
        &mut self,
        expr: &o::ReadKeyExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        expr.receiver.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "[", false);
        }
        expr.index.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "]", false);
        }
        Box::new(())
    }

    fn visit_conditional_expr(
        &mut self,
        expr: &o::ConditionalExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "(", false);
        }
        expr.condition.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), " ? ", false);
        }
        expr.true_case.as_ref().visit_expression(self, context);
        if let Some(false_case) = &expr.false_case {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                ctx.print(Some(expr), " : ", false);
            }
            false_case.as_ref().visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ")", false);
        }
        Box::new(())
    }

    fn visit_unary_operator_expr(
        &mut self,
        expr: &o::UnaryOperatorExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            let op_str = match expr.operator {
                o::UnaryOperator::Minus => "-",
                o::UnaryOperator::Plus => "+",
            };
            ctx.print(Some(expr), op_str, false);
        }
        expr.expr.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_parenthesized_expr(
        &mut self,
        expr: &o::ParenthesizedExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "(", false);
        }
        expr.expr.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ")", false);
        }
        Box::new(())
    }

    fn visit_function_expr(
        &mut self,
        expr: &o::FunctionExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            if let Some(name) = &expr.name {
                ctx.print(Some(expr), "function ", false);
                let func_name = escape_identifier(name, false, false);
                ctx.print(Some(expr), &func_name, false);
            } else {
                ctx.print(Some(expr), "function", false);
            }
            ctx.print(Some(expr), "(", false);
            for (i, param) in expr.params.iter().enumerate() {
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
                let param_name = escape_identifier(&param.name, false, false);
                ctx.print(Some(expr), &param_name, false);
            }
            ctx.println(Some(expr), ") {");
            ctx.inc_indent();
        }
        for statement in &expr.statements {
            statement.visit_statement(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.dec_indent();
            ctx.println(Some(expr), "}");
        }
        Box::new(())
    }

    fn visit_arrow_function_expr(
        &mut self,
        expr: &o::ArrowFunctionExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "(", false);
            for (i, param) in expr.params.iter().enumerate() {
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
                let param_name = escape_identifier(&param.name, false, false);
                ctx.print(Some(expr), &param_name, false);
            }
            ctx.print(Some(expr), ") => ", false);
        }
        match &expr.body {
            o::ArrowFunctionBody::Expression(e) => {
                let needs_parens = matches!(e.as_ref(), o::Expression::LiteralMap(_));
                if needs_parens {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.print(Some(expr), "(", false);
                }
                e.as_ref().visit_expression(self, context);
                if needs_parens {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.print(Some(expr), ")", false);
                }
            }
            o::ArrowFunctionBody::Statements(stmts) => {
                {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.println(Some(expr), "{");
                    ctx.inc_indent();
                }
                for statement in stmts {
                    statement.visit_statement(self, context);
                }
                {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.dec_indent();
                    ctx.println(Some(expr), "}");
                }
            }
        }
        Box::new(())
    }

    fn visit_literal_array_expr(
        &mut self,
        expr: &o::LiteralArrayExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "[", false);
        }
        for (i, entry) in expr.entries.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
            }
            entry.visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "]", false);
        }
        Box::new(())
    }

    fn visit_literal_map_expr(
        &mut self,
        expr: &o::LiteralMapExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "{", false);
        }
        for (i, entry) in expr.entries.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
                let key = if entry.quoted {
                    escape_identifier(&entry.key, true, true)
                } else {
                    escape_identifier(&entry.key, false, false)
                };
                ctx.print(Some(expr), &key, false);
                ctx.print(Some(expr), ": ", false);
            }
            entry.value.as_ref().visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "}", false);
        }
        Box::new(())
    }

    fn visit_comma_expr(&mut self, expr: &o::CommaExpr, context: &mut dyn Any) -> Box<dyn Any> {
        for (i, part) in expr.parts.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
            }
            part.visit_expression(self, context);
        }
        Box::new(())
    }

    fn visit_typeof_expr(&mut self, expr: &o::TypeofExpr, context: &mut dyn Any) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "typeof ", false);
        }
        expr.expr.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_void_expr(&mut self, expr: &o::VoidExpr, context: &mut dyn Any) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "void ", false);
        }
        expr.expr.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_not_expr(&mut self, expr: &o::NotExpr, context: &mut dyn Any) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "!", false);
        }
        expr.condition.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_if_null_expr(&mut self, expr: &o::IfNullExpr, context: &mut dyn Any) -> Box<dyn Any> {
        expr.condition.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), " ?? ", false);
        }
        expr.null_case.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_assert_not_null_expr(
        &mut self,
        expr: &o::AssertNotNullExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        expr.condition.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "!", false);
        }
        Box::new(())
    }

    fn visit_cast_expr(&mut self, expr: &o::CastExpr, context: &mut dyn Any) -> Box<dyn Any> {
        // JavaScript doesn't support casts, just emit value
        expr.value.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_dynamic_import_expr(
        &mut self,
        expr: &o::DynamicImportExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "import(", false);
            let url = escape_identifier(&expr.url, true, true);
            ctx.print(Some(expr), &url, false);
            ctx.print(Some(expr), ")", false);
        }
        Box::new(())
    }

    fn visit_template_literal_expr(
        &mut self,
        expr: &o::TemplateLiteralExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "`", false);
        }
        for (i, element) in expr.elements.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                ctx.print(Some(expr), &element.text, false);
            }
            if i < expr.expressions.len() {
                {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.print(Some(expr), "${", false);
                }
                expr.expressions[i].visit_expression(self, context);
                {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.print(Some(expr), "}", false);
                }
            }
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "`", false);
        }
        Box::new(())
    }

    fn visit_regular_expression_literal(
        &mut self,
        expr: &o::RegularExpressionLiteralExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        ctx.print(Some(expr), "/", false);
        ctx.print(Some(expr), &expr.pattern, false);
        ctx.print(Some(expr), "/", false);
        if !expr.flags.is_empty() {
            ctx.print(Some(expr), &expr.flags, false);
        }
        Box::new(())
    }

    fn visit_wrapped_node_expr(
        &mut self,
        expr: &o::WrappedNodeExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        if let Some(s) = expr.node.downcast_ref::<String>() {
            println!("DEBUG: visit_wrapped_node_expr found string: {}", s);
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(None, s, false);
            return Box::new(());
        }
        println!("DEBUG: visit_wrapped_node_expr failed to downcast");
        // WrappedNodeExpr should throw error in JavaScript emitter if not valid
        panic!(
            "Cannot emit a wrapped node expression as JavaScript code: {:?}",
            expr
        );
    }

    // IR Expression visitor methods - delegate to base emitter
    fn visit_lexical_read_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::LexicalReadExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_lexical_read_expr(expr, context)
    }

    fn visit_reference_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ReferenceExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_reference_expr(expr, context)
    }

    fn visit_context_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ContextExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_context_expr(expr, context)
    }

    fn visit_next_context_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::NextContextExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_next_context_expr(expr, context)
    }

    fn visit_get_current_view_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::GetCurrentViewExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_get_current_view_expr(expr, context)
    }

    fn visit_restore_view_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::RestoreViewExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_restore_view_expr(expr, context)
    }

    fn visit_reset_view_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ResetViewExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_reset_view_expr(expr, context)
    }

    fn visit_read_variable_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ReadVariableExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_read_variable_expr(expr, context)
    }

    fn visit_pure_function_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::PureFunctionExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_pure_function_expr(expr, context)
    }

    fn visit_pure_function_parameter_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::PureFunctionParameterExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_pure_function_parameter_expr(expr, context)
    }

    fn visit_pipe_binding_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::PipeBindingExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_pipe_binding_expr(expr, context)
    }

    fn visit_pipe_binding_variadic_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::PipeBindingVariadicExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_pipe_binding_variadic_expr(expr, context)
    }

    fn visit_safe_property_read_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SafePropertyReadExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_safe_property_read_expr(expr, context)
    }

    fn visit_safe_keyed_read_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SafeKeyedReadExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_safe_keyed_read_expr(expr, context)
    }

    fn visit_safe_invoke_function_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SafeInvokeFunctionExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_safe_invoke_function_expr(expr, context)
    }

    fn visit_safe_ternary_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SafeTernaryExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_safe_ternary_expr(expr, context)
    }

    fn visit_empty_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::EmptyExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_empty_expr(expr, context)
    }

    fn visit_assign_temporary_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::AssignTemporaryExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_assign_temporary_expr(expr, context)
    }

    fn visit_read_temporary_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ReadTemporaryExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_read_temporary_expr(expr, context)
    }

    fn visit_slot_literal_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SlotLiteralExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_slot_literal_expr(expr, context)
    }

    fn visit_conditional_case_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ConditionalCaseExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_conditional_case_expr(expr, context)
    }

    fn visit_const_collected_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ConstCollectedExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_const_collected_expr(expr, context)
    }

    fn visit_two_way_binding_set_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::TwoWayBindingSetExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_two_way_binding_set_expr(expr, context)
    }

    fn visit_context_let_reference_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ContextLetReferenceExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_context_let_reference_expr(expr, context)
    }

    fn visit_store_let_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::StoreLetExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_store_let_expr(expr, context)
    }

    fn visit_track_context_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::TrackContextExpr,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        self.base.visit_track_context_expr(expr, context)
    }
}

impl o::StatementVisitor for AbstractJsEmitterVisitor {
    fn visit_declare_var_stmt(
        &mut self,
        stmt: &o::DeclareVarStmt,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        // Use 'const' for Final modifier, otherwise 'var'
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            let keyword = match stmt.modifiers {
                o::StmtModifier::Final => "const ",
                _ => "var ",
            };
            ctx.print(Some(stmt), keyword, false);
            let name = escape_identifier(&stmt.name, false, false);
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

    fn visit_declare_function_stmt(
        &mut self,
        stmt: &o::DeclareFunctionStmt,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(stmt), "function ", false);
            let name = escape_identifier(&stmt.name, false, false);
            ctx.print(Some(stmt), &name, false);
            ctx.print(Some(stmt), "(", false);
            for (i, param) in stmt.params.iter().enumerate() {
                if i > 0 {
                    ctx.print(Some(stmt), ", ", false);
                }
                let param_name = escape_identifier(&param.name, false, false);
                ctx.print(Some(stmt), &param_name, false);
            }
            ctx.println(Some(stmt), ") {");
            ctx.inc_indent();
        }
        // Use self (JS emitter with aliasing) instead of base for inner statements
        for statement in &stmt.statements {
            statement.visit_statement(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.dec_indent();
            ctx.println(Some(stmt), "}");
        }
        Box::new(())
    }

    fn visit_expression_stmt(
        &mut self,
        stmt: &o::ExpressionStatement,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        stmt.expr.visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.println(Some(stmt), ";");
        }
        Box::new(())
    }

    fn visit_return_stmt(
        &mut self,
        stmt: &o::ReturnStatement,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(stmt), "return ", false);
        }
        stmt.value.visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.println(Some(stmt), ";");
        }
        Box::new(())
    }

    fn visit_if_stmt(&mut self, stmt: &o::IfStmt, context: &mut dyn Any) -> Box<dyn Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(stmt), "if (", false);
        }
        stmt.condition.visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(stmt), ") {", true);
            ctx.inc_indent();
        }
        self.visit_all_statements(&stmt.true_case, {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx
        });
        if !stmt.false_case.is_empty() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                ctx.dec_indent();
                ctx.print(Some(stmt), "} else {", true);
                ctx.inc_indent();
            }
            self.visit_all_statements(&stmt.false_case, {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                ctx
            });
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.dec_indent();
            ctx.println(Some(stmt), "}");
        }
        Box::new(())
    }
}

impl Default for AbstractJsEmitterVisitor {
    fn default() -> Self {
        Self::new()
    }
}
