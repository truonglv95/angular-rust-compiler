//! Ingest Module
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/ingest.ts
//! Converts R3 AST nodes into IR operations

use crate::constant_pool::ConstantPool;
use crate::core::ChangeDetectionStrategy;
use crate::expression_parser::ast::AST as ExprAST;
use crate::i18n::i18n_ast::{I18nMeta, Node as I18nNode};
use crate::ml_parser::tags::split_ns_name;
use crate::output::output_ast::Expression;
use crate::parse_util::ParseSourceSpan;
use crate::render3::r3_ast as t;
use crate::render3::view::api::R3ComponentDeferMetadata;
use crate::template_parser::binding_parser::{ParsedEvent, ParsedProperty};
use std::sync::Arc;
// Note: DomElementSchemaRegistry is created per-call in ingest_element_bindings

use crate::render3::view::api::R3TemplateDependencyMetadata;
use crate::template::pipeline::ir;
use crate::template::pipeline::src::compilation::{
    CompilationJob, ComponentCompilationJob, HostBindingCompilationJob, TemplateCompilationMode,
    ViewCompilationUnit,
};
use crate::template::pipeline::src::conversion::{namespace_for_key, prefix_with_namespace};
// Note: LazyLock was used for DOM_SCHEMA, but now created per-call

// Compatibility mode
const COMPATIBILITY_MODE: ir::CompatibilityMode = ir::CompatibilityMode::TemplateDefinitionBuilder;

// Schema containing DOM elements and their properties.
// Using LazyLock for non-const initialization
// Note: DOM_SCHEMA is created per-call in ingest_element_bindings instead
// static DOM_SCHEMA: LazyLock<DomElementSchemaRegistry> = LazyLock::new(|| DomElementSchemaRegistry::new());

// Tag name of the `ng-template` element.
#[allow(dead_code)]
const NG_TEMPLATE_TAG_NAME: &str = "ng-template";

// Prefix for any animation binding
#[allow(dead_code)]
const ANIMATE_PREFIX: &str = "animate.";

/// Check if i18n metadata is a root message node
pub fn is_i18n_root_node(meta: Option<&I18nMeta>) -> bool {
    matches!(meta, Some(I18nMeta::Message(_)))
}

/// Check if i18n metadata is a single ICU
pub fn is_single_i18n_icu(meta: Option<&I18nMeta>) -> bool {
    if let Some(I18nMeta::Message(msg)) = meta {
        // Check if message has exactly one ICU node
        msg.nodes.len() == 1 && matches!(msg.nodes[0], crate::i18n::i18n_ast::Node::Icu(_))
    } else {
        false
    }
}

/// Process a template AST and convert it into a `ComponentCompilationJob` in the intermediate representation.
pub fn ingest_component(
    component_name: String,
    template: Vec<t::R3Node>,
    constant_pool: ConstantPool,
    compilation_mode: TemplateCompilationMode,
    relative_context_file_path: String,
    i18n_use_external_ids: bool,
    defer_meta: R3ComponentDeferMetadata,
    all_deferrable_deps_fn: Option<Expression>,
    relative_template_path: Option<String>,
    enable_debug_locations: bool,
    change_detection: Option<ChangeDetectionStrategy>,
    available_dependencies: Vec<R3TemplateDependencyMetadata>,
) -> ComponentCompilationJob {
    let mut job = ComponentCompilationJob::new(
        component_name.clone(),
        constant_pool,
        COMPATIBILITY_MODE,
        compilation_mode,
        relative_context_file_path,
        i18n_use_external_ids,
        defer_meta,
        all_deferrable_deps_fn,
        relative_template_path,
        enable_debug_locations,
        change_detection,
        available_dependencies,
    );

    // Ingest nodes into root using Safe XrefId approach
    // Ingest nodes into root using Safe XrefId approach
    let root_xref = job.root.xref;
    ingest_children_into_view(&mut job, root_xref, template);

    job
}

/// Internal helper to ingest nodes directly into a ViewCompilationUnit
fn ingest_nodes_internal(
    view: &mut ViewCompilationUnit,
    template: Vec<t::R3Node>,
    job: &mut ComponentCompilationJob,
) {
    let mut iter = template.into_iter().peekable();

    while let Some(node) = { iter.next() } {
        match node {
            t::R3Node::Element(el) => {
                ingest_element(view, *el, job);
            }
            t::R3Node::Template(tmpl) => {
                ingest_template(view, *tmpl, job);
            }
            t::R3Node::Content(content) => {
                ingest_content(view, *content, job);
            }
            t::R3Node::Text(text) => {
                let mut prefix = text.value.to_string();

                // Coalesce adjacent text nodes and skip comments
                loop {
                    match iter.peek() {
                        Some(t::R3Node::Text(_)) => {
                            if let Some(t::R3Node::Text(next_text)) = iter.next() {
                                prefix.push_str(&next_text.value);
                            }
                        }
                        Some(t::R3Node::Comment(_)) => {
                            // Skip comments to allow merging across them
                            iter.next();
                        }
                        _ => break,
                    }
                }

                // Check if next node is BoundText to merge
                let is_next_bound_text = matches!(iter.peek(), Some(t::R3Node::BoundText(_)));
                if is_next_bound_text {
                    if let Some(t::R3Node::BoundText(bound_text)) = iter.next() {
                        // Merge text value as prefix for BoundText
                        let prefix = if prefix.trim().is_empty() {
                            String::new()
                        } else {
                            prefix
                        };
                        ingest_bound_text(view, *bound_text, None, prefix, job);
                    }
                } else {
                    // Standard text node - skip whitespace-only
                    if !prefix.trim().is_empty() {
                        ingest_text(
                            view,
                            t::Text {
                                value: prefix.into(),
                                source_span: text.source_span.clone(),
                            },
                            None,
                            job,
                        );
                    }
                }
            }
            t::R3Node::BoundText(bound_text) => {
                ingest_bound_text(view, *bound_text, None, String::new(), job);
            }
            t::R3Node::IfBlock(if_block) => {
                ingest_if_block(view, *if_block, job);
            }
            t::R3Node::SwitchBlock(switch_block) => {
                ingest_switch_block(view, *switch_block, job);
            }
            t::R3Node::DeferredBlock(deferred_block) => {
                ingest_defer_block(view, *deferred_block, job);
            }
            t::R3Node::Icu(icu) => {
                ingest_icu(view, *icu, job);
            }
            t::R3Node::ForLoopBlock(for_loop) => {
                ingest_for_block(view, *for_loop, job);
            }
            t::R3Node::LetDeclaration(let_decl) => {
                ingest_let_declaration(view, *let_decl, job);
            }
            t::R3Node::Component(_) => {
                // TODO: Account for selectorless nodes
            }
            _ => {
                // Unsupported node type
                // TODO: Log warning or error
            }
        }
    }
    drop(iter);
}

use crate::expression_parser::parser::Parser;
use crate::parse_util::ParseLocation;

use crate::schema::dom_element_schema_registry::DomElementSchemaRegistry;
use crate::schema::element_schema_registry::ElementSchemaRegistry;
use crate::template_parser::binding_parser::BindingParser;

pub struct HostBindingInput {
    pub component_name: String,
    pub component_selector: String,
    pub properties: std::collections::HashMap<String, String>,
    pub attributes: std::collections::HashMap<String, Expression>,
    pub events: std::collections::HashMap<String, String>,
}

/// Process a host binding AST and convert it into a `HostBindingCompilationJob` in the intermediate representation.
pub fn ingest_host_binding(
    input: HostBindingInput,
    constant_pool: ConstantPool,
) -> HostBindingCompilationJob {
    let mut job = HostBindingCompilationJob::new(
        input.component_name,
        constant_pool,
        COMPATIBILITY_MODE,
        TemplateCompilationMode::DomOnly,
    );

    // Setup partial binding parser for host bindings
    let expr_parser = Parser::new();
    let schema_registry = DomElementSchemaRegistry::new();
    let errors = Vec::new();
    let mut binding_parser = BindingParser::new(&expr_parser, &schema_registry, errors);

    let dummy_loc = ParseLocation::from_source("".to_string(), "".to_string(), 0, 0, 0);
    let dummy_span = ParseSourceSpan::new(dummy_loc.clone(), dummy_loc);

    // Process properties
    let mut parsed_properties: Vec<ParsedProperty> = Vec::new();
    for (name, expr) in input.properties {
        binding_parser.parse_property_binding(
            &name,
            &expr,
            true,  // is_host
            false, // is_part_of_assignment_binding
            dummy_span.clone(),
            0,    // absolute_offset
            None, // value_span
            &mut vec![],
            &mut parsed_properties,
            dummy_span.clone(),
        );
    }

    for property in parsed_properties {
        // Determine binding kind from property
        let mut binding_kind = ir::BindingKind::Property;
        let mut property_name = property.name.clone();

        // Handle attr.* prefix
        let mut unit = None;
        if property_name.starts_with("attr.") {
            property_name = property_name[5..].to_string();
            binding_kind = ir::BindingKind::Attribute;
        } else if property_name.starts_with("class.") {
            property_name = property_name[6..].to_string();
            binding_kind = ir::BindingKind::ClassName;
        } else if property_name.starts_with("style.") {
            property_name = property_name[6..].to_string();
            binding_kind = ir::BindingKind::StyleProperty;
            // Check for unit suffix in style bindings (e.g. style.width.px)
            if let Some(idx) = property_name.rfind('.') {
                unit = Some(property_name[idx + 1..].to_string());
                property_name = property_name[..idx].to_string();
            }
        }

        // Handle animation bindings
        if property.is_animation {
            binding_kind = ir::BindingKind::Animation;
        } else if property.is_legacy_animation {
            binding_kind = ir::BindingKind::LegacyAnimation;
        }

        // Calculate security contexts
        let security_contexts: Vec<_> = vec![schema_registry.security_context(
            &input.component_selector,
            &property_name,
            binding_kind == ir::BindingKind::Attribute,
        )];

        // Update property name with sanitized name
        let mut property = property;
        property.name = property_name;

        super::ingest_helpers::ingest_dom_property(
            &mut job,
            property,
            binding_kind,
            unit,
            security_contexts,
        );
    }

    // Process attributes
    for (name, expr) in input.attributes {
        // Calculate security contexts for host attribute
        let security_contexts: Vec<_> =
            vec![schema_registry.security_context(&input.component_selector, &name, true)];

        super::ingest_helpers::ingest_host_attribute(&mut job, name, expr, security_contexts);
    }

    // Process events
    let mut parsed_events: Vec<ParsedEvent> = Vec::new();
    for (name, expr) in input.events {
        binding_parser.parse_event(
            &name,
            &expr,
            false, // is_assignment_event
            dummy_span.clone(),
            dummy_span.clone(), // handler_span
            &mut vec![],
            &mut parsed_events,
            None, // key_span
        );
    }

    for event in parsed_events {
        super::ingest_helpers::ingest_host_event(&mut job, event);
    }

    job
}

/// Ingest the nodes of a template AST into the given `ViewCompilationUnit`.
/// Helper to ingest nodes into a ViewCompilationUnit
fn ingest_nodes(
    view: &mut ViewCompilationUnit,
    template: Vec<t::R3Node>,
    job: &mut ComponentCompilationJob,
) {
    ingest_nodes_internal(view, template, job);
}

fn create_dummy_view() -> ViewCompilationUnit {
    ViewCompilationUnit::new(ir::XrefId::new(usize::MAX), None)
}

/// Safely ingest children into a view by using XrefId.
/// Takes the view out of the job to process children efficiently without repeated lookups.
fn ingest_children_into_view(
    job: &mut ComponentCompilationJob,
    view_xref: ir::XrefId,
    children: Vec<t::R3Node>,
) {
    // Check if we are processing the root view BEFORE modifying job.root
    let is_root = view_xref == job.root.xref;

    // Extract the view from the job to allow passing &mut ViewCompilationUnit separately
    let mut view = if is_root {
        std::mem::replace(&mut job.root, create_dummy_view())
    } else {
        job.views
            .shift_remove(&view_xref)
            .unwrap_or_else(|| panic!("View not found: {:?}", view_xref))
    };

    ingest_nodes_internal(&mut view, children, job);

    // Put the view back
    if is_root {
        job.root = view;
    } else {
        job.views.insert(view_xref, view);
    }
}

