// Common Utilities
//
// Various utility functions for annotations processing.

use std::collections::HashSet;

/// Module name of the framework core.
pub const CORE_MODULE: &str = "@angular/core";

/// Decorator metadata.
#[derive(Debug, Clone)]
pub struct Decorator {
    /// Decorator name.
    pub name: String,
    /// Import information if available.
    pub import: Option<Import>,
    /// Decorator arguments.
    pub args: Option<Vec<String>>,
    /// The node identifier.
    pub node: String,
}

/// Import information.
#[derive(Debug, Clone)]
pub struct Import {
    /// The imported name.
    pub name: String,
    /// The module from which it's imported.
    pub from: String,
}

impl Decorator {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            import: None,
            args: None,
            node: String::new(),
        }
    }
    
    pub fn with_import(mut self, from: impl Into<String>) -> Self {
        self.import = Some(Import {
            name: self.name.clone(),
            from: from.into(),
        });
        self
    }
    
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
        self
    }
    
    /// Check if this is an Angular core decorator.
    pub fn is_angular_core(&self) -> bool {
        self.import.as_ref().map_or(false, |i| i.from == CORE_MODULE)
    }
}

/// Check if a decorator is from @angular/core.
pub fn is_angular_core(decorator: &Decorator) -> bool {
    decorator.is_angular_core()
}

/// Check if a reference is from @angular/core with potential aliasing.
pub fn is_angular_core_reference_with_potential_aliasing(
    ref_module: Option<&str>,
    symbol_name: &str,
    actual_name: &str,
    is_core: bool,
) -> bool {
    // If compiling core, reference is from core
    if is_core {
        return true;
    }
    
    // Check if from @angular/core
    if ref_module != Some(CORE_MODULE) {
        return false;
    }
    
    // Account for potential aliasing (internalXxx -> Xxx)
    actual_name == symbol_name || 
        actual_name == format!("internal{}", symbol_name) ||
        actual_name == format!("ɵ{}", symbol_name)
}

/// Find an Angular decorator by name.
pub fn find_angular_decorator<'a>(
    decorators: &'a [Decorator],
    name: &str,
    is_core: bool,
) -> Option<&'a Decorator> {
    decorators.iter().find(|d| is_angular_decorator(d, name, is_core))
}

/// Check if a decorator matches the given name and is from Angular.
pub fn is_angular_decorator(decorator: &Decorator, name: &str, is_core: bool) -> bool {
    if decorator.name != name {
        return false;
    }
    
    if is_core {
        return true;
    }
    
    decorator.is_angular_core()
}

/// Get all Angular decorators matching any of the given names.
pub fn get_angular_decorators<'a>(
    decorators: &'a [Decorator],
    names: &[&str],
    is_core: bool,
) -> Vec<&'a Decorator> {
    decorators
        .iter()
        .filter(|d| names.iter().any(|name| is_angular_decorator(d, name, is_core)))
        .collect()
}

/// Unwrap an expression by removing type casts and parentheses.
pub fn unwrap_expression(expr: &str) -> &str {
    let mut result = expr.trim();
    
    // Remove outer parentheses
    while result.starts_with('(') && result.ends_with(')') {
        result = result[1..result.len()-1].trim();
    }
    
    // Remove "as Type" casts
    if let Some(idx) = result.find(" as ") {
        result = result[..idx].trim();
    }
    
    result
}

/// Try to expand a forwardRef expression.
pub fn expand_forward_ref(expr: &str) -> Option<&str> {
    let trimmed = expr.trim();
    
    // Look for forwardRef(() => Xxx)
    if !trimmed.starts_with("forwardRef") {
        return None;
    }
    
    // Extract inner expression
    if let Some(start) = trimmed.find("=>") {
        let after_arrow = &trimmed[start + 2..];
        if let Some(end) = after_arrow.rfind(')') {
            let inner = after_arrow[..end].trim();
            return Some(inner);
        }
    }
    
    None
}

/// Read base class from a class declaration.
pub fn read_base_class(extends_clause: Option<&str>) -> Option<String> {
    extends_clause.map(|s| s.trim().to_string())
}

/// Check if a forward reference should be wrapped.
pub fn is_expression_forward_reference(
    ref_name: &str,
    context_position: usize,
    ref_position: usize,
) -> bool {
    // Reference is forward if it appears after current position
    ref_position > context_position
}

/// R3 Reference for code generation.
#[derive(Debug, Clone)]
pub struct R3Reference {
    /// Value expression.
    pub value: String,
    /// Type expression.
    pub type_: String,
}

impl R3Reference {
    pub fn new(value: impl Into<String>, type_: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            type_: type_.into(),
        }
    }
    
    pub fn same(expr: impl Into<String>) -> Self {
        let s = expr.into();
        Self {
            value: s.clone(),
            type_: s,
        }
    }
}

/// Convert a reference to an R3Reference.
pub fn to_r3_reference(
    ref_name: &str,
    ref_module: Option<&str>,
) -> R3Reference {
    match ref_module {
        Some(module) => R3Reference::new(
            format!("i0.importExpr({})", ref_name),
            ref_name.to_string(),
        ),
        None => R3Reference::same(ref_name),
    }
}

/// Wrap type reference for a class.
pub fn wrap_type_reference(class_name: &str) -> R3Reference {
    R3Reference::same(class_name)
}

/// Resolve providers that require factory definitions.
pub fn resolve_providers_requiring_factory(
    provider_names: &[String],
) -> HashSet<String> {
    // In full implementation, would analyze providers
    // For now, return empty set
    HashSet::new()
}
