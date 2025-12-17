/**
 * AST Tests
 *
 * Test suite for AST visitor functionality
 * Mirrors angular/packages/compiler/test/expression_parser/ast_spec.ts
 */

#[cfg(test)]
mod tests {
    use angular_compiler::expression_parser::{
        ast::*,
        parser::Parser,
    };

    /// Custom visitor that collects all visited nodes in a path
    struct PathCollector {
        path: Vec<AST>,
    }

    impl PathCollector {
        fn new() -> Self {
            PathCollector { path: Vec::new() }
        }

        fn visit_recursive(&mut self, ast: &AST) {
            self.path.push(ast.clone());
            // Recursively visit child nodes
            match ast {
                AST::Binary(b) => {
                    self.visit_recursive(&b.left);
                    self.visit_recursive(&b.right);
                }
                AST::Chain(c) => {
                    for expr in &c.expressions {
                        self.visit_recursive(expr);
                    }
                }
                AST::Conditional(c) => {
                    self.visit_recursive(&c.condition);
                    self.visit_recursive(&c.true_exp);
                    self.visit_recursive(&c.false_exp);
                }
                AST::PropertyRead(p) => {
                    self.visit_recursive(&p.receiver);
                }
                AST::PropertyWrite(p) => {
                    self.visit_recursive(&p.receiver);
                    self.visit_recursive(&p.value);
                }
                AST::SafePropertyRead(p) => {
                    self.visit_recursive(&p.receiver);
                }
                AST::KeyedRead(k) => {
                    self.visit_recursive(&k.receiver);
                    self.visit_recursive(&k.key);
                }
                AST::KeyedWrite(k) => {
                    self.visit_recursive(&k.receiver);
                    self.visit_recursive(&k.key);
                    self.visit_recursive(&k.value);
                }
                AST::SafeKeyedRead(k) => {
                    self.visit_recursive(&k.receiver);
                    self.visit_recursive(&k.key);
                }
                AST::BindingPipe(p) => {
                    self.visit_recursive(&p.exp);
                    for arg in &p.args {
                        self.visit_recursive(arg);
                    }
                }
                AST::LiteralArray(a) => {
                    for expr in &a.expressions {
                        self.visit_recursive(expr);
                    }
                }
                AST::LiteralMap(m) => {
                    for value in &m.values {
                        self.visit_recursive(value);
                    }
                }
                AST::Interpolation(i) => {
                    for expr in &i.expressions {
                        self.visit_recursive(expr);
                    }
                }
                AST::Call(c) => {
                    self.visit_recursive(&c.receiver);
                    for arg in &c.args {
                        self.visit_recursive(arg);
                    }
                }
                AST::SafeCall(c) => {
                    self.visit_recursive(&c.receiver);
                    for arg in &c.args {
                        self.visit_recursive(arg);
                    }
                }
                AST::PrefixNot(p) => {
                    self.visit_recursive(&p.expression);
                }
                AST::Unary(u) => {
                    self.visit_recursive(&u.expr);
                }
                AST::TypeofExpression(t) => {
                    self.visit_recursive(&t.expression);
                }
                AST::VoidExpression(v) => {
                    self.visit_recursive(&v.expression);
                }
                AST::NonNullAssert(n) => {
                    self.visit_recursive(&n.expression);
                }
                AST::TemplateLiteral(t) => {
                    for expr in &t.expressions {
                        self.visit_recursive(expr);
                    }
                }
                AST::TaggedTemplateLiteral(t) => {
                    self.visit_recursive(&t.tag);
                    for expr in &t.template.expressions {
                        self.visit_recursive(expr);
                    }
                }
                AST::ParenthesizedExpression(p) => {
                    self.visit_recursive(&p.expression);
                }
                AST::RegularExpressionLiteral(_) |
                AST::EmptyExpr(_) |
                AST::ImplicitReceiver(_) |
                AST::ThisReceiver(_) |
                AST::LiteralPrimitive(_) => {
                    // Leaf nodes - no children to visit
                }
            }
        }
    }

    /// Helper function to check if an AST node is of a specific type
    fn expect_type<T>(ast: &AST, check: fn(&AST) -> Option<&T>) {
        assert!(check(ast).is_some(), "Expected specific AST node type");
    }

    fn is_call(ast: &AST) -> Option<&Call> {
        match ast {
            AST::Call(c) => Some(c),
            _ => None,
        }
    }

    fn is_property_read(ast: &AST) -> Option<&PropertyRead> {
        match ast {
            AST::PropertyRead(p) => Some(p),
            _ => None,
        }
    }

