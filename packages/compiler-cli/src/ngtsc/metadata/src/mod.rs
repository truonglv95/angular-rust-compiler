//! Metadata source modules.
//!
//! This submodule contains the core metadata types and utilities
//! matching the TypeScript structure from angular/packages/compiler-cli/src/ngtsc/metadata/src/

pub mod api;
pub mod property_mapping;
pub mod registry;
pub mod util;

// Re-export commonly used types
pub use api::*;
pub use property_mapping::{ClassPropertyMapping, ClassPropertyName, InputOrOutput};
pub use registry::{MetadataReader, OxcMetadataReader};
pub use util::{extract_directive_metadata, extract_pipe_metadata, extract_injectable_metadata};
