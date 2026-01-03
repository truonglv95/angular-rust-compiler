//! Render3 View Module
//!
//! Corresponds to packages/compiler/src/render3/view/
//! Contains view compilation utilities and APIs

pub mod api;
pub mod compiler;
pub mod config;
pub mod i18n;
pub mod query_generation;
pub mod t2_api;
pub mod t2_binder;
pub mod template;
pub mod util;

#[cfg(test)]
pub mod compiler_spec;

// Re-exports
pub use api::*;
pub use compiler::*;
pub use config::*;
pub use i18n::*;
pub use query_generation::*;
pub use t2_api::*;
pub use t2_binder::*;
pub use template::*;
pub use util::*;
