//! Shadow CSS Tests
//!
//! Corresponds to packages/compiler/test/shadow_css/shadow_css_spec.ts
//! All test cases match exactly with TypeScript version

mod utils;
use utils::{assert_equal_css, shim};

#[test]
fn should_handle_empty_string() {
    assert_equal_css(&shim("", "contenta", ""), "");
}

#[test]
fn should_add_an_attribute_to_every_rule() {
    let css = "one {color: red;}two {color: red;}";
    let expected = "one[contenta] {color:red;}two[contenta] {color:red;}";
    assert_equal_css(&shim(css, "contenta", ""), expected);
}

#[test]
fn should_handle_invalid_css() {
    let css = "one {color: red;}garbage";
    let expected = "one[contenta] {color:red;}garbage";
    assert_equal_css(&shim(css, "contenta", ""), expected);
}

#[test]
fn should_add_an_attribute_to_every_selector() {
    let css = "one, two {color: red;}";
    let expected = "one[contenta], two[contenta] {color:red;}";
    assert_equal_css(&shim(css, "contenta", ""), expected);
}

#[test]
fn should_support_newlines_in_the_selector_and_content() {
    let css = "
      one,
      two {
        color: red;
      }
    ";
    let expected = "
      one[contenta],
      two[contenta] {
        color: red;
      }
    ";
    assert_equal_css(&shim(css, "contenta", ""), expected);
}

#[test]
fn should_support_newlines_in_the_same_selector_and_content() {
    let selector = ".foo:not(
      .bar) {
        background-color:
          green;
    }";
    assert_equal_css(
        &shim(selector, "contenta", "a-host"),
        ".foo[contenta]:not( .bar) { background-color:green;}",
    );
}

#[test]
fn should_handle_complicated_selectors() {
    assert_equal_css(&shim("one::before {}", "contenta", ""), "one[contenta]::before {}");
    assert_equal_css(&shim("one two {}", "contenta", ""), "one[contenta] two[contenta] {}");
    assert_equal_css(&shim("one > two {}", "contenta", ""), "one[contenta] > two[contenta] {}");
    assert_equal_css(&shim("one + two {}", "contenta", ""), "one[contenta] + two[contenta] {}");
    assert_equal_css(&shim("one ~ two {}", "contenta", ""), "one[contenta] ~ two[contenta] {}");
    assert_equal_css(
        &shim(".one.two > three {}", "contenta", ""),
        ".one.two[contenta] > three[contenta] {}",
    );
    assert_equal_css(
        &shim("one[attr=\"value\"] {}", "contenta", ""),
        "one[attr=\"value\"][contenta] {}",
    );
    assert_equal_css(
        &shim("one[attr=value] {}", "contenta", ""),
        "one[attr=value][contenta] {}",
    );
    assert_equal_css(
        &shim("one[attr^=\"value\"] {}", "contenta", ""),
        "one[attr^=\"value\"][contenta] {}",
    );
    assert_equal_css(
        &shim("one[attr$=\"value\"] {}", "contenta", ""),
        "one[attr$=\"value\"][contenta] {}",
    );
    assert_equal_css(
        &shim("one[attr*=\"value\"] {}", "contenta", ""),
        "one[attr*=\"value\"][contenta] {}",
    );
    assert_equal_css(
        &shim("one[attr|=\"value\"] {}", "contenta", ""),
        "one[attr|=\"value\"][contenta] {}",
    );
    assert_equal_css(
        &shim("one[attr~=\"value\"] {}", "contenta", ""),
        "one[attr~=\"value\"][contenta] {}",
    );
    assert_equal_css(
        &shim("one[attr=\"va lue\"] {}", "contenta", ""),
        "one[attr=\"va lue\"][contenta] {}",
    );
    assert_equal_css(&shim("one[attr] {}", "contenta", ""), "one[attr][contenta] {}");
    assert_equal_css(&shim("[is=\"one\"] {}", "contenta", ""), "[is=\"one\"][contenta] {}");
    assert_equal_css(&shim("[attr] {}", "contenta", ""), "[attr][contenta] {}");
}

