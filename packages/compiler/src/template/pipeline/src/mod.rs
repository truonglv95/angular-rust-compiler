//! Pipeline Source Module
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/

pub mod compilation;
pub mod conversion;
pub mod emit;
pub mod ingest;
pub mod ingest_helpers;
mod ingest_test;
pub mod instruction;
pub mod phases;
pub mod util;

pub use compilation::*;
pub use conversion::*;
// pub use emit::*; // Unused for now
pub use ingest::*;
// pub use instruction::*; // Unused for now
