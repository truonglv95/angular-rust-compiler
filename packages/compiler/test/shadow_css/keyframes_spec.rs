//! Keyframes and Animations Tests
//!
//! Corresponds to packages/compiler/test/shadow_css/keyframes_spec.ts
//! All test cases match exactly with TypeScript version

mod utils;
use utils::{assert_contains, assert_not_contains, shim};
use regex::Regex;

#[test]
fn should_scope_keyframes_rules() {
    let css = "@keyframes foo {0% {transform:translate(-50%) scaleX(0);}}";
    let expected = "@keyframes host-a_foo {0% {transform:translate(-50%) scaleX(0);}}";
    assert_eq!(shim(css, "host-a", ""), expected);
}

#[test]
fn should_scope_webkit_keyframes_rules() {
    let css = "@-webkit-keyframes foo {0% {-webkit-transform:translate(-50%) scaleX(0);}} ";
    let expected = "@-webkit-keyframes host-a_foo {0% {-webkit-transform:translate(-50%) scaleX(0);}}";
    assert_eq!(shim(css, "host-a", ""), expected);
}

#[test]
fn should_scope_animations_using_local_keyframes_identifiers() {
    let css = "
        button {
            animation: foo 10s ease;
        }
        @keyframes foo {
            0% {
            transform: translate(-50%) scaleX(0);
            }
        }
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "animation: host-a_foo 10s ease;");
}

#[test]
fn should_not_scope_animations_using_non_local_keyframes_identifiers() {
    let css = "
        button {
            animation: foo 10s ease;
        }
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "animation: foo 10s ease;");
}

#[test]
fn should_scope_animation_names_using_local_keyframes_identifiers() {
    let css = "
        button {
            animation-name: foo;
        }
        @keyframes foo {
            0% {
            transform: translate(-50%) scaleX(0);
            }
        }
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "animation-name: host-a_foo;");
}

#[test]
fn should_not_scope_animation_names_using_non_local_keyframes_identifiers() {
    let css = "
        button {
            animation-name: foo;
        }
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "animation-name: foo;");
}

#[test]
fn should_handle_scope_or_not_multiple_animation_names() {
    let css = "
        button {
            animation-name: foo, bar,baz, qux , quux ,corge ,grault ,garply, waldo;
        }
        @keyframes foo {}
        @keyframes baz {}
        @keyframes quux {}
        @keyframes grault {}
        @keyframes waldo {}";
    let result = shim(css, "host-a", "");
    let expected = "animation-name: host-a_foo, bar,host-a_baz, qux , host-a_quux ,corge ,host-a_grault ,garply, host-a_waldo;";
    assert_contains(&result, expected);
}

#[test]
fn should_handle_scope_or_not_multiple_animation_names_defined_over_multiple_lines() {
    let css = "
        button {
            animation-name: foo,
                            bar,baz,
                            qux ,
                            quux ,
                            grault,
                            garply, waldo;
        }
        @keyframes foo {}
        @keyframes baz {}
        @keyframes quux {}
        @keyframes grault {}";
    let result = shim(css, "host-a", "");
    for scoped in &["foo", "baz", "quux", "grault"] {
        assert_contains(&result, &format!("host-a_{}", scoped));
    }
    for non_scoped in &["bar", "qux", "garply", "waldo"] {
        assert_contains(&result, non_scoped);
        assert_not_contains(&result, &format!("host-a_{}", non_scoped));
    }
}

