
//! TypeScript-compatible interfaces and types for the Angular compiler.
//! This crate serves as a shared compatibility layer.

use std::fmt;

pub mod node;
pub mod program;
pub mod type_checker;

pub use node::*;
pub use program::*;
pub use type_checker::*;

// --- Enums ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptTarget {
    ES3,
    ES5,
    ES2015,
    ES2016,
    ES2017,
    ES2018,
    ES2019,
    ES2020,
    ES2021,
    ES2022,
    ESNext,
    JSON,
    Latest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleKind {
    None,
    CommonJS,
    AMD,
    UMD,
    System,
    ES2015,
    ES2020,
    ES2022,
    ESNext,
    Node16,
    NodeNext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCategory {
    Warning,
    Error,
    Suggestion,
    Message,
}

// --- Diagnostic Structures ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticMessageChain {
    String(String),
    Chain {
        message_text: String,
        category: DiagnosticCategory,
        code: i32,
        next: Option<Vec<DiagnosticMessageChain>>,
    }
}

impl DiagnosticMessageChain {
     pub fn new(message: impl Into<String>) -> Self {
        Self::String(message.into())
    }
}

impl From<String> for DiagnosticMessageChain {
    fn from(s: String) -> Self {
        DiagnosticMessageChain::String(s)
    }
}

impl From<&str> for DiagnosticMessageChain {
    fn from(s: &str) -> Self {
        DiagnosticMessageChain::String(s.to_string())
    }
}

impl fmt::Display for DiagnosticMessageChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticMessageChain::String(s) => write!(f, "{}", s),
            DiagnosticMessageChain::Chain { message_text, .. } => write!(f, "{}", message_text),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticRelatedInformation {
    pub category: DiagnosticCategory,
    pub code: i32,
    pub file: Option<String>, // Placeholder for SourceFile
    pub start: Option<usize>,
    pub length: Option<usize>,
    pub message_text: String,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub category: DiagnosticCategory,
    pub code: i32,
    pub file: Option<String>, // Placeholder for SourceFile
    pub start: usize,
    pub length: usize,
    pub message_text: DiagnosticMessageChain,
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticWithLocation {
    pub category: DiagnosticCategory,
    pub code: i32,
    pub file: Option<String>, // Placeholder for SourceFile
    pub start: usize,
    pub length: usize,
    pub message_text: DiagnosticMessageChain,
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
}

// --- Node / SourceFile Traits ---

// Moved to node.rs

// --- Utilities ---

pub fn make_diagnostic_chain(
    message_text: String,
    next: Option<Vec<DiagnosticMessageChain>>,
) -> DiagnosticMessageChain {
    DiagnosticMessageChain::Chain {
        category: DiagnosticCategory::Message,
        code: 0,
        message_text,
        next
    }
}

pub fn add_diagnostic_chain(
    message_text: DiagnosticMessageChain,
    add: Vec<DiagnosticMessageChain>,
) -> DiagnosticMessageChain {
    match message_text {
        DiagnosticMessageChain::String(s) => make_diagnostic_chain(s, Some(add)),
         DiagnosticMessageChain::Chain { message_text, category, code, next } => {
            let mut next_vec = next.unwrap_or_default();
            next_vec.extend(add);
            DiagnosticMessageChain::Chain {
                message_text,
                category,
                code,
                next: Some(next_vec),
            }
        }
    }
}
