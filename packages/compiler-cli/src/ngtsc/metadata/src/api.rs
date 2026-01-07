//! Core metadata API types matching TypeScript api.ts
//!
//! This module defines the core metadata types used throughout the Angular compiler.
//! Matches: angular/packages/compiler-cli/src/ngtsc/metadata/src/api.ts

use super::property_mapping::{ClassPropertyMapping, DecoratorInputTransform};
pub use crate::ngtsc::imports::{OwningModule, Reference};
use angular_compiler::ml_parser::ast::Node as HtmlNode;
pub use angular_compiler::render3::view::t2_api::{
    DirectiveMeta as T2DirectiveMeta, LegacyAnimationTriggerNames,
};
use oxc_ast::ast as oxc_ast;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

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

/// Represents a base class reference that may be dynamic.
/// Matches TypeScript's `Reference<ClassDeclaration> | 'dynamic' | null`
#[derive(Debug, Clone)]
pub enum BaseClass<'a> {
    /// Static reference to base class.
    Static(Reference<'a>),
    /// Dynamic base class (couldn't statically determine).
    Dynamic,
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
    /// The set of input fields which do not have corresponding members.
    pub undeclared_input_fields: HashSet<String>,
    /// Whether the Directive is generic (has type parameters in its declaration).
    pub is_generic: bool,
}

/// Template guard metadata.
/// Matches TypeScript's TemplateGuardMeta interface (L64-69)
#[derive(Debug, Clone)]
pub struct TemplateGuardMeta {
    pub input_name: String,
    pub guard_type: TemplateGuardType,
}

/// Type of template guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateGuardType {
    Invocation,
    Binding,
}

/// Host directive metadata.
/// Matches TypeScript's HostDirectiveMeta (L307-326)
///
/// The lifetime `'a` is tied to the OXC AST allocator.
#[derive(Debug)]
pub struct HostDirectiveMeta<'a> {
    /// Reference to the host directive class.
    pub directive: Option<Reference<'a>>,
    /// Whether the reference to the host directive is a forward reference.
    pub is_forward_reference: bool,
    /// Inputs from the host directive that have been exposed.
    pub inputs: Option<HashMap<String, String>>,
    /// Outputs from the host directive that have been exposed.
    pub outputs: Option<HashMap<String, String>>,
}

impl<'a> Clone for HostDirectiveMeta<'a> {
    fn clone(&self) -> Self {
        Self {
            directive: self.directive.clone(),
            is_forward_reference: self.is_forward_reference,
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
        }
    }
}

/// Metadata regarding a directive that's needed to match it against template elements.
/// Part of T2DirectiveMeta trait implementation.
#[derive(Debug, Clone, Default)]
pub struct T2DirectiveMetadata {
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
    /// The list of selectors from any NgModule `ng-content` in the component's template.
    pub ng_content_selectors: Option<Vec<String>>,
    /// Whether the directive/component has `preserveWhitespaces: true`.
    pub preserve_whitespaces: bool,
    /// Animation trigger names.
    pub animation_trigger_names: Option<LegacyAnimationTriggerNames>,
}

/// Metadata specifically for components.
#[derive(Debug, Clone, Default)]
pub struct ComponentMetadata {
    pub template: Option<String>,
    pub template_url: Option<String>,
    pub template_ast: Option<Vec<HtmlNode>>,
    pub styles: Option<Vec<String>>,
    pub style_urls: Option<Vec<String>>,
    pub change_detection: Option<angular_compiler::core::ChangeDetectionStrategy>,
}

