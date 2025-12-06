//! Repeat Groups Tests
//!
//! Corresponds to packages/compiler/test/shadow_css/repeat_groups_spec.ts
//! All test cases match exactly with TypeScript version

use angular_compiler::shadow_css::repeat_groups;

#[test]
fn should_do_nothing_if_multiples_is_0() {
    let mut groups = vec![
        vec!["a1".to_string(), "b1".to_string(), "c1".to_string()],
        vec!["a2".to_string(), "b2".to_string(), "c2".to_string()],
    ];
    let expected = groups.clone();
    repeat_groups(&mut groups, 0);
    assert_eq!(groups, expected);
}

#[test]
fn should_do_nothing_if_multiples_is_1() {
    let mut groups = vec![
        vec!["a1".to_string(), "b1".to_string(), "c1".to_string()],
        vec!["a2".to_string(), "b2".to_string(), "c2".to_string()],
    ];
    let expected = groups.clone();
    repeat_groups(&mut groups, 1);
    assert_eq!(groups, expected);
}

#[test]
fn should_add_clones_of_the_original_groups_if_multiples_is_greater_than_1() {
    let group1 = vec!["a1".to_string(), "b1".to_string(), "c1".to_string()];
    let group2 = vec!["a2".to_string(), "b2".to_string(), "c2".to_string()];
    let mut groups = vec![group1.clone(), group2.clone()];
    repeat_groups(&mut groups, 3);
    
    assert_eq!(groups.len(), 6);
    assert_eq!(groups[0], group1);
    assert_eq!(groups[1], group2);
    assert_eq!(groups[2], group1);
    assert_eq!(groups[3], group2);
    assert_eq!(groups[4], group1);
    assert_eq!(groups[5], group2);
}