#[test]
fn should_handle_scope_or_not_animation_definition_containing_some_names_which_do_not_have_a_preceding_space() {
    let component_variable = "%COMP%";
    let host_attr = format!("_nghost-{}", component_variable);
    let content_attr = format!("_ngcontent-{}", component_variable);
    let css = ".test {
      animation:my-anim 1s,my-anim2 2s, my-anim3 3s,my-anim4 4s;
    }
    
    @keyframes my-anim {
      0% {color: red}
      100% {color: blue}
    }
    
    @keyframes my-anim2 {
      0% {font-size: 1em}
      100% {font-size: 1.2em}
    }
    ";
    let result = shim(&css, &content_attr, &host_attr);
    let animation_line_re = Regex::new(r"animation:[^;]+;").unwrap();
    let animation_line = animation_line_re
        .find(&result)
        .map(|m| m.as_str())
        .unwrap_or("");
    for scoped in &["my-anim", "my-anim2"] {
        assert_contains(animation_line, &format!("_ngcontent-%COMP%_{}", scoped));
    }
    for non_scoped in &["my-anim3", "my-anim4"] {
        assert_contains(animation_line, non_scoped);
        assert_not_contains(animation_line, &format!("_ngcontent-%COMP%_{}", non_scoped));
    }
}

#[test]
fn should_handle_scope_or_not_animation_definitions_preceded_by_an_erroneous_comma() {
    let component_variable = "%COMP%";
    let host_attr = format!("_nghost-{}", component_variable);
    let content_attr = format!("_ngcontent-{}", component_variable);
    let css = ".test {
      animation:, my-anim 1s,my-anim2 2s, my-anim3 3s,my-anim4 4s;
    }
    
    @keyframes my-anim {
      0% {color: red}
      100% {color: blue}
    }
    
    @keyframes my-anim2 {
      0% {font-size: 1em}
      100% {font-size: 1.2em}
    }
    ";
    let result = shim(&css, &content_attr, &host_attr);
    assert_not_contains(&result, "animation:,");
    let animation_line_re = Regex::new(r"animation:[^;]+;").unwrap();
    let animation_line = animation_line_re
        .find(&result)
        .map(|m| m.as_str())
        .unwrap_or("");
    for scoped in &["my-anim", "my-anim2"] {
        assert_contains(animation_line, &format!("_ngcontent-%COMP%_{}", scoped));
    }
    for non_scoped in &["my-anim3", "my-anim4"] {
        assert_contains(animation_line, non_scoped);
        assert_not_contains(animation_line, &format!("_ngcontent-%COMP%_{}", non_scoped));
    }
}

#[test]
fn should_handle_scope_or_not_multiple_animation_definitions_in_a_single_declaration() {
    let css = "
        div {
            animation: 1s ease foo, 2s bar infinite, forwards baz 3s;
        }

        p {
            animation: 1s \"foo\", 2s \"bar\";
        }

        span {
            animation: .5s ease 'quux',
                        1s foo infinite, forwards \"baz'\" 1.5s,
                        2s bar;
        }

        button {
            animation: .5s bar,
                        1s foo 0.3s, 2s quux;
        }

        @keyframes bar {}
        @keyframes quux {}
        @keyframes \"baz'\" {}";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "animation: 1s ease foo, 2s host-a_bar infinite, forwards baz 3s;");
    assert_contains(&result, "animation: 1s \"foo\", 2s \"host-a_bar\";");
    assert_contains(
        &result,
        "
            animation: .5s host-a_bar,
                        1s foo 0.3s, 2s host-a_quux;",
    );
    assert_contains(
        &result,
        "
            animation: .5s ease 'host-a_quux',
                        1s foo infinite, forwards \"host-a_baz'\" 1.5s,
                        2s host-a_bar;",
    );
}

#[test]
fn should_not_modify_css_variables_ending_with_animation_even_if_they_reference_a_local_keyframes_identifier() {
    let css = "
        button {
            --variable-animation: foo;
        }
        @keyframes foo {}";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "--variable-animation: foo;");
}

#[test]
fn should_not_modify_css_variables_ending_with_animation_name_even_if_they_reference_a_local_keyframes_identifier() {
    let css = "
        button {
            --variable-animation-name: foo;
        }
        @keyframes foo {}";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "--variable-animation-name: foo;");
}

