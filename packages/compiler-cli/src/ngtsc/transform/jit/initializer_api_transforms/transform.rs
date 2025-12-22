// Initializer API JIT Transform
//
// Creates an AST transform that looks for Angular classes and transforms
// initializer-based declared members to work with JIT compilation.
//
// For example, an `input()` member may be transformed to add an `@Input`
// decorator for JIT.

use crate::ngtsc::imports::ImportedSymbolsTracker;

use super::transform_api::{PropertyTransform, PropertyTransformResult, PropertyInfo};
use super::input_function::signal_inputs_transform;
use super::output_function::initializer_api_output_transform;
use super::query_functions::query_functions_transforms;
use super::model_function::signal_model_transform;

/// Decorators for classes that should be transformed.
const DECORATORS_WITH_INPUTS: &[&str] = &["Directive", "Component"];

/// List of possible property transforms.
/// The first one matched on a class member will apply.
fn get_property_transforms() -> Vec<PropertyTransform> {
    vec![
        signal_inputs_transform,
        initializer_api_output_transform,
        query_functions_transforms,
        signal_model_transform,
    ]
}

/// Configuration for the initializer API JIT transform.
#[derive(Debug, Clone)]
pub struct InitializerApiJitTransformConfig {
    /// Whether this is the Angular core package.
    pub is_core: bool,
}

impl Default for InitializerApiJitTransformConfig {
    fn default() -> Self {
        Self { is_core: false }
    }
}

/// The initializer API JIT transform.
///
/// This transform looks for Angular classes and transforms initializer-based
/// declared members to work with JIT compilation.
pub struct InitializerApiJitTransform {
    /// Import tracker for efficient import checking.
    import_tracker: ImportedSymbolsTracker,
    /// Configuration.
    config: InitializerApiJitTransformConfig,
}

impl InitializerApiJitTransform {
    /// Create a new initializer API JIT transform.
    pub fn new(import_tracker: ImportedSymbolsTracker, is_core: bool) -> Self {
        Self {
            import_tracker,
            config: InitializerApiJitTransformConfig { is_core },
        }
    }

    /// Check if a decorator indicates a class should be transformed.
    pub fn is_transformable_class_decorator(&self, decorator_name: &str) -> bool {
        DECORATORS_WITH_INPUTS.contains(&decorator_name)
    }

    /// Transform a property.
    ///
    /// Applies property transforms in order, returning the first matching result.
    pub fn transform_property(&self, property: &PropertyInfo) -> PropertyTransformResult {
        let transforms = get_property_transforms();
        
        for transform in transforms {
            let result = transform(property, &self.import_tracker, self.config.is_core);
            
            if result.transformed {
                return result;
            }
        }
        
        PropertyTransformResult::unchanged()
    }

    /// Transform a class.
    ///
    /// Returns a list of transformation results for each property that was transformed.
    pub fn transform_class(
        &self,
        _class_name: &str,
        properties: &[PropertyInfo],
    ) -> Vec<(String, PropertyTransformResult)> {
        let mut results = Vec::new();
        
        for property in properties {
            let result = self.transform_property(property);
            if result.transformed {
                results.push((property.name.clone(), result));
            }
        }
        
        results
    }
}

/// Create an initializer API JIT transform.
pub fn get_initializer_api_jit_transform(
    import_tracker: ImportedSymbolsTracker,
    is_core: bool,
) -> InitializerApiJitTransform {
    InitializerApiJitTransform::new(import_tracker, is_core)
}
