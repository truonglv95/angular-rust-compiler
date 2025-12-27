/**
 * Test Utilities
 *
 * Helper functions for expression parser tests
 * Mirrors angular/packages/compiler/test/expression_parser/utils
 */
pub mod unparser;

use angular_compiler::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};

/// Gets a fake `ParseSourceSpan` for testing purposes
#[allow(dead_code)]
pub fn get_fake_span(file_name: &str) -> ParseSourceSpan {
    let file = ParseSourceFile::new(String::new(), file_name.to_string());
    let location = ParseLocation::new(file, 0, 0, 0);
    ParseSourceSpan::new(location.clone(), location)
}
