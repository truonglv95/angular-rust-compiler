// Injectable Registry
//
// Registry for tracking classes that can be constructed via dependency injection.

use std::collections::HashMap;
use super::factory::R3DependencyMetadata;

/// Metadata about an injectable class's constructor dependencies.
#[derive(Debug, Clone)]
pub enum InjectableMeta {
    /// Valid constructor dependencies.
    Valid(Vec<R3DependencyMetadata>),
    /// Invalid constructor (cannot be analyzed).
    Invalid,
    /// No constructor dependencies.
    None,
}

impl InjectableMeta {
    pub fn is_valid(&self) -> bool {
        matches!(self, InjectableMeta::Valid(_) | InjectableMeta::None)
    }
    
    pub fn get_deps(&self) -> Option<&Vec<R3DependencyMetadata>> {
        match self {
            InjectableMeta::Valid(deps) => Some(deps),
            _ => None,
        }
    }
}

/// Registry that keeps track of classes that can be constructed via dependency injection.
#[derive(Debug, Default)]
pub struct InjectableClassRegistry {
    classes: HashMap<String, InjectableMeta>,
    is_core: bool,
}

impl InjectableClassRegistry {
    pub fn new(is_core: bool) -> Self {
        Self {
            classes: HashMap::new(),
            is_core,
        }
    }
    
    /// Register an injectable class with its metadata.
    pub fn register_injectable(&mut self, class_name: impl Into<String>, meta: InjectableMeta) {
        self.classes.insert(class_name.into(), meta);
    }
    
    /// Get the injectable metadata for a class.
    pub fn get_injectable_meta(&self, class_name: &str) -> Option<&InjectableMeta> {
        self.classes.get(class_name)
    }
    
    /// Check if a class is registered as injectable.
    pub fn is_injectable(&self, class_name: &str) -> bool {
        self.classes.contains_key(class_name)
    }
    
    /// Whether this is compiling @angular/core.
    pub fn is_core(&self) -> bool {
        self.is_core
    }
    
    /// Get all registered classes.
    pub fn get_all_classes(&self) -> impl Iterator<Item = (&String, &InjectableMeta)> {
        self.classes.iter()
    }
}
