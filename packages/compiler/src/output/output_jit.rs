//! Output JIT Module
//!
//! Corresponds to packages/compiler/src/output/output_jit.ts
//! JIT compilation and evaluation support

use crate::output::abstract_emitter::EmitterVisitorContext;
use crate::output::abstract_js_emitter::AbstractJsEmitterVisitor;
use crate::output::output_ast as o;
use std::collections::HashMap;

/// External reference resolver trait
pub trait ExternalReferenceResolver {
    fn resolve_external_reference(
        &self,
        reference: &o::ExternalReference,
    ) -> Box<dyn std::any::Any>;
}

/// A helper class to manage the evaluation of JIT generated code
pub struct JitEvaluator;

impl JitEvaluator {
    pub fn new() -> Self {
        JitEvaluator
    }

    /// Evaluate Angular statements
    ///
    /// # Arguments
    /// * `source_url` - The URL of the generated code
    /// * `statements` - An array of Angular statement AST nodes to be evaluated
    /// * `ref_resolver` - Resolves `ExternalReference`s into values
    /// * `create_source_maps` - If true then create a source-map for the generated code
    ///
    /// # Returns
    /// A map of all the variables in the generated code
    pub fn evaluate_statements(
        &self,
        source_url: &str,
        statements: &[o::Statement],
        ref_resolver: &dyn ExternalReferenceResolver,
        create_source_maps: bool,
    ) -> HashMap<String, Box<dyn std::any::Any>> {
        let mut converter = JitEmitterVisitor::new(ref_resolver);
        let mut ctx = EmitterVisitorContext::create_root();

        converter.visit_all_statements(statements, &mut ctx);
        converter.create_return_stmt(&mut ctx);

        self.evaluate_code(source_url, &ctx, &converter.get_args(), create_source_maps)
    }

    /// Evaluate a piece of JIT generated code
    pub fn evaluate_code(
        &self,
        source_url: &str,
        ctx: &EmitterVisitorContext,
        vars: &HashMap<String, Box<dyn std::any::Any>>,
        create_source_map: bool,
    ) -> HashMap<String, Box<dyn std::any::Any>> {
        let _fn_body = format!(
            "\"use strict\";{}\n//# sourceURL={}",
            ctx.to_source(),
            source_url
        );

        let mut fn_arg_names: Vec<String> = Vec::new();

        for (arg_name, _arg_value) in vars {
            // Note: Cannot clone Box<dyn Any>, so we skip adding values
            // This is a placeholder implementation - actual JIT execution would need
            // a different approach (e.g., using Rc/Arc or changing the API)
            fn_arg_names.push(arg_name.clone());
        }

        if create_source_map {
            // TODO: Generate source map
            // For now, just add placeholder
        }

        // TODO: Actually execute the function if we had a JS engine
        // Since we are in Rust without an embedded JS engine, we return an empty map.
        // In a real scenario, we would compile fn_body and execute it with fn_arg_values.

        HashMap::new()
    }

    /// Execute a JIT generated function by calling it
    ///
    /// This method can be overridden in tests to capture the functions that are generated
    pub fn execute_function(
        &self,
        _fn: &str,
        _args: &[Box<dyn std::any::Any>],
    ) -> Box<dyn std::any::Any> {
        // TODO: Implement function execution
        // This would require JavaScript runtime integration
        Box::new(())
    }
}

impl Default for JitEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// An Angular AST visitor that converts AST nodes into executable JavaScript code
pub struct JitEmitterVisitor<'a> {
    base: AbstractJsEmitterVisitor,
    eval_arg_names: Vec<String>,
    eval_arg_values: Vec<Box<dyn std::any::Any>>,
    _eval_exported_vars: Vec<String>,
    _ref_resolver: &'a dyn ExternalReferenceResolver,
}

