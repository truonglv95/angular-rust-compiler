/**
 * Unparser
 *
 * Converts an AST back to a string representation.
 * This is used for testing the parser.
 * 
 * Mirrors angular/packages/compiler/test/expression_parser/utils/unparser.ts
 */

use angular_compiler::expression_parser::ast::*;

/// Quote regular expression pattern for escaping
const QUOTE_CHAR: char = '"';

/// Unparses an AST back to a string representation
pub fn unparse(ast: &AST) -> String {
    let mut unparser = Unparser::new();
    unparser.visit(ast);
    unparser.expression
}

struct Unparser {
    expression: String,
}

impl Unparser {
    fn new() -> Self {
        Unparser {
            expression: String::new(),
        }
    }

    fn visit(&mut self, ast: &AST) {
        match ast {
            AST::PropertyRead(node) => self.visit_property_read(node),
            AST::PropertyWrite(node) => self.visit_property_write(node),
            AST::SafePropertyRead(node) => self.visit_safe_property_read(node),
            AST::KeyedRead(node) => self.visit_keyed_read(node),
            AST::KeyedWrite(node) => self.visit_keyed_write(node),
            AST::SafeKeyedRead(node) => self.visit_safe_keyed_read(node),
            AST::Call(node) => self.visit_call(node),
            AST::SafeCall(node) => self.visit_safe_call(node),
            AST::Binary(node) => self.visit_binary(node),
            AST::Unary(node) => self.visit_unary(node),
            AST::Conditional(node) => self.visit_conditional(node),
            AST::Chain(node) => self.visit_chain(node),
            AST::LiteralPrimitive(node) => self.visit_literal_primitive(node),
            AST::LiteralArray(node) => self.visit_literal_array(node),
            AST::LiteralMap(node) => self.visit_literal_map(node),
            AST::Interpolation(node) => self.visit_interpolation(node),
            AST::PrefixNot(node) => self.visit_prefix_not(node),
            AST::NonNullAssert(node) => self.visit_non_null_assert(node),
            AST::BindingPipe(node) => self.visit_pipe(node),
            AST::ImplicitReceiver(_) => self.visit_implicit_receiver(),
            AST::ThisReceiver(_) => self.visit_this_receiver(),
            AST::TypeofExpression(node) => self.visit_typeof_expression(node),
            AST::VoidExpression(node) => self.visit_void_expression(node),
            AST::EmptyExpr(_) => {},
            AST::TemplateLiteral(node) => self.visit_template_literal(node),
            AST::TaggedTemplateLiteral(node) => self.visit_tagged_template_literal(node),
            AST::ParenthesizedExpression(node) => self.visit_parenthesized_expression(node),
            AST::RegularExpressionLiteral(node) => self.visit_regular_expression_literal(node),
        }
    }

    fn visit_property_read(&mut self, ast: &PropertyRead) {
        self.visit(&ast.receiver);
        // If receiver was ImplicitReceiver (empty), just add name
        // Otherwise add dot notation
        if self.is_implicit_receiver(&ast.receiver) {
            self.expression.push_str(&ast.name);
        } else {
            self.expression.push('.');
            self.expression.push_str(&ast.name);
        }
    }

    fn visit_property_write(&mut self, ast: &PropertyWrite) {
        self.visit(&ast.receiver);
        if self.is_implicit_receiver(&ast.receiver) {
            self.expression.push_str(&ast.name);
        } else {
            self.expression.push('.');
            self.expression.push_str(&ast.name);
        }
        self.expression.push_str(" = ");
        self.visit(&ast.value);
    }

    fn visit_safe_property_read(&mut self, ast: &SafePropertyRead) {
        self.visit(&ast.receiver);
        self.expression.push_str("?.");
        self.expression.push_str(&ast.name);
    }

    fn visit_keyed_read(&mut self, ast: &KeyedRead) {
        self.visit(&ast.receiver);
        self.expression.push('[');
        self.visit(&ast.key);
        self.expression.push(']');
    }

    fn visit_keyed_write(&mut self, ast: &KeyedWrite) {
        self.visit(&ast.receiver);
        self.expression.push('[');
        self.visit(&ast.key);
        self.expression.push_str("] = ");
        self.visit(&ast.value);
    }

    fn visit_safe_keyed_read(&mut self, ast: &SafeKeyedRead) {
        self.visit(&ast.receiver);
        self.expression.push_str("?.[");
        self.visit(&ast.key);
        self.expression.push(']');
    }

    fn visit_call(&mut self, ast: &Call) {
        self.visit(&ast.receiver);
        self.expression.push('(');
        let mut is_first = true;
        for arg in &ast.args {
            if !is_first {
                self.expression.push_str(", ");
            }
            is_first = false;
            self.visit(arg);
        }
        self.expression.push(')');
    }

    fn visit_safe_call(&mut self, ast: &SafeCall) {
        self.visit(&ast.receiver);
        self.expression.push_str("?.(");
        let mut is_first = true;
        for arg in &ast.args {
            if !is_first {
                self.expression.push_str(", ");
            }
            is_first = false;
            self.visit(arg);
        }
        self.expression.push(')');
    }

    fn visit_binary(&mut self, ast: &Binary) {
        self.visit(&ast.left);
        self.expression.push(' ');
        self.expression.push_str(&ast.operation);
        self.expression.push(' ');
        self.visit(&ast.right);
    }

    fn visit_unary(&mut self, ast: &Unary) {
        self.expression.push_str(&ast.operator);
        self.visit(&ast.expr);
    }

