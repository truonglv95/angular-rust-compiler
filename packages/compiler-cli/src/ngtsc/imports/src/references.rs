//! References - Reference type for tracking AST node references
//!
//! A Reference is a pointer to a `ts.Node` that was extracted from the program somehow.
//! It contains not only the node itself, but the information regarding how the node was located.
//! In particular, it might track different identifiers by which the node is exposed, as well as
//! potentially a module specifier which might expose the node.
//!
//! The Angular compiler uses `Reference`s instead of `ts.Node`s when tracking classes or
//! generating imports.
//!
//! Matches: angular/packages/compiler-cli/src/ngtsc/imports/src/references.ts

use angular_compiler::output::output_ast::Expression;
use oxc_ast::ast as oxc_ast;
use std::path::PathBuf;

/// Sentinel value indicating an ambient import.
/// Matches TypeScript's `AmbientImport` from reflection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmbientImport;

/// Information about the module that owns a particular reference.
/// Matches TypeScript's `OwningModule` interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwningModule {
    /// The module specifier (e.g., "@angular/core").
    pub specifier: String,
    /// The resolution context (usually the file path where the import was found).
    pub resolution_context: String,
}

impl OwningModule {
    pub fn new(specifier: impl Into<String>, resolution_context: impl Into<String>) -> Self {
        Self {
            specifier: specifier.into(),
            resolution_context: resolution_context.into(),
        }
    }
}

/// A reference to a TypeScript node.
///
/// A `Reference` is a pointer to a `ts.Node` that was extracted from the program somehow. It
/// contains not only the node itself, but the information regarding how the node was located. In
/// particular, it might track different identifiers by which the node is exposed, as well as
/// potentially a module specifier which might expose the node.
///
/// The Angular compiler uses `Reference`s instead of `ts.Node`s when tracking classes or generating
/// imports.
///
/// The lifetime `'a` is tied to the OXC AST allocator when a node reference is present.
/// Matches TypeScript's `Reference<T extends ts.Node = ts.Node>` class.
#[derive(Debug)]
pub struct Reference<'a> {
    /// The AST node being referenced (optional - may be None when node is not available).
    /// Matches TypeScript's `readonly node: T`.
    pub node: Option<&'a oxc_ast::Class<'a>>,
    
    /// The compiler's best guess at an absolute module specifier which owns this `Reference`.
    ///
    /// This is usually determined by tracking the import statements which led the compiler to a given
    /// node. If any of these imports are absolute, it's an indication that the node being imported
    /// might come from that module.
    ///
    /// It is not _guaranteed_ that the node in question is exported from its `bestGuessOwningModule` -
    /// that is mostly a convention that applies in certain package formats.
    ///
    /// If `bestGuessOwningModule` is `None`, then it's likely the node came from the current program.
    pub best_guess_owning_module: Option<OwningModule>,
    
    /// Indicates that the Reference was created synthetically, not as a result of natural value
    /// resolution.
    ///
    /// This is used to avoid misinterpreting the Reference in certain contexts.
    pub synthetic: bool,
    
    /// Whether this reference is an ambient import.
    pub is_ambient: bool,
    
    /// Alias expression for this reference, if any.
    alias: Option<Box<Expression>>,
    
    /// Known identifiers that can be used to refer to this node.
    /// In TypeScript this is `ts.Identifier[]`.
    identifiers: Vec<String>,
    
    /// Source file path where the node is defined.
    pub source_file: Option<PathBuf>,
    
    /// Cached name for when node is not available.
    name: String,
}

impl<'a> Reference<'a> {
    /// Create a new Reference from a class node.
    /// Matches TypeScript constructor: `constructor(readonly node: T, bestGuessOwningModule: OwningModule | AmbientImport | null = null)`
    pub fn new(node: &'a oxc_ast::Class<'a>) -> Self {
        let name = node.id.as_ref().map(|id| id.name.to_string()).unwrap_or_default();
        let identifiers = if name.is_empty() { vec![] } else { vec![name.clone()] };
        
        Self {
            node: Some(node),
            best_guess_owning_module: None,
            synthetic: false,
            is_ambient: false,
            alias: None,
            identifiers,
            source_file: None,
            name,
        }
    }
    
    /// Create a Reference from name and source file (when node is not available).
    /// This is useful for contexts where the AST is not accessible.
    pub fn from_name(name: impl Into<String>, source_file: Option<PathBuf>) -> Self {
        let name = name.into();
        let identifiers = if name.is_empty() { vec![] } else { vec![name.clone()] };
        
        Self {
            node: None,
            best_guess_owning_module: None,
            synthetic: false,
            is_ambient: false,
            alias: None,
            identifiers,
            source_file,
            name,
        }
    }
    
    /// Create a new Reference with an owning module.
    pub fn with_owning_module(node: &'a oxc_ast::Class<'a>, owning_module: OwningModule) -> Self {
        let name = node.id.as_ref().map(|id| id.name.to_string()).unwrap_or_default();
        let identifiers = if name.is_empty() { vec![] } else { vec![name.clone()] };
        
        Self {
            node: Some(node),
            best_guess_owning_module: Some(owning_module),
            synthetic: false,
            is_ambient: false,
            alias: None,
            identifiers,
            source_file: None,
            name,
        }
    }
    