fn maybe_record_directive_usage(
    job: &mut ComponentCompilationJob,
    tag_name: &str,
    attributes: &[t::TextAttribute],
    inputs: &[t::BoundAttribute],
    outputs: &[t::BoundEvent],
    template_attrs: &[t::TemplateAttr],
) -> bool {
    // Reset and populate temp_selector
    job.temp_selector.reset();
    job.temp_selector.set_element(tag_name);

    // Add attributes
    for attr in attributes {
        job.temp_selector.add_attribute(&attr.name, &attr.value);
    }

    // Add inputs as attributes (Angular matching rule: bindings are also attributes)
    for input in inputs {
        job.temp_selector.add_attribute(&input.name, "");
    }

    // Add outputs as attributes
    for output in outputs {
        job.temp_selector.add_attribute(&output.name, "");
    }

    // Add template attrs
    for attr in template_attrs {
        match attr {
            t::TemplateAttr::Text(a) => job.temp_selector.add_attribute(&a.name, &a.value),
            t::TemplateAttr::Bound(a) => job.temp_selector.add_attribute(&a.name, ""),
            _ => {}
        }
    }

    // Match against available dependencies
    // Split borrows to allow matching
    let component_name = job.component_name().to_string();
    let matcher = &job.selector_matcher;
    let selector = &job.temp_selector;
    let used_dependencies = &mut job.used_dependencies;

    let mut has_directives = false;
    let mut matched_indices = Vec::new();
    matcher.match_selector(selector, |_, &dep_index| {
        matched_indices.push(dep_index);
        used_dependencies.insert(dep_index);
        has_directives = true;
    });

    if tag_name == "ng-template" {
        eprintln!(
            "[INGEST] matched {} directives for <ng-template>: {:?}",
            matched_indices.len(),
            matched_indices
        );
        for &idx in &matched_indices {
            if let Some(R3TemplateDependencyMetadata::Directive(dir)) =
                job.available_dependencies.get(idx)
            {
                eprintln!("[INGEST]   - matched: {}", dir.selector);
            }
        }
    }

    has_directives
}

/// Ingest an element AST from the template into the given `ViewCompilationUnit`.
fn ingest_element(
    view: &mut ViewCompilationUnit,
    element: t::Element,
    job: &mut ComponentCompilationJob,
) {
    // Check i18n metadata
    if let Some(ref i18n_meta) = element.i18n {
        match i18n_meta {
            I18nMeta::Message(_) | I18nMeta::Node(I18nNode::TagPlaceholder(_)) => {
                // Valid i18n metadata types
            }
            _ => {
                panic!("Unhandled i18n metadata type for element");
            }
        }
    }

    let i18n_placeholder = match &element.i18n {
        Some(I18nMeta::Node(I18nNode::TagPlaceholder(ph))) => Some(ph.clone()),
        _ => None,
    };

    // Split namespace and element name (strip prefix from tag name)
    let (namespace_key, element_name) = match split_ns_name(&element.name, false) {
        Ok((ns, name)) => (ns, name),
        Err(_) => (None, element.name.to_string()),
    };

    let namespace = namespace_for_key(namespace_key.as_deref());

    let id = job.allocate_xref_id();

    // Ingest attributes, inputs, outputs, etc.
    // Note: We need to update ingest_element_bindings to take 'view'
    ingest_element_bindings(view, id, &element, job);

    // Use element_name (stripped namespace) for matching, but keep original for other purposes
    let has_directives = maybe_record_directive_usage(
        job,
        &element_name, // Use stripped name for matching
        &element.attributes,
        &element.inputs,
        &element.outputs,
        &[], // Element does not have template_attrs
    );
    let is_structural = has_directives;

    let mut start_op = ir::ops::create::ElementStartOp::new(
        element_name, // Use stripped name without namespace prefix
        id,
        namespace,
        i18n_placeholder,
        element.start_source_span.clone(),
        element.source_span.clone(),
    );
    start_op.base.has_directives = is_structural;
    let handle = start_op.base.base.handle.clone();
    view.create.push(Box::new(start_op));

    // Ingest references
    ingest_references(view, id, &element, job);

    // Ingest element events
    ingest_element_events(view, id, &element.name, &element, handle, job);

    // Handle i18n start if needed
    let mut i18n_block_id: Option<ir::XrefId> = None;
    if let Some(I18nMeta::Message(msg)) = &element.i18n {
        i18n_block_id = Some(job.allocate_xref_id());
        // Create i18n start op
        let i18n_start_op = ir::ops::create::create_i18n_start_op(
            i18n_block_id.unwrap(),
            msg.clone(),
            Some(id),
            Some(element.start_source_span.clone()),
        );
        view.create.push(i18n_start_op);
    }

    // Ingest children
    ingest_nodes(view, element.children, job);

    // Handle i18n end if needed (before element end)
    if let Some(i18n_id) = i18n_block_id {
        let i18n_end_op =
            ir::ops::create::create_i18n_end_op(i18n_id, element.end_source_span.clone());
        view.create.push(i18n_end_op);
    }

    // Create element end op

    let end_op = ir::ops::create::create_element_end_op(id, element.end_source_span.clone());
    view.create.push(end_op);
}

/// Ingest a template AST node
fn ingest_template(
    view: &mut ViewCompilationUnit,
    tmpl: t::Template,
    job: &mut ComponentCompilationJob,
) {
    // DEBUG: Print tag_name
    if let Some(tag_name) = &tmpl.tag_name {
        eprintln!("DEBUG: ingest_template tag_name: {}", tag_name);
    } else {
        eprintln!("DEBUG: ingest_template tag_name: None");
    }

    // Check i18n metadata
    if let Some(ref i18n_meta) = tmpl.i18n {
        match i18n_meta {
            I18nMeta::Message(_) | I18nMeta::Node(I18nNode::TagPlaceholder(_)) => {
                // Valid i18n metadata types
            }
            _ => {
                panic!("Unhandled i18n metadata type for template");
            }
        }
    }

    // Allocate View ID for valid view (use view.xref as parent)
    let child_view_xref = job.allocate_view(Some(view.xref));

    let (namespace_prefix, tag_name_without_namespace) = if let Some(ref tag_name) = tmpl.tag_name {
        match split_ns_name(tag_name, false) {
            Ok((ns, name)) => (ns, Some(name)),
            Err(_) => (None, Some(tag_name.to_string())),
        }
    } else {
        (None, None)
    };

    let i18n_placeholder = match &tmpl.i18n {
        Some(I18nMeta::Node(I18nNode::TagPlaceholder(ph))) => Some(ph.clone()),
        _ => None,
    };

    let namespace = namespace_for_key(namespace_prefix.as_deref());
    let function_name_suffix = tag_name_without_namespace
        .as_ref()
        .map(|tag| prefix_with_namespace(tag, namespace))
        .unwrap_or_default();

    let template_kind = if is_plain_template(&tmpl) {
        ir::TemplateKind::NgTemplate
    } else {
        ir::TemplateKind::Structural
    };
    // Record directive usage for templates (structural directives)
    let has_directives = maybe_record_directive_usage(
        job,
        tag_name_without_namespace.as_deref().unwrap_or_default(),
        &tmpl.attributes,
        &tmpl.inputs,
        &tmpl.outputs,
        &tmpl.template_attrs,
    );

    let mut template_op_struct = ir::ops::create::TemplateOp::new(
        child_view_xref,
        template_kind,
        tag_name_without_namespace.clone(),
        function_name_suffix.clone(),
        namespace,
        i18n_placeholder.clone(),
        tmpl.start_source_span.clone(),
        tmpl.source_span.clone(),
    );
    template_op_struct.base.has_directives = has_directives;
    if template_kind == ir::TemplateKind::NgTemplate && has_directives {
        eprintln!(
            "[INGEST_TMPL] Created NgTemplate OP with has_directives=true for xref {:?}",
            child_view_xref
        );
    }

    // Get handle from struct before boxing based on trait
    let handle = template_op_struct.base.base.handle.clone();

    // Convert to boxed CreateOp
    // We need to match the return type of create_template_op which is Box<dyn CreateOp>
    // but here we are pushing to view.create which accepts Box<dyn CreateOp>
    view.create.push(Box::new(template_op_struct));

    // Ingest template bindings, events, and references
    ingest_template_bindings(view, child_view_xref, &tmpl, template_kind, job);
    ingest_template_events(
        view,
        child_view_xref,
        tag_name_without_namespace.as_deref(),
        &tmpl.outputs,
        template_kind,
        handle,
        job,
    );
    ingest_references_template(view, child_view_xref, &tmpl, job);

    // Ingest children into child view
    let variables_to_add: Vec<_> = tmpl
        .variables
        .iter()
        .map(|v| {
            let value = if v.value.is_empty() {
                "$implicit".to_string()
            } else {
                v.value.to_string()
            };
            (v.name.to_string(), value)
        })
        .collect();

    // Recurse using helper (handles skipping lookup)
    ingest_children_into_view(job, child_view_xref, tmpl.children);

    // Set up context variables (need to access child view via job)
    {
        let child_view = job.views.get_mut(&child_view_xref).unwrap();
        for (name, value) in variables_to_add {
            child_view.context_variables.insert(name, value);
        }
    }

    // Handle i18n for plain templates
    if template_kind == ir::TemplateKind::NgTemplate {
        if let Some(I18nMeta::Message(msg)) = &tmpl.i18n {
            let id = job.allocate_xref_id();

            // Insert i18n start op at index 0 (after head) inside child view
            let i18n_start_op = ir::ops::create::create_i18n_start_op(
                id,
                msg.clone(),
                None, // i18n_context
                Some(tmpl.start_source_span.clone()),
            );

            // To mutate child view, we access via job
            let child_view = job.views.get_mut(&child_view_xref).unwrap();
            child_view.create.insert_at(0, i18n_start_op);

            // Insert i18n end op at the end
            let end_span = tmpl
                .end_source_span
                .as_ref()
                .unwrap_or(&tmpl.start_source_span)
                .clone();
            let i18n_end_op = ir::ops::create::create_i18n_end_op(id, Some(end_span));

            // Re-borrow child view (borrow checker might complain if we held it?)
            // No, scope ended. Re-acquire.
            let child_view = job.views.get_mut(&child_view_xref).unwrap();
            child_view.create.push(i18n_end_op);
        }
    }
}

/// Ingest a content (ng-content) node
fn ingest_content(
    view: &mut ViewCompilationUnit,
    content: t::Content,
    job: &mut ComponentCompilationJob,
) {
    // Check i18n metadata
    if let Some(ref i18n_meta) = content.i18n {
        match i18n_meta {
            I18nMeta::Node(I18nNode::TagPlaceholder(_)) => {
                // OK
            }
            _ => {
                // TODO: Log error or panic
                panic!("Unhandled i18n metadata type for content: {:?}", i18n_meta);
            }
        }
    }

    let mut fallback_view: Option<ir::XrefId> = None;

    // Don't capture default content that's only made up of empty text nodes and comments.
    let has_non_empty_content = content.children.iter().any(|child| match child {
        t::R3Node::Comment(_) => false,
        t::R3Node::Text(text) => !text.value.trim().is_empty(),
        _ => true,
    });

    if has_non_empty_content {
        let fallback_view_xref = job.allocate_view(Some(view.xref));
        // Take ownership - no clone needed!
        // Recurse using helper
        ingest_children_into_view(job, fallback_view_xref, content.children);
        fallback_view = Some(fallback_view_xref);
    }

    let id = job.allocate_xref_id();
    let i18n_placeholder = match &content.i18n {
        Some(I18nMeta::Node(I18nNode::TagPlaceholder(ph))) => Some(ph.clone()),
        _ => None,
    };

    // Get the source order for this projection (tracks template source order)
    let source_order = job.next_projection_order;
    job.next_projection_order += 1;

    let mut op = ir::ops::create::create_projection_op(
        id,
        content.selector.to_string(),
        i18n_placeholder,
        fallback_view,
        content.source_span.clone(),
    );

    // Set the source order on the projection op
    if let Some(proj_op) = op
        .as_any_mut()
        .downcast_mut::<ir::ops::create::ProjectionOp>()
    {
        proj_op.source_order = source_order;
    }

    view.create.push(op);

    // Ingest content attributes as bindings
    for attr in content.attributes {
        let security_context =
            job.schema_registry
                .security_context("ng-content", &attr.name, false);

        let expression = crate::output::output_ast::Expression::Literal(
            crate::output::output_ast::LiteralExpr {
                value: crate::output::output_ast::LiteralValue::String(attr.value.to_string()),
                type_: None,
                source_span: Some(attr.source_span.clone()),
            },
        );

        let binding_op = ir::ops::update::create_binding_op(
            id,
            ir::BindingKind::Attribute,
            attr.name,
            ir::ops::update::BindingExpression::Expression(expression),
            None,
            vec![security_context],
            true,
            false,
            None,
            None, // i18n_message
            attr.source_span,
        );
        view.update.push(binding_op);
    }
}

/// Ingest a text node
fn ingest_text(
    view: &mut ViewCompilationUnit,
    text: t::Text,
    icu_placeholder: Option<String>,
    job: &mut ComponentCompilationJob,
) {
    // Skip whitespace-only text nodes to match NGTSC default behavior
    if text.value.trim().is_empty() {
        return;
    }

    let text_op = ir::ops::create::create_text_op(
        job.allocate_xref_id(),
        text.value.to_string(),
        icu_placeholder,
        Some(text.source_span.clone()),
    );
    view.create.push(text_op);
}