#[test]
fn should_maintain_the_spacing_when_handling_scoping_or_not_keyframes_and_animations() {
    let css = "
        div {
            animation-name : foo;
            animation:  5s bar   1s backwards;
            animation : 3s baz ;
            animation-name:foobar ;
            animation:1s \"foo\" ,   2s \"bar\",3s \"quux\";
        }

        @-webkit-keyframes  bar {}
        @keyframes foobar  {}
        @keyframes quux {}
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "animation-name : foo;");
    assert_contains(&result, "animation:  5s host-a_bar   1s backwards;");
    assert_contains(&result, "animation : 3s baz ;");
    assert_contains(&result, "animation-name:host-a_foobar ;");
    assert_contains(&result, "@-webkit-keyframes  host-a_bar {}");
    assert_contains(&result, "@keyframes host-a_foobar  {}");
    assert_contains(&result, "animation:1s \"foo\" ,   2s \"host-a_bar\",3s \"host-a_quux\"");
}

#[test]
fn should_correctly_process_animations_defined_without_any_prefixed_space() {
    let css = ".test{display: flex;animation:foo 1s forwards;} @keyframes foo {}";
    let expected = ".test[host-a]{display: flex;animation:host-a_foo 1s forwards;} @keyframes host-a_foo {}";
    assert_eq!(shim(css, "host-a", ""), expected);
    
    let css = ".test{animation:foo 2s forwards;} @keyframes foo {}";
    let expected = ".test[host-a]{animation:host-a_foo 2s forwards;} @keyframes host-a_foo {}";
    assert_eq!(shim(css, "host-a", ""), expected);
    
    let css = "button {display: block;animation-name: foobar;} @keyframes foobar {}";
    let expected = "button[host-a] {display: block;animation-name: host-a_foobar;} @keyframes host-a_foobar {}";
    assert_eq!(shim(css, "host-a", ""), expected);
}

#[test]
fn should_correctly_process_keyframes_defined_without_any_prefixed_space() {
    let css = ".test{display: flex;animation:bar 1s forwards;}@keyframes bar {}";
    let expected = ".test[host-a]{display: flex;animation:host-a_bar 1s forwards;}@keyframes host-a_bar {}";
    assert_eq!(shim(css, "host-a", ""), expected);
    
    let css = ".test{animation:bar 2s forwards;}@-webkit-keyframes bar {}";
    let expected = ".test[host-a]{animation:host-a_bar 2s forwards;}@-webkit-keyframes host-a_bar {}";
    assert_eq!(shim(css, "host-a", ""), expected);
}

#[test]
fn should_ignore_keywords_values_when_scoping_local_animations() {
    let css = "
        div {
            animation: inherit;
            animation: unset;
            animation: 3s ease reverse foo;
            animation: 5s foo 1s backwards;
            animation: none 1s foo;
            animation: .5s foo paused;
            animation: 1s running foo;
            animation: 3s linear 1s infinite running foo;
            animation: 5s foo ease;
            animation: 3s .5s infinite steps(3,end) foo;
            animation: 5s steps(9, jump-start) jump .5s;
            animation: 1s step-end steps;
        }

        @keyframes foo {}
        @keyframes inherit {}
        @keyframes unset {}
        @keyframes ease {}
        @keyframes reverse {}
        @keyframes backwards {}
        @keyframes none {}
        @keyframes paused {}
        @keyframes linear {}
        @keyframes running {}
        @keyframes end {}
        @keyframes jump {}
        @keyframes start {}
        @keyframes steps {}
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "animation: inherit;");
    assert_contains(&result, "animation: unset;");
    assert_contains(&result, "animation: 3s ease reverse host-a_foo;");
    assert_contains(&result, "animation: 5s host-a_foo 1s backwards;");
    assert_contains(&result, "animation: none 1s host-a_foo;");
    assert_contains(&result, "animation: .5s host-a_foo paused;");
    assert_contains(&result, "animation: 1s running host-a_foo;");
    assert_contains(&result, "animation: 3s linear 1s infinite running host-a_foo;");
    assert_contains(&result, "animation: 5s host-a_foo ease;");
    assert_contains(&result, "animation: 3s .5s infinite steps(3,end) host-a_foo;");
    assert_contains(&result, "animation: 5s steps(9, jump-start) host-a_jump .5s;");
    assert_contains(&result, "animation: 1s step-end host-a_steps;");
}

