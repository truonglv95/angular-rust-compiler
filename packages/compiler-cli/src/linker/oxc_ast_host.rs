//! OXC AST Host Implementation
//!
//! Implements the `AstHost` trait for the OXC AST.

use crate::linker::ast::{AstHost, AstNode, Range};
use oxc_ast::ast::{Argument, ArrayExpressionElement, Expression, PropertyKey, Statement};
use oxc_span::Span;

/// Wrapper around various OXC AST node types to satisfy AstNode traits and unify disjoint enum types.
/// OXC uses `inherit_variants!` which creates distinct enums (Expression, Argument, ArrayExpressionElement)
/// that share variants but are not type-compatible. This wrapper unifies them.
#[derive(Debug, Clone, Copy)]
pub enum OxcNode<'a> {
    Expression(&'a Expression<'a>),
    ArrayElement(&'a ArrayExpressionElement<'a>),
    Argument(&'a Argument<'a>),
    Statement(&'a Statement<'a>),
}

impl<'a> AstNode for OxcNode<'a> {}

pub struct OxcAstHost<'a> {
    source: &'a str,
}

impl<'a> OxcAstHost<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }

    fn get_span(&self, node: &OxcNode<'a>) -> Span {
        match node {
            OxcNode::Expression(e) => e.span(),
            OxcNode::ArrayElement(e) => e.span(),
            OxcNode::Argument(e) => e.span(),
            OxcNode::Statement(s) => s.span(),
        }
    }
}

// Ensure GetSpan is in scope
use oxc_span::GetSpan;

