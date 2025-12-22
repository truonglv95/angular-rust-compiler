// TypeCheck Checker Implementation
//
// Main template type-checker implementation.

use std::collections::{HashMap, HashSet};
use super::super::api::{
    TemplateTypeChecker, TypeCheckResult, TypeCheckError,
    TypeCheckingConfig, TypeCheckContext,
};
use super::type_check_block::TypeCheckBlockGenerator;

/// Implementation of the template type-checker.
pub struct TemplateTypeCheckerImpl {
    /// Configuration.
    config: TypeCheckingConfig,
    /// Components that have been type-checked.
    checked_components: HashSet<String>,
    /// Cached diagnostics per component.
    cached_diagnostics: HashMap<String, Vec<TypeCheckError>>,
    /// Global context.
    context: TypeCheckContext,
}

impl TemplateTypeCheckerImpl {
    pub fn new(config: TypeCheckingConfig) -> Self {
        Self {
            config,
            checked_components: HashSet::new(),
            cached_diagnostics: HashMap::new(),
            context: TypeCheckContext::new(),
        }
    }
    
    /// Type-check a component.
    pub fn type_check_component(
        &mut self,
        component: &str,
        template: &str,
    ) -> TypeCheckResult {
        if self.checked_components.contains(component) {
            // Return cached result
            let diagnostics = self.cached_diagnostics
                .get(component)
                .cloned()
                .unwrap_or_default();
            return TypeCheckResult {
                success: diagnostics.is_empty(),
                diagnostics,
            };
        }
        
        // Generate type-check block
        let mut generator = TypeCheckBlockGenerator::new(self.config.clone());
        let result = generator.generate(component, template);
        
        let diagnostics = match result {
            Ok(_tcb) => {
                // In a real implementation, we would feed the TCB to TypeScript
                // and collect diagnostics. For now, return empty.
                Vec::new()
            }
            Err(e) => vec![e],
        };
        
        self.checked_components.insert(component.to_string());
        self.cached_diagnostics.insert(component.to_string(), diagnostics.clone());
        
        TypeCheckResult {
            success: diagnostics.is_empty(),
            diagnostics,
        }
    }
}

impl TemplateTypeChecker for TemplateTypeCheckerImpl {
    fn get_diagnostics_for_component(&self, component: &str) -> Vec<TypeCheckError> {
        self.cached_diagnostics
            .get(component)
            .cloned()
            .unwrap_or_default()
    }
    
    fn get_all_diagnostics(&self) -> Vec<TypeCheckError> {
        self.cached_diagnostics
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect()
    }
    
    fn is_type_checked(&self, component: &str) -> bool {
        self.checked_components.contains(component)
    }
    
    fn invalidate(&mut self, component: &str) {
        self.checked_components.remove(component);
        self.cached_diagnostics.remove(component);
    }
    
    fn invalidate_all(&mut self) {
        self.checked_components.clear();
        self.cached_diagnostics.clear();
    }
}