#[test]
fn should_transform_host_with_attributes() {
    assert_equal_css(
        &shim(":host [attr] {}", "contenta", "hosta"),
        "[hosta] [attr][contenta] {}",
    );
    assert_equal_css(
        &shim(":host(create-first-project) {}", "contenta", "hosta"),
        "create-first-project[hosta] {}",
    );
    assert_equal_css(
        &shim(":host[attr] {}", "contenta", "hosta"),
        "[attr][hosta] {}",
    );
    assert_equal_css(
        &shim(":host[attr]:where(:not(.cm-button)) {}", "contenta", "hosta"),
        "[attr][hosta]:where(:not(.cm-button)) {}",
    );
}

#[test]
fn should_handle_escaped_sequences_in_selectors() {
    assert_equal_css(&shim("one\\/two {}", "contenta", ""), "one\\/two[contenta] {}");
    assert_equal_css(&shim("one\\:two {}", "contenta", ""), "one\\:two[contenta] {}");
    assert_equal_css(&shim("one\\\\:two {}", "contenta", ""), "one\\\\[contenta]:two {}");
    assert_equal_css(&shim(".one\\:two {}", "contenta", ""), ".one\\:two[contenta] {}");
    assert_equal_css(&shim(".one\\:\\fc ber {}", "contenta", ""), ".one\\:\\fc ber[contenta] {}");
    assert_equal_css(
        &shim(".one\\:two .three\\:four {}", "contenta", ""),
        ".one\\:two[contenta] .three\\:four[contenta] {}",
    );
    assert_equal_css(
        &shim("div:where(.one) {}", "contenta", "hosta"),
        "div[contenta]:where(.one) {}",
    );
    assert_equal_css(&shim("div:where() {}", "contenta", "hosta"), "div[contenta]:where() {}");
    assert_equal_css(
        &shim(":where(a):where(b) {}", "contenta", "hosta"),
        ":where(a[contenta]):where(b[contenta]) {}",
    );
    assert_equal_css(
        &shim("*:where(.one) {}", "contenta", "hosta"),
        "*[contenta]:where(.one) {}",
    );
    assert_equal_css(
        &shim("*:where(.one) ::ng-deep .foo {}", "contenta", "hosta"),
        "*[contenta]:where(.one) .foo {}",
    );
}

