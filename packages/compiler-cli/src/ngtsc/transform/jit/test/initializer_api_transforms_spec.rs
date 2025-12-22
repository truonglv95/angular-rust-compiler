// Initializer API Transforms Tests
//
// Tests for signal input/output/model/query transforms.

use super::super::initializer_api_transforms::*;
use super::super::initializer_api_transforms::transform_api::*;
use crate::ngtsc::imports::ImportedSymbolsTracker;

// ============================================================================
// PropertyInfo Helper Tests
// ============================================================================

#[test]
fn test_property_info_creation() {
    let info = PropertyInfo {
        name: "myInput".to_string(),
        value_string: Some("input()".to_string()),
        is_static: false,
    };
    
    assert_eq!(info.name, "myInput");
    assert!(!info.is_static);
}

// ============================================================================
// Signal Input Transform Tests
// ============================================================================

#[test]
fn test_is_signal_input_call_basic() {
    assert!(is_signal_input_call(Some("input()"), true));
    assert!(is_signal_input_call(Some("input(0)"), true));
    assert!(is_signal_input_call(Some("input({ alias: 'foo' })"), true));
}

#[test]
fn test_is_signal_input_call_required() {
    assert!(is_signal_input_call(Some("input.required()"), true));
    assert!(is_signal_input_call(Some("input.required<string>()"), true));
}

#[test]
fn test_is_signal_input_call_negative() {
    assert!(!is_signal_input_call(Some("output()"), true));
    assert!(!is_signal_input_call(Some("model()"), true));
    assert!(!is_signal_input_call(None, true));
}

#[test]
fn test_signal_inputs_transform_basic() {
    let property = PropertyInfo {
        name: "someInput".to_string(),
        value_string: Some("input()".to_string()),
        is_static: false,
    };
    let tracker = ImportedSymbolsTracker::new();
    
    let result = signal_inputs_transform(&property, &tracker, true);
    
    assert!(result.transformed);
    assert_eq!(result.decorators.len(), 1);
    assert_eq!(result.decorators[0].name, "Input");
    assert_eq!(result.decorators[0].import_from, "@angular/core");
}

#[test]
fn test_signal_inputs_transform_with_alias() {
    let property = PropertyInfo {
        name: "someInput".to_string(),
        value_string: Some("input({ alias: 'customName' })".to_string()),
        is_static: false,
    };
    let tracker = ImportedSymbolsTracker::new();
    
    let result = signal_inputs_transform(&property, &tracker, true);
    
    assert!(result.transformed);
    assert_eq!(result.decorators.len(), 1);
    // Check that alias is in the args
    assert!(!result.decorators[0].args.is_empty());
    assert!(result.decorators[0].args[0].contains("customName"));
}

#[test]
fn test_signal_inputs_transform_non_input() {
    let property = PropertyInfo {
        name: "someOutput".to_string(),
        value_string: Some("output()".to_string()),
        is_static: false,
    };
    let tracker = ImportedSymbolsTracker::new();
    
    let result = signal_inputs_transform(&property, &tracker, true);
    
    assert!(!result.transformed);
}

// ============================================================================
// Signal Output Transform Tests
// ============================================================================

#[test]
fn test_is_signal_output_call() {
    assert!(is_signal_output_call(Some("output()"), true));
    assert!(is_signal_output_call(Some("output({ alias: 'foo' })"), true));
    assert!(!is_signal_output_call(Some("input()"), true));
}

#[test]
fn test_output_transform_basic() {
    let property = PropertyInfo {
        name: "clicked".to_string(),
        value_string: Some("output()".to_string()),
        is_static: false,
    };
    let tracker = ImportedSymbolsTracker::new();
    
    let result = initializer_api_output_transform(&property, &tracker, true);
    
    assert!(result.transformed);
    assert_eq!(result.decorators.len(), 1);
    assert_eq!(result.decorators[0].name, "Output");
}

#[test]
fn test_output_transform_with_alias() {
    let property = PropertyInfo {
        name: "clicked".to_string(),
        value_string: Some("output({ alias: 'onClicked' })".to_string()),
        is_static: false,
    };
    let tracker = ImportedSymbolsTracker::new();
    
    let result = initializer_api_output_transform(&property, &tracker, true);
    
    assert!(result.transformed);
    assert!(!result.decorators[0].args.is_empty());
    assert!(result.decorators[0].args[0].contains("onClicked"));
}

// ============================================================================
// Signal Model Transform Tests
// ============================================================================

#[test]
fn test_is_signal_model_call() {
    assert!(is_signal_model_call(Some("model()"), true));
    assert!(is_signal_model_call(Some("model(0)"), true));
    assert!(is_signal_model_call(Some("model.required()"), true));
    assert!(!is_signal_model_call(Some("input()"), true));
}

#[test]
fn test_model_transform_creates_input_and_output() {
    let property = PropertyInfo {
        name: "value".to_string(),
        value_string: Some("model(0)".to_string()),
        is_static: false,
    };
    let tracker = ImportedSymbolsTracker::new();
    
    let result = signal_model_transform(&property, &tracker, true);
    
    assert!(result.transformed);
    assert_eq!(result.decorators.len(), 2);
    assert_eq!(result.decorators[0].name, "Input");
    assert_eq!(result.decorators[1].name, "Output");
}

#[test]
fn test_model_transform_output_has_change_suffix() {
    let property = PropertyInfo {
        name: "value".to_string(),
        value_string: Some("model()".to_string()),
        is_static: false,
    };
    let tracker = ImportedSymbolsTracker::new();
    
    let result = signal_model_transform(&property, &tracker, true);
    
    // Output should have "valueChange" in args
    assert!(result.decorators[1].args[0].contains("valueChange"));
}

