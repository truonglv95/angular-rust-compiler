// Input Function
//
// Handles signal input parsing and validation.

use super::initializer_function_access::{AccessLevel, InitializerApiConfig};
use super::input_output_parse_options::InputOutputOptions;

/// Configuration for the input() initializer function.
pub fn input_initializer_config() -> InitializerApiConfig {
    InitializerApiConfig::new(
        "input",
        vec![AccessLevel::Public, AccessLevel::Protected],
    )
}

/// Parsed signal input mapping.
#[derive(Debug, Clone)]
pub struct SignalInputMapping {
    /// Whether this is a signal input.
    pub is_signal: bool,
    /// The class property name.
    pub class_property_name: String,
    /// The binding property name (public name).
    pub binding_property_name: String,
    /// Whether this input is required.
    pub required: bool,
    /// Transform metadata (not used for signal inputs).
    pub transform: Option<String>,
}

impl SignalInputMapping {
    pub fn new(class_property_name: impl Into<String>, required: bool) -> Self {
        let name = class_property_name.into();
        Self {
            is_signal: true,
            class_property_name: name.clone(),
            binding_property_name: name,
            required,
            transform: None,
        }
    }
    
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.binding_property_name = alias.into();
        self
    }
}

/// Try to parse a signal input from member metadata.
pub fn try_parse_signal_input_mapping(
    member_name: &str,
    options: Option<&InputOutputOptions>,
    is_required: bool,
) -> SignalInputMapping {
    let alias = options.and_then(|o| o.alias.clone());
    
    SignalInputMapping {
        is_signal: true,
        class_property_name: member_name.to_string(),
        binding_property_name: alias.unwrap_or_else(|| member_name.to_string()),
        required: is_required,
        transform: None,
    }
}
