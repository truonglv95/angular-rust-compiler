// References Registry
//
// Registry for tracking references found during analysis.

use std::collections::HashSet;

/// Registry for tracking references found by DecoratorHandlers.
pub trait ReferencesRegistry: Send + Sync {
    /// Register one or more references in the registry.
    fn add(&mut self, source: &str, references: &[String]);
}

/// A no-op references registry that does nothing.
#[derive(Debug, Default)]
pub struct NoopReferencesRegistry;

impl NoopReferencesRegistry {
    pub fn new() -> Self {
        Self
    }
}

impl ReferencesRegistry for NoopReferencesRegistry {
    fn add(&mut self, _source: &str, _references: &[String]) {
        // Do nothing
    }
}

/// A registry that collects all references.
#[derive(Debug, Default)]
pub struct CollectingReferencesRegistry {
    references: HashSet<String>,
}

impl CollectingReferencesRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn get_references(&self) -> &HashSet<String> {
        &self.references
    }
    
    pub fn into_references(self) -> HashSet<String> {
        self.references
    }
}

impl ReferencesRegistry for CollectingReferencesRegistry {
    fn add(&mut self, _source: &str, references: &[String]) {
        for reference in references {
            self.references.insert(reference.clone());
        }
    }
}
