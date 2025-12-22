//! Core metadata API types matching TypeScript api.ts
//!
//! This module defines the core metadata types used throughout the Angular compiler.
//! Matches: angular/packages/compiler-cli/src/ngtsc/metadata/src/api.ts

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use super::property_mapping::ClassPropertyMapping;
use angular_compiler::ml_parser::ast::Node as HtmlNode;

/// Discriminant for different kinds of compiler metadata objects.
/// Matches TypeScript's MetaKind enum from api.ts (L128-133)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaKind {
    Directive,
    Pipe,
    NgModule,
}

/// Possible ways that a directive can be matched.
/// Matches TypeScript's MatchSource enum from api.ts (L140-147)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchSource {
    /// The directive was matched by its selector.
    #[default]
    Selector,
    /// The directive was applied as a host directive.
    HostDirective,
}

/// Metadata for an `@Input()` transform function.
/// Matches TypeScript's DecoratorInputTransform (L180-193)
#[derive(Debug, Clone)]
pub struct DecoratorInputTransform {
    pub node: String,  // AST node placeholder
    pub type_ref: String,
}

/// Metadata for a single input mapping.
/// Matches TypeScript's InputMapping (L149-165)
#[derive(Debug, Clone)]
pub struct InputMapping {
    pub required: bool,
    pub transform: Option<DecoratorInputTransform>,
}

/// Typing metadata collected for a directive within an NgModule's scope.
/// Matches TypeScript's DirectiveTypeCheckMeta interface (L82-126)
#[derive(Debug, Clone, Default)]
pub struct DirectiveTypeCheckMeta {
    /// List of static `ngTemplateGuard_xx` members found on the Directive's class.
    pub ng_template_guards: Vec<TemplateGuardMeta>,
    /// Whether the Directive's class has a static ngTemplateContextGuard function.
    pub has_ng_template_context_guard: bool,
    /// The set of input fields which have a corresponding static `ngAcceptInputType_`.
    pub coerced_input_fields: HashSet<String>,
    /// The set of input fields which map to `readonly`, `private`, or `protected` members.
    pub restricted_input_fields: HashSet<String>,
    /// The set of input fields which are declared as string literal members.
    pub string_literal_input_fields: HashSet<String>,
    /// The set of input fields which do not have corresponding members in the class.
    pub undeclared_input_fields: HashSet<String>,
    /// Names of the public methods of the class.
    pub public_methods: HashSet<String>,
    /// Whether the Directive's class is generic.
    pub is_generic: bool,
}

/// Metadata that describes a template guard for one of the directive's inputs.
/// Matches TypeScript's TemplateGuardMeta (L347-361)
#[derive(Debug, Clone)]
pub struct TemplateGuardMeta {
    /// The input name that this guard should be applied to.
    pub input_name: String,
    /// Type of the template guard: 'invocation' or 'binding'.
    pub guard_type: TemplateGuardType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateGuardType {
    Invocation,
    Binding,
}

/// Host directive metadata.
/// Matches TypeScript's HostDirectiveMeta (L307-326)
#[derive(Debug, Clone)]
pub struct HostDirectiveMeta {
    /// Reference to the host directive class.
    pub directive: String,
    /// Whether the reference to the host directive is a forward reference.
    pub is_forward_reference: bool,
    /// Inputs from the host directive that have been exposed.
    pub inputs: Option<HashMap<String, String>>,
    /// Outputs from the host directive that have been exposed.
    pub outputs: Option<HashMap<String, String>>,
}

/// Metadata collected for a directive within an NgModule's scope.
/// Matches TypeScript's DirectiveMeta interface (L198-305)
/// Extends T2DirectiveMeta + DirectiveTypeCheckMeta
#[derive(Debug, Clone)]
pub struct DirectiveMeta {
    // === MetaKind ===
    pub kind: MetaKind,
    
    // === MatchSource ===
    /// Way in which the directive was matched.
    pub match_source: MatchSource,
    
