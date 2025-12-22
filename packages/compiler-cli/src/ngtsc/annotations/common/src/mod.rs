// Annotations Common Source Module

pub mod api;
pub mod factory;
pub mod references_registry;
pub mod jit_declaration_registry;
pub mod injectable_registry;
pub mod input_transforms;
pub mod debug_info;
pub mod schema;
pub mod di;
pub mod evaluation;
pub mod util;
pub mod metadata;
pub mod diagnostics;

// Re-exports
pub use api::{ResourceLoader, ResourceLoaderContext, ResourceType, NoopResourceLoader};
pub use factory::{
    CompileResult, R3FactoryMetadata, FactoryTarget,
    compile_ng_factory_def_field, compile_declare_factory,
};
pub use references_registry::{ReferencesRegistry, NoopReferencesRegistry, CollectingReferencesRegistry};
pub use jit_declaration_registry::JitDeclarationRegistry;
pub use injectable_registry::{InjectableClassRegistry, InjectableMeta};
pub use input_transforms::{InputMapping, InputTransform, compile_input_transform_fields};
pub use debug_info::{R3ClassDebugInfo, extract_class_debug_info};
pub use schema::{SchemaMetadata, SchemaError, extract_schemas, has_custom_elements_schema, has_no_errors_schema};
pub use di::{
    R3DependencyMetadata, R3ResolvedDependencyType, ConstructorDeps, ConstructorDepError,
    UnavailableValueKind, CtorParameter, ParameterDecorator,
    get_constructor_dependencies, unwrap_constructor_dependencies, get_valid_constructor_dependencies,
};
pub use evaluation::{
    ViewEncapsulation, EnumValue, ResolvedValue,
    resolve_enum_value, resolve_encapsulation_enum_value_locally,
};
pub use util::{
    CORE_MODULE, Decorator, Import, R3Reference,
    is_angular_core, find_angular_decorator, is_angular_decorator, get_angular_decorators,
    unwrap_expression, expand_forward_ref, to_r3_reference, wrap_type_reference,
};
pub use metadata::{
    R3ClassMetadata, DecoratorMetadata, CtorParameterMetadata, PropDecoratorMetadata,
    extract_class_metadata, decorator_to_metadata, ctor_parameter_to_metadata,
};
pub use diagnostics::{
    ErrorCode, Diagnostic, RelatedInfo, FatalDiagnosticError,
    make_diagnostic_chain, create_value_has_wrong_type_error, make_duplicate_declaration_error,
    get_provider_diagnostics, get_undecorated_class_with_angular_features_diagnostic,
};

