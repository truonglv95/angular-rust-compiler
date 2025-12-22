// Query Functions Transform
//
// Transform for adding query decorators (@ViewChild, @ViewChildren, @ContentChild, @ContentChildren)
// to signal query properties for JIT compatibility.

use crate::ngtsc::imports::ImportedSymbolsTracker;
use super::transform_api::{
    PropertyTransformResult, PropertyInfo, is_query_function_call,
    create_synthetic_angular_core_decorator_access, SyntheticDecorator,
};

/// Transform for query functions (viewChild, viewChildren, contentChild, contentChildren).
///
/// Checks if a class member is a query function call and adds the appropriate
/// decorator for JIT compatibility.
pub fn query_functions_transforms(
    property: &PropertyInfo,
    _import_tracker: &ImportedSymbolsTracker,
    is_core: bool,
) -> PropertyTransformResult {
    // Check if this is a query function call
    if !is_query_function_call(property.value_string.as_deref(), is_core) {
        return PropertyTransformResult::unchanged();
    }

    // Determine which type of query this is and create the appropriate decorator
    let value = property.value_string.as_deref().unwrap_or("");
    
    let decorator = if value.starts_with("viewChild(") {
        create_view_child_decorator(value)
    } else if value.starts_with("viewChildren(") {
        create_view_children_decorator(value)
    } else if value.starts_with("contentChild(") {
        create_content_child_decorator(value)
    } else if value.starts_with("contentChildren(") {
        create_content_children_decorator(value)
    } else {
        return PropertyTransformResult::unchanged();
    };
    
    PropertyTransformResult::with_decorators(vec![decorator])
}

/// Create a @ViewChild decorator.
fn create_view_child_decorator(value: &str) -> SyntheticDecorator {
    let mut decorator = create_synthetic_angular_core_decorator_access("ViewChild");
    
    // Extract the selector from viewChild(selector, options?)
    if let Some(selector) = extract_query_selector(value) {
        decorator = decorator.with_arg(selector);
    }
    
    decorator
}

/// Create a @ViewChildren decorator.
fn create_view_children_decorator(value: &str) -> SyntheticDecorator {
    let mut decorator = create_synthetic_angular_core_decorator_access("ViewChildren");
    
    if let Some(selector) = extract_query_selector(value) {
        decorator = decorator.with_arg(selector);
    }
    
    decorator
}

/// Create a @ContentChild decorator.
fn create_content_child_decorator(value: &str) -> SyntheticDecorator {
    let mut decorator = create_synthetic_angular_core_decorator_access("ContentChild");
    
    if let Some(selector) = extract_query_selector(value) {
        decorator = decorator.with_arg(selector);
    }
    
    decorator
}

/// Create a @ContentChildren decorator.
fn create_content_children_decorator(value: &str) -> SyntheticDecorator {
    let mut decorator = create_synthetic_angular_core_decorator_access("ContentChildren");
    
    if let Some(selector) = extract_query_selector(value) {
        decorator = decorator.with_arg(selector);
    }
    
    decorator
}

/// Extract the selector from a query function call.
/// e.g., viewChild('myRef') -> 'myRef'
/// e.g., viewChild(MyComponent) -> MyComponent
fn extract_query_selector(value: &str) -> Option<String> {
    // Find the opening paren
    let start = value.find('(')?;
    let rest = &value[start + 1..];
    
    // Find the first argument (before comma or closing paren)
    let end = rest.find(|c| c == ',' || c == ')').unwrap_or(rest.len());
    let selector = rest[..end].trim();
    
    if selector.is_empty() {
        None
    } else {
        Some(selector.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_query_selector_string() {
        let value = "viewChild('myRef')";
        assert_eq!(extract_query_selector(value), Some("'myRef'".to_string()));
    }

    #[test]
    fn test_extract_query_selector_class() {
        let value = "viewChild(MyComponent)";
        assert_eq!(extract_query_selector(value), Some("MyComponent".to_string()));
    }

    #[test]
    fn test_extract_query_selector_with_options() {
        let value = "viewChild('myRef', { read: ElementRef })";
        assert_eq!(extract_query_selector(value), Some("'myRef'".to_string()));
    }

    #[test]
    fn test_extract_query_selector_empty() {
        let value = "viewChild()";
        assert_eq!(extract_query_selector(value), None);
    }
}