    // === T2DirectiveMeta fields (from t2_api.ts) ===
    /// Name of the directive class (used for debugging).
    pub name: String,
    /// The selector for the directive or None if there isn't one.
    pub selector: Option<String>,
    /// Whether the directive is a component.
    pub is_component: bool,
    /// Set of inputs which this directive claims.
    pub inputs: ClassPropertyMapping,
    /// Set of outputs which this directive claims.
    pub outputs: ClassPropertyMapping,
    /// Name under which the directive is exported, if any.
    pub export_as: Option<Vec<String>>,
    /// Whether the directive is a structural directive.
    pub is_structural: bool,
    /// If the directive is a component, includes the selectors of its `ng-content` elements.
    pub ng_content_selectors: Vec<String>,
    /// Whether the template of the component preserves whitespaces.
    pub preserve_whitespaces: bool,
    
    // === DirectiveTypeCheckMeta fields ===
    pub type_check_meta: DirectiveTypeCheckMeta,
    
    // === DirectiveMeta-specific fields ===
    /// Query field names.
    pub queries: Vec<String>,
    /// List of input fields that were defined in the class decorator metadata.
    pub input_field_names_from_metadata_array: Option<HashSet<String>>,
    /// A Reference to the base class for the directive, if one was detected.
    pub base_class: Option<String>,
    /// Whether the directive had some issue with its declaration.
    pub is_poisoned: bool,
    /// Whether the directive is a standalone entity.
    pub is_standalone: bool,
    /// Whether the directive is a signal entity.
    pub is_signal: bool,
    /// For standalone components, the list of imported types.
    pub imports: Option<Vec<String>>,
    /// Raw imports expression.
    pub raw_imports: Option<String>,
    /// For standalone components, the list of imported types for `@defer` blocks.
    pub deferred_imports: Option<Vec<String>>,
    /// For standalone components, the list of schemas declared.
    pub schemas: Option<Vec<String>>,
    /// The primary decorator associated with this directive.
    pub decorator: Option<String>,
    /// Additional directives applied to the directive host.
    pub host_directives: Option<Vec<HostDirectiveMeta>>,
    /// Whether the directive should be assumed to export providers if imported as a standalone type.
    pub assumed_to_export_providers: bool,
    /// Whether this class was imported via `@Component.deferredImports` field.
    pub is_explicitly_deferred: bool,
    /// Whether selectorless is enabled for the specific component.
    pub selectorless_enabled: bool,
    /// Names of the symbols within the source file that are referenced directly inside the template.
    pub local_referenced_symbols: Option<HashSet<String>>,
    
    // === Component-specific fields (only valid when is_component=true) ===
    pub template: Option<String>,
    pub template_url: Option<String>,
    pub template_ast: Option<Vec<HtmlNode>>,
    pub styles: Option<Vec<String>>,
    pub style_urls: Option<Vec<String>>,
    pub change_detection: Option<angular_compiler::core::ChangeDetectionStrategy>,
    