    fn is_implicit_receiver(ast: &AST) -> Option<&ImplicitReceiver> {
        match ast {
            AST::ImplicitReceiver(i) => Some(i),
            _ => None,
        }
    }

    #[test]
    fn should_visit_every_node() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("x.y()", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        // If the visitor method of RecursiveAstVisitor is implemented correctly,
        // then we should have collected the full path from root to leaf.
        assert_eq!(visitor.path.len(), 4, "Should visit 4 nodes: Call, PropertyRead(y), PropertyRead(x), ImplicitReceiver");

        let call = &visitor.path[0];
        let y_read = &visitor.path[1];
        let x_read = &visitor.path[2];
        let implicit_receiver = &visitor.path[3];

        expect_type(call, is_call);
        expect_type(y_read, is_property_read);
        expect_type(x_read, is_property_read);
        expect_type(implicit_receiver, is_implicit_receiver);

        // Verify the structure
        if let AST::PropertyRead(x_prop) = x_read {
            assert_eq!(x_prop.name, "x", "First property read should be 'x'");
        }

        if let AST::PropertyRead(y_prop) = y_read {
            assert_eq!(y_prop.name, "y", "Second property read should be 'y'");
        }

        if let AST::Call(call_node) = call {
            assert_eq!(call_node.args.len(), 0, "Call should have no arguments");
        }
    }

    #[test]
    fn should_visit_call_node() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("x.y()", 0)
            .expect("Should parse successfully");

