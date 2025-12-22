//! Metadata registry for Angular decorators.
//!
//! This module provides traits and implementations for reading metadata from TypeScript/JavaScript AST.
//! Matches TypeScript's registry.ts

use oxc_ast::ast::Program;
use std::path::Path;
use super::api::DecoratorMetadata;

/// Trait for reading Angular decorator metadata from source files.
/// This is the primary interface for metadata extraction.
pub trait MetadataReader {
    /// Extract all Angular decorator metadata from a program AST.
    fn get_directive_metadata(&self, program: &Program, path: &Path) -> Vec<DecoratorMetadata>;
}

/// OXC-based implementation of MetadataReader.
/// Uses the OXC parser to analyze TypeScript/JavaScript files.
pub struct OxcMetadataReader;

impl OxcMetadataReader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OxcMetadataReader {
    fn default() -> Self {
        Self::new()
    }
}