    /// Create an ambient Reference (from an ambient import).
    pub fn ambient(node: &'a oxc_ast::Class<'a>) -> Self {
        let name = node.id.as_ref().map(|id| id.name.to_string()).unwrap_or_default();
        let identifiers = if name.is_empty() { vec![] } else { vec![name.clone()] };
        
        Self {
            node: Some(node),
            best_guess_owning_module: None,
            synthetic: false,
            is_ambient: true,
            alias: None,
            identifiers,
            source_file: None,
            name,
        }
    }
    
    /// The best guess at which module specifier owns this particular reference, or `None` if there
    /// isn't one.
    /// Matches TypeScript getter: `get ownedByModuleGuess(): string | null`
    pub fn owned_by_module_guess(&self) -> Option<&str> {
        self.best_guess_owning_module.as_ref().map(|m| m.specifier.as_str())
    }
    
    /// Whether this reference has a potential owning module or not.
    /// Matches TypeScript getter: `get hasOwningModuleGuess(): boolean`
    pub fn has_owning_module_guess(&self) -> bool {
        self.best_guess_owning_module.is_some()
    }
    
    /// A name for the node, if one is available.
    ///
    /// This is only suited for debugging. Any actual references to this node should be made with
    /// `ts.Identifier`s (see `getIdentityIn`).
    /// Matches TypeScript getter: `get debugName(): string | null`
    pub fn debug_name(&self) -> &str {
        &self.name
    }
    
    /// Get the alias expression, if any.
    /// Matches TypeScript getter: `get alias(): Expression | null`
    pub fn alias(&self) -> Option<&Expression> {
        self.alias.as_deref()
    }
    
    /// Record an identifier by which it's valid to refer to this node, within the context of this
    /// `Reference`.
    /// Matches TypeScript: `addIdentifier(identifier: ts.Identifier): void`
    pub fn add_identifier(&mut self, identifier: impl Into<String>) {
        self.identifiers.push(identifier.into());
    }
    
    /// Get an identifier within this `Reference` that can be used to refer within the context of a
    /// given source file, if any.
    /// Matches TypeScript: `getIdentityIn(context: ts.SourceFile): ts.Identifier | null`
    pub fn get_identity_in(&self, context_source_file: &str) -> Option<&str> {
        let is_same_file = self.source_file
            .as_ref()
            .map(|sf| sf.to_string_lossy() == context_source_file)
            .unwrap_or(false);
            
        if is_same_file && !self.identifiers.is_empty() {
            Some(&self.identifiers[0])
        } else if !self.identifiers.is_empty() {
            // Fallback: return first identifier if available
            Some(&self.identifiers[0])
        } else {
            None
        }
    }
    
    /// Clone this reference with a new alias.
    /// Matches TypeScript: `cloneWithAlias(alias: Expression): Reference<T>`
    pub fn clone_with_alias(&self, alias: Expression) -> Self {
        Self {
            node: self.node,
            best_guess_owning_module: self.best_guess_owning_module.clone(),
            synthetic: self.synthetic,
            is_ambient: self.is_ambient,
            alias: Some(Box::new(alias)),
            identifiers: self.identifiers.clone(),
            source_file: self.source_file.clone(),
            name: self.name.clone(),
        }
    }
    
    /// Clone this reference without identifiers.
    /// Matches TypeScript: `cloneWithNoIdentifiers(): Reference<T>`
    pub fn clone_with_no_identifiers(&self) -> Self {
        Self {
            node: self.node,
            best_guess_owning_module: self.best_guess_owning_module.clone(),
            synthetic: self.synthetic,
            is_ambient: self.is_ambient,
            alias: self.alias.clone(),
            identifiers: Vec::new(),
            source_file: self.source_file.clone(),
            name: self.name.clone(),
        }
    }
    
    /// Get source file as string for compatibility.
    pub fn source_file_str(&self) -> Option<String> {
        self.source_file.as_ref().map(|p| p.to_string_lossy().to_string())
    }
}

impl<'a> Clone for Reference<'a> {
    fn clone(&self) -> Self {
        Self {
            node: self.node,
            best_guess_owning_module: self.best_guess_owning_module.clone(),
            synthetic: self.synthetic,
            is_ambient: self.is_ambient,
            alias: self.alias.clone(),
            identifiers: self.identifiers.clone(),
            source_file: self.source_file.clone(),
            name: self.name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owning_module() {
        let module = OwningModule::new("@angular/core", "/path/to/file.ts");
        assert_eq!(module.specifier, "@angular/core");
        assert_eq!(module.resolution_context, "/path/to/file.ts");
    }

    #[test]
    fn test_reference_from_name() {
        let reference = Reference::from_name(
            "MyComponent",
            Some(PathBuf::from("/path/to/component.ts")),
        );
        
        assert_eq!(reference.debug_name(), "MyComponent");
        assert!(!reference.has_owning_module_guess());
        assert!(!reference.synthetic);
        assert!(!reference.is_ambient);
    }

    #[test]
    fn test_reference_with_owning_module() {
        let reference: Reference<'static> = {
            let mut r = Reference::from_name("Injectable", None);
            r.best_guess_owning_module = Some(OwningModule::new("@angular/core", "/path/to/file.ts"));
            r
        };
        
        assert!(reference.has_owning_module_guess());
        assert_eq!(reference.owned_by_module_guess(), Some("@angular/core"));
    }

    #[test]
    fn test_reference_clone_with_no_identifiers() {
        let reference = Reference::from_name("MyComponent", None);
        let cloned = reference.clone_with_no_identifiers();
        assert_eq!(cloned.debug_name(), "MyComponent");
    }
}
