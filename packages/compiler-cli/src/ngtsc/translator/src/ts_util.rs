use oxc_ast::ast::Expression;
use oxc_ast::AstBuilder;
use oxc_span::Span;
use oxc_syntax::number::NumberBase;
use oxc_syntax::operator::UnaryOperator;

use oxc_span::Atom;

/// Creates a TypeScript node representing a numeric value.
pub fn ts_numeric_expression<'a>(builder: &AstBuilder<'a>, value: f64) -> Expression<'a> {
    // As of TypeScript 5.3 negative numbers are represented as `prefixUnaryOperator` and passing a
    // negative number (even as a string) into `createNumericLiteral` will result in an error.
    if value < 0.0 {
        let abs_value = value.abs();
        let raw = builder.allocator.alloc_str(&abs_value.to_string());
        let literal = builder.alloc_numeric_literal(Span::default(), abs_value, Some(Atom::from(raw)), NumberBase::Decimal);
        let operand = Expression::NumericLiteral(literal);
        oxc_ast::ast::Expression::UnaryExpression(builder.alloc(builder.unary_expression(Span::default(), UnaryOperator::UnaryNegation, operand)))
    } else {
        let raw = builder.allocator.alloc_str(&value.to_string());
        let literal = builder.alloc_numeric_literal(Span::default(), value, Some(Atom::from(raw)), NumberBase::Decimal);
        Expression::NumericLiteral(literal)
    }
}