    // === Source tracking ===
    pub source_file: Option<PathBuf>,
}

impl Default for DirectiveMeta {
    fn default() -> Self {
        Self {
            kind: MetaKind::Directive,
            match_source: MatchSource::default(),
            name: String::new(),
            selector: None,
            is_component: false,
            inputs: ClassPropertyMapping::new(),
            outputs: ClassPropertyMapping::new(),
            export_as: None,
            is_structural: false,
            ng_content_selectors: Vec::new(),
            preserve_whitespaces: false,
            type_check_meta: DirectiveTypeCheckMeta::default(),
            queries: Vec::new(),
            input_field_names_from_metadata_array: None,
            base_class: None,
            is_poisoned: false,
            is_standalone: true,
            is_signal: false,
            imports: None,
            raw_imports: None,
            deferred_imports: None,
            schemas: None,
            decorator: None,
            host_directives: None,
            assumed_to_export_providers: false,
            is_explicitly_deferred: false,
            selectorless_enabled: false,
            local_referenced_symbols: None,
            template: None,
            template_url: None,
            template_ast: None,
            styles: None,
            style_urls: None,
            change_detection: None,
            source_file: None,
        }
    }
}

/// Metadata for @Pipe decorator.
/// Matches TypeScript's PipeMeta interface (L366-375)
#[derive(Debug, Clone)]
pub struct PipeMeta {
    pub kind: MetaKind,
    pub name: String,
    pub pipe_name: String,
    pub name_expr: Option<String>,
    pub is_standalone: bool,
    pub is_pure: bool,
    pub decorator: Option<String>,
    pub is_explicitly_deferred: bool,
    pub source_file: Option<PathBuf>,
}

impl Default for PipeMeta {
    fn default() -> Self {
        Self {
            kind: MetaKind::Pipe,
            name: String::new(),
            pipe_name: String::new(),
            name_expr: None,
            is_standalone: true,
            is_pure: true,
            decorator: None,
            is_explicitly_deferred: false,
            source_file: None,
        }
    }
}

/// Metadata for @Injectable decorator.
#[derive(Debug, Clone)]
pub struct InjectableMeta {
    pub name: String,
    pub provided_in: Option<String>,
    pub source_file: Option<PathBuf>,
}

/// Metadata for @NgModule decorator.
/// Matches TypeScript's NgModuleMeta interface (L25-77)
#[derive(Debug, Clone)]
pub struct NgModuleMeta {
    pub kind: MetaKind,
    pub name: String,
    pub declarations: Vec<String>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub schemas: Vec<String>,
    pub is_poisoned: bool,
    pub raw_declarations: Option<String>,
    pub raw_imports: Option<String>,
    pub raw_exports: Option<String>,
    pub decorator: Option<String>,
    pub may_declare_providers: bool,
    pub source_file: Option<PathBuf>,
}

impl Default for NgModuleMeta {
    fn default() -> Self {
        Self {
            kind: MetaKind::NgModule,
            name: String::new(),
            declarations: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            schemas: Vec::new(),
            is_poisoned: false,
            raw_declarations: None,
            raw_imports: None,
            raw_exports: None,
            decorator: None,
            may_declare_providers: false,
            source_file: None,
        }
    }
}

/// Unified enum for all Angular decorator metadata types.
/// Note: Component is now part of DirectiveMeta with is_component=true
#[derive(Debug, Clone)]
pub enum DecoratorMetadata {
    Directive(DirectiveMeta),  // is_component flag distinguishes component vs directive
    Pipe(PipeMeta),
    Injectable(InjectableMeta),
    NgModule(NgModuleMeta),
}

impl DecoratorMetadata {
    /// Get the MetaKind for this metadata.
    pub fn meta_kind(&self) -> MetaKind {
        match self {
            DecoratorMetadata::Directive(d) => d.kind,
            DecoratorMetadata::Pipe(p) => p.kind,
            DecoratorMetadata::Injectable(_) => MetaKind::Directive, // Injectable doesn't have MetaKind
            DecoratorMetadata::NgModule(n) => n.kind,
        }
    }
    
    /// Get the name of the decorated class.
    pub fn name(&self) -> &str {
        match self {
            DecoratorMetadata::Directive(d) => &d.name,
            DecoratorMetadata::Pipe(p) => &p.name,
            DecoratorMetadata::Injectable(i) => &i.name,
            DecoratorMetadata::NgModule(n) => &n.name,
        }
    }
    
    /// Get the source file path for this metadata.
    pub fn source_file(&self) -> Option<&PathBuf> {
        match self {
            DecoratorMetadata::Directive(d) => d.source_file.as_ref(),
            DecoratorMetadata::Pipe(p) => p.source_file.as_ref(),
            DecoratorMetadata::Injectable(i) => i.source_file.as_ref(),
            DecoratorMetadata::NgModule(n) => n.source_file.as_ref(),
        }
    }
    
    /// Check if this is a component.
    pub fn is_component(&self) -> bool {
        matches!(self, DecoratorMetadata::Directive(d) if d.is_component)
    }
    
    /// Check if this is a pipe.
    pub fn is_pipe(&self) -> bool {
        matches!(self, DecoratorMetadata::Pipe(_))
    }
    
    /// Check if this is an injectable.
    pub fn is_injectable(&self) -> bool {
        matches!(self, DecoratorMetadata::Injectable(_))
    }
}

/// Type alias for backward compatibility during migration.
pub type DirectiveMetadata = DecoratorMetadata;