impl<'a> JitEmitterVisitor<'a> {
    pub fn new(ref_resolver: &'a dyn ExternalReferenceResolver) -> Self {
        JitEmitterVisitor {
            base: AbstractJsEmitterVisitor::new(),
            eval_arg_names: Vec::new(),
            eval_arg_values: Vec::new(),
            _eval_exported_vars: Vec::new(),
            _ref_resolver: ref_resolver,
        }
    }

    pub fn create_return_stmt(&mut self, ctx: &mut EmitterVisitorContext) {
        if self._eval_exported_vars.is_empty() {
            return;
        }

        ctx.print(None, "return {", false);
        for (i, export_name) in self._eval_exported_vars.iter().enumerate() {
            if i > 0 {
                ctx.print(None, ",", false);
            }
            ctx.print(None, &format!("{}: {}", export_name, export_name), false);
        }
        ctx.print(None, "};", true);
    }

    pub fn visit_all_statements(
        &mut self,
        statements: &[o::Statement],
        ctx: &mut EmitterVisitorContext,
    ) {
        self.base.visit_all_statements(statements, ctx);
    }

    pub fn get_args(&self) -> HashMap<String, Box<dyn std::any::Any>> {
        // Note: Cannot clone Box<dyn Any>, so we return empty HashMap
        // In full implementation, we would transfer ownership or share references
        HashMap::new()
    }

    pub fn visit_external_expr(&mut self, expr: &o::ExternalExpr, ctx: &mut EmitterVisitorContext) {
        let value = self._ref_resolver.resolve_external_reference(&expr.value);
        let ast = o::Expression::External(expr.clone());
        self._emit_reference_to_external(&ast, value, ctx);
    }

    pub fn visit_wrapped_node_expr(
        &mut self,
        _expr: &o::WrappedNodeExpr,
        _ctx: &mut EmitterVisitorContext,
    ) {
        // TODO: Handle wrapped node expression
        // Current issue: Cannot clone Box<dyn Any> from AST to evaluated args
        // This requires changing WrappedNodeExpr to use Rc<dyn Any> or similar
    }

    fn _emit_reference_to_external(
        &mut self,
        _ast: &o::Expression,
        value: Box<dyn std::any::Any>,
        ctx: &mut EmitterVisitorContext,
    ) {
        let mut id = None;
        // Simple linear search (pointer comparison not possible easily with Box<dyn Any>)
        // We rely on simple deduplication if possible, or just append.

        if id.is_none() {
            id = Some(self.eval_arg_values.len());
            self.eval_arg_values.push(value);
            let name = "val"; // TODO: Use actual name from metadata if available
            self.eval_arg_names
                .push(format!("jit_{}_{}", name, id.unwrap()));
        }

        ctx.print(None, &self.eval_arg_names[id.unwrap()], false);
    }
}

impl<'a> o::StatementVisitor for JitEmitterVisitor<'a> {
    fn visit_declare_var_stmt(
        &mut self,
        stmt: &o::DeclareVarStmt,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        if stmt.modifiers == o::StmtModifier::Exported {
            self._eval_exported_vars.push(stmt.name.clone());
        }
        self.base.visit_declare_var_stmt(stmt, context)
    }

    fn visit_declare_function_stmt(
        &mut self,
        stmt: &o::DeclareFunctionStmt,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        if stmt.modifiers == o::StmtModifier::Exported {
            self._eval_exported_vars.push(stmt.name.clone());
        }
        self.base.visit_declare_function_stmt(stmt, context)
    }

    fn visit_expression_stmt(
        &mut self,
        stmt: &o::ExpressionStatement,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_expression_stmt(stmt, context)
    }

    fn visit_return_stmt(
        &mut self,
        stmt: &o::ReturnStatement,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_return_stmt(stmt, context)
    }

    fn visit_if_stmt(
        &mut self,
        stmt: &o::IfStmt,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_if_stmt(stmt, context)
    }

    fn visit_block_stmt(
        &mut self,
        stmt: &o::BlockStmt,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_block_stmt(stmt, context)
    }
}

