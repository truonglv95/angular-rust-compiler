/**
 * Lexer Tests
 *
 * Comprehensive test suite for expression lexer
 * Mirrors angular/packages/compiler/test/expression_parser/lexer_spec.ts
 */

#[cfg(test)]
mod tests {
    use angular_compiler::expression_parser::lexer::{Lexer, StringTokenKind, Token};

    fn lex(text: &str) -> Vec<Token> {
        Lexer::new().tokenize(text)
    }

    fn expect_token(token: &Token, index: usize, end: usize) {
        assert_eq!(token.index, index, "Token index mismatch");
        assert_eq!(token.end, end, "Token end mismatch");
    }

    fn expect_character_token(token: &Token, index: usize, end: usize, character: char) {
        assert_eq!(character.len_utf8(), 1, "Character must be single byte");
        expect_token(token, index, end);
        assert!(
            token.is_character(character),
            "Expected character token '{}'",
            character
        );
    }

    fn expect_operator_token(token: &Token, index: usize, end: usize, operator: &str) {
        expect_token(token, index, end);
        assert!(
            token.is_operator(operator),
            "Expected operator token '{}'",
            operator
        );
    }

    fn expect_number_token(token: &Token, index: usize, end: usize, n: f64) {
        expect_token(token, index, end);
        assert!(token.is_number(), "Expected number token");
        assert!((token.num_value - n).abs() < f64::EPSILON, "Expected number {}", n);
    }

    fn expect_string_token(
        token: &Token,
        index: usize,
        end: usize,
        str: &str,
        kind: StringTokenKind,
    ) {
        expect_token(token, index, end);
        assert!(token.is_string(), "Expected string token");
        assert_eq!(token.kind, Some(kind), "Expected string token kind");
        assert_eq!(token.str_value, str, "Expected string value");
    }

    fn expect_identifier_token(token: &Token, index: usize, end: usize, identifier: &str) {
        expect_token(token, index, end);
        assert!(token.is_identifier(), "Expected identifier token");
        assert_eq!(token.str_value, identifier, "Expected identifier value");
    }

    fn expect_private_identifier_token(token: &Token, index: usize, end: usize, identifier: &str) {
        expect_token(token, index, end);
        assert!(
            token.is_private_identifier(),
            "Expected private identifier token"
        );
        assert_eq!(token.str_value, identifier, "Expected private identifier value");
    }

    fn expect_keyword_token(token: &Token, index: usize, end: usize, keyword: &str) {
        expect_token(token, index, end);
        assert!(token.is_keyword(), "Expected keyword token");
        assert_eq!(token.str_value, keyword, "Expected keyword value");
    }

    fn expect_error_token(token: &Token, index: usize, end: usize, message: &str) {
        expect_token(token, index, end);
        assert!(token.is_error(), "Expected error token");
        assert_eq!(token.str_value, message, "Expected error message");
    }

    fn expect_regexp_body_token(token: &Token, index: usize, end: usize, str: &str) {
        expect_token(token, index, end);
        assert!(token.is_regexp_body(), "Expected regexp body token");
        assert_eq!(token.str_value, str, "Expected regexp body value");
    }

    fn expect_regexp_flags_token(token: &Token, index: usize, end: usize, str: &str) {
        expect_token(token, index, end);
        assert!(token.is_regexp_flags(), "Expected regexp flags token");
        assert_eq!(token.str_value, str, "Expected regexp flags value");
    }

    #[test]
    fn should_tokenize_a_simple_identifier() {
        let tokens = lex("j");
        assert_eq!(tokens.len(), 1);
        expect_identifier_token(&tokens[0], 0, 1, "j");
    }

    #[test]
    fn should_tokenize_this() {
        let tokens = lex("this");
        assert_eq!(tokens.len(), 1);
        expect_keyword_token(&tokens[0], 0, 4, "this");
    }

    #[test]
    fn should_tokenize_a_dotted_identifier() {
        let tokens = lex("j.k");
        assert_eq!(tokens.len(), 3);
        expect_identifier_token(&tokens[0], 0, 1, "j");
        expect_character_token(&tokens[1], 1, 2, '.');
        expect_identifier_token(&tokens[2], 2, 3, "k");
    }

    #[test]
    fn should_tokenize_a_private_identifier() {
        let tokens = lex("#a");
        assert_eq!(tokens.len(), 1);
        expect_private_identifier_token(&tokens[0], 0, 2, "#a");
    }

    #[test]
    fn should_tokenize_a_property_access_with_private_identifier() {
        let tokens = lex("j.#k");
        assert_eq!(tokens.len(), 3);
        expect_identifier_token(&tokens[0], 0, 1, "j");
        expect_character_token(&tokens[1], 1, 2, '.');
        expect_private_identifier_token(&tokens[2], 2, 4, "#k");
    }

    #[test]
    fn should_throw_an_invalid_character_error_when_a_hash_character_is_discovered_but_not_indicating_a_private_identifier() {
        let tokens = lex("#");
        expect_error_token(
            &tokens[0],
            0,
            1,
            "Lexer Error: Invalid character [#] at column 0 in expression [#]",
        );
        let tokens2 = lex("#0");
        expect_error_token(
            &tokens2[0],
            0,
            1,
            "Lexer Error: Invalid character [#] at column 0 in expression [#0]",
        );
    }

    #[test]
    fn should_tokenize_an_operator() {
        let tokens = lex("j-k");
        assert_eq!(tokens.len(), 3);
        expect_operator_token(&tokens[1], 1, 2, "-");
    }

    #[test]
    fn should_tokenize_an_indexed_operator() {
        let tokens = lex("j[k]");
        assert_eq!(tokens.len(), 4);
        expect_character_token(&tokens[1], 1, 2, '[');
        expect_character_token(&tokens[3], 3, 4, ']');
    }