#[test]
fn should_handle_the_usage_of_quotes() {
    let css = "
        div {
            animation: 1.5s foo;
        }

        p {
            animation: 1s 'foz bar';
        }

        @keyframes 'foo' {}
        @keyframes \"foz bar\" {}
        @keyframes bar {}
        @keyframes baz {}
        @keyframes qux {}
        @keyframes quux {}
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "@keyframes 'host-a_foo' {}");
    assert_contains(&result, "@keyframes \"host-a_foz bar\" {}");
    assert_contains(&result, "animation: 1.5s host-a_foo;");
    assert_contains(&result, "animation: 1s 'host-a_foz bar';");
}

#[test]
fn should_handle_the_usage_of_quotes_containing_escaped_quotes() {
    let css = "
        div {
            animation: 1.5s \"foo\\\"bar\";
        }

        p {
            animation: 1s 'bar\\' \\'baz';
        }

        button {
            animation-name: 'foz \" baz';
        }

        @keyframes \"foo\\\"bar\" {}
        @keyframes \"bar' 'baz\" {}
        @keyframes \"foz \\\" baz\" {}
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "@keyframes \"host-a_foo\\\"bar\" {}");
    assert_contains(&result, "@keyframes \"host-a_bar' 'baz\" {}");
    assert_contains(&result, "@keyframes \"host-a_foz \\\" baz\" {}");
    assert_contains(&result, "animation: 1.5s \"host-a_foo\\\"bar\";");
    assert_contains(&result, "animation: 1s 'host-a_bar\\' \\'baz';");
    assert_contains(&result, "animation-name: 'host-a_foz \" baz';");
}

#[test]
fn should_handle_the_usage_of_commas_in_multiple_animation_definitions_in_a_single_declaration() {
    let css = "
         button {
           animation: 1s \"foo bar, baz\", 2s 'qux quux';
         }

         div {
           animation: 500ms foo, 1s 'bar, baz', 1500ms bar;
         }

         p {
           animation: 3s \"bar, baz\", 3s 'foo, bar' 1s, 3s \"qux quux\";
         }

         @keyframes \"qux quux\" {}
         @keyframes \"bar, baz\" {}
       ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "@keyframes \"host-a_qux quux\" {}");
    assert_contains(&result, "@keyframes \"host-a_bar, baz\" {}");
    assert_contains(&result, "animation: 1s \"foo bar, baz\", 2s 'host-a_qux quux';");
    assert_contains(&result, "animation: 500ms foo, 1s 'host-a_bar, baz', 1500ms bar;");
    assert_contains(
        &result,
        "animation: 3s \"host-a_bar, baz\", 3s 'foo, bar' 1s, 3s \"host-a_qux quux\";",
    );
}

