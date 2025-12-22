// Declaration Transform - DTS file transformation utilities
//
// This module provides utilities for transforming .d.ts declaration files
// to add static field declarations with types.

use std::collections::HashMap;
use crate::ngtsc::transform::src::api::{
    DtsTransform, ReferenceEmitter, ImportManager, ReflectionHost,
};

// ============================================================================
// DTS Transform Registry
// ============================================================================

/// Keeps track of `DtsTransform`s per source file, so that it is known which source
/// files need to have their declaration file transformed.
pub struct DtsTransformRegistry {
    /// Map from source file path to Ivy declaration transforms.
    ivy_declaration_transforms: HashMap<String, IvyDeclarationDtsTransform>,
}

impl DtsTransformRegistry {
    /// Create a new registry.
    pub fn new() -> Self {
        Self {
            ivy_declaration_transforms: HashMap::new(),
        }
    }

    /// Get or create an Ivy declaration transform for the given source file.
    pub fn get_ivy_declaration_transform(&mut self, sf_path: &str) -> &mut IvyDeclarationDtsTransform {
        self.ivy_declaration_transforms
            .entry(sf_path.to_string())
            .or_insert_with(IvyDeclarationDtsTransform::new)
    }

    /// Get all transforms for a source file, or None if no transforms are needed.
    ///
    /// Note: Due to how TypeScript afterDeclarations transformers work, the source file
    /// path is the same as the original .ts. We check `is_declaration_file` to determine
    /// if it's actually a declaration file.
    pub fn get_all_transforms(&self, sf_path: &str, is_declaration_file: bool) -> Option<Vec<&dyn DtsTransform>> {
        // No need to transform if it's not a declarations file
        if !is_declaration_file {
            return None;
        }

        let mut transforms: Vec<&dyn DtsTransform> = Vec::new();
        
        if let Some(ivy_transform) = self.ivy_declaration_transforms.get(sf_path) {
            transforms.push(ivy_transform);
        }

        if transforms.is_empty() {
            None
        } else {
            Some(transforms)
        }
    }

    /// Check if any transforms are registered for any file.
    pub fn has_transforms(&self) -> bool {
        !self.ivy_declaration_transforms.is_empty()
    }
}

impl Default for DtsTransformRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Ivy Declaration Field
// ============================================================================

/// A field to be added to a class declaration in .d.ts.
#[derive(Debug, Clone)]
pub struct IvyDeclarationField {
    /// The name of the field.
    pub name: String,
    /// The type of the field (as a string representation).
    pub type_str: String,
}

impl IvyDeclarationField {
    pub fn new(name: impl Into<String>, type_str: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_str: type_str.into(),
        }
    }
}

// ============================================================================
// Ivy Declaration DTS Transform
// ============================================================================

/// Transform for adding Ivy static field declarations to .d.ts files.
pub struct IvyDeclarationDtsTransform {
    /// Map from class declaration identifier to fields to add.
    /// Key is the class name (used as identifier).
    declaration_fields: HashMap<String, Vec<IvyDeclarationField>>,
}

impl IvyDeclarationDtsTransform {
    /// Create a new transform.
    pub fn new() -> Self {
        Self {
            declaration_fields: HashMap::new(),
        }
    }

    /// Add fields to be declared for a class.
    pub fn add_fields(&mut self, class_name: &str, fields: Vec<IvyDeclarationField>) {
        self.declaration_fields
            .entry(class_name.to_string())
            .or_default()
            .extend(fields);
    }

    /// Get the fields registered for a class.
    pub fn get_fields(&self, class_name: &str) -> Option<&Vec<IvyDeclarationField>> {
        self.declaration_fields.get(class_name)
    }

    /// Check if any fields are registered for any class.
    pub fn has_fields(&self) -> bool {
        !self.declaration_fields.is_empty()
    }
}

impl Default for IvyDeclarationDtsTransform {
    fn default() -> Self {
        Self::new()
    }
}

impl DtsTransform for IvyDeclarationDtsTransform {
    fn transform_class(
        &self,
        _clazz: &oxc_ast::ast::Class<'_>,
        _elements: &[oxc_ast::ast::ClassElement<'_>],
        _reflector: &dyn ReflectionHost,
        _ref_emitter: &ReferenceEmitter,
        _imports: &mut ImportManager,
    ) -> Option<oxc_ast::ast::Class<'_>> {
        // TODO: Implement actual class transformation
        // This would:
        // 1. Get the original class from the AST
        // 2. Look up registered fields for this class
        // 3. Create new static property declarations
        // 4. Return updated class with new members
        None
    }
}
