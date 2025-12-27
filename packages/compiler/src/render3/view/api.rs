//! Render3 View API
//!
//! Corresponds to packages/compiler/src/render3/view/api.ts
//! Contains API definitions for view compilation

use indexmap::IndexMap;
use std::collections::HashMap;

use crate::core::{ChangeDetectionStrategy, ViewEncapsulation};
use crate::output::output_ast::Expression;
use crate::parse_util::ParseSourceSpan;
use crate::render3::r3_ast as t;
use crate::render3::r3_factory::R3DependencyMetadata;
use crate::render3::util::{MaybeForwardRefExpression, R3Reference};

/// Information needed to compile a directive for the render3 runtime.
#[derive(Debug, Clone)]
pub struct R3DirectiveMetadata {
    /// Name of the directive type.
    pub name: String,
    /// An expression representing a reference to the directive itself.
    pub type_: R3Reference,
    /// Number of generic type parameters of the type itself.
    pub type_argument_count: usize,
    /// A source span for the directive type.
    pub type_source_span: ParseSourceSpan,
    /// Dependencies of the directive's constructor.
    pub deps: Option<Vec<R3DependencyMetadata>>,
    /// Unparsed selector of the directive, or `None` if there was no selector.
    pub selector: Option<String>,
    /// Information about the content queries made by the directive.
    pub queries: Vec<R3QueryMetadata>,
    /// Information about the view queries made by the directive.
    pub view_queries: Vec<R3QueryMetadata>,
    /// Mappings indicating how the directive interacts with its host element.
    pub host: R3HostMetadata,
    /// Information about usage of specific lifecycle events.
    pub lifecycle: R3LifecycleMetadata,
    /// A mapping of inputs from class property names to binding property names.
    pub inputs: IndexMap<String, R3InputMetadata>,
    /// A mapping of outputs from class property names to binding property names.
    pub outputs: IndexMap<String, String>,
    /// Whether or not the component or directive inherits from another class.
    pub uses_inheritance: bool,
    /// Reference name under which to export the directive's type in a template.
    pub export_as: Option<Vec<String>>,
    /// The list of providers defined in the directive.
    pub providers: Option<Expression>,
    /// Whether or not the component or directive is standalone.
    pub is_standalone: bool,
    /// Whether or not the component or directive is signal-based.
    pub is_signal: bool,
    /// Additional directives applied to the directive host.
    pub host_directives: Option<Vec<R3HostDirectiveMetadata>>,
}

/// Lifecycle metadata
#[derive(Debug, Clone, Default)]
pub struct R3LifecycleMetadata {
    /// Whether the directive uses NgOnChanges.
    pub uses_on_changes: bool,
}

/// Defines how dynamic imports for deferred dependencies should be emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferBlockDepsEmitMode {
    /// Dynamic imports are grouped on per-block basis.
    PerBlock,
    /// Dynamic imports are grouped on per-component basis.
    PerComponent,
}

/// Specifies how a list of declaration type references should be emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarationListEmitMode {
    /// The list of declarations is emitted into the generated code as is.
    Direct,
    /// The list is emitted wrapped inside a closure.
    Closure,
    /// Similar to `Closure`, with forward reference resolution.
    ClosureResolved,
    /// Resolved at runtime.
    RuntimeResolved,
}

/// Information needed to compile a component for the render3 runtime.
#[derive(Debug, Clone)]
pub struct R3ComponentMetadata {
    /// Base directive metadata.
    pub directive: R3DirectiveMetadata,
    /// Information about the component's template.
    pub template: R3ComponentTemplate,
    /// Template declarations.
    pub declarations: Vec<R3TemplateDependencyMetadata>,
    /// Metadata related to deferred blocks.
    pub defer: R3ComponentDeferMetadata,
    /// Specifies how the declarations array should be emitted.
    pub declaration_list_emit_mode: DeclarationListEmitMode,
    /// A collection of styling data.
    pub styles: Vec<String>,
    /// External stylesheet paths.
    pub external_styles: Option<Vec<String>>,
    /// An encapsulation policy for the component's styling.
    pub encapsulation: ViewEncapsulation,
    /// A collection of animation triggers.
    pub animations: Option<Expression>,
    /// The list of view providers defined in the component.
    pub view_providers: Option<Expression>,
    /// Path to the .ts file.
    pub relative_context_file_path: String,
    /// Whether translation variable name should contain external message id.
    pub i18n_use_external_ids: bool,
    /// Strategy used for detecting changes in the component.
    pub change_detection: Option<ChangeDetectionOrExpression>,
    /// Relative path to the component's template.
    pub relative_template_path: Option<String>,
    /// Whether any of the component's dependencies are directives.
    pub has_directive_dependencies: bool,
    /// The imports expression for standalone components.
    pub raw_imports: Option<Expression>,
}

/// Change detection strategy or expression
#[derive(Debug, Clone)]
pub enum ChangeDetectionOrExpression {
    Strategy(ChangeDetectionStrategy),
    Expression(Expression),
}

/// Component template metadata
#[derive(Debug, Clone)]
pub struct R3ComponentTemplate {
    /// Parsed nodes of the template.
    pub nodes: Vec<t::R3Node>,
    /// Any ng-content selectors extracted from the template.
    pub ng_content_selectors: Vec<String>,
    /// Whether the template preserves whitespaces.
    pub preserve_whitespaces: bool,
}

/// Information about the deferred blocks in a component's template.
#[derive(Debug, Clone)]
pub enum R3ComponentDeferMetadata {
    PerBlock {
        blocks: HashMap<usize, Option<Expression>>,
    },
    PerComponent {
        dependencies_fn: Option<Expression>,
    },
}