/// Convert an interpolation AST to an IR interpolation
fn ingest_interpolation(
    interpolation: &crate::expression_parser::ast::Interpolation,
    i18n: Option<&I18nMeta>,
    job: &mut dyn CompilationJob,
    view_xref: ir::XrefId,
    mut view: Option<&mut ViewCompilationUnit>,
    base_source_span: Option<&ParseSourceSpan>,
) -> ir::ops::update::Interpolation {
    // Extract i18n placeholders
    let i18n_placeholders: Vec<Arc<str>> = match i18n {
        Some(I18nMeta::Node(I18nNode::Container(container))) => container
            .children
            .iter()
            .filter_map(|child| {
                if let I18nNode::Placeholder(ph) = child {
                    Some(ph.name.clone().into())
                } else {
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    };

    let mut converted_expressions: Vec<Expression> =
        Vec::with_capacity(interpolation.expressions.len());
    for expr in &interpolation.expressions {
        let view_param = match view {
            Some(ref mut v) => Some(&mut **v),
            None => None,
        };
        converted_expressions.push(crate::template::pipeline::src::conversion::convert_ast(
            expr,
            job,
            view_xref,
            view_param,
            base_source_span,
        ));
    }

    ir::ops::update::Interpolation::new(
        interpolation
            .strings
            .iter()
            .map(|s| Arc::from(s.as_str()))
            .collect(),
        converted_expressions,
        i18n_placeholders,
    )
}

/// Convert a binding AST to an IR binding expression
fn convert_binding_expression(
    value: &ExprAST,
    i18n: Option<&I18nMeta>,
    job: &mut dyn CompilationJob,
    view_xref: ir::XrefId,
    view: Option<&mut ViewCompilationUnit>,
    base_source_span: Option<&ParseSourceSpan>,
) -> ir::ops::update::BindingExpression {
    use ir::ops::update::BindingExpression;

    if let ExprAST::Interpolation(interp) = value {
        BindingExpression::Interpolation(ingest_interpolation(
            interp,
            i18n,
            job,
            view_xref,
            view,
            base_source_span,
        ))
    } else {
        BindingExpression::Expression(crate::template::pipeline::src::conversion::convert_ast(
            value,
            job,
            view_xref,
            view,
            base_source_span,
        ))
    }
}

/// Ingest a bound text node
fn ingest_bound_text(
    view: &mut ViewCompilationUnit,
    bound_text: t::BoundText,
    icu_placeholder: Option<String>,
    prefix: String,
    job: &mut ComponentCompilationJob,
) {
    let value = &bound_text.value;
    let interpolation_ast = if let ExprAST::Interpolation(interp) = value {
        interp
    } else {
        panic!(
            "AssertionError: expected Interpolation for BoundText node, got {:?}",
            std::mem::discriminant(value)
        );
    };

    // Validate i18n metadata - should be Container or None
    if let Some(
        I18nMeta::Message(_)
        | I18nMeta::Node(
            I18nNode::IcuPlaceholder(_)
            | I18nNode::BlockPlaceholder(_)
            | I18nNode::TagPlaceholder(_),
        ),
    ) = &bound_text.i18n
    {
        panic!(
            "Unhandled i18n metadata type for text interpolation: {:?}",
            bound_text.i18n
        );
    }

    // Extract i18n placeholders from Container
    let i18n_placeholders: Vec<Arc<str>> = match &bound_text.i18n {
        Some(I18nMeta::Node(I18nNode::Container(container))) => container
            .children
            .iter()
            .filter_map(|child| {
                if let I18nNode::Placeholder(ph) = child {
                    Some(ph.name.clone().into())
                } else {
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    };

    // Validate placeholder count matches expression count
    if !i18n_placeholders.is_empty()
        && i18n_placeholders.len() != interpolation_ast.expressions.len()
    {
        panic!(
            "Unexpected number of i18n placeholders ({}) for BoundText with {} expressions",
            i18n_placeholders.len(),
            interpolation_ast.expressions.len()
        );
    }

    // Create TextOp
    let text_xref = job.allocate_xref_id();
    let text_op = ir::ops::create::create_text_op(
        text_xref,
        prefix, // Use provided prefix as initial value
        icu_placeholder,
        Some(bound_text.source_span.clone()),
    );
    view.create.push(text_op);

    // Convert expressions - use compatibility mode to determine base source span
    let base_source_span =
        if job.compatibility() == ir::CompatibilityMode::TemplateDefinitionBuilder {
            None
        } else {
            Some(&bound_text.source_span)
        };

    // Create Interpolation using helper
    let interpolation = ingest_interpolation(
        interpolation_ast,
        bound_text.i18n.as_ref(),
        job,
        view.xref,
        Some(view),
        base_source_span,
    );

    // Create InterpolateTextOp
    let interpolate_op = ir::ops::update::create_interpolate_text_op(
        text_xref,
        interpolation,
        bound_text.source_span.clone(),
    );
    view.update.push(interpolate_op);
}

/// Ingest an if block
fn ingest_if_block(
    view: &mut ViewCompilationUnit,
    if_block: t::IfBlock,
    job: &mut ComponentCompilationJob,
) {
    use crate::template::pipeline::ir::expression::ConditionalCaseExpr;

    let mut first_xref: Option<ir::XrefId> = None;
    let mut conditions: Vec<ConditionalCaseExpr> = Vec::new();

    for (i, branch) in if_block.branches.into_iter().enumerate() {
        // Take ownership of children - no clone needed!
        let children = branch.children;

        let c_view_xref = job.allocate_view(Some(view.xref));

        // Extract tag name from single root element/template for content projection
        let tag_name = ingest_control_flow_insertion_point_from_children(view, job, &children);

        // Extract i18n metadata
        let _i18n_placeholder: Option<crate::i18n::i18n_ast::BlockPlaceholder> = match &branch.i18n
        {
            Some(I18nMeta::Node(I18nNode::BlockPlaceholder(ph))) => Some(ph.clone()),
            Some(_) => {
                panic!(
                    "Unhandled i18n metadata type for if block: {:?}",
                    branch.i18n
                );
            }
            None => None,
        };

        // Create conditional op (first branch is ConditionalCreate, others are ConditionalBranchCreate)
        // We need to get the handle from the op before boxing it
        let (conditional_op, conditional_handle) = if i == 0 {
            let op = ir::ops::create::ConditionalCreateOp::new(
                c_view_xref,
                ir::TemplateKind::Block,
                tag_name.clone(),
                String::from("Conditional"),
                ir::Namespace::HTML,
                // BlockPlaceholder will be converted to TagPlaceholder in resolve_i18n_element_placeholders phase
                // For now, pass None - the placeholder is stored in branch.i18n and will be processed later
                None,
                branch.block.start_source_span.clone(),
                branch.block.source_span.clone(),
            );
            let handle = op.base.base.handle.clone(); // Get handle value directly (SlotHandle is Copy)
            (Box::new(op) as Box<dyn ir::CreateOp + Send + Sync>, handle)
        } else {
            let op = ir::ops::create::ConditionalBranchCreateOp::new(
                c_view_xref,
                ir::TemplateKind::Block,
                tag_name.clone(),
                String::from("Conditional"),
                ir::Namespace::HTML,
                // BlockPlaceholder will be converted to TagPlaceholder in resolve_i18n_element_placeholders phase
                // For now, pass None - the placeholder is stored in branch.i18n and will be processed later
                None,
                branch.block.start_source_span.clone(),
                branch.block.source_span.clone(),
            );
            let handle = op.base.base.handle.clone(); // Get handle value directly (SlotHandle is Copy)
            (Box::new(op) as Box<dyn ir::CreateOp + Send + Sync>, handle)
        };

        view.create.push(conditional_op);

        if first_xref.is_none() {
            first_xref = Some(c_view_xref);
        }

        // Get view and set expression alias if present
        if let Some(ref expr_alias) = branch.expression_alias {
            if let Some(c_view) = job.views.get_mut(&c_view_xref) {
                c_view.context_variables.insert(
                    expr_alias.name.to_string(),
                    ir::variable::CTX_REF.to_string(),
                );
            }
        }

        // Convert expression if present
        let case_expr = branch.expression.as_ref().map(|expr| {
            Box::new(crate::template::pipeline::src::conversion::convert_ast(
                expr,
                job,
                view.xref,
                Some(&mut *view),
                None,
            ))
        });

        // Create ConditionalCaseExpr
        let conditional_case_expr = ConditionalCaseExpr::new(
            case_expr,
            c_view_xref,
            conditional_handle,
            branch.expression_alias,
        );
        conditions.push(conditional_case_expr);

        // Ingest children into conditional view
        ingest_children_into_view(job, c_view_xref, children);
    }

    // Create ConditionalOp update operation
    let conditional_update_op = ir::ops::update::create_conditional_op(
        first_xref.expect("If block must have at least one branch"),
        None, // If blocks don't have a test expression
        conditions,
        if_block.block.source_span.clone(),
    );
    view.update.push(conditional_update_op);
}

/// Ingest a switch block
fn ingest_switch_block(
    view: &mut ViewCompilationUnit,
    switch_block: t::SwitchBlock,
    job: &mut ComponentCompilationJob,
) {
    use crate::template::pipeline::ir::expression::ConditionalCaseExpr;

    // Don't ingest empty switches since they won't render anything.
    if switch_block.cases.is_empty() {
        return;
    }

    let mut first_xref: Option<ir::XrefId> = None;
    let mut conditions: Vec<ConditionalCaseExpr> = Vec::new();

    for (i, case) in switch_block.cases.into_iter().enumerate() {
        // Take ownership of children - no clone needed!
        let children = case.children;

        let c_view_xref = job.allocate_view(Some(view.xref));

        // Extract tag name from single root element/template for content projection
        let tag_name = ingest_control_flow_insertion_point_from_children(view, job, &children);

        // Extract i18n metadata
        let _i18n_placeholder: Option<crate::i18n::i18n_ast::BlockPlaceholder> = match &case.i18n {
            Some(I18nMeta::Node(I18nNode::BlockPlaceholder(ph))) => Some(ph.clone()),
            Some(_) => {
                panic!(
                    "Unhandled i18n metadata type for switch block: {:?}",
                    case.i18n
                );
            }
            None => None,
        };

        // Create conditional op (first case is ConditionalCreate, others are ConditionalBranchCreate)
        let (conditional_op, conditional_handle) = if i == 0 {
            let op = ir::ops::create::ConditionalCreateOp::new(
                c_view_xref,
                ir::TemplateKind::Block,
                tag_name.clone(),
                String::from("Case"),
                ir::Namespace::HTML,
                // BlockPlaceholder will be converted to TagPlaceholder in resolve_i18n_element_placeholders phase
                // For now, pass None - the placeholder is stored in branch.i18n and will be processed later
                None,
                case.block.start_source_span.clone(),
                case.block.source_span.clone(),
            );
            let handle = op.base.base.handle.clone();
            (Box::new(op) as Box<dyn ir::CreateOp + Send + Sync>, handle)
        } else {
            let op = ir::ops::create::ConditionalBranchCreateOp::new(
                c_view_xref,
                ir::TemplateKind::Block,
                tag_name.clone(),
                String::from("Case"),
                ir::Namespace::HTML,
                // BlockPlaceholder will be converted to TagPlaceholder in resolve_i18n_element_placeholders phase
                // For now, pass None - the placeholder is stored in branch.i18n and will be processed later
                None,
                case.block.start_source_span.clone(),
                case.block.source_span.clone(),
            );
            let handle = op.base.base.handle.clone();
            (Box::new(op) as Box<dyn ir::CreateOp + Send + Sync>, handle)
        };

        view.create.push(conditional_op);

        if first_xref.is_none() {
            first_xref = Some(c_view_xref);
        }

        // Convert expression if present
        let case_expr = case.expression.as_ref().map(|expr| {
            Box::new(crate::template::pipeline::src::conversion::convert_ast(
                expr,
                job,
                view.xref,
                Some(&mut *view),
                Some(&switch_block.block.start_source_span),
            ))
        });

        // Create ConditionalCaseExpr
        let conditional_case_expr = ConditionalCaseExpr::new(
            case_expr,
            c_view_xref,
            conditional_handle,
            None, // Switch cases don't have expression aliases
        );
        conditions.push(conditional_case_expr);

        // Ingest children into conditional view
        ingest_children_into_view(job, c_view_xref, children);
    }

    // Convert switch expression
    let switch_span = switch_block.block.source_span.clone();
    let switch_expr = crate::template::pipeline::src::conversion::convert_ast(
        &switch_block.expression,
        job,
        view.xref,
        Some(&mut *view),
        Some(&switch_span),
    );

    // Create ConditionalOp update operation with switch expression
    let conditional_update_op = ir::ops::update::create_conditional_op(
        first_xref.expect("Switch block must have at least one case"),
        Some(switch_expr),
        conditions,
        switch_block.block.source_span.clone(),
    );
    view.update.push(conditional_update_op);
}

/// Ingest a deferred block
fn ingest_defer_block(
    view: &mut ViewCompilationUnit,
    deferred_block: t::DeferredBlock,
    job: &mut ComponentCompilationJob,
) {
    let defer_xref = job.allocate_xref_id();

    // 1. Ingest main block
    let main_view_xref = job.allocate_view(Some(view.xref));

    // Extract children for main view
    let main_children = deferred_block.children.clone();

    // Ingest children into main view
    ingest_children_into_view(job, main_view_xref, main_children);

    // Resolver (not yet supported in Rust AST?)
    let own_resolver_fn = None;

    // 2. Ingest @loading block if present
    let (loading_view, loading_minimum_time, loading_after_time) =
        if let Some(loading) = &deferred_block.loading {
            let view_xref = job.allocate_view(Some(view.xref));
            ingest_children_into_view(job, view_xref, loading.children.clone());

            let min = loading.minimum_time.map(|t| t as f64);
            let after = loading.after_time.map(|t| t as f64);
            (Some(view_xref), min, after)
        } else {
            (None, None, None)
        };

    // 3. Ingest @placeholder block if present
    let (placeholder_view, placeholder_minimum_time) =
        if let Some(placeholder) = &deferred_block.placeholder {
            let view_xref = job.allocate_view(Some(view.xref));
            ingest_children_into_view(job, view_xref, placeholder.children.clone());

            let min = placeholder.minimum_time.map(|t| t as f64);
            (Some(view_xref), min)
        } else {
            (None, None)
        };

    // 4. Ingest @error block if present
    let error_view = if let Some(error) = &deferred_block.error {
        let view_xref = job.allocate_view(Some(view.xref));
        ingest_children_into_view(job, view_xref, error.children.clone());

        Some(view_xref)
    } else {
        None
    };

    // Create DeferOp
    let mut defer_op = ir::ops::create::DeferOp::new(
        defer_xref,
        main_view_xref,
        ir::handle::SlotHandle::with_slot(0), // main_slot - TODO: extract from view
        own_resolver_fn,
        job.all_deferrable_deps_fn.clone(), // resolver_fn
        deferred_block.block.source_span.clone(),
    );

    // Set secondary views and their slots
    defer_op.placeholder_view = placeholder_view;
    defer_op.placeholder_slot = placeholder_view.map(|_| ir::handle::SlotHandle::with_slot(0)); // TODO: extract
    defer_op.loading_view = loading_view;
    defer_op.loading_slot = loading_view.map(|_| ir::handle::SlotHandle::with_slot(0)); // TODO: extract
    defer_op.error_view = error_view;
    defer_op.error_slot = error_view.map(|_| ir::handle::SlotHandle::with_slot(0)); // TODO: extract

    // Set minimum times
    defer_op.placeholder_minimum_time = placeholder_minimum_time;
    defer_op.loading_minimum_time = loading_minimum_time;
    defer_op.loading_after_time = loading_after_time;

    // Calculate flags
    if !deferred_block.hydrate_triggers.when.is_none()
        || !deferred_block.hydrate_triggers.idle.is_none()
        || !deferred_block.hydrate_triggers.immediate.is_none()
        || !deferred_block.hydrate_triggers.timer.is_none()
        || !deferred_block.hydrate_triggers.hover.is_none()
        || !deferred_block.hydrate_triggers.interaction.is_none()
        || !deferred_block.hydrate_triggers.viewport.is_none()
        || !deferred_block.hydrate_triggers.never.is_none()
    {
        defer_op.flags = Some(1); // HasHydrateTriggers flag
    }

    view.create.push(Box::new(defer_op));

    // Ingest defer triggers (on, when, etc.)
    // Use vectors to collect ops before pushing
    let mut defer_on_ops: Vec<Box<dyn ir::CreateOp + Send + Sync>> = Vec::new();
    let mut defer_when_ops: Vec<Box<dyn ir::UpdateOp + Send + Sync>> = Vec::new();

    // Ingest hydrate triggers
    ingest_defer_triggers(
        view.xref,
        ir::enums::DeferOpModifierKind::Hydrate,
        &deferred_block.hydrate_triggers,
        &mut defer_on_ops,
        &mut defer_when_ops,
        defer_xref,
        Some(&mut *view),
        job,
    );

    // Ingest regular triggers
    ingest_defer_triggers(
        view.xref,
        ir::enums::DeferOpModifierKind::None,
        &deferred_block.triggers,
        &mut defer_on_ops,
        &mut defer_when_ops,
        defer_xref,
        Some(&mut *view),
        job,
    );

    // Ingest prefetch triggers
    ingest_defer_triggers(
        view.xref,
        ir::enums::DeferOpModifierKind::Prefetch,
        &deferred_block.prefetch_triggers,
        &mut defer_on_ops,
        &mut defer_when_ops,
        defer_xref,
        Some(&mut *view),
        job,
    );

    // If no concrete (non-prefetch, non-hydrate) triggers were provided, default to 'idle'
    // Check if we added any None-modifier triggers.
    // If deferred_block.triggers is essentially empty (all fields None).
    let has_concrete_triggers = deferred_block.triggers.idle.is_some()
        || deferred_block.triggers.immediate.is_some()
        || deferred_block.triggers.timer.is_some()
        || deferred_block.triggers.hover.is_some()
        || deferred_block.triggers.interaction.is_some()
        || deferred_block.triggers.viewport.is_some()
        || deferred_block.triggers.never.is_some()
        || deferred_block.triggers.when.is_some();

    if !has_concrete_triggers {
        let idle_op = ir::ops::create::create_defer_on_op(
            defer_xref,
            ir::ops::create::DeferTrigger::Idle,
            ir::enums::DeferOpModifierKind::None,
            deferred_block.block.source_span.clone(),
        );
        defer_on_ops.push(idle_op);
    }

    // Push converted ops
    for op in defer_on_ops {
        view.create.push(op);
    }
    for op in defer_when_ops {
        view.update.push(op);
    }
}

/// Ingest defer triggers and create DeferOnOp/DeferWhenOp operations
fn ingest_defer_triggers(
    unit_xref: ir::XrefId,
    modifier: ir::enums::DeferOpModifierKind,
    triggers: &t::DeferredBlockTriggers,
    defer_on_ops: &mut Vec<Box<dyn ir::CreateOp + Send + Sync>>,
    defer_when_ops: &mut Vec<Box<dyn ir::UpdateOp + Send + Sync>>,
    defer_xref: ir::XrefId,
    mut external_view: Option<&mut ViewCompilationUnit>,
    job: &mut ComponentCompilationJob,
) {
    use crate::template::pipeline::ir::ops::create::DeferTrigger as IRDeferTrigger;
    use crate::template::pipeline::src::conversion::convert_ast;

    // Handle idle trigger
    if let Some(ref idle) = triggers.idle {
        let defer_on_op = ir::ops::create::create_defer_on_op(
            defer_xref,
            IRDeferTrigger::Idle,
            modifier,
            idle.source_span.clone(),
        );
        defer_on_ops.push(defer_on_op);
    }

    // Handle immediate trigger
    if let Some(ref immediate) = triggers.immediate {
        let defer_on_op = ir::ops::create::create_defer_on_op(
            defer_xref,
            IRDeferTrigger::Immediate,
            modifier,
            immediate.source_span.clone(),
        );
        defer_on_ops.push(defer_on_op);
    }

    // Handle timer trigger
    if let Some(ref timer) = triggers.timer {
        let defer_on_op = ir::ops::create::create_defer_on_op(
            defer_xref,
            IRDeferTrigger::Timer {
                delay: timer.delay as f64,
            },
            modifier,
            timer.source_span.clone(),
        );
        defer_on_ops.push(defer_on_op);
    }

    // Handle hover trigger
    if let Some(ref hover) = triggers.hover {
        let defer_on_op = ir::ops::create::create_defer_on_op(
            defer_xref,
            IRDeferTrigger::Hover {
                target_name: hover.reference.as_ref().map(|r| r.to_string()),
                target_xref: None, // Will be resolved later
                target_slot: None,
                target_view: None,
                target_slot_view_steps: None,
            },
            modifier,
            hover.source_span.clone(),
        );
        defer_on_ops.push(defer_on_op);
    }

    // Handle interaction trigger
    if let Some(ref interaction) = triggers.interaction {
        let defer_on_op = ir::ops::create::create_defer_on_op(
            defer_xref,
            IRDeferTrigger::Interaction {
                target_name: interaction.reference.as_ref().map(|r| r.to_string()),
                target_xref: None, // Will be resolved later
                target_slot: None,
                target_view: None,
                target_slot_view_steps: None,
            },
            modifier,
            interaction.source_span.clone(),
        );
        defer_on_ops.push(defer_on_op);
    }

    // Handle viewport trigger
    if let Some(ref viewport) = triggers.viewport {
        let options_expr = None; // TODO: convert viewport options

        let defer_on_op = ir::ops::create::create_defer_on_op(
            defer_xref,
            IRDeferTrigger::Viewport {
                target_name: viewport.reference.as_ref().map(|r| r.to_string()),
                target_xref: None, // Will be resolved later
                target_slot: None,
                target_view: None,
                target_slot_view_steps: None,
                options: options_expr,
            },
            modifier,
            viewport.source_span.clone(),
        );
        defer_on_ops.push(defer_on_op);
    }

    // Handle never trigger
    if let Some(ref never) = triggers.never {
        let defer_on_op = ir::ops::create::create_defer_on_op(
            defer_xref,
            IRDeferTrigger::Never,
            modifier,
            never.source_span.clone(),
        );
        defer_on_ops.push(defer_on_op);
    }

    // Handle when trigger (creates DeferWhenOp, not DeferOnOp)
    if let Some(ref when) = triggers.when {
        let span = when.source_span.clone();
        let expr = convert_ast(
            &when.value,
            job,
            unit_xref,
            external_view.as_deref_mut(),
            Some(&span),
        );

        let defer_when_op = ir::ops::update::create_defer_when_op(
            defer_xref,
            expr,
            modifier,
            when.source_span.clone(),
        );
        defer_when_ops.push(defer_when_op);
    }
}
/// Helper function to extract ICU name from i18n message
/// Equivalent to TypeScript's icuFromI18nMessage(message).name
fn icu_from_i18n_message(message: &crate::i18n::i18n_ast::Message) -> Option<String> {
    use crate::i18n::i18n_ast::Node as I18nNode;

    // Get first node which should be IcuPlaceholder for single ICU
    if let Some(I18nNode::IcuPlaceholder(icu_ph)) = message.nodes.first() {
        Some(icu_ph.name.clone())
    } else {
        None
    }
}

/// Ingest an ICU node
fn ingest_icu(view: &mut ViewCompilationUnit, icu: t::Icu, job: &mut ComponentCompilationJob) {
    // Check if this is a single i18n ICU
    if let Some(I18nMeta::Message(ref msg)) = icu.i18n {
        if is_single_i18n_icu(icu.i18n.as_ref()) {
            let xref = job.allocate_xref_id();

            // Extract ICU name from message
            // In TypeScript: icuFromI18nMessage(icu.i18n).name
            let icu_name = icu_from_i18n_message(msg).unwrap_or_else(|| format!("ICU_{:?}", xref));

            // Create IcuStartOp
            let icu_start_op = ir::ops::create::create_icu_start_op(
                xref,
                msg.clone(),
                icu_name.clone(),
                icu.source_span.clone(),
            );
            view.create.push(icu_start_op);

            // Ingest ICU variables and placeholders
            // Iterate over icu.vars (BoundText) - pass placeholder name
            for (placeholder, bound_text) in &icu.vars {
                ingest_bound_text(
                    view,
                    bound_text.clone(),
                    Some(placeholder.to_string()),
                    String::new(),
                    job,
                );
            }

            // Iterate over icu.placeholders (IcuPlaceholder enum) - pass placeholder name
            for (placeholder, icu_ph) in &icu.placeholders {
                match icu_ph {
                    t::IcuPlaceholder::Text(text) => {
                        ingest_text(view, text.clone(), Some(placeholder.to_string()), job);
                    }
                    t::IcuPlaceholder::BoundText(bound_text) => {
                        ingest_bound_text(
                            view,
                            bound_text.clone(),
                            Some(placeholder.to_string()),
                            String::new(),
                            job,
                        );
                    }
                }
            }

            // Create IcuEndOp
            let icu_end_op = ir::ops::create::create_icu_end_op(xref);
            view.create.push(icu_end_op);
        } else {
            panic!("ICU must be a single i18n ICU");
        }
    } else {
        panic!("Unhandled i18n metadata type for ICU: {:?}", icu.i18n);
    }
}

/// Gets an expression that represents a variable in an `@for` loop.
/// @param variable AST representing the variable.
/// @param index_name Loop-specific name for `$index`.
/// @param count_name Loop-specific name for `$count`.
fn get_computed_for_loop_variable_expression(
    variable: &t::Variable,
    index_name: &str,
    count_name: &str,
) -> Expression {
    use crate::output::output_ast::BinaryOperator;
    use crate::template::pipeline::ir::expression::LexicalReadExpr;

    match variable.value.as_ref() {
        "$index" => Expression::LexicalRead(LexicalReadExpr::new(index_name.to_string())),
        "$count" => Expression::LexicalRead(LexicalReadExpr::new(count_name.to_string())),
        "$first" => {
            // $index === 0
            Expression::BinaryOp(crate::output::output_ast::BinaryOperatorExpr {
                operator: BinaryOperator::Identical,
                lhs: Box::new(Expression::LexicalRead(LexicalReadExpr::new(
                    index_name.to_string(),
                ))),
                rhs: Box::new(Expression::Literal(
                    crate::output::output_ast::LiteralExpr {
                        value: crate::output::output_ast::LiteralValue::Number(0.0),
                        type_: None,
                        source_span: None,
                    },
                )),
                type_: None,
                source_span: None,
            })
        }
        "$last" => {
            // $index === $count - 1
            Expression::BinaryOp(crate::output::output_ast::BinaryOperatorExpr {
                operator: BinaryOperator::Identical,
                lhs: Box::new(Expression::LexicalRead(LexicalReadExpr::new(
                    index_name.to_string(),
                ))),
                rhs: Box::new(Expression::BinaryOp(
                    crate::output::output_ast::BinaryOperatorExpr {
                        operator: BinaryOperator::Minus,
                        lhs: Box::new(Expression::LexicalRead(LexicalReadExpr::new(
                            count_name.to_string(),
                        ))),
                        rhs: Box::new(Expression::Literal(
                            crate::output::output_ast::LiteralExpr {
                                value: crate::output::output_ast::LiteralValue::Number(1.0),
                                type_: None,
                                source_span: None,
                            },
                        )),
                        type_: None,
                        source_span: None,
                    },
                )),
                type_: None,
                source_span: None,
            })
        }
        "$even" => {
            // ($index % 2) === 0
            Expression::BinaryOp(crate::output::output_ast::BinaryOperatorExpr {
                operator: BinaryOperator::Identical,
                lhs: Box::new(Expression::BinaryOp(
                    crate::output::output_ast::BinaryOperatorExpr {
                        operator: BinaryOperator::Modulo,
                        lhs: Box::new(Expression::LexicalRead(LexicalReadExpr::new(
                            index_name.to_string(),
                        ))),
                        rhs: Box::new(Expression::Literal(
                            crate::output::output_ast::LiteralExpr {
                                value: crate::output::output_ast::LiteralValue::Number(2.0),
                                type_: None,
                                source_span: None,
                            },
                        )),
                        type_: None,
                        source_span: None,
                    },
                )),
                rhs: Box::new(Expression::Literal(
                    crate::output::output_ast::LiteralExpr {
                        value: crate::output::output_ast::LiteralValue::Number(0.0),
                        type_: None,
                        source_span: None,
                    },
                )),
                type_: None,
                source_span: None,
            })
        }
        "$odd" => {
            // ($index % 2) !== 0
            Expression::BinaryOp(crate::output::output_ast::BinaryOperatorExpr {
                operator: BinaryOperator::NotIdentical,
                lhs: Box::new(Expression::BinaryOp(
                    crate::output::output_ast::BinaryOperatorExpr {
                        operator: BinaryOperator::Modulo,
                        lhs: Box::new(Expression::LexicalRead(LexicalReadExpr::new(
                            index_name.to_string(),
                        ))),
                        rhs: Box::new(Expression::Literal(
                            crate::output::output_ast::LiteralExpr {
                                value: crate::output::output_ast::LiteralValue::Number(2.0),
                                type_: None,
                                source_span: None,
                            },
                        )),
                        type_: None,
                        source_span: None,
                    },
                )),
                rhs: Box::new(Expression::Literal(
                    crate::output::output_ast::LiteralExpr {
                        value: crate::output::output_ast::LiteralValue::Number(0.0),
                        type_: None,
                        source_span: None,
                    },
                )),
                type_: None,
                source_span: None,
            })
        }
        _ => {
            panic!(
                "AssertionError: unknown @for loop variable {}",
                variable.value
            );
        }
    }
}

/// Helper to ingest control flow insertion point from children directly
/// With the directive-based control flow users were able to conditionally project content using
/// the `*` syntax. E.g. `<div *ngIf="expr" projectMe></div>` will be projected into
/// `<ng-content select="[projectMe]"/>`, because the attributes and tag name from the `div` are
/// copied to the template via the template creation instruction. With `@if` and `@for` that is
/// not the case, because the conditional is placed *around* elements, rather than *on* them.
/// The result is that content projection won't work in the same way if a user converts from
/// `*ngIf` to `@if`.
///
/// This function aims to cover the most common case by doing the same copying when a control flow
/// node has *one and only one* root element or template node.
///
/// @returns Tag name to be used for the control flow template.
fn ingest_control_flow_insertion_point_from_children(
    view: &mut ViewCompilationUnit,
    job: &mut ComponentCompilationJob,
    children: &[t::R3Node],
) -> Option<String> {
    use crate::i18n::i18n_ast::I18nMeta;
    use crate::schema::dom_element_schema_registry::DomElementSchemaRegistry;
    use crate::schema::element_schema_registry::ElementSchemaRegistry;

    let dom_schema = DomElementSchemaRegistry::new();
    let mut root: Option<&t::R3Node> = None;

    for child in children {
        // Skip over comment nodes and @let declarations since
        // it doesn't matter where they end up in the DOM.
        if matches!(child, t::R3Node::Comment(_)) || matches!(child, t::R3Node::LetDeclaration(_)) {
            continue;
        }

        // We can only infer the tag name/attributes if there's a single root node.
        if root.is_some() {
            return None;
        }

        // Root nodes can only elements or templates with a tag name (e.g. `<div *foo></div>`).
        match child {
            t::R3Node::Element(_) => {
                root = Some(child);
            }
            t::R3Node::Template(tmpl) => {
                if tmpl.tag_name.is_some() {
                    root = Some(child);
                } else {
                    return None;
                }
            }
            _ => {
                return None;
            }
        }
    }

    // If we've found a single root node, its tag name and attributes can be
    // copied to the surrounding template to be used for content projection.
    if let Some(root_node) = root {
        match root_node {
            t::R3Node::Element(el) => {
                // Collect the static attributes for content projection purposes.
                for attr in &el.attributes {
                    if !attr.name.starts_with(ANIMATE_PREFIX) {
                        let security_context =
                            dom_schema.security_context(&el.name, &attr.name, true);
                        let expression = crate::output::output_ast::Expression::Literal(
                            crate::output::output_ast::LiteralExpr {
                                value: crate::output::output_ast::LiteralValue::String(
                                    attr.value.to_string(),
                                ),
                                type_: None,
                                source_span: Some(attr.source_span.clone()),
                            },
                        );

                        // Extract i18n message
                        let i18n_message = match &attr.i18n {
                            Some(I18nMeta::Message(msg)) => Some(msg.clone()),
                            _ => None,
                        };

                        if &*attr.name.as_ref() == "class" {
                            if let crate::output::output_ast::Expression::Literal(lit) = &expression
                            {
                                if let crate::output::output_ast::LiteralValue::String(
                                    class_value,
                                ) = &lit.value
                                {
                                    for class_name in class_value.split_whitespace() {
                                        let extracted_attr_op =
                                            ir::ops::create::create_extracted_attribute_op(
                                                view.xref,
                                                ir::BindingKind::ClassName,
                                                None, // namespace
                                                class_name.to_string().into(),
                                                None, // value
                                                None, // i18n_context
                                                i18n_message.clone(),
                                                vec![security_context.clone()],
                                                None, // source_span - derived from class literal?
                                            );
                                        view.create.push(extracted_attr_op);
                                    }
                                    continue;
                                }
                            }
                        }

                        let extracted_attr_op = ir::ops::create::create_extracted_attribute_op(
                            view.xref,
                            ir::BindingKind::Attribute,
                            None, // namespace
                            attr.name.clone(),
                            Some(expression),
                            None, // i18n_context
                            i18n_message,
                            vec![security_context],
                            Some(attr.source_span.clone()),
                        );
                        view.create.push(extracted_attr_op);
                    }
                }

                // Also collect the inputs since they participate in content projection as well.
                for input in &el.inputs {
                    if input.type_ != crate::expression_parser::ast::BindingType::LegacyAnimation
                        && input.type_ != crate::expression_parser::ast::BindingType::Animation
                        && input.type_ != crate::expression_parser::ast::BindingType::Attribute
                    {
                        let security_context =
                            dom_schema.security_context(&NG_TEMPLATE_TAG_NAME, &input.name, true);
                        let extracted_attr_op = ir::ops::create::create_extracted_attribute_op(
                            view.xref,
                            ir::BindingKind::Property,
                            None, // name
                            input.name.clone(),
                            None, // value
                            None, // i18n_context
                            None, // i18n_message
                            vec![security_context],
                            Some(input.source_span.clone()),
                        );
                        view.create.push(extracted_attr_op);
                    }
                }

                let tag_name = el.name.clone();

                // Don't pass along `ng-template` tag name since it enables directive matching.
                if &*tag_name == NG_TEMPLATE_TAG_NAME {
                    return None;
                }
                return Some(tag_name.to_string());
            }
            t::R3Node::Template(tmpl) => {
                // Similar logic for template nodes
                // Collect attributes
                for attr in &tmpl.attributes {
                    if !attr.name.starts_with(ANIMATE_PREFIX) {
                        let security_context =
                            dom_schema.security_context(&NG_TEMPLATE_TAG_NAME, &attr.name, true);
                        let expression = crate::output::output_ast::Expression::Literal(
                            crate::output::output_ast::LiteralExpr {
                                value: crate::output::output_ast::LiteralValue::String(
                                    attr.value.to_string(),
                                ),
                                type_: None,
                                source_span: Some(attr.source_span.clone()),
                            },
                        );

                        // Extract i18n message
                        let i18n_message = match &attr.i18n {
                            Some(I18nMeta::Message(msg)) => Some(msg.clone()),
                            _ => None,
                        };

                        let extracted_attr_op = ir::ops::create::create_extracted_attribute_op(
                            view.xref,
                            ir::BindingKind::Attribute,
                            None, // namespace
                            attr.name.clone(),
                            Some(expression),
                            None, // i18n_context
                            i18n_message,
                            vec![security_context],
                            Some(attr.source_span.clone()),
                        );
                        view.create.push(extracted_attr_op);
                    }
                }

                // Collect inputs
                for input in &tmpl.inputs {
                    if input.type_ != crate::expression_parser::ast::BindingType::LegacyAnimation
                        && input.type_ != crate::expression_parser::ast::BindingType::Animation
                        && input.type_ != crate::expression_parser::ast::BindingType::Attribute
                    {
                        let security_context =
                            dom_schema.security_context(&NG_TEMPLATE_TAG_NAME, &input.name, true);
                        let extracted_attr_op = ir::ops::create::create_extracted_attribute_op(
                            view.xref,
                            ir::BindingKind::Property,
                            None,
                            input.name.clone(),
                            None,
                            None, // i18n_context
                            None, // i18n_message
                            vec![security_context],
                            Some(input.source_span.clone()),
                        );
                        view.create.push(extracted_attr_op);
                    }
                }

                if let Some(tag_name) = &tmpl.tag_name {
                    if tag_name.as_ref() == NG_TEMPLATE_TAG_NAME {
                        return None;
                    }
                    return Some(tag_name.to_string());
                }
            }
            _ => {}
        }
    }

    None
}

/// Ingest a for loop block
fn ingest_for_block(
    view: &mut ViewCompilationUnit,
    for_loop: t::ForLoopBlock,
    job: &mut ComponentCompilationJob,
) {
    // Allocate view for repeater
    let repeater_view_xref = job.allocate_view(Some(view.xref));

    // Get repeater view - safe access via method if we had one, but here we need to mutate
    // We should use the safe access pattern:
    // 1. Get view to modify context vars
    {
        let repeater_view = job.views.get_mut(&repeater_view_xref).unwrap();

        // Set context variable for the item - read from ctx.$implicit
        repeater_view.context_variables.insert(
            for_loop.item.name.to_string(),
            "$implicit".to_string(), // Item is accessed via ctx.$implicit
        );

        // We copy TemplateDefinitionBuilder's scheme of creating names for `$count` and `$index`
        // that are suffixed with special information, to disambiguate which level of nested loop
        // the below aliases refer to.
        let index_name = format!("ɵ$index_{}", repeater_view_xref.as_usize());
        let count_name = format!("ɵ$count_{}", repeater_view_xref.as_usize());
        let mut index_var_names = std::collections::HashSet::new();

        // Set all the context variables and aliases available in the repeater
        for variable in &for_loop.context_variables {
            if variable.value.as_ref() == "$index" {
                index_var_names.insert(variable.name.clone());
            }
            if &*variable.name == "$index" {
                // $index should be read as ctx.$index property, not ctx directly
                repeater_view.context_variables.insert(
                    "$index".to_string(),
                    "$index".to_string(), // Property name to read from context
                );
                repeater_view
                    .context_variables
                    .insert(index_name.clone(), "$index".to_string());
            } else if &*variable.name == "$count" {
                // $count should be read as ctx.$count property, not ctx directly
                repeater_view.context_variables.insert(
                    "$count".to_string(),
                    "$count".to_string(), // Property name to read from context
                );
                repeater_view
                    .context_variables
                    .insert(count_name.clone(), "$count".to_string());
            } else {
                // For other variables, we need to create an alias
                let expression =
                    get_computed_for_loop_variable_expression(variable, &index_name, &count_name);
                repeater_view.aliases.push(ir::AliasVariable::new(
                    variable.name.to_string(),
                    expression,
                ));
            }
        }
    }

    // Re-build index names for repeater op creation
    let index_name = format!("ɵ$index_{}", repeater_view_xref.as_usize());
    let mut index_var_names = std::collections::HashSet::new();
    for variable in &for_loop.context_variables {
        if &*variable.value == "$index" {
            index_var_names.insert(variable.name.clone());
        }
    }

    // Clone children before calls
    let children = for_loop.children.clone();

    // Convert track expression - use source span from track_by if available
    let track_source_span = Some(&for_loop.track_keyword_span);
    let track_expr = crate::template::pipeline::src::conversion::convert_ast(
        &for_loop.track_by.ast,
        job,
        view.xref,
        Some(&mut *view),
        track_source_span,
    );

    // Ingest children into repeater view
    ingest_children_into_view(job, repeater_view_xref, children);

    // Handle empty view if present
    let (empty_view_xref, empty_tag_name) = if let Some(empty) = &for_loop.empty {
        let empty_xref = job.allocate_view(Some(view.xref));
        let empty_children = empty.children.clone();

        // Extract tag name from single root element/template for content projection
        let empty_tag_name = {
            let mut empty_view =
                std::mem::replace(job.views.get_mut(&empty_xref).unwrap(), create_dummy_view());
            let name = ingest_control_flow_insertion_point_from_children(
                &mut empty_view,
                job,
                &empty_children,
            );
            *job.views.get_mut(&empty_xref).unwrap() = empty_view;
            name
        };

        // Ingest empty children
        ingest_children_into_view(job, empty_xref, empty_children);

        (Some(empty_xref), empty_tag_name)
    } else {
        (None, None)
    };

    // Build var names
    let var_names = ir::ops::create::RepeaterVarNames {
        dollar_index: index_var_names.into_iter().map(|s| s.to_string()).collect(),
        dollar_implicit: for_loop.item.name.to_string(),
    };

    // Validate i18n metadata
    let i18n_placeholder: Option<crate::i18n::i18n_ast::BlockPlaceholder> = match &for_loop.i18n {
        Some(I18nMeta::Node(I18nNode::BlockPlaceholder(ph))) => Some(ph.clone()),
        Some(_) => panic!("Unhandled i18n metadata type for @for: {:?}", for_loop.i18n),
        None => None,
    };

    let empty_i18n_placeholder: Option<crate::i18n::i18n_ast::BlockPlaceholder> =
        for_loop.empty.as_ref().and_then(|empty| match &empty.i18n {
            Some(I18nMeta::Node(I18nNode::BlockPlaceholder(ph))) => Some(ph.clone()),
            Some(_) => panic!("Unhandled i18n metadata type for @empty: {:?}", empty.i18n),
            None => None,
        });

    // Extract tag name from single root element/template for content projection
    let tag_name = {
        let mut repeater_view = std::mem::replace(
            job.views.get_mut(&repeater_view_xref).unwrap(),
            create_dummy_view(),
        );
        let name = ingest_control_flow_insertion_point_from_children(
            &mut repeater_view,
            job,
            &for_loop.children,
        );
        *job.views.get_mut(&repeater_view_xref).unwrap() = repeater_view;
        name
    };

    // Create RepeaterCreateOp - need to create it first to get handle
    let repeater_create_op = ir::ops::create::RepeaterCreateOp::new(
        repeater_view_xref,
        empty_view_xref,
        tag_name,
        Box::new(track_expr),
        var_names,
        empty_tag_name,
        i18n_placeholder,
        empty_i18n_placeholder,
        for_loop.block.start_source_span.clone(),
        for_loop.block.source_span.clone(),
    );

    // Get handle before boxing
    let repeater_handle = repeater_create_op.base.base.handle.clone();

    // Box and push
    view.create
        .push(Box::new(repeater_create_op) as Box<dyn ir::CreateOp + Send + Sync>);

    // Convert for loop expression - use source span from expression if available
    let expression_source_span = Some(&for_loop.block.source_span);
    let collection_expr = crate::template::pipeline::src::conversion::convert_ast(
        &for_loop.expression.ast,
        job,
        view.xref,
        Some(&mut *view),
        expression_source_span,
    );

    // Create RepeaterOp
    let repeater_op = ir::ops::update::create_repeater_op(
        repeater_view_xref,
        repeater_handle,
        collection_expr,
        for_loop.block.source_span.clone(),
    );

    view.update.push(repeater_op);
}

/// Ingest a let declaration
fn ingest_let_declaration(
    view: &mut ViewCompilationUnit,
    let_decl: t::LetDeclaration,
    job: &mut ComponentCompilationJob,
) {
    let target = job.allocate_xref_id();

    // Create DeclareLetOp
    let declare_let_op = ir::ops::create::create_declare_let_op(
        target,
        let_decl.name.clone(),
        let_decl.source_span.clone(),
    );
    view.create.push(declare_let_op);

    // Convert value expression
    let value_expr = crate::template::pipeline::src::conversion::convert_ast(
        &let_decl.value,
        job,
        view.xref,
        Some(&mut *view),
        Some(&let_decl.value_span),
    );

    // Create StoreLetOp update operation
    let store_let_op = ir::ops::update::create_store_let_op(
        target,
        let_decl.name.clone(),
        value_expr,
        let_decl.source_span.clone(),
    );
    view.update.push(store_let_op);
}

/// Check if a template is a plain ng-template (as opposed to a structural directive template).
///
/// This is checked based on the tagName. Only plain ng-templates will have tagName 'ng-template'.
///
/// Examples:
/// | Angular HTML                       | Template tagName   | isPlainTemplate |
/// | ---------------------------------- | ------------------ | --------------- |
/// | `<ng-template>`                    | 'ng-template'      | true            |
/// | `<ng-template [ngTemplateOutlet]>` | 'ng-template'      | true            |
/// | `<div *ngIf="true">`               | 'div'              | false           |
/// | `<svg><ng-template>`               | 'svg:ng-template'  | true            |
fn is_plain_template(tmpl: &t::Template) -> bool {
    if let Some(ref tag_name) = tmpl.tag_name {
        // Split namespace and check if tag name is 'ng-template'
        let is_ng_template = match crate::ml_parser::tags::split_ns_name(tag_name, false) {
            Ok((_, name)) => name == NG_TEMPLATE_TAG_NAME,
            Err(_) => tag_name.as_ref() == NG_TEMPLATE_TAG_NAME,
        };
        is_ng_template
    } else {
        // Templates without tagName (e.g., from control flow blocks) are not plain templates
        false
    }
}

/// Process all of the bindings (inputs/attributes) on an element
fn ingest_element_bindings(
    view: &mut ViewCompilationUnit,
    element_xref: ir::XrefId,
    element: &t::Element,
    job: &mut ComponentCompilationJob,
) {
    use crate::core::SecurityContext;
    use crate::i18n::i18n_ast::I18nMeta;
    use crate::schema::element_schema_registry::ElementSchemaRegistry;
    use crate::template::pipeline::ir::ops::update::{create_binding_op, BindingExpression};

    // PHASE 1: Collect security contexts using immutable borrow

    let attr_security_contexts: Vec<SecurityContext> = element
        .attributes
        .iter()
        .map(|attr| {
            job.schema_registry
                .security_context(&element.name, &attr.name, true)
        })
        .collect();

    let mut i18n_attribute_binding_names = std::collections::HashSet::new();

    // PHASE 2: Process attributes (text attributes) - now we can mutate job
    for (idx, attr) in element.attributes.iter().enumerate() {
        let security_context = attr_security_contexts[idx];

        // Convert attribute value - for now, treat as literal string
        let expression =
            BindingExpression::Expression(crate::output::output_ast::Expression::Literal(
                crate::output::output_ast::LiteralExpr {
                    value: crate::output::output_ast::LiteralValue::String(attr.value.to_string()),
                    type_: None,
                    source_span: Some(attr.source_span.clone()),
                },
            ));

        // Extract i18n message if present
        let i18n_message = match &attr.i18n {
            Some(I18nMeta::Message(msg)) => Some(msg.clone()),
            _ => None,
        };

        let binding_op = create_binding_op(
            element_xref,
            ir::BindingKind::Attribute,
            attr.name.clone(),
            expression,
            None, // unit
            vec![security_context],
            true,  // is_text_attr
            false, // is_structural_template_attribute
            None,  // template_kind
            i18n_message,
            attr.source_span.clone(),
        );

        view.update.push(binding_op);

        if attr.i18n.is_some() {
            i18n_attribute_binding_names.insert(attr.name.clone());
        }
    }

    // Process inputs (bound attributes)
    for input in &element.inputs {
        if i18n_attribute_binding_names.contains(&input.name) {
            // TODO: Log warning about i18n attribute and property binding conflict
            // For now, just continue - the binding will be created anyway
        }

        let binding_kind = match input.type_ {
            crate::expression_parser::ast::BindingType::Property => ir::BindingKind::Property,
            crate::expression_parser::ast::BindingType::Attribute => ir::BindingKind::Attribute,
            crate::expression_parser::ast::BindingType::Class => ir::BindingKind::ClassName,
            crate::expression_parser::ast::BindingType::Style => ir::BindingKind::StyleProperty,
            crate::expression_parser::ast::BindingType::Animation => ir::BindingKind::Animation,
            crate::expression_parser::ast::BindingType::TwoWay => ir::BindingKind::TwoWayProperty,
            _ => ir::BindingKind::Property, // Default fallback
        };

        // Convert input value
        let expression = convert_binding_expression(
            &input.value,
            input.i18n.as_ref(),
            job,
            view.xref,
            Some(view),
            input.value_span.as_ref(),
        );

        // Extract i18n message if present
        let i18n_message = match &input.i18n {
            Some(I18nMeta::Message(msg)) => Some(msg.clone()),
            _ => None,
        };

        let binding_op = create_binding_op(
            element_xref,
            binding_kind,
            input.name.clone(),
            expression,
            input.unit.as_ref().map(|u| u.to_string()),
            vec![input.security_context],
            false, // is_text_attr
            false, // is_structural_template_attribute
            None,  // template_kind
            i18n_message,
            input.source_span.clone(),
        );

        view.update.push(binding_op);
    }
}

/// Process all of the events (outputs) on an element in the template AST
fn ingest_element_events(
    view: &mut ViewCompilationUnit,
    element_xref: ir::XrefId,
    element_tag: &str,
    element: &t::Element,
    target_slot: crate::template::pipeline::ir::handle::SlotHandle,
    job: &mut ComponentCompilationJob,
) {
    use crate::expression_parser::ast::ParsedEventType;
    use crate::template::pipeline::ir::ops::create::create_listener_op;

    // target_slot is passed in

    // Identify outputs that are part of a two-way binding
    let mut two_way_outputs = std::collections::HashSet::new();
    for input in &element.inputs {
        if input.type_ == crate::expression_parser::ast::BindingType::TwoWay {
            two_way_outputs.insert(format!("{}Change", input.name));
        }
    }

    for output in &element.outputs {
        let handler_ops = make_listener_handler_ops(
            &output.handler,
            &output.handler_span,
            view.xref,
            Some(&mut *view),
            job,
        );

        let output_type = if output.type_ == ParsedEventType::Regular
            && two_way_outputs.contains(&*output.name)
        {
            ParsedEventType::TwoWay
        } else {
            output.type_
        };

        match output_type {
            ParsedEventType::Animation => {
                // Determine animation kind based on event name
                // Animation events ending with 'enter' are ENTER, others are LEAVE
                let animation_kind = if output.name.ends_with("enter") {
                    ir::enums::AnimationKind::Enter
                } else {
                    ir::enums::AnimationKind::Leave
                };

                // Create animation listener op
                let animation_listener_op = ir::ops::create::create_animation_listener_op(
                    job.allocate_xref_id(),
                    element_xref, // element xref
                    target_slot.clone(),
                    output.name.clone(),
                    Some(element_tag.to_string()),
                    handler_ops,
                    animation_kind,
                    output.target.as_ref().map(|t| t.to_string()), // event_target
                    false,                                         // host_listener
                    output.source_span.clone(),
                );
                view.create.push(animation_listener_op);
            }
            ParsedEventType::Regular => {
                let consumes_dollar_event = uses_dollar_event(&output.handler);
                let listener_op = create_listener_op(
                    job.allocate_xref_id(),
                    element_xref, // element xref
                    target_slot.clone(),
                    output.name.clone(),
                    Some(element_tag.to_string()),
                    handler_ops,
                    None,                                          // legacy_animation_phase
                    output.target.as_ref().map(|t| t.to_string()), // event_target
                    false,                                         // host_listener
                    output.source_span.clone(),
                    consumes_dollar_event,
                );
                view.create.push(listener_op);
            }
            ParsedEventType::TwoWay => {
                // TwoWay events need special handling: wrap the handler expression with TwoWayBindingSetExpr
                // so that transform_two_way_binding_set phase can transform it to:
                // twoWayBindingSet(target, $event) || (target = $event)
                let two_way_handler_ops = make_two_way_listener_handler_ops(
                    &output.handler,
                    &output.handler_span,
                    view.xref,
                    job,
                    Some(&mut *view),
                );
                let two_way_listener_op = ir::ops::create::create_two_way_listener_op(
                    job.allocate_xref_id(),
                    element_xref, // element xref
                    target_slot.clone(),
                    output.name.clone(),
                    Some(element_tag.to_string()),
                    two_way_handler_ops,
                    output.source_span.clone(),
                );
                view.create.push(two_way_listener_op);
            }
            ParsedEventType::LegacyAnimation => {
                // LegacyAnimation events use phase instead of target
                let consumes_dollar_event = uses_dollar_event(&output.handler);
                let listener_op = create_listener_op(
                    job.allocate_xref_id(),
                    element_xref, // element xref
                    target_slot.clone(),
                    output.name.clone(),
                    Some(element_tag.to_string()),
                    handler_ops,
                    output.target.as_ref().map(|t| t.to_string()), // legacy_animation_phase (phase is stored in target for LegacyAnimation)
                    None,  // event_target (null for LegacyAnimation)
                    false, // host_listener
                    output.source_span.clone(),
                    consumes_dollar_event,
                );
                view.create.push(listener_op);
            }
        }
    }
}

pub fn uses_dollar_event(ast: &crate::expression_parser::ast::AST) -> bool {
    use crate::expression_parser::ast::AST;
    match ast {
        AST::PropertyRead(prop) => {
            if prop.name == "$event" {
                if let AST::ImplicitReceiver(_) = *prop.receiver {
                    return true;
                }
            }
            uses_dollar_event(&prop.receiver)
        }
        AST::SafePropertyRead(read) => {
            if read.name == "$event" {
                if let AST::ImplicitReceiver(_) = *read.receiver {
                    return true;
                }
            }
            uses_dollar_event(&read.receiver)
        }
        AST::KeyedRead(keyed) => {
            uses_dollar_event(&keyed.receiver) || uses_dollar_event(&keyed.key)
        }
        AST::SafeKeyedRead(read) => {
            uses_dollar_event(&read.receiver) || uses_dollar_event(&read.key)
        }

        AST::Binary(bin) => uses_dollar_event(&bin.left) || uses_dollar_event(&bin.right),
        AST::Chain(chain) => chain.expressions.iter().any(|e| uses_dollar_event(e)),
        AST::Conditional(cond) => {
            uses_dollar_event(&cond.condition)
                || uses_dollar_event(&cond.true_exp)
                || uses_dollar_event(&cond.false_exp)
        }
        AST::Call(call) => {
            uses_dollar_event(&call.receiver) || call.args.iter().any(|a| uses_dollar_event(a))
        }
        AST::SafeCall(call) => {
            uses_dollar_event(&call.receiver) || call.args.iter().any(|a| uses_dollar_event(a))
        }
        AST::ImplicitReceiver(_)
        | AST::ThisReceiver(_)
        | AST::EmptyExpr(_)
        | AST::LiteralPrimitive(_)
        | AST::RegularExpressionLiteral(_) => false,

        AST::Interpolation(interp) => interp.expressions.iter().any(|e| uses_dollar_event(e)),
        AST::KeyedWrite(write) => {
            uses_dollar_event(&write.receiver)
                || uses_dollar_event(&write.key)
                || uses_dollar_event(&write.value)
        }
        AST::LiteralArray(arr) => arr.expressions.iter().any(|e| uses_dollar_event(e)),
        AST::LiteralMap(map) => map.values.iter().any(|e| uses_dollar_event(e)),
        AST::NonNullAssert(assert) => uses_dollar_event(&assert.expression),
        AST::BindingPipe(pipe) => {
            uses_dollar_event(&pipe.exp) || pipe.args.iter().any(|a| uses_dollar_event(a))
        }
        AST::PrefixNot(not) => uses_dollar_event(&not.expression),
        AST::PropertyWrite(write) => {
            uses_dollar_event(&write.receiver) || uses_dollar_event(&write.value)
        }
        AST::TypeofExpression(typeof_expr) => uses_dollar_event(&typeof_expr.expression),
        AST::Unary(unary) => uses_dollar_event(&unary.expr),
        AST::VoidExpression(void_expr) => uses_dollar_event(&void_expr.expression),
        AST::TemplateLiteral(tmpl) => tmpl.expressions.iter().any(|e| uses_dollar_event(e)),
        AST::TaggedTemplateLiteral(tmpl) => {
            uses_dollar_event(&tmpl.tag)
                || tmpl
                    .template
                    .expressions
                    .iter()
                    .any(|e| uses_dollar_event(e))
        }
        AST::ParenthesizedExpression(paren) => uses_dollar_event(&paren.expression),
    }
}

/// Helper function to convert event handler AST into UpdateOps
fn make_listener_handler_ops(
    handler: &crate::expression_parser::ast::AST,
    handler_span: &ParseSourceSpan,
    unit_xref: ir::XrefId,
    mut external_view: Option<&mut ViewCompilationUnit>,
    job: &mut ComponentCompilationJob,
) -> crate::template::pipeline::ir::operations::OpList<
    Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
> {
    use crate::expression_parser::ast::AST;
    use crate::output::output_ast::{Expression, ExpressionStatement, ReturnStatement, Statement};
    use crate::template::pipeline::ir::operations::OpList;
    use crate::template::pipeline::ir::ops::shared::create_statement_op;

    let mut handler_ops: OpList<
        Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
    > = OpList::new();

    // Unwrap AST - AST doesn't have ASTWithSource wrapper in Rust
    let handler_ast: &AST = handler;

    // Handle Chain expressions - split into multiple statements
    let handler_exprs: Vec<&AST> = match handler_ast {
        AST::Chain(chain) => {
            // chain.expressions is Vec<Box<AST>>, so we need to dereference
            chain.expressions.iter().map(|expr| expr.as_ref()).collect()
        }
        _ => vec![handler_ast],
    };

    if handler_exprs.is_empty() {
        panic!("Expected listener to have non-empty expression list");
    }

    // Convert expressions
    let mut expressions: Vec<Expression> = handler_exprs
        .iter()
        .map(|expr| {
            crate::template::pipeline::src::conversion::convert_ast(
                expr,
                job,
                unit_xref,
                external_view.as_deref_mut(),
                Some(handler_span),
            )
        })
        .collect();

    // The last expression is the return value
    let return_expr = expressions.pop().unwrap();

    // Add statements for intermediate expressions
    for expr in expressions {
        let expr_stmt = ExpressionStatement {
            expr: Box::new(expr),
            source_span: Some(handler_span.clone()),
        };
        let stmt = Statement::Expression(expr_stmt);
        let stmt_op = create_statement_op::<
            Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
        >(Box::new(stmt));
        handler_ops.push(Box::new(stmt_op));
    }

    // Add return statement
    let return_stmt_val = ReturnStatement {
        value: Box::new(return_expr),
        source_span: Some(handler_span.clone()),
    };
    let stmt = Statement::Return(return_stmt_val);
    let stmt_op = create_statement_op::<
        Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
    >(Box::new(stmt));
    handler_ops.push(Box::new(stmt_op));
    handler_ops
}

/// Helper function to convert event handler AST into UpdateOps for two-way bindings
/// This wraps the handler expression with TwoWayBindingSetExpr(target, $event)
/// so that transform_two_way_binding_set phase can transform it properly.
fn make_two_way_listener_handler_ops(
    handler: &crate::expression_parser::ast::AST,
    handler_span: &ParseSourceSpan,
    unit_xref: ir::XrefId,
    job: &mut ComponentCompilationJob,
    mut external_view: Option<&mut ViewCompilationUnit>,
) -> crate::template::pipeline::ir::operations::OpList<
    Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
> {
    use crate::expression_parser::ast::AST;
    use crate::output::output_ast::{
        Expression, ExpressionStatement, ReadVarExpr, ReturnStatement, Statement,
    };
    use crate::template::pipeline::ir::expression::TwoWayBindingSetExpr;
    use crate::template::pipeline::ir::operations::OpList;
    use crate::template::pipeline::ir::ops::shared::create_statement_op;

    let mut handler_ops: OpList<
        Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
    > = OpList::new();

    // Unwrap AST - AST doesn't have ASTWithSource wrapper in Rust
    let handler_ast: &AST = handler;

    // Handle Chain expressions - split into multiple statements
    let handler_exprs: Vec<&AST> = match handler_ast {
        AST::Chain(chain) => {
            // chain.expressions is Vec<Box<AST>>, so we need to dereference
            chain.expressions.iter().map(|expr| expr.as_ref()).collect()
        }
        _ => vec![handler_ast],
    };

    if handler_exprs.is_empty() {
        panic!("Expected listener to have non-empty expression list");
    }

    // Convert expressions
    let mut expressions: Vec<Expression> = handler_exprs
        .iter()
        .map(|expr| {
            crate::template::pipeline::src::conversion::convert_ast(
                expr,
                job,
                unit_xref,
                external_view.as_deref_mut(),
                Some(handler_span),
            )
        })
        .collect();

    // The last expression is the target for two-way binding
    let target_expr = expressions.pop().unwrap();

    // Create $event variable reference
    let event_var = Expression::ReadVar(ReadVarExpr {
        name: "$event".to_string(),
        type_: None,
        source_span: None,
    });

    // Wrap with TwoWayBindingSetExpr: this will be transformed by transform_two_way_binding_set phase
    // into: twoWayBindingSet(target, $event) || (target = $event)
    let two_way_set_expr = Expression::TwoWayBindingSet(TwoWayBindingSetExpr::new(
        Box::new(target_expr),
        Box::new(event_var.clone()),
    ));

    // Add statements for intermediate expressions
    for expr in expressions {
        let expr_stmt = ExpressionStatement {
            expr: Box::new(expr),
            source_span: Some(handler_span.clone()),
        };
        let stmt = Statement::Expression(expr_stmt);
        let stmt_op = create_statement_op::<
            Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
        >(Box::new(stmt));
        handler_ops.push(Box::new(stmt_op));
    }

    // Add statement for TwoWayBindingSetExpr
    let set_stmt = ExpressionStatement {
        expr: Box::new(two_way_set_expr),
        source_span: Some(handler_span.clone()),
    };
    let stmt = Statement::Expression(set_stmt);
    let stmt_op = create_statement_op::<
        Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
    >(Box::new(stmt));
    handler_ops.push(Box::new(stmt_op));

    // Add return statement with $event
    let return_stmt_val = ReturnStatement {
        value: Box::new(event_var),
        source_span: Some(handler_span.clone()),
    };
    let stmt = Statement::Return(return_stmt_val);
    let stmt_op = create_statement_op::<
        Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>,
    >(Box::new(stmt));
    handler_ops.push(Box::new(stmt_op));
    handler_ops
}

/// Process all of the local references on an element-like structure in the template AST
fn ingest_references(
    view: &mut ViewCompilationUnit,
    element_xref: ir::XrefId,
    element: &t::Element,
    job: &mut ComponentCompilationJob,
) {
    use crate::template::pipeline::ir::enums::OpKind;
    use crate::template::pipeline::ir::ops::create::{ElementStartOp, LocalRef};

    // We can access ops directly in the view
    let unit = view;

    // Find the ElementStartOp in the create list and update its local_refs
    for op in unit.create.iter_mut() {
        if op.xref() == element_xref && op.kind() == OpKind::ElementStart {
            // Safe to downcast because we've verified the kind
            unsafe {
                let op_ptr = op as *mut Box<
                    dyn crate::template::pipeline::ir::operations::CreateOp + Send + Sync,
                >;
                let element_op_ptr = op_ptr as *mut Box<ElementStartOp>;
                if !element_op_ptr.is_null() {
                    let element_op = &mut **element_op_ptr;
                    // Add references from element
                    for reference in &element.references {
                        element_op.base.base.local_refs.push(LocalRef {
                            name: reference.name.clone(),
                            target: reference.value.clone(),
                        });
                    }
                    return;
                }
            }
        }
    }

    // If we couldn't find the op, that's an error
    panic!(
        "Could not find ElementStartOp with xref {:?} to add references",
        element_xref
    );
}

/// Process all of the events (outputs) on a template in the template AST
fn ingest_template_events(
    view: &mut ViewCompilationUnit,
    template_xref: ir::XrefId,
    template_tag: Option<&str>,
    outputs: &[t::BoundEvent],
    template_kind: ir::TemplateKind,
    target_slot: crate::template::pipeline::ir::handle::SlotHandle,
    job: &mut ComponentCompilationJob,
) {
    use crate::expression_parser::ast::ParsedEventType;
    use crate::template::pipeline::ir::ops::create::create_listener_op;

    // target_slot is passed in

    for output in outputs {
        // For NgTemplate, handle all event types
        if template_kind == ir::TemplateKind::NgTemplate {
            let handler_ops = make_listener_handler_ops(
                &output.handler,
                &output.handler_span,
                view.xref,
                Some(&mut *view),
                job,
            );

            let consumes_dollar_event = uses_dollar_event(&output.handler);
            let listener_op = create_listener_op(
                job.allocate_xref_id(),
                template_xref, // template xref
                target_slot.clone(),
                output.name.clone(),
                template_tag.map(|s| s.to_string()),
                handler_ops,
                output.phase.as_ref().map(|p| p.to_string()),
                output.target.as_ref().map(|t| t.to_string()),
                false,
                output.source_span.clone(),
                consumes_dollar_event,
            );
            view.create.push(listener_op);
        }

        // For structural templates, only handle animation events
        if template_kind == ir::TemplateKind::Structural {
            if let ParsedEventType::Animation = output.type_ {
                let handler_ops = make_listener_handler_ops(
                    &output.handler,
                    &output.handler_span,
                    view.xref,
                    Some(&mut *view),
                    job,
                );

                // Determine animation kind based on event name
                let animation_kind = if output.name.ends_with("enter") {
                    ir::enums::AnimationKind::Enter
                } else {
                    ir::enums::AnimationKind::Leave
                };

                // Create animation listener op
                let animation_listener_op = ir::ops::create::create_animation_listener_op(
                    job.allocate_xref_id(),
                    template_xref, // template xref
                    target_slot.clone(),
                    output.name.clone(),
                    template_tag.map(|s| s.to_string()),
                    handler_ops,
                    animation_kind,
                    output.target.as_ref().map(|t| t.to_string()),
                    false, // host_listener
                    output.source_span.clone(),
                );
                view.create.push(animation_listener_op);
            } else if let ParsedEventType::Regular = output.type_ {
                // For regular events on structural directives, we don't create a ListenerOp
                // because the listener belongs to the element inside the views.
                // HOWEVER, the presence of the binding must be recorded in the `consts` array
                // for directive matching purposes. So we manually create an ExtractedAttributeOp.

                // We need to access SecurityContext but don't have schema registry here easily.
                // Listeners typically have SecurityContext::NONE.
                use crate::core::SecurityContext;

                let extracted_attr_op = ir::ops::create::create_extracted_attribute_op(
                    template_xref,
                    ir::BindingKind::Property, // Listeners are treated as properties in consts
                    None,
                    output.name.clone(),
                    None,
                    None,
                    None, // i18n_message
                    vec![SecurityContext::NONE],
                    Some(output.source_span.clone()),
                );
                view.create.push(extracted_attr_op);
            }
        }
    }
}

/// Process all of the local references on a template
fn ingest_references_template(
    view: &mut ViewCompilationUnit,
    template_xref: ir::XrefId,
    tmpl: &t::Template,
    job: &mut ComponentCompilationJob,
) {
    use crate::template::pipeline::ir::enums::OpKind;
    use crate::template::pipeline::ir::ops::create::{LocalRef, TemplateOp};

    let unit = view;

    // Find the TemplateOp in the create list and update its local_refs
    for op in unit.create.iter_mut() {
        if op.xref() == template_xref && op.kind() == OpKind::Template {
            // Safe to downcast because we've verified the kind
            unsafe {
                let op_ptr = op as *mut Box<
                    dyn crate::template::pipeline::ir::operations::CreateOp + Send + Sync,
                >;
                let template_op_ptr = op_ptr as *mut Box<TemplateOp>;
                if !template_op_ptr.is_null() {
                    let template_op = &mut **template_op_ptr;
                    // Add references from template
                    for reference in &tmpl.references {
                        template_op.base.base.local_refs.push(LocalRef {
                            name: reference.name.clone(),
                            target: reference.value.clone(),
                        });
                    }
                    return;
                }
            }
        }
    }

    // If we couldn't find the op, that's an error
    panic!(
        "Could not find TemplateOp with xref {:?} to add references",
        template_xref
    );
}

/// Process all of the bindings on a template in the template AST
fn ingest_template_bindings(
    view: &mut ViewCompilationUnit,
    template_xref: ir::XrefId,
    tmpl: &t::Template,
    template_kind: ir::TemplateKind,
    job: &mut ComponentCompilationJob,
) {
    use crate::i18n::i18n_ast::I18nMeta;
    use crate::template::pipeline::ir::ops::update::{create_binding_op, BindingExpression};

    // Process template inputs (bound attributes)
    for input in &tmpl.inputs {
        // For structural templates, inputs are actually bindings on the inner element.
        // They should NOT generate a BindingOp on the template (anchor).
        // However, they MUST appear in the `consts` array of the template for directive matching.
        // So we create an ExtractedAttributeOp for them.
        if template_kind == ir::TemplateKind::Structural {
            use crate::template::pipeline::ir::ops::create::create_extracted_attribute_op;

            // Map binding types to BindingKind::Property for const collection purposes.
            // Ivy consts uses AttributeMarker.Bindings (3) for all bindings.
            // In const_collection, BindingKind::Property maps to self.bindings.
            let kind = ir::BindingKind::Property;

            let extracted_op = create_extracted_attribute_op(
                template_xref,
                kind,
                None, // namespace
                input.name.clone(),
                None, // expression (value) - not needed for bindings
                None, // i18n_context
                None, // i18n_message
                vec![input.security_context],
                Some(input.source_span.clone()),
            );
            view.create.push(extracted_op);
            continue;
        }

        let binding_kind = match input.type_ {
            crate::expression_parser::ast::BindingType::Property => ir::BindingKind::Property,
            crate::expression_parser::ast::BindingType::Attribute => ir::BindingKind::Attribute,
            crate::expression_parser::ast::BindingType::Class => ir::BindingKind::ClassName,
            crate::expression_parser::ast::BindingType::Style => ir::BindingKind::StyleProperty,
            crate::expression_parser::ast::BindingType::Animation => ir::BindingKind::Animation,
            crate::expression_parser::ast::BindingType::TwoWay => ir::BindingKind::TwoWayProperty,
            _ => ir::BindingKind::Property, // Default fallback
        };

        // Convert input value
        let expression = convert_binding_expression(
            &input.value,
            input.i18n.as_ref(),
            job,
            view.xref,
            Some(view),
            input.value_span.as_ref(),
        );

        // Extract i18n message if present
        let i18n_message = match &input.i18n {
            Some(I18nMeta::Message(msg)) => Some(msg.clone()),
            _ => None,
        };

        let binding_op = create_binding_op(
            template_xref,
            binding_kind,
            input.name.clone(),
            expression,
            input.unit.as_ref().map(|u| u.to_string()),
            vec![input.security_context],
            false,               // is_text_attr
            false,               // is_structural_template_attribute
            Some(template_kind), // template_kind
            i18n_message,
            input.source_span.clone(),
        );

        view.update.push(binding_op);
    }

    // Process template attributes (e.g. structural directive attrs)
    for attr in &tmpl.template_attrs {
        match attr {
            t::TemplateAttr::Text(text_attr) => {
                // Convert attribute value
                let expression = crate::output::output_ast::Expression::Literal(
                    crate::output::output_ast::LiteralExpr {
                        value: crate::output::output_ast::LiteralValue::String(
                            text_attr.value.to_string(),
                        ),
                        type_: None,
                        source_span: Some(text_attr.source_span.clone()),
                    },
                );

                // Extract i18n message if present
                let i18n_message = match &text_attr.i18n {
                    Some(I18nMeta::Message(msg)) => Some(msg.clone()),
                    _ => None,
                };

                let binding_op = create_binding_op(
                    template_xref,
                    ir::BindingKind::Attribute,
                    text_attr.name.clone(),
                    BindingExpression::Expression(expression),
                    None,                                          // unit
                    vec![crate::core::SecurityContext::NONE],      // Default security context
                    true,                                          // is_text_attr
                    template_kind == ir::TemplateKind::Structural, // is_structural_template_attribute
                    Some(template_kind),                           // template_kind
                    i18n_message,
                    text_attr.source_span.clone(),
                );

                view.update.push(binding_op);
            }
            t::TemplateAttr::Bound(bound_attr) => {
                // Process bound attributes on templates (like ngForOf for *ngFor)
                // These are critical for structural directives to receive their input data
                let binding_kind = match bound_attr.type_ {
                    crate::expression_parser::ast::BindingType::Property => {
                        ir::BindingKind::Property
                    }
                    crate::expression_parser::ast::BindingType::Attribute => {
                        ir::BindingKind::Attribute
                    }
                    crate::expression_parser::ast::BindingType::Class => ir::BindingKind::ClassName,
                    crate::expression_parser::ast::BindingType::Style => {
                        ir::BindingKind::StyleProperty
                    }
                    crate::expression_parser::ast::BindingType::Animation => {
                        ir::BindingKind::Animation
                    }
                    crate::expression_parser::ast::BindingType::TwoWay => {
                        ir::BindingKind::TwoWayProperty
                    }
                    _ => ir::BindingKind::Property,
                };

                // Convert input value
                let expression = convert_binding_expression(
                    &bound_attr.value,
                    bound_attr.i18n.as_ref(),
                    job,
                    view.xref,
                    Some(view),
                    bound_attr.value_span.as_ref(),
                );

                // Extract i18n message if present
                let i18n_message = match &bound_attr.i18n {
                    Some(I18nMeta::Message(msg)) => Some(msg.clone()),
                    _ => None,
                };

                let binding_op = create_binding_op(
                    template_xref,
                    binding_kind,
                    bound_attr.name.clone(),
                    expression,
                    bound_attr.unit.as_ref().map(|u| u.to_string()),
                    vec![bound_attr.security_context],
                    false,               // is_text_attr
                    true,                // is_structural_template_attribute
                    Some(template_kind), // template_kind
                    i18n_message,
                    bound_attr.source_span.clone(),
                );

                view.update.push(binding_op);

                // For structural directives, bound attributes (inputs) like `ngForOf` must also
                // appear in the `consts` array as attributes for directive matching to work.
                // We emit an ExtractedAttributeOp with an empty value to ensure it's collected.
                if template_kind == ir::TemplateKind::Structural {
                    use crate::template::pipeline::ir::ops::create::create_extracted_attribute_op;
                    let extracted_op = create_extracted_attribute_op(
                        template_xref,
                        ir::BindingKind::Template,
                        None, // namespace
                        bound_attr.name.clone(),
                        Some(crate::output::output_ast::Expression::Literal(
                            crate::output::output_ast::LiteralExpr {
                                value: crate::output::output_ast::LiteralValue::String(
                                    "".to_string(),
                                ),
                                type_: None,
                                source_span: None,
                            },
                        )),
                        None, // i18n_context
                        None, // i18n_message
                        vec![crate::core::SecurityContext::NONE],
                        Some(bound_attr.source_span.clone()),
                    );
                    view.create.push(extracted_op);
                }
            }
        }
    }

    // Process regular attributes (like class="ngfor-test") on structural templates
    // These should NOT be marked as structural template attributes, so they appear
    // BEFORE the AttributeMarker::Template (4) in the consts array.
    // This matches TSC behavior at ingest.ts lines 1471-1488.
    for attr in &tmpl.attributes {
        // Convert attribute value
        let expression =
            BindingExpression::Expression(crate::output::output_ast::Expression::Literal(
                crate::output::output_ast::LiteralExpr {
                    value: crate::output::output_ast::LiteralValue::String(attr.value.to_string()),
                    type_: None,
                    source_span: Some(attr.source_span.clone()),
                },
            ));

        // Extract i18n message if present
        let i18n_message = match &attr.i18n {
            Some(I18nMeta::Message(msg)) => Some(msg.clone()),
            _ => None,
        };

        let binding_op = create_binding_op(
            template_xref,
            ir::BindingKind::Attribute,
            attr.name.clone(),
            expression,
            None,                                     // unit
            vec![crate::core::SecurityContext::NONE], // Default security context
            true,                                     // is_text_attr
            false,               // is_structural_template_attribute - FALSE for regular attrs!
            Some(template_kind), // template_kind
            i18n_message,
            attr.source_span.clone(),
        );

        view.update.push(binding_op);
    }
}

// ingest_dom_property is now in ingest_helpers.rs
// This duplicate implementation has been removed - use ingest_helpers::ingest_dom_property instead

// ingest_host_event is now in ingest_helpers.rs
// This duplicate implementation has been removed - use ingest_helpers::ingest_host_event instead
