// JIT Transform Module
//
// This module provides JIT compilation transforms for Angular applications.
// Used by the Angular CLI for unit tests and explicit JIT applications.
//
// The transforms include:
// - Downlevel decorators transform (ES2015 TDZ workaround)
// - Initializer API transforms (signal inputs/outputs/queries)

pub mod downlevel_decorators_transform;
pub mod initializer_api_transforms;

#[cfg(test)]
mod test;

pub use downlevel_decorators_transform::*;
pub use initializer_api_transforms::get_initializer_api_jit_transform;

use crate::ngtsc::imports::ImportedSymbolsTracker;
use ts::Diagnostic;

/// JIT transform for Angular applications. Used by the Angular CLI for unit tests and
/// explicit JIT applications.
///
/// The transforms include:
///
///  - A transform for downleveling Angular decorators and Angular-decorated class constructor
///    parameters for dependency injection. This transform can be used by the CLI for JIT-mode
///    compilation where constructor parameters and associated Angular decorators should be
///    downleveled so that apps are not exposed to the ES2015 temporal dead zone limitation
///    in TypeScript.
///
///  - A transform for adding `@Input` to signal inputs. Signal inputs cannot be recognized
///    at runtime using reflection. That is because the class would need to be instantiated-
///    but is not possible before creation. To fix this for JIT, a decorator is automatically
///    added that will declare the input as a signal input while also capturing the necessary
///    metadata.
pub struct AngularJitApplicationTransform {
    /// Whether this is the Angular core package.
    is_core: bool,
    /// Import tracker for efficient import checking.
    import_tracker: ImportedSymbolsTracker,
    /// Collected diagnostics.
    diagnostics: Vec<Diagnostic>,
}

impl AngularJitApplicationTransform {
    /// Create a new JIT application transform.
    pub fn new(is_core: bool) -> Self {
        Self {
            is_core,
            import_tracker: ImportedSymbolsTracker::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Get collected diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Transform a source file for JIT compilation.
    ///
    /// This applies both the downlevel decorators transform and the initializer API transform.
    pub fn transform_source_file(&mut self, _source_file_path: &str) {
        // TODO: Implement source file transformation
        // 1. Apply initializer API JIT transform
        // 2. Apply downlevel decorator transform
    }

    /// Get a reference to the import tracker.
    pub fn import_tracker(&self) -> &ImportedSymbolsTracker {
        &self.import_tracker
    }

    /// Whether this is compiling Angular core.
    pub fn is_core(&self) -> bool {
        self.is_core
    }
}

/// Configuration for the JIT application transform.
#[derive(Debug, Clone, Default)]
pub struct JitTransformConfig {
    /// Whether this is the Angular core package.
    pub is_core: bool,
    /// Whether to enable Closure Compiler annotations.
    pub enable_closure_compiler: bool,
}

impl JitTransformConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_core(mut self, is_core: bool) -> Self {
        self.is_core = is_core;
        self
    }

    pub fn enable_closure_compiler(mut self, enable: bool) -> Self {
        self.enable_closure_compiler = enable;
        self
    }
}
