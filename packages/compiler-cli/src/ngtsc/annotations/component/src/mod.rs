// Annotations Component Source Module

pub mod handler;
pub mod symbol;
pub mod resources;
pub mod metadata;

// Re-exports
pub use handler::ComponentDecoratorHandler;
pub use symbol::{ComponentSymbol, SemanticReference};
pub use resources::{
    StyleUrlMeta, ResourceTypeForDiagnostics, ParsedComponentTemplate,
    ParsedTemplateWithSource, SourceMapping, TemplateDeclaration,
    ExtractTemplateOptions, extract_template, parse_template_declaration,
};
pub use metadata::{
    R3ComponentMetadata, ComponentInput, ComponentOutput, ComponentHostBindings,
    ComponentTemplateInfo, ViewEncapsulation, ChangeDetectionStrategy,
    DeferredBlock, DeferTrigger,
};
