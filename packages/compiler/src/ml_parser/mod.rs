//! ML (Markup Language) Parser Module
//!
//! Corresponds to packages/compiler/src/ml_parser/
//! Handles HTML/XML parsing

pub mod tags;
pub mod defaults;
pub mod xml_tags;
pub mod html_tags;
pub mod tokens;
pub mod ast;
pub mod html_parser;
pub mod xml_parser;
pub mod lexer;
pub mod parser;
pub mod entities;

pub use tags::*;
pub use defaults::*;
pub use xml_tags::*;
pub use html_tags::*;
pub use tokens::*;
pub use ast::*;
pub use parser::{Parser, ParseTreeResult, TreeError, ParseOptions};
pub use lexer::{tokenize, TokenizeOptions};
