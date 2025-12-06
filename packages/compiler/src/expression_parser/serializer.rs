/**
 * Angular Expression Serializer
 *
 * Serializes AST back to string format
 * Mirrors packages/compiler/src/expression_parser/serializer.ts
 */

use super::ast::*;

/// Serialize AST to string
pub fn serialize(ast: &AST) -> String {
    let mut visitor = SerializeExpressionVisitor;
    visit_ast(&mut visitor, ast)
}

struct SerializeExpressionVisitor;

fn visit_ast(visitor: &mut SerializeExpressionVisitor, ast: &AST) -> String {
    match ast {
        AST::Binary(b) => visitor.visit_binary(b),
        AST::PropertyRead(p) => visitor.visit_property_read(p),
        AST::SafePropertyRead(p) => visitor.visit_safe_property_read(p),
        AST::KeyedRead(k) => visitor.visit_keyed_read(k),
        AST::SafeKeyedRead(k) => visitor.visit_safe_keyed_read(k),
        AST::LiteralPrimitive(l) => visitor.visit_literal_primitive(l),
        AST::LiteralArray(a) => visitor.visit_literal_array(a),
        AST::LiteralMap(m) => visitor.visit_literal_map(m),
        AST::Interpolation(i) => visitor.visit_interpolation(i),
        AST::Conditional(c) => visitor.visit_conditional(c),
        AST::BindingPipe(p) => visitor.visit_pipe(p),
        AST::Call(c) => visitor.visit_call(c),
        AST::SafeCall(c) => visitor.visit_safe_call(c),
        AST::Chain(c) => visitor.visit_chain(c),
        AST::PrefixNot(p) => visitor.visit_prefix_not(p),
        AST::Unary(u) => visitor.visit_unary(u),
        AST::TypeofExpression(t) => visitor.visit_typeof(t),
        AST::VoidExpression(v) => visitor.visit_void(v),
        AST::NonNullAssert(n) => visitor.visit_non_null_assert(n),
        AST::TemplateLiteral(t) => visitor.visit_template_literal(t),
        AST::TaggedTemplateLiteral(t) => visitor.visit_tagged_template(t),
        AST::ParenthesizedExpression(p) => visitor.visit_parenthesized(p),
        AST::RegularExpressionLiteral(r) => visitor.visit_regexp(r),
        AST::ImplicitReceiver(_) => visitor.visit_implicit_receiver(),
        AST::ThisReceiver(_) => visitor.visit_this_receiver(),
        AST::EmptyExpr(_) => String::new(),
    }
}

impl SerializeExpressionVisitor {
    fn visit_unary(&mut self, ast: &Unary) -> String {
        format!("{}{}", ast.operator, visit_ast(self, &ast.expr))
    }

    fn visit_binary(&mut self, ast: &Binary) -> String {
        format!(
            "{} {} {}",
            visit_ast(self, &ast.left),
            ast.operation,
            visit_ast(self, &ast.right)
        )
    }

    fn visit_chain(&mut self, ast: &Chain) -> String {
        ast.expressions
            .iter()
            .map(|e| visit_ast(self, e))
            .collect::<Vec<_>>()
            .join("; ")
    }

    fn visit_conditional(&mut self, ast: &Conditional) -> String {
        format!(
            "{} ? {} : {}",
            visit_ast(self, &ast.condition),
            visit_ast(self, &ast.true_exp),
            visit_ast(self, &ast.false_exp)
        )
    }

    fn visit_this_receiver(&mut self) -> String {
        "this".to_string()
    }

    fn visit_implicit_receiver(&mut self) -> String {
        String::new()
    }

    fn visit_keyed_read(&mut self, ast: &KeyedRead) -> String {
        format!(
            "{}[{}]",
            visit_ast(self, &ast.receiver),
            visit_ast(self, &ast.key)
        )
    }

