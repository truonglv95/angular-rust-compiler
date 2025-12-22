use oxc_ast::ast::{
    Statement, Expression, ObjectPropertyKind, PropertyKey, PropertyKind
};
use oxc_ast::AstBuilder;
use oxc_span::Span;
use oxc_syntax::operator::{BinaryOperator as OxcBinaryOperator, UnaryOperator as OxcUnaryOperator, AssignmentOperator};

use crate::ngtsc::translator::src::api::ast_factory::{
    AstFactory, BinaryOperator, LeadingComment, LiteralValue, ObjectLiteralProperty, SourceMapRange,
    TemplateLiteral, UnaryOperator, VariableDeclarationType, ArrowFunctionBody
};
use crate::ngtsc::translator::src::ts_util::ts_numeric_expression;

pub struct TypeScriptAstFactory<'a> {
    builder: AstBuilder<'a>,
    annotate_for_closure_compiler: bool,
}

impl<'a> TypeScriptAstFactory<'a> {
    pub fn new(builder: AstBuilder<'a>, annotate_for_closure_compiler: bool) -> Self {
        Self {
            builder,
            annotate_for_closure_compiler,
        }
    }

    fn map_binary_operator(&self, op: BinaryOperator) -> OxcBinaryOperator {
        match op {
            BinaryOperator::BitAnd => OxcBinaryOperator::BitwiseAnd,
            BinaryOperator::BitOr => OxcBinaryOperator::BitwiseOR,
            BinaryOperator::Divide => OxcBinaryOperator::Division,
            BinaryOperator::Equals => OxcBinaryOperator::Equality,
            BinaryOperator::IdentityEquals => OxcBinaryOperator::StrictEquality,
            BinaryOperator::Greater => OxcBinaryOperator::GreaterThan,
            BinaryOperator::GreaterEquals => OxcBinaryOperator::GreaterEqualThan,
            BinaryOperator::Less => OxcBinaryOperator::LessThan,
            BinaryOperator::LessEquals => OxcBinaryOperator::LessEqualThan,
            BinaryOperator::Minus => OxcBinaryOperator::Subtraction,
            BinaryOperator::Modulo => OxcBinaryOperator::Remainder,
            BinaryOperator::Multiply => OxcBinaryOperator::Multiplication,
            BinaryOperator::NotEquals => OxcBinaryOperator::Inequality,
            BinaryOperator::IdentityNotEquals => OxcBinaryOperator::StrictInequality,
            BinaryOperator::Plus => OxcBinaryOperator::Addition,
            BinaryOperator::Power => OxcBinaryOperator::Exponential,
            // Note: And, Or, NullishCoalesce are handled separately in create_binary_expression
            // as LogicalExpression nodes, not BinaryExpression
            _ => panic!("Unsupported binary operator: {:?}. Logical operators (And, Or, NullishCoalesce) should be handled separately.", op),
        }
    }
    
    fn map_assignment_operator(&self, op: BinaryOperator) -> AssignmentOperator {
        match op {
             BinaryOperator::Assign => AssignmentOperator::Assign,
             BinaryOperator::PlusAssign => AssignmentOperator::Addition,
             BinaryOperator::MinusAssign => AssignmentOperator::Subtraction,
             BinaryOperator::MultiplyAssign => AssignmentOperator::Multiplication,
             BinaryOperator::DivideAssign => AssignmentOperator::Division,
             BinaryOperator::ModuloAssign => AssignmentOperator::Remainder,
             BinaryOperator::PowerAssign => AssignmentOperator::Exponential,
             BinaryOperator::AndAssign => AssignmentOperator::BitwiseAnd,
             BinaryOperator::OrAssign => AssignmentOperator::BitwiseOR,
             _ => panic!("Unsupported assignment operator: {:?}", op),
        }
    }
}

impl<'a> AstFactory for TypeScriptAstFactory<'a> {
    type Statement = Statement<'a>;
    type Expression = Expression<'a>;

    fn attach_comments(&self, _statement: &mut Self::Statement, _leading_comments: &[Box<dyn LeadingComment>]) {
        // TODO: Implement comment attachment with oxc (requires trivia management)
    }