impl<'a> o::ExpressionVisitor for JitEmitterVisitor<'a> {
    fn visit_raw_code_expr(
        &mut self,
        expr: &o::RawCodeExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_raw_code_expr(expr, context)
    }
    fn visit_read_var_expr(
        &mut self,
        expr: &o::ReadVarExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_read_var_expr(expr, context)
    }

    fn visit_write_var_expr(
        &mut self,
        expr: &o::WriteVarExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_write_var_expr(expr, context)
    }

    fn visit_write_key_expr(
        &mut self,
        expr: &o::WriteKeyExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_write_key_expr(expr, context)
    }

    fn visit_write_prop_expr(
        &mut self,
        expr: &o::WritePropExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_write_prop_expr(expr, context)
    }

    fn visit_invoke_function_expr(
        &mut self,
        expr: &o::InvokeFunctionExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_invoke_function_expr(expr, context)
    }

    fn visit_tagged_template_expr(
        &mut self,
        expr: &o::TaggedTemplateLiteralExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_tagged_template_expr(expr, context)
    }

    fn visit_instantiate_expr(
        &mut self,
        expr: &o::InstantiateExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_instantiate_expr(expr, context)
    }

    fn visit_literal_expr(
        &mut self,
        expr: &o::LiteralExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_literal_expr(expr, context)
    }

    fn visit_localized_string(
        &mut self,
        expr: &o::LocalizedString,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_localized_string(expr, context)
    }

    fn visit_external_expr(
        &mut self,
        expr: &o::ExternalExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        // Custom handling for JIT
        self.visit_external_expr(expr, ctx);
        Box::new(())
    }

    fn visit_binary_operator_expr(
        &mut self,
        expr: &o::BinaryOperatorExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_binary_operator_expr(expr, context)
    }

    fn visit_read_prop_expr(
        &mut self,
        expr: &o::ReadPropExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_read_prop_expr(expr, context)
    }

    fn visit_read_key_expr(
        &mut self,
        expr: &o::ReadKeyExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_read_key_expr(expr, context)
    }

    fn visit_conditional_expr(
        &mut self,
        expr: &o::ConditionalExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_conditional_expr(expr, context)
    }

    fn visit_unary_operator_expr(
        &mut self,
        expr: &o::UnaryOperatorExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_unary_operator_expr(expr, context)
    }

    fn visit_parenthesized_expr(
        &mut self,
        expr: &o::ParenthesizedExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_parenthesized_expr(expr, context)
    }

    fn visit_function_expr(
        &mut self,
        expr: &o::FunctionExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_function_expr(expr, context)
    }

    fn visit_arrow_function_expr(
        &mut self,
        expr: &o::ArrowFunctionExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_arrow_function_expr(expr, context)
    }

    fn visit_literal_array_expr(
        &mut self,
        expr: &o::LiteralArrayExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_literal_array_expr(expr, context)
    }

    fn visit_literal_map_expr(
        &mut self,
        expr: &o::LiteralMapExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_literal_map_expr(expr, context)
    }

    fn visit_comma_expr(
        &mut self,
        expr: &o::CommaExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_comma_expr(expr, context)
    }

    fn visit_typeof_expr(
        &mut self,
        expr: &o::TypeofExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_typeof_expr(expr, context)
    }

    fn visit_void_expr(
        &mut self,
        expr: &o::VoidExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_void_expr(expr, context)
    }

    fn visit_not_expr(
        &mut self,
        expr: &o::NotExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_not_expr(expr, context)
    }

    fn visit_if_null_expr(
        &mut self,
        expr: &o::IfNullExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_if_null_expr(expr, context)
    }

    fn visit_assert_not_null_expr(
        &mut self,
        expr: &o::AssertNotNullExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_assert_not_null_expr(expr, context)
    }