#[test]
fn test_model_transform_with_alias() {
    let property = PropertyInfo {
        name: "value".to_string(),
        value_string: Some("model({ alias: 'customValue' })".to_string()),
        is_static: false,
    };
    let tracker = ImportedSymbolsTracker::new();
    
    let result = signal_model_transform(&property, &tracker, true);
    
    assert!(result.transformed);
    // Input should use alias
    assert!(result.decorators[0].args[0].contains("customValue"));
    // Output should be "customValueChange"
    assert!(result.decorators[1].args[0].contains("customValueChange"));
}

// ============================================================================
// Query Functions Transform Tests
// ============================================================================

#[test]
fn test_is_query_function_call() {
    assert!(is_query_function_call(Some("viewChild('ref')"), true));
    assert!(is_query_function_call(Some("viewChildren(MyComp)"), true));
    assert!(is_query_function_call(Some("contentChild('ref')"), true));
    assert!(is_query_function_call(Some("contentChildren(MyComp)"), true));
    assert!(!is_query_function_call(Some("input()"), true));
}

#[test]
fn test_query_functions_transform_view_child() {
    let property = PropertyInfo {
        name: "myRef".to_string(),
        value_string: Some("viewChild('ref')".to_string()),
        is_static: false,
    };
    let tracker = ImportedSymbolsTracker::new();
    
    let result = query_functions_transforms(&property, &tracker, true);
    
    assert!(result.transformed);
    assert_eq!(result.decorators.len(), 1);
    assert_eq!(result.decorators[0].name, "ViewChild");
}

#[test]
fn test_query_functions_transform_view_children() {
    let property = PropertyInfo {
        name: "items".to_string(),
        value_string: Some("viewChildren(ItemComponent)".to_string()),
        is_static: false,
    };
    let tracker = ImportedSymbolsTracker::new();
    
    let result = query_functions_transforms(&property, &tracker, true);
    
    assert!(result.transformed);
    assert_eq!(result.decorators[0].name, "ViewChildren");
}

#[test]
fn test_query_functions_transform_content_child() {
    let property = PropertyInfo {
        name: "header".to_string(),
        value_string: Some("contentChild('header')".to_string()),
        is_static: false,
    };
    let tracker = ImportedSymbolsTracker::new();
    
    let result = query_functions_transforms(&property, &tracker, true);
    
    assert!(result.transformed);
    assert_eq!(result.decorators[0].name, "ContentChild");
}

#[test]
fn test_query_functions_transform_content_children() {
    let property = PropertyInfo {
        name: "tabs".to_string(),
        value_string: Some("contentChildren(TabComponent)".to_string()),
        is_static: false,
    };
    let tracker = ImportedSymbolsTracker::new();
    
    let result = query_functions_transforms(&property, &tracker, true);
    
    assert!(result.transformed);
    assert_eq!(result.decorators[0].name, "ContentChildren");
}

// ============================================================================
// Synthetic Decorator Tests
// ============================================================================

#[test]
fn test_synthetic_decorator_creation() {
    let decorator = SyntheticDecorator::new("Input", "@angular/core");
    
    assert_eq!(decorator.name, "Input");
    assert_eq!(decorator.import_from, "@angular/core");
    assert!(decorator.args.is_empty());
}

#[test]
fn test_synthetic_decorator_with_arg() {
    let decorator = SyntheticDecorator::new("Output", "@angular/core")
        .with_arg("'clicked'");
    
    assert_eq!(decorator.args.len(), 1);
    assert_eq!(decorator.args[0], "'clicked'");
}

#[test]
fn test_synthetic_decorator_with_multiple_args() {
    let decorator = SyntheticDecorator::new("ViewChild", "@angular/core")
        .with_args(vec!["'template'".to_string(), "{ read: ElementRef }".to_string()]);
    
    assert_eq!(decorator.args.len(), 2);
}

#[test]
fn test_create_synthetic_angular_core_decorator_access() {
    let decorator = create_synthetic_angular_core_decorator_access("Input");
    
    assert_eq!(decorator.name, "Input");
    assert_eq!(decorator.import_from, "@angular/core");
}

// ============================================================================
// Transform Result Tests
// ============================================================================

#[test]
fn test_property_transform_result_unchanged() {
    let result = PropertyTransformResult::unchanged();
    
    assert!(!result.transformed);
    assert!(result.decorators.is_empty());
    assert!(result.new_initializer.is_none());
}

#[test]
fn test_property_transform_result_with_decorators() {
    let decorators = vec![
        SyntheticDecorator::new("Input", "@angular/core"),
    ];
    let result = PropertyTransformResult::with_decorators(decorators);
    
    assert!(result.transformed);
    assert_eq!(result.decorators.len(), 1);
}

// ============================================================================
// InitializerApiJitTransform Tests
// ============================================================================

#[test]
fn test_initializer_api_jit_transform_creation() {
    let tracker = ImportedSymbolsTracker::new();
    let transform = get_initializer_api_jit_transform(tracker, false);
    
    assert!(!transform.is_transformable_class_decorator("SomeDecorator"));
    assert!(transform.is_transformable_class_decorator("Directive"));
    assert!(transform.is_transformable_class_decorator("Component"));
}

#[test]
fn test_initializer_api_jit_transform_class() {
    let tracker = ImportedSymbolsTracker::new();
    let transform = get_initializer_api_jit_transform(tracker, true);
    
    let properties = vec![
        PropertyInfo {
            name: "name".to_string(),
            value_string: Some("input()".to_string()),
            is_static: false,
        },
        PropertyInfo {
            name: "clicked".to_string(),
            value_string: Some("output()".to_string()),
            is_static: false,
        },
    ];
    
    let results = transform.transform_class("MyDir", &properties);
    
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, "name"); // Property name
    assert_eq!(results[1].0, "clicked");
}
