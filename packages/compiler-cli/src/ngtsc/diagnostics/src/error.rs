use super::error_code::ErrorCode;
use super::util::ng_error_code;
use std::fmt;

// Mocking TS types for now to maintain structure. 
// In a real integration, these might map to oxc_span types or similar.
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCategory {
    Warning,
    Error,
    Suggestion,
    Message,
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
pub struct DiagnosticWithLocation {
    pub category: DiagnosticCategory,
    pub code: i32,
    pub file: Option<String>, // Placeholder for SourceFile
    pub start: usize,
    pub length: usize,
    pub message_text: DiagnosticMessageChain,
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
}

// Since we don't have full TS Node/SourceFile, we'll store what we need.
// For now, let's keep it generic.
pub trait Node: std::fmt::Debug {
    fn get_start(&self) -> usize;
    fn get_width(&self) -> usize;
    fn get_source_file(&self) -> Option<String>; // Placeholder
}

#[derive(Debug)]
pub struct FatalDiagnosticError {
    pub code: ErrorCode,
    pub node: Box<dyn Node>, 
    pub diagnostic_message: DiagnosticMessageChain,
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
}

impl FatalDiagnosticError {
    pub fn new(
        code: ErrorCode, 
        node: Box<dyn Node>, 
        diagnostic_message: impl Into<DiagnosticMessageChain>, 
        related_information: Option<Vec<DiagnosticRelatedInformation>>
    ) -> Self {
        Self {
            code,
            node,
            diagnostic_message: diagnostic_message.into(),
            related_information,
        }
    }

    pub fn to_diagnostic(&self) -> DiagnosticWithLocation {
        make_diagnostic(self.code, &*self.node, self.diagnostic_message.clone(), self.related_information.clone(), DiagnosticCategory::Error)
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

impl fmt::Display for FatalDiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Approximate flattening logic
        let msg = match &self.diagnostic_message {
            DiagnosticMessageChain::String(s) => s.clone(),
            DiagnosticMessageChain::Chain { message_text, .. } => message_text.clone(),
        };
        write!(f, "FatalDiagnosticError: Code: {:?}, Message: {}", self.code, msg)
    }
}

impl std::error::Error for FatalDiagnosticError {}

pub fn make_diagnostic(
    code: ErrorCode,
    node: &dyn Node,
    message_text: DiagnosticMessageChain,
    related_information: Option<Vec<DiagnosticRelatedInformation>>,
    category: DiagnosticCategory,
) -> DiagnosticWithLocation {
    DiagnosticWithLocation {
        category,
        code: ng_error_code(code),
        file: node.get_source_file(),
        start: node.get_start(),
        length: node.get_width(),
        message_text,
        related_information,
    }
}

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

pub fn make_related_information(
    node: &dyn Node,
    message_text: String,
) -> DiagnosticRelatedInformation {
    DiagnosticRelatedInformation {
        category: DiagnosticCategory::Message,
        code: 0,
        file: node.get_source_file(),
        start: Some(node.get_start()),
        length: Some(node.get_width()),
        message_text,
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

pub fn is_fatal_diagnostic_error(err: &(dyn std::error::Error + 'static)) -> bool {
    err.is::<FatalDiagnosticError>()
}

// Temporary implementation for dyn Error downcasting check
impl FatalDiagnosticError {
    pub fn is_fatal(err: &(dyn std::error::Error + 'static)) -> bool {
        err.downcast_ref::<FatalDiagnosticError>().is_some()
    }
}