    fn attach_comments_to_expression(&self, _expression: &mut Self::Expression, _leading_comments: &[Box<dyn LeadingComment>]) {
        // TODO
    }

    fn create_array_literal(&self, elements: Vec<Self::Expression>) -> Self::Expression {
        let elements = oxc_allocator::Vec::from_iter_in(
            elements.into_iter().map(|e| oxc_ast::ast::ArrayExpressionElement::from(e)),
            self.builder.allocator
        );
        oxc_ast::ast::Expression::ArrayExpression(
            self.builder.alloc(self.builder.array_expression(Span::default(), elements))
        )
    }

    fn create_assignment(&self, target: Self::Expression, operator: BinaryOperator, value: Self::Expression) -> Self::Expression {
        let op = self.map_assignment_operator(operator);
        let target = match target {
             oxc_ast::ast::Expression::Identifier(id) => oxc_ast::ast::AssignmentTarget::AssignmentTargetIdentifier(id),
             oxc_ast::ast::Expression::ComputedMemberExpression(e) => oxc_ast::ast::AssignmentTarget::ComputedMemberExpression(e),
             oxc_ast::ast::Expression::StaticMemberExpression(e) => oxc_ast::ast::AssignmentTarget::StaticMemberExpression(e),
             oxc_ast::ast::Expression::PrivateFieldExpression(e) => oxc_ast::ast::AssignmentTarget::PrivateFieldExpression(e),
             _ => panic!("Invalid assignment target expression: {:?}", target),
        };
        oxc_ast::ast::Expression::AssignmentExpression(
             self.builder.alloc(self.builder.assignment_expression(Span::default(), op, target, value))
        )
    }

    fn create_binary_expression(&self, left_operand: Self::Expression, operator: BinaryOperator, right_operand: Self::Expression) -> Self::Expression {
        // Check for Logical operators
        match operator {
            BinaryOperator::And => oxc_ast::ast::Expression::LogicalExpression(self.builder.alloc(self.builder.logical_expression(Span::default(), left_operand, oxc_syntax::operator::LogicalOperator::And, right_operand))),
            BinaryOperator::Or => oxc_ast::ast::Expression::LogicalExpression(self.builder.alloc(self.builder.logical_expression(Span::default(), left_operand, oxc_syntax::operator::LogicalOperator::Or, right_operand))),
            BinaryOperator::NullishCoalesce => oxc_ast::ast::Expression::LogicalExpression(self.builder.alloc(self.builder.logical_expression(Span::default(), left_operand, oxc_syntax::operator::LogicalOperator::Coalesce, right_operand))),
            _ => {
                let op = self.map_binary_operator(operator);
                oxc_ast::ast::Expression::BinaryExpression(self.builder.alloc(self.builder.binary_expression(Span::default(), left_operand, op, right_operand)))
            }
        }
    }

    fn create_block(&self, body: Vec<Self::Statement>) -> Self::Statement {
        oxc_ast::ast::Statement::BlockStatement(
            self.builder.alloc(self.builder.block_statement(Span::default(), oxc_allocator::Vec::from_iter_in(body, self.builder.allocator)))
        )
    }

    fn create_call_expression(&self, callee: Self::Expression, args: Vec<Self::Expression>, pure: bool) -> Self::Expression {
        let args = oxc_allocator::Vec::from_iter_in(
            args.into_iter().map(|a| oxc_ast::ast::Argument::from(a)),
            self.builder.allocator
        );
        let call = self.builder.call_expression(
            Span::default(),
            callee,
            None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterInstantiation>>,
            args,
            false
        );
        
        if pure {
            // TODO: Add pure comment
        }
        oxc_ast::ast::Expression::CallExpression(self.builder.alloc(call))
    }

    fn create_conditional(&self, condition: Self::Expression, then_expression: Self::Expression, else_expression: Self::Expression) -> Self::Expression {
        oxc_ast::ast::Expression::ConditionalExpression(
            self.builder.alloc(self.builder.conditional_expression(Span::default(), condition, then_expression, else_expression))
        )
    }

    fn create_element_access(&self, expression: Self::Expression, element: Self::Expression) -> Self::Expression {
        self.builder.member_expression_computed(Span::default(), expression, element, false).into()
    }

