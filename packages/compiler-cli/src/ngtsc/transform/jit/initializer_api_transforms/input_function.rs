// Signal Inputs Transform
//
// Transform for adding `@Input` decorator to signal input properties.
// Signal inputs cannot be recognized at runtime using reflection, so
// this transform adds a decorator for JIT compatibility.

use crate::ngtsc::imports::ImportedSymbolsTracker;
use super::transform_api::{
    PropertyTransformResult, PropertyInfo, is_signal_input_call,
    create_synthetic_angular_core_decorator_access,
};

/// Transform for signal inputs.
///
/// Checks if a class member is a signal input and adds the appropriate
/// `@Input` decorator for JIT compatibility.
pub fn signal_inputs_transform(
    property: &PropertyInfo,
    _import_tracker: &ImportedSymbolsTracker,
    is_core: bool,
) -> PropertyTransformResult {
    // Check if this is a signal input call
    if !is_signal_input_call(property.value_string.as_deref(), is_core) {
        return PropertyTransformResult::unchanged();
    }

    // Create the @Input decorator
    let input_decorator = create_signal_input_decorator(&property.name, property.value_string.as_deref());
    
    PropertyTransformResult::with_decorators(vec![input_decorator])
}

/// Create an @Input decorator for a signal input.
fn create_signal_input_decorator(property_name: &str, value: Option<&str>) -> super::transform_api::SyntheticDecorator {
    let mut decorator = create_synthetic_angular_core_decorator_access("Input");
    
    // Extract options from the input() call if present
    if let Some(value_str) = value {
        // Parse input options: input({ alias: 'foo', transform: fn })
        if let Some(alias) = extract_input_alias(value_str) {
            decorator = decorator.with_arg(format!("{{ alias: '{}', isSignal: true }}", alias));
        } else {
            decorator = decorator.with_arg(format!("{{ alias: '{}', isSignal: true }}", property_name));
        }
    }
    
    decorator
}

/// Extract alias from an input() call if present.
fn extract_input_alias(value: &str) -> Option<String> {
    // Simple parsing: look for alias: 'value' or alias: "value"
    if let Some(start) = value.find("alias:") {
        let rest = &value[start + 6..];
        let rest = rest.trim_start();
        
        // Find the string value
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
    fn test_extract_input_alias_single_quotes() {
        let value = "input({ alias: 'customName' })";
        assert_eq!(extract_input_alias(value), Some("customName".to_string()));
    }

    #[test]
    fn test_extract_input_alias_double_quotes() {
        let value = r#"input({ alias: "customName" })"#;
        assert_eq!(extract_input_alias(value), Some("customName".to_string()));
    }

    #[test]
    fn test_extract_input_alias_none() {
        let value = "input()";
        assert_eq!(extract_input_alias(value), None);
    }
}
