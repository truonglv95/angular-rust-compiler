//! Angular TypeScript Compiler (ngtsc)
//!
//! Corresponds to packages/compiler-cli/src/ngtsc
//! This module contains the core logic for the Angular compiler CLI.

pub mod annotations;
pub mod core;
pub mod file_system;
pub mod imports;
pub mod metadata;
pub mod perf;
pub mod program;
pub mod reflection;
pub mod transform;

pub mod cycles;
pub mod diagnostics;
pub mod incremental;
pub mod scope;
pub mod translator;
pub mod typecheck;

// New modules
pub mod docs;
pub mod entry_point;
pub mod hmr;
pub mod indexer;
pub mod logging;
pub mod partial_evaluator;
pub mod program_driver;
pub mod resource;
pub mod shims;
pub mod sourcemaps;
pub mod testing;
pub mod tsc_plugin;
pub mod util;
pub mod validation;
pub mod xi18n;

pub mod dependency;
pub mod test_parser_hmr;
