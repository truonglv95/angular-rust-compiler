//! Angular TypeScript Compiler (ngtsc)
//!
//! Corresponds to packages/compiler-cli/src/ngtsc
//! This module contains the core logic for the Angular compiler CLI.

pub mod file_system;
pub mod core;
pub mod program;
pub mod metadata;
pub mod reflection;
pub mod imports;
pub mod annotations;
pub mod transform;
pub mod perf;

pub mod cycles;
pub mod diagnostics;
pub mod translator;
pub mod scope;
pub mod incremental;
pub mod typecheck;

// New modules
pub mod entry_point;
pub mod indexer;
pub mod resource;
pub mod xi18n;
pub mod partial_evaluator;
pub mod shims;
pub mod sourcemaps;
pub mod validation;
pub mod logging;
pub mod util;
pub mod hmr;
pub mod program_driver;
pub mod docs;
pub mod testing;
pub mod tsc_plugin;
