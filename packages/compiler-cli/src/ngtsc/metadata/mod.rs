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
    DecoratorMetadata, DirectiveMetadata, OwnedDirectiveMeta,
    HostDirectiveMeta, DirectiveTypeCheckMeta, TemplateGuardMeta,
    // Reference types
    Reference, OwningModule, BaseClass,
};
pub use src::property_mapping::{ClassPropertyMapping, ClassPropertyName, InputOrOutput};
pub use src::registry::{MetadataReader, OxcMetadataReader};
pub use src::util::{
    extract_directive_metadata, extract_pipe_metadata, extract_injectable_metadata,
    get_all_metadata,
};

// Implement MetadataReader for OxcMetadataReader
// Note: The lifetime is tied to the Program's allocator
use oxc_ast::ast::Program;
use std::path::Path;

impl<'a> OxcMetadataReader {
    /// Get directive metadata with lifetime tied to the program's AST.
    pub fn get_directive_metadata_with_lifetime(
        &self, 
        program: &'a Program<'a>, 
        path: &Path
    ) -> Vec<DecoratorMetadata<'a>> {
        get_all_metadata(program, path)
    }
}

// For backward compatibility, implement MetadataReader trait with static lifetime
// This requires that the caller ensures the data outlives the metadata
impl MetadataReader for OxcMetadataReader {
    fn get_directive_metadata(&self, program: &Program, path: &Path) -> Vec<DecoratorMetadata<'static>> {
        // Safety: This is for backward compat - caller must ensure program lives long enough
        // We clear all lifetime-bound references to avoid dangling pointers
        let metadata = get_all_metadata(program, path);
        metadata.into_iter().map(|m| {
            match m {
                DecoratorMetadata::Directive(mut d) => {
                    // Clear all lifetime-bound references
                    d.decorator = None;
                    DecoratorMetadata::Directive(unsafe { std::mem::transmute(d) })
                }
                DecoratorMetadata::Pipe(p) => DecoratorMetadata::Pipe(p),
                DecoratorMetadata::Injectable(i) => DecoratorMetadata::Injectable(i),
                DecoratorMetadata::NgModule(n) => DecoratorMetadata::NgModule(n),
            }
        }).collect()
    }
}

// Keep backward compatibility module for tests
#[cfg(test)]
mod selector_test;
