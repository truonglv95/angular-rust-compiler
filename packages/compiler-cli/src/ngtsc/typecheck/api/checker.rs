// TypeCheck Checker API
//
// Template type-checker interface.

use super::api::TypeCheckError;

/// Interface for requesting type-checking for a component.
pub trait TemplateTypeChecker {
    /// Get diagnostics for a component.
    fn get_diagnostics_for_component(&self, component: &str) -> Vec<TypeCheckError>;
    
    /// Get all diagnostics.
    fn get_all_diagnostics(&self) -> Vec<TypeCheckError>;
    
    /// Check if a component has been type-checked.
    fn is_type_checked(&self, component: &str) -> bool;
    
    /// Invalidate a component, forcing re-type-check.
    fn invalidate(&mut self, component: &str);
    
    /// Invalidate all components.
    fn invalidate_all(&mut self);
}

/// Result of template type-checking.
#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    /// Whether type-checking succeeded.
    pub success: bool,
    /// Diagnostics produced.
    pub diagnostics: Vec<TypeCheckError>,
}

impl TypeCheckResult {
    pub fn success() -> Self {
        Self {
            success: true,
            diagnostics: Vec::new(),
        }
    }
    
    pub fn failure(diagnostics: Vec<TypeCheckError>) -> Self {
        Self {
            success: false,
            diagnostics,
        }
    }
}
