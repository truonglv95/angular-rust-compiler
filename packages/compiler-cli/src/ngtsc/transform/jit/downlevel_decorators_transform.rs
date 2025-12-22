// Downlevel Decorators Transform
//
// A transform for downleveling Angular decorators and Angular-decorated class constructor
// parameters for dependency injection. This transform can be used for JIT-mode compilation
// where constructor parameters and associated Angular decorators should be downleveled so
// that apps are not exposed to the ES2015 temporal dead zone limitation in TypeScript.
//
// Based on tsickle's decorator_downlevel_transformer.

use std::collections::HashMap;
use ts::Diagnostic;
use crate::ngtsc::reflection::Decorator;

/// JSDoc type for decorator invocation arrays (for Closure Compiler).
const DECORATOR_INVOCATION_JSDOC_TYPE: &str = "!Array<{type: !Function, args: (undefined|!Array<?>)}>";

// ============================================================================
// Types
// ============================================================================

/// Information about a single constructor parameter's decorators and type.
#[derive(Debug, Clone)]
pub struct ParameterDecorationInfo {
    /// The type declaration for the parameter. Only set if the type is a value
    /// (e.g. a class, not an interface).
    pub type_ref: Option<String>,
    /// The list of decorators found on the parameter.
    pub decorators: Vec<DecoratorMetadata>,
}

/// Extracted decorator metadata.
#[derive(Debug, Clone)]
pub struct DecoratorMetadata {
    /// The decorator type/function name.
    pub type_name: String,
    /// Arguments passed to the decorator, if any.
    pub args: Option<Vec<String>>,
}

/// Configuration for the downlevel decorators transform.
#[derive(Debug, Clone)]
pub struct DownlevelDecoratorsConfig {
    /// Whether this is the Angular core package.
    pub is_core: bool,
    /// Whether to add Closure Compiler annotations.
    pub is_closure_compiler_enabled: bool,
}