#[test]
fn should_handle_the_usage_of_double_quotes_escaping_in_multiple_animation_definitions_in_a_single_declaration() {
    let css = "
        div {
            animation: 1s \"foo\", 1.5s \"bar\";
            animation: 2s \"fo\\\"o\", 2.5s \"bar\";
            animation: 3s \"foo\\\"\", 3.5s \"bar\", 3.7s \"ba\\\"r\";
            animation: 4s \"foo\\\\\", 4.5s \"bar\", 4.7s \"baz\\\"\";
            animation: 5s \"fo\\\\\\\"o\", 5.5s \"bar\", 5.7s \"baz\\\"\";
        }

        @keyframes \"foo\" {}
        @keyframes \"fo\\\"o\" {}
        @keyframes 'foo\"' {}
        @keyframes 'foo\\\\' {}
        @keyframes bar {}
        @keyframes \"ba\\\"r\" {}
        @keyframes \"fo\\\\\\\"o\" {}
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "@keyframes \"host-a_foo\" {}");
    assert_contains(&result, "@keyframes \"host-a_fo\\\"o\" {}");
    assert_contains(&result, "@keyframes 'host-a_foo\"' {}");
    assert_contains(&result, "@keyframes 'host-a_foo\\\\' {}");
    assert_contains(&result, "@keyframes host-a_bar {}");
    assert_contains(&result, "@keyframes \"host-a_ba\\\"r\" {}");
    assert_contains(&result, "@keyframes \"host-a_fo\\\\\\\"o\"");
    assert_contains(&result, "animation: 1s \"host-a_foo\", 1.5s \"host-a_bar\";");
    assert_contains(&result, "animation: 2s \"host-a_fo\\\"o\", 2.5s \"host-a_bar\";");
    assert_contains(
        &result,
        "animation: 3s \"host-a_foo\\\"\", 3.5s \"host-a_bar\", 3.7s \"host-a_ba\\\"r\";",
    );
    assert_contains(&result, "animation: 4s \"host-a_foo\\\\\", 4.5s \"host-a_bar\", 4.7s \"baz\\\"\";");
    assert_contains(
        &result,
        "animation: 5s \"host-a_fo\\\\\\\"o\", 5.5s \"host-a_bar\", 5.7s \"baz\\\"\";",
    );
}

#[test]
fn should_handle_the_usage_of_single_quotes_escaping_in_multiple_animation_definitions_in_a_single_declaration() {
    let css = "
        div {
            animation: 1s 'foo', 1.5s 'bar';
            animation: 2s 'fo\\'o', 2.5s 'bar';
            animation: 3s 'foo\\'', 3.5s 'bar', 3.7s 'ba\\'r';
            animation: 4s 'foo\\\\', 4.5s 'bar', 4.7s 'baz\\'';
            animation: 5s 'fo\\\\\\'o', 5.5s 'bar', 5.7s 'baz\\'';
        }

        @keyframes foo {}
        @keyframes 'fo\\'o' {}
        @keyframes 'foo\\'' {}
        @keyframes 'foo\\\\' {}
        @keyframes \"bar\" {}
        @keyframes 'ba\\'r' {}
        @keyframes \"fo\\\\\\'o\" {}
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "@keyframes host-a_foo {}");
    assert_contains(&result, "@keyframes 'host-a_fo\\'o' {}");
    assert_contains(&result, "@keyframes 'host-a_foo\\'' {}");
    assert_contains(&result, "@keyframes 'host-a_foo\\\\' {}");
    assert_contains(&result, "@keyframes \"host-a_bar\" {}");
    assert_contains(&result, "@keyframes 'host-a_ba\\'r' {}");
    assert_contains(&result, "@keyframes \"host-a_fo\\\\\\'o\" {}");
    assert_contains(&result, "animation: 1s 'host-a_foo', 1.5s 'host-a_bar';");
    assert_contains(&result, "animation: 2s 'host-a_fo\\'o', 2.5s 'host-a_bar';");
    assert_contains(
        &result,
        "animation: 3s 'host-a_foo\\'', 3.5s 'host-a_bar', 3.7s 'host-a_ba\\'r';",
    );
    assert_contains(&result, "animation: 4s 'host-a_foo\\\\', 4.5s 'host-a_bar', 4.7s 'baz\\'';");
    assert_contains(&result, "animation: 5s 'host-a_fo\\\\\\'o', 5.5s 'host-a_bar', 5.7s 'baz\\''");
}