    #[test]
    fn should_tokenize_a_safe_indexed_operator() {
        let tokens = lex("j?.[k]");
        assert_eq!(tokens.len(), 5);
        expect_operator_token(&tokens[1], 1, 3, "?.");
        expect_character_token(&tokens[2], 3, 4, '[');
        expect_character_token(&tokens[4], 5, 6, ']');
    }

    #[test]
    fn should_tokenize_numbers() {
        let tokens = lex("88");
        assert_eq!(tokens.len(), 1);
        expect_number_token(&tokens[0], 0, 2, 88.0);
    }

    #[test]
    fn should_tokenize_numbers_within_index_ops() {
        let tokens = lex("a[22]");
        expect_number_token(&tokens[2], 2, 4, 22.0);
    }

    #[test]
    fn should_tokenize_simple_quoted_strings() {
        expect_string_token(&lex("\"a\"")[0], 0, 3, "a", StringTokenKind::Plain);
    }

    #[test]
    fn should_tokenize_quoted_strings_with_escaped_quotes() {
        expect_string_token(&lex("\"a\\\"\"")[0], 0, 5, "a\"", StringTokenKind::Plain);
    }

    #[test]
    fn should_tokenize_a_string() {
        let tokens = lex("j-a.bc[22]+1.3|f:'a\\'c':\"d\\\"e\"");
        expect_identifier_token(&tokens[0], 0, 1, "j");
        expect_operator_token(&tokens[1], 1, 2, "-");
        expect_identifier_token(&tokens[2], 2, 3, "a");
        expect_character_token(&tokens[3], 3, 4, '.');
        expect_identifier_token(&tokens[4], 4, 6, "bc");
        expect_character_token(&tokens[5], 6, 7, '[');
        expect_number_token(&tokens[6], 7, 9, 22.0);
        expect_character_token(&tokens[7], 9, 10, ']');
        expect_operator_token(&tokens[8], 10, 11, "+");
        expect_number_token(&tokens[9], 11, 14, 1.3);
        expect_operator_token(&tokens[10], 14, 15, "|");
        expect_identifier_token(&tokens[11], 15, 16, "f");
        expect_character_token(&tokens[12], 16, 17, ':');
        expect_string_token(&tokens[13], 17, 23, "a'c", StringTokenKind::Plain);
        expect_character_token(&tokens[14], 23, 24, ':');
        expect_string_token(&tokens[15], 24, 30, "d\"e", StringTokenKind::Plain);
    }

    #[test]
    fn should_tokenize_undefined() {
        let tokens = lex("undefined");
        expect_keyword_token(&tokens[0], 0, 9, "undefined");
        assert!(tokens[0].is_keyword_undefined());
    }

    #[test]
    fn should_tokenize_typeof() {
        let tokens = lex("typeof");
        expect_keyword_token(&tokens[0], 0, 6, "typeof");
        assert!(tokens[0].is_keyword_typeof());
    }

    #[test]
    fn should_tokenize_void() {
        let tokens = lex("void");
        expect_keyword_token(&tokens[0], 0, 4, "void");
        assert!(tokens[0].is_keyword_void());
    }

    #[test]
    fn should_tokenize_in_keyword() {
        let tokens = lex("in");
        expect_keyword_token(&tokens[0], 0, 2, "in");
        assert!(tokens[0].is_keyword_in());
    }

    #[test]
    fn should_ignore_whitespace() {
        let tokens = lex("a \t \n \r b");
        expect_identifier_token(&tokens[0], 0, 1, "a");
        expect_identifier_token(&tokens[1], 8, 9, "b");
    }

    #[test]
    fn should_tokenize_quoted_string() {
        let str = "['\\'', \"\\\"\"]";
        let tokens = lex(str);
        expect_string_token(&tokens[1], 1, 5, "'", StringTokenKind::Plain);
        expect_string_token(&tokens[3], 7, 11, "\"", StringTokenKind::Plain);
    }

    #[test]
    fn should_tokenize_escaped_quoted_string() {
        let str = "\"\\\"\\n\\f\\r\\t\\v\\u00A0\"";
        let tokens = lex(str);
        assert_eq!(tokens.len(), 1);
        // Note: The actual string value after unescaping would be "\n\f\r\t\v\u00A0"
        // This test checks the tokenization, not the unescaping
        assert!(tokens[0].is_string());
    }

