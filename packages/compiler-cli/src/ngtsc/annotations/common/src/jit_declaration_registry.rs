// JIT Declaration Registry
//
// Registry for tracking Angular declarations marked for JIT compilation.

use std::collections::HashSet;

/// Registry that keeps track of Angular declarations that are explicitly
/// marked for JIT compilation and are skipping compilation by trait handlers.
#[derive(Debug, Default)]
pub struct JitDeclarationRegistry {
    /// Set of class names that are marked for JIT compilation.
    jit_declarations: HashSet<String>,
}

impl JitDeclarationRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Mark a class as a JIT declaration.
    pub fn register(&mut self, class_name: impl Into<String>) {
        self.jit_declarations.insert(class_name.into());
    }
    
    /// Check if a class is registered as a JIT declaration.
    pub fn is_jit_declaration(&self, class_name: &str) -> bool {
        self.jit_declarations.contains(class_name)
    }
    
    /// Get all registered JIT declarations.
    pub fn get_declarations(&self) -> &HashSet<String> {
        &self.jit_declarations
    }
    
    /// Get the number of registered JIT declarations.
    pub fn len(&self) -> usize {
        self.jit_declarations.len()
    }
    
    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.jit_declarations.is_empty()
    }
}
