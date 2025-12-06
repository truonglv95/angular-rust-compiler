//! HTML Parser
//!
//! Corresponds to packages/compiler/src/ml_parser/html_parser.ts (22 lines)
//!
//! Simple wrapper around Parser with HTML tag definitions

use super::html_tags::get_html_tag_definition;
use super::lexer::TokenizeOptions;
use super::parser::{Parser, ParseTreeResult};
use super::tags::TagDefinition;

/// HTML parser (extends generic Parser with HTML tag definitions)
///
/// TypeScript equivalent:
/// ```typescript
/// export class HtmlParser extends Parser {
///   constructor() {
///     super(getHtmlTagDefinition);
///   }
/// }
/// ```
pub struct HtmlParser {
    // Store tag definition function instead of Parser instance
    // Parser will be created on-demand in parse() method
}

impl HtmlParser {
    /// Create new HTML parser with default HTML tag definitions
    pub fn new() -> Self {
        HtmlParser {}
    }

    /// Parse HTML template source
    ///
    /// # Arguments
    /// * `source` - HTML template string
    /// * `url` - Source file URL/path (for error reporting)
    /// * `options` - Tokenization options (optional)
    ///
    /// # Returns
    /// ParseTreeResult with root nodes and errors
    pub fn parse(&self, source: &str, url: &str, options: Option<TokenizeOptions>) -> ParseTreeResult {
        fn tag_def(name: &str) -> &'static dyn TagDefinition {
            get_html_tag_definition(name)
        }

        let parser = Parser::new(tag_def);
        parser.parse(source, url, options)
    }
}

impl Default for HtmlParser {
    fn default() -> Self {
        Self::new()
    }
}

