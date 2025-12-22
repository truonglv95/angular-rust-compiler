// Model Function Transform
//
// Transform for adding `@Input` and `@Output` decorators to signal model properties.
// Signal models create two-way bindings and need both decorators for JIT compatibility.

use crate::ngtsc::imports::ImportedSymbolsTracker;
use super::transform_api::{
    PropertyTransformResult, PropertyInfo, is_signal_model_call,
    create_synthetic_angular_core_decorator_access, SyntheticDecorator,
};

/// Transform for signal models (model()).
///
/// Checks if a class member is a signal model and adds the appropriate
/// `@Input` and `@Output` decorators for JIT compatibility.
///
/// A model() creates a two-way binding, so it needs both:
/// - @Input for the property itself
/// - @Output for the propertyChange event
pub fn signal_model_transform(
    property: &PropertyInfo,
    _import_tracker: &ImportedSymbolsTracker,
    is_core: bool,
) -> PropertyTransformResult {
    // Check if this is a signal model call
    if !is_signal_model_call(property.value_string.as_deref(), is_core) {
        return PropertyTransformResult::unchanged();
    }

    // Create both @Input and @Output decorators for two-way binding
    let decorators = create_model_decorators(&property.name, property.value_string.as_deref());
    
    PropertyTransformResult::with_decorators(decorators)
}

/// Create decorators for a signal model (both @Input and @Output).
fn create_model_decorators(property_name: &str, value: Option<&str>) -> Vec<SyntheticDecorator> {
    let alias = value.and_then(extract_model_alias).unwrap_or_else(|| property_name.to_string());
    
    // Create @Input decorator for the property
    let input_decorator = create_synthetic_angular_core_decorator_access("Input")
        .with_arg(format!("{{ alias: '{}', isSignal: true }}", alias));
    
    // Create @Output decorator for the propertyChange event
    let output_name = format!("{}Change", alias);
    let output_decorator = create_synthetic_angular_core_decorator_access("Output")
        .with_arg(format!("'{}'", output_name));
    
    vec![input_decorator, output_decorator]
}

/// Extract alias from a model() call if present.
fn extract_model_alias(value: &str) -> Option<String> {
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
    fn test_create_model_decorators() {
        let decorators = create_model_decorators("value", None);
        
        assert_eq!(decorators.len(), 2);
        assert_eq!(decorators[0].name, "Input");
        assert_eq!(decorators[1].name, "Output");
    }

    #[test]
    fn test_extract_model_alias() {
        let value = "model({ alias: 'customValue' })";
        assert_eq!(extract_model_alias(value), Some("customValue".to_string()));
    }

    #[test]
    fn test_extract_model_alias_none() {
        let value = "model()";
        assert_eq!(extract_model_alias(value), None);
    }
}