    fn create_expression_statement(&self, expression: Self::Expression) -> Self::Statement {
        oxc_ast::ast::Statement::ExpressionStatement(
            self.builder.alloc(self.builder.expression_statement(Span::default(), expression))
        )
    }

    fn create_function_declaration(&self, _function_name: &str, _parameters: Vec<String>, _body: Self::Statement) -> Self::Statement {
        // TODO: Implement proper function declaration with lifetime management
        // The challenge is that `&self` has a different lifetime than `'a` in the struct
        todo!("create_function_declaration requires proper lifetime handling")
    }

    fn create_function_expression(&self, _function_name: Option<&str>, _parameters: Vec<String>, _body: Self::Statement) -> Self::Expression {
        // TODO: Implement proper function expression with lifetime management
        todo!("create_function_expression requires proper lifetime handling")
    }

    fn create_arrow_function_expression(&self, _parameters: Vec<String>, _body: ArrowFunctionBody<Self::Statement, Self::Expression>) -> Self::Expression {
        // TODO: Implement proper arrow function expression with lifetime management
        todo!("create_arrow_function_expression requires proper lifetime handling")
    }

    fn create_dynamic_import(&self, url: Result<&str, Self::Expression>) -> Self::Expression {
        let arg = match url {
            Ok(s) => oxc_ast::ast::Expression::StringLiteral(self.builder.alloc(self.builder.string_literal(Span::default(), self.builder.allocator.alloc_str(s), None))),
            Err(e) => e,
        };
        oxc_ast::ast::Expression::ImportExpression(
            self.builder.alloc(self.builder.import_expression(Span::default(), arg, None, None))
        )
    }

    fn create_identifier(&self, name: &str) -> Self::Expression {
        oxc_ast::ast::Expression::Identifier(
            self.builder.alloc(self.builder.identifier_reference(Span::default(), self.builder.allocator.alloc_str(name)))
        )
    }

    fn create_if_statement(&self, condition: Self::Expression, then_statement: Self::Statement, else_statement: Option<Self::Statement>) -> Self::Statement {
        oxc_ast::ast::Statement::IfStatement(
            self.builder.alloc(self.builder.if_statement(Span::default(), condition, then_statement, else_statement))
        )
    }

    fn create_literal(&self, value: LiteralValue) -> Self::Expression {
        match value {
            LiteralValue::String(s) => oxc_ast::ast::Expression::StringLiteral(self.builder.alloc(self.builder.string_literal(Span::default(), self.builder.allocator.alloc_str(&s), None))),
            LiteralValue::Number(n) => ts_numeric_expression(&self.builder, n),
            LiteralValue::Boolean(b) => oxc_ast::ast::Expression::BooleanLiteral(self.builder.alloc(self.builder.boolean_literal(Span::default(), b))),
            LiteralValue::Null => oxc_ast::ast::Expression::NullLiteral(self.builder.alloc(self.builder.null_literal(Span::default()))),
            LiteralValue::Undefined => oxc_ast::ast::Expression::Identifier(self.builder.alloc(self.builder.identifier_reference(Span::default(), "undefined"))),
        }
    }

    fn create_new_expression(&self, expression: Self::Expression, args: Vec<Self::Expression>) -> Self::Expression {
        oxc_ast::ast::Expression::NewExpression(
            self.builder.alloc(self.builder.new_expression(
                Span::default(), 
                expression, 
                None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterInstantiation>>,
                oxc_allocator::Vec::from_iter_in(args.into_iter().map(|a| oxc_ast::ast::Argument::from(a)), self.builder.allocator)
            ))
        )
    }

