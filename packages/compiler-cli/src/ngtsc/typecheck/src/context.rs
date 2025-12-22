// TypeCheck Context Implementation
//
// Manages type-check block generation context.

use std::collections::HashMap;
use super::super::api::{TypeCheckError, TypeCheckingConfig};

/// Context for generating type-check blocks for a file.
pub struct TypeCheckingContext {
    /// Configuration.
    config: TypeCheckingConfig,
    /// File being type-checked.
    file: String,
    /// Generated type-check block code.
    generated_code: Vec<String>,
    /// Mapping from component to TCB.
    component_to_tcb: HashMap<String, String>,
    /// Errors.
    errors: Vec<TypeCheckError>,
}

impl TypeCheckingContext {
    pub fn new(config: TypeCheckingConfig, file: impl Into<String>) -> Self {
        Self {
            config,
            file: file.into(),
            generated_code: Vec::new(),
            component_to_tcb: HashMap::new(),
            errors: Vec::new(),
        }
    }
    
    /// Get the configuration.
    pub fn config(&self) -> &TypeCheckingConfig {
        &self.config
    }
    
    /// Add a type-check block for a component.
    pub fn add_type_check_block(&mut self, component: impl Into<String>, tcb: impl Into<String>) {
        let comp = component.into();
        let code = tcb.into();
        self.component_to_tcb.insert(comp, code.clone());
        self.generated_code.push(code);
    }
    
    /// Add an error.
    pub fn add_error(&mut self, error: TypeCheckError) {
        self.errors.push(error);
    }
    
    /// Get all generated code.
    pub fn generated_code(&self) -> &[String] {
        &self.generated_code
    }
    
    /// Get all errors.
    pub fn errors(&self) -> &[TypeCheckError] {
        &self.errors
    }
    
    /// Get TCB for a component.
    pub fn get_tcb(&self, component: &str) -> Option<&String> {
        self.component_to_tcb.get(component)
    }
}

/// Environment for type-checking.
pub struct TypeCheckEnvironment {
    /// DOM schema registry.
    dom_schema: HashMap<String, Vec<String>>,
}

impl TypeCheckEnvironment {
    pub fn new() -> Self {
        Self {
            dom_schema: Self::create_dom_schema(),
        }
    }
    
    /// Check if an element is a valid DOM element.
    pub fn is_valid_element(&self, tag: &str) -> bool {
        self.dom_schema.contains_key(tag)
    }
    
    /// Get valid attributes for an element.
    pub fn get_valid_attributes(&self, tag: &str) -> Vec<String> {
        self.dom_schema.get(tag).cloned().unwrap_or_default()
    }
    
    fn create_dom_schema() -> HashMap<String, Vec<String>> {
        let mut schema = HashMap::new();
        // Common elements
        schema.insert("div".to_string(), vec!["class", "id", "style"].iter().map(|s| s.to_string()).collect());
        schema.insert("span".to_string(), vec!["class", "id", "style"].iter().map(|s| s.to_string()).collect());
        schema.insert("button".to_string(), vec!["class", "id", "type", "disabled"].iter().map(|s| s.to_string()).collect());
        schema.insert("input".to_string(), vec!["class", "id", "type", "value", "placeholder", "disabled"].iter().map(|s| s.to_string()).collect());
        schema
    }
}

impl Default for TypeCheckEnvironment {
    fn default() -> Self {
        Self::new()
    }
}
