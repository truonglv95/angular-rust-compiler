// Downlevel Decorators Transform Tests
//
// Tests for decorator downleveling functionality.

use super::super::downlevel_decorators_transform::*;

// ============================================================================
// Decorator Metadata Tests
// ============================================================================

#[test]
fn test_decorator_metadata_creation() {
    let metadata = DecoratorMetadata {
        type_name: "Injectable".to_string(),
        args: None,
    };
    
    assert_eq!(metadata.type_name, "Injectable");
    assert!(metadata.args.is_none());
}

#[test]
fn test_decorator_metadata_with_args() {
    let metadata = DecoratorMetadata {
        type_name: "Directive".to_string(),
        args: Some(vec!["{ selector: 'app' }".to_string()]),
    };
    
    assert_eq!(metadata.type_name, "Directive");
    assert_eq!(metadata.args.as_ref().unwrap().len(), 1);
}

// ============================================================================
// Constructor Parameters Property Tests
// ============================================================================

#[test]
fn test_create_ctor_parameters_without_closure() {
    let params = vec![
        ParameterDecorationInfo {
            type_ref: Some("ClassInject".to_string()),
            decorators: vec![],
        },
    ];
    
    let property = create_ctor_parameters_class_property(&params, false);
    
    assert_eq!(property.parameters.len(), 1);
    assert!(property.closure_annotation.is_none());
}

#[test]
fn test_create_ctor_parameters_with_closure() {
    let params = vec![
        ParameterDecorationInfo {
            type_ref: Some("ClassInject".to_string()),
            decorators: vec![],
        },
    ];
    
    let property = create_ctor_parameters_class_property(&params, true);
    
    assert_eq!(property.parameters.len(), 1);
    assert!(property.closure_annotation.is_some());
    assert!(property.closure_annotation.as_ref().unwrap().contains("@nocollapse"));
}

#[test]
fn test_create_ctor_parameters_with_decorators() {
    let params = vec![
        ParameterDecorationInfo {
            type_ref: Some("Document".to_string()),
            decorators: vec![
                DecoratorMetadata {
                    type_name: "Inject".to_string(),
                    args: Some(vec!["DOCUMENT".to_string()]),
                },
            ],
        },
    ];
    
    let property = create_ctor_parameters_class_property(&params, false);
    
    assert_eq!(property.parameters.len(), 1);
    assert_eq!(property.parameters[0].decorators.len(), 1);
    assert_eq!(property.parameters[0].decorators[0].type_name, "Inject");
}

#[test]
fn test_create_ctor_parameters_multiple() {
    let params = vec![
        ParameterDecorationInfo {
            type_ref: Some("HttpClient".to_string()),
            decorators: vec![],
        },
        ParameterDecorationInfo {
            type_ref: Some("Router".to_string()),
            decorators: vec![],
        },
        ParameterDecorationInfo {
            type_ref: None, // optional with @Optional
            decorators: vec![
                DecoratorMetadata {
                    type_name: "Optional".to_string(),
                    args: None,
                },
            ],
        },
    ];
    
    let property = create_ctor_parameters_class_property(&params, false);
    
    assert_eq!(property.parameters.len(), 3);
    assert!(property.parameters[2].type_ref.is_none());
    assert_eq!(property.parameters[2].decorators.len(), 1);
}

// ============================================================================
// Property Decorators Tests
// ============================================================================

#[test]
fn test_create_prop_decorators() {
    let mut properties = std::collections::HashMap::new();
    properties.insert(
        "disabled".to_string(),
        vec![DecoratorMetadata {
            type_name: "Input".to_string(),
            args: None,
        }],
    );
    
    let property = create_prop_decorators_class_property(&properties, false);
    
    assert_eq!(property.properties.len(), 1);
    assert!(property.properties.contains_key("disabled"));
    assert!(property.closure_annotation.is_none());
}

#[test]
fn test_create_prop_decorators_with_closure() {
    let mut properties = std::collections::HashMap::new();
    properties.insert(
        "myProp".to_string(),
        vec![DecoratorMetadata {
            type_name: "ViewChild".to_string(),
            args: Some(vec!["'template'".to_string()]),
        }],
    );
    
    let property = create_prop_decorators_class_property(&properties, true);
    
    assert!(property.closure_annotation.is_some());
    assert!(property.closure_annotation.as_ref().unwrap().contains("@type"));
}

#[test]
fn test_create_prop_decorators_multiple() {
    let mut properties = std::collections::HashMap::new();
    properties.insert(
        "name".to_string(),
        vec![DecoratorMetadata {
            type_name: "Input".to_string(),
            args: None,
        }],
    );
    properties.insert(
        "clicked".to_string(),
        vec![DecoratorMetadata {
            type_name: "Output".to_string(),
            args: None,
        }],
    );
    properties.insert(
        "template".to_string(),
        vec![DecoratorMetadata {
            type_name: "ViewChild".to_string(),
            args: Some(vec!["'template'".to_string()]),
        }],
    );
    
    let property = create_prop_decorators_class_property(&properties, false);
    
    assert_eq!(property.properties.len(), 3);
    assert!(property.properties.contains_key("name"));
    assert!(property.properties.contains_key("clicked"));
    assert!(property.properties.contains_key("template"));
}

// ============================================================================
// Transform Tests
// ============================================================================

#[test]
fn test_downlevel_decorators_transform_creation() {
    let mut transform = get_downlevel_decorators_transform(false, false);
    
    // Test config
    let result = transform.transform_class("MyService");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn test_downlevel_decorators_transform_is_core() {
    let mut transform = get_downlevel_decorators_transform(true, true);
    
    // When is_core=true, config should reflect that
    let result = transform.transform_class("Injectable");
    assert!(result.ctor_parameters.is_none()); // TODO - not implemented
}

#[test]
fn test_downlevel_decorators_transform_class() {
    let mut transform = get_downlevel_decorators_transform(false, false);
    
    let result = transform.transform_class("MyService");
    
    // Currently returns empty result (TODO)
    assert!(result.ctor_parameters.is_none());
    assert!(result.prop_decorators.is_none());
    assert!(result.diagnostics.is_empty());
}

// ============================================================================
// Downleveled Class Result Tests  
// ============================================================================

#[test]
fn test_downleveled_class_empty() {
    let result = DownleveledClass {
        ctor_parameters: None,
        prop_decorators: None,
        diagnostics: vec![],
    };
    
    assert!(result.ctor_parameters.is_none());
    assert!(result.prop_decorators.is_none());
    assert!(result.diagnostics.is_empty());
}

#[test]
fn test_downleveled_class_with_ctor() {
    let ctor = CtorParametersProperty {
        parameters: vec![
            ParameterDecorationInfo {
                type_ref: Some("MyService".to_string()),
                decorators: vec![],
            },
        ],
        closure_annotation: None,
    };
    
    let result = DownleveledClass {
        ctor_parameters: Some(ctor),
        prop_decorators: None,
        diagnostics: vec![],
    };
    
    assert!(result.ctor_parameters.is_some());
    assert_eq!(result.ctor_parameters.unwrap().parameters.len(), 1);
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_downlevel_decorators_config() {
    let config = DownlevelDecoratorsConfig {
        is_core: true,
        is_closure_compiler_enabled: true,
    };
    
    assert!(config.is_core);
    assert!(config.is_closure_compiler_enabled);
}

#[test]
fn test_downlevel_decorators_config_default() {
    let config = DownlevelDecoratorsConfig {
        is_core: false,
        is_closure_compiler_enabled: false,
    };
    
    assert!(!config.is_core);
    assert!(!config.is_closure_compiler_enabled);
}
