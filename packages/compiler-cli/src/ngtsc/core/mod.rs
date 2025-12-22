//! Core Types for ngtsc
//!
//! Corresponds to packages/compiler-cli/src/ngtsc/core

use std::path::PathBuf;

/// Compiler options (subset of tsconfig)

pub mod compiler;
pub mod ast_transformer;
#[cfg(test)]
mod compiler_test;

pub use compiler::{NgCompiler, CompilationTicket, CompilationTicketKind, CompilationResult};

#[derive(Debug, Clone, Default)]
pub struct NgCompilerOptions {
    pub project: String,
    // Add other options as needed
    pub strict_templates: bool,
    pub strict_injection_parameters: bool,
    pub skip_template_codegen: bool,
    pub flat_module_out_file: Option<String>,
    pub out_dir: Option<String>,
}

/// Compilation diagnostics
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub file: Option<PathBuf>,
    pub message: String,
    pub code: usize,
    // Start/length for source mapping would go here
}
