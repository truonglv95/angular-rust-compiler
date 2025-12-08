/**
 * Serializer Tests
 *
 * Test suite for expression serializer
 * Mirrors angular/packages/compiler/test/expression_parser/serializer_spec.ts
 */

#[cfg(test)]
mod tests {
    use angular_compiler::expression_parser::{AST, parser::Parser, serializer::serialize};

    fn parse(expression: &str) -> AST {
        let parser = Parser::new();
        parser
            .parse_binding(expression, 0)
            .expect("Should parse successfully")
    }

    fn parse_action(expression: &str) -> AST {
        let parser = Parser::new();
        parser
            .parse_action(expression, 0)
            .expect("Should parse successfully")
    }

    #[test]
    fn serializes_unary_plus() {
        assert_eq!(serialize(&parse(" + 1234 ")), "+1234");
    }

    #[test]
    fn serializes_unary_negative() {
        assert_eq!(serialize(&parse(" - 1234 ")), "-1234");
    }

    #[test]
    fn serializes_binary_operations() {
        assert_eq!(serialize(&parse(" 1234   +   4321 ")), "1234 + 4321");
    }

    #[test]
    fn serializes_exponentiation() {
        assert_eq!(serialize(&parse(" 1  *  2  **  3 ")), "1 * 2 ** 3");
    }

    #[test]
    fn serializes_chains() {
        assert_eq!(serialize(&parse_action(" 1234;   4321 ")), "1234; 4321");
    }

    #[test]
    fn serializes_conditionals() {
        assert_eq!(
            serialize(&parse(" cond   ?   1234   :   4321 ")),
            "cond ? 1234 : 4321"
        );
    }

    #[test]
    fn serializes_this() {
        assert_eq!(serialize(&parse(" this ")), "this");
    }

    #[test]
    fn serializes_keyed_reads() {
        assert_eq!(serialize(&parse(" foo   [bar] ")), "foo[bar]");
    }

    #[test]
    fn serializes_keyed_write() {
        assert_eq!(
            serialize(&parse_action(" foo   [bar]   =   baz ")),
            "foo[bar] = baz"
        );
    }

    #[test]
    fn serializes_array_literals() {
        assert_eq!(
            serialize(&parse(" [   foo,   bar,   baz   ] ")),
            "[foo, bar, baz]"
        );
    }

    #[test]
    fn serializes_object_literals() {
        assert_eq!(
            serialize(&parse(" {   foo:   bar,   baz:   test   } ")),
            "{foo: bar, baz: test}"
        );
    }

    #[test]
    fn serializes_primitives() {
        // TypeScript uses single quotes for strings
        assert_eq!(serialize(&parse(" 'test' ")), "'test'");
        assert_eq!(serialize(&parse(" \"test\" ")), "'test'");
        assert_eq!(serialize(&parse(" true ")), "true");
        assert_eq!(serialize(&parse(" false ")), "false");
        assert_eq!(serialize(&parse(" 1234 ")), "1234");
        assert_eq!(serialize(&parse(" null ")), "null");
        assert_eq!(serialize(&parse(" undefined ")), "undefined");
    }

    #[test]
    fn escapes_string_literals() {
        // TypeScript uses single quotes and escapes single quotes inside
        assert_eq!(
            serialize(&parse(" 'Hello, \\'World\\'...' ")),
            "'Hello, \\'World\\'...'"
        );
        assert_eq!(
            serialize(&parse(" 'Hello, \\\"World\\\"...' ")),
            "'Hello, \"World\"...'"
        );
    }

    #[test]
    fn serializes_pipes() {
        // No parentheses around pipes in TypeScript serializer.ts
        assert_eq!(serialize(&parse(" foo   |   pipe ")), "foo | pipe");
    }

    #[test]
    fn serializes_not_prefixes() {
        assert_eq!(serialize(&parse(" !   foo ")), "!foo");
    }

    #[test]
    fn serializes_non_null_assertions() {
        assert_eq!(serialize(&parse(" foo   ! ")), "foo!");
    }

    #[test]
    fn serializes_property_reads() {
        assert_eq!(serialize(&parse(" foo   .   bar ")), "foo.bar");
    }

    #[test]
    fn serializes_property_writes() {
        assert_eq!(
            serialize(&parse_action(" foo   .   bar   =   baz ")),
            "foo.bar = baz"
        );
    }

    #[test]
    fn serializes_safe_property_reads() {
        assert_eq!(serialize(&parse(" foo   ?.   bar ")), "foo?.bar");
    }

    #[test]
    fn serializes_safe_keyed_reads() {
        assert_eq!(
            serialize(&parse(" foo   ?.   [   bar   ] ")),
            "foo?.[bar]"
        );
    }

    #[test]
    fn serializes_calls() {
        assert_eq!(serialize(&parse(" foo   (   ) ")), "foo()");
        assert_eq!(serialize(&parse(" foo   (   bar   ) ")), "foo(bar)");
        // Trailing comma is preserved - matches TypeScript serializer.ts
        assert_eq!(serialize(&parse(" foo   (   bar   ,   ) ")), "foo(bar, )");
        assert_eq!(
            serialize(&parse(" foo   (   bar   ,   baz   ) ")),
            "foo(bar, baz)"
        );
    }

    #[test]
    fn serializes_safe_calls() {
        assert_eq!(serialize(&parse(" foo   ?.   (   ) ")), "foo?.()");
        assert_eq!(serialize(&parse(" foo   ?.   (   bar   ) ")), "foo?.(bar)");
        // Trailing comma is preserved - matches TypeScript serializer.ts
        assert_eq!(
            serialize(&parse(" foo   ?.   (   bar   ,   ) ")),
            "foo?.(bar, )"
        );
        assert_eq!(
            serialize(&parse(" foo   ?.   (   bar   ,   baz   ) ")),
            "foo?.(bar, baz)"
        );
    }

    #[test]
    fn serializes_void_expressions() {
        assert_eq!(serialize(&parse(" void   0 ")), "void 0");
    }

    #[test]
    fn serializes_in_expressions() {
        assert_eq!(serialize(&parse(" foo   in   bar ")), "foo in bar");
    }
}

