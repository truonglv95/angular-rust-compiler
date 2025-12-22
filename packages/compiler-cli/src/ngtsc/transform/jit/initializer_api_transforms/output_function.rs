// Output Function Transform
//
// Transform for adding `@Output` decorator to signal output properties.
// Signal outputs cannot be recognized at runtime using reflection, so
// this transform adds a decorator for JIT compatibility.

use crate::ngtsc::imports::ImportedSymbolsTracker;
use super::transform_api::{
    PropertyTransformResult, PropertyInfo, is_signal_output_call,
    create_synthetic_angular_core_decorator_access,
};

/// Transform for initializer API outputs (output()).
///
/// Checks if a class member is a signal output and adds the appropriate
/// `@Output` decorator for JIT compatibility.
pub fn initializer_api_output_transform(
    property: &PropertyInfo,
    _import_tracker: &ImportedSymbolsTracker,
    is_core: bool,
) -> PropertyTransformResult {
    // Check if this is a signal output call
    if !is_signal_output_call(property.value_string.as_deref(), is_core) {
        return PropertyTransformResult::unchanged();
    }

    // Create the @Output decorator
    let output_decorator = create_signal_output_decorator(&property.name, property.value_string.as_deref());
    
    PropertyTransformResult::with_decorators(vec![output_decorator])
}

/// Create an @Output decorator for a signal output.
fn create_signal_output_decorator(property_name: &str, value: Option<&str>) -> super::transform_api::SyntheticDecorator {
    let mut decorator = create_synthetic_angular_core_decorator_access("Output");
    
    // Extract alias from the output() call if present
    if let Some(value_str) = value {
        if let Some(alias) = extract_output_alias(value_str) {
            decorator = decorator.with_arg(format!("'{}'", alias));
        } else {
            // Use property name as the output name
            decorator = decorator.with_arg(format!("'{}'", property_name));
        }
    }
    
    decorator
}

/// Extract alias from an output() call if present.
fn extract_output_alias(value: &str) -> Option<String> {
    // Simple parsing: look for alias: 'value' or alias: "value"
    if let Some(start) = value.find("alias:") {
        let rest = &value[start + 6..];
        let rest = rest.trim_start();
        
        if rest.starts_with('\'') || rest.starts_with('"') {
            let quote = rest.chars().next().unwrap();
            if let Some(end) = rest[1..].find(quote) {
                return Some(rest[1..=end].to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_output_alias() {
        let value = "output({ alias: 'customOutput' })";
        assert_eq!(extract_output_alias(value), Some("customOutput".to_string()));
    }

    #[test]
    fn test_extract_output_alias_none() {
        let value = "output()";
        assert_eq!(extract_output_alias(value), None);
    }
}
