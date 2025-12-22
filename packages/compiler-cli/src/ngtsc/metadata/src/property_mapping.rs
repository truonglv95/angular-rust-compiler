//! Metadata property mapping utilities.
//!
//! This module handles mapping of @Input() and @Output() decorators to class properties.
//! Matches TypeScript's property_mapping.ts

use indexmap::IndexMap;

/// Represents an input or output property mapping.
#[derive(Debug, Clone)]
pub struct InputOrOutput {
    pub class_property_name: String,
    pub binding_property_name: String,
    pub is_signal: bool,
}

/// A mapping of class properties to their Angular bindings (inputs or outputs).
/// Uses IndexMap to preserve insertion order for deterministic output.
#[derive(Debug, Clone, Default)]
pub struct ClassPropertyMapping {
    entries: IndexMap<String, InputOrOutput>,
}

impl ClassPropertyMapping {
    pub fn new() -> Self {
        Self {
            entries: IndexMap::new(),
        }
    }

    pub fn insert(&mut self, entry: InputOrOutput) {
        self.entries.insert(entry.class_property_name.clone(), entry);
    }

    pub fn get(&self, class_property_name: &str) -> Option<&InputOrOutput> {
        self.entries.get(class_property_name)
    }

    /// Alias for `get` - lookup by class property name.
    pub fn get_by_class_property_name(&self, class_property_name: &str) -> Option<&InputOrOutput> {
        self.get(class_property_name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &InputOrOutput)> {
        self.entries.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Convert to a Vec of (binding_name, class_property_name) tuples for codegen.
    /// Preserves insertion order (no sorting needed).
    pub fn to_binding_vec(&self) -> Vec<(String, String)> {
        self.entries
            .values()
            .map(|v| (v.binding_property_name.clone(), v.class_property_name.clone()))
            .collect()
    }
}

/// Type alias for class property names.
pub type ClassPropertyName = String;