    fn visit_conditional(&mut self, ast: &Conditional) {
        self.visit(&ast.condition);
        self.expression.push_str(" ? ");
        self.visit(&ast.true_exp);
        self.expression.push_str(" : ");
        self.visit(&ast.false_exp);
    }

    fn visit_chain(&mut self, ast: &Chain) {
        let len = ast.expressions.len();
        for (i, expr) in ast.expressions.iter().enumerate() {
            self.visit(expr);
            if i == len - 1 {
                self.expression.push(';');
            } else {
                self.expression.push_str("; ");
            }
        }
    }

    fn visit_literal_primitive(&mut self, ast: &LiteralPrimitive) {
        match ast {
            LiteralPrimitive::String { value, .. } => {
                // TypeScript unparser uses double quotes and escapes double quotes
                let escaped = value.replace(QUOTE_CHAR, "\"");
                self.expression.push('"');
                self.expression.push_str(&escaped);
                self.expression.push('"');
            }
            LiteralPrimitive::Number { value, .. } => {
                self.expression.push_str(&value.to_string());
            }
            LiteralPrimitive::Boolean { value, .. } => {
                self.expression.push_str(&value.to_string());
            }
            LiteralPrimitive::Null { .. } => {
                self.expression.push_str("null");
            }
            LiteralPrimitive::Undefined { .. } => {
                self.expression.push_str("undefined");
            }
        }
    }

    fn visit_literal_array(&mut self, ast: &LiteralArray) {
        self.expression.push('[');
        let mut is_first = true;
        for expr in &ast.expressions {
            if !is_first {
                self.expression.push_str(", ");
            }
            is_first = false;
            self.visit(expr);
        }
        self.expression.push(']');
    }

    fn visit_literal_map(&mut self, ast: &LiteralMap) {
        self.expression.push('{');
        let mut is_first = true;
        for (key, value) in ast.keys.iter().zip(ast.values.iter()) {
            if !is_first {
                self.expression.push_str(", ");
            }
            is_first = false;
            // TypeScript uses JSON.stringify for quoted keys (which produces double quotes)
            if key.quoted {
                self.expression.push('"');
                self.expression.push_str(&key.key);
                self.expression.push('"');
            } else {
                self.expression.push_str(&key.key);
            }
            self.expression.push_str(": ");
            self.visit(value);
        }
        self.expression.push('}');
    }

    fn visit_interpolation(&mut self, ast: &Interpolation) {
        for i in 0..ast.strings.len() {
            self.expression.push_str(&ast.strings[i]);
            if i < ast.expressions.len() {
                self.expression.push_str("{{ ");
                self.visit(&ast.expressions[i]);
                self.expression.push_str(" }}");
            }
        }
    }

    fn visit_prefix_not(&mut self, ast: &PrefixNot) {
        self.expression.push('!');
        self.visit(&ast.expression);
    }

    fn visit_non_null_assert(&mut self, ast: &NonNullAssert) {
        self.visit(&ast.expression);
        self.expression.push('!');
    }

    fn visit_pipe(&mut self, ast: &BindingPipe) {
        // TypeScript unparser wraps pipe in parentheses
        self.expression.push('(');
        self.visit(&ast.exp);
        self.expression.push_str(" | ");
        self.expression.push_str(&ast.name);
        for arg in &ast.args {
            self.expression.push(':');
            self.visit(arg);
        }
        self.expression.push(')');
    }

    fn visit_implicit_receiver(&mut self) {
        // TypeScript: visitImplicitReceiver does nothing (returns empty)
    }

    fn visit_this_receiver(&mut self) {
        // TypeScript: visitThisReceiver does nothing (returns empty)
    }

    fn visit_typeof_expression(&mut self, ast: &TypeofExpression) {
        self.expression.push_str("typeof ");
        self.visit(&ast.expression);
    }

    fn visit_void_expression(&mut self, ast: &VoidExpression) {
        self.expression.push_str("void ");
        self.visit(&ast.expression);
    }

    fn visit_template_literal(&mut self, ast: &TemplateLiteral) {
        self.expression.push('`');
        for i in 0..ast.elements.len() {
            self.visit_template_literal_element(&ast.elements[i]);
            if i < ast.expressions.len() {
                self.expression.push_str("${");
                self.visit(&ast.expressions[i]);
                self.expression.push('}');
            }
        }
        self.expression.push('`');
    }

    fn visit_template_literal_element(&mut self, ast: &TemplateLiteralElement) {
        self.expression.push_str(&ast.text);
    }

    fn visit_tagged_template_literal(&mut self, ast: &TaggedTemplateLiteral) {
        self.visit(&ast.tag);
        self.visit_template_literal(&ast.template);
    }

    fn visit_parenthesized_expression(&mut self, ast: &ParenthesizedExpression) {
        self.expression.push('(');
        self.visit(&ast.expression);
        self.expression.push(')');
    }

    fn visit_regular_expression_literal(&mut self, ast: &RegularExpressionLiteral) {
        self.expression.push('/');
        self.expression.push_str(&ast.body);
        self.expression.push('/');
        if let Some(flags) = &ast.flags {
            self.expression.push_str(flags);
        }
    }

    /// Helper to check if an AST is an ImplicitReceiver (returns empty string when visited)
    fn is_implicit_receiver(&self, ast: &AST) -> bool {
        matches!(ast, AST::ImplicitReceiver(_) | AST::ThisReceiver(_))
    }
}
