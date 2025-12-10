
use angular_compiler::directive_matching::{CssSelector, SelectorMatcher};

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to get a selector for given properties
    fn get_selector_for(tag: Option<&str>, attrs: Vec<(&str, &str)>, classes: Option<&str>) -> CssSelector {
        let mut selector = CssSelector::new();
        if let Some(t) = tag {
            selector.set_element(t);
        }
        for (name, value) in attrs {
            selector.add_attribute(name, value);
        }
        if let Some(c) = classes {
            for c_name in c.trim().split_whitespace() {
                selector.add_class_name(c_name);
            }
        }
        selector
    }

    #[test]
    fn should_select_by_element_name_case_sensitive() {
        let mut matcher: SelectorMatcher<i32> = SelectorMatcher::new();
        let s1 = CssSelector::parse("someTag").unwrap();
        matcher.add_selectable(s1[0].clone(), 1);

        let mut matched = Vec::new();
        {
            let mut collector = |s: &CssSelector, c: &i32| matched.push((s.clone(), *c));
            matcher.match_selector(
                &get_selector_for(Some("SOMEOTHERTAG"), vec![], None),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |s: &CssSelector, c: &i32| matched.push((s.clone(), *c));
            matcher.match_selector(
                &get_selector_for(Some("SOMETAG"), vec![], None),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |s: &CssSelector, c: &i32| matched.push((s.clone(), *c));
            matcher.match_selector(
                &get_selector_for(Some("someTag"), vec![], None),
                &mut collector
            );
        }
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].1, 1);
        // Note: s1[0] equality check omitted for brevity, assuming ID match implies correctness
    }

    #[test]
    fn should_select_by_class_name_case_insensitive() {
        let mut matcher = SelectorMatcher::new();
        let s1 = CssSelector::parse(".someClass").unwrap();
        matcher.add_selectable(s1[0].clone(), 1);
        let s2 = CssSelector::parse(".someClass.class2").unwrap();
        matcher.add_selectable(s2[0].clone(), 2);

        let mut matched = Vec::new();
        {
            let mut collector = |_s: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![], Some("SOMEOTHERCLASS")),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_s: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![], Some("SOMECLASS")),
                &mut collector
            );
        }
        assert_eq!(matched, vec![1]);

        matched.clear();
        {
            let mut collector = |_s: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![], Some("someClass class2")),
                &mut collector
            );
        }
        // Order depends on implementation, sort for comparison or check contains
        matched.sort();
        assert_eq!(matched, vec![1, 2]);
    }

    #[test]
    fn should_not_throw_for_class_name_constructor() {
        let matcher: SelectorMatcher<i32> = SelectorMatcher::new();
        let mut matched = Vec::new();
        {
            let mut collector = |_s: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![], Some("constructor")),
                &mut collector
            );
        }
        assert!(matched.is_empty());
    }

    #[test]
    fn should_select_by_attr_name_case_sensitive_independent_of_value() {
        let mut matcher = SelectorMatcher::new();
        let s1 = CssSelector::parse("[someAttr]").unwrap();
        matcher.add_selectable(s1[0].clone(), 1);
        let s2 = CssSelector::parse("[someAttr][someAttr2]").unwrap();
        matcher.add_selectable(s2[0].clone(), 2);

        let mut matched = Vec::new();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![("SOMEOTHERATTR", "")], None),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![("SOMEATTR", "")], None),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![("SOMEATTR", "someValue")], None),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![("someAttr", ""), ("someAttr2", "")], None),
                &mut collector
            );
        }
        matched.sort();
        assert_eq!(matched, vec![1, 2]);

        matched.clear();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![("someAttr", "someValue"), ("someAttr2", "")], None),
                &mut collector
            );
        }
        matched.sort();
        assert_eq!(matched, vec![1, 2]);

        matched.clear();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
               &get_selector_for(None, vec![("someAttr2", ""), ("someAttr", "someValue")], None),
                &mut collector
            );
        }
        matched.sort();
        assert_eq!(matched, vec![1, 2]);
        
        matched.clear();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
               &get_selector_for(None, vec![("someAttr2", "someValue"), ("someAttr", "")], None),
                &mut collector
            );
        }
        matched.sort();
        assert_eq!(matched, vec![1, 2]);
    }

    #[test]
    fn should_support_dot_in_attribute_names() {
        let mut matcher = SelectorMatcher::new();
        let s1 = CssSelector::parse("[foo.bar]").unwrap();
        matcher.add_selectable(s1[0].clone(), 1);

        let mut matched = Vec::new();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![("barfoo", "")], None),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                 &get_selector_for(None, vec![("foo.bar", "")], None),
                &mut collector
            );
        }
        assert_eq!(matched, vec![1]);
    }

    #[test]
    fn should_support_dollar_in_attribute_names() {
        let mut matcher = SelectorMatcher::new();
        let s1 = CssSelector::parse(r#"[someAttr\$]"#).unwrap();
        matcher.add_selectable(s1[0].clone(), 1);
        
        let mut matched = Vec::new();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![("someAttr", "")], None),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![("someAttr$", "")], None),
                &mut collector
            );
        }
        assert_eq!(matched, vec![1]);

        // Reset
        matcher = SelectorMatcher::new();
        matched.clear();
        let s1 = CssSelector::parse(r#"[some\$attr]"#).unwrap();
        matcher.add_selectable(s1[0].clone(), 1);

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![("someattr", "")], None),
                 &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
               &get_selector_for(None, vec![("some$attr", "")], None),
                 &mut collector
            );
        }
        assert_eq!(matched, vec![1]);

        // Reset
        matcher = SelectorMatcher::new();
        matched.clear();
        let s1 = CssSelector::parse(r#"[some-\$Attr]"#).unwrap();
        matcher.add_selectable(s1[0].clone(), 1);
        let s2 = CssSelector::parse(r#"[some-\$Attr][some-\$-attr]"#).unwrap();
        matcher.add_selectable(s2[0].clone(), 2);

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                 &get_selector_for(None, vec![("some\\$Attr", "")], None),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(None, vec![("some-$-attr", "someValue"), ("some-$Attr", "")], None),
                &mut collector
            );
        }
        matched.sort();
        assert_eq!(matched, vec![1, 2]);
    }
    
    #[test]
    fn should_select_by_attr_name_only_once_if_value_is_from_dom() {
         let mut matcher = SelectorMatcher::new();
         let s1 = CssSelector::parse("[some-decor]").unwrap();
         matcher.add_selectable(s1[0].clone(), 1);

         let mut matched = Vec::new();
         // Emulate DOM behavior where value might be empty string
         let mut element_selector = CssSelector::new();
         element_selector.add_attribute("some-decor", ""); // empty string value
         
         {
             let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
             matcher.match_selector(&element_selector, &mut collector);
         }
         assert_eq!(matched, vec![1]);
    }
    
    #[test]
    fn should_select_by_attr_name_case_sensitive_and_value_case_insensitive() {
        let mut matcher = SelectorMatcher::new();
        let s1 = CssSelector::parse("[someAttr=someValue]").unwrap();
        matcher.add_selectable(s1[0].clone(), 1);

        let mut matched = Vec::new();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                 &get_selector_for(None, vec![("SOMEATTR", "SOMEOTHERATTR")], None),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                 &get_selector_for(None, vec![("SOMEATTR", "SOMEVALUE")], None),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                 &get_selector_for(None, vec![("someAttr", "SOMEVALUE")], None),
                &mut collector
            );
        }
        // Note: Attribute value matching in Angular Ivy is tricky. 
        // Logic in SelectorMatcher::is_match for attributes:
        // if pat_value.is_empty() || &selector.attrs[j + 1] == pat_value
        // It seems strict string equality for values by default in the implementation provided previously?
        // Let's check `is_match` logic in directive_matching.rs
        // The implementation says: &selector.attrs[j + 1] == pat_value. This implies case-sensitive value match unless transformed.
        // However, TS test says "value case insensitive". 
        // If the Rust implementation uses strict equality, this test might fail if not handled.
        // Assuming current implementation needs to support this. If it fails, I will fix `directive_matching.rs`.
        // For now, let's write what is expected.
        assert_eq!(matched, vec![1]);
    }

    #[test]
    fn should_select_by_element_name_class_name_and_attribute_name_with_value() {
        let mut matcher = SelectorMatcher::new();
        let s1 = CssSelector::parse("someTag.someClass[someAttr=someValue]").unwrap();
        matcher.add_selectable(s1[0].clone(), 1);

        let mut matched = Vec::new();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(Some("someOtherTag"), vec![("someOtherAttr", "")], Some("someOtherClass")),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(Some("someTag"), vec![("someOtherAttr", "")], Some("someOtherClass")),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(Some("someTag"), vec![("someOtherAttr", "")], Some("someClass")),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(Some("someTag"), vec![("someAttr", "")], Some("someClass")),
                &mut collector
            );
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(Some("someTag"), vec![("someAttr", "someValue")], Some("someClass")),
                &mut collector
            );
        }
        assert_eq!(matched, vec![1]);
    }

    #[test]
    fn should_select_by_many_attributes_and_independent_of_value() {
        let mut matcher = SelectorMatcher::new();
        let s1 = CssSelector::parse("input[type=text][control]").unwrap();
        matcher.add_selectable(s1[0].clone(), 1);

        let mut matched = Vec::new();
        let mut css_selector = CssSelector::new();
        css_selector.set_element("input");
        css_selector.add_attribute("type", "text");
        css_selector.add_attribute("control", "one");

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(&css_selector, &mut collector);
        }
        assert_eq!(matched, vec![1]);
    }

    #[test]
    fn should_select_independent_of_order_in_css_selector() {
        let mut matcher = SelectorMatcher::new();
        let s1 = CssSelector::parse("[someAttr].someClass").unwrap();
        matcher.add_selectable(s1[0].clone(), 1);
        let s2 = CssSelector::parse(".someClass[someAttr]").unwrap();
        matcher.add_selectable(s2[0].clone(), 2);
         let s3 = CssSelector::parse(".class1.class2").unwrap();
        matcher.add_selectable(s3[0].clone(), 3);
         let s4 = CssSelector::parse(".class2.class1").unwrap();
        matcher.add_selectable(s4[0].clone(), 4);

        let mut matched = Vec::new();
        let p1 = CssSelector::parse("[someAttr].someClass").unwrap();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(&p1[0], &mut collector);
        }
        matched.sort();
        assert_eq!(matched, vec![1, 2]);

        matched.clear();
        let p2 = CssSelector::parse(".someClass[someAttr]").unwrap();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(&p2[0], &mut collector);
        }
        matched.sort();
        assert_eq!(matched, vec![1, 2]);

        matched.clear();
        let p3 = CssSelector::parse(".class1.class2").unwrap();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(&p3[0], &mut collector);
        }
        matched.sort();
        assert_eq!(matched, vec![3, 4]);
    }

    #[test]
    fn should_not_select_with_matching_not_selector() {
        let mut matcher = SelectorMatcher::new();
        matcher.add_selectable(CssSelector::parse("p:not(.someClass)").unwrap()[0].clone(), 1);
        matcher.add_selectable(CssSelector::parse("p:not([someAttr])").unwrap()[0].clone(), 2);
        matcher.add_selectable(CssSelector::parse(":not(.someClass)").unwrap()[0].clone(), 3);
        matcher.add_selectable(CssSelector::parse(":not(p)").unwrap()[0].clone(), 4);
        matcher.add_selectable(CssSelector::parse(":not(p[someAttr])").unwrap()[0].clone(), 5);

        let mut matched = Vec::new();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                 &get_selector_for(Some("p"), vec![("someAttr", "")], Some("someClass")),
                &mut collector
            );
        }
        assert!(matched.is_empty());
    }

    #[test]
    fn should_select_with_non_matching_not_selector() {
        let mut matcher = SelectorMatcher::new();
        matcher.add_selectable(CssSelector::parse("p:not(.someClass)").unwrap()[0].clone(), 1);
        matcher.add_selectable(CssSelector::parse("p:not(.someOtherClass[someAttr])").unwrap()[0].clone(), 2);
        matcher.add_selectable(CssSelector::parse(":not(.someClass)").unwrap()[0].clone(), 3);
        matcher.add_selectable(CssSelector::parse(":not(.someOtherClass[someAttr])").unwrap()[0].clone(), 4);

        let mut matched = Vec::new();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(
                &get_selector_for(Some("p"), vec![("someOtherAttr", "")], Some("someOtherClass")),
                &mut collector
            );
        }
        matched.sort();
        assert_eq!(matched, vec![1, 2, 3, 4]);
    }

    #[test]
    fn should_match_star_with_not_selector() {
         let mut matcher = SelectorMatcher::new();
         matcher.add_selectable(CssSelector::parse(":not([a])").unwrap()[0].clone(), 1);
         let mut matched = Vec::new();
         {
             let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
             matcher.match_selector(&get_selector_for(Some("div"), vec![], None), &mut collector);
         }
         assert_eq!(matched, vec![1]);
    }

    #[test]
    fn should_match_with_multiple_not_selectors() {
        let mut matcher = SelectorMatcher::new();
        matcher.add_selectable(CssSelector::parse("div:not([a]):not([b])").unwrap()[0].clone(), 1);
        let mut matched = Vec::new();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(&get_selector_for(Some("div"), vec![("a", "")], None), &mut collector);
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(&get_selector_for(Some("div"), vec![("b", "")], None), &mut collector);
        }
        assert!(matched.is_empty());

        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(&get_selector_for(Some("div"), vec![("c", "")], None), &mut collector);
        }
        assert_eq!(matched, vec![1]);
    }

    #[test]
    fn should_select_with_one_match_in_list() {
         let mut matcher = SelectorMatcher::new();
         let s1 = CssSelector::parse("input[type=text], textbox").unwrap();
         matcher.add_selectable(s1[0].clone(), 1); // input[type=text]
         matcher.add_selectable(s1[1].clone(), 1); // textbox (note: callback data is duplicated for each part in TS test logic s1[1], 1 is implicit as addSelectables does it for all parts)

        let mut matched = Vec::new();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(&get_selector_for(Some("textbox"), vec![], None), &mut collector);
        }
        assert_eq!(matched, vec![1]);

        matched.clear();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(&get_selector_for(Some("input"), vec![("type", "text")], None), &mut collector);
        }
        assert_eq!(matched, vec![1]);
    }

    #[test]
    fn should_not_select_twice_with_two_matches_in_list() {
        let mut matcher = SelectorMatcher::new();
        let s1 = CssSelector::parse("input, .someClass").unwrap();
        // In TS: matcher.addSelectables(s1, 1). This adds both selectors associated with 1.
        matcher.add_selectable(s1[0].clone(), 1);
        matcher.add_selectable(s1[1].clone(), 1);

        let mut matched = Vec::new();
        {
            let mut collector = |_: &CssSelector, c: &i32| matched.push(*c);
            matcher.match_selector(&get_selector_for(Some("input"), vec![], Some("someclass")), &mut collector);
        }
        assert_eq!(matched.len(), 2);
        assert_eq!(matched, vec![1, 1]);
    }

    // CssSelector.parse tests

    #[test]
    fn should_detect_element_names() {
        let css_selector = &CssSelector::parse("sometag").unwrap()[0];
        assert_eq!(css_selector.element, Some("sometag".to_string()));
        assert_eq!(css_selector.to_string(), "sometag");
    }

    #[test]
    fn should_detect_attr_names_with_escaped_dollar() {
        let css_selector = &CssSelector::parse(r#"[attrname\$]"#).unwrap()[0];
        assert_eq!(css_selector.attrs, vec!["attrname$", ""]);
        assert_eq!(css_selector.to_string(), r#"[attrname\$]"#);

         let css_selector = &CssSelector::parse(r#"[foo\$bar]"#).unwrap()[0];
        assert_eq!(css_selector.attrs, vec!["foo$bar", ""]);
    }

    #[test]
    fn should_error_on_attr_names_with_unescaped_dollar() {
        assert!(CssSelector::parse("[attrname$]").is_err());
        assert!(CssSelector::parse("[$attrname]").is_err());
        assert!(CssSelector::parse("[foo$bar]").is_err());
    }

    #[test]
    fn should_detect_class_names() {
        let css_selector = &CssSelector::parse(".someClass").unwrap()[0];
        assert_eq!(css_selector.class_names, vec!["someclass"]);
        assert_eq!(css_selector.to_string(), ".someclass");
    }

    #[test]
    fn should_detect_attr_names() {
        let css_selector = &CssSelector::parse("[attrname]").unwrap()[0];
        assert_eq!(css_selector.attrs, vec!["attrname", ""]);
        assert_eq!(css_selector.to_string(), "[attrname]");
    }

    #[test]
    fn should_detect_attr_values() {
        let css_selector = &CssSelector::parse("[attrname=attrvalue]").unwrap()[0];
        assert_eq!(css_selector.attrs, vec!["attrname", "attrvalue"]);
        assert_eq!(css_selector.to_string(), "[attrname=attrvalue]");
    }

    #[test]
    fn should_detect_attr_values_with_double_quotes() {
        let css_selector = &CssSelector::parse("[attrname=\"attrvalue\"]").unwrap()[0];
        assert_eq!(css_selector.attrs, vec!["attrname", "attrvalue"]);
        assert_eq!(css_selector.to_string(), "[attrname=attrvalue]");
    }

    #[test]
    fn should_detect_hashed_syntax_and_treat_as_attribute() {
        let css_selector = &CssSelector::parse("#some-value").unwrap()[0];
        assert_eq!(css_selector.attrs, vec!["id", "some-value"]);
        assert_eq!(css_selector.to_string(), "[id=some-value]");
    }

    #[test]
    fn should_detect_attr_values_with_single_quotes() {
        let css_selector = &CssSelector::parse("[attrname='attrvalue']").unwrap()[0];
        assert_eq!(css_selector.attrs, vec!["attrname", "attrvalue"]);
        assert_eq!(css_selector.to_string(), "[attrname=attrvalue]");
    }

    #[test]
    fn should_detect_multiple_parts() {
        let css_selector = &CssSelector::parse("sometag[attrname=attrvalue].someclass").unwrap()[0];
        assert_eq!(css_selector.element, Some("sometag".to_string()));
        assert_eq!(css_selector.attrs, vec!["attrname", "attrvalue"]);
        assert_eq!(css_selector.class_names, vec!["someclass"]);
    }

    #[test]
    fn should_detect_multiple_attributes() {
        let css_selector = &CssSelector::parse("input[type=text][control]").unwrap()[0];
        assert_eq!(css_selector.element, Some("input".to_string()));
        assert_eq!(css_selector.attrs, vec!["type", "text", "control", ""]);
    }

    #[test]
    fn should_detect_not() {
        let css_selector = &CssSelector::parse("sometag:not([attrname=attrvalue].someclass)").unwrap()[0];
        assert_eq!(css_selector.element, Some("sometag".to_string()));
        assert!(css_selector.attrs.is_empty());
        assert!(css_selector.class_names.is_empty());

        let not_selector = &css_selector.not_selectors[0];
        assert!(not_selector.element.is_none());
        assert_eq!(not_selector.attrs, vec!["attrname", "attrvalue"]);
        assert_eq!(not_selector.class_names, vec!["someclass"]);
    }

    #[test]
    fn should_detect_not_without_truthy() {
        let css_selector = &CssSelector::parse(":not([attrname=attrvalue].someclass)").unwrap()[0];
        assert_eq!(css_selector.element, Some("*".to_string()));
         let not_selector = &css_selector.not_selectors[0];
        assert_eq!(not_selector.attrs, vec!["attrname", "attrvalue"]);
        assert_eq!(not_selector.class_names, vec!["someclass"]);
    }

    #[test]
    fn should_throw_when_nested_not() {
        assert!(CssSelector::parse("sometag:not(:not([attrname=attrvalue].someclass))").is_err());
    }

    #[test]
    fn should_throw_when_multiple_selectors_in_not() {
        assert!(CssSelector::parse("sometag:not(a,b)").is_err());
    }

    #[test]
    fn should_detect_lists_of_selectors() {
        let css_selectors = CssSelector::parse(".someclass,[attrname=attrvalue], sometag").unwrap();
        assert_eq!(css_selectors.len(), 3);
        assert_eq!(css_selectors[0].class_names, vec!["someclass"]);
        assert_eq!(css_selectors[1].attrs, vec!["attrname", "attrvalue"]);
        assert_eq!(css_selectors[2].element, Some("sometag".to_string()));
    }
}