/// Metadata collected for a directive within an NgModule's scope.
/// Matches TypeScript's DirectiveMeta interface (L198-305)
///
/// The lifetime `'a` is tied to the OXC AST allocator, allowing direct
/// access to the decorator AST node without cloning.
#[derive(Debug)]
pub struct DirectiveMeta<'a> {
    // === MetaKind ===
    pub kind: MetaKind,

    // === MatchSource ===
    /// Way in which the directive was matched.
    pub match_source: MatchSource,

    // === Compositional Metadata ===
    /// Core directive metadata used for matching and T2 analysis.
    pub t2: T2DirectiveMetadata,
    /// Component-specific metadata, if this is a component.
    pub component: Option<ComponentMetadata>,

    // === Type checking metadata (Compositional) ===
    pub type_check: DirectiveTypeCheckMeta,

    // === Additional DirectiveMeta fields ===
    /// The list of content queries declared by the directive.
    pub queries: Vec<QueryMetadata>,
    /// Class inputs which come from decorator array (not class members).
    pub input_field_names_from_metadata_array: Option<HashSet<String>>,
    /// A Reference to the base class for the directive, if one was detected.
    /// Value of `Dynamic` indicates base was detected but couldn't be statically resolved.
    pub base_class: Option<BaseClass<'a>>,
    /// Whether the directive had some issue with its declaration.
    pub is_poisoned: bool,
    /// Whether the directive is a standalone entity.
    pub is_standalone: bool,
    /// Whether the directive is a signal entity.
    pub is_signal: bool,
    /// For standalone components, the list of imported types.
    pub imports: Option<Vec<Reference<'a>>>,
    /// Raw imports expression.
    pub raw_imports: Option<String>,
    /// For standalone components, the list of imported types for `@defer` blocks.
    pub deferred_imports: Option<Vec<Reference<'a>>>,
    /// For standalone components, the list of schemas declared.
    pub schemas: Option<Vec<String>>,
    /// The primary decorator associated with this directive.
    /// This is a direct reference to the OXC AST decorator node.
    pub decorator: Option<&'a oxc_ast::Decorator<'a>>,
    /// Host bindings extracted from the decorator.
    pub host: angular_compiler::render3::view::api::R3HostMetadata,
    /// Additional directives applied to the directive host.
    pub host_directives: Option<Vec<HostDirectiveMeta<'a>>>,
    /// Whether the directive should be assumed to export providers if imported as a standalone type.
    pub assumed_to_export_providers: bool,
    /// Whether this class was imported via `@Component.deferredImports` field.
    pub is_explicitly_deferred: bool,
    /// Whether selectorless is enabled for the specific component.
    pub selectorless_enabled: bool,
    /// Names of the symbols within the source file that are referenced directly inside the template.
    pub local_referenced_symbols: Option<HashSet<String>>,
    /// Source file path for source tracking.
    pub source_file: Option<PathBuf>,
    /// Constructor parameters for dependency injection.
    pub constructor_params: Vec<ConstructorParam>,
    /// View queries (@ViewChild, @ViewChildren, viewChild, viewChildren).
    pub view_queries: Vec<QueryMetadata>,
    /// Lifecycle hooks detected on the class.
    pub lifecycle: angular_compiler::render3::view::api::R3LifecycleMetadata,
    /// File-level imports mapping (local name -> module path).
    pub file_imports: Option<HashMap<String, String>>,
    /// Providers expression extracted from the decorator.
    pub providers: Option<angular_compiler::output::output_ast::Expression>,
}

/// Constructor parameter metadata.
#[derive(Debug, Clone)]
pub struct ConstructorParam {
    /// Parameter name.
    pub name: Option<String>,
    /// Type name (e.g., "ElementRef", "NgControl", "Renderer2").
    pub type_name: Option<String>,
    /// Module where the type comes from (e.g., "@angular/core", "@angular/forms").
    pub from_module: Option<String>,
    /// Dependency flags.
    pub attribute: Option<String>, // @Attribute('name')
    pub optional: bool,  // @Optional()
    pub host: bool,      // @Host()
    pub self_: bool,     // @Self()
    pub skip_self: bool, // @SkipSelf()
}

/// Metadata for queries (ViewChild, ViewChildren, ContentChild, ContentChildren).
#[derive(Debug, Clone)]
pub struct QueryMetadata {
    /// Property name on the component class.
    pub property_name: String,
    /// The template reference variable or component/directive type selector.
    pub selector: String,
    /// Whether to return only the first match (ViewChild) or all matches (ViewChildren).
    pub first: bool,
    /// Whether to include descendants.
    pub descendants: bool,
    /// Whether the query is static (resolved before change detection).
    pub is_static: bool,
    /// Optional read token.
    pub read: Option<String>,
    /// Whether the query is signal-based.
    pub is_signal: bool,
}

impl<'a> T2DirectiveMeta for DirectiveMeta<'a> {
    fn name(&self) -> &str {
        &self.t2.name
    }

    fn selector(&self) -> Option<&str> {
        self.t2.selector.as_deref()
    }

    fn is_component(&self) -> bool {
        self.t2.is_component
    }

    fn inputs(&self) -> &dyn angular_compiler::render3::view::t2_api::InputOutputPropertySet {
        &self.t2.inputs
    }

