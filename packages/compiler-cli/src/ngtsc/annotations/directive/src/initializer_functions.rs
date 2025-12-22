// Initializer Functions
//
// Utilities for parsing Angular initializer API functions.

use super::initializer_function_access::AccessLevel;

/// Module that owns an initializer function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwningModule {
    AngularCore,
    AngularCoreRxjsInterop,
}

impl OwningModule {
    pub fn as_str(&self) -> &'static str {
        match self {
            OwningModule::AngularCore => "@angular/core",
            OwningModule::AngularCoreRxjsInterop => "@angular/core/rxjs-interop",
        }
    }
}

/// Initializer API function names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitializerFunctionName {
    Input,
    Model,
    Output,
    OutputFromObservable,
    ViewChild,
    ViewChildren,
    ContentChild,
    ContentChildren,
}

impl InitializerFunctionName {
    pub fn as_str(&self) -> &'static str {
        match self {
            InitializerFunctionName::Input => "input",
            InitializerFunctionName::Model => "model",
            InitializerFunctionName::Output => "output",
            InitializerFunctionName::OutputFromObservable => "outputFromObservable",
            InitializerFunctionName::ViewChild => "viewChild",
            InitializerFunctionName::ViewChildren => "viewChildren",
            InitializerFunctionName::ContentChild => "contentChild",
            InitializerFunctionName::ContentChildren => "contentChildren",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "input" => Some(InitializerFunctionName::Input),
            "model" => Some(InitializerFunctionName::Model),
            "output" => Some(InitializerFunctionName::Output),
            "outputFromObservable" => Some(InitializerFunctionName::OutputFromObservable),
            "viewChild" => Some(InitializerFunctionName::ViewChild),
            "viewChildren" => Some(InitializerFunctionName::ViewChildren),
            "contentChild" => Some(InitializerFunctionName::ContentChild),
            "contentChildren" => Some(InitializerFunctionName::ContentChildren),
            _ => None,
        }
    }
}

/// Initializer API function configuration.
#[derive(Debug, Clone)]
pub struct InitializerApiFunction {
    pub owning_module: OwningModule,
    pub function_name: InitializerFunctionName,
    pub allowed_access_levels: Vec<AccessLevel>,
}

impl InitializerApiFunction {
    pub fn new(owning_module: OwningModule, function_name: InitializerFunctionName) -> Self {
        Self {
            owning_module,
            function_name,
            allowed_access_levels: vec![AccessLevel::Public, AccessLevel::Protected],
        }
    }
    
    pub fn with_access_levels(mut self, levels: Vec<AccessLevel>) -> Self {
        self.allowed_access_levels = levels;
        self
    }
}

/// Metadata for a recognized initializer function call.
#[derive(Debug, Clone)]
pub struct InitializerFunctionMetadata {
    /// The matched API function.
    pub api: InitializerApiFunction,
    /// Whether this is a required initializer (e.g., input.required).
    pub is_required: bool,
    /// The call expression text.
    pub call_text: String,
}

/// Standard input/model/output initializer APIs.
pub fn input_initializer_api() -> InitializerApiFunction {
    InitializerApiFunction::new(OwningModule::AngularCore, InitializerFunctionName::Input)
}

pub fn model_initializer_api() -> InitializerApiFunction {
    InitializerApiFunction::new(OwningModule::AngularCore, InitializerFunctionName::Model)
}

pub fn output_initializer_api() -> InitializerApiFunction {
    InitializerApiFunction::new(OwningModule::AngularCore, InitializerFunctionName::Output)
}

pub fn output_from_observable_api() -> InitializerApiFunction {
    InitializerApiFunction::new(OwningModule::AngularCoreRxjsInterop, InitializerFunctionName::OutputFromObservable)
}

/// Query initializer APIs.
pub fn view_child_api() -> InitializerApiFunction {
    InitializerApiFunction::new(OwningModule::AngularCore, InitializerFunctionName::ViewChild)
}

pub fn view_children_api() -> InitializerApiFunction {
    InitializerApiFunction::new(OwningModule::AngularCore, InitializerFunctionName::ViewChildren)
}

pub fn content_child_api() -> InitializerApiFunction {
    InitializerApiFunction::new(OwningModule::AngularCore, InitializerFunctionName::ContentChild)
}

pub fn content_children_api() -> InitializerApiFunction {
    InitializerApiFunction::new(OwningModule::AngularCore, InitializerFunctionName::ContentChildren)
}

/// Try to parse an initializer API from expression text.
pub fn try_parse_initializer_api(
    expression_text: &str,
    allowed_apis: &[InitializerApiFunction],
) -> Option<InitializerFunctionMetadata> {
    let trimmed = expression_text.trim();
    
    // Check for .required() pattern
    let (base_name, is_required) = if let Some(idx) = trimmed.find(".required(") {
        (&trimmed[..idx], true)
    } else if let Some(idx) = trimmed.find('(') {
        (&trimmed[..idx], false)
    } else {
        return None;
    };
    
    // Try to match function name
    let fn_name = InitializerFunctionName::from_str(base_name)?;
    
    // Find matching API
    let api = allowed_apis.iter()
        .find(|a| a.function_name == fn_name)?
        .clone();
    
    Some(InitializerFunctionMetadata {
        api,
        is_required,
        call_text: trimmed.to_string(),
    })
}
