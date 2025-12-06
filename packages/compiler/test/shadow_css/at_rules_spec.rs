//! At Rules Tests
//!
//! Corresponds to packages/compiler/test/shadow_css/at_rules_spec.ts
//! All test cases match exactly with TypeScript version

mod utils;
use utils::{assert_equal_css, shim};

#[test]
fn should_handle_media_rules_with_simple_rules() {
    let css = "@media screen and (max-width: 800px) {div {font-size: 50px;}} div {}";
    let expected = "@media screen and (max-width:800px) {div[contenta] {font-size:50px;}} div[contenta] {}";
    assert_equal_css(&shim(css, "contenta", ""), expected);
}

#[test]
fn should_handle_media_rules_with_both_width_and_height() {
    let css = "@media screen and (max-width:800px, max-height:100%) {div {font-size:50px;}}";
    let expected = "@media screen and (max-width:800px, max-height:100%) {div[contenta] {font-size:50px;}}";
    assert_equal_css(&shim(css, "contenta", ""), expected);
}

#[test]
fn should_preserve_page_rules() {
    let content_attr = "contenta";
    let css = "
        @page {
          margin-right: 4in;

          @top-left {
            content: \"Hamlet\";
          }

          @top-right {
            content: \"Page \" counter(page);
          }
        }

        @page main {
          margin-left: 4in;
        }

        @page :left {
          margin-left: 3cm;
          margin-right: 4cm;
        }

        @page :right {
          margin-left: 4cm;
          margin-right: 3cm;
        }
      ";
    let result = shim(css, content_attr, "");
    assert_equal_css(&result, css);
    assert!(!result.contains(content_attr));
}

#[test]
fn should_strip_ng_deep_and_host_from_within_page_rules() {
    assert_equal_css(
        &shim("@page { margin-right: 4in; }", "contenta", "h"),
        "@page { margin-right:4in;}",
    );
    assert_equal_css(
        &shim("@page { ::ng-deep @top-left { content: \"Hamlet\";}}", "contenta", "h"),
        "@page { @top-left { content:\"Hamlet\";}}",
    );
    assert_equal_css(
        &shim("@page { :host ::ng-deep @top-left { content:\"Hamlet\";}}", "contenta", "h"),
        "@page { @top-left { content:\"Hamlet\";}}",
    );
}

#[test]
fn should_handle_support_rules() {
    let css = "@supports (display: flex) {section {display: flex;}}";
    let expected = "@supports (display:flex) {section[contenta] {display:flex;}}";
    assert_equal_css(&shim(css, "contenta", ""), expected);
}

#[test]
fn should_strip_ng_deep_and_host_from_within_supports() {
    assert_equal_css(
        &shim(
            "@supports (display: flex) { @font-face { :host ::ng-deep font-family{} } }",
            "contenta",
            "h",
        ),
        "@supports (display:flex) { @font-face { font-family{}}}",
    );
}

#[test]
fn should_strip_ng_deep_and_host_from_within_font_face() {
    assert_equal_css(
        &shim("@font-face { font-family {} }", "contenta", "h"),
        "@font-face { font-family {}}",
    );
    assert_equal_css(
        &shim("@font-face { ::ng-deep font-family{} }", "contenta", "h"),
        "@font-face { font-family{}}",
    );
    assert_equal_css(
        &shim("@font-face { :host ::ng-deep font-family{} }", "contenta", "h"),
        "@font-face { font-family{}}",
    );
}

#[test]
fn should_pass_through_import_directives() {
    let style_str = "@import url(\"https://fonts.googleapis.com/css?family=Roboto\");";
    let css = shim(style_str, "contenta", "");
    assert_equal_css(&css, style_str);
}

#[test]
fn should_shim_rules_after_import() {
    let style_str = "@import url(\"a\"); div {}";
    let css = shim(style_str, "contenta", "");
    assert_equal_css(&css, "@import url(\"a\"); div[contenta] {}");
}

#[test]
fn should_shim_rules_with_quoted_content_after_import() {
    let style_str = "@import url(\"a\"); div {background-image: url(\"a.jpg\"); color: red;}";
    let css = shim(style_str, "contenta", "");
    assert_equal_css(
        &css,
        "@import url(\"a\"); div[contenta] {background-image:url(\"a.jpg\"); color:red;}",
    );
}

#[test]
fn should_pass_through_import_directives_whose_url_contains_colons_and_semicolons() {
    let style_str = "@import url(\"https://fonts.googleapis.com/css2?family=Roboto:wght@400;500&display=swap\");";
    let css = shim(style_str, "contenta", "");
    assert_equal_css(&css, style_str);
}

#[test]
fn should_shim_rules_after_import_with_colons_and_semicolons() {
    let style_str = "@import url(\"https://fonts.googleapis.com/css2?family=Roboto:wght@400;500&display=swap\"); div {}";
    let css = shim(style_str, "contenta", "");
    assert_equal_css(
        &css,
        "@import url(\"https://fonts.googleapis.com/css2?family=Roboto:wght@400;500&display=swap\"); div[contenta] {}",
    );
}

#[test]
fn should_scope_normal_selectors_inside_an_unnamed_container_rules() {
    let css = "@container max(max-width: 500px) {
               .item {
                 color: red;
               }
             }";
    let result = shim(css, "host-a", "");
    assert_equal_css(
        &result,
        "@container max(max-width: 500px) {
           .item[host-a] {
             color: red;
           }
         }",
    );
}

#[test]
fn should_scope_normal_selectors_inside_a_named_container_rules() {
    let css = "
          @container container max(max-width: 500px) {
               .item {
                 color: red;
               }
          }";
    let result = shim(css, "host-a", "");
    assert_equal_css(
        &result,
        "@container container max(max-width: 500px) {
          .item[host-a] {
            color: red;
          }
        }",
    );
}

#[test]
fn should_scope_normal_selectors_inside_a_scope_rule_with_scoping_limits() {
    let css = "
          @scope (.media-object) to (.content > *) {
              img { border-radius: 50%; }
              .content { padding: 1em; }
          }";
    let result = shim(css, "host-a", "");
    assert_equal_css(
        &result,
        "@scope (.media-object) to (.content > *) {
          img[host-a] { border-radius: 50%; }
          .content[host-a] { padding: 1em; }
        }",
    );
}

#[test]
fn should_scope_normal_selectors_inside_a_scope_rule() {
    let css = "
          @scope (.light-scheme) {
              a { color: darkmagenta; }
          }";
    let result = shim(css, "host-a", "");
    assert_equal_css(
        &result,
        "@scope (.light-scheme) {
          a[host-a] { color: darkmagenta; }
        }",
    );
}

#[test]
fn should_handle_document_rules() {
    let css = "@document url(http://www.w3.org/) {div {font-size:50px;}}";
    let expected = "@document url(http://www.w3.org/) {div[contenta] {font-size:50px;}}";
    assert_equal_css(&shim(css, "contenta", ""), expected);
}

#[test]
fn should_handle_layer_rules() {
    let css = "@layer utilities {section {display: flex;}}";
    let expected = "@layer utilities {section[contenta] {display:flex;}}";
    assert_equal_css(&shim(css, "contenta", ""), expected);
}

#[test]
fn should_scope_normal_selectors_inside_a_starting_style_rule() {
    let css = "
          @starting-style {
              img { border-radius: 50%; }
              .content { padding: 1em; }
          }";
    let result = shim(css, "host-a", "");
    assert_equal_css(
        &result,
        "@starting-style {
          img[host-a] { border-radius: 50%; }
          .content[host-a] { padding: 1em; }
        }",
    );
}

