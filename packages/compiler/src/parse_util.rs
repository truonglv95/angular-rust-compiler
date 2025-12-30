//! Parse Utilities
//!
//! Corresponds to packages/compiler/src/parse_util.ts (241 lines)

use crate::chars;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseSourceFile {
    pub content: String,
    pub url: String,
}

impl ParseSourceFile {
    pub fn new(content: String, url: String) -> Self {
        ParseSourceFile { content, url }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseLocation {
    pub file: Arc<ParseSourceFile>,
    pub offset: usize,
    pub line: usize,
    pub col: usize,
}

impl ParseLocation {
    pub fn new(file: Arc<ParseSourceFile>, offset: usize, line: usize, col: usize) -> Self {
        ParseLocation {
            file,
            offset,
            line,
            col,
        }
    }

    /// Helper to create ParseLocation from raw source content and url
    /// This wraps the file in Arc automatically for convenience
    pub fn from_source(
        content: String,
        url: String,
        offset: usize,
        line: usize,
        col: usize,
    ) -> Self {
        ParseLocation {
            file: Arc::new(ParseSourceFile::new(content, url)),
            offset,
            line,
            col,
        }
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

        ParseLocation::new(Arc::clone(&self.file), offset, line, col)
    }

    /// Return the source around the location
    /// Up to `max_chars` or `max_lines` on each side of the location
    pub fn get_context(&self, max_chars: usize, max_lines: usize) -> Option<(String, String)> {
        let content = &self.file.content;
        let mut start_offset = self.offset;

        if start_offset > content.len().saturating_sub(1) {
            start_offset = content.len().saturating_sub(1);
        }

        let mut end_offset = start_offset;
        let mut ctx_chars = 0;
        let mut ctx_lines = 0;

        // Move backward
        while ctx_chars < max_chars && start_offset > 0 {
            start_offset -= 1;
            ctx_chars += 1;
            if content.chars().nth(start_offset) == Some('\n') {
                ctx_lines += 1;
                if ctx_lines >= max_lines {
                    break;
                }
            }
        }

        // Move forward
        ctx_chars = 0;
        ctx_lines = 0;
        while ctx_chars < max_chars && end_offset < content.len().saturating_sub(1) {
            end_offset += 1;
            ctx_chars += 1;
            if content.chars().nth(end_offset) == Some('\n') {
                ctx_lines += 1;
                if ctx_lines >= max_lines {
                    break;
                }
            }
        }

        let before = content[start_offset..self.offset].to_string();
        let after = content[self.offset..=end_offset].to_string();
        Some((before, after))
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
        ParseSourceSpan {
            start,
            end,
            details: None,
        }
    }

    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    pub fn to_string(&self) -> String {
        self.start.file.content[self.start.offset..self.end.offset].to_string()
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

    pub fn contextual_message(&self) -> String {
        if let Some((before, after)) = self.span.start.get_context(100, 3) {
            let level_str = match self.level {
                ParseErrorLevel::Warning => "WARNING",
                ParseErrorLevel::Error => "ERROR",
            };
            format!("{} (\"{}[{} ->]{}\")", self.msg, before, level_str, after)
        } else {
            self.msg.clone()
        }
    }

    pub fn to_string(&self) -> String {
        let details = self
            .span
            .details
            .as_ref()
            .map(|d| format!(", {}", d))
            .unwrap_or_default();
        format!(
            "{}: {}{}",
            self.contextual_message(),
            self.span.start.to_string(),
            details
        )
    }
}

/// Compile identifier metadata
#[derive(Debug, Clone)]
pub struct CompileIdentifierMetadata {
    pub reference: serde_json::Value,
}

/// Generate source span object for a given R3 Type for JIT mode
pub fn r3_jit_type_source_span(kind: &str, type_name: &str, source_url: &str) -> ParseSourceSpan {
    let source_file_name = format!("in {} {} in {}", kind, type_name, source_url);
    let source_file = Arc::new(ParseSourceFile::new(String::new(), source_file_name));
    ParseSourceSpan::new(
        ParseLocation::new(Arc::clone(&source_file), usize::MAX, usize::MAX, usize::MAX),
        ParseLocation::new(source_file, usize::MAX, usize::MAX, usize::MAX),
    )
}

static mut ANONYMOUS_TYPE_INDEX: usize = 0;

/// Get identifier name from compile identifier metadata
pub fn identifier_name(compile_identifier: Option<&CompileIdentifierMetadata>) -> Option<String> {
    let metadata = compile_identifier?;

    // In TypeScript, this checks for reference property
    // For Rust, we'll use serde_json::Value and check for string representation
    let ref_value = &metadata.reference;

    // Check for __anonymousType
    if let Some(anon_type) = ref_value.get("__anonymousType") {
        if let Some(s) = anon_type.as_str() {
            return Some(s.to_string());
        }
    }

    // Check for __forward_ref__
    if ref_value.get("__forward_ref__").is_some() {
        return Some("__forward_ref__".to_string());
    }

    // Try to stringify the reference
    let identifier =
        serde_json::to_string(ref_value).unwrap_or_else(|_| format!("{:?}", ref_value));

    // Check if it contains '(' (anonymous functions)
    if identifier.contains('(') {
        unsafe {
            let idx = ANONYMOUS_TYPE_INDEX;
            ANONYMOUS_TYPE_INDEX += 1;
            let result = format!("anonymous_{}", idx);
            // Note: In TypeScript, this sets ref['__anonymousType'] = result
            // In Rust, we can't mutate the Value easily, so we just return the result
            return Some(result);
        }
    } else {
        Some(sanitize_identifier(&identifier))
    }
}

/// Sanitize identifier by replacing non-word characters with underscores
pub fn sanitize_identifier(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
