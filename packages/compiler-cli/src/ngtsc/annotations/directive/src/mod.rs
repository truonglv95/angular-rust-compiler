// Annotations Directive Source Module

pub mod symbol;
pub mod initializer_function_access;
pub mod input_output_parse_options;
pub mod input_function;
pub mod output_function;
pub mod initializer_functions;
pub mod query_functions;
pub mod model_function;
pub mod handler;

// Re-exports
pub use symbol::{
    DirectiveSymbol, DirectiveTypeCheckMeta, SemanticTypeParameter,
    InputOrOutput, InputMappingMeta, TemplateGuardMeta, TemplateGuardType,
};
pub use initializer_function_access::{
    AccessLevel, InitializerApiConfig, AccessLevelError,
    validate_access_of_initializer_api_member,
};
pub use input_output_parse_options::{
    InputOutputOptions, OptionsParseError,
    parse_and_validate_input_and_output_options, parse_options_from_pairs,
};
pub use input_function::{
    SignalInputMapping, input_initializer_config, try_parse_signal_input_mapping,
};
pub use output_function::{
    OutputMapping, output_initializer_config, output_from_observable_config,
    output_initializer_configs, try_parse_initializer_based_output,
};
pub use initializer_functions::{
    OwningModule, InitializerFunctionName, InitializerApiFunction,
    InitializerFunctionMetadata, try_parse_initializer_api,
};
pub use query_functions::{
    QueryFunctionMetadata, query_initializer_apis, try_parse_signal_query,
};
pub use model_function::{
    ModelFunctionMetadata, try_parse_model_function,
};
pub use handler::{
    DirectiveDecoratorHandler, DirectiveHandlerData, R3DirectiveMetadata,
    DirectiveInput, DirectiveOutput, DirectiveQuery, HostBindings, HostDirectiveMeta,
    FIELD_DECORATORS, LIFECYCLE_HOOKS,
};

