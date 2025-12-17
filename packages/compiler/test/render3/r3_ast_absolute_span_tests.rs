//! R3 AST Absolute Source Span Tests
//!
//! Mirrors angular/packages/compiler/test/render3/r3_ast_absolute_span_spec.ts

// Include test utilities
#[path = "util/expression.rs"]
mod expression_util;
use expression_util::humanize_expression_source;

#[path = "view/util.rs"]
mod view_util;
use view_util::{parse_r3, ParseR3Options};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_handle_comment_in_interpolation() {
        let result = parse_r3("{{foo // comment}}", ParseR3Options {
            preserve_whitespaces: Some(true),
            ..Default::default()
        });
        let spans = humanize_expression_source(&result.nodes);
        assert!(spans.iter().any(|(s, span)| s == "foo" && span.start == 2 && span.end == 5));
    }

    #[test]
    fn should_handle_whitespace_in_interpolation() {
        let result = parse_r3("{{  foo  }}", ParseR3Options {
            preserve_whitespaces: Some(true),
            ..Default::default()
        });
        let spans = humanize_expression_source(&result.nodes);
        assert!(spans.iter().any(|(s, span)| s == "foo" && span.start == 4 && span.end == 7));
    }

    #[test]
    fn should_handle_whitespace_and_comment_in_interpolation() {
        let result = parse_r3("{{  foo // comment  }}", ParseR3Options {
            preserve_whitespaces: Some(true),
            ..Default::default()
        });
        let spans = humanize_expression_source(&result.nodes);
        assert!(spans.iter().any(|(s, span)| s == "foo" && span.start == 4 && span.end == 7));
    }

    #[test]
    fn should_handle_comment_in_an_action_binding() {
        let result = parse_r3("<button (click)=\"foo = true // comment\">Save</button>", ParseR3Options {
            preserve_whitespaces: Some(true),
            ..Default::default()
        });
        let spans = humanize_expression_source(&result.nodes);
        assert!(spans.iter().any(|(s, span)| s == "foo = true" && span.start == 17 && span.end == 27));
    }

    #[test]
    fn should_provide_absolute_offsets_with_arbitrary_whitespace() {
        let result = parse_r3("<div>\n  \n{{foo}}</div>", ParseR3Options {
            preserve_whitespaces: Some(true),
            ..Default::default()
        });
        let spans = humanize_expression_source(&result.nodes);
        assert!(spans.iter().any(|(s, span)| s.contains("foo") && span.start == 5 && span.end == 16));
    }

    #[test]
    fn should_provide_absolute_offsets_of_an_expression_in_a_bound_text() {
        let result = parse_r3("<div>{{foo}}</div>", Default::default());
        let spans = humanize_expression_source(&result.nodes);
        assert!(spans.iter().any(|(s, span)| s.contains("foo") && span.start == 5 && span.end == 12));
    }

    #[test]
    fn should_provide_absolute_offsets_of_an_expression_in_a_bound_event() {
        let result = parse_r3("<div (click)=\"foo();bar();\"></div>", Default::default());
        let spans = humanize_expression_source(&result.nodes);
        assert!(spans.iter().any(|(s, span)| s.contains("foo(); bar();") && span.start == 14 && span.end == 26));
        
        let result2 = parse_r3("<div on-click=\"foo();bar();\"></div>", Default::default());
        let spans2 = humanize_expression_source(&result2.nodes);
        assert!(spans2.iter().any(|(s, span)| s.contains("foo(); bar();") && span.start == 15 && span.end == 27));
    }

    #[test]
    fn should_provide_absolute_offsets_of_an_expression_in_a_bound_attribute() {
        let result = parse_r3("<input [disabled]=\"condition ? true : false\" />", Default::default());
        let spans = humanize_expression_source(&result.nodes);
        assert!(spans.iter().any(|(s, span)| s == "condition ? true : false" && span.start == 19 && span.end == 43));
        
        let result2 = parse_r3("<input bind-disabled=\"condition ? true : false\" />", Default::default());
        let spans2 = humanize_expression_source(&result2.nodes);
        assert!(spans2.iter().any(|(s, span)| s == "condition ? true : false" && span.start == 22 && span.end == 46));
    }

    #[test]
    fn should_provide_absolute_offsets_of_an_expression_in_a_template_attribute() {
        let result = parse_r3("<div *ngIf=\"value | async\"></div>", Default::default());
        let spans = humanize_expression_source(&result.nodes);
        assert!(spans.iter().any(|(s, span)| s.contains("value | async") && span.start == 12 && span.end == 25));
    }

    mod binary_expression {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_binary_expression() {
            let result = parse_r3("<div>{{1 + 2}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "1 + 2" && span.start == 7 && span.end == 12));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_binary_expression() {
            let result = parse_r3("<div>{{1 + 2}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "1" && span.start == 7 && span.end == 8));
            assert!(spans.iter().any(|(s, span)| s == "2" && span.start == 11 && span.end == 12));
        }
    }

    mod conditional {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_conditional() {
            let result = parse_r3("<div>{{bool ? 1 : 0}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "bool ? 1 : 0" && span.start == 7 && span.end == 19));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_conditional() {
            let result = parse_r3("<div>{{bool ? 1 : 0}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "bool" && span.start == 7 && span.end == 11));
            assert!(spans.iter().any(|(s, span)| s == "1" && span.start == 14 && span.end == 15));
            assert!(spans.iter().any(|(s, span)| s == "0" && span.start == 18 && span.end == 19));
        }
    }

    mod chain {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_chain() {
            let result = parse_r3("<div (click)=\"a(); b();\"><div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s.contains("a(); b();") && span.start == 14 && span.end == 23));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_chain() {
            let result = parse_r3("<div (click)=\"a(); b();\"><div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "a()" && span.start == 14 && span.end == 17));
            assert!(spans.iter().any(|(s, span)| s == "b()" && span.start == 19 && span.end == 22));
        }
    }

    mod function_call {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_function_call() {
            let result = parse_r3("<div>{{fn()()}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "fn()()" && span.start == 7 && span.end == 13));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_function_call() {
            let result = parse_r3("<div>{{fn()(param)}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "param" && span.start == 12 && span.end == 17));
        }
    }

    #[test]
    fn should_provide_absolute_offsets_of_an_implicit_receiver() {
        let result = parse_r3("<div>{{a.b}}<div>", Default::default());
        let spans = humanize_expression_source(&result.nodes);
        assert!(spans.iter().any(|(s, span)| s == "" && span.start == 7 && span.end == 7));
    }

    mod interpolation {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_an_interpolation() {
            let result = parse_r3("<div>{{1 + foo.length}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s.contains("1 + foo.length") && span.start == 5 && span.end == 23));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_an_interpolation() {
            let result = parse_r3("<div>{{1 + 2}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "1" && span.start == 7 && span.end == 8));
            assert!(spans.iter().any(|(s, span)| s == "2" && span.start == 11 && span.end == 12));
        }

        #[test]
        fn should_handle_html_entity_before_interpolation() {
            let result = parse_r3("&nbsp;{{abc}}", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "abc" && span.start == 8 && span.end == 11));
        }

        #[test]
        fn should_handle_many_html_entities_and_many_interpolations() {
            let result = parse_r3("&quot;{{abc}}&quot;{{def}}&nbsp;{{ghi}}", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "abc" && span.start == 8 && span.end == 11));
            assert!(spans.iter().any(|(s, span)| s == "def" && span.start == 21 && span.end == 24));
            assert!(spans.iter().any(|(s, span)| s == "ghi" && span.start == 34 && span.end == 37));
        }

        #[test]
        fn should_handle_interpolation_in_attribute() {
            let result = parse_r3("<div class=\"{{abc}}\"><div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "abc" && span.start == 14 && span.end == 17));
        }

        #[test]
        fn should_handle_interpolation_preceded_by_html_entity_in_attribute() {
            let result = parse_r3("<div class=\"&nbsp;{{abc}}\"><div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "abc" && span.start == 20 && span.end == 23));
        }

        #[test]
        fn should_handle_many_interpolation_with_html_entities_in_attribute() {
            let result = parse_r3("<div class=\"&quot;{{abc}}&quot;&nbsp;{{def}}\"><div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "abc" && span.start == 20 && span.end == 23));
            assert!(spans.iter().any(|(s, span)| s == "def" && span.start == 39 && span.end == 42));
        }
    }

    mod keyed_read {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_keyed_read() {
            let result = parse_r3("<div>{{obj[key]}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "obj[key]" && span.start == 7 && span.end == 15));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_keyed_read() {
            let result = parse_r3("<div>{{obj[key]}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "key" && span.start == 11 && span.end == 14));
        }
    }

    mod keyed_write {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_keyed_write() {
            let result = parse_r3("<div>{{obj[key] = 0}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "obj[key] = 0" && span.start == 7 && span.end == 19));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_keyed_write() {
            let result = parse_r3("<div>{{obj[key] = 0}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "key" && span.start == 11 && span.end == 14));
            assert!(spans.iter().any(|(s, span)| s == "0" && span.start == 18 && span.end == 19));
        }
    }

    #[test]
    fn should_provide_absolute_offsets_of_a_literal_primitive() {
        let result = parse_r3("<div>{{100}}<div>", Default::default());
        let spans = humanize_expression_source(&result.nodes);
        assert!(spans.iter().any(|(s, span)| s == "100" && span.start == 7 && span.end == 10));
    }

    mod literal_array {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_literal_array() {
            let result = parse_r3("<div>{{[0, 1, 2]}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "[0, 1, 2]" && span.start == 7 && span.end == 16));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_literal_array() {
            let result = parse_r3("<div>{{[0, 1, 2]}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "0" && span.start == 8 && span.end == 9));
            assert!(spans.iter().any(|(s, span)| s == "1" && span.start == 11 && span.end == 12));
            assert!(spans.iter().any(|(s, span)| s == "2" && span.start == 14 && span.end == 15));
        }
    }

    mod literal_map {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_literal_map() {
            let result = parse_r3("<div>{{ {a: 0} }}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "{a: 0}" && span.start == 8 && span.end == 14));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_literal_map() {
            let result = parse_r3("<div>{{ {a: 0} }}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "0" && span.start == 12 && span.end == 13));
        }
    }

    mod method_call {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_method_call() {
            let result = parse_r3("<div>{{method()}}</div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "method()" && span.start == 7 && span.end == 15));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_method_call() {
            let result = parse_r3("<div>{{method(param)}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "param" && span.start == 14 && span.end == 19));
        }
    }

    mod non_null_assert {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_non_null_assert() {
            let result = parse_r3("<div>{{prop!}}</div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "prop!" && span.start == 7 && span.end == 12));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_non_null_assert() {
            let result = parse_r3("<div>{{prop!}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "prop" && span.start == 7 && span.end == 11));
        }
    }

    mod pipe {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_pipe() {
            let result = parse_r3("<div>{{prop | pipe}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s.contains("prop | pipe") && span.start == 7 && span.end == 18));
        }

        #[test]
        fn should_provide_absolute_offsets_expressions_in_a_pipe() {
            let result = parse_r3("<div>{{prop | pipe}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "prop" && span.start == 7 && span.end == 11));
        }
    }

    mod property_read {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_property_read() {
            let result = parse_r3("<div>{{prop.obj}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "prop.obj" && span.start == 7 && span.end == 15));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_property_read() {
            let result = parse_r3("<div>{{prop.obj}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "prop" && span.start == 7 && span.end == 11));
        }
    }

    mod property_write {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_property_write() {
            let result = parse_r3("<div (click)=\"prop = 0\"></div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "prop = 0" && span.start == 14 && span.end == 22));
        }

        #[test]
        fn should_provide_absolute_offsets_of_an_accessed_property_write() {
            let result = parse_r3("<div (click)=\"prop.inner = 0\"></div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "prop.inner = 0" && span.start == 14 && span.end == 28));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_property_write() {
            let result = parse_r3("<div (click)=\"prop = 0\"></div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "0" && span.start == 21 && span.end == 22));
        }
    }

    mod not_prefix {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_not_prefix() {
            let result = parse_r3("<div>{{!prop}}</div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "!prop" && span.start == 7 && span.end == 12));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_a_not_prefix() {
            let result = parse_r3("<div>{{!prop}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "prop" && span.start == 8 && span.end == 12));
        }
    }

    mod safe_method_call {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_safe_method_call() {
            let result = parse_r3("<div>{{prop?.safe()}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "prop?.safe()" && span.start == 7 && span.end == 19));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_safe_method_call() {
            let result = parse_r3("<div>{{prop?.safe()}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "prop" && span.start == 7 && span.end == 11));
        }
    }

    mod safe_property_read {
        use super::*;

        #[test]
        fn should_provide_absolute_offsets_of_a_safe_property_read() {
            let result = parse_r3("<div>{{prop?.safe}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "prop?.safe" && span.start == 7 && span.end == 17));
        }

        #[test]
        fn should_provide_absolute_offsets_of_expressions_in_safe_property_read() {
            let result = parse_r3("<div>{{prop?.safe}}<div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "prop" && span.start == 7 && span.end == 11));
        }
    }

    mod absolute_offsets_for_template_expressions {
        use super::*;

        #[test]
        fn should_work_for_simple_cases() {
            let result = parse_r3("<div *ngFor=\"let item of items\">{{item}}</div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "items" && span.start == 25 && span.end == 30));
        }

        #[test]
        fn should_work_with_multiple_bindings() {
            let result = parse_r3("<div *ngFor=\"let a of As; let b of Bs\"></div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "As" && span.start == 22 && span.end == 24));
            assert!(spans.iter().any(|(s, span)| s == "Bs" && span.start == 35 && span.end == 37));
        }
    }

    mod icu_expressions {
        use super::*;

        #[test]
        fn is_correct_for_variables_and_placeholders() {
            let result = parse_r3("<span i18n>{item.var, plural, other { {{item.placeholder}} items } }</span>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "item.var" && span.start == 12 && span.end == 20));
            assert!(spans.iter().any(|(s, span)| s == "item.placeholder" && span.start == 40 && span.end == 56));
        }

        #[test]
        fn is_correct_for_variables_and_placeholders_nested() {
            let result = parse_r3(
                "<span i18n>{item.var, plural, other { {{item.placeholder}} {nestedVar, plural, other { {{nestedPlaceholder}} }}} }</span>",
                Default::default()
            );
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s == "item.var" && span.start == 12 && span.end == 20));
            assert!(spans.iter().any(|(s, span)| s == "item.placeholder" && span.start == 40 && span.end == 56));
            assert!(spans.iter().any(|(s, span)| s == "nestedVar" && span.start == 60 && span.end == 69));
            assert!(spans.iter().any(|(s, span)| s == "nestedPlaceholder" && span.start == 89 && span.end == 106));
        }
    }

    mod object_literal {
        use super::*;

        #[test]
        fn is_correct_for_object_literals_with_shorthand_property_declarations() {
            let result = parse_r3("<div (click)=\"test({a: 1, b, c: 3, foo})\"></div>", Default::default());
            let spans = humanize_expression_source(&result.nodes);
            assert!(spans.iter().any(|(s, span)| s.contains("a: 1, b: b, c: 3, foo: foo") && span.start == 19 && span.end == 39));
            assert!(spans.iter().any(|(s, span)| s == "b" && span.start == 26 && span.end == 27));
            assert!(spans.iter().any(|(s, span)| s == "foo" && span.start == 35 && span.end == 38));
        }
    }
}

