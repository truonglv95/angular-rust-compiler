use super::*;
use crate::ngtsc::diagnostics::{ErrorCode, FatalDiagnosticError, Node, ng_error_code, replace_ts_with_ng_in_errors};

#[derive(Debug)]
struct MockNode {
    start: usize,
    width: usize,
    source_file: Option<String>,
}

impl Node for MockNode {
    fn get_start(&self) -> usize { self.start }
    fn get_width(&self) -> usize { self.width }
    fn get_source_file(&self) -> Option<String> { self.source_file.clone() }
}

#[test]
fn test_error_code_mapping() {
    assert_eq!(ng_error_code(ErrorCode::DecoratorArgNotLiteral), -991001);
    assert_eq!(ng_error_code(ErrorCode::ComponentMissingTemplate), -992001);
}

#[test]
fn test_replace_ts_with_ng() {
    let input = "\u{001b}[31mTS-991001: \u{001b}[0mError message";
    let expected = "\u{001b}[31mNG1001: \u{001b}[0mError message";
    assert_eq!(replace_ts_with_ng_in_errors(input), expected);
}

#[test]
fn test_fatal_diagnostic_error() {
    let node = MockNode { start: 10, width: 20, source_file: Some("test.ts".to_string()) };
    let err = FatalDiagnosticError::new(
        ErrorCode::DecoratorArgNotLiteral,
        Box::new(node),
        "Something went wrong",
        None
    );
    
    let diag = err.to_diagnostic();
    assert_eq!(diag.code, -991001);
    assert_eq!(diag.start, 10);
    assert_eq!(diag.length, 20);
    assert_eq!(diag.file.as_deref(), Some("test.ts"));
    
    let display = format!("{}", err);
    assert!(display.contains("FatalDiagnosticError"));
    assert!(display.contains("Code: DecoratorArgNotLiteral"));
    assert!(display.contains("Message: Something went wrong"));
}