/// Metadata for an individual input on a directive.
#[derive(Debug, Clone)]
pub struct R3InputMetadata {
    pub class_property_name: String,
    pub binding_property_name: String,
    pub required: bool,
    pub is_signal: bool,
    /// Transform function for the input.
    pub transform_function: Option<Expression>,
}

/// Template dependency kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum R3TemplateDependencyKind {
    Directive = 0,
    Pipe = 1,
    NgModule = 2,
}

/// A dependency that's used within a component template.
#[derive(Debug, Clone)]
pub struct R3TemplateDependency {
    pub kind: R3TemplateDependencyKind,
    /// The type of the dependency as an expression.
    pub type_: Expression,
}

/// Template dependency metadata
#[derive(Debug, Clone)]
pub enum R3TemplateDependencyMetadata {
    Directive(R3DirectiveDependencyMetadata),
    Pipe(R3PipeDependencyMetadata),
    NgModule(R3NgModuleDependencyMetadata),
}

/// Information about a directive that is used in a component template.
#[derive(Debug, Clone)]
pub struct R3DirectiveDependencyMetadata {
    pub kind: R3TemplateDependencyKind,
    pub type_: Expression,
    /// The selector of the directive.
    pub selector: String,
    /// The binding property names of the inputs.
    pub inputs: Vec<String>,
    /// The binding property names of the outputs.
    pub outputs: Vec<String>,
    /// Name under which the directive is exported.
    pub export_as: Option<Vec<String>>,
    /// If true then this directive is actually a component.
    pub is_component: bool,
    /// A source span for the directive reference in the imports array.
    pub source_span: Option<ParseSourceSpan>,
}

/// Pipe dependency metadata
#[derive(Debug, Clone)]
pub struct R3PipeDependencyMetadata {
    pub kind: R3TemplateDependencyKind,
    pub type_: Expression,
    pub name: String,
    /// A source span for the pipe reference in the imports array.
    pub source_span: Option<ParseSourceSpan>,
}

/// NgModule dependency metadata
#[derive(Debug, Clone)]
pub struct R3NgModuleDependencyMetadata {
    pub kind: R3TemplateDependencyKind,
    pub type_: Expression,
}

/// Information needed to compile a query (view or content).
#[derive(Debug, Clone)]
pub struct R3QueryMetadata {
    /// Name of the property on the class to update with query results.
    pub property_name: String,
    /// Whether to read only the first matching result.
    pub first: bool,
    /// The predicate for the query.
    pub predicate: R3QueryPredicate,
    /// Whether to include only direct children or all descendants.
    pub descendants: bool,
    /// If the `QueryList` should fire change event only if actual change was computed.
    pub emit_distinct_changes_only: bool,
    /// An expression representing a type to read from each matched node.
    pub read: Option<Expression>,
    /// Whether or not this query should collect only static results.
    pub static_: bool,
    /// Whether the query is signal-based.
    pub is_signal: bool,
}

/// Query predicate - either an expression or string selectors
#[derive(Debug, Clone)]
pub enum R3QueryPredicate {
    Expression(MaybeForwardRefExpression),
    Selectors(Vec<String>),
}

/// Mappings indicating how the class interacts with its host element.
#[derive(Debug, Clone, Default)]
pub struct R3HostMetadata {
    /// A mapping of attribute binding keys to expressions.
    pub attributes: HashMap<String, Expression>,
    /// A mapping of event binding keys to unparsed expressions.
    pub listeners: HashMap<String, String>,
    /// A mapping of property binding keys to unparsed expressions.
    pub properties: HashMap<String, String>,
    /// Special attributes.
    pub special_attributes: R3HostSpecialAttributes,
}

/// Special host attributes
#[derive(Debug, Clone, Default)]
pub struct R3HostSpecialAttributes {
    pub style_attr: Option<String>,
    pub class_attr: Option<String>,
}

/// Information needed to compile a host directive for the render3 runtime.
#[derive(Debug, Clone)]
pub struct R3HostDirectiveMetadata {
    /// An expression representing the host directive class itself.
    pub directive: R3Reference,
    /// Whether the expression referring to the host directive is a forward reference.
    pub is_forward_reference: bool,
    /// Inputs from the host directive that will be exposed on the host.
    pub inputs: Option<HashMap<String, String>>,
    /// Outputs from the host directive that will be exposed on the host.
    pub outputs: Option<HashMap<String, String>>,
}

/// Information needed to compile the defer block resolver function.
#[derive(Debug, Clone)]
pub enum R3DeferResolverFunctionMetadata {
    PerBlock {
        dependencies: Vec<R3DeferPerBlockDependency>,
    },
    PerComponent {
        dependencies: Vec<R3DeferPerComponentDependency>,
    },
}

/// Information about a single dependency of a defer block in `PerBlock` mode.
#[derive(Debug, Clone)]
pub struct R3DeferPerBlockDependency {
    /// Reference to a dependency.
    pub type_reference: Expression,
    /// Dependency class name.
    pub symbol_name: String,
    /// Whether this dependency can be defer-loaded.
    pub is_deferrable: bool,
    /// Import path where this dependency is located.
    pub import_path: Option<String>,
    /// Whether the symbol is the default export.
    pub is_default_import: bool,
}

/// Information about a single dependency of a defer block in `PerComponent` mode.
#[derive(Debug, Clone)]
pub struct R3DeferPerComponentDependency {
    /// Dependency class name.
    pub symbol_name: String,
    /// Import path where this dependency is located.
    pub import_path: String,
    /// Whether the symbol is the default export.
    pub is_default_import: bool,
}
