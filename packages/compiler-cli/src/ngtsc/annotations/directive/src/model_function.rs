// Model Function
//
// Handles parsing of model() initializer for two-way binding.

use super::input_output_parse_options::InputOutputOptions;

/// Model metadata.
#[derive(Debug, Clone)]
pub struct ModelFunctionMetadata {
    /// Property name.
    pub property_name: String,
    /// Input binding name.
    pub input_name: String,
    /// Output binding name (typically propertyName + "Change").
    pub output_name: String,
    /// Whether the model is required.
    pub required: bool,
    /// Whether this is signal-based.
    pub is_signal: bool,
}

impl ModelFunctionMetadata {
    pub fn new(property_name: impl Into<String>) -> Self {
        let name = property_name.into();
        let output = format!("{}Change", name);
        Self {
            property_name: name.clone(),
            input_name: name,
            output_name: output,
            required: false,
            is_signal: true,
        }
    }
    
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        let a = alias.into();
        self.output_name = format!("{}Change", a);
        self.input_name = a;
        self
    }
    
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
}

/// Try to parse a model from member metadata.
pub fn try_parse_model_function(
    member_name: &str,
    options: Option<&InputOutputOptions>,
    is_required: bool,
) -> ModelFunctionMetadata {
    let alias = options.and_then(|o| o.alias.clone());
    
    let mut model = ModelFunctionMetadata::new(member_name);
    model.required = is_required;
    
    if let Some(a) = alias {
        model = model.with_alias(a);
    }
    
    model
}