#[test]
fn should_handle_the_usage_of_mixed_single_and_double_quotes_escaping_in_multiple_animation_definitions_in_a_single_declaration() {
    let css = "
        div {
            animation: 1s 'f\\\"oo', 1.5s \"ba\\'r\";
            animation: 2s \"fo\\\"\\\"o\", 2.5s 'b\\\\\"ar';
            animation: 3s 'foo\\\\', 3.5s \"b\\\\\\\"ar\", 3.7s 'ba\\'\\\"\\'r';
            animation: 4s 'fo\\'o', 4.5s 'b\\\"ar\\\"', 4.7s \"baz\\'\";
        }

        @keyframes 'f\"oo' {}
        @keyframes 'fo\"\"o' {}
        @keyframes 'foo\\\\' {}
        @keyframes 'fo\\'o' {}
        @keyframes 'ba\\'r' {}
        @keyframes 'b\\\\\"ar' {}
        @keyframes 'b\\\\\\\"ar' {}
        @keyframes 'b\"ar\"' {}
        @keyframes 'ba\\'\\\"\\'r' {}
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "@keyframes 'host-a_f\"oo' {}");
    assert_contains(&result, "@keyframes 'host-a_fo\"\"o' {}");
    assert_contains(&result, "@keyframes 'host-a_foo\\\\' {}");
    assert_contains(&result, "@keyframes 'host-a_fo\\'o' {}");
    assert_contains(&result, "@keyframes 'host-a_ba\\'r' {}");
    assert_contains(&result, "@keyframes 'host-a_b\\\\\"ar' {}");
    assert_contains(&result, "@keyframes 'host-a_b\\\\\\\"ar' {}");
    assert_contains(&result, "@keyframes 'host-a_b\"ar\"' {}");
    assert_contains(&result, "@keyframes 'host-a_ba\\'\\\"\\'r' {}");
    assert_contains(&result, "animation: 1s 'host-a_f\\\"oo', 1.5s \"host-a_ba\\'r\";");
    assert_contains(&result, "animation: 2s \"host-a_fo\\\"\\\"o\", 2.5s 'host-a_b\\\\\"ar';");
    assert_contains(
        &result,
        "animation: 3s 'host-a_foo\\\\', 3.5s \"host-a_b\\\\\\\"ar\", 3.7s 'host-a_ba\\'\\\"\\'r';",
    );
    assert_contains(
        &result,
        "animation: 4s 'host-a_fo\\'o', 4.5s 'host-a_b\\\"ar\\\"', 4.7s \"baz\\'\";",
    );
}

#[test]
fn should_handle_the_usage_of_commas_inside_quotes() {
    let css = "
        div {
            animation: 3s 'bar,, baz';
        }

        p {
            animation-name: \"bar,, baz\", foo,'ease, linear , inherit', bar;
        }

        @keyframes 'foo' {}
        @keyframes 'bar,, baz' {}
        @keyframes 'ease, linear , inherit' {}
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "@keyframes 'host-a_bar,, baz' {}");
    assert_contains(&result, "animation: 3s 'host-a_bar,, baz';");
    assert_contains(
        &result,
        "animation-name: \"host-a_bar,, baz\", host-a_foo,'host-a_ease, linear , inherit', bar;",
    );
}

#[test]
fn should_not_ignore_animation_keywords_when_they_are_inside_quotes() {
    let css = "
        div {
            animation: 3s 'unset';
        }

        button {
            animation: 5s \"forwards\" 1s forwards;
        }

        @keyframes unset {}
        @keyframes forwards {}
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "@keyframes host-a_unset {}");
    assert_contains(&result, "@keyframes host-a_forwards {}");
    assert_contains(&result, "animation: 3s 'host-a_unset';");
    assert_contains(&result, "animation: 5s \"host-a_forwards\" 1s forwards;");
}

#[test]
fn should_handle_css_functions_correctly() {
    let css = "
        div {
            animation: foo 0.5s alternate infinite cubic-bezier(.17, .67, .83, .67);
        }

        button {
            animation: calc(2s / 2) calc;
        }

        @keyframes foo {}
        @keyframes cubic-bezier {}
        @keyframes calc {}
        ";
    let result = shim(css, "host-a", "");
    assert_contains(&result, "@keyframes host-a_cubic-bezier {}");
    assert_contains(&result, "@keyframes host-a_calc {}");
    assert_contains(
        &result,
        "animation: host-a_foo 0.5s alternate infinite cubic-bezier(.17, .67, .83, .67);",
    );
    assert_contains(&result, "animation: calc(2s / 2) host-a_calc;");
}

