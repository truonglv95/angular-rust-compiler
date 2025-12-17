//! Style Parser Tests
//!
//! Mirrors angular/packages/compiler/test/render3/style_parser_spec.ts

use angular_compiler::template::pipeline::src::phases::parse_extracted_styles::{parse_style, hyphenate};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_empty_or_blank_strings() {
        let result1 = parse_style("");
        assert_eq!(result1, Vec::<String>::new());

        let result2 = parse_style("    ");
        assert_eq!(result2, Vec::<String>::new());
    }

    #[test]
    fn should_parse_a_string_into_a_key_value_map() {
        let result = parse_style("width:100px;height:200px;opacity:0");
        assert_eq!(result, vec!["width", "100px", "height", "200px", "opacity", "0"]);
    }

    #[test]
    fn should_allow_empty_values() {
        let result = parse_style("width:;height:   ;");
        assert_eq!(result, vec!["width", "", "height", ""]);
    }

    #[test]
    fn should_trim_values_and_properties() {
        let result = parse_style("width :333px ; height:666px    ; opacity: 0.5;");
        assert_eq!(result, vec!["width", "333px", "height", "666px", "opacity", "0.5"]);
    }

    #[test]
    fn should_not_mess_up_with_quoted_strings_that_contain_special_chars() {
        let result = parse_style("content: \"foo; man: guy\"; width: 100px");
        assert_eq!(result, vec!["content", "\"foo; man: guy\"", "width", "100px"]);
    }

    #[test]
    fn should_not_mess_up_with_quoted_strings_that_contain_inner_quote_values() {
        let quote_str = "\"one 'two' three \"four\" five\"";
        let result = parse_style(&format!("content: {}; width: 123px", quote_str));
        assert_eq!(result, vec!["content", quote_str, "width", "123px"]);
    }

    #[test]
    fn should_respect_parenthesis_that_are_placed_within_a_style() {
        let result = parse_style("background-image: url(\"foo.jpg\")");
        assert_eq!(result, vec!["background-image", "url(\"foo.jpg\")"]);
    }

    #[test]
    fn should_respect_multi_level_parenthesis_that_contain_special_chars() {
        let result = parse_style("color: rgba(calc(50 * 4), var(--cool), :5;); height: 100px;");
        assert_eq!(result, vec!["color", "rgba(calc(50 * 4), var(--cool), :5;)", "height", "100px"]);
    }

    #[test]
    fn should_hyphenate_style_properties_from_camel_case() {
        let result = parse_style("borderWidth: 200px");
        assert_eq!(result, vec!["border-width", "200px"]);
    }

    mod should_not_remove_quotes {
        use super::*;

        #[test]
        fn from_string_data_types() {
            let result = parse_style("content: \"foo\"");
            assert_eq!(result, vec!["content", "\"foo\""]);
        }

        #[test]
        fn that_changes_the_value_context_from_invalid_to_valid() {
            let result = parse_style("width: \"1px\"");
            assert_eq!(result, vec!["width", "\"1px\""]);
        }
    }

    mod camel_casing_to_hyphenation {
        use super::*;

        #[test]
        fn should_convert_a_camel_cased_value_to_a_hyphenated_value() {
            assert_eq!(hyphenate("fooBar"), "foo-bar");
            assert_eq!(hyphenate("fooBarMan"), "foo-bar-man");
            assert_eq!(hyphenate("-fooBar-man"), "-foo-bar-man");
        }

        #[test]
        fn should_make_everything_lowercase() {
            assert_eq!(hyphenate("-WebkitAnimation"), "-webkit-animation");
        }
    }
}

