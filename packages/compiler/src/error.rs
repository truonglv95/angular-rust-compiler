use thiserror::Error;

#[cfg(feature = "napi-bindings")]
use napi::Error as NapiError;
#[cfg(feature = "napi-bindings")]
use napi::Status;

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("Parse error: {message}")]
    ParseError { message: String },

    #[error("Lexical error: {message}")]
    LexicalError { message: String },

    #[error("Syntax error: {message}")]
    SyntaxError { message: String },

    #[error("Semantic error: {message}")]
    SemanticError { message: String },

    #[error("Code generation error: {message}")]
    CodeGenerationError { message: String },

    #[error("Internal error: {message}")]
    InternalError { message: String },
}

// Implement conversion from CompilerError to NAPI Error
#[cfg(feature = "napi-bindings")]
impl From<CompilerError> for NapiError {
    fn from(err: CompilerError) -> Self {
        NapiError::new(Status::GenericFailure, err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CompilerError>;