impl Default for DownlevelDecoratorsConfig {
    fn default() -> Self {
        Self {
            is_core: false,
            is_closure_compiler_enabled: false,
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Whether a given decorator should be treated as an Angular decorator.
/// Either it's used in @angular/core, or it's imported from there.
pub fn is_angular_decorator(decorator: &Decorator, is_core: bool) -> bool {
    if is_core {
        return true;
    }
    
    if let Some(import) = &decorator.import {
        return import.from == "@angular/core";
    }
    
    false
}

/// Extracts the type of the decorator (the function or expression invoked), as well as all the
/// arguments passed to the decorator.
///
/// Returns metadata in the form: { type: decorator, args: [arg1, arg2] }
pub fn extract_metadata_from_single_decorator(
    decorator: &Decorator,
    _diagnostics: &mut Vec<Diagnostic>,
) -> DecoratorMetadata {
    DecoratorMetadata {
        type_name: decorator.name.clone(),
        // Simplified: just count args, actual stringification would need AST printing
        args: decorator.args.as_ref().map(|args| {
            args.iter().map(|_| "<expr>".to_string()).collect()
        }),
    }
}

/// Creates the static 'ctorParameters' property containing downleveled decorator information.
///
/// The property contains an arrow function that returns an array of object literals of the shape:
/// ```javascript
/// static ctorParameters = () => [{
///   type: SomeClass|undefined,  // the type of the param that's decorated, if it's a value.
///   decorators: [{
///     type: DecoratorFn,  // the type of the decorator that's invoked.
///     args: [ARGS],       // the arguments passed to the decorator.
///   }]
/// }];
/// ```
pub fn create_ctor_parameters_class_property(
    ctor_parameters: &[ParameterDecorationInfo],
    is_closure_compiler_enabled: bool,
) -> CtorParametersProperty {
    CtorParametersProperty {
        parameters: ctor_parameters.to_vec(),
        closure_annotation: if is_closure_compiler_enabled {
            Some(format!(
                "/**\n * @type {{function(): !Array<(null|{{\n *   type: ?,\n *   decorators: (undefined|{}),\n * }})>}}\n * @nocollapse\n */",
                DECORATOR_INVOCATION_JSDOC_TYPE
            ))
        } else {
            None
        },
    }
}

/// Creates the static 'propDecorators' property containing type information for every
/// property that has a decorator applied.
///
/// ```javascript
/// static propDecorators: {[key: string]: {type: Function, args?: any[]}[]} = {
///   propA: [{type: MyDecorator, args: [1, 2]}, ...],
///   ...
/// };
/// ```
pub fn create_prop_decorators_class_property(
    properties: &HashMap<String, Vec<DecoratorMetadata>>,
    is_closure_compiler_enabled: bool,
) -> PropDecoratorsProperty {
    PropDecoratorsProperty {
        properties: properties.clone(),
        closure_annotation: if is_closure_compiler_enabled {
            Some(format!("/** @type {{!Object<string, {}>}} */", DECORATOR_INVOCATION_JSDOC_TYPE))
        } else {
            None
        },
    }
}

// ============================================================================
// Output Structures
// ============================================================================

/// Represents the generated ctorParameters static property.
#[derive(Debug, Clone)]
pub struct CtorParametersProperty {
    /// The constructor parameter information.
    pub parameters: Vec<ParameterDecorationInfo>,
    /// Optional Closure Compiler JSDoc annotation.
    pub closure_annotation: Option<String>,
}

/// Represents the generated propDecorators static property.
#[derive(Debug, Clone)]
pub struct PropDecoratorsProperty {
    /// Map of property name to decorator metadata.
    pub properties: HashMap<String, Vec<DecoratorMetadata>>,
    /// Optional Closure Compiler JSDoc annotation.
    pub closure_annotation: Option<String>,
}

// ============================================================================
// Transform Factory
// ============================================================================

/// Result of transforming a class for decorator downleveling.
#[derive(Debug, Clone)]
pub struct DownleveledClass {
    /// The ctorParameters property to add, if any.
    pub ctor_parameters: Option<CtorParametersProperty>,
    /// The propDecorators property to add, if any.
    pub prop_decorators: Option<PropDecoratorsProperty>,
    /// Diagnostics generated during transformation.
    pub diagnostics: Vec<Diagnostic>,
}

/// Gets a transformer for downleveling Angular constructor parameter and property decorators.
///
/// Note that Angular class decorators are never processed as those rely on side effects that
/// would otherwise no longer be executed. i.e. the creation of a component definition.
pub struct DownlevelDecoratorsTransform {
    /// Configuration for the transform.
    config: DownlevelDecoratorsConfig,
    /// Collected diagnostics.
    diagnostics: Vec<Diagnostic>,
}

impl DownlevelDecoratorsTransform {
    /// Create a new downlevel decorators transform.
    pub fn new(config: DownlevelDecoratorsConfig) -> Self {
        Self {
            config,
            diagnostics: Vec::new(),
        }
    }

    /// Transform a class declaration.
    ///
    /// Returns the additional properties to add to the class.
    pub fn transform_class(&mut self, class_name: &str) -> DownleveledClass {
        let _ = class_name;
        // TODO: Implement class transformation
        // 1. Analyze constructor parameters and collect decorator info
        // 2. Analyze property decorators
        // 3. Create ctorParameters property if needed
        // 4. Create propDecorators property if needed
        
        DownleveledClass {
            ctor_parameters: None,
            prop_decorators: None,
            diagnostics: std::mem::take(&mut self.diagnostics),
        }
    }

    /// Check if a decorator is an Angular decorator that should be downleveled.
    pub fn is_angular_decorator(&self, decorator: &Decorator) -> bool {
        is_angular_decorator(decorator, self.config.is_core)
    }
}

/// Create a downlevel decorators transform.
pub fn get_downlevel_decorators_transform(
    is_core: bool,
    is_closure_compiler_enabled: bool,
) -> DownlevelDecoratorsTransform {
    DownlevelDecoratorsTransform::new(
        DownlevelDecoratorsConfig {
            is_core,
            is_closure_compiler_enabled,
        },
    )
}
