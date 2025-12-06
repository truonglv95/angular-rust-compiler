//! Host and Host Context Tests
//!
//! Corresponds to packages/compiler/test/shadow_css/host_and_host_context_spec.ts
//! All test cases match exactly with TypeScript version

mod utils;
use utils::{assert_equal_css, shim};

#[test]
fn should_handle_no_context() {
    assert_equal_css(&shim(":host {}", "contenta", "a-host"), "[a-host] {}");
}

#[test]
fn should_handle_tag_selector() {
    assert_equal_css(&shim(":host(ul) {}", "contenta", "a-host"), "ul[a-host] {}");
}

#[test]
fn should_handle_class_selector() {
    assert_equal_css(&shim(":host(.x) {}", "contenta", "a-host"), ".x[a-host] {}");
}

#[test]
fn should_handle_attribute_selector() {
    assert_equal_css(
        &shim(":host([a=\"b\"]) {}", "contenta", "a-host"),
        "[a=\"b\"][a-host] {}",
    );
    assert_equal_css(&shim(":host([a=b]) {}", "contenta", "a-host"), "[a=b][a-host] {}");
}

#[test]
fn should_handle_attribute_and_next_operator_without_spaces() {
    assert_equal_css(
        &shim(":host[foo]>div {}", "contenta", "a-host"),
        "[foo][a-host] > div[contenta] {}",
    );
}

// Note: xit test skipped - "should handle host with escaped class selector"
// This test is known to not pass in TypeScript version

#[test]
fn should_handle_multiple_tag_selectors() {
    assert_equal_css(
        &shim(":host(ul,li) {}", "contenta", "a-host"),
        "ul[a-host], li[a-host] {}",
    );
    assert_equal_css(
        &shim(":host(ul,li) > .z {}", "contenta", "a-host"),
        "ul[a-host] > .z[contenta], li[a-host] > .z[contenta] {}",
    );
}

#[test]
fn should_handle_compound_class_selectors() {
    assert_equal_css(&shim(":host(.a.b) {}", "contenta", "a-host"), ".a.b[a-host] {}");
}

#[test]
fn should_handle_multiple_class_selectors() {
    assert_equal_css(
        &shim(":host(.x,.y) {}", "contenta", "a-host"),
        ".x[a-host], .y[a-host] {}",
    );
    assert_equal_css(
        &shim(":host(.x,.y) > .z {}", "contenta", "a-host"),
        ".x[a-host] > .z[contenta], .y[a-host] > .z[contenta] {}",
    );
}

#[test]
fn should_handle_multiple_attribute_selectors() {
    assert_equal_css(
        &shim(":host([a=\"b\"],[c=d]) {}", "contenta", "a-host"),
        "[a=\"b\"][a-host], [c=d][a-host] {}",
    );
}

#[test]
fn should_handle_pseudo_selectors() {
    assert_equal_css(
        &shim(":host(:before) {}", "contenta", "a-host"),
        "[a-host]:before {}",
    );
    assert_equal_css(&shim(":host:before {}", "contenta", "a-host"), "[a-host]:before {}");
    assert_equal_css(
        &shim(":host:nth-child(8n+1) {}", "contenta", "a-host"),
        "[a-host]:nth-child(8n+1) {}",
    );
    assert_equal_css(
        &shim(":host(:nth-child(3n of :not(p, a))) {}", "contenta", "a-host"),
        "[a-host]:nth-child(3n of :not(p, a)) {}",
    );
    assert_equal_css(
        &shim(":host:nth-of-type(8n+1) {}", "contenta", "a-host"),
        "[a-host]:nth-of-type(8n+1) {}",
    );
    assert_equal_css(
        &shim(":host(.class):before {}", "contenta", "a-host"),
        ".class[a-host]:before {}",
    );
    assert_equal_css(
        &shim(":host.class:before {}", "contenta", "a-host"),
        ".class[a-host]:before {}",
    );
    assert_equal_css(
        &shim(":host(:not(p)):before {}", "contenta", "a-host"),
        "[a-host]:not(p):before {}",
    );
    assert_equal_css(
        &shim(":host(:not(:has(p))) {}", "contenta", "a-host"),
        "[a-host]:not(:has(p)) {}",
    );
    assert_equal_css(
        &shim(":host:not(:host.foo) {}", "contenta", "a-host"),
        "[a-host]:not([a-host].foo) {}",
    );
    assert_equal_css(
        &shim(":host:not(.foo:host) {}", "contenta", "a-host"),
        "[a-host]:not(.foo[a-host]) {}",
    );
    assert_equal_css(
        &shim(":host:not(:host.foo, :host.bar) {}", "contenta", "a-host"),
        "[a-host]:not([a-host].foo, .bar[a-host]) {}",
    );
    assert_equal_css(
        &shim(":host:not(:host.foo, .bar :host) {}", "contenta", "a-host"),
        "[a-host]:not([a-host].foo, .bar [a-host]) {}",
    );
    assert_equal_css(
        &shim(":host:not(.foo, .bar) {}", "contenta", "a-host"),
        "[a-host]:not(.foo, .bar) {}",
    );
    assert_equal_css(
        &shim(":host:not(:has(p, a)) {}", "contenta", "a-host"),
        "[a-host]:not(:has(p, a)) {}",
    );
    assert_equal_css(
        &shim(":host(:not(.foo, .bar)) {}", "contenta", "a-host"),
        "[a-host]:not(.foo, .bar) {}",
    );
    assert_equal_css(
        &shim(":host:has(> child-element:not(.foo)) {}", "contenta", "a-host"),
        "[a-host]:has(> child-element:not(.foo)) {}",
    );
}

