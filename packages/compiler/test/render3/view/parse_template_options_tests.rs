//! Parse Template Options Tests
//!
//! Mirrors angular/packages/compiler/test/render3/view/parse_template_options_spec.ts



// Include test utilities
#[path = "../view/util.rs"]
mod view_util;
use view_util::{parse_r3, ParseR3Options};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_include_an_array_of_html_comment_nodes_on_the_returned_r3_ast() {
        let html = r#"
      <!-- eslint-disable-next-line -->
      <div *ngFor="let item of items">
        {{item.name}}
      </div>

      <div>
        <p>
          <!-- some nested comment -->
          <span>Text</span>
        </p>
      </div>
    "#;

        // Test with default options (collect_comment_nodes should be None/false)
        let template_no_comments_option = parse_r3(html, ParseR3Options::default());
        assert!(template_no_comments_option.comment_nodes.is_none(), 
            "comment_nodes should be None when collect_comment_nodes is not enabled");

        // Test with collect_comment_nodes explicitly disabled
        let template_comments_option_disabled = parse_r3(html, ParseR3Options {
            collect_comment_nodes: Some(false),
            ..Default::default()
        });
        assert!(template_comments_option_disabled.comment_nodes.is_none(),
            "comment_nodes should be None when collect_comment_nodes is false");

        // Test with collect_comment_nodes enabled
        let template_comments_option_enabled = parse_r3(html, ParseR3Options {
            collect_comment_nodes: Some(true),
            ..Default::default()
        });
        
        assert!(template_comments_option_enabled.comment_nodes.is_some(),
            "comment_nodes should be Some when collect_comment_nodes is true");
        
        let comment_nodes = template_comments_option_enabled.comment_nodes.as_ref().unwrap();
        assert_eq!(comment_nodes.len(), 2,
            "Expected 2 comment nodes, got {}", comment_nodes.len());

        // Verify first comment
        assert!(comment_nodes[0].value.contains("eslint-disable-next-line"),
            "First comment should contain 'eslint-disable-next-line', got: {}", comment_nodes[0].value);
        assert_eq!(comment_nodes[0].value.trim(), "eslint-disable-next-line",
            "First comment value should be 'eslint-disable-next-line'");
        assert!(comment_nodes[0].source_span.to_string().contains("<!-- eslint-disable-next-line -->") ||
                comment_nodes[0].source_span.to_string().contains("eslint-disable-next-line"),
            "First comment source span should contain the comment text");

        // Verify second comment
        assert!(comment_nodes[1].value.contains("some nested comment"),
            "Second comment should contain 'some nested comment', got: {}", comment_nodes[1].value);
        assert_eq!(comment_nodes[1].value.trim(), "some nested comment",
            "Second comment value should be 'some nested comment'");
        assert!(comment_nodes[1].source_span.to_string().contains("<!-- some nested comment -->") ||
                comment_nodes[1].source_span.to_string().contains("some nested comment"),
            "Second comment source span should contain the comment text");
    }
}

