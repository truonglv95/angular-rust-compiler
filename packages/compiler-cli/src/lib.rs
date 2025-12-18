#![deny(clippy::all)]

/**
 * Angular Compiler CLI - Rust Implementation
 *
 * CLI tools and utilities for Angular compilation
 */

// Re-export compiler for convenience
// Note: We'll selectively re-export what's needed rather than using *
// to avoid conflicts and make dependencies explicit
pub use angular_compiler as compiler;

// CLI-specific modules
pub mod ngtsc;
pub mod perform_compile;
pub mod transformers;
// For now, this is a thin wrapper around the compiler

/// CLI version
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