#[test]
fn should_handle_pseudo_functions_correctly() {
    // :where()
    assert_equal_css(
        &shim(":where(.one) {}", "contenta", "hosta"),
        ":where(.one[contenta]) {}",
    );
    assert_equal_css(
        &shim(":where(div.one span.two) {}", "contenta", "hosta"),
        ":where(div.one[contenta] span.two[contenta]) {}",
    );
    assert_equal_css(
        &shim(":where(.one) .two {}", "contenta", "hosta"),
        ":where(.one[contenta]) .two[contenta] {}",
    );
    assert_equal_css(
        &shim(":where(:host) {}", "contenta", "hosta"),
        ":where([hosta]) {}",
    );
    assert_equal_css(
        &shim(":where(:host) .one {}", "contenta", "hosta"),
        ":where([hosta]) .one[contenta] {}",
    );
    assert_equal_css(
        &shim(":where(.one) :where(:host) {}", "contenta", "hosta"),
        ":where(.one) :where([hosta]) {}",
    );
    assert_equal_css(
        &shim(":where(.one :host) {}", "contenta", "hosta"),
        ":where(.one [hosta]) {}",
    );
    assert_equal_css(
        &shim("div :where(.one) {}", "contenta", "hosta"),
        "div[contenta] :where(.one[contenta]) {}",
    );
    assert_equal_css(
        &shim(":host :where(.one .two) {}", "contenta", "hosta"),
        "[hosta] :where(.one[contenta] .two[contenta]) {}",
    );
    assert_equal_css(
        &shim(":where(.one, .two) {}", "contenta", "hosta"),
        ":where(.one[contenta], .two[contenta]) {}",
    );
    assert_equal_css(
        &shim(":where(.one > .two) {}", "contenta", "hosta"),
        ":where(.one[contenta] > .two[contenta]) {}",
    );
    assert_equal_css(
        &shim(":where(> .one) {}", "contenta", "hosta"),
        ":where( > .one[contenta]) {}",
    );
    assert_equal_css(
        &shim(":where(:not(.one) ~ .two) {}", "contenta", "hosta"),
        ":where([contenta]:not(.one) ~ .two[contenta]) {}",
    );
    assert_equal_css(
        &shim(":where([foo]) {}", "contenta", "hosta"),
        ":where([foo][contenta]) {}",
    );

    // :is()
    assert_equal_css(
        &shim("div:is(.foo) {}", "contenta", "a-host"),
        "div[contenta]:is(.foo) {}",
    );
    assert_equal_css(
        &shim(":is(.dark :host) {}", "contenta", "a-host"),
        ":is(.dark [a-host]) {}",
    );
    assert_equal_css(
        &shim(":is(.dark) :is(:host) {}", "contenta", "a-host"),
        ":is(.dark) :is([a-host]) {}",
    );
    assert_equal_css(
        &shim(":host:is(.foo) {}", "contenta", "a-host"),
        "[a-host]:is(.foo) {}",
    );
    assert_equal_css(
        &shim(":is(.foo) {}", "contenta", "a-host"),
        ":is(.foo[contenta]) {}",
    );
    assert_equal_css(
        &shim(":is(.foo, .bar, .baz) {}", "contenta", "a-host"),
        ":is(.foo[contenta], .bar[contenta], .baz[contenta]) {}",
    );
    assert_equal_css(
        &shim(":is(.foo, .bar) :host {}", "contenta", "a-host"),
        ":is(.foo, .bar) [a-host] {}",
    );

    // :is() and :where()
    assert_equal_css(
        &shim(
            ":is(.foo, .bar) :is(.baz) :where(.one, .two) :host :where(.three:first-child) {}",
            "contenta",
            "a-host",
        ),
        ":is(.foo, .bar) :is(.baz) :where(.one, .two) [a-host] :where(.three[contenta]:first-child) {}",
    );
    assert_equal_css(
        &shim(":where(:is(a)) {}", "contenta", "hosta"),
        ":where(:is(a[contenta])) {}",
    );
    assert_equal_css(
        &shim(":where(:is(a, b)) {}", "contenta", "hosta"),
        ":where(:is(a[contenta], b[contenta])) {}",
    );
    assert_equal_css(
        &shim(":where(:host:is(.one, .two)) {}", "contenta", "hosta"),
        ":where([hosta]:is(.one, .two)) {}",
    );
    assert_equal_css(
        &shim(":where(:host :is(.one, .two)) {}", "contenta", "hosta"),
        ":where([hosta] :is(.one[contenta], .two[contenta])) {}",
    );
    assert_equal_css(
        &shim(":where(:is(a, b) :is(.one, .two)) {}", "contenta", "hosta"),
        ":where(:is(a[contenta], b[contenta]) :is(.one[contenta], .two[contenta])) {}",
    );
    assert_equal_css(
        &shim(
            ":where(:where(a:has(.foo), b) :is(.one, .two:where(.foo > .bar))) {}",
            "contenta",
            "hosta",
        ),
        ":where(:where(a[contenta]:has(.foo), b[contenta]) :is(.one[contenta], .two[contenta]:where(.foo > .bar))) {}",
    );
    assert_equal_css(
        &shim(":where(.two):first-child {}", "contenta", "hosta"),
        "[contenta]:where(.two):first-child {}",
    );
    assert_equal_css(
        &shim(":first-child:where(.two) {}", "contenta", "hosta"),
        "[contenta]:first-child:where(.two) {}",
    );
    assert_equal_css(
        &shim(":where(.two):nth-child(3) {}", "contenta", "hosta"),
        "[contenta]:where(.two):nth-child(3) {}",
    );
    assert_equal_css(
        &shim("table :where(td, th):hover { color: lime; }", "contenta", "hosta"),
        "table[contenta] [contenta]:where(td, th):hover { color:lime;}",
    );

    // :nth
    assert_equal_css(
        &shim(":nth-child(3n of :not(p, a), :is(.foo)) {}", "contenta", "hosta"),
        "[contenta]:nth-child(3n of :not(p, a), :is(.foo)) {}",
    );
    assert_equal_css(
        &shim("li:nth-last-child(-n + 3) {}", "contenta", "a-host"),
        "li[contenta]:nth-last-child(-n + 3) {}",
    );
    assert_equal_css(
        &shim("dd:nth-last-of-type(3n) {}", "contenta", "a-host"),
        "dd[contenta]:nth-last-of-type(3n) {}",
    );
    assert_equal_css(
        &shim("dd:nth-of-type(even) {}", "contenta", "a-host"),
        "dd[contenta]:nth-of-type(even) {}",
    );

    // complex selectors
    assert_equal_css(
        &shim(":host:is([foo],[foo-2])>div.example-2 {}", "contenta", "a-host"),
        "[a-host]:is([foo],[foo-2]) > div.example-2[contenta] {}",
    );
    assert_equal_css(
        &shim(":host:is([foo], [foo-2]) > div.example-2 {}", "contenta", "a-host"),
        "[a-host]:is([foo], [foo-2]) > div.example-2[contenta] {}",
    );
    assert_equal_css(
        &shim(":host:has([foo],[foo-2])>div.example-2 {}", "contenta", "a-host"),
        "[a-host]:has([foo],[foo-2]) > div.example-2[contenta] {}",
    );

    // :has()
    assert_equal_css(
        &shim("div:has(a) {}", "contenta", "hosta"),
        "div[contenta]:has(a) {}",
    );
    assert_equal_css(
        &shim("div:has(a) :host {}", "contenta", "hosta"),
        "div:has(a) [hosta] {}",
    );
    assert_equal_css(
        &shim(":has(a) :host :has(b) {}", "contenta", "hosta"),
        ":has(a) [hosta] [contenta]:has(b) {}",
    );
    assert_equal_css(
        &shim("div:has(~ .one) {}", "contenta", "hosta"),
        "div[contenta]:has(~ .one) {}",
    );
    assert_equal_css(
        &shim(":has(a) :has(b) {}", "contenta", "hosta"),
        "[contenta]:has(a) [contenta]:has(b) {}",
    );
    assert_equal_css(
        &shim(":has(a, b) {}", "contenta", "hosta"),
        "[contenta]:has(a, b) {}",
    );
    assert_equal_css(
        &shim(":has(a, b:where(.foo), :is(.bar)) {}", "contenta", "hosta"),
        "[contenta]:has(a, b:where(.foo), :is(.bar)) {}",
    );
    assert_equal_css(
        &shim(":has(a, b:where(.foo), :is(.bar):first-child):first-letter {}", "contenta", "hosta"),
        "[contenta]:has(a, b:where(.foo), :is(.bar):first-child):first-letter {}",
    );
    assert_equal_css(
        &shim(":where(a, b:where(.foo), :has(.bar):first-child) {}", "contenta", "hosta"),
        ":where(a[contenta], b[contenta]:where(.foo), [contenta]:has(.bar):first-child) {}",
    );
    assert_equal_css(
        &shim(":has(.one :host, .two) {}", "contenta", "hosta"),
        "[contenta]:has(.one [hosta], .two) {}",
    );
    assert_equal_css(
        &shim(":has(.one, :host) {}", "contenta", "hosta"),
        "[contenta]:has(.one, [hosta]) {}",
    );
}

