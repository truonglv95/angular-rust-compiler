// Schema Utilities
//
// Extract and validate schemas from module/component metadata.

/// Schema metadata for custom elements validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaMetadata {
    /// CUSTOM_ELEMENTS_SCHEMA - allows custom elements.
    CustomElements,
    /// NO_ERRORS_SCHEMA - suppresses all template errors.
    NoErrors,
}

impl SchemaMetadata {
    /// Get the schema name as used in Angular.
    pub fn name(&self) -> &'static str {
        match self {
            SchemaMetadata::CustomElements => "CUSTOM_ELEMENTS_SCHEMA",
            SchemaMetadata::NoErrors => "NO_ERRORS_SCHEMA",
        }
    }
    
    /// Parse a schema from its name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "CUSTOM_ELEMENTS_SCHEMA" => Some(SchemaMetadata::CustomElements),
            "NO_ERRORS_SCHEMA" => Some(SchemaMetadata::NoErrors),
            _ => None,
        }
    }
}

/// Error when extracting schemas.
#[derive(Debug, Clone)]
pub struct SchemaError {
    pub message: String,
}

impl SchemaError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

/// Extract schemas from a list of schema names.
pub fn extract_schemas(
    schema_names: &[String],
    context: &str,
) -> Result<Vec<SchemaMetadata>, SchemaError> {
    let mut schemas = Vec::new();
    
    for name in schema_names {
        match SchemaMetadata::from_name(name) {
            Some(schema) => schemas.push(schema),
            None => {
                return Err(SchemaError::new(format!(
                    "'{}' is not a valid {} schema",
                    name, context
                )));
            }
        }
    }
    
    Ok(schemas)
}

/// Check if a list of schemas contains CUSTOM_ELEMENTS_SCHEMA.
pub fn has_custom_elements_schema(schemas: &[SchemaMetadata]) -> bool {
    schemas.contains(&SchemaMetadata::CustomElements)
}

/// Check if a list of schemas contains NO_ERRORS_SCHEMA.
pub fn has_no_errors_schema(schemas: &[SchemaMetadata]) -> bool {
    schemas.contains(&SchemaMetadata::NoErrors)
}
