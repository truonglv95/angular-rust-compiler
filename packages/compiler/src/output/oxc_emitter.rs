use oxc_allocator::{Allocator, Box as OxcBox, FromIn, Vec as OxcVec};
use oxc_ast::ast as oxc;
use oxc_span::{Atom, Span};
use std::cell::Cell;

use crate::output::output_ast::*;

pub struct OxcEmitter<'a> {
    pub allocator: &'a Allocator,
    /// Module name → alias (e.g. "@angular/core" → "i0", "@angular/common" → "i1")
    imports_map: std::collections::HashMap<String, String>,
}

impl<'a> OxcEmitter<'a> {
    pub fn new(allocator: &'a Allocator) -> Self {
        Self {
            allocator,
            imports_map: std::collections::HashMap::new(),
        }
    }

    pub fn with_imports(
        allocator: &'a Allocator,
        imports_map: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            allocator,
            imports_map,
        }
    }

    /// Intern a string into the allocator arena.
    #[inline]
    fn atom(&self, s: &str) -> Atom<'a> {
        Atom::from_in(s, self.allocator)
    }

    pub fn emit_expression(&self, expr: &Expression) -> oxc::Expression<'a> {
        match expr {
            Expression::ReadVar(read_var) => self.emit_read_var(read_var),
            Expression::WriteVar(write_var) => self.emit_write_var(write_var),
            Expression::WriteProp(write_prop) => self.emit_write_prop(write_prop),
            Expression::WriteKey(write_key) => self.emit_write_key(write_key),
            Expression::InvokeFn(invoke_fn) => self.emit_invoke_function(invoke_fn),
            Expression::Instantiate(instantiate) => self.emit_instantiate(instantiate),
            Expression::Literal(literal) => self.emit_literal(literal),
            Expression::External(external) => self.emit_external(external),
            Expression::Conditional(cond) => self.emit_conditional(cond),
            Expression::NotExpr(not_expr) => self.emit_not(not_expr),
            Expression::ArrowFn(arrow_fn) => self.emit_arrow_function(arrow_fn),
            Expression::BinaryOp(binary_op) => self.emit_binary_operator(binary_op),
            Expression::ReadProp(read_prop) => self.emit_read_prop(read_prop),
            Expression::ReadKey(read_key) => self.emit_read_key(read_key),
            Expression::LiteralArray(literal_array) => self.emit_literal_array(literal_array),
            Expression::LiteralMap(literal_map) => self.emit_literal_map(literal_map),
            Expression::CommaExpr(comma_expr) => self.emit_comma(comma_expr),
            Expression::Unary(unary_op) => self.emit_unary_operator(unary_op),
            Expression::Parens(parens) => self.emit_parenthesized(parens),
            Expression::Fn(fn_expr) => self.emit_function(fn_expr),
            Expression::TypeOf(typeof_expr) => self.emit_typeof(typeof_expr),
            Expression::Void(void_expr) => self.emit_void(void_expr),
            _ => unimplemented!("OxcEmitter::emit_expression for {:?}", expr),
        }
    }

    // -- Helpers -- //

    fn make_id_ref(&self, name: &str) -> OxcBox<'a, oxc::IdentifierReference<'a>> {
        OxcBox::new_in(
            oxc::IdentifierReference {
                span: Span::default(),
                name: self.atom(name),
                reference_id: Cell::new(None),
            },
            self.allocator,
        )
    }

    fn make_binding_id(&self, name: &str) -> oxc::BindingIdentifier<'a> {
        oxc::BindingIdentifier {
            span: Span::default(),
            name: self.atom(name),
            symbol_id: Cell::new(None),
        }
    }

    fn make_binding_id_boxed(&self, name: &str) -> OxcBox<'a, oxc::BindingIdentifier<'a>> {
        OxcBox::new_in(self.make_binding_id(name), self.allocator)
    }

    // -- Expression emission -- //

    fn emit_read_var(&self, expr: &ReadVarExpr) -> oxc::Expression<'a> {
        oxc::Expression::Identifier(self.make_id_ref(&expr.name))
    }

    fn emit_write_var(&self, expr: &WriteVarExpr) -> oxc::Expression<'a> {
        let left = oxc::AssignmentTarget::AssignmentTargetIdentifier(self.make_id_ref(&expr.name));
        let right = self.emit_expression(&expr.value);

        oxc::Expression::AssignmentExpression(OxcBox::new_in(
            oxc::AssignmentExpression {
                span: Span::default(),
                operator: oxc_syntax::operator::AssignmentOperator::Assign,
                left,
                right,
            },
            self.allocator,
        ))
    }

    fn emit_write_prop(&self, expr: &WritePropExpr) -> oxc::Expression<'a> {
        let left = oxc::AssignmentTarget::StaticMemberExpression(OxcBox::new_in(
            oxc::StaticMemberExpression {
                span: Span::default(),
                object: self.emit_expression(&expr.receiver),
                property: oxc::IdentifierName {
                    span: Span::default(),
                    name: self.atom(&expr.name),
                },
                optional: false,
            },
            self.allocator,
        ));
        let right = self.emit_expression(&expr.value);

        oxc::Expression::AssignmentExpression(OxcBox::new_in(
            oxc::AssignmentExpression {
                span: Span::default(),
                operator: oxc_syntax::operator::AssignmentOperator::Assign,
                left,
                right,
            },
            self.allocator,
        ))
    }

    fn emit_write_key(&self, expr: &WriteKeyExpr) -> oxc::Expression<'a> {
        let left = oxc::AssignmentTarget::ComputedMemberExpression(OxcBox::new_in(
            oxc::ComputedMemberExpression {
                span: Span::default(),
                object: self.emit_expression(&expr.receiver),
                expression: self.emit_expression(&expr.index),
                optional: false,
            },
            self.allocator,
        ));
        let right = self.emit_expression(&expr.value);

        oxc::Expression::AssignmentExpression(OxcBox::new_in(
            oxc::AssignmentExpression {
                span: Span::default(),
                operator: oxc_syntax::operator::AssignmentOperator::Assign,
                left,
                right,
            },
            self.allocator,
        ))
    }

    fn emit_invoke_function(&self, expr: &InvokeFunctionExpr) -> oxc::Expression<'a> {
        let callee = self.emit_expression(&expr.fn_);
        let mut arguments = OxcVec::new_in(self.allocator);
        for arg in &expr.args {
            arguments.push(oxc::Argument::from(self.emit_expression(arg)));
        }

        oxc::Expression::CallExpression(OxcBox::new_in(
            oxc::CallExpression {
                span: Span::default(),
                callee,
                arguments,
                optional: false,
                type_arguments: None,
                pure: false,
            },
            self.allocator,
        ))
    }

    fn emit_instantiate(&self, expr: &InstantiateExpr) -> oxc::Expression<'a> {
        let callee = self.emit_expression(&expr.class_expr);
        let mut arguments = OxcVec::new_in(self.allocator);
        for arg in &expr.args {
            arguments.push(oxc::Argument::from(self.emit_expression(arg)));
        }

        oxc::Expression::NewExpression(OxcBox::new_in(
            oxc::NewExpression {
                span: Span::default(),
                callee,
                arguments,
                type_arguments: None,
                pure: false,
            },
            self.allocator,
        ))
    }

    fn emit_literal(&self, expr: &LiteralExpr) -> oxc::Expression<'a> {
        match &expr.value {
            LiteralValue::Null => oxc::Expression::NullLiteral(OxcBox::new_in(
                oxc::NullLiteral {
                    span: Span::default(),
                },
                self.allocator,
            )),
            LiteralValue::Undefined => oxc::Expression::Identifier(self.make_id_ref("undefined")),
            LiteralValue::String(s) => oxc::Expression::StringLiteral(OxcBox::new_in(
                oxc::StringLiteral {
                    span: Span::default(),
                    value: self.atom(s),
                    raw: None,
                    lone_surrogates: false,
                },
                self.allocator,
            )),
            LiteralValue::Number(n) => oxc::Expression::NumericLiteral(OxcBox::new_in(
                oxc::NumericLiteral {
                    span: Span::default(),
                    value: *n,
                    raw: None,
                    base: oxc_syntax::number::NumberBase::Decimal,
                },
                self.allocator,
            )),
            LiteralValue::Bool(b) => oxc::Expression::BooleanLiteral(OxcBox::new_in(
                oxc::BooleanLiteral {
                    span: Span::default(),
                    value: *b,
                },
                self.allocator,
            )),
        }
    }

    fn emit_external(&self, expr: &ExternalExpr) -> oxc::Expression<'a> {
        let name = expr.value.name.clone().unwrap_or_default();
        let module_name = expr.value.module_name.clone();

        if let Some(module) = module_name {
            // Look up the alias from imports_map first, fall back to hardcoded i0 for @angular/core
            let alias = self.imports_map.get(&module).cloned().unwrap_or_else(|| {
                if module == "@angular/core" {
                    "i0".to_string()
                } else {
                    // No alias found — emit bare identifier as fallback
                    return name.clone();
                }
            });

            // If alias == name, it means we didn't find a module mapping, emit bare identifier
            if alias == name {
                return oxc::Expression::Identifier(self.make_id_ref(&name));
            }

            let object = oxc::Expression::Identifier(self.make_id_ref(&alias));
            oxc::Expression::StaticMemberExpression(OxcBox::new_in(
                oxc::StaticMemberExpression {
                    span: Span::default(),
                    object,
                    property: oxc::IdentifierName {
                        span: Span::default(),
                        name: self.atom(&name),
                    },
                    optional: false,
                },
                self.allocator,
            ))
        } else {
            oxc::Expression::Identifier(self.make_id_ref(&name))
        }
    }

    fn emit_conditional(&self, expr: &ConditionalExpr) -> oxc::Expression<'a> {
        let test = self.emit_expression(&expr.condition);
        let consequent = self.emit_expression(&expr.true_case);
        let alternate = if let Some(false_case) = &expr.false_case {
            self.emit_expression(false_case)
        } else {
            oxc::Expression::Identifier(self.make_id_ref("undefined"))
        };

        oxc::Expression::ConditionalExpression(OxcBox::new_in(
            oxc::ConditionalExpression {
                span: Span::default(),
                test,
                consequent,
                alternate,
            },
            self.allocator,
        ))
    }

    fn emit_not(&self, expr: &NotExpr) -> oxc::Expression<'a> {
        let argument = self.emit_expression(&expr.condition);

        oxc::Expression::UnaryExpression(OxcBox::new_in(
            oxc::UnaryExpression {
                span: Span::default(),
                operator: oxc_syntax::operator::UnaryOperator::LogicalNot,
                argument,
            },
            self.allocator,
        ))
    }

    fn emit_arrow_function(&self, expr: &ArrowFunctionExpr) -> oxc::Expression<'a> {
        let params = self.emit_fn_params(&expr.params);
        let (body, expression) = match &expr.body {
            ArrowFunctionBody::Expression(e) => {
                let mut stmts = OxcVec::new_in(self.allocator);
                stmts.push(oxc::Statement::ExpressionStatement(OxcBox::new_in(
                    oxc::ExpressionStatement {
                        span: Span::default(),
                        expression: self.emit_expression(e),
                    },
                    self.allocator,
                )));
                (
                    OxcBox::new_in(
                        oxc::FunctionBody {
                            span: Span::default(),
                            directives: OxcVec::new_in(self.allocator),
                            statements: stmts,
                        },
                        self.allocator,
                    ),
                    true,
                )
            }
            ArrowFunctionBody::Statements(stmts) => {
                let oxc_stmts = self.emit_statements(stmts);
                (
                    OxcBox::new_in(
                        oxc::FunctionBody {
                            span: Span::default(),
                            directives: OxcVec::new_in(self.allocator),
                            statements: oxc_stmts,
                        },
                        self.allocator,
                    ),
                    false,
                )
            }
        };

        oxc::Expression::ArrowFunctionExpression(OxcBox::new_in(
            oxc::ArrowFunctionExpression {
                span: Span::default(),
                expression,
                r#async: false,
                params: OxcBox::new_in(params, self.allocator),
                body,
                scope_id: Cell::new(None),
                pure: false,
                pife: false,
                type_parameters: None,
                return_type: None,
            },
            self.allocator,
        ))
    }

    fn emit_function(&self, expr: &FunctionExpr) -> oxc::Expression<'a> {
        let params = self.emit_fn_params(&expr.params);
        let oxc_stmts = self.emit_statements(&expr.statements);

        let id = expr.name.as_ref().map(|n| self.make_binding_id(n));

        let body = OxcBox::new_in(
            oxc::FunctionBody {
                span: Span::default(),
                directives: OxcVec::new_in(self.allocator),
                statements: oxc_stmts,
            },
            self.allocator,
        );

        oxc::Expression::FunctionExpression(OxcBox::new_in(
            oxc::Function {
                r#type: oxc::FunctionType::FunctionExpression,
                span: Span::default(),
                id,
                generator: false,
                r#async: false,
                declare: false,
                type_parameters: None,
                this_param: None,
                params: OxcBox::new_in(params, self.allocator),
                return_type: None,
                body: Some(body),
                scope_id: Cell::new(None),
                pure: false,
                pife: false,
            },
            self.allocator,
        ))
    }

    fn emit_binary_operator(&self, expr: &BinaryOperatorExpr) -> oxc::Expression<'a> {
        let left = self.emit_expression(&expr.lhs);
        let right = self.emit_expression(&expr.rhs);
        let operator = match expr.operator {
            BinaryOperator::Equals => oxc_syntax::operator::BinaryOperator::Equality,
            BinaryOperator::NotEquals => oxc_syntax::operator::BinaryOperator::Inequality,
            BinaryOperator::Identical => oxc_syntax::operator::BinaryOperator::StrictEquality,
            BinaryOperator::NotIdentical => oxc_syntax::operator::BinaryOperator::StrictInequality,
            BinaryOperator::Minus => oxc_syntax::operator::BinaryOperator::Subtraction,
            BinaryOperator::Plus => oxc_syntax::operator::BinaryOperator::Addition,
            BinaryOperator::Divide => oxc_syntax::operator::BinaryOperator::Division,
            BinaryOperator::Multiply => oxc_syntax::operator::BinaryOperator::Multiplication,
            BinaryOperator::Modulo => oxc_syntax::operator::BinaryOperator::Remainder,
            BinaryOperator::And => {
                return oxc::Expression::LogicalExpression(OxcBox::new_in(
                    oxc::LogicalExpression {
                        span: Span::default(),
                        operator: oxc_syntax::operator::LogicalOperator::And,
                        left,
                        right,
                    },
                    self.allocator,
                ));
            }
            BinaryOperator::Or => {
                return oxc::Expression::LogicalExpression(OxcBox::new_in(
                    oxc::LogicalExpression {
                        span: Span::default(),
                        operator: oxc_syntax::operator::LogicalOperator::Or,
                        left,
                        right,
                    },
                    self.allocator,
                ));
            }
            BinaryOperator::NullishCoalesce => {
                return oxc::Expression::LogicalExpression(OxcBox::new_in(
                    oxc::LogicalExpression {
                        span: Span::default(),
                        operator: oxc_syntax::operator::LogicalOperator::Coalesce,
                        left,
                        right,
                    },
                    self.allocator,
                ));
            }
            BinaryOperator::BitwiseAnd => oxc_syntax::operator::BinaryOperator::BitwiseAnd,
            BinaryOperator::BitwiseOr => oxc_syntax::operator::BinaryOperator::BitwiseOR,
            BinaryOperator::Lower => oxc_syntax::operator::BinaryOperator::LessThan,
            BinaryOperator::LowerEquals => oxc_syntax::operator::BinaryOperator::LessEqualThan,
            BinaryOperator::Bigger => oxc_syntax::operator::BinaryOperator::GreaterThan,
            BinaryOperator::BiggerEquals => oxc_syntax::operator::BinaryOperator::GreaterEqualThan,
            BinaryOperator::In => oxc_syntax::operator::BinaryOperator::In,
            BinaryOperator::Exponentiation => oxc_syntax::operator::BinaryOperator::Exponential,
            BinaryOperator::Assign => {
                let target = match left {
                    oxc::Expression::Identifier(id) => {
                        oxc::AssignmentTarget::AssignmentTargetIdentifier(id)
                    }
                    oxc::Expression::StaticMemberExpression(member) => {
                        oxc::AssignmentTarget::StaticMemberExpression(member)
                    }
                    oxc::Expression::ComputedMemberExpression(member) => {
                        oxc::AssignmentTarget::ComputedMemberExpression(member)
                    }
                    _ => {
                        // Fallback: wrap in parenthesized expression isn't valid for assignment
                        // This shouldn't happen in practice for Angular output AST
                        unimplemented!("Cannot assign to this expression type");
                    }
                };
                return oxc::Expression::AssignmentExpression(OxcBox::new_in(
                    oxc::AssignmentExpression {
                        span: Span::default(),
                        operator: oxc_syntax::operator::AssignmentOperator::Assign,
                        left: target,
                        right,
                    },
                    self.allocator,
                ));
            }
            op => unimplemented!("Binary operator {:?}", op),
        };

        oxc::Expression::BinaryExpression(OxcBox::new_in(
            oxc::BinaryExpression {
                span: Span::default(),
                operator,
                left,
                right,
            },
            self.allocator,
        ))
    }

    fn emit_read_prop(&self, expr: &ReadPropExpr) -> oxc::Expression<'a> {
        let object = self.emit_expression(&expr.receiver);

        oxc::Expression::StaticMemberExpression(OxcBox::new_in(
            oxc::StaticMemberExpression {
                span: Span::default(),
                object,
                property: oxc::IdentifierName {
                    span: Span::default(),
                    name: self.atom(&expr.name),
                },
                optional: false,
            },
            self.allocator,
        ))
    }

    fn emit_read_key(&self, expr: &ReadKeyExpr) -> oxc::Expression<'a> {
        let object = self.emit_expression(&expr.receiver);
        let expression = self.emit_expression(&expr.index);

        oxc::Expression::ComputedMemberExpression(OxcBox::new_in(
            oxc::ComputedMemberExpression {
                span: Span::default(),
                object,
                expression,
                optional: false,
            },
            self.allocator,
        ))
    }

    fn emit_literal_array(&self, expr: &LiteralArrayExpr) -> oxc::Expression<'a> {
        let mut elements = OxcVec::new_in(self.allocator);
        for entry in &expr.entries {
            elements.push(oxc::ArrayExpressionElement::from(
                self.emit_expression(entry),
            ));
        }

        oxc::Expression::ArrayExpression(OxcBox::new_in(
            oxc::ArrayExpression {
                span: Span::default(),
                elements,
            },
            self.allocator,
        ))
    }

    fn emit_literal_map(&self, expr: &LiteralMapExpr) -> oxc::Expression<'a> {
        let mut properties = OxcVec::new_in(self.allocator);
        for entry in &expr.entries {
            let key = if entry.quoted {
                oxc::PropertyKey::StringLiteral(OxcBox::new_in(
                    oxc::StringLiteral {
                        span: Span::default(),
                        value: self.atom(&entry.key),
                        raw: None,
                        lone_surrogates: false,
                    },
                    self.allocator,
                ))
            } else {
                oxc::PropertyKey::StaticIdentifier(OxcBox::new_in(
                    oxc::IdentifierName {
                        span: Span::default(),
                        name: self.atom(&entry.key),
                    },
                    self.allocator,
                ))
            };
            let value = self.emit_expression(&entry.value);

            properties.push(oxc::ObjectPropertyKind::ObjectProperty(OxcBox::new_in(
                oxc::ObjectProperty {
                    span: Span::default(),
                    kind: oxc::PropertyKind::Init,
                    key,
                    value,
                    method: false,
                    shorthand: false,
                    computed: false,
                },
                self.allocator,
            )));
        }

        oxc::Expression::ObjectExpression(OxcBox::new_in(
            oxc::ObjectExpression {
                span: Span::default(),
                properties,
            },
            self.allocator,
        ))
    }

    fn emit_comma(&self, expr: &CommaExpr) -> oxc::Expression<'a> {
        let mut expressions = OxcVec::new_in(self.allocator);
        for part in &expr.parts {
            expressions.push(self.emit_expression(part));
        }

        oxc::Expression::SequenceExpression(OxcBox::new_in(
            oxc::SequenceExpression {
                span: Span::default(),
                expressions,
            },
            self.allocator,
        ))
    }

    fn emit_unary_operator(&self, expr: &UnaryOperatorExpr) -> oxc::Expression<'a> {
        let argument = self.emit_expression(&expr.expr);
        let operator = match expr.operator {
            UnaryOperator::Minus => oxc_syntax::operator::UnaryOperator::UnaryNegation,
            UnaryOperator::Plus => oxc_syntax::operator::UnaryOperator::UnaryPlus,
        };

        oxc::Expression::UnaryExpression(OxcBox::new_in(
            oxc::UnaryExpression {
                span: Span::default(),
                operator,
                argument,
            },
            self.allocator,
        ))
    }

    fn emit_parenthesized(&self, expr: &ParenthesizedExpr) -> oxc::Expression<'a> {
        let expression = self.emit_expression(&expr.expr);

        oxc::Expression::ParenthesizedExpression(OxcBox::new_in(
            oxc::ParenthesizedExpression {
                span: Span::default(),
                expression,
            },
            self.allocator,
        ))
    }

    fn emit_typeof(&self, expr: &TypeofExpr) -> oxc::Expression<'a> {
        let argument = self.emit_expression(&expr.expr);

        oxc::Expression::UnaryExpression(OxcBox::new_in(
            oxc::UnaryExpression {
                span: Span::default(),
                operator: oxc_syntax::operator::UnaryOperator::Typeof,
                argument,
            },
            self.allocator,
        ))
    }

    fn emit_void(&self, expr: &VoidExpr) -> oxc::Expression<'a> {
        let argument = self.emit_expression(&expr.expr);

        oxc::Expression::UnaryExpression(OxcBox::new_in(
            oxc::UnaryExpression {
                span: Span::default(),
                operator: oxc_syntax::operator::UnaryOperator::Void,
                argument,
            },
            self.allocator,
        ))
    }

    // -- Params & Statements -- //

    fn emit_fn_params(&self, params: &[FnParam]) -> oxc::FormalParameters<'a> {
        let mut items = OxcVec::new_in(self.allocator);
        for param in params {
            let binding = oxc::BindingPattern {
                kind: oxc::BindingPatternKind::BindingIdentifier(
                    self.make_binding_id_boxed(&param.name),
                ),
                type_annotation: None,
                optional: false,
            };
            items.push(oxc::FormalParameter {
                span: Span::default(),
                decorators: OxcVec::new_in(self.allocator),
                pattern: binding,
                accessibility: None,
                readonly: false,
                r#override: false,
            });
        }

        oxc::FormalParameters {
            span: Span::default(),
            kind: oxc::FormalParameterKind::FormalParameter,
            items,
            rest: None,
        }
    }

    pub fn emit_statements(&self, statements: &[Statement]) -> OxcVec<'a, oxc::Statement<'a>> {
        let mut result = OxcVec::new_in(self.allocator);
        for stmt in statements {
            result.push(self.emit_statement(stmt));
        }
        result
    }

    pub fn emit_statement(&self, stmt: &Statement) -> oxc::Statement<'a> {
        match stmt {
            Statement::Expression(expr_stmt) => {
                oxc::Statement::ExpressionStatement(OxcBox::new_in(
                    oxc::ExpressionStatement {
                        span: Span::default(),
                        expression: self.emit_expression(&expr_stmt.expr),
                    },
                    self.allocator,
                ))
            }
            Statement::Return(return_stmt) => oxc::Statement::ReturnStatement(OxcBox::new_in(
                oxc::ReturnStatement {
                    span: Span::default(),
                    argument: Some(self.emit_expression(&return_stmt.value)),
                },
                self.allocator,
            )),
            Statement::DeclareVar(decl_var) => {
                let kind = if decl_var.modifiers as u8 & StmtModifier::Final as u8 != 0 {
                    oxc::VariableDeclarationKind::Const
                } else {
                    oxc::VariableDeclarationKind::Let
                };
                let id = oxc::BindingPattern {
                    kind: oxc::BindingPatternKind::BindingIdentifier(
                        self.make_binding_id_boxed(&decl_var.name),
                    ),
                    type_annotation: None,
                    optional: false,
                };
                let init = decl_var.value.as_ref().map(|val| self.emit_expression(val));

                let mut declarations = OxcVec::new_in(self.allocator);
                declarations.push(oxc::VariableDeclarator {
                    span: Span::default(),
                    kind,
                    id,
                    init,
                    definite: false,
                });

                oxc::Statement::VariableDeclaration(OxcBox::new_in(
                    oxc::VariableDeclaration {
                        span: Span::default(),
                        kind,
                        declarations,
                        declare: false,
                    },
                    self.allocator,
                ))
            }
            Statement::IfStmt(if_stmt) => {
                let test = self.emit_expression(&if_stmt.condition);
                let consequent = oxc::Statement::BlockStatement(OxcBox::new_in(
                    oxc::BlockStatement {
                        span: Span::default(),
                        body: self.emit_statements(&if_stmt.true_case),
                        scope_id: Cell::new(None),
                    },
                    self.allocator,
                ));
                let alternate = if !if_stmt.false_case.is_empty() {
                    Some(oxc::Statement::BlockStatement(OxcBox::new_in(
                        oxc::BlockStatement {
                            span: Span::default(),
                            body: self.emit_statements(&if_stmt.false_case),
                            scope_id: Cell::new(None),
                        },
                        self.allocator,
                    )))
                } else {
                    None
                };

                oxc::Statement::IfStatement(OxcBox::new_in(
                    oxc::IfStatement {
                        span: Span::default(),
                        test,
                        consequent,
                        alternate,
                    },
                    self.allocator,
                ))
            }
            Statement::DeclareFn(decl_fn) => {
                let id = self.make_binding_id(&decl_fn.name);
                let params = self.emit_fn_params(&decl_fn.params);
                let body_stmts = self.emit_statements(&decl_fn.statements);
                let body = OxcBox::new_in(
                    oxc::FunctionBody {
                        span: Span::default(),
                        directives: OxcVec::new_in(self.allocator),
                        statements: body_stmts,
                    },
                    self.allocator,
                );

                oxc::Statement::FunctionDeclaration(OxcBox::new_in(
                    oxc::Function {
                        r#type: oxc::FunctionType::FunctionDeclaration,
                        span: Span::default(),
                        id: Some(id),
                        generator: false,
                        r#async: false,
                        declare: false,
                        type_parameters: None,
                        this_param: None,
                        params: OxcBox::new_in(params, self.allocator),
                        return_type: None,
                        body: Some(body),
                        scope_id: Cell::new(None),
                        pure: false,
                        pife: false,
                    },
                    self.allocator,
                ))
            }
            _ => unimplemented!("OxcEmitter::emit_statement for {:?}", stmt),
        }
    }
}
