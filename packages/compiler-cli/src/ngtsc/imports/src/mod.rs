// Imports Source Module

pub mod references;
pub mod core;
pub mod imported_symbols_tracker;
pub mod reexport;
pub mod resolver;
pub mod find_export;
pub mod default;
pub mod deferred_symbol_tracker;
pub mod local_compilation_extra_imports_tracker;
pub mod alias;
pub mod emitter;

// Re-exports
pub use references::{Reference, OwningModule};
pub use core::{ImportRewriter, NoopImportRewriter, R3SymbolsImportRewriter, validate_and_rewrite_core_symbol};
pub use imported_symbols_tracker::ImportedSymbolsTracker;
pub use reexport::Reexport;
pub use resolver::ModuleResolver;
pub use find_export::{find_exported_name_of_node, ExportInfo, ExportMap};
pub use default::{DefaultImportTracker, attach_default_import_declaration, get_default_import_declaration};
pub use deferred_symbol_tracker::{DeferredSymbolTracker, SymbolState};
pub use local_compilation_extra_imports_tracker::{LocalCompilationExtraImportsTracker, remove_quotations};
pub use alias::{AliasingHost, AliasStrategy, UnifiedModulesAliasingHost, PrivateExportAliasingHost};
pub use emitter::{
    ReferenceEmitter, ReferenceEmitStrategy, ReferenceEmitResult, ReferenceEmitKind,
    EmittedReference, FailedEmitResult, ImportFlags, ImportedFile,
    LocalIdentifierStrategy, RelativePathStrategy, AbsoluteModuleStrategy, LogicalProjectStrategy,
};
