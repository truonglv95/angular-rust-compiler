// Output Function
//
// Handles signal output parsing and validation.

use super::initializer_function_access::{AccessLevel, InitializerApiConfig};
use super::input_output_parse_options::InputOutputOptions;

/// Configuration for the output() initializer function.
pub fn output_initializer_config() -> InitializerApiConfig {
    InitializerApiConfig::new(
        "output",
        vec![AccessLevel::Public, AccessLevel::Protected],
    )
}

/// Configuration for outputFromObservable() from rxjs-interop.
pub fn output_from_observable_config() -> InitializerApiConfig {
    InitializerApiConfig::new(
        "outputFromObservable",
        vec![AccessLevel::Public, AccessLevel::Protected],
    )
}

/// List of all output initializer configurations.
pub fn output_initializer_configs() -> Vec<InitializerApiConfig> {
    vec![
        output_initializer_config(),
        output_from_observable_config(),
    ]
}

/// Parsed output mapping.
#[derive(Debug, Clone)]
pub struct OutputMapping {
    /// Whether this is a signal-based output.
    pub is_signal: bool,
    /// The class property name.
    pub class_property_name: String,
    /// The binding property name (public name).
    pub binding_property_name: String,
}

impl OutputMapping {
    pub fn new(class_property_name: impl Into<String>) -> Self {
        let name = class_property_name.into();
        Self {
            is_signal: false,
            class_property_name: name.clone(),
            binding_property_name: name,
        }
    }
    
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.binding_property_name = alias.into();
        self
    }
}

/// Try to parse an initializer-based output from member metadata.
pub fn try_parse_initializer_based_output(
    member_name: &str,
    options: Option<&InputOutputOptions>,
) -> OutputMapping {
    let alias = options.and_then(|o| o.alias.clone());
    
    OutputMapping {
        is_signal: false,
        class_property_name: member_name.to_string(),
        binding_property_name: alias.unwrap_or_else(|| member_name.to_string()),
    }
}
