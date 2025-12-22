use angular_compiler::output::output_ast as o;

use crate::ngtsc::translator::src::api::ast_factory::{
    AstFactory, BinaryOperator,
    SourceMapRange, SourceMapLocation
};
use crate::ngtsc::translator::src::api::import_generator::{ImportGenerator, ImportRequest};
use crate::ngtsc::translator::src::context::Context;

pub type RecordWrappedNodeFn<TExpression> = Box<dyn Fn(&o::WrappedNodeExpr) -> Option<TExpression>>;

pub struct TranslatorOptions<TExpression> {
    pub downlevel_tagged_templates: bool,
    pub downlevel_variable_declarations: bool,
    pub record_wrapped_node: Option<RecordWrappedNodeFn<TExpression>>,
    pub annotate_for_closure_compiler: bool,
}

pub struct ExpressionTranslatorVisitor<'a, A: AstFactory, TFile> {
    factory: &'a A,
    imports: &'a mut dyn ImportGenerator<TFile, A::Expression>,
    context_file: TFile,
    downlevel_tagged_templates: bool,
    downlevel_variable_declarations: bool,
    record_wrapped_node: Option<RecordWrappedNodeFn<A::Expression>>,
}

impl<'a, A: AstFactory, TFile: Clone> ExpressionTranslatorVisitor<'a, A, TFile> 
where A::Expression: Clone 
{
    pub fn new(
        factory: &'a A,
        imports: &'a mut dyn ImportGenerator<TFile, A::Expression>,
        context_file: TFile,
        options: TranslatorOptions<A::Expression>,
    ) -> Self {
        Self {
            factory,
            imports,
            context_file,
            downlevel_tagged_templates: options.downlevel_tagged_templates,
            downlevel_variable_declarations: options.downlevel_variable_declarations,
            record_wrapped_node: options.record_wrapped_node,
        }
    }

    // Main entry points matching on Enums
    pub fn visit_statement(&mut self, stmt: &o::Statement, context: Context) -> A::Statement {
        match stmt {
            o::Statement::DeclareVar(s) => self.visit_declare_var_stmt(s, context),
            o::Statement::DeclareFn(s) => self.visit_declare_fn_stmt(s, context),
            o::Statement::Expression(s) => self.visit_expression_stmt(s, context),
            o::Statement::Return(s) => self.visit_return_stmt(s, context),
            o::Statement::IfStmt(s) => self.visit_if_stmt(s, context),
            // TODO: Add other statement variants as needed
        }
    }

    pub fn visit_expression(&mut self, expr: &o::Expression, context: Context) -> A::Expression {
        match expr {
            o::Expression::ReadVar(e) => self.visit_read_var_expr(e, context),
            o::Expression::InvokeFn(e) => self.visit_invoke_fn_expr(e, context),
            o::Expression::TaggedTemplate(e) => self.visit_tagged_template_expr(e, context),
            o::Expression::TemplateLiteral(e) => self.visit_template_literal_expr(e, context),
            o::Expression::Instantiate(e) => self.visit_instantiate_expr(e, context),
            o::Expression::Literal(e) => self.visit_literal_expr(e, context),
            o::Expression::External(e) => self.visit_external_expr(e, context),
            o::Expression::Conditional(e) => self.visit_conditional_expr(e, context),
            o::Expression::DynamicImport(e) => self.visit_dynamic_import_expr(e, context),
            o::Expression::NotExpr(e) => self.visit_not_expr(e, context),
            o::Expression::Fn(e) => self.visit_function_expr(e, context),
            o::Expression::ArrowFn(e) => self.visit_arrow_function_expr(e, context),
            o::Expression::BinaryOp(e) => self.visit_binary_operator_expr(e, context),
            o::Expression::ReadProp(e) => self.visit_read_prop_expr(e, context),
            o::Expression::ReadKey(e) => self.visit_read_key_expr(e, context),
            o::Expression::LiteralArray(e) => self.visit_literal_array_expr(e, context),
            o::Expression::LiteralMap(e) => self.visit_literal_map_expr(e, context),
            o::Expression::Unary(e) => self.visit_unary_operator_expr(e, context),
            o::Expression::Parens(e) => self.visit_parenthesized_expr(e, context),
            o::Expression::TypeOf(e) => self.visit_typeof_expr(e, context),
            o::Expression::Void(e) => self.visit_void_expr(e, context),
            o::Expression::WrappedNode(e) => self.visit_wrapped_node_expr(e, context),
            o::Expression::Localized(e) => self.visit_localized_string(e, context),
            o::Expression::RegularExpressionLiteral(e) => self.visit_regular_expression_literal(e, context),
            // TODO: Handle others or panic/error
            _ => panic!("Unsupported expression type in translator: {:?}", expr),
        }
    }

    fn visit_declare_var_stmt(&mut self, stmt: &o::DeclareVarStmt, context: Context) -> A::Statement {
        // Map modifier to type
        let var_type = if self.downlevel_variable_declarations {
             crate::ngtsc::translator::src::api::ast_factory::VariableDeclarationType::Var
        } else if stmt.modifiers == o::StmtModifier::Final {
             crate::ngtsc::translator::src::api::ast_factory::VariableDeclarationType::Const
        } else {
             crate::ngtsc::translator::src::api::ast_factory::VariableDeclarationType::Let
        };

        let initializer = stmt.value.as_ref().map(|v| self.visit_expression(v, context.with_expression_mode()));
        
        // TODO: attach comments
        self.factory.create_variable_declaration(&stmt.name, initializer, var_type)
    }

    fn visit_declare_fn_stmt(&mut self, stmt: &o::DeclareFunctionStmt, context: Context) -> A::Statement {
         let body = self.factory.create_block(self.visit_statements(&stmt.statements, context.with_statement_mode()));
         let params = stmt.params.iter().map(|p| p.name.clone()).collect();
         self.factory.create_function_declaration(&stmt.name, params, body)
    }

    fn visit_expression_stmt(&mut self, stmt: &o::ExpressionStatement, context: Context) -> A::Statement {
         self.factory.create_expression_statement(self.visit_expression(&stmt.expr, context.with_statement_mode()))
    }

    fn visit_return_stmt(&mut self, stmt: &o::ReturnStatement, context: Context) -> A::Statement {
         self.factory.create_return_statement(Some(self.visit_expression(&stmt.value, context.with_expression_mode())))
    }

    fn visit_if_stmt(&mut self, stmt: &o::IfStmt, context: Context) -> A::Statement {
         let condition = self.visit_expression(&stmt.condition, context);
         let true_case = self.factory.create_block(self.visit_statements(&stmt.true_case, context.with_statement_mode()));
         let false_case = if !stmt.false_case.is_empty() {
             Some(self.factory.create_block(self.visit_statements(&stmt.false_case, context.with_statement_mode())))
         } else {
             None
         };
         self.factory.create_if_statement(condition, true_case, false_case)
    }

    fn visit_read_var_expr(&mut self, ast: &o::ReadVarExpr, _context: Context) -> A::Expression {
         let identifier = self.factory.create_identifier(&ast.name);
         self.set_source_map_range_expr(identifier, ast.source_span.as_ref())
    }
    
    fn visit_invoke_fn_expr(&mut self, ast: &o::InvokeFunctionExpr, context: Context) -> A::Expression {
         let callee = self.visit_expression(&ast.fn_, context);
         let args = ast.args.iter().map(|a| self.visit_expression(a, context)).collect();
         self.set_source_map_range_expr(
             self.factory.create_call_expression(callee, args, ast.pure),
             ast.source_span.as_ref()
         )
    }

    fn visit_literal_expr(&mut self, ast: &o::LiteralExpr, _context: Context) -> A::Expression {
        let val = match &ast.value {
            o::LiteralValue::Null => crate::ngtsc::translator::src::api::ast_factory::LiteralValue::Null,
            o::LiteralValue::String(s) => crate::ngtsc::translator::src::api::ast_factory::LiteralValue::String(s),
            o::LiteralValue::Number(n) => crate::ngtsc::translator::src::api::ast_factory::LiteralValue::Number(*n),
            o::LiteralValue::Bool(b) => crate::ngtsc::translator::src::api::ast_factory::LiteralValue::Boolean(*b),
        };
        self.set_source_map_range_expr(self.factory.create_literal(val), ast.source_span.as_ref())
    }
    
    fn visit_external_expr(&mut self, ast: &o::ExternalExpr, _context: Context) -> A::Expression {
        // If module name is present, import it. Else ambient global.
        if let Some(module_name) = &ast.value.module_name {
            self.imports.add_import(ImportRequest {
                export_module_specifier: module_name.clone(),
                export_symbol_name: ast.value.name.clone(),
                requested_file: self.context_file.clone(),
                unsafe_alias_override: None,
            })
        } else if let Some(name) = &ast.value.name {
            self.factory.create_identifier(name)
        } else {
            panic!("Invalid external expr: {:?}", ast);
        }
    }
    
    // ... Implement other visit methods ...
    // Placeholder versions for brevity in this step, need to complete all
    
    fn visit_conditional_expr(&mut self, ast: &o::ConditionalExpr, context: Context) -> A::Expression {
        self.factory.create_conditional(
            self.visit_expression(&ast.condition, context),
            self.visit_expression(&ast.true_case, context),
            // false_case is Option in o::ConditionalExpr?
            ast.false_case.as_ref().map(|f| self.visit_expression(f, context)).unwrap_or_else(|| self.factory.create_literal(crate::ngtsc::translator::src::api::ast_factory::LiteralValue::Null)) // Default? In TS implies mandatory or logic error if missing
        )
    }

    fn visit_binary_operator_expr(&mut self, ast: &o::BinaryOperatorExpr, context: Context) -> A::Expression {
        // Map operator
        match map_binary_operator(ast.operator) {
            Some(op) => {
                 self.factory.create_binary_expression(
                     self.visit_expression(&ast.lhs, context),
                     op,
                     self.visit_expression(&ast.rhs, context)
                 )
            },
            None => {
                // Should check if it is assignment
                 if let Some(op) = map_binary_operator_assignment(ast.operator) {
                     // Note: BinaryOperator enum in AstFactory might reuse Same enum or we should check
                     // AstFactory has one BinaryOperator enum. If it covers assignments, good.
                     // Our AstFactory defines Assignment separately? No, it has create_assignment.
                     self.factory.create_assignment(
                         self.visit_expression(&ast.lhs, context),
                         op,
                         self.visit_expression(&ast.rhs, context)
                     )
                 } else {
                     panic!("Unknown binary operator {:?}", ast.operator);
                 }
            }
        }
    }
    
    // Helper to visit statements
    fn visit_statements(&mut self, statements: &[o::Statement], context: Context) -> Vec<A::Statement> {
        statements.iter().map(|s| self.visit_statement(s, context)).collect()
    }

    fn set_source_map_range_expr(&self, expr: A::Expression, span: Option<&angular_compiler::parse_util::ParseSourceSpan>) -> A::Expression {
        if let Some(span) = span {
            // Convert span to SourceMapRange
            // Need access to file url/content from span.start.file
             let start = &span.start;
             let end = &span.end;
             let url = start.file.url.to_string(); // Assuming methods exist
             let content = start.file.content.to_string(); 
             
             let range = SourceMapRange {
                 url,
                 content,
                 start: SourceMapLocation { offset: start.offset, line: start.line, column: start.col },
                 end: SourceMapLocation { offset: end.offset, line: end.line, column: end.col },
             };
             self.factory.set_source_map_range_for_expr(expr, Some(range))
        } else {
            expr
        }
    }
    
    // ... Stubbing remaining mandatory visit methods for compilation success,
    // in real implementation I should fill them.

    fn visit_tagged_template_expr(&mut self, _e: &o::TaggedTemplateLiteralExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_template_literal_expr(&mut self, _e: &o::TemplateLiteralExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_instantiate_expr(&mut self, _e: &o::InstantiateExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_dynamic_import_expr(&mut self, _e: &o::DynamicImportExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_not_expr(&mut self, _e: &o::NotExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_function_expr(&mut self, _e: &o::FunctionExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_arrow_function_expr(&mut self, _e: &o::ArrowFunctionExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_read_prop_expr(&mut self, _e: &o::ReadPropExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_read_key_expr(&mut self, _e: &o::ReadKeyExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_literal_array_expr(&mut self, _e: &o::LiteralArrayExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_literal_map_expr(&mut self, _e: &o::LiteralMapExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_unary_operator_expr(&mut self, _e: &o::UnaryOperatorExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_parenthesized_expr(&mut self, _e: &o::ParenthesizedExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_typeof_expr(&mut self, _e: &o::TypeofExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_void_expr(&mut self, _e: &o::VoidExpr, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_wrapped_node_expr(&mut self, e: &o::WrappedNodeExpr, _c: Context) -> A::Expression {
        if let Some(record) = &self.record_wrapped_node {
            if let Some(result) = record(e) {
                return result;
            }
        }
        panic!("WrappedNodeExpr visited but not handled by record_wrapped_node")
    }
    fn visit_localized_string(&mut self, _e: &o::LocalizedString, _c: Context) -> A::Expression { unimplemented!() }
    fn visit_regular_expression_literal(&mut self, e: &o::RegularExpressionLiteralExpr, _c: Context) -> A::Expression {
        self.factory.create_regular_expression_literal(&e.pattern, if e.flags.is_empty() { None } else { Some(&e.flags) })
    }

}

fn map_binary_operator(op: o::BinaryOperator) -> Option<BinaryOperator> {
    match op {
        o::BinaryOperator::And => Some(BinaryOperator::And),
        o::BinaryOperator::Bigger => Some(BinaryOperator::Greater),
        // ... map others
        _ => None
    }
}

fn map_binary_operator_assignment(op: o::BinaryOperator) -> Option<BinaryOperator> {
    // Map assignments
    None
}
