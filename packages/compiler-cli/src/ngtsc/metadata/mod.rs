//! Angular metadata reader and types.
//!
//! This module provides types and utilities for reading Angular decorator metadata
//! from TypeScript/JavaScript source files.
//!
//! The structure mirrors the TypeScript implementation at:
//! angular/packages/compiler-cli/src/ngtsc/metadata/src/

// Re-export the src submodule
pub mod src;

// Re-export all public types from src for convenient access
pub use src::api::{
    MetaKind, MatchSource,
    DirectiveMeta, PipeMeta, InjectableMeta, NgModuleMeta,
    DecoratorMetadata, DirectiveMetadata,
    HostDirectiveMeta, DirectiveTypeCheckMeta, TemplateGuardMeta,
};
pub use src::property_mapping::{ClassPropertyMapping, ClassPropertyName, InputOrOutput};
pub use src::registry::{MetadataReader, OxcMetadataReader};
pub use src::util::{
    extract_directive_metadata, extract_pipe_metadata, extract_injectable_metadata,
    get_all_metadata,
};

// Implement MetadataReader for OxcMetadataReader
use oxc_ast::ast::Program;
use std::path::Path;

impl MetadataReader for OxcMetadataReader {
    fn get_directive_metadata(&self, program: &Program, path: &Path) -> Vec<DecoratorMetadata> {
        get_all_metadata(program, path)
    }
}

// Keep backward compatibility module for tests
#[cfg(test)]
mod selector_test;
