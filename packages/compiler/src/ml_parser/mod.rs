//! ML (Markup Language) Parser Module
//!
//! Corresponds to packages/compiler/src/ml_parser/
//! Handles HTML/XML parsing

pub mod ast;
pub mod defaults;
pub mod entities;
pub mod html_parser;
pub mod html_tags;
pub mod html_whitespaces;
pub mod lexer;
pub mod parser;
pub mod tags;
pub mod tokens;
pub mod xml_parser;
pub mod xml_tags;

pub use ast::*;
pub use defaults::*;
pub use html_tags::*;
pub use html_whitespaces::{
    process_whitespace, remove_whitespaces, replace_ngsp, WhitespaceVisitor, PRESERVE_WS_ATTR_NAME,
};
pub use lexer::{tokenize, TokenizeOptions};
pub use parser::{ParseOptions, ParseTreeResult, Parser, TreeError};
pub use tags::*;
pub use tokens::*;
pub use xml_tags::*;
