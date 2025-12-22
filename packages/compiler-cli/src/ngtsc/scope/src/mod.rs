// Scope Source Module

pub mod api;
pub mod local;
pub mod standalone;
pub mod util;
pub mod typecheck;
pub mod dependency;
pub mod component_scope;

// Re-exports
pub use api::{
    ExportScope, DirectiveExport, PipeExport,
    CompilationScope, DirectiveInScope, PipeInScope,
    RegisterResult,
};
pub use local::LocalModuleScopeRegistry;
pub use standalone::{StandaloneComponentScopeReader, StandaloneImport, RemoteScope};
pub use util::{ReferenceKind, SelectorPart, selector_matches_element, parse_selector};
pub use typecheck::{
    TypeCheckScope, TypeCheckScopeRegistry,
    TypeCheckDirective, TypeCheckPipe,
    TypeCheckInput, TypeCheckOutput,
};
pub use dependency::{DependencyScopeReader, ExternalDirectiveMetadata, ExternalPipeMetadata};
pub use component_scope::ComponentScopeReader;