    fn visit_cast_expr(
        &mut self,
        expr: &o::CastExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_cast_expr(expr, context)
    }

    fn visit_dynamic_import_expr(
        &mut self,
        expr: &o::DynamicImportExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_dynamic_import_expr(expr, context)
    }

    fn visit_template_literal_expr(
        &mut self,
        expr: &o::TemplateLiteralExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_template_literal_expr(expr, context)
    }

    fn visit_regular_expression_literal(
        &mut self,
        expr: &o::RegularExpressionLiteralExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_regular_expression_literal(expr, context)
    }

    fn visit_wrapped_node_expr(
        &mut self,
        _expr: &o::WrappedNodeExpr,
        _context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        // TODO: Handle wrapped node expression
        Box::new(())
    }

    // IR Expression visitor methods
    fn visit_lexical_read_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::LexicalReadExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_lexical_read_expr(expr, context)
    }

    fn visit_reference_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ReferenceExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_reference_expr(expr, context)
    }

    fn visit_context_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ContextExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_context_expr(expr, context)
    }

    fn visit_next_context_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::NextContextExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_next_context_expr(expr, context)
    }

    fn visit_get_current_view_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::GetCurrentViewExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_get_current_view_expr(expr, context)
    }

    fn visit_restore_view_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::RestoreViewExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_restore_view_expr(expr, context)
    }

    fn visit_reset_view_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ResetViewExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_reset_view_expr(expr, context)
    }

    fn visit_read_variable_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ReadVariableExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_read_variable_expr(expr, context)
    }

    fn visit_pure_function_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::PureFunctionExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_pure_function_expr(expr, context)
    }

    fn visit_pure_function_parameter_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::PureFunctionParameterExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_pure_function_parameter_expr(expr, context)
    }

    fn visit_pipe_binding_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::PipeBindingExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_pipe_binding_expr(expr, context)
    }

    fn visit_pipe_binding_variadic_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::PipeBindingVariadicExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_pipe_binding_variadic_expr(expr, context)
    }

    fn visit_safe_property_read_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SafePropertyReadExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_safe_property_read_expr(expr, context)
    }

    fn visit_safe_keyed_read_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SafeKeyedReadExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_safe_keyed_read_expr(expr, context)
    }

    fn visit_safe_invoke_function_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SafeInvokeFunctionExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_safe_invoke_function_expr(expr, context)
    }

    fn visit_safe_ternary_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SafeTernaryExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_safe_ternary_expr(expr, context)
    }

    fn visit_empty_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::EmptyExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_empty_expr(expr, context)
    }

    fn visit_assign_temporary_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::AssignTemporaryExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_assign_temporary_expr(expr, context)
    }

    fn visit_read_temporary_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ReadTemporaryExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_read_temporary_expr(expr, context)
    }

    fn visit_slot_literal_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SlotLiteralExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_slot_literal_expr(expr, context)
    }

    fn visit_conditional_case_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ConditionalCaseExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_conditional_case_expr(expr, context)
    }

    fn visit_const_collected_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ConstCollectedExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_const_collected_expr(expr, context)
    }

    fn visit_two_way_binding_set_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::TwoWayBindingSetExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_two_way_binding_set_expr(expr, context)
    }

    fn visit_context_let_reference_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ContextLetReferenceExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_context_let_reference_expr(expr, context)
    }

    fn visit_store_let_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::StoreLetExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_store_let_expr(expr, context)
    }

    fn visit_track_context_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::TrackContextExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any> {
        self.base.visit_track_context_expr(expr, context)
    }
}

fn _is_use_strict_statement(statement: &o::Statement) -> bool {
    if let o::Statement::Expression(expr_stmt) = statement {
        if let o::Expression::Literal(lit) = &*expr_stmt.expr {
            if let o::LiteralValue::String(s) = &lit.value {
                return s == "use strict";
            }
        }
    }
    false
}
