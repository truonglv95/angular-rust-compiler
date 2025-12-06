//! Process Rules Tests
//!
//! Corresponds to packages/compiler/test/shadow_css/process_rules_spec.ts
//! All test cases match exactly with TypeScript version

use angular_compiler::shadow_css::{process_rules, CssRule};
use std::cell::RefCell;

#[test]
fn should_work_with_empty_css() {
    let rules = capture_rules("");
    assert_eq!(rules.len(), 0);
}

#[test]
fn should_capture_a_rule_without_body() {
    let rules = capture_rules("a;");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].selector, "a");
    assert_eq!(rules[0].content, "");
}

#[test]
fn should_capture_css_rules_with_body() {
    let rules = capture_rules("a {b}");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].selector, "a");
    assert_eq!(rules[0].content, "b");
}

#[test]
fn should_capture_css_rules_with_nested_rules() {
    let rules = capture_rules("a {b {c}} d {e}");
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].selector, "a");
    assert_eq!(rules[0].content, "b {c}");
    assert_eq!(rules[1].selector, "d");
    assert_eq!(rules[1].content, "e");
}

#[test]
fn should_capture_multiple_rules_where_some_have_no_body() {
    let rules = capture_rules("@import a ; b {c}");
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].selector, "@import a");
    assert_eq!(rules[0].content, "");
    assert_eq!(rules[1].selector, "b");
    assert_eq!(rules[1].content, "c");
}

#[test]
fn should_allow_to_change_the_selector_while_preserving_whitespaces() {
    let result = process_rules(
        "@import a; b {c {d}} e {f}",
        |css_rule: CssRule| CssRule::new(
            format!("{}2", css_rule.selector),
            css_rule.content,
        ),
    );
    assert_eq!(result, "@import a2; b2 {c {d}} e2 {f}");
}

#[test]
fn should_allow_to_change_the_content() {
    let result = process_rules(
        "a {b}",
        |css_rule: CssRule| CssRule::new(
            css_rule.selector,
            format!("{}2", css_rule.content),
        ),
    );
    assert_eq!(result, "a {b2}");
}

fn capture_rules(input: &str) -> Vec<CssRule> {
    let result = RefCell::new(Vec::new());
    process_rules(input, |css_rule: CssRule| {
        result.borrow_mut().push(css_rule.clone());
        css_rule
    });
    result.into_inner()
}