// see b/63672152
#[test]
fn should_handle_unexpected_selectors_in_the_most_reasonable_way() {
    assert_equal_css(&shim("cmp:host {}", "contenta", "a-host"), "cmp[a-host] {}");
    assert_equal_css(&shim("cmp:host >>> {}", "contenta", "a-host"), "cmp[a-host] {}");
    assert_equal_css(
        &shim("cmp:host child {}", "contenta", "a-host"),
        "cmp[a-host] child[contenta] {}",
    );
    assert_equal_css(
        &shim("cmp:host >>> child {}", "contenta", "a-host"),
        "cmp[a-host] child {}",
    );
    assert_equal_css(&shim("cmp :host {}", "contenta", "a-host"), "cmp [a-host] {}");
    assert_equal_css(&shim("cmp :host >>> {}", "contenta", "a-host"), "cmp [a-host] {}");
    assert_equal_css(
        &shim("cmp :host child {}", "contenta", "a-host"),
        "cmp [a-host] child[contenta] {}",
    );
    assert_equal_css(
        &shim("cmp :host >>> child {}", "contenta", "a-host"),
        "cmp [a-host] child {}",
    );
}

#[test]
fn should_support_newlines_in_the_same_selector_and_content() {
    let selector = ".foo:not(
        :host) {
          background-color:
            green;
      }";
    assert_equal_css(
        &shim(selector, "contenta", "a-host"),
        ".foo[contenta]:not( [a-host]) { background-color:green;}",
    );
}

#[test]
fn should_transform_host_context_with_pseudo_selectors() {
    assert_equal_css(
        &shim(":host-context(backdrop:not(.borderless)) .backdrop {}", "contenta", "hosta"),
        "backdrop:not(.borderless)[hosta] .backdrop[contenta], backdrop:not(.borderless) [hosta] .backdrop[contenta] {}",
    );
    assert_equal_css(
        &shim(":where(:host-context(backdrop)) {}", "contenta", "hosta"),
        ":where(backdrop[hosta]), :where(backdrop [hosta]) {}",
    );
    assert_equal_css(
        &shim(":where(:host-context(outer1)) :host(bar) {}", "contenta", "hosta"),
        ":where(outer1) bar[hosta] {}",
    );
    assert_equal_css(
        &shim(":where(:host-context(.one)) :where(:host-context(.two)) {}", "contenta", "a-host"),
        ":where(.one.two[a-host]), :where(.one.two [a-host]), :where(.one .two[a-host]), :where(.one .two [a-host]), :where(.two .one[a-host]), :where(.two .one [a-host]) {}",
    );
    assert_equal_css(
        &shim(":where(:host-context(backdrop)) .foo ~ .bar {}", "contenta", "hosta"),
        ":where(backdrop[hosta]) .foo[contenta] ~ .bar[contenta], :where(backdrop [hosta]) .foo[contenta] ~ .bar[contenta] {}",
    );
    assert_equal_css(
        &shim(":where(:host-context(backdrop)) :host {}", "contenta", "hosta"),
        ":where(backdrop) [hosta] {}",
    );
    assert_equal_css(
        &shim("div:where(:host-context(backdrop)) :host {}", "contenta", "hosta"),
        "div:where(backdrop) [hosta] {}",
    );
}

#[test]
fn should_transform_host_context_with_nested_pseudo_selectors() {
    assert_equal_css(
        &shim(":host-context(:where(.foo:not(.bar))) {}", "contenta", "hosta"),
        ":where(.foo:not(.bar))[hosta], :where(.foo:not(.bar)) [hosta] {}",
    );
    assert_equal_css(
        &shim(":host-context(:is(.foo:not(.bar))) {}", "contenta", "hosta"),
        ":is(.foo:not(.bar))[hosta], :is(.foo:not(.bar)) [hosta] {}",
    );
    assert_equal_css(
        &shim(":host-context(:where(.foo:not(.bar, .baz))) .inner {}", "contenta", "hosta"),
        ":where(.foo:not(.bar, .baz))[hosta] .inner[contenta], :where(.foo:not(.bar, .baz)) [hosta] .inner[contenta] {}",
    );
}

#[test]
fn should_handle_tag_selector_host_context() {
    assert_equal_css(
        &shim(":host-context(div) {}", "contenta", "a-host"),
        "div[a-host], div [a-host] {}",
    );
    assert_equal_css(
        &shim(":host-context(ul) > .y {}", "contenta", "a-host"),
        "ul[a-host] > .y[contenta], ul [a-host] > .y[contenta] {}",
    );
}

