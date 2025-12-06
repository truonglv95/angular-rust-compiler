//! Element Schema Registry
//!
//! Corresponds to packages/compiler/src/schema/element_schema_registry.ts (30 lines)

use crate::core::{SchemaMetadata, SecurityContext};

/// Validation result for properties/attributes
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub error: bool,
    pub msg: Option<String>,
}

/// Normalization result for animation styles
#[derive(Debug, Clone)]
pub struct NormalizationResult {
    pub error: String,
    pub value: String,
}

/// Abstract base class for element schema registries
pub trait ElementSchemaRegistry {
    /// Check if a property exists on an element
    fn has_property(&self, tag_name: &str, prop_name: &str, schema_metas: &[SchemaMetadata]) -> bool;

    /// Check if an element exists
    fn has_element(&self, tag_name: &str, schema_metas: &[SchemaMetadata]) -> bool;

    /// Get security context for a property
    fn security_context(&self, element_name: &str, prop_name: &str, is_attribute: bool) -> SecurityContext;

    /// Get all known element names
    fn all_known_element_names(&self) -> Vec<String>;

    /// Get mapped property name
    fn get_mapped_prop_name(&self, prop_name: &str) -> String;

    /// Get default component element name
    fn get_default_component_element_name(&self) -> String;

    /// Validate property name
    fn validate_property(&self, name: &str) -> ValidationResult;

    /// Validate attribute name
    fn validate_attribute(&self, name: &str) -> ValidationResult;

    /// Normalize animation style property
    fn normalize_animation_style_property(&self, prop_name: &str) -> String;

    /// Normalize animation style value
    fn normalize_animation_style_value(
        &self,
        camel_case_prop: &str,
        user_provided_prop: &str,
        val: &str,
    ) -> NormalizationResult;
}

