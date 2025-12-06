//! Parse Utilities
//!
//! Corresponds to packages/compiler/src/parse_util.ts (241 lines)

use serde::{Deserialize, Serialize};
use crate::chars;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseSourceFile {
    pub content: String,
    pub url: String,
}

impl ParseSourceFile {
    pub fn new(content: String, url: String) -> Self {
        ParseSourceFile { content, url }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseLocation {
    pub file: ParseSourceFile,
    pub offset: usize,
    pub line: usize,
    pub col: usize,
}

impl ParseLocation {
    pub fn new(file: ParseSourceFile, offset: usize, line: usize, col: usize) -> Self {
        ParseLocation { file, offset, line, col }
    }

    pub fn to_string(&self) -> String {
        format!("{}@{}:{}", self.file.url, self.line, self.col)
    }

    pub fn move_by(&self, delta: i32) -> ParseLocation {
        let source = &self.file.content;
        let len = source.len();
        let mut offset = self.offset;
        let mut line = self.line;
        let mut col = self.col;
        let mut delta = delta;

        // Move backward
        while offset > 0 && delta < 0 {
            offset -= 1;
            delta += 1;
            let ch = source.as_bytes()[offset];
            if ch == chars::NEWLINE as u8 {
                line -= 1;
                if let Some(prior_line) = source[..offset].rfind('\n') {
                    col = offset - prior_line;
                } else {
                    col = offset;
                }
            } else {
                col -= 1;
            }
        }

        // Move forward
        while offset < len && delta > 0 {
            let ch = source.as_bytes()[offset];
            offset += 1;
            delta -= 1;
            if ch == chars::NEWLINE as u8 {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        ParseLocation::new(self.file.clone(), offset, line, col)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseSourceSpan {
    pub start: ParseLocation,
    pub end: ParseLocation,
    pub details: Option<String>,
}

impl ParseSourceSpan {
    pub fn new(start: ParseLocation, end: ParseLocation) -> Self {
        ParseSourceSpan { start, end, details: None }
    }

    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    pub fn to_string(&self) -> String {
        self.start.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseError {
    pub span: ParseSourceSpan,
    pub msg: String,
    pub level: ParseErrorLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParseErrorLevel {
    Warning,
    Error,
}

impl ParseError {
    pub fn new(span: ParseSourceSpan, msg: String) -> Self {
        ParseError {
            span,
            msg,
            level: ParseErrorLevel::Error,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}: {}", self.span.to_string(), self.msg)
    }
}