impl<'a> AstHost<OxcNode<'a>> for OxcAstHost<'a> {
    fn get_symbol_name(&self, node: &OxcNode<'a>) -> Option<String> {
        match node {
            OxcNode::Expression(e) => match e {
                Expression::Identifier(ident) => Some(ident.name.to_string()),
                _ => None,
            },
            // Add other wrappers if necessary
            OxcNode::ArrayElement(ArrayExpressionElement::Identifier(ident)) => {
                Some(ident.name.to_string())
            }
            OxcNode::Argument(Argument::Identifier(ident)) => Some(ident.name.to_string()),
            _ => None,
        }
    }

    fn is_string_literal(&self, node: &OxcNode<'a>) -> bool {
        match node {
            OxcNode::Expression(Expression::StringLiteral(_)) => true,
            OxcNode::Expression(Expression::TemplateLiteral(t)) => t.quasis.len() == 1,
            OxcNode::ArrayElement(ArrayExpressionElement::StringLiteral(_)) => true,
            OxcNode::ArrayElement(ArrayExpressionElement::TemplateLiteral(t)) => {
                t.quasis.len() == 1
            }
            OxcNode::Argument(Argument::StringLiteral(_)) => true,
            OxcNode::Argument(Argument::TemplateLiteral(t)) => t.quasis.len() == 1,
            _ => false,
        }
    }

    fn parse_string_literal(&self, node: &OxcNode<'a>) -> Result<String, String> {
        match node {
            OxcNode::Expression(Expression::StringLiteral(l)) => Ok(l.value.to_string()),
            OxcNode::Expression(Expression::TemplateLiteral(l)) => {
                if l.quasis.len() == 1 {
                    Ok(l.quasis[0].value.raw.to_string())
                } else {
                    Err("Template literal with substitutions not supported".to_string())
                }
            }
            OxcNode::ArrayElement(ArrayExpressionElement::StringLiteral(l)) => {
                Ok(l.value.to_string())
            }
            OxcNode::ArrayElement(ArrayExpressionElement::TemplateLiteral(l)) => {
                if l.quasis.len() == 1 {
                    Ok(l.quasis[0].value.raw.to_string())
                } else {
                    Err("Template literal with substitutions not supported".to_string())
                }
            }
            OxcNode::Argument(Argument::StringLiteral(l)) => Ok(l.value.to_string()),
            OxcNode::Argument(Argument::TemplateLiteral(l)) => {
                if l.quasis.len() == 1 {
                    Ok(l.quasis[0].value.raw.to_string())
                } else {
                    Err("Template literal with substitutions not supported".to_string())
                }
            }
            _ => Err("Not a string literal".to_string()),
        }
    }

    fn is_numeric_literal(&self, node: &OxcNode<'a>) -> bool {
        match node {
            OxcNode::Expression(Expression::NumericLiteral(_)) => true,
            OxcNode::ArrayElement(ArrayExpressionElement::NumericLiteral(_)) => true,
            OxcNode::Argument(Argument::NumericLiteral(_)) => true,
            _ => false,
        }
    }

    fn parse_numeric_literal(&self, node: &OxcNode<'a>) -> Result<f64, String> {
        match node {
            OxcNode::Expression(Expression::NumericLiteral(l)) => Ok(l.value),
            OxcNode::ArrayElement(ArrayExpressionElement::NumericLiteral(l)) => Ok(l.value),
            OxcNode::Argument(Argument::NumericLiteral(l)) => Ok(l.value),
            _ => Err("Not a numeric literal".to_string()),
        }
    }

    fn is_boolean_literal(&self, node: &OxcNode<'a>) -> bool {
        match node {
            OxcNode::Expression(Expression::BooleanLiteral(_)) => true,
            OxcNode::ArrayElement(ArrayExpressionElement::BooleanLiteral(_)) => true,
            OxcNode::Argument(Argument::BooleanLiteral(_)) => true,
            _ => false,
        }
    }

    fn parse_boolean_literal(&self, node: &OxcNode<'a>) -> Result<bool, String> {
        match node {
            OxcNode::Expression(Expression::BooleanLiteral(l)) => Ok(l.value),
            OxcNode::ArrayElement(ArrayExpressionElement::BooleanLiteral(l)) => Ok(l.value),
            OxcNode::Argument(Argument::BooleanLiteral(l)) => Ok(l.value),
            _ => Err("Not a boolean literal".to_string()),
        }
    }

    fn is_null(&self, node: &OxcNode<'a>) -> bool {
        match node {
            OxcNode::Expression(Expression::NullLiteral(_)) => true,
            OxcNode::ArrayElement(ArrayExpressionElement::NullLiteral(_)) => true,
            OxcNode::Argument(Argument::NullLiteral(_)) => true,
            _ => false,
        }
    }

    fn is_array_literal(&self, node: &OxcNode<'a>) -> bool {
        match node {
            OxcNode::Expression(Expression::ArrayExpression(_)) => true,
            OxcNode::ArrayElement(ArrayExpressionElement::ArrayExpression(_)) => true,
            OxcNode::Argument(Argument::ArrayExpression(_)) => true,
            _ => false,
        }
    }

    fn parse_array_literal(&self, node: &OxcNode<'a>) -> Result<Vec<OxcNode<'a>>, String> {
        let elements = match node {
            OxcNode::Expression(Expression::ArrayExpression(e)) => &e.elements,
            OxcNode::ArrayElement(ArrayExpressionElement::ArrayExpression(e)) => &e.elements,
            OxcNode::Argument(Argument::ArrayExpression(e)) => &e.elements,
            _ => return Err("Not an array literal".to_string()),
        };

        let mut result = Vec::new();
        for element in elements {
            // Check if it's a spread or elision, which we might support or error on.
            // But we need to wrap it in OxcNode::ArrayElement
            // We can check strictly if check explicitly against Elision/Spread.
            match element {
                ArrayExpressionElement::SpreadElement(_) => {
                    return Err("Spread elements not supported in linker array literals".to_string())
                }
                ArrayExpressionElement::Elision(_) => {
                    return Err("Elisions not supported".to_string())
                }
                _ => result.push(OxcNode::ArrayElement(element)),
            }
        }
        Ok(result)
    }

    fn is_object_literal(&self, node: &OxcNode<'a>) -> bool {
        match node {
            OxcNode::Expression(Expression::ObjectExpression(_)) => true,
            OxcNode::ArrayElement(ArrayExpressionElement::ObjectExpression(_)) => true,
            OxcNode::Argument(Argument::ObjectExpression(_)) => true,
            _ => false,
        }
    }

    fn parse_object_literal(
        &self,
        node: &OxcNode<'a>,
    ) -> Result<std::collections::HashMap<String, OxcNode<'a>>, String> {
        let properties = match node {
            OxcNode::Expression(Expression::ObjectExpression(e)) => &e.properties,
            OxcNode::ArrayElement(ArrayExpressionElement::ObjectExpression(e)) => &e.properties,
            OxcNode::Argument(Argument::ObjectExpression(e)) => &e.properties,
            _ => return Err("Not an object literal".to_string()),
        };

        let mut result = std::collections::HashMap::new();
        for prop in properties {
            match prop {
                oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) => {
                    let key = match &p.key {
                        PropertyKey::StaticIdentifier(ident) => ident.name.to_string(),
                        PropertyKey::StringLiteral(lit) => lit.value.to_string(),
                        _ => return Err("Unsupported object key type".to_string()),
                    };
                    result.insert(key, OxcNode::Expression(&p.value));
                }
                _ => return Err("Unsupported object property type (spread)".to_string()),
            }
        }
        Ok(result)
    }

    fn is_function_expression(&self, node: &OxcNode<'a>) -> bool {
        match node {
            OxcNode::Expression(
                Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_),
            ) => true,
            OxcNode::ArrayElement(
                ArrayExpressionElement::FunctionExpression(_)
                | ArrayExpressionElement::ArrowFunctionExpression(_),
            ) => true,
            OxcNode::Argument(
                Argument::FunctionExpression(_) | Argument::ArrowFunctionExpression(_),
            ) => true,
            _ => false,
        }
    }

    fn parse_return_value(&self, node: &OxcNode<'a>) -> Result<OxcNode<'a>, String> {
        // Handle ArrowFunctionExpression
        if let Some((body, is_expr)) = match node {
            OxcNode::Expression(Expression::ArrowFunctionExpression(a)) => {
                Some((&a.body, a.expression))
            }
            OxcNode::ArrayElement(ArrayExpressionElement::ArrowFunctionExpression(a)) => {
                Some((&a.body, a.expression))
            }
            OxcNode::Argument(Argument::ArrowFunctionExpression(a)) => {
                Some((&a.body, a.expression))
            }
            _ => None,
        } {
            if is_expr {
                if let Some(Statement::ExpressionStatement(stmt)) = body.statements.first() {
                    return Ok(OxcNode::Expression(&stmt.expression));
                }
                return Err("Expression body not found in arrow function".to_string());
            }
            // Block body arrow function - fall through to generic block handling?
            // Oxc's ArrowFunctionExpression body IS a FunctionBody, same as FunctionExpression.
            // But for expression body, Oxc wraps it in a FunctionBody with one expression statement?
            // Actually Oxc AST: `body: FunctionBody`. `expression: bool`.
            // If `expression` is true, body has 1 statement which is ExpressionStatement.
            // If `expression` is false, it's a block.
        }

        // Handle FunctionExpression and Block-body ArrowFunctionExpression
        let body_opt = match node {
            OxcNode::Expression(Expression::FunctionExpression(f)) => f.body.as_ref(),
            OxcNode::ArrayElement(ArrayExpressionElement::FunctionExpression(f)) => f.body.as_ref(),
            OxcNode::Argument(Argument::FunctionExpression(f)) => f.body.as_ref(),
            OxcNode::Expression(Expression::ArrowFunctionExpression(a)) => Some(&a.body),
            OxcNode::ArrayElement(ArrayExpressionElement::ArrowFunctionExpression(a)) => {
                Some(&a.body)
            }
            OxcNode::Argument(Argument::ArrowFunctionExpression(a)) => Some(&a.body),
            _ => return Err("Not a function expression".to_string()),
        };

        if let Some(body) = body_opt {
            for stmt in &body.statements {
                if let Statement::ReturnStatement(ret) = stmt {
                    if let Some(arg) = &ret.argument {
                        return Ok(OxcNode::Expression(arg));
                    }
                }
            }
            Err("No return statement found in function body".to_string())
        } else {
            Err("Function has no body".to_string())
        }
    }

    fn parse_parameters(&self, _fn_node: &OxcNode<'a>) -> Result<Vec<OxcNode<'a>>, String> {
        Err("Parameter parsing not implemented".to_string())
    }

    fn is_call_expression(&self, node: &OxcNode<'a>) -> bool {
        match node {
            OxcNode::Expression(Expression::CallExpression(_)) => true,
            OxcNode::ArrayElement(ArrayExpressionElement::CallExpression(_)) => true,
            OxcNode::Argument(Argument::CallExpression(_)) => true,
            _ => false,
        }
    }

    fn parse_callee(&self, node: &OxcNode<'a>) -> Result<OxcNode<'a>, String> {
        match node {
            OxcNode::Expression(Expression::CallExpression(e)) => {
                Ok(OxcNode::Expression(&e.callee))
            }
            OxcNode::ArrayElement(ArrayExpressionElement::CallExpression(e)) => {
                Ok(OxcNode::Expression(&e.callee))
            }
            OxcNode::Argument(Argument::CallExpression(e)) => Ok(OxcNode::Expression(&e.callee)),
            _ => Err("Not a call expression".to_string()),
        }
    }

    fn parse_arguments(&self, node: &OxcNode<'a>) -> Result<Vec<OxcNode<'a>>, String> {
        let args = match node {
            OxcNode::Expression(Expression::CallExpression(e)) => &e.arguments,
            OxcNode::ArrayElement(ArrayExpressionElement::CallExpression(e)) => &e.arguments,
            OxcNode::Argument(Argument::CallExpression(e)) => &e.arguments,
            _ => return Err("Not a call expression".to_string()),
        };

        let mut result = Vec::new();
        for arg in args {
            // Check for spreads if necessary
            match arg {
                Argument::SpreadElement(_) => {
                    return Err("Spread arguments not supported".to_string())
                }
                _ => result.push(OxcNode::Argument(arg)),
            }
        }
        Ok(result)
    }

    fn get_range(&self, node: &OxcNode<'a>) -> Result<Range, String> {
        let span = self.get_span(node);
        Ok(Range {
            start_pos: span.start as usize,
            end_pos: span.end as usize,
            start_line: 0,
            start_col: 0,
        })
    }

    fn print_node(&self, node: &OxcNode<'a>) -> String {
        let span = self.get_span(node);
        self.source[span.start as usize..span.end as usize].to_string()
    }
}