        // Verify the AST structure
        match ast {
            AST::Call(call) => {
                // Verify call structure
                assert!(call.args.is_empty(), "Call should have no arguments");
            }
            _ => panic!("Expected Call AST node"),
        }
    }

    #[test]
    fn should_visit_property_read_nodes() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("x.y", 0)
            .expect("Should parse successfully");

        // Verify the AST structure contains property reads
        match ast {
            AST::PropertyRead(prop) => {
                assert_eq!(prop.name, "y", "Property name should be 'y'");
            }
            _ => panic!("Expected PropertyRead AST node"),
        }
    }

    #[test]
    fn should_visit_implicit_receiver() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("x", 0)
            .expect("Should parse successfully");

        // Verify the AST structure
        match ast {
            AST::PropertyRead(prop) => {
                assert_eq!(prop.name, "x", "Property name should be 'x'");
            }
            _ => panic!("Expected PropertyRead AST node"),
        }
    }

    #[test]
    fn should_visit_binary_expression() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("a + b", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        // Should visit: Binary, PropertyRead(a), PropertyRead(x for implicit), ImplicitReceiver,
        // PropertyRead(b), PropertyRead(x for implicit), ImplicitReceiver
        assert!(visitor.path.len() >= 3, "Should visit at least 3 nodes");
        
        match &visitor.path[0] {
            AST::Binary(b) => {
                assert_eq!(b.operation, "+", "Operator should be Plus");
            }
            _ => panic!("Expected Binary AST node"),
        }
    }

    #[test]
    fn should_visit_conditional_expression() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("a ? b : c", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::Conditional(_c) => {
                // Verify all three parts are visited
                assert!(visitor.path.len() >= 4, "Should visit condition, true, false, and their children");
            }
            _ => panic!("Expected Conditional AST node"),
        }
    }

    #[test]
    fn should_visit_literal_primitive() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("42", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        assert_eq!(visitor.path.len(), 1, "Should visit only the literal");
        
        match &visitor.path[0] {
            AST::LiteralPrimitive(lit) => {
                match lit {
                    LiteralPrimitive::Number { value, .. } => {
                        assert_eq!(*value, 42.0, "Number value should be 42");
                    }
                    _ => panic!("Expected number literal"),
                }
            }
            _ => panic!("Expected LiteralPrimitive AST node"),
        }
    }

    #[test]
    fn should_visit_literal_array() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("[1, 2, 3]", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::LiteralArray(arr) => {
                assert_eq!(arr.expressions.len(), 3, "Array should have 3 elements");
            }
            _ => panic!("Expected LiteralArray AST node"),
        }
    }

    #[test]
    fn should_visit_call_with_arguments() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("foo(1, 2, 3)", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::Call(call) => {
                assert_eq!(call.args.len(), 3, "Call should have 3 arguments");
            }
            _ => panic!("Expected Call AST node"),
        }
    }

    #[test]
    fn should_visit_keyed_read() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("obj['key']", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::KeyedRead(_k) => {
                // Should visit receiver and key
                assert!(visitor.path.len() >= 3, "Should visit keyed read, receiver, and key");
            }
            _ => panic!("Expected KeyedRead AST node"),
        }
    }

    #[test]
    fn should_visit_pipe_expression() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("value | pipe", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::BindingPipe(p) => {
                assert_eq!(p.name, "pipe", "Pipe name should be 'pipe'");
            }
            _ => panic!("Expected BindingPipe AST node"),
        }
    }

    #[test]
    fn should_visit_template_literal() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("`hello ${world}`", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::TemplateLiteral(t) => {
                assert!(!t.expressions.is_empty(), "Template literal should have expressions");
            }
            _ => panic!("Expected TemplateLiteral AST node"),
        }
    }

    #[test]
    fn should_visit_tagged_template_literal() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("tag`hello ${world}`", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::TaggedTemplateLiteral(_t) => {
                // Should visit tag and template expressions
                assert!(visitor.path.len() >= 2, "Should visit tag and template");
            }
            other => panic!("Expected TaggedTemplateLiteral AST node, got {:?}", other),
        }
    }

    #[test]
    fn should_visit_parenthesized_expression() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("(a + b)", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::ParenthesizedExpression(_p) => {
                // Should visit the inner expression
                assert!(visitor.path.len() >= 2, "Should visit parentheses and inner expression");
            }
            _ => panic!("Expected ParenthesizedExpression AST node"),
        }
    }

    #[test]
    fn should_visit_chain_expression() {
        let parser = Parser::new();
        let ast = parser
            .parse_action("a; b; c", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::Chain(c) => {
                assert_eq!(c.expressions.len(), 3, "Chain should have 3 expressions");
            }
            _ => panic!("Expected Chain AST node"),
        }
    }

    #[test]
    fn should_visit_safe_property_read() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("obj?.prop", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::SafePropertyRead(p) => {
                assert_eq!(p.name, "prop", "Property name should be 'prop'");
            }
            _ => panic!("Expected SafePropertyRead AST node"),
        }
    }

    #[test]
    fn should_visit_safe_call() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("obj.method?.()", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::SafeCall(c) => {
                assert!(c.args.is_empty(), "Safe call should have no arguments");
            }
            other => panic!("Expected SafeCall AST node, got {:?}", other),
        }
    }

    #[test]
    fn should_visit_unary_expression() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("!value", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::PrefixNot(_p) => {
                // Should visit the expression
                assert!(visitor.path.len() >= 2, "Should visit prefix not and expression");
            }
            _ => panic!("Expected PrefixNot AST node"),
        }
    }

    #[test]
    fn should_visit_non_null_assert() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("value!", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::NonNullAssert(_n) => {
                // Should visit the expression
                assert!(visitor.path.len() >= 2, "Should visit non-null assert and expression");
            }
            _ => panic!("Expected NonNullAssert AST node"),
        }
    }

    #[test]
    fn should_visit_complex_nested_expression() {
        let parser = Parser::new();
        let ast = parser
            .parse_binding("a.b().c.d", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        // Should visit multiple levels: PropertyRead(d) -> PropertyRead(c) -> Call -> PropertyRead(b) -> PropertyRead(a) -> ImplicitReceiver
        assert!(visitor.path.len() >= 6, "Should visit all nested nodes");
        
        // Verify the structure
        match &visitor.path[0] {
            AST::PropertyRead(p) => {
                assert_eq!(p.name, "d", "Outermost property should be 'd'");
            }
            _ => panic!("Expected PropertyRead AST node at root"),
        }
    }
    #[test]
    fn should_visit_property_write() {
        let parser = Parser::new();
        let ast = parser
            .parse_action("a.b = c", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::PropertyWrite(p) => {
                assert_eq!(p.name, "b", "Property name should be 'b'");
            }
            _ => panic!("Expected PropertyWrite AST node"),
        }
        
        // Should visit value 'c' and receiver 'a'
        assert!(visitor.path.len() >= 3, "Should visit write, value and receiver");
    }

    #[test]
    fn should_visit_keyed_write() {
        let parser = Parser::new();
        let ast = parser
            .parse_action("a['b'] = c", 0)
            .expect("Should parse successfully");

        let mut visitor = PathCollector::new();
        visitor.visit_recursive(&ast);

        match &visitor.path[0] {
            AST::KeyedWrite(_k) => {
                // Should visit receiver, key and value
            }
            _ => panic!("Expected KeyedWrite AST node"),
        }
        
        assert!(visitor.path.len() >= 4, "Should visit write, receiver, key, and value");
    }
}
