// TypeCheck Source Module

pub mod context;
pub mod type_check_block;
pub mod checker;
pub mod diagnostics;

// Re-exports
pub use context::{TypeCheckingContext, TypeCheckEnvironment};
pub use type_check_block::{TypeCheckBlockGenerator, OutOfBandDiagnosticRecorder};
pub use checker::TemplateTypeCheckerImpl;
pub use diagnostics::{
    TemplateDiagnosticCode,
    create_unknown_property_diagnostic,
    create_unknown_element_diagnostic,
    create_missing_pipe_diagnostic,
    create_type_mismatch_diagnostic,
    create_missing_required_input_diagnostic,
};