#[test]
fn should_handle_host_inclusions_inside_pseudo_selectors_selectors() {
    assert_equal_css(
        &shim(".header:not(.admin) {}", "contenta", "hosta"),
        ".header[contenta]:not(.admin) {}",
    );
    assert_equal_css(
        &shim(".header:is(:host > .toolbar, :host ~ .panel) {}", "contenta", "hosta"),
        ".header[contenta]:is([hosta] > .toolbar, [hosta] ~ .panel) {}",
    );
    assert_equal_css(
        &shim(".header:where(:host > .toolbar, :host ~ .panel) {}", "contenta", "hosta"),
        ".header[contenta]:where([hosta] > .toolbar, [hosta] ~ .panel) {}",
    );
    assert_equal_css(
        &shim(".header:not(.admin, :host.super .header) {}", "contenta", "hosta"),
        ".header[contenta]:not(.admin, .super[hosta] .header) {}",
    );
    assert_equal_css(
        &shim(".header:not(.admin, :host.super .header, :host.mega .header) {}", "contenta", "hosta"),
        ".header[contenta]:not(.admin, .super[hosta] .header, .mega[hosta] .header) {}",
    );
    assert_equal_css(
        &shim(".one :where(.two, :host) {}", "contenta", "hosta"),
        ".one :where(.two[contenta], [hosta]) {}",
    );
    assert_equal_css(
        &shim(".one :where(:host, .two) {}", "contenta", "hosta"),
        ".one :where([hosta], .two[contenta]) {}",
    );
    assert_equal_css(
        &shim(":is(.foo):is(:host):is(.two) {}", "contenta", "hosta"),
        ":is(.foo):is([hosta]):is(.two[contenta]) {}",
    );
    assert_equal_css(
        &shim(":where(.one, :host .two):first-letter {}", "contenta", "hosta"),
        "[contenta]:where(.one, [hosta] .two):first-letter {}",
    );
    assert_equal_css(
        &shim(":first-child:where(.one, :host .two) {}", "contenta", "hosta"),
        "[contenta]:first-child:where(.one, [hosta] .two) {}",
    );
    assert_equal_css(
        &shim(":where(.one, :host .two):nth-child(3):is(.foo, a:where(.bar)) {}", "contenta", "hosta"),
        "[contenta]:where(.one, [hosta] .two):nth-child(3):is(.foo, a:where(.bar)) {}",
    );
}

