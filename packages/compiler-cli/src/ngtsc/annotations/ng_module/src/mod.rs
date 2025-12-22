// Annotations NgModule Source Module

pub mod module_with_providers;
pub mod symbol;
pub mod handler;

// Re-exports
pub use module_with_providers::{
    ResolvedModuleWithProviders, ModuleWithProvidersError, MwpResolverConfig,
    is_module_with_providers_type, try_resolve_module_with_providers,
    is_resolved_module_with_providers,
};
pub use symbol::{NgModuleSymbol, RemotelyScopedComponent};
pub use handler::{
    NgModuleDecoratorHandler, NgModuleAnalysis, NgModuleResolution,
    R3NgModuleMetadata, R3InjectorMetadata, R3FactoryMetadata,
};