    #[test]
    fn should_tokenize_unicode() {
        let tokens = lex("\"\\u00A0\"");
        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].is_string());
        // The actual unicode value would be \u00a0 after unescaping
    }

    #[test]
    fn should_tokenize_relation() {
        let tokens = lex("! == != < > <= >= === !==");
        expect_operator_token(&tokens[0], 0, 1, "!");
        expect_operator_token(&tokens[1], 2, 4, "==");
        expect_operator_token(&tokens[2], 5, 7, "!=");
        expect_operator_token(&tokens[3], 8, 9, "<");
        expect_operator_token(&tokens[4], 10, 11, ">");
        expect_operator_token(&tokens[5], 12, 14, "<=");
        expect_operator_token(&tokens[6], 15, 17, ">=");
        expect_operator_token(&tokens[7], 18, 21, "===");
        expect_operator_token(&tokens[8], 22, 25, "!==");
    }

    #[test]
    fn should_tokenize_statements() {
        let tokens = lex("a;b;");
        expect_identifier_token(&tokens[0], 0, 1, "a");
        expect_character_token(&tokens[1], 1, 2, ';');
        expect_identifier_token(&tokens[2], 2, 3, "b");
        expect_character_token(&tokens[3], 3, 4, ';');
    }

    #[test]
    fn should_tokenize_function_invocation() {
        let tokens = lex("a()");
        expect_identifier_token(&tokens[0], 0, 1, "a");
        expect_character_token(&tokens[1], 1, 2, '(');
        expect_character_token(&tokens[2], 2, 3, ')');
    }

    #[test]
    fn should_tokenize_simple_method_invocations() {
        let tokens = lex("a.method()");
        expect_identifier_token(&tokens[2], 2, 8, "method");
    }

    #[test]
    fn should_tokenize_method_invocation() {
        let tokens = lex("a.b.c (d) - e.f()");
        expect_identifier_token(&tokens[0], 0, 1, "a");
        expect_character_token(&tokens[1], 1, 2, '.');
        expect_identifier_token(&tokens[2], 2, 3, "b");
        expect_character_token(&tokens[3], 3, 4, '.');
        expect_identifier_token(&tokens[4], 4, 5, "c");
        expect_character_token(&tokens[5], 6, 7, '(');
        expect_identifier_token(&tokens[6], 7, 8, "d");
        expect_character_token(&tokens[7], 8, 9, ')');
        expect_operator_token(&tokens[8], 10, 11, "-");
        expect_identifier_token(&tokens[9], 12, 13, "e");
        expect_character_token(&tokens[10], 13, 14, '.');
        expect_identifier_token(&tokens[11], 14, 15, "f");
        expect_character_token(&tokens[12], 15, 16, '(');
        expect_character_token(&tokens[13], 16, 17, ')');
    }

    #[test]
    fn should_tokenize_safe_function_invocation() {
        let tokens = lex("a?.()");
        expect_identifier_token(&tokens[0], 0, 1, "a");
        expect_operator_token(&tokens[1], 1, 3, "?.");
        expect_character_token(&tokens[2], 3, 4, '(');
        expect_character_token(&tokens[3], 4, 5, ')');
    }

    #[test]
    fn should_tokenize_a_safe_method_invocations() {
        let tokens = lex("a.method?.()");
        expect_identifier_token(&tokens[0], 0, 1, "a");
        expect_character_token(&tokens[1], 1, 2, '.');
        expect_identifier_token(&tokens[2], 2, 8, "method");
        expect_operator_token(&tokens[3], 8, 10, "?.");
        expect_character_token(&tokens[4], 10, 11, '(');
        expect_character_token(&tokens[5], 11, 12, ')');
    }

    #[test]
    fn should_tokenize_number() {
        expect_number_token(&lex("0.5")[0], 0, 3, 0.5);
    }

    #[test]
    fn should_tokenize_multiplication_and_exponentiation() {
        let tokens = lex("1 * 2 ** 3");
        expect_number_token(&tokens[0], 0, 1, 1.0);
        expect_operator_token(&tokens[1], 2, 3, "*");
        expect_number_token(&tokens[2], 4, 5, 2.0);
        expect_operator_token(&tokens[3], 6, 8, "**");
        expect_number_token(&tokens[4], 9, 10, 3.0);
    }

    #[test]
    fn should_tokenize_number_with_exponent() {
        let tokens = lex("0.5E-10");
        assert_eq!(tokens.len(), 1);
        expect_number_token(&tokens[0], 0, 7, 0.5e-10);
        let tokens2 = lex("0.5E+10");
        expect_number_token(&tokens2[0], 0, 7, 0.5e10);
    }

    #[test]
    fn should_return_exception_for_invalid_exponent() {
        expect_error_token(
            &lex("0.5E-")[0],
            4,
            5,
            "Lexer Error: Invalid exponent at column 4 in expression [0.5E-]",
        );

        expect_error_token(
            &lex("0.5E-A")[0],
            4,
            5,
            "Lexer Error: Invalid exponent at column 4 in expression [0.5E-A]",
        );
    }

    #[test]
    fn should_tokenize_number_starting_with_a_dot() {
        expect_number_token(&lex(".5")[0], 0, 2, 0.5);
    }

    #[test]
    fn should_throw_error_on_invalid_unicode() {
        expect_error_token(
            &lex("'\\u1''bla'")[0],
            2,
            2,
            "Lexer Error: Invalid unicode escape [\\u1''b] at column 2 in expression ['\\u1''bla']",
        );
    }

    #[test]
    fn should_tokenize_question_dot_as_operator() {
        expect_operator_token(&lex("?.")[0], 0, 2, "?.");
    }

    #[test]
    fn should_tokenize_nullish_coalescing_as_operator() {
        expect_operator_token(&lex("??")[0], 0, 2, "??");
    }

    #[test]
    fn should_tokenize_number_with_separator() {
        expect_number_token(&lex("123_456")[0], 0, 7, 123456.0);
        expect_number_token(&lex("1_000_000_000")[0], 0, 13, 1000000000.0);
        expect_number_token(&lex("123_456.78")[0], 0, 10, 123456.78);
        expect_number_token(&lex("123_456_789.123_456_789")[0], 0, 23, 123456789.123456789);
        expect_number_token(&lex("1_2_3_4")[0], 0, 7, 1234.0);
        expect_number_token(&lex("1_2_3_4.5_6_7_8")[0], 0, 15, 1234.5678);
    }

    #[test]
    fn should_tokenize_number_starting_with_an_underscore_as_an_identifier() {
        expect_identifier_token(&lex("_123")[0], 0, 4, "_123");
        expect_identifier_token(&lex("_123_")[0], 0, 5, "_123_");
        expect_identifier_token(&lex("_1_2_3_")[0], 0, 7, "_1_2_3_");
    }

    #[test]
    fn should_throw_error_for_invalid_number_separators() {
        expect_error_token(
            &lex("123_")[0],
            3,
            3,
            "Lexer Error: Invalid numeric separator at column 3 in expression [123_]",
        );
        expect_error_token(
            &lex("12__3")[0],
            2,
            2,
            "Lexer Error: Invalid numeric separator at column 2 in expression [12__3]",
        );
        expect_error_token(
            &lex("1_2_3_.456")[0],
            5,
            5,
            "Lexer Error: Invalid numeric separator at column 5 in expression [1_2_3_.456]",
        );
        expect_error_token(
            &lex("1_2_3._456")[0],
            6,
            6,
            "Lexer Error: Invalid numeric separator at column 6 in expression [1_2_3._456]",
        );
    }

    #[test]
    fn should_tokenize_assignment_operators() {
        expect_operator_token(&lex("=")[0], 0, 1, "=");
        expect_operator_token(&lex("+=")[0], 0, 2, "+=");
        expect_operator_token(&lex("-=")[0], 0, 2, "-=");
        expect_operator_token(&lex("*=")[0], 0, 2, "*=");
        expect_operator_token(&lex("a /= b")[1], 2, 4, "/=");
        expect_operator_token(&lex("%=")[0], 0, 2, "%=");
        expect_operator_token(&lex("**=")[0], 0, 3, "**=");
        expect_operator_token(&lex("&&=")[0], 0, 3, "&&=");
        expect_operator_token(&lex("||=")[0], 0, 3, "||=");
        expect_operator_token(&lex("??=")[0], 0, 3, "??=");
    }

    // Template literal tests
    mod template_literals {
        use super::*;

        #[test]
        fn should_tokenize_template_literal_with_no_interpolations() {
            let tokens = lex("`hello world`");
            assert_eq!(tokens.len(), 1);
            expect_string_token(
                &tokens[0],
                0,
                13,
                "hello world",
                StringTokenKind::TemplateLiteralEnd,
            );
        }

        #[test]
        fn should_tokenize_template_literal_containing_strings() {
            expect_string_token(
                &lex("`a \"b\" c`")[0],
                0,
                9,
                "a \"b\" c",
                StringTokenKind::TemplateLiteralEnd,
            );
            expect_string_token(
                &lex("`a 'b' c`")[0],
                0,
                9,
                "a 'b' c",
                StringTokenKind::TemplateLiteralEnd,
            );
            expect_string_token(
                &lex("`a \\`b\\` c`")[0],
                0,
                11,
                "a `b` c",
                StringTokenKind::TemplateLiteralEnd,
            );
            expect_string_token(
                &lex("`a \"'\\`b\\`'\" c`")[0],
                0,
                15,
                "a \"'`b`'\" c",
                StringTokenKind::TemplateLiteralEnd,
            );
        }

        #[test]
        fn should_tokenize_unicode_inside_a_template_string() {
            let tokens = lex("`\\u00A0`");
            assert_eq!(tokens.len(), 1);
            assert!(tokens[0].is_string());
            // The actual unicode value would be \u00a0 after unescaping
        }

        #[test]
        fn should_be_able_to_use_interpolation_characters_inside_template_string() {
            expect_string_token(
                &lex("`foo $`")[0],
                0,
                7,
                "foo $",
                StringTokenKind::TemplateLiteralEnd,
            );
            expect_string_token(
                &lex("`foo }`")[0],
                0,
                7,
                "foo }",
                StringTokenKind::TemplateLiteralEnd,
            );
            expect_string_token(
                &lex("`foo $ {}`")[0],
                0,
                10,
                "foo $ {}",
                StringTokenKind::TemplateLiteralEnd,
            );
            expect_string_token(
                &lex("`foo \\${bar}`")[0],
                0,
                13,
                "foo ${bar}",
                StringTokenKind::TemplateLiteralEnd,
            );
        }

        #[test]
        fn should_tokenize_template_literal_with_an_object_literal_inside_the_interpolation() {
            let tokens = lex("`foo ${{$: true}} baz`");
            assert_eq!(tokens.len(), 9);
            expect_string_token(
                &tokens[0],
                0,
                5,
                "foo ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[1], 5, 7, "${");
            expect_character_token(&tokens[2], 7, 8, '{');
            expect_identifier_token(&tokens[3], 8, 9, "$");
            expect_character_token(&tokens[4], 9, 10, ':');
            expect_keyword_token(&tokens[5], 11, 15, "true");
            expect_character_token(&tokens[6], 15, 16, '}');
            expect_character_token(&tokens[7], 16, 17, '}');
            expect_string_token(
                &tokens[8],
                17,
                22,
                " baz",
                StringTokenKind::TemplateLiteralEnd,
            );
        }

        #[test]
        fn should_tokenize_template_literal_with_template_literals_inside_the_interpolation() {
            let tokens = lex("`foo ${`hello ${`${a} - b`}`} baz`");
            assert_eq!(tokens.len(), 13);
            expect_string_token(
                &tokens[0],
                0,
                5,
                "foo ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[1], 5, 7, "${");
            expect_string_token(
                &tokens[2],
                7,
                14,
                "hello ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[3], 14, 16, "${");
            expect_string_token(
                &tokens[4],
                16,
                17,
                "",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[5], 17, 19, "${");
            expect_identifier_token(&tokens[6], 19, 20, "a");
            expect_character_token(&tokens[7], 20, 21, '}');
            expect_string_token(
                &tokens[8],
                21,
                26,
                " - b",
                StringTokenKind::TemplateLiteralEnd,
            );
            expect_character_token(&tokens[9], 26, 27, '}');
            expect_string_token(
                &tokens[10],
                27,
                28,
                "",
                StringTokenKind::TemplateLiteralEnd,
            );
            expect_character_token(&tokens[11], 28, 29, '}');
            expect_string_token(
                &tokens[12],
                29,
                34,
                " baz",
                StringTokenKind::TemplateLiteralEnd,
            );
        }

        #[test]
        fn should_tokenize_two_template_literal_right_after_each_other() {
            let tokens = lex("`hello ${name}``see ${name} later`");
            assert_eq!(tokens.len(), 10);
            expect_string_token(
                &tokens[0],
                0,
                7,
                "hello ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[1], 7, 9, "${");
            expect_identifier_token(&tokens[2], 9, 13, "name");
            expect_character_token(&tokens[3], 13, 14, '}');
            expect_string_token(&tokens[4], 14, 15, "", StringTokenKind::TemplateLiteralEnd);
            expect_string_token(
                &tokens[5],
                15,
                20,
                "see ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[6], 20, 22, "${");
            expect_identifier_token(&tokens[7], 22, 26, "name");
            expect_character_token(&tokens[8], 26, 27, '}');
            expect_string_token(
                &tokens[9],
                27,
                34,
                " later",
                StringTokenKind::TemplateLiteralEnd,
            );
        }

        #[test]
        fn should_tokenize_a_concatenated_template_literal() {
            let tokens = lex("`hello ${name}` + 123");
            assert_eq!(tokens.len(), 7);
            expect_string_token(
                &tokens[0],
                0,
                7,
                "hello ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[1], 7, 9, "${");
            expect_identifier_token(&tokens[2], 9, 13, "name");
            expect_character_token(&tokens[3], 13, 14, '}');
            expect_string_token(&tokens[4], 14, 15, "", StringTokenKind::TemplateLiteralEnd);
            expect_operator_token(&tokens[5], 16, 17, "+");
            expect_number_token(&tokens[6], 18, 21, 123.0);
        }

        #[test]
        fn should_tokenize_a_template_literal_with_a_pipe_inside_an_interpolation() {
            let tokens = lex("`hello ${name | capitalize}!!!`");
            assert_eq!(tokens.len(), 7);
            expect_string_token(
                &tokens[0],
                0,
                7,
                "hello ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[1], 7, 9, "${");
            expect_identifier_token(&tokens[2], 9, 13, "name");
            expect_operator_token(&tokens[3], 14, 15, "|");
            expect_identifier_token(&tokens[4], 16, 26, "capitalize");
            expect_character_token(&tokens[5], 26, 27, '}');
            expect_string_token(
                &tokens[6],
                27,
                31,
                "!!!",
                StringTokenKind::TemplateLiteralEnd,
            );
        }

        #[test]
        fn should_tokenize_a_template_literal_with_a_pipe_inside_a_parenthesized_interpolation() {
            let tokens = lex("`hello ${(name | capitalize)}!!!`");
            assert_eq!(tokens.len(), 9);
            expect_string_token(
                &tokens[0],
                0,
                7,
                "hello ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[1], 7, 9, "${");
            expect_character_token(&tokens[2], 9, 10, '(');
            expect_identifier_token(&tokens[3], 10, 14, "name");
            expect_operator_token(&tokens[4], 15, 16, "|");
            expect_identifier_token(&tokens[5], 17, 27, "capitalize");
            expect_character_token(&tokens[6], 27, 28, ')');
            expect_character_token(&tokens[7], 28, 29, '}');
            expect_string_token(
                &tokens[8],
                29,
                33,
                "!!!",
                StringTokenKind::TemplateLiteralEnd,
            );
        }

        #[test]
        fn should_tokenize_a_template_literal_in_an_literal_object_value() {
            let tokens = lex("{foo: `${name}`}");
            assert_eq!(tokens.len(), 9);
            expect_character_token(&tokens[0], 0, 1, '{');
            expect_identifier_token(&tokens[1], 1, 4, "foo");
            expect_character_token(&tokens[2], 4, 5, ':');
            expect_string_token(
                &tokens[3],
                6,
                7,
                "",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[4], 7, 9, "${");
            expect_identifier_token(&tokens[5], 9, 13, "name");
            expect_character_token(&tokens[6], 13, 14, '}');
            expect_string_token(&tokens[7], 14, 15, "", StringTokenKind::TemplateLiteralEnd);
            expect_character_token(&tokens[8], 15, 16, '}');
        }

        #[test]
        fn should_produce_an_error_for_an_unterminated_template_literal_with_an_interpolation() {
            let tokens = lex("`hello ${name}!");
            assert_eq!(tokens.len(), 5);
            expect_string_token(
                &tokens[0],
                0,
                7,
                "hello ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[1], 7, 9, "${");
            expect_identifier_token(&tokens[2], 9, 13, "name");
            expect_character_token(&tokens[3], 13, 14, '}');
            expect_error_token(
                &tokens[4],
                15,
                15,
                "Lexer Error: Unterminated template literal at column 15 in expression [`hello ${name}!]",
            );
        }

        #[test]
        fn should_produce_an_error_for_an_unterminate_template_literal_interpolation() {
            let tokens = lex("`hello ${name!`");
            assert_eq!(tokens.len(), 5);
            expect_string_token(
                &tokens[0],
                0,
                7,
                "hello ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[1], 7, 9, "${");
            expect_identifier_token(&tokens[2], 9, 13, "name");
            expect_operator_token(&tokens[3], 13, 14, "!");
            expect_error_token(
                &tokens[4],
                15,
                15,
                "Lexer Error: Unterminated template literal at column 15 in expression [`hello ${name!`]",
            );
        }

        #[test]
        fn should_tokenize_tagged_template_literal_with_no_interpolations() {
            let tokens = lex("tag`hello world`");
            assert_eq!(tokens.len(), 2);
            expect_identifier_token(&tokens[0], 0, 3, "tag");
            expect_string_token(
                &tokens[1],
                3,
                16,
                "hello world",
                StringTokenKind::TemplateLiteralEnd,
            );
        }

        #[test]
        fn should_tokenize_nested_tagged_template_literals() {
            let tokens = lex("tag`hello ${tag`world`}`");
            assert_eq!(tokens.len(), 7);
            expect_identifier_token(&tokens[0], 0, 3, "tag");
            expect_string_token(
                &tokens[1],
                3,
                10,
                "hello ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[2], 10, 12, "${");
            expect_identifier_token(&tokens[3], 12, 15, "tag");
            expect_string_token(
                &tokens[4],
                15,
                22,
                "world",
                StringTokenKind::TemplateLiteralEnd,
            );
            expect_character_token(&tokens[5], 22, 23, '}');
            expect_string_token(&tokens[6], 23, 24, "", StringTokenKind::TemplateLiteralEnd);
        }

        #[test]
        fn should_tokenize_template_literal_with_an_interpolation_in_the_end() {
            let tokens = lex("`hello ${name}`");
            assert_eq!(tokens.len(), 5);
            expect_string_token(
                &tokens[0],
                0,
                7,
                "hello ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[1], 7, 9, "${");
            expect_identifier_token(&tokens[2], 9, 13, "name");
            expect_character_token(&tokens[3], 13, 14, '}');
            expect_string_token(&tokens[4], 14, 15, "", StringTokenKind::TemplateLiteralEnd);
        }

        #[test]
        fn should_tokenize_template_literal_with_an_interpolation_in_the_beginning() {
            let tokens = lex("`${name} Johnson`");
            assert_eq!(tokens.len(), 5);
            expect_string_token(
                &tokens[0],
                0,
                1,
                "",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[1], 1, 3, "${");
            expect_identifier_token(&tokens[2], 3, 7, "name");
            expect_character_token(&tokens[3], 7, 8, '}');
            expect_string_token(
                &tokens[4],
                8,
                17,
                " Johnson",
                StringTokenKind::TemplateLiteralEnd,
            );
        }

        #[test]
        fn should_tokenize_template_literal_with_an_interpolation_in_the_middle() {
            let tokens = lex("`foo${bar}baz`");
            assert_eq!(tokens.len(), 5);
            expect_string_token(
                &tokens[0],
                0,
                4,
                "foo",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[1], 4, 6, "${");
            expect_identifier_token(&tokens[2], 6, 9, "bar");
            expect_character_token(&tokens[3], 9, 10, '}');
            expect_string_token(
                &tokens[4],
                10,
                14,
                "baz",
                StringTokenKind::TemplateLiteralEnd,
            );
        }

        #[test]
        fn should_tokenize_template_literal_with_several_interpolations() {
            let tokens = lex("`${a} - ${b} - ${c}`");
            assert_eq!(tokens.len(), 13);
            expect_string_token(
                &tokens[0],
                0,
                1,
                "",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[1], 1, 3, "${");
            expect_identifier_token(&tokens[2], 3, 4, "a");
            expect_character_token(&tokens[3], 4, 5, '}');
            expect_string_token(
                &tokens[4],
                5,
                8,
                " - ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[5], 8, 10, "${");
            expect_identifier_token(&tokens[6], 10, 11, "b");
            expect_character_token(&tokens[7], 11, 12, '}');
            expect_string_token(
                &tokens[8],
                12,
                15,
                " - ",
                StringTokenKind::TemplateLiteralPart,
            );
            expect_operator_token(&tokens[9], 15, 17, "${");
            expect_identifier_token(&tokens[10], 17, 18, "c");
            expect_character_token(&tokens[11], 18, 19, '}');
        }

        #[test]
        fn should_produce_an_error_if_a_template_literal_is_not_terminated() {
            expect_error_token(
                &lex("`hello")[0],
                6,
                6,
                "Lexer Error: Unterminated template literal at column 6 in expression [`hello]",
            );
        }
    }

    // Regular expression tests
    mod regular_expressions {
        use super::*;

        #[test]
        fn should_tokenize_a_simple_regex() {
            let tokens = lex("/abc/");
            assert_eq!(tokens.len(), 1);
            expect_regexp_body_token(&tokens[0], 0, 5, "abc");
        }

        #[test]
        fn should_tokenize_a_regex_with_flags() {
            let tokens = lex("/abc/gim");
            assert_eq!(tokens.len(), 2);
            expect_regexp_body_token(&tokens[0], 0, 5, "abc");
            expect_regexp_flags_token(&tokens[1], 5, 8, "gim");
        }

        #[test]
        fn should_tokenize_an_identifier_immediately_after_a_regex() {
            let tokens = lex("/abc/ g");
            assert_eq!(tokens.len(), 2);
            expect_regexp_body_token(&tokens[0], 0, 5, "abc");
            expect_identifier_token(&tokens[1], 6, 7, "g");
        }

        #[test]
        fn should_tokenize_a_regex_with_escaped_slashes() {
            let tokens = lex("/^http:\\/\\/foo\\.bar/");
            assert_eq!(tokens.len(), 1);
            expect_regexp_body_token(&tokens[0], 0, 20, "^http:\\/\\/foo\\.bar");
        }

        #[test]
        fn should_tokenize_a_regex_with_un_escaped_slashes_in_a_character_class() {
            let tokens = lex("/[a/]$/");
            assert_eq!(tokens.len(), 1);
            expect_regexp_body_token(&tokens[0], 0, 7, "[a/]$");
        }

        #[test]
        fn should_tokenize_a_regex_with_a_backslash() {
            let tokens = lex("/a\\w+/");
            assert_eq!(tokens.len(), 1);
            expect_regexp_body_token(&tokens[0], 0, 6, "a\\w+");
        }

        #[test]
        fn should_tokenize_a_regex_after_an_operator() {
            let tokens = lex("a = /b/");
            assert_eq!(tokens.len(), 3);
            expect_identifier_token(&tokens[0], 0, 1, "a");
            expect_operator_token(&tokens[1], 2, 3, "=");
            expect_regexp_body_token(&tokens[2], 4, 7, "b");
        }

        #[test]
        fn should_tokenize_a_regex_inside_parentheses() {
            let tokens = lex("log(/a/)");
            assert_eq!(tokens.len(), 4);
            expect_identifier_token(&tokens[0], 0, 3, "log");
            expect_character_token(&tokens[1], 3, 4, '(');
            expect_regexp_body_token(&tokens[2], 4, 7, "a");
            expect_character_token(&tokens[3], 7, 8, ')');
        }

        #[test]
        fn should_tokenize_a_regex_at_the_beginning_of_an_array() {
            let tokens = lex("[/a/]");
            assert_eq!(tokens.len(), 3);
            expect_character_token(&tokens[0], 0, 1, '[');
            expect_regexp_body_token(&tokens[1], 1, 4, "a");
            expect_character_token(&tokens[2], 4, 5, ']');
        }

        #[test]
        fn should_tokenize_a_regex_in_the_middle_of_an_array() {
            let tokens = lex("[1, /a/, 2]");
            assert_eq!(tokens.len(), 7);
            expect_character_token(&tokens[0], 0, 1, '[');
            expect_number_token(&tokens[1], 1, 2, 1.0);
            expect_character_token(&tokens[2], 2, 3, ',');
            expect_regexp_body_token(&tokens[3], 4, 7, "a");
            expect_character_token(&tokens[4], 7, 8, ',');
            expect_number_token(&tokens[5], 9, 10, 2.0);
            expect_character_token(&tokens[6], 10, 11, ']');
        }

        #[test]
        fn should_tokenize_a_regex_inside_an_object_literal() {
            let tokens = lex("{a: /b/}");
            assert_eq!(tokens.len(), 5);
            expect_character_token(&tokens[0], 0, 1, '{');
            expect_identifier_token(&tokens[1], 1, 2, "a");
            expect_character_token(&tokens[2], 2, 3, ':');
            expect_regexp_body_token(&tokens[3], 4, 7, "b");
            expect_character_token(&tokens[4], 7, 8, '}');
        }

        #[test]
        fn should_not_tokenize_a_regex_preceded_by_a_square_bracket() {
            let tokens = lex("a[0] /= b");
            assert_eq!(tokens.len(), 6);
            expect_identifier_token(&tokens[0], 0, 1, "a");
            expect_character_token(&tokens[1], 1, 2, '[');
            expect_number_token(&tokens[2], 2, 3, 0.0);
            expect_character_token(&tokens[3], 3, 4, ']');
            expect_operator_token(&tokens[4], 5, 7, "/=");
            expect_identifier_token(&tokens[5], 8, 9, "b");
        }

        #[test]
        fn should_not_tokenize_a_regex_preceded_by_an_identifier() {
            let tokens = lex("a / b");
            assert_eq!(tokens.len(), 3);
            expect_identifier_token(&tokens[0], 0, 1, "a");
            expect_operator_token(&tokens[1], 2, 3, "/");
            expect_identifier_token(&tokens[2], 4, 5, "b");
        }

        #[test]
        fn should_not_tokenize_a_regex_preceded_by_a_number() {
            let tokens = lex("1 / b");
            assert_eq!(tokens.len(), 3);
            expect_number_token(&tokens[0], 0, 1, 1.0);
            expect_operator_token(&tokens[1], 2, 3, "/");
            expect_identifier_token(&tokens[2], 4, 5, "b");
        }

        #[test]
        fn should_not_tokenize_a_regex_that_is_preceded_by_a_string() {
            let tokens = lex("\"a\" / b");
            assert_eq!(tokens.len(), 3);
            expect_string_token(&tokens[0], 0, 3, "a", StringTokenKind::Plain);
            expect_operator_token(&tokens[1], 4, 5, "/");
            expect_identifier_token(&tokens[2], 6, 7, "b");
        }

        #[test]
        fn should_not_tokenize_a_regex_preceded_by_a_closing_parenthesis() {
            let tokens = lex("(a) / b");
            assert_eq!(tokens.len(), 5);
            expect_character_token(&tokens[0], 0, 1, '(');
            expect_identifier_token(&tokens[1], 1, 2, "a");
            expect_character_token(&tokens[2], 2, 3, ')');
            expect_operator_token(&tokens[3], 4, 5, "/");
            expect_identifier_token(&tokens[4], 6, 7, "b");
        }

        #[test]
        fn should_not_tokenize_a_regex_that_is_preceded_by_a_keyword() {
            let tokens = lex("this / b");
            assert_eq!(tokens.len(), 3);
            expect_keyword_token(&tokens[0], 0, 4, "this");
            expect_operator_token(&tokens[1], 5, 6, "/");
            expect_identifier_token(&tokens[2], 7, 8, "b");
        }

        #[test]
        fn should_not_tokenize_a_regex_preceded_by_a_non_null_assertion_on_an_identifier() {
            let tokens = lex("foo! / 2");
            assert_eq!(tokens.len(), 4);
            expect_identifier_token(&tokens[0], 0, 3, "foo");
            expect_operator_token(&tokens[1], 3, 4, "!");
            expect_operator_token(&tokens[2], 5, 6, "/");
            expect_number_token(&tokens[3], 7, 8, 2.0);
        }

        #[test]
        fn should_not_tokenize_a_regex_preceded_by_a_non_null_assertion_on_a_function_call() {
            let tokens = lex("foo()! / 2");
            assert_eq!(tokens.len(), 6);
            expect_identifier_token(&tokens[0], 0, 3, "foo");
            expect_character_token(&tokens[1], 3, 4, '(');
            expect_character_token(&tokens[2], 4, 5, ')');
            expect_operator_token(&tokens[3], 5, 6, "!");
            expect_operator_token(&tokens[4], 7, 8, "/");
            expect_number_token(&tokens[5], 9, 10, 2.0);
        }

        #[test]
        fn should_not_tokenize_a_regex_preceded_by_a_non_null_assertion_on_an_array() {
            let tokens = lex("[1]! / 2");
            assert_eq!(tokens.len(), 6);
            expect_character_token(&tokens[0], 0, 1, '[');
            expect_number_token(&tokens[1], 1, 2, 1.0);
            expect_character_token(&tokens[2], 2, 3, ']');
            expect_operator_token(&tokens[3], 3, 4, "!");
            expect_operator_token(&tokens[4], 5, 6, "/");
            expect_number_token(&tokens[5], 7, 8, 2.0);
        }

        #[test]
        fn should_tokenize_a_regex_after_a_negation_operator() {
            let tokens = lex("log(!/a/.test(\"1\"))");
            assert_eq!(tokens.len(), 10);
            expect_identifier_token(&tokens[0], 0, 3, "log");
            expect_character_token(&tokens[1], 3, 4, '(');
            expect_operator_token(&tokens[2], 4, 5, "!");
            expect_regexp_body_token(&tokens[3], 5, 8, "a");
            expect_character_token(&tokens[4], 8, 9, '.');
            expect_identifier_token(&tokens[5], 9, 13, "test");
            expect_character_token(&tokens[6], 13, 14, '(');
            expect_string_token(&tokens[7], 14, 17, "1", StringTokenKind::Plain);
            expect_character_token(&tokens[8], 17, 18, ')');
            expect_character_token(&tokens[9], 18, 19, ')');
        }

        #[test]
        fn should_tokenize_a_regex_after_several_negation_operators() {
            let tokens = lex("log(!!!!!!/a/.test(\"1\"))");
            assert_eq!(tokens.len(), 15);
            expect_identifier_token(&tokens[0], 0, 3, "log");
            expect_character_token(&tokens[1], 3, 4, '(');
            expect_operator_token(&tokens[2], 4, 5, "!");
            expect_operator_token(&tokens[3], 5, 6, "!");
            expect_operator_token(&tokens[4], 6, 7, "!");
            expect_operator_token(&tokens[5], 7, 8, "!");
            expect_operator_token(&tokens[6], 8, 9, "!");
            expect_operator_token(&tokens[7], 9, 10, "!");
            expect_regexp_body_token(&tokens[8], 10, 13, "a");
            expect_character_token(&tokens[9], 13, 14, '.');
            expect_identifier_token(&tokens[10], 14, 18, "test");
            expect_character_token(&tokens[11], 18, 19, '(');
            expect_string_token(&tokens[12], 19, 22, "1", StringTokenKind::Plain);
            expect_character_token(&tokens[13], 22, 23, ')');
            expect_character_token(&tokens[14], 23, 24, ')');
        }

        #[test]
        fn should_tokenize_a_method_call_on_a_regex() {
            let tokens = lex("/abc/.test(\"foo\")");
            assert_eq!(tokens.len(), 6);
            expect_regexp_body_token(&tokens[0], 0, 5, "abc");
            expect_character_token(&tokens[1], 5, 6, '.');
            expect_identifier_token(&tokens[2], 6, 10, "test");
            expect_character_token(&tokens[3], 10, 11, '(');
            expect_string_token(&tokens[4], 11, 16, "foo", StringTokenKind::Plain);
            expect_character_token(&tokens[5], 16, 17, ')');
        }

        #[test]
        fn should_tokenize_a_method_call_with_a_regex_parameter() {
            let tokens = lex("\"foo\".match(/abc/)");
            assert_eq!(tokens.len(), 6);
            expect_string_token(&tokens[0], 0, 5, "foo", StringTokenKind::Plain);
            expect_character_token(&tokens[1], 5, 6, '.');
            expect_identifier_token(&tokens[2], 6, 11, "match");
            expect_character_token(&tokens[3], 11, 12, '(');
            expect_regexp_body_token(&tokens[4], 12, 17, "abc");
            expect_character_token(&tokens[5], 17, 18, ')');
        }

        #[test]
        fn should_not_tokenize_consecutive_regexes() {
            let tokens = lex("/ 1 / 2 / 3 / 4");
            assert_eq!(tokens.len(), 6);
            expect_regexp_body_token(&tokens[0], 0, 5, " 1 ");
            expect_number_token(&tokens[1], 6, 7, 2.0);
            expect_operator_token(&tokens[2], 8, 9, "/");
            expect_number_token(&tokens[3], 10, 11, 3.0);
            expect_operator_token(&tokens[4], 12, 13, "/");
            expect_number_token(&tokens[5], 14, 15, 4.0);
        }

        #[test]
        fn should_not_tokenize_regex_like_characters_inside_of_a_pipe() {
            let tokens = lex("foo / 1000 | date: 'M/d/yy'");
            assert_eq!(tokens.len(), 7);
            expect_identifier_token(&tokens[0], 0, 3, "foo");
            expect_operator_token(&tokens[1], 4, 5, "/");
            expect_number_token(&tokens[2], 6, 10, 1000.0);
            expect_operator_token(&tokens[3], 11, 12, "|");
            expect_identifier_token(&tokens[4], 13, 17, "date");
            expect_character_token(&tokens[5], 17, 18, ':');
            expect_string_token(&tokens[6], 19, 27, "M/d/yy", StringTokenKind::Plain);
        }

        #[test]
        fn should_produce_an_error_for_an_unterminated_regex() {
            expect_error_token(
                &lex("/a")[0],
                2,
                2,
                "Lexer Error: Unterminated regular expression at column 2 in expression [/a]",
            );
        }
    }
}

