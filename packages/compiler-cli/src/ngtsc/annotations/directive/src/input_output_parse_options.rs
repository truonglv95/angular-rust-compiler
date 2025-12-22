// Input/Output Parse Options
//
// Parses and validates input and output initializer function options.

use std::collections::HashMap;

/// Options for input/output initializer functions.
#[derive(Debug, Clone, Default)]
pub struct InputOutputOptions {
    /// The alias binding name, if specified.
    pub alias: Option<String>,
}

/// Error when parsing input/output options.
#[derive(Debug, Clone)]
pub struct OptionsParseError {
    pub message: String,
}

impl OptionsParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
    
    pub fn wrong_type(expected: &str) -> Self {
        Self::new(format!("Argument needs to be {} that is statically analyzable.", expected))
    }
}

/// Parse and validate input/output options from an object literal.
pub fn parse_and_validate_input_and_output_options(
    options: &HashMap<String, String>,
) -> Result<InputOutputOptions, OptionsParseError> {
    let alias = options.get("alias").cloned();
    
    Ok(InputOutputOptions { alias })
}

/// Parse options from a simplified representation (key-value pairs).
pub fn parse_options_from_pairs(
    pairs: &[(String, String)],
) -> Result<InputOutputOptions, OptionsParseError> {
    let options: HashMap<String, String> = pairs.iter().cloned().collect();
    parse_and_validate_input_and_output_options(&options)
}