#[test]
fn should_handle_class_selector_host_context() {
    assert_equal_css(
        &shim(":host-context(.x) {}", "contenta", "a-host"),
        ".x[a-host], .x [a-host] {}",
    );
    assert_equal_css(
        &shim(":host-context(.x) > .y {}", "contenta", "a-host"),
        ".x[a-host] > .y[contenta], .x [a-host] > .y[contenta] {}",
    );
}

#[test]
fn should_handle_attribute_selector_host_context() {
    assert_equal_css(
        &shim(":host-context([a=\"b\"]) {}", "contenta", "a-host"),
        "[a=\"b\"][a-host], [a=\"b\"] [a-host] {}",
    );
    assert_equal_css(
        &shim(":host-context([a=b]) {}", "contenta", "a-host"),
        "[a=b][a-host], [a=b] [a-host] {}",
    );
}

#[test]
fn should_handle_multiple_host_context_selectors() {
    assert_equal_css(
        &shim(":host-context(.one):host-context(.two) {}", "contenta", "a-host"),
        ".one.two[a-host], .one.two [a-host], .one .two[a-host], .one .two [a-host], .two .one[a-host], .two .one [a-host] {}",
    );
    
    // Test with 3 selectors - very long expected string
    let expected = ".X.Y.Z[a-host], .X.Y.Z [a-host], .X.Y .Z[a-host], .X.Y .Z [a-host], .X.Z .Y[a-host], .X.Z .Y [a-host], .X .Y.Z[a-host], .X .Y.Z [a-host], .X .Y .Z[a-host], .X .Y .Z [a-host], .X .Z .Y[a-host], .X .Z .Y [a-host], .Y.Z .X[a-host], .Y.Z .X [a-host], .Y .Z .X[a-host], .Y .Z .X [a-host], .Z .Y .X[a-host], .Z .Y .X [a-host] {}";
    assert_equal_css(
        &shim(":host-context(.X):host-context(.Y):host-context(.Z) {}", "contenta", "a-host"),
        expected,
    );
}

#[test]
fn should_handle_host_context_with_no_ancestor_selectors() {
    assert_equal_css(
        &shim(":host-context .inner {}", "contenta", "a-host"),
        "[a-host] .inner[contenta] {}",
    );
    assert_equal_css(
        &shim(":host-context() .inner {}", "contenta", "a-host"),
        "[a-host] .inner[contenta] {}",
    );
    assert_equal_css(
        &shim(":host-context :host-context(.a) {}", "contenta", "host-a"),
        ".a[host-a], .a [host-a] {}",
    );
}

#[test]
fn should_handle_selectors_host_context() {
    assert_equal_css(
        &shim(":host-context(.one,.two) .inner {}", "contenta", "a-host"),
        ".one[a-host] .inner[contenta], .one [a-host] .inner[contenta], .two[a-host] .inner[contenta], .two [a-host] .inner[contenta] {}",
    );
}

#[test]
fn should_handle_host_context_with_comma_separated_child_selector() {
    assert_equal_css(
        &shim(":host-context(.foo) a:not(.a, .b) {}", "contenta", "a-host"),
        ".foo[a-host] a[contenta]:not(.a, .b), .foo [a-host] a[contenta]:not(.a, .b) {}",
    );
    assert_equal_css(
        &shim(
            ":host-context(.foo) a:not([a], .b), .bar, :host-context(.baz) a:not([c], .d) {}",
            "contenta",
            "a-host",
        ),
        ".foo[a-host] a[contenta]:not([a], .b), .foo [a-host] a[contenta]:not([a], .b), .bar[contenta], .baz[a-host] a[contenta]:not([c], .d), .baz [a-host] a[contenta]:not([c], .d) {}",
    );
}

#[test]
fn should_handle_selectors_on_the_same_element() {
    assert_equal_css(
        &shim(":host-context(div):host(.x) > .y {}", "contenta", "a-host"),
        "div.x[a-host] > .y[contenta] {}",
    );
}

#[test]
fn should_handle_no_selector_host() {
    assert_equal_css(
        &shim(":host:host-context(.one) {}", "contenta", "a-host"),
        ".one[a-host][a-host], .one [a-host] {}",
    );
    assert_equal_css(
        &shim(":host-context(.one) :host {}", "contenta", "a-host"),
        ".one [a-host] {}",
    );
}

#[test]
fn should_handle_selectors_on_different_elements() {
    assert_equal_css(
        &shim(":host-context(div) :host(.x) > .y {}", "contenta", "a-host"),
        "div .x[a-host] > .y[contenta] {}",
    );
    assert_equal_css(
        &shim(":host-context(div) > :host(.x) > .y {}", "contenta", "a-host"),
        "div > .x[a-host] > .y[contenta] {}",
    );
}

#[test]
fn should_parse_multiple_rules_containing_host_context_and_host() {
    let input = "
            :host-context(outer1) :host(bar) {}
            :host-context(outer2) :host(foo) {}
        ";
    assert_equal_css(
        &shim(input, "contenta", "a-host"),
        "outer1 bar[a-host] {} outer2 foo[a-host] {}",
    );
}