#[test]
fn should_handle_escaped_selector_with_space_if_followed_by_a_hex_char() {
    assert_equal_css(&shim(".\\fc ber {}", "contenta", ""), ".\\fc ber[contenta] {}");
    assert_equal_css(&shim(".\\fc ker {}", "contenta", ""), ".\\fc[contenta]   ker[contenta] {}");
    assert_equal_css(&shim(".pr\\fc fung {}", "contenta", ""), ".pr\\fc fung[contenta] {}");
}

#[test]
fn should_handle_shadow() {
    assert_equal_css(
        &shim("x::shadow > y {}", "contenta", ""),
        "x[contenta] > y[contenta] {}",
    );
}

#[test]
fn should_leave_calc_unchanged() {
    let style_str = "div {height:calc(100% - 55px);}";
    assert_equal_css(
        &shim(style_str, "contenta", ""),
        "div[contenta] {height:calc(100% - 55px);}",
    );
}

#[test]
fn should_shim_rules_with_quoted_content() {
    let style_str = "div {background-image: url(\"a.jpg\"); color: red;}";
    assert_equal_css(
        &shim(style_str, "contenta", ""),
        "div[contenta] {background-image:url(\"a.jpg\"); color:red;}",
    );
}

#[test]
fn should_handle_when_quoted_content_contains_a_closing_parenthesis() {
    assert_equal_css(
        &shim("p { background-image: url(\")\") } p { color: red }", "contenta", ""),
        "p[contenta] { background-image: url(\")\") } p[contenta] { color: red }",
    );
}

#[test]
fn should_shim_rules_with_an_escaped_quote_inside_quoted_content() {
    let style_str = "div::after { content: \"\\\"\" }";
    assert_equal_css(
        &shim(style_str, "contenta", ""),
        "div[contenta]::after { content:\"\\\"\"}",
    );
}

#[test]
fn should_shim_rules_with_curly_braces_inside_quoted_content() {
    let style_str = "div::after { content: \"{}\" }";
    assert_equal_css(
        &shim(style_str, "contenta", ""),
        "div[contenta]::after { content:\"{}\"}",
    );
}

#[test]
fn should_keep_retain_multiline_selectors() {
    let style_str = ".foo,\n.bar { color: red;}";
    assert_eq!(
        shim(style_str, "contenta", ""),
        ".foo[contenta], \n.bar[contenta] { color: red;}"
    );
}

// Comments tests
#[test]
fn should_replace_multiline_comments_with_newline() {
    assert_eq!(shim("/* b {c} */ b {c}", "contenta", ""), "\n b[contenta] {c}");
}

#[test]
fn should_replace_multiline_comments_with_newline_in_the_original_position() {
    assert_eq!(shim("/* b {c}\n */ b {c}", "contenta", ""), "\n\n b[contenta] {c}");
}

#[test]
fn should_replace_comments_with_newline_in_the_original_position() {
    assert_eq!(
        shim("/* b {c} */ b {c} /* a {c} */ a {c}", "contenta", ""),
        "\n b[contenta] {c} \n a[contenta] {c}"
    );
}

#[test]
fn should_keep_source_mapping_url_comments() {
    assert_eq!(
        shim("b {c} /*# sourceMappingURL=data:x */", "contenta", ""),
        "b[contenta] {c} /*# sourceMappingURL=data:x */"
    );
    assert_eq!(
        shim("b {c}/* #sourceMappingURL=data:x */", "contenta", ""),
        "b[contenta] {c}/* #sourceMappingURL=data:x */"
    );
}

#[test]
fn should_handle_adjacent_comments() {
    assert_eq!(
        shim("/* comment 1 */ /* comment 2 */ b {c}", "contenta", ""),
        "\n \n b[contenta] {c}"
    );
}