    fn create_object_literal(&self, properties: Vec<ObjectLiteralProperty<Self::Expression>>) -> Self::Expression {
        let props: oxc_allocator::Vec<'a, ObjectPropertyKind<'a>> = oxc_allocator::Vec::from_iter_in(
            properties.into_iter().map(|p| {
                let key: PropertyKey<'a> = if p.quoted {
                    PropertyKey::StringLiteral(self.builder.alloc(self.builder.string_literal(Span::default(), self.builder.allocator.alloc_str(p.property_name.as_str()), None)))
                } else {
                    self.builder.property_key_static_identifier(Span::default(), self.builder.allocator.alloc_str(p.property_name.as_str()))
                };
                self.builder.object_property_kind_object_property(Span::default(), PropertyKind::Init, key, p.value, false, false, false)
            }),
            self.builder.allocator
        );
        oxc_ast::ast::Expression::ObjectExpression(self.builder.alloc(self.builder.object_expression(Span::default(), props)))
    }

    fn create_parenthesized_expression(&self, expression: Self::Expression) -> Self::Expression {
        oxc_ast::ast::Expression::ParenthesizedExpression(
            self.builder.alloc(self.builder.parenthesized_expression(Span::default(), expression))
        )
    }

    fn create_property_access(&self, expression: Self::Expression, property_name: &str) -> Self::Expression {
        self.builder.member_expression_static(Span::default(), expression, self.builder.identifier_name(Span::default(), self.builder.allocator.alloc_str(property_name)), false).into()
    }

    fn create_return_statement(&self, expression: Option<Self::Expression>) -> Self::Statement {
        oxc_ast::ast::Statement::ReturnStatement(
             self.builder.alloc(self.builder.return_statement(Span::default(), expression))
        )
    }

    fn create_tagged_template(&self, tag: Self::Expression, _template: TemplateLiteral<Self::Expression>) -> Self::Expression {
        // Implement template literal creation
        // Placeholder
        oxc_ast::ast::Expression::TaggedTemplateExpression(
            self.builder.alloc(self.builder.tagged_template_expression(
                Span::default(), 
                tag, 
                None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterInstantiation>>,
                self.builder.template_literal(Span::default(), self.builder.vec(), self.builder.vec())
            ))
        )
    }

    fn create_template_literal(&self, _template: TemplateLiteral<Self::Expression>) -> Self::Expression {
        // Placeholder
        oxc_ast::ast::Expression::TemplateLiteral(
            self.builder.alloc(self.builder.template_literal(Span::default(), self.builder.vec(), self.builder.vec()))
        )
    }

    fn create_throw_statement(&self, expression: Self::Expression) -> Self::Statement {
        oxc_ast::ast::Statement::ThrowStatement(
            self.builder.alloc(self.builder.throw_statement(Span::default(), expression))
        )
    }

    fn create_type_of_expression(&self, expression: Self::Expression) -> Self::Expression {
        oxc_ast::ast::Expression::UnaryExpression(
            self.builder.alloc(self.builder.unary_expression(Span::default(), OxcUnaryOperator::Typeof, expression))
        )
    }

    fn create_void_expression(&self, expression: Self::Expression) -> Self::Expression {
         oxc_ast::ast::Expression::UnaryExpression(
             self.builder.alloc(self.builder.unary_expression(Span::default(), OxcUnaryOperator::Void, expression))
         )
    }

    fn create_unary_expression(&self, operator: UnaryOperator, operand: Self::Expression) -> Self::Expression {
        let op = match operator {
            UnaryOperator::Plus => OxcUnaryOperator::UnaryPlus,
            UnaryOperator::Minus => OxcUnaryOperator::UnaryNegation,
            UnaryOperator::Not => OxcUnaryOperator::LogicalNot,
        };
        oxc_ast::ast::Expression::UnaryExpression(
            self.builder.alloc(self.builder.unary_expression(Span::default(), op, operand))
        )
    }

    fn create_variable_declaration(&self, _variable_name: &str, _initializer: Option<Self::Expression>, _type_: VariableDeclarationType) -> Self::Statement {
        // TODO: Implement proper variable declaration with lifetime management
        // The challenge is that `&self` has a different lifetime than `'a` in the struct
        todo!("create_variable_declaration requires proper lifetime handling")
    }
    
    fn create_regular_expression_literal(&self, _body: &str, _flags: Option<&str>) -> Self::Expression {
        // TODO: Implement proper regex literal with lifetime management
        todo!("create_regular_expression_literal requires proper lifetime handling")
    }
    
    fn set_source_map_range_for_stmt(&self, node: Self::Statement, _source_map_range: Option<SourceMapRange>) -> Self::Statement {
        // TODO: Handle source mapping
        node
    }

    fn set_source_map_range_for_expr(&self, node: Self::Expression, _source_map_range: Option<SourceMapRange>) -> Self::Expression {
        // TODO: Handle source mapping
        node
    }
}
