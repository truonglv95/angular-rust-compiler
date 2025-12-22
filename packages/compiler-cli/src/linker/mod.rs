//! Angular Linker
//!
//! The linker is responsible for linking partial declarations (components, directives, pipes, etc.)
//! into full definitions. It is used during the build process to transform library code ensuring
//! compatibility with the application's Angular version.

pub mod ast;
pub mod ast_value;
pub mod error;
pub mod file_linker;
pub mod oxc_ast_host;
pub mod partial_linker;
pub mod partial_linkers;
pub mod napi;
// pub mod file_linker; // To be implemented