    fn outputs(&self) -> &dyn angular_compiler::render3::view::t2_api::InputOutputPropertySet {
        &self.t2.outputs
    }

    fn export_as(&self) -> Option<&[String]> {
        self.t2.export_as.as_deref()
    }

    fn is_structural(&self) -> bool {
        self.t2.is_structural
    }

    fn ng_content_selectors(&self) -> Option<&[String]> {
        self.t2.ng_content_selectors.as_deref()
    }

    fn preserve_whitespaces(&self) -> bool {
        self.t2.preserve_whitespaces
    }

    fn animation_trigger_names(&self) -> Option<&LegacyAnimationTriggerNames> {
        self.t2.animation_trigger_names.as_ref()
    }
}

impl<'a> Default for DirectiveMeta<'a> {
    fn default() -> Self {
        Self {
            kind: MetaKind::Directive,
            match_source: MatchSource::default(),
            t2: T2DirectiveMetadata::default(),
            component: None,
            type_check: DirectiveTypeCheckMeta::default(),
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
            host: angular_compiler::render3::view::api::R3HostMetadata::default(),
            host_directives: None,
            assumed_to_export_providers: false,
            is_explicitly_deferred: false,
            selectorless_enabled: false,
            local_referenced_symbols: None,
            source_file: None,
            constructor_params: Vec::new(),
            view_queries: Vec::new(),
            lifecycle: angular_compiler::render3::view::api::R3LifecycleMetadata::default(),

            file_imports: None,
            providers: None,
        }
    }
}

// Implement Clone manually since we have a reference field
impl<'a> Clone for DirectiveMeta<'a> {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind,
            match_source: self.match_source,
            t2: self.t2.clone(),
            component: self.component.clone(),
            type_check: self.type_check.clone(),
            queries: self.queries.clone(),
            input_field_names_from_metadata_array: self
                .input_field_names_from_metadata_array
                .clone(),
            base_class: self.base_class.clone(),
            is_poisoned: self.is_poisoned,
            is_standalone: self.is_standalone,
            is_signal: self.is_signal,
            imports: self.imports.clone(),
            raw_imports: self.raw_imports.clone(),
            deferred_imports: self.deferred_imports.clone(),
            schemas: self.schemas.clone(),
            decorator: self.decorator, // Copy the reference
            host: self.host.clone(),
            host_directives: self.host_directives.clone(),
            assumed_to_export_providers: self.assumed_to_export_providers,
            is_explicitly_deferred: self.is_explicitly_deferred,
            selectorless_enabled: self.selectorless_enabled,
            local_referenced_symbols: self.local_referenced_symbols.clone(),
            source_file: self.source_file.clone(),
            constructor_params: self.constructor_params.clone(),
            view_queries: self.view_queries.clone(),
            lifecycle: self.lifecycle.clone(),
            file_imports: self.file_imports.clone(),
            providers: self.providers.clone(),
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
///
/// The lifetime `'a` is tied to the OXC AST allocator.
#[derive(Debug)]
pub enum DecoratorMetadata<'a> {
    Directive(DirectiveMeta<'a>), // is_component flag distinguishes component vs directive
    Pipe(PipeMeta),
    Injectable(InjectableMeta),
    NgModule(NgModuleMeta),
}

impl<'a> Clone for DecoratorMetadata<'a> {
    fn clone(&self) -> Self {
        match self {
            DecoratorMetadata::Directive(d) => DecoratorMetadata::Directive(d.clone()),
            DecoratorMetadata::Pipe(p) => DecoratorMetadata::Pipe(p.clone()),
            DecoratorMetadata::Injectable(i) => DecoratorMetadata::Injectable(i.clone()),
            DecoratorMetadata::NgModule(n) => DecoratorMetadata::NgModule(n.clone()),
        }
    }
}

impl<'a> DecoratorMetadata<'a> {
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
            DecoratorMetadata::Directive(d) => &d.t2.name,
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
        matches!(self, DecoratorMetadata::Directive(d) if d.t2.is_component)
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
pub type DirectiveMetadata<'a> = DecoratorMetadata<'a>;

/// Owned version of DirectiveMeta for cases where lifetime is inconvenient.
/// This version does not hold a reference to the OXC decorator.
pub type OwnedDirectiveMeta = DirectiveMeta<'static>;