    fn visit_literal_array(&mut self, ast: &LiteralArray) -> String {
        let elements = ast
            .expressions
            .iter()
            .map(|e| visit_ast(self, e))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}]", elements)
    }

    fn visit_literal_map(&mut self, ast: &LiteralMap) -> String {
        let pairs: Vec<String> = ast
            .keys
            .iter()
            .zip(ast.values.iter())
            .map(|(key, value)| {
                let key_str = if key.quoted {
                    format!("'{}'", key.key)
                } else {
                    key.key.clone()
                };
                format!("{}: {}", key_str, visit_ast(self, value))
            })
            .collect();
        format!("{{{}}}", pairs.join(", "))
    }

    fn visit_literal_primitive(&mut self, ast: &LiteralPrimitive) -> String {
        match ast {
            LiteralPrimitive::String { value, .. } => format!("'{}'", value.replace('\'', "\\'")),
            LiteralPrimitive::Number { value, .. } => value.to_string(),
            LiteralPrimitive::Boolean { value, .. } => value.to_string(),
            LiteralPrimitive::Null { .. } => "null".to_string(),
            LiteralPrimitive::Undefined { .. } => "undefined".to_string(),
        }
    }

    fn visit_pipe(&mut self, ast: &BindingPipe) -> String {
        let args = if ast.args.is_empty() {
            String::new()
        } else {
            format!(
                ":{}",
                ast.args
                    .iter()
                    .map(|a| visit_ast(self, a))
                    .collect::<Vec<_>>()
                    .join(":")
            )
        };
        format!("{} | {}{}", visit_ast(self, &ast.exp), ast.name, args)
    }

    fn visit_prefix_not(&mut self, ast: &PrefixNot) -> String {
        format!("!{}", visit_ast(self, &ast.expression))
    }

    fn visit_non_null_assert(&mut self, ast: &NonNullAssert) -> String {
        format!("{}!", visit_ast(self, &ast.expression))
    }

    fn visit_property_read(&mut self, ast: &PropertyRead) -> String {
        let receiver_str = visit_ast(self, &ast.receiver);
        if receiver_str.is_empty() {
            // Implicit receiver
            ast.name.clone()
        } else {
            format!("{}.{}", receiver_str, ast.name)
        }
    }

    fn visit_safe_property_read(&mut self, ast: &SafePropertyRead) -> String {
        format!("{}?.{}", visit_ast(self, &ast.receiver), ast.name)
    }

    fn visit_safe_keyed_read(&mut self, ast: &SafeKeyedRead) -> String {
        format!(
            "{}?.[{}]",
            visit_ast(self, &ast.receiver),
            visit_ast(self, &ast.key)
        )
    }

    fn visit_call(&mut self, ast: &Call) -> String {
        let args = ast
            .args
            .iter()
            .map(|a| visit_ast(self, a))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}({})", visit_ast(self, &ast.receiver), args)
    }

    fn visit_safe_call(&mut self, ast: &SafeCall) -> String {
        let args = ast
            .args
            .iter()
            .map(|a| visit_ast(self, a))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}?.({})", visit_ast(self, &ast.receiver), args)
    }

    fn visit_interpolation(&mut self, ast: &Interpolation) -> String {
        let mut result = String::new();
        for (idx, s) in ast.strings.iter().enumerate() {
            result.push_str(s);
            if idx < ast.expressions.len() {
                result.push_str("{{");
                result.push_str(&visit_ast(self, &ast.expressions[idx]));
                result.push_str("}}");
            }
        }
        result
    }

    fn visit_typeof(&mut self, ast: &TypeofExpression) -> String {
        format!("typeof {}", visit_ast(self, &ast.expression))
    }

    fn visit_void(&mut self, ast: &VoidExpression) -> String {
        format!("void {}", visit_ast(self, &ast.expression))
    }

    fn visit_template_literal(&mut self, ast: &TemplateLiteral) -> String {
        let mut result = String::from("`");
        for (idx, elem) in ast.elements.iter().enumerate() {
            result.push_str(&elem.text);
            if idx < ast.expressions.len() {
                result.push_str("${");
                result.push_str(&visit_ast(self, &ast.expressions[idx]));
                result.push('}');
            }
        }
        result.push('`');
        result
    }

    fn visit_tagged_template(&mut self, ast: &TaggedTemplateLiteral) -> String {
        format!("{}{}", visit_ast(self, &ast.tag), self.visit_template_literal(&ast.template))
    }

    fn visit_parenthesized(&mut self, ast: &ParenthesizedExpression) -> String {
        format!("({})", visit_ast(self, &ast.expression))
    }

    fn visit_regexp(&mut self, ast: &RegularExpressionLiteral) -> String {
        if let Some(ref flags) = ast.flags {
            format!("/{}/{}", ast.body, flags)
        } else {
            format!("/{}/", ast.body)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_binary() {
        let ast = AST::Binary(Binary {
            span: ParseSpan::new(0, 5),
            source_span: AbsoluteSourceSpan::new(0, 5),
            operation: "+".to_string(),
            left: Box::new(AST::LiteralPrimitive(LiteralPrimitive::number(
                ParseSpan::new(0, 1),
                AbsoluteSourceSpan::new(0, 1),
                1.0,
            ))),
            right: Box::new(AST::LiteralPrimitive(LiteralPrimitive::number(
                ParseSpan::new(4, 5),
                AbsoluteSourceSpan::new(4, 5),
                2.0,
            ))),
        });

        let result = serialize(&ast);
        assert_eq!(result, "1 + 2");
    }

    #[test]
    fn test_serialize_property_read() {
        let ast = AST::PropertyRead(PropertyRead {
            span: ParseSpan::new(0, 4),
            source_span: AbsoluteSourceSpan::new(0, 4),
            name_span: AbsoluteSourceSpan::new(0, 4),
            receiver: Box::new(AST::ImplicitReceiver(ImplicitReceiver::new(
                ParseSpan::new(0, 0),
                AbsoluteSourceSpan::new(0, 0),
            ))),
            name: "name".to_string(),
        });

        let result = serialize(&ast);
        assert_eq!(result, "name");
    }
}
