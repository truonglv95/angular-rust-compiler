//! Ingest Module
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/ingest.ts
//! Converts R3 AST nodes into IR operations

use crate::constant_pool::ConstantPool;
use crate::expression_parser::ast::{AST as ExprAST, ParsedEvent, ParsedProperty};
use crate::i18n::i18n_ast::{I18nMeta, Node as I18nNode};
use crate::ml_parser::tags::split_ns_name;
use crate::output::output_ast::Expression;
use crate::parse_util::ParseSourceSpan;
use crate::render3::r3_ast as t;
use crate::render3::view::api::R3ComponentDeferMetadata;
// Note: DomElementSchemaRegistry is created per-call in ingest_element_bindings
use crate::template::pipeline::ir as ir;
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
) -> ComponentCompilationJob {
    let mut job = ComponentCompilationJob::new(
        component_name,
        constant_pool,
        COMPATIBILITY_MODE,
        compilation_mode,
        relative_context_file_path,
        i18n_use_external_ids,
        defer_meta,
        all_deferrable_deps_fn,
        relative_template_path,
        enable_debug_locations,
    );
    // Ingest nodes into root - we need to work around borrow checker
    // because we need both job.root and job (for allocating views)
    // This is safe because root and views are separate fields
    let root_ptr: *mut ViewCompilationUnit = &mut job.root;
    let job_ptr: *mut ComponentCompilationJob = &mut job;
    unsafe {
        let root = &mut *root_ptr;
        ingest_nodes_internal(root, template, &mut *job_ptr);
    }
    job
}

/// Internal helper to ingest nodes while working around borrow checker
fn ingest_nodes_internal(
    unit: &mut ViewCompilationUnit,
    template: Vec<t::R3Node>,
    job: &mut ComponentCompilationJob,
) {
    for node in template {
        match node {
            t::R3Node::Element(el) => ingest_element(unit, el, job),
            t::R3Node::Template(tmpl) => ingest_template(unit, tmpl, job),
            t::R3Node::Content(content) => ingest_content(unit, content, job),
            t::R3Node::Text(text) => ingest_text(unit, text, None, job),
            t::R3Node::BoundText(bound_text) => ingest_bound_text(unit, bound_text, None, job),
            t::R3Node::IfBlock(if_block) => ingest_if_block(unit, if_block, job),
            t::R3Node::SwitchBlock(switch_block) => ingest_switch_block(unit, switch_block, job),
            t::R3Node::DeferredBlock(deferred_block) => ingest_defer_block(unit, deferred_block, job),
            t::R3Node::Icu(icu) => ingest_icu(unit, icu, job),
            t::R3Node::ForLoopBlock(for_loop) => ingest_for_block(unit, for_loop, job),
            t::R3Node::LetDeclaration(let_decl) => ingest_let_declaration(unit, let_decl, job),
            t::R3Node::Component(_) => {
                // TODO: Account for selectorless nodes
            }
            _ => {
                // Unsupported node type
                // TODO: Log warning or error
            }
        }
    }
}

/// Host binding input structure
pub struct HostBindingInput {
    pub component_name: String,
    pub component_selector: String,
    pub properties: Option<Vec<ParsedProperty>>,
    pub attributes: std::collections::HashMap<String, Expression>,
    pub events: Option<Vec<ParsedEvent>>,
}

/// Process a host binding AST and convert it into a `HostBindingCompilationJob` in the intermediate representation.
pub fn ingest_host_binding(
    input: HostBindingInput,
    _binding_parser: &crate::template_parser::binding_parser::BindingParser,
    constant_pool: ConstantPool,
) -> HostBindingCompilationJob {
    let mut job = HostBindingCompilationJob::new(
        input.component_name,
        constant_pool,
        COMPATIBILITY_MODE,
        TemplateCompilationMode::DomOnly,
    );

    // Process properties
    if let Some(properties) = input.properties {
        for property in properties {
            // Determine binding kind from property
            let mut binding_kind = ir::BindingKind::Property;
            let mut property_name = property.name.clone();
            
            // Handle attr.* prefix
            if property_name.starts_with("attr.") {
                property_name = property_name[5..].to_string();
                binding_kind = ir::BindingKind::Attribute;
            }
            
            // Handle animation bindings based on property_type
            if property.property_type == crate::expression_parser::ast::ParsedPropertyType::Animation {
                binding_kind = ir::BindingKind::Animation;
            }
            // Note: LegacyAnimation would need special handling if needed
            
            // Calculate security contexts
            use crate::schema::dom_element_schema_registry::DomElementSchemaRegistry;
            use crate::schema::element_schema_registry::ElementSchemaRegistry;
            let dom_schema = DomElementSchemaRegistry::new();
            let security_contexts: Vec<_> = vec![
                dom_schema.security_context(&input.component_selector, &property_name, binding_kind == ir::BindingKind::Attribute)
            ];
            
            ingest_dom_property(&mut job, property, binding_kind, security_contexts);
        }
    }

    // Process attributes
    for (name, expr) in input.attributes {
        // Calculate security contexts for host attribute
        use crate::schema::dom_element_schema_registry::DomElementSchemaRegistry;
        use crate::schema::element_schema_registry::ElementSchemaRegistry;
        let dom_schema = DomElementSchemaRegistry::new();
        let security_contexts: Vec<_> = vec![
            dom_schema.security_context(&input.component_selector, &name, true)
        ];
        
        ingest_host_attribute(&mut job, name, expr, security_contexts);
    }

    // Process events
    if let Some(events) = input.events {
        for event in events {
            ingest_host_event(&mut job, event);
        }
    }

    job
}

/// Ingest the nodes of a template AST into the given `ViewCompilationUnit`.
fn ingest_nodes(
    unit: &mut ViewCompilationUnit,
    template: Vec<t::R3Node>,
    job: &mut ComponentCompilationJob,
) {
    // Note: job is borrowed mutably here but we also need to borrow from it
    // This is a design limitation that may need refactoring
    for node in template {
        match node {
            t::R3Node::Element(el) => ingest_element(unit, el, job),
            t::R3Node::Template(tmpl) => ingest_template(unit, tmpl, job),
            t::R3Node::Content(content) => ingest_content(unit, content, job),
            t::R3Node::Text(text) => ingest_text(unit, text, None, job),
            t::R3Node::BoundText(bound_text) => ingest_bound_text(unit, bound_text, None, job),
            t::R3Node::IfBlock(if_block) => ingest_if_block(unit, if_block, job),
            t::R3Node::SwitchBlock(switch_block) => ingest_switch_block(unit, switch_block, job),
            t::R3Node::DeferredBlock(deferred_block) => ingest_defer_block(unit, deferred_block, job),
            t::R3Node::Icu(icu) => ingest_icu(unit, icu, job),
            t::R3Node::ForLoopBlock(for_loop) => ingest_for_block(unit, for_loop, job),
            t::R3Node::LetDeclaration(let_decl) => ingest_let_declaration(unit, let_decl, job),
            t::R3Node::Component(_) => {
                // TODO: Account for selectorless nodes
            }
            _ => {
                // Unsupported node type
                // TODO: Log warning or error
            }
        }
    }
}

/// Ingest an element AST from the template into the given `ViewCompilationUnit`.
fn ingest_element(
    unit: &mut ViewCompilationUnit,
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

    let id = job.allocate_xref_id();

    let (namespace_key, element_name) = match split_ns_name(&element.name, false) {
        Ok((ns, name)) => (ns, name),
        Err(_) => (None, element.name.clone()),
    };

    let namespace = namespace_for_key(namespace_key.as_deref());
    let i18n_placeholder = match &element.i18n {
        Some(I18nMeta::Node(I18nNode::TagPlaceholder(ph))) => Some(ph.clone()),
        _ => None,
    };

    // Create element start op
    let start_op = ir::ops::create::create_element_start_op(
        element_name.clone(),
        id,
        namespace,
        i18n_placeholder.clone(),
        element.start_source_span.clone(),
        element.source_span.clone(),
    );
    unit.create.push(start_op);

    // Ingest element bindings, events, and references
    ingest_element_bindings(unit, id, &element, job);
    ingest_element_events(unit, id, &element_name, &element, job);
    ingest_references(unit, id, &element);

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
        unit.create.push(i18n_start_op);
    }

    // Ingest children - extract first to avoid borrow conflicts
    let children = element.children;
    ingest_nodes(unit, children, job);

    // Handle i18n end if needed (before element end)
    if let Some(i18n_id) = i18n_block_id {
        let i18n_end_op = ir::ops::create::create_i18n_end_op(
            i18n_id,
            element.end_source_span.clone(),
        );
        unit.create.push(i18n_end_op);
    }

    // Create element end op
    let end_op = ir::ops::create::create_element_end_op(
        id,
        element.end_source_span.clone(),
    );
    unit.create.push(end_op);
}

/// Ingest an `ng-template` node from the AST into the given `ViewCompilationUnit`.
fn ingest_template(
    unit: &mut ViewCompilationUnit,
    tmpl: t::Template,
    job: &mut ComponentCompilationJob,
) {
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

    // Extract children first to avoid borrow conflicts
    let children_to_ingest = tmpl.children.clone();
    
    let child_view_xref = job.allocate_view(Some(unit.xref));

    let (namespace_prefix, tag_name_without_namespace) = if let Some(ref tag_name) = tmpl.tag_name {
        match split_ns_name(tag_name, false) {
            Ok((ns, name)) => (ns, Some(name)),
            Err(_) => (None, Some(tag_name.clone())),
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

    // Create template op
    let template_op = ir::ops::create::create_template_op(
        child_view_xref,
        template_kind,
        tag_name_without_namespace.clone(),
        function_name_suffix.clone(),
        namespace,
        i18n_placeholder.clone(),
        tmpl.start_source_span.clone(),
        tmpl.source_span.clone(),
    );
    unit.create.push(template_op);

    // Clone needed values before borrowing job multiple times
    let template_tag_clone = tag_name_without_namespace.clone();
    let tmpl_outputs = tmpl.outputs.clone();
    let tmpl_inputs = tmpl.inputs.clone();
    let tmpl_template_attrs = tmpl.template_attrs.clone();
    
    let tmpl_attributes = tmpl.attributes.clone();
    
    // Ingest template bindings, events, and references
    // Process bindings first (borrows job)
    {
        ingest_template_bindings(unit, child_view_xref, &tmpl_inputs, &tmpl_template_attrs, &tmpl_attributes, template_kind, job);
    }
    // Process events (borrows job)
    {
        ingest_template_events(unit, child_view_xref, template_tag_clone.as_deref(), &tmpl_outputs, template_kind, job);
    }
    // Process references - doesn't need job
    ingest_references_template(unit, child_view_xref, &tmpl);

    // Ingest children into child view - use pre-extracted children
    // Extract variables before unsafe block to avoid issues
    let variables_to_add: Vec<_> = tmpl.variables.iter().map(|v| {
        let value = if v.value.is_empty() {
            "$implicit".to_string()
        } else {
            v.value.clone()
        };
        (v.name.clone(), value)
    }).collect();
    
    // Use unsafe to work around borrow checker - safe because child_view is separate entry in HashMap
    let child_view_ptr = job.views.get_mut(&child_view_xref).unwrap() as *mut ViewCompilationUnit;
    let job_ptr: *mut ComponentCompilationJob = job;
    
    unsafe {
        let child_view = &mut *child_view_ptr;
        ingest_nodes_internal(child_view, children_to_ingest, &mut *job_ptr);
    }

    // Set up context variables - do this after unsafe block to avoid borrow conflicts
    {
        let child_view = job.views.get_mut(&child_view_xref).unwrap();
        for (name, value) in variables_to_add {
            child_view.context_variables.insert(name, value);
        }
    }

    // Handle i18n for plain templates
    // If this is a plain template and there is an i18n message associated with it, insert i18n start
    // and end ops. For structural directive templates, the i18n ops will be added when ingesting the
    // element/template the directive is placed on.
    if template_kind == ir::TemplateKind::NgTemplate {
        if let Some(I18nMeta::Message(msg)) = &tmpl.i18n {
            let id = job.allocate_xref_id();
            
            // Insert i18n start op at the beginning of child view's create list (after head)
            // Use unsafe to access child_view while job is still in scope
            unsafe {
                let child_view = &mut *child_view_ptr;
                
                // Insert i18n start op at index 0 (after head)
                let i18n_start_op = ir::ops::create::create_i18n_start_op(
                    id,
                    msg.clone(),
                    None, // i18n_context
                    Some(tmpl.start_source_span.clone()),
                );
                child_view.create.insert_at(0, i18n_start_op);
                
                // Insert i18n end op at the end (before tail, which is after all ops)
                let end_span = tmpl.end_source_span.as_ref().unwrap_or(&tmpl.start_source_span).clone();
                let i18n_end_op = ir::ops::create::create_i18n_end_op(id, Some(end_span));
                child_view.create.push(i18n_end_op);
            }
        }
    }
}

/// Ingest a content (ng-content) node
fn ingest_content(
    unit: &mut ViewCompilationUnit,
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
    // Note that we process the default content before the projection in order to match the
    // insertion order at runtime.
    let has_non_empty_content = content.children.iter().any(|child| {
        match child {
            t::R3Node::Comment(_) => false,
            t::R3Node::Text(text) => !text.value.trim().is_empty(),
            _ => true,
        }
    });

    if has_non_empty_content {
        let fallback_view_xref = job.allocate_view(Some(unit.xref));
        let children = content.children.clone();
        // Get view and ingest children - use raw pointer to avoid borrow conflicts
        let fallback_view_ptr = job.views.get_mut(&fallback_view_xref).unwrap() as *mut ViewCompilationUnit;
        unsafe {
            let fallback_view_unit = &mut *fallback_view_ptr;
            ingest_nodes_internal(fallback_view_unit, children, job);
        }
        fallback_view = Some(fallback_view_xref);
    }

    let id = job.allocate_xref_id();
    let i18n_placeholder = match &content.i18n {
        Some(I18nMeta::Node(I18nNode::TagPlaceholder(ph))) => Some(ph.clone()),
        _ => None,
    };

    let op = ir::ops::create::create_projection_op(
        id,
        content.selector.clone(),
        i18n_placeholder,
        fallback_view,
        content.source_span.clone(),
    );

    // Ingest content attributes as bindings
    use crate::schema::dom_element_schema_registry::DomElementSchemaRegistry;
    use crate::schema::element_schema_registry::ElementSchemaRegistry;
    use crate::i18n::i18n_ast::{I18nMeta, Node as I18nNode};
    
    let dom_schema = DomElementSchemaRegistry::new();
    for attr in &content.attributes {
        let security_context = dom_schema.security_context(&content.selector, &attr.name, true);
        let expression = crate::output::output_ast::Expression::Literal(
            crate::output::output_ast::LiteralExpr {
                value: crate::output::output_ast::LiteralValue::String(attr.value.clone()),
                type_: None,
                source_span: Some(attr.source_span.clone()),
            }
        );
        
        // Extract i18n message if present
        let i18n_message = match &attr.i18n {
            Some(I18nMeta::Node(I18nNode::Placeholder(_ph))) => {
                // TODO: Convert placeholder to message if needed
                None
            }
            Some(I18nMeta::Message(msg)) => Some(msg.clone()),
            _ => None,
        };
        
        let binding_op = ir::ops::update::create_binding_op(
            id,
            ir::BindingKind::Attribute,
            attr.name.clone(),
            ir::ops::update::BindingExpression::Expression(expression),
            None, // unit
            vec![security_context],
            true, // is_text_attr
            false, // is_structural_template_attribute
            None, // template_kind
            i18n_message,
            attr.source_span.clone(),
        );
        unit.update.push(binding_op);
    }

    unit.create.push(op);
}

/// Ingest a text node
fn ingest_text(
    unit: &mut ViewCompilationUnit,
    text: t::Text,
    icu_placeholder: Option<String>,
    job: &mut ComponentCompilationJob,
) {
    let text_op = ir::ops::create::create_text_op(
        job.allocate_xref_id(),
        text.value.clone(),
        icu_placeholder,
        Some(text.source_span.clone()),
    );
    unit.create.push(text_op);
}

/// Ingest a bound text node
fn ingest_bound_text(
    unit: &mut ViewCompilationUnit,
    bound_text: t::BoundText,
    icu_placeholder: Option<String>,
    job: &mut ComponentCompilationJob,
) {
    // Unwrap ASTWithSource if present
    // Note: In Rust, ASTWithSource is a struct wrapper, not a variant
    // The value is already the AST, so we don't need to unwrap
    let value = &bound_text.value;
    
    // Extract Interpolation from AST
    let interpolation_ast = match value {
        ExprAST::Interpolation(interp) => interp.clone(),
        _ => panic!(
            "AssertionError: expected Interpolation for BoundText node, got {:?}",
            std::mem::discriminant(value)
        ),
    };
    
    // Validate i18n metadata - should be Container or None
    if let Some(I18nMeta::Message(_) | I18nMeta::Node(I18nNode::IcuPlaceholder(_) | I18nNode::BlockPlaceholder(_) | I18nNode::TagPlaceholder(_))) = &bound_text.i18n {
        panic!(
            "Unhandled i18n metadata type for text interpolation: {:?}",
            bound_text.i18n
        );
    }
    
    // Extract i18n placeholders from Container
    let i18n_placeholders: Vec<String> = match &bound_text.i18n {
        Some(I18nMeta::Node(I18nNode::Container(container))) => {
            container.children
                .iter()
                .filter_map(|child| {
                    if let I18nNode::Placeholder(ph) = child {
                        Some(ph.name.clone())
                    } else {
                        None
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    };
    
    // Validate placeholder count matches expression count
    if !i18n_placeholders.is_empty() && i18n_placeholders.len() != interpolation_ast.expressions.len() {
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
        String::new(), // Empty initial value for interpolated text
        icu_placeholder,
        Some(bound_text.source_span.clone()),
    );
    unit.create.push(text_op);
    
        // Convert expressions - use compatibility mode to determine base source span
        let base_source_span = if job.compatibility() == ir::CompatibilityMode::TemplateDefinitionBuilder {
            None
        } else {
            Some(&bound_text.source_span)
        };
    
    let converted_expressions: Vec<crate::output::output_ast::Expression> = interpolation_ast
        .expressions
        .iter()
        .map(|expr| {
            crate::template::pipeline::src::conversion::convert_ast(expr, job, base_source_span)
        })
        .collect();
    
    // Create Interpolation
    let interpolation = ir::ops::update::Interpolation::new(
        interpolation_ast.strings,
        converted_expressions,
        i18n_placeholders,
    );
    
    // Create InterpolateTextOp
    let interpolate_op = ir::ops::update::create_interpolate_text_op(
        text_xref,
        interpolation,
        bound_text.source_span.clone(),
    );
    unit.update.push(interpolate_op);
}

/// Ingest an if block
fn ingest_if_block(
    unit: &mut ViewCompilationUnit,
    if_block: t::IfBlock,
    job: &mut ComponentCompilationJob,
) {
    use crate::template::pipeline::ir::expression::ConditionalCaseExpr;
    
    let mut first_xref: Option<ir::XrefId> = None;
    let mut conditions: Vec<ConditionalCaseExpr> = Vec::new();

    for (i, branch) in if_block.branches.iter().enumerate() {
        // Clone children before borrowing job
        let children = branch.children.clone();
        
        let c_view_xref = job.allocate_view(Some(unit.xref));

        // Extract tag name from single root element/template for content projection
        let tag_name = ingest_control_flow_insertion_point_from_children(unit, c_view_xref, &children);

        // Extract i18n metadata
        let _i18n_placeholder: Option<crate::i18n::i18n_ast::BlockPlaceholder> = match &branch.i18n {
            Some(I18nMeta::Node(I18nNode::BlockPlaceholder(ph))) => {
                Some(ph.clone())
            }
            Some(_) => {
                panic!("Unhandled i18n metadata type for if block: {:?}", branch.i18n);
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
            let handle = op.base.base.handle; // Get handle value directly (SlotHandle is Copy)
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
            let handle = op.base.base.handle; // Get handle value directly (SlotHandle is Copy)
            (Box::new(op) as Box<dyn ir::CreateOp + Send + Sync>, handle)
        };

        unit.create.push(conditional_op);

        if first_xref.is_none() {
            first_xref = Some(c_view_xref);
        }

        // Get view and set expression alias if present
        if let Some(ref expr_alias) = branch.expression_alias {
            if let Some(c_view) = job.views.get_mut(&c_view_xref) {
                c_view.context_variables.insert(expr_alias.name.clone(), ir::variable::CTX_REF.to_string());
            }
        }

        // Convert expression if present
        let case_expr = branch.expression.as_ref().map(|expr| {
            Box::new(crate::template::pipeline::src::conversion::convert_ast(expr, job, None))
        });

        // Create ConditionalCaseExpr
        let conditional_case_expr = ConditionalCaseExpr::new(
            case_expr,
            c_view_xref,
            conditional_handle,
            branch.expression_alias.clone(),
        );
        conditions.push(conditional_case_expr);

        // Get view and ingest children - use raw pointer to avoid borrow conflicts
        let c_view_ptr = job.views.get_mut(&c_view_xref).unwrap() as *mut ViewCompilationUnit;
        unsafe {
            let c_view = &mut *c_view_ptr;
            ingest_nodes_internal(c_view, children, job);
        }
    }

    // Create ConditionalOp update operation
    let conditional_update_op = ir::ops::update::create_conditional_op(
        first_xref.expect("If block must have at least one branch"),
        None, // If blocks don't have a test expression
        conditions,
        if_block.block.source_span.clone(),
    );
    unit.update.push(conditional_update_op);
}

/// Ingest a switch block
fn ingest_switch_block(
    unit: &mut ViewCompilationUnit,
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

    for (i, case) in switch_block.cases.iter().enumerate() {
        // Clone children before borrowing job
        let children = case.children.clone();
        
        let c_view_xref = job.allocate_view(Some(unit.xref));

        // Extract tag name from single root element/template for content projection
        let tag_name = ingest_control_flow_insertion_point_from_children(unit, c_view_xref, &children);

        // Extract i18n metadata
        let _i18n_placeholder: Option<crate::i18n::i18n_ast::BlockPlaceholder> = match &case.i18n {
            Some(I18nMeta::Node(I18nNode::BlockPlaceholder(ph))) => {
                Some(ph.clone())
            }
            Some(_) => {
                panic!("Unhandled i18n metadata type for switch block: {:?}", case.i18n);
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
            let handle = op.base.base.handle;
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
            let handle = op.base.base.handle;
            (Box::new(op) as Box<dyn ir::CreateOp + Send + Sync>, handle)
        };

        unit.create.push(conditional_op);

        if first_xref.is_none() {
            first_xref = Some(c_view_xref);
        }

        // Convert expression if present
        let case_expr = case.expression.as_ref().map(|expr| {
            Box::new(crate::template::pipeline::src::conversion::convert_ast(expr, job, Some(&switch_block.block.start_source_span)))
        });

        // Create ConditionalCaseExpr
        let conditional_case_expr = ConditionalCaseExpr::new(
            case_expr,
            c_view_xref,
            conditional_handle,
            None, // Switch cases don't have expression aliases
        );
        conditions.push(conditional_case_expr);

        // Get view and ingest children - use raw pointer to avoid borrow conflicts
        let c_view_ptr = job.views.get_mut(&c_view_xref).unwrap() as *mut ViewCompilationUnit;
        unsafe {
            let c_view = &mut *c_view_ptr;
            ingest_nodes_internal(c_view, children, job);
        }
    }

    // Convert switch expression
    let switch_expr = crate::template::pipeline::src::conversion::convert_ast(
        &switch_block.expression,
        job,
        None,
    );

    // Create ConditionalOp update operation with switch expression
    let conditional_update_op = ir::ops::update::create_conditional_op(
        first_xref.expect("Switch block must have at least one case"),
        Some(switch_expr),
        conditions,
        switch_block.block.source_span.clone(),
    );
    unit.update.push(conditional_update_op);
}

/// Ingest a deferred block
fn ingest_defer_block(
    unit: &mut ViewCompilationUnit,
    deferred_block: t::DeferredBlock,
    job: &mut ComponentCompilationJob,
) {
    // Helper function to ingest defer view
    fn ingest_defer_view(
        unit: &mut ViewCompilationUnit,
        _suffix: &str,
        i18n_meta: Option<&I18nMeta>,
        children: Option<&[t::R3Node]>,
        _source_span: Option<&crate::parse_util::ParseSourceSpan>,
        job: &mut ComponentCompilationJob,
    ) -> Option<ir::XrefId> {
        if let Some(ref i18n) = i18n_meta {
            match i18n {
                I18nMeta::Node(I18nNode::BlockPlaceholder(_)) => {
                    // OK
                }
                _ => {
                    panic!("Unhandled i18n metadata type for defer block");
                }
            }
        }
        
        if let Some(children) = children {
            let view_xref = job.allocate_view(Some(unit.xref));
            let children_clone = children.to_vec();
            let view_ptr = job.views.get_mut(&view_xref).unwrap() as *mut ViewCompilationUnit;
            unsafe {
                let view = &mut *view_ptr;
                ingest_nodes_internal(view, children_clone, job);
            }
            
            // TODO: Create TemplateOp for defer view
            // const template_op = ir::ops::create::create_template_op(
            //     view_xref,
            //     ir::TemplateKind::Block,
            //     None,
            //     format!("Defer{}", suffix),
            //     ir::Namespace::HTML,
            //     i18n_placeholder,
            //     source_span.clone(),
            //     source_span.clone(),
            // );
            // unit.create.push(template_op);
            
            Some(view_xref)
        } else {
            None
        }
    }
    
    // Ingest main view
    let main_view = ingest_defer_view(
        unit,
        "",
        deferred_block.i18n.as_ref(),
        Some(&deferred_block.children),
        Some(&deferred_block.block.source_span),
        job,
    );
    
    // Ingest loading view
    let loading_view = deferred_block.loading.as_ref().and_then(|loading| {
        ingest_defer_view(
            unit,
            "Loading",
            loading.i18n.as_ref(),
            Some(&loading.children),
            Some(&loading.block.source_span),
            job,
        )
    });
    
    // Ingest placeholder view
    let placeholder_view = deferred_block.placeholder.as_ref().and_then(|placeholder| {
        ingest_defer_view(
            unit,
            "Placeholder",
            placeholder.i18n.as_ref(),
            Some(&placeholder.children),
            Some(&placeholder.block.source_span),
            job,
        )
    });
    
    // Ingest error view
    let error_view = deferred_block.error.as_ref().and_then(|error| {
        ingest_defer_view(
            unit,
            "Error",
            error.i18n.as_ref(),
            Some(&error.children),
            Some(&error.block.source_span),
            job,
        )
    });
    
    // Get own resolver function if in per-block mode
    // TODO: Check defer_meta mode and get resolver function when available
    let own_resolver_fn: Option<crate::output::output_ast::Expression> = None;
    
    // Create DeferOp - main_view is required
    let main_view_xref = main_view.expect("Defer block must have a main view");
    let defer_xref = job.allocate_xref_id();
    
    // Create DeferOp with default slot handles for now
    // TODO: Extract actual handles from TemplateOp views when available
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
    if let Some(ref placeholder) = deferred_block.placeholder {
        defer_op.placeholder_minimum_time = placeholder.minimum_time.map(|t| t as f64);
    }
    if let Some(ref loading) = deferred_block.loading {
        defer_op.loading_minimum_time = loading.minimum_time.map(|t| t as f64);
        defer_op.loading_after_time = loading.after_time.map(|t| t as f64);
    }
    
    // Calculate flags - set HasHydrateTriggers if hydrate triggers exist
    if !deferred_block.hydrate_triggers.when.is_none()
        || !deferred_block.hydrate_triggers.idle.is_none()
        || !deferred_block.hydrate_triggers.immediate.is_none()
        || !deferred_block.hydrate_triggers.timer.is_none()
        || !deferred_block.hydrate_triggers.hover.is_none()
        || !deferred_block.hydrate_triggers.interaction.is_none()
        || !deferred_block.hydrate_triggers.viewport.is_none()
        || !deferred_block.hydrate_triggers.never.is_none() {
        defer_op.flags = Some(1); // HasHydrateTriggers flag
    }
    
    unit.create.push(Box::new(defer_op));
    
    // Ingest defer triggers (on, when, etc.)
    // Use vectors to collect ops before pushing
    let mut defer_on_ops: Vec<Box<dyn ir::CreateOp + Send + Sync>> = Vec::new();
    let mut defer_when_ops: Vec<Box<dyn ir::UpdateOp + Send + Sync>> = Vec::new();
    
    // Ingest hydrate triggers first (they set up other triggers during SSR)
    ingest_defer_triggers(
        unit,
        ir::enums::DeferOpModifierKind::Hydrate,
        &deferred_block.hydrate_triggers,
        &mut defer_on_ops,
        &mut defer_when_ops,
        defer_xref,
        job,
    );
    
    // Ingest regular triggers
    ingest_defer_triggers(
        unit,
        ir::enums::DeferOpModifierKind::None,
        &deferred_block.triggers,
        &mut defer_on_ops,
        &mut defer_when_ops,
        defer_xref,
        job,
    );
    
    // Ingest prefetch triggers
    ingest_defer_triggers(
        unit,
        ir::enums::DeferOpModifierKind::Prefetch,
        &deferred_block.prefetch_triggers,
        &mut defer_on_ops,
        &mut defer_when_ops,
        defer_xref,
        job,
    );
    
    // If no concrete (non-prefetch, non-hydrate) triggers were provided, default to 'idle'
    let has_concrete_trigger = defer_on_ops.iter().any(|op| {
        // Check if op is DeferOnOp with modifier None
        // For now, just check if we have any defer ops
        op.kind() == ir::OpKind::DeferOn
    }) || !defer_when_ops.is_empty();
    
    if !has_concrete_trigger {
        let idle_op = ir::ops::create::create_defer_on_op(
            defer_xref,
            ir::ops::create::DeferTrigger::Idle,
            ir::enums::DeferOpModifierKind::None,
            deferred_block.block.source_span.clone(),
        );
        defer_on_ops.push(idle_op);
    }
    
    // Push all defer ops
    for op in defer_on_ops {
        unit.create.push(op);
    }
    for op in defer_when_ops {
        unit.update.push(op);
    }
}

/// Ingest defer triggers and create DeferOnOp/DeferWhenOp operations
fn ingest_defer_triggers(
    _unit: &mut ViewCompilationUnit,
    modifier: ir::enums::DeferOpModifierKind,
    triggers: &t::DeferredBlockTriggers,
    defer_on_ops: &mut Vec<Box<dyn ir::CreateOp + Send + Sync>>,
    defer_when_ops: &mut Vec<Box<dyn ir::UpdateOp + Send + Sync>>,
    defer_xref: ir::XrefId,
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
            IRDeferTrigger::Timer { delay: timer.delay as f64 },
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
                target_name: hover.reference.clone(),
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
                target_name: interaction.reference.clone(),
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
        // Viewport options is LiteralMap, need to convert to Expression
        // For now, set to None - can be converted later if needed
        let options_expr = None;
        
        let defer_on_op = ir::ops::create::create_defer_on_op(
            defer_xref,
            IRDeferTrigger::Viewport {
                target_name: viewport.reference.clone(),
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
        // Convert when.value AST to Expression
        // convert_ast requires (ast, job, base_source_span)
        let base_source_span = None;
        let expr = convert_ast(&when.value, job, base_source_span);
        
        // Create DeferWhenOp
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
fn ingest_icu(
    unit: &mut ViewCompilationUnit,
    icu: t::Icu,
    job: &mut ComponentCompilationJob,
) {
    // Check if this is a single i18n ICU
    if let Some(I18nMeta::Message(ref msg)) = icu.i18n {
        if is_single_i18n_icu(icu.i18n.as_ref()) {
            let xref = job.allocate_xref_id();
            
            // Extract ICU name from message
            // In TypeScript: icuFromI18nMessage(icu.i18n).name
            let icu_name = icu_from_i18n_message(msg)
                .unwrap_or_else(|| format!("ICU_{:?}", xref));
            
            // Create IcuStartOp
            let icu_start_op = ir::ops::create::create_icu_start_op(
                xref,
                msg.clone(),
                icu_name.clone(),
                icu.source_span.clone(),
            );
            unit.create.push(icu_start_op);
            
            // Ingest ICU variables and placeholders
            // Iterate over icu.vars (BoundText) - pass placeholder name
            for (placeholder, bound_text) in &icu.vars {
                ingest_bound_text(unit, bound_text.clone(), Some(placeholder.clone()), job);
            }
            
            // Iterate over icu.placeholders (IcuPlaceholder enum) - pass placeholder name
            for (placeholder, icu_ph) in &icu.placeholders {
                match icu_ph {
                    t::IcuPlaceholder::Text(text) => {
                        ingest_text(unit, text.clone(), Some(placeholder.clone()), job);
                    }
                    t::IcuPlaceholder::BoundText(bound_text) => {
                        ingest_bound_text(unit, bound_text.clone(), Some(placeholder.clone()), job);
                    }
                }
            }
            
            // Create IcuEndOp
            let icu_end_op = ir::ops::create::create_icu_end_op(xref);
            unit.create.push(icu_end_op);
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
    
    match variable.value.as_str() {
        "$index" => {
            Expression::LexicalRead(LexicalReadExpr::new(index_name.to_string()))
        }
        "$count" => {
            Expression::LexicalRead(LexicalReadExpr::new(count_name.to_string()))
        }
        "$first" => {
            // $index === 0
            Expression::BinaryOp(crate::output::output_ast::BinaryOperatorExpr {
                operator: BinaryOperator::Identical,
                lhs: Box::new(Expression::LexicalRead(LexicalReadExpr::new(index_name.to_string()))),
                rhs: Box::new(Expression::Literal(crate::output::output_ast::LiteralExpr {
                    value: crate::output::output_ast::LiteralValue::Number(0.0),
                    type_: None,
                    source_span: None,
                })),
                type_: None,
                source_span: None,
            })
        }
        "$last" => {
            // $index === $count - 1
            Expression::BinaryOp(crate::output::output_ast::BinaryOperatorExpr {
                operator: BinaryOperator::Identical,
                lhs: Box::new(Expression::LexicalRead(LexicalReadExpr::new(index_name.to_string()))),
                rhs: Box::new(Expression::BinaryOp(crate::output::output_ast::BinaryOperatorExpr {
                    operator: BinaryOperator::Minus,
                    lhs: Box::new(Expression::LexicalRead(LexicalReadExpr::new(count_name.to_string()))),
                    rhs: Box::new(Expression::Literal(crate::output::output_ast::LiteralExpr {
                        value: crate::output::output_ast::LiteralValue::Number(1.0),
                        type_: None,
                        source_span: None,
                    })),
                    type_: None,
                    source_span: None,
                })),
                type_: None,
                source_span: None,
            })
        }
        "$even" => {
            // ($index % 2) === 0
            Expression::BinaryOp(crate::output::output_ast::BinaryOperatorExpr {
                operator: BinaryOperator::Identical,
                lhs: Box::new(Expression::BinaryOp(crate::output::output_ast::BinaryOperatorExpr {
                    operator: BinaryOperator::Modulo,
                    lhs: Box::new(Expression::LexicalRead(LexicalReadExpr::new(index_name.to_string()))),
                    rhs: Box::new(Expression::Literal(crate::output::output_ast::LiteralExpr {
                        value: crate::output::output_ast::LiteralValue::Number(2.0),
                        type_: None,
                        source_span: None,
                    })),
                    type_: None,
                    source_span: None,
                })),
                rhs: Box::new(Expression::Literal(crate::output::output_ast::LiteralExpr {
                    value: crate::output::output_ast::LiteralValue::Number(0.0),
                    type_: None,
                    source_span: None,
                })),
                type_: None,
                source_span: None,
            })
        }
        "$odd" => {
            // ($index % 2) !== 0
            Expression::BinaryOp(crate::output::output_ast::BinaryOperatorExpr {
                operator: BinaryOperator::NotIdentical,
                lhs: Box::new(Expression::BinaryOp(crate::output::output_ast::BinaryOperatorExpr {
                    operator: BinaryOperator::Modulo,
                    lhs: Box::new(Expression::LexicalRead(LexicalReadExpr::new(index_name.to_string()))),
                    rhs: Box::new(Expression::Literal(crate::output::output_ast::LiteralExpr {
                        value: crate::output::output_ast::LiteralValue::Number(2.0),
                        type_: None,
                        source_span: None,
                    })),
                    type_: None,
                    source_span: None,
                })),
                rhs: Box::new(Expression::Literal(crate::output::output_ast::LiteralExpr {
                    value: crate::output::output_ast::LiteralValue::Number(0.0),
                    type_: None,
                    source_span: None,
                })),
                type_: None,
                source_span: None,
            })
        }
        _ => {
            panic!("AssertionError: unknown @for loop variable {}", variable.value);
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
    unit: &mut ViewCompilationUnit,
    xref: ir::XrefId,
    children: &[t::R3Node],
) -> Option<String> {
    use crate::schema::dom_element_schema_registry::DomElementSchemaRegistry;
    use crate::schema::element_schema_registry::ElementSchemaRegistry;
    use crate::i18n::i18n_ast::{I18nMeta, Node as I18nNode};
    
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
                        let security_context = dom_schema.security_context(&el.name, &attr.name, true);
                        let expression = crate::output::output_ast::Expression::Literal(
                            crate::output::output_ast::LiteralExpr {
                                value: crate::output::output_ast::LiteralValue::String(attr.value.clone()),
                                type_: None,
                                source_span: Some(attr.source_span.clone()),
                            }
                        );
                        
                        // Extract i18n message if present
                        let i18n_message = match &attr.i18n {
                            Some(I18nMeta::Node(I18nNode::Placeholder(_ph))) => {
                                // Convert to Message if needed - for now use placeholder name
                                None // TODO: Convert properly
                            }
                            _ => None,
                        };
                        
                        let binding_op = ir::ops::update::create_binding_op(
                            xref,
                            ir::BindingKind::Attribute,
                            attr.name.clone(),
                            ir::ops::update::BindingExpression::Expression(expression),
                            None,
                            vec![security_context],
                            true, // is_text_attr
                            false, // is_structural_template_attribute
                            None, // template_kind
                            i18n_message,
                            attr.source_span.clone(),
                        );
                        unit.update.push(binding_op);
                    }
                }
                
                // Also collect the inputs since they participate in content projection as well.
                // Note that TDB used to collect the outputs as well, but it wasn't passing them into
                // the template instruction. Here we just don't collect them.
                for input in &el.inputs {
                    if input.type_ != crate::expression_parser::ast::BindingType::LegacyAnimation
                        && input.type_ != crate::expression_parser::ast::BindingType::Animation
                        && input.type_ != crate::expression_parser::ast::BindingType::Attribute
                    {
                        let security_context = dom_schema.security_context(&NG_TEMPLATE_TAG_NAME, &input.name, true);
                        let extracted_attr_op = ir::ops::create::create_extracted_attribute_op(
                            xref,
                            ir::BindingKind::Property,
                            None, // name
                            input.name.clone(),
                            None, // value
                            None, // i18n_message
                            None, // source_span
                            vec![security_context],
                        );
                        unit.create.push(extracted_attr_op);
                    }
                }
                
                let tag_name = &el.name;
                
                // Don't pass along `ng-template` tag name since it enables directive matching.
                if tag_name == NG_TEMPLATE_TAG_NAME {
                    return None;
                }
                return Some(tag_name.clone());
            }
            t::R3Node::Template(tmpl) => {
                // Similar logic for template nodes
                // Collect attributes
                for attr in &tmpl.attributes {
                    if !attr.name.starts_with(ANIMATE_PREFIX) {
                        let security_context = dom_schema.security_context(&NG_TEMPLATE_TAG_NAME, &attr.name, true);
                        let expression = crate::output::output_ast::Expression::Literal(
                            crate::output::output_ast::LiteralExpr {
                                value: crate::output::output_ast::LiteralValue::String(attr.value.clone()),
                                type_: None,
                                source_span: Some(attr.source_span.clone()),
                            }
                        );
                        
                        let binding_op = ir::ops::update::create_binding_op(
                            xref,
                            ir::BindingKind::Attribute,
                            attr.name.clone(),
                            ir::ops::update::BindingExpression::Expression(expression),
                            None,
                            vec![security_context],
                            true,
                            false,
                            None,
                            None,
                            attr.source_span.clone(),
                        );
                        unit.update.push(binding_op);
                    }
                }
                
                // Collect inputs
                for input in &tmpl.inputs {
                    if input.type_ != crate::expression_parser::ast::BindingType::LegacyAnimation
                        && input.type_ != crate::expression_parser::ast::BindingType::Animation
                        && input.type_ != crate::expression_parser::ast::BindingType::Attribute
                    {
                        let security_context = dom_schema.security_context(&NG_TEMPLATE_TAG_NAME, &input.name, true);
                        let extracted_attr_op = ir::ops::create::create_extracted_attribute_op(
                            xref,
                            ir::BindingKind::Property,
                            None,
                            input.name.clone(),
                            None,
                            None,
                            None,
                            vec![security_context],
                        );
                        unit.create.push(extracted_attr_op);
                    }
                }
                
                if let Some(tag_name) = &tmpl.tag_name {
                    if tag_name == NG_TEMPLATE_TAG_NAME {
                        return None;
                    }
                    return Some(tag_name.clone());
                }
            }
            _ => {}
        }
    }
    
    None
}

/// Ingest a for loop block
fn ingest_for_block(
    unit: &mut ViewCompilationUnit,
    for_loop: t::ForLoopBlock,
    job: &mut ComponentCompilationJob,
) {
    // Allocate view for repeater
    let repeater_view_xref = job.allocate_view(Some(unit.xref));
    
    // Get repeater view
    let repeater_view = job.views.get_mut(&repeater_view_xref).unwrap();
    
    // Set context variable for the item - read from ctx.$implicit
    repeater_view.context_variables.insert(
        for_loop.item.name.clone(),
        "$implicit".to_string(),  // Item is accessed via ctx.$implicit
    );
    
    // We copy TemplateDefinitionBuilder's scheme of creating names for `$count` and `$index`
    // that are suffixed with special information, to disambiguate which level of nested loop
    // the below aliases refer to.
    let index_name = format!("ɵ$index_{}", repeater_view_xref.as_usize());
    let count_name = format!("ɵ$count_{}", repeater_view_xref.as_usize());
    let mut index_var_names = std::collections::HashSet::new();
    
    // Set all the context variables and aliases available in the repeater
    for variable in &for_loop.context_variables {
        if variable.value == "$index" {
            index_var_names.insert(variable.name.clone());
        }
        if variable.name == "$index" {
            // $index should be read as ctx.$index property, not ctx directly
            repeater_view.context_variables.insert(
                "$index".to_string(),
                "$index".to_string(),  // Property name to read from context
            );
            repeater_view.context_variables.insert(
                index_name.clone(),
                "$index".to_string(),
            );
        } else if variable.name == "$count" {
            // $count should be read as ctx.$count property, not ctx directly
            repeater_view.context_variables.insert(
                "$count".to_string(),
                "$count".to_string(),  // Property name to read from context
            );
            repeater_view.context_variables.insert(
                count_name.clone(),
                "$count".to_string(),
            );
        } else {
            // For other variables, we need to create an alias
            let expression = get_computed_for_loop_variable_expression(variable, &index_name, &count_name);
            repeater_view.aliases.push(ir::AliasVariable::new(
                variable.name.clone(),
                expression,
            ));
        }
    }
    
    // Clone children before borrowing job
    let children = for_loop.children.clone();
    
    // Convert track expression - use source span from track_by if available
    let track_source_span = Some(&for_loop.track_keyword_span);
    let track_expr = crate::template::pipeline::src::conversion::convert_ast(
        &for_loop.track_by.ast,
        job,
        track_source_span,
    );
    
    // Ingest children into repeater view - use raw pointer to avoid borrow conflicts
    let repeater_view_ptr = job.views.get_mut(&repeater_view_xref).unwrap() as *mut ViewCompilationUnit;
    unsafe {
        let repeater_view_mut = &mut *repeater_view_ptr;
        ingest_nodes_internal(repeater_view_mut, children, job);
    }
    
    // Handle empty view if present
    let (empty_view_xref, empty_tag_name) = if let Some(empty) = &for_loop.empty {
        let empty_xref = job.allocate_view(Some(unit.xref));
        let empty_children = empty.children.clone();
        
        // Extract tag name from single root element/template for content projection
        let empty_tag_name = ingest_control_flow_insertion_point_from_children(unit, empty_xref, &empty_children);
        
        // Ingest empty children - use raw pointer to avoid borrow conflicts
        let empty_view_ptr = job.views.get_mut(&empty_xref).unwrap() as *mut ViewCompilationUnit;
        unsafe {
            let empty_view_mut = &mut *empty_view_ptr;
            ingest_nodes_internal(empty_view_mut, empty_children, job);
        }
        
        (Some(empty_xref), empty_tag_name)
    } else {
        (None, None)
    };
    
    // Build var names
    let var_names = ir::ops::create::RepeaterVarNames {
        dollar_index: index_var_names.into_iter().collect(),
        dollar_implicit: for_loop.item.name.clone(),
    };
    
    // Validate i18n metadata
    let i18n_placeholder: Option<crate::i18n::i18n_ast::BlockPlaceholder> = match &for_loop.i18n {
        Some(I18nMeta::Node(I18nNode::BlockPlaceholder(ph))) => Some(ph.clone()),
        Some(_) => panic!("Unhandled i18n metadata type for @for: {:?}", for_loop.i18n),
        None => None,
    };
    
    let empty_i18n_placeholder: Option<crate::i18n::i18n_ast::BlockPlaceholder> = 
        for_loop.empty.as_ref().and_then(|empty| {
            match &empty.i18n {
                Some(I18nMeta::Node(I18nNode::BlockPlaceholder(ph))) => Some(ph.clone()),
                Some(_) => panic!("Unhandled i18n metadata type for @empty: {:?}", empty.i18n),
                None => None,
            }
        });
    
    // Extract tag name from single root element/template for content projection
    let tag_name = ingest_control_flow_insertion_point_from_children(unit, repeater_view_xref, &for_loop.children);
    
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
    let repeater_handle = repeater_create_op.base.base.handle;
    
    // Box and push
    unit.create.push(Box::new(repeater_create_op) as Box<dyn ir::CreateOp + Send + Sync>);
    
    // Convert for loop expression - use source span from expression if available
    let expression_source_span = Some(&for_loop.block.source_span);
    let collection_expr = crate::template::pipeline::src::conversion::convert_ast(
        &for_loop.expression.ast,
        job,
        expression_source_span,
    );
    
    // Create RepeaterOp
    let repeater_op = ir::ops::update::create_repeater_op(
        repeater_view_xref,
        repeater_handle,
        collection_expr,
        for_loop.block.source_span.clone(),
    );
    
    unit.update.push(repeater_op);
}

/// Ingest a let declaration
fn ingest_let_declaration(
    unit: &mut ViewCompilationUnit,
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
    unit.create.push(declare_let_op);
    
    // Convert value expression
    let value_expr = crate::template::pipeline::src::conversion::convert_ast(
        &let_decl.value,
        job,
        Some(&let_decl.value_span),
    );
    
    // Create StoreLetOp update operation
    let store_let_op = ir::ops::update::create_store_let_op(
        target,
        let_decl.name.clone(),
        value_expr,
        let_decl.source_span.clone(),
    );
    unit.update.push(store_let_op);
}

/// Check if a template is a plain template (not a structural directive)
fn is_plain_template(tmpl: &t::Template) -> bool {
    // A plain template has no structural directive attributes
    tmpl.template_attrs.is_empty()
        && tmpl.inputs.is_empty()
        && tmpl.outputs.is_empty()
}

/// Process all of the bindings on an element in the template AST and convert them to their IR representation.
fn ingest_element_bindings(
    unit: &mut ViewCompilationUnit,
    element_xref: ir::XrefId,
    element: &t::Element,
    job: &mut ComponentCompilationJob,
) {
    use crate::schema::dom_element_schema_registry::DomElementSchemaRegistry;
    use crate::schema::element_schema_registry::ElementSchemaRegistry;
    use crate::template::pipeline::ir::ops::update::{BindingExpression, create_binding_op};
    use crate::i18n::i18n_ast::I18nMeta;
    
    // Create schema registry instance
    let schema = DomElementSchemaRegistry::new();
    
    let mut i18n_attribute_binding_names = std::collections::HashSet::new();
    
    // Process attributes (text attributes) - currently only text attributes are supported
    for attr in &element.attributes {
        let security_context = schema.security_context(&element.name, &attr.name, true);
        
        // Convert attribute value - for now, treat as literal string
        // TODO: Support interpolation in attributes properly
        let expression = BindingExpression::Expression(
            crate::output::output_ast::Expression::Literal(
                crate::output::output_ast::LiteralExpr {
                    value: crate::output::output_ast::LiteralValue::String(attr.value.clone()),
                    type_: None,
                    source_span: Some(attr.source_span.clone()),
                }
            )
        );
        
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
        
        unit.update.push(binding_op);
        
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
        let expression = crate::template::pipeline::src::conversion::convert_ast(
            &input.value,
            job,
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
            BindingExpression::Expression(expression),
            input.unit.clone(),
            vec![input.security_context],
            false, // is_text_attr
            false, // is_structural_template_attribute
            None,  // template_kind
            i18n_message,
            input.source_span.clone(),
        );
        
        unit.update.push(binding_op);
    }
}

/// Process all of the events (outputs) on an element in the template AST
fn ingest_element_events(
    unit: &mut ViewCompilationUnit,
    element_xref: ir::XrefId,
    element_tag: &str,
    element: &t::Element,
    job: &mut ComponentCompilationJob,
) {
    use crate::template::pipeline::ir::ops::create::create_listener_op;
    use crate::template::pipeline::ir::handle::SlotHandle;
    use crate::expression_parser::ast::ParsedEventType;
    
    // Get handle from the last op we created (ElementStartOp)
    // We need to extract it - for now use a default slot handle
    // TODO: Extract actual handle from ElementStartOp
    let target_slot = SlotHandle::with_slot(0);
    
    for output in &element.outputs {
        let handler_ops = make_listener_handler_ops(unit, &output.handler, &output.handler_span, job);
        
        match output.type_ {
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
                    element_xref,
                    target_slot.clone(),
                    output.name.clone(),
                    Some(element_tag.to_string()),
                    handler_ops,
                    animation_kind,
                    output.target.clone(), // event_target
                    false, // host_listener
                    output.source_span.clone(),
                );
                unit.create.push(animation_listener_op);
            }
            ParsedEventType::Regular => {
                let listener_op = create_listener_op(
                    element_xref,
                    target_slot.clone(),
                    output.name.clone(),
                    Some(element_tag.to_string()),
                    handler_ops,
                    None, // legacy_animation_phase
                    output.target.clone(), // event_target
                    false, // host_listener
                    output.source_span.clone(),
                );
                unit.create.push(listener_op);
            }
            ParsedEventType::TwoWay => {
                // TwoWay events use create_two_way_listener_op
                let two_way_handler_ops = make_listener_handler_ops(unit, &output.handler, &output.handler_span, job);
                let two_way_listener_op = ir::ops::create::create_two_way_listener_op(
                    element_xref,
                    target_slot.clone(),
                    output.name.clone(),
                    Some(element_tag.to_string()),
                    two_way_handler_ops,
                    output.source_span.clone(),
                );
                unit.create.push(two_way_listener_op);
            }
            ParsedEventType::LegacyAnimation => {
                // LegacyAnimation events use phase instead of target
                let listener_op = create_listener_op(
                    element_xref,
                    target_slot.clone(),
                    output.name.clone(),
                    Some(element_tag.to_string()),
                    handler_ops,
                    output.target.clone(), // legacy_animation_phase (phase is stored in target for LegacyAnimation)
                    None, // event_target (null for LegacyAnimation)
                    false, // host_listener
                    output.source_span.clone(),
                );
                unit.create.push(listener_op);
            }
        }
    }
}

/// Helper function to convert event handler AST into UpdateOps
fn make_listener_handler_ops(
    _unit: &mut ViewCompilationUnit,
    handler: &crate::expression_parser::ast::AST,
    handler_span: &ParseSourceSpan,
    job: &mut ComponentCompilationJob,
) -> crate::template::pipeline::ir::operations::OpList<Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>> {
    use crate::template::pipeline::ir::operations::OpList;
    use crate::template::pipeline::ir::ops::shared::create_statement_op;
    use crate::output::output_ast::{Expression, ExpressionStatement, ReturnStatement, Statement};
    use crate::expression_parser::ast::AST;
    
    let mut handler_ops: OpList<Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>> = OpList::new();
    
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
            crate::template::pipeline::src::conversion::convert_ast(expr, job, Some(handler_span))
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
        let stmt_op = create_statement_op::<Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>>(Box::new(stmt));
        handler_ops.push(Box::new(stmt_op));
    }
    
    // Add return statement
    let return_stmt_val = ReturnStatement {
        value: Box::new(return_expr),
        source_span: Some(handler_span.clone()),
    };
    let stmt = Statement::Return(return_stmt_val);
    let stmt_op = create_statement_op::<Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>>(Box::new(stmt));
    handler_ops.push(Box::new(stmt_op));
    
    handler_ops
}

/// Helper function to convert event handler AST into UpdateOps for host bindings
fn make_host_listener_handler_ops(
    handler: &crate::expression_parser::ast::AST,
    handler_span: &ParseSourceSpan,
    job: &mut HostBindingCompilationJob,
) -> crate::template::pipeline::ir::operations::OpList<Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>> {
    use crate::template::pipeline::ir::operations::OpList;
    use crate::template::pipeline::ir::ops::shared::create_statement_op;
    use crate::output::output_ast::{Expression, ExpressionStatement, ReturnStatement, Statement};
    use crate::expression_parser::ast::AST;
    
    let mut handler_ops: OpList<Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>> = OpList::new();
    
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
            crate::template::pipeline::src::conversion::convert_ast(expr, job, Some(handler_span))
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
        let stmt_op = create_statement_op::<Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>>(Box::new(stmt));
        handler_ops.push(Box::new(stmt_op));
    }
    
    // Add return statement
    let return_stmt_val = ReturnStatement {
        value: Box::new(return_expr),
        source_span: Some(handler_span.clone()),
    };
    let stmt = Statement::Return(return_stmt_val);
    let stmt_op = create_statement_op::<Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>>(Box::new(stmt));
    handler_ops.push(Box::new(stmt_op));
    
    handler_ops
}

/// Process all of the local references on an element-like structure in the template AST
fn ingest_references(
    unit: &mut ViewCompilationUnit,
    element_xref: ir::XrefId,
    element: &t::Element,
) {
    use crate::template::pipeline::ir::ops::create::{ElementStartOp, LocalRef};
    use crate::template::pipeline::ir::enums::OpKind;
    
    // Find the ElementStartOp in the create list and update its local_refs
    for op in unit.create.iter_mut() {
        if op.xref() == element_xref && op.kind() == OpKind::ElementStart {
            // Safe to downcast because we've verified the kind
            unsafe {
                let op_ptr = op as *mut Box<dyn crate::template::pipeline::ir::operations::CreateOp + Send + Sync>;
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
    panic!("Could not find ElementStartOp with xref {:?} to add references", element_xref);
}

/// Process all of the events (outputs) on a template in the template AST
fn ingest_template_events(
    unit: &mut ViewCompilationUnit,
    template_xref: ir::XrefId,
    template_tag: Option<&str>,
    outputs: &[t::BoundEvent],
    template_kind: ir::TemplateKind,
    job: &mut ComponentCompilationJob,
) {
    use crate::template::pipeline::ir::ops::create::create_listener_op;
    use crate::template::pipeline::ir::handle::SlotHandle;
    use crate::expression_parser::ast::ParsedEventType;
    
    // Get handle - TODO: Extract actual handle from TemplateOp
    let target_slot = SlotHandle::with_slot(0);
    
    for output in outputs {
        // For NgTemplate, handle all event types
        if template_kind == ir::TemplateKind::NgTemplate {
            let handler_ops = make_listener_handler_ops(unit, &output.handler, &output.handler_span, job);
            
            let listener_op = create_listener_op(
                template_xref,
                target_slot.clone(),
                output.name.clone(),
                template_tag.map(|s| s.to_string()),
                handler_ops,
                output.phase.clone(),
                output.target.clone(),
                false,
                output.source_span.clone(),
            );
            unit.create.push(listener_op);
        }
        
        // For structural templates, only handle animation events
        if template_kind == ir::TemplateKind::Structural {
            if let ParsedEventType::Animation = output.type_ {
                let handler_ops = make_listener_handler_ops(unit, &output.handler, &output.handler_span, job);
                
                // Determine animation kind based on event name
                let animation_kind = if output.name.ends_with("enter") {
                    ir::enums::AnimationKind::Enter
                } else {
                    ir::enums::AnimationKind::Leave
                };
                
                // Create animation listener op
                let animation_listener_op = ir::ops::create::create_animation_listener_op(
                    template_xref,
                    target_slot.clone(),
                    output.name.clone(),
                    template_tag.map(|s| s.to_string()),
                    handler_ops,
                    animation_kind,
                    output.target.clone(),
                    false, // host_listener
                    output.source_span.clone(),
                );
                unit.create.push(animation_listener_op);
            }
        }
    }
}

/// Process all of the local references on a template
fn ingest_references_template(
    unit: &mut ViewCompilationUnit,
    template_xref: ir::XrefId,
    tmpl: &t::Template,
) {
    use crate::template::pipeline::ir::ops::create::{TemplateOp, LocalRef};
    use crate::template::pipeline::ir::enums::OpKind;
    
    // Find the TemplateOp in the create list and update its local_refs
    for op in unit.create.iter_mut() {
        if op.xref() == template_xref && op.kind() == OpKind::Template {
            // Safe to downcast because we've verified the kind
            unsafe {
                let op_ptr = op as *mut Box<dyn crate::template::pipeline::ir::operations::CreateOp + Send + Sync>;
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
    panic!("Could not find TemplateOp with xref {:?} to add references", template_xref);
}


/// Process all of the bindings on a template in the template AST
fn ingest_template_bindings(
    unit: &mut ViewCompilationUnit,
    template_xref: ir::XrefId,
    inputs: &[t::BoundAttribute],
    template_attrs: &[t::TemplateAttr],
    attributes: &[t::TextAttribute], // Regular attributes like class
    template_kind: ir::TemplateKind,
    job: &mut ComponentCompilationJob,
) {
    use crate::template::pipeline::ir::ops::update::{BindingExpression, create_binding_op};
    use crate::i18n::i18n_ast::I18nMeta;
    
    // Process template inputs (bound attributes)
    for input in inputs {
        let binding_kind = match input.type_ {
            crate::expression_parser::ast::BindingType::Property => ir::BindingKind::Property,
            crate::expression_parser::ast::BindingType::Attribute => ir::BindingKind::Attribute,
            crate::expression_parser::ast::BindingType::Class => ir::BindingKind::ClassName,
            crate::expression_parser::ast::BindingType::Style => ir::BindingKind::StyleProperty,
            crate::expression_parser::ast::BindingType::Animation => ir::BindingKind::Animation,
            crate::expression_parser::ast::BindingType::TwoWay => ir::BindingKind::TwoWayProperty,
            _ => ir::BindingKind::Property,
        };
        
        // Convert input value
        let expression = crate::template::pipeline::src::conversion::convert_ast(
            &input.value,
            job,
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
            BindingExpression::Expression(expression),
            input.unit.clone(),
            vec![input.security_context],
            false, // is_text_attr
            template_kind == ir::TemplateKind::Structural, // is_structural_template_attribute
            Some(template_kind), // template_kind
            i18n_message,
            input.source_span.clone(),
        );
        
        unit.update.push(binding_op);
    }
    
    // Process template attributes (text attributes on templates - directive-specific like ngFor)
    // TemplateAttr is an enum with Bound and Text variants
    for attr in template_attrs {
        match attr {
            t::TemplateAttr::Text(text_attr) => {
                // Convert attribute value
                let expression = BindingExpression::Expression(
                    crate::output::output_ast::Expression::Literal(
                        crate::output::output_ast::LiteralExpr {
                            value: crate::output::output_ast::LiteralValue::String(text_attr.value.clone()),
                            type_: None,
                            source_span: Some(text_attr.source_span.clone()),
                        }
                    )
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
                    expression,
                    None, // unit
                    vec![crate::core::SecurityContext::NONE], // Default security context for template attrs
                    true,  // is_text_attr
                    template_kind == ir::TemplateKind::Structural, // is_structural_template_attribute
                    Some(template_kind), // template_kind
                    i18n_message,
                    text_attr.source_span.clone(),
                );
                
                unit.update.push(binding_op);
            }
            t::TemplateAttr::Bound(bound_attr) => {
                // Process bound attributes on templates (like ngForOf for *ngFor)
                // These are critical for structural directives to receive their input data
                let binding_kind = match bound_attr.type_ {
                    crate::expression_parser::ast::BindingType::Property => ir::BindingKind::Property,
                    crate::expression_parser::ast::BindingType::Attribute => ir::BindingKind::Attribute,
                    crate::expression_parser::ast::BindingType::Class => ir::BindingKind::ClassName,
                    crate::expression_parser::ast::BindingType::Style => ir::BindingKind::StyleProperty,
                    crate::expression_parser::ast::BindingType::Animation => ir::BindingKind::Animation,
                    crate::expression_parser::ast::BindingType::TwoWay => ir::BindingKind::TwoWayProperty,
                    _ => ir::BindingKind::Property,
                };
                
                // Convert input value
                let expression = crate::template::pipeline::src::conversion::convert_ast(
                    &bound_attr.value,
                    job,
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
                    BindingExpression::Expression(expression),
                    bound_attr.unit.clone(),
                    vec![bound_attr.security_context],
                    false, // is_text_attr
                    template_kind == ir::TemplateKind::Structural, // is_structural_template_attribute
                    Some(template_kind), // template_kind
                    i18n_message,
                    bound_attr.source_span.clone(),
                );
                
                unit.update.push(binding_op);

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
                                value: crate::output::output_ast::LiteralValue::String("".to_string()),
                                type_: None,
                                source_span: None,
                            }
                        )),
                        None, // i18n_context
                        None, // i18n_message
                        vec![crate::core::SecurityContext::NONE],
                    );
                    unit.create.push(extracted_op);
                }
            }
        }
    }
    
    // Process regular attributes (like class="ngfor-test") on structural templates
    // These should NOT be marked as structural template attributes, so they appear
    // BEFORE the AttributeMarker::Template (4) in the consts array.
    // This matches TSC behavior at ingest.ts lines 1471-1488.
    for attr in attributes {
        // Convert attribute value
        let expression = BindingExpression::Expression(
            crate::output::output_ast::Expression::Literal(
                crate::output::output_ast::LiteralExpr {
                    value: crate::output::output_ast::LiteralValue::String(attr.value.clone()),
                    type_: None,
                    source_span: Some(attr.source_span.clone()),
                }
            )
        );
        
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
            None, // unit
            vec![crate::core::SecurityContext::NONE], // Default security context
            true,  // is_text_attr
            false, // is_structural_template_attribute - FALSE for regular attrs!
            Some(template_kind), // template_kind
            i18n_message,
            attr.source_span.clone(),
        );
        
        unit.update.push(binding_op);
    }
}

/// Ingest a DOM property binding for host bindings
fn ingest_dom_property(
    job: &mut HostBindingCompilationJob,
    property: ParsedProperty,
    binding_kind: ir::BindingKind,
    security_contexts: Vec<crate::core::SecurityContext>,
) {
    use crate::expression_parser::ast::AST as ExprAST;
    use crate::template::pipeline::src::conversion::convert_ast;
    use crate::template::pipeline::ir::ops::update::{BindingExpression, create_binding_op};
    
    // Convert expression - handle interpolation if present
    let expression = match property.expression.as_ref() {
        ExprAST::Interpolation(interp) => {
            // Convert interpolation to IR Interpolation
            let exprs: Vec<Expression> = interp.expressions.iter().map(|expr| {
                convert_ast(expr, job, Some(&property.source_span))
            }).collect();
            
            // Create Interpolation expression
            use crate::template::pipeline::ir::ops::update::Interpolation;
            BindingExpression::Interpolation(Interpolation {
                strings: interp.strings.clone(),
                expressions: exprs,
                i18n_placeholders: vec![], // Host bindings don't have i18n placeholders
            })
        }
        ast => {
            // Convert regular AST to Expression
            BindingExpression::Expression(convert_ast(ast, job, Some(&property.source_span)))
        }
    };
    
    // Create binding op - push to update list
    let binding_op = create_binding_op(
        job.root.xref,
        binding_kind,
        property.name,
        expression,
        None, // unit - ParsedProperty doesn't have unit field
        security_contexts,
        false, // is_text_attr
        false, // is_structural_template_attribute
        None,  // template_kind
        None,  // i18n_message - host bindings don't handle i18n
        property.source_span,
    );
    
    job.root.update.push(binding_op);
}

/// Ingest a host attribute binding
fn ingest_host_attribute(
    job: &mut HostBindingCompilationJob,
    name: String,
    value: Expression,
    security_contexts: Vec<crate::core::SecurityContext>,
) {
    use crate::template::pipeline::ir::ops::update::{BindingExpression, create_binding_op};
    
    // Host attributes should always be extracted to const hostAttrs
    // Create binding op with is_text_attr = true
    use crate::output::output_ast::ExpressionTrait;
    use crate::parse_util::{ParseLocation, ParseSourceFile};
    let source_span = value.source_span()
        .cloned()
        .unwrap_or_else(|| {
            let file = ParseSourceFile::new("".to_string(), "".to_string());
            let loc = ParseLocation::new(file, 0, 0, 0);
            ParseSourceSpan::new(loc.clone(), loc)
        });
    
    let binding_op = create_binding_op(
        job.root.xref,
        ir::BindingKind::Attribute,
        name,
        BindingExpression::Expression(value.clone()),
        None, // unit
        security_contexts,
        true, // is_text_attr - always true for host attributes
        false, // is_structural_template_attribute
        None,  // template_kind
        None,  // i18n_message
        source_span,
    );
    
    job.root.update.push(binding_op);
}

/// Ingest a host event binding
fn ingest_host_event(
    job: &mut HostBindingCompilationJob,
    event: ParsedEvent,
) {
    use crate::expression_parser::ast::ParsedEventType;
    use crate::template::pipeline::ir::handle::SlotHandle;
    use crate::template::pipeline::src::conversion::convert_ast;
    use crate::template::pipeline::ir::ops::shared::create_statement_op;
    use crate::output::output_ast::{Statement, ReturnStatement};
    
    // Create handler ops - simplified version for host events
    // Use the same approach as make_listener_handler_ops
    use crate::template::pipeline::ir::operations::OpList;
    let mut handler_ops: OpList<Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>> = OpList::new();
    let handler_expr = convert_ast(&event.handler, job, Some(&event.handler_span));
    let return_stmt = ReturnStatement {
        value: Box::new(handler_expr),
        source_span: Some(event.handler_span.clone()),
    };
    let stmt = Statement::Return(return_stmt);
    let stmt_op = create_statement_op::<Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>>(Box::new(stmt));
    // Box the StatementOp - it implements UpdateOp so can be coerced to trait object
    handler_ops.push(Box::new(stmt_op) as Box<dyn crate::template::pipeline::ir::operations::UpdateOp + Send + Sync>);
    
    match event.event_type {
        ParsedEventType::Animation => {
            // Determine animation kind based on event name
            let animation_kind = if event.name.ends_with("enter") {
                ir::enums::AnimationKind::Enter
            } else {
                ir::enums::AnimationKind::Leave
            };
            
            // Create animation listener op
            let animation_listener_op = ir::ops::create::create_animation_listener_op(
                job.root.xref,
                SlotHandle::default(),
                event.name,
                None, // tag - host listeners don't have tags
                handler_ops,
                animation_kind,
                event.target_or_phase.clone(), // event_target
                true, // host_listener
                event.source_span,
            );
            job.root.create.push(animation_listener_op);
        }
        ParsedEventType::Regular => {
            // Create regular listener op
            let listener_op = ir::ops::create::create_listener_op(
                job.root.xref,
                SlotHandle::default(),
                event.name,
                None, // tag
                handler_ops,
                None, // legacy_animation_phase
                event.target_or_phase.clone(), // event_target
                true, // host_listener
                event.source_span,
            );
            job.root.create.push(listener_op);
        }
        ParsedEventType::TwoWay => {
            // TwoWay events use create_two_way_listener_op
            let two_way_handler_ops = make_host_listener_handler_ops(&event.handler, &event.handler_span, job);
            let two_way_listener_op = ir::ops::create::create_two_way_listener_op(
                job.root.xref,
                SlotHandle::default(),
                event.name,
                None, // tag
                two_way_handler_ops,
                event.source_span,
            );
            job.root.create.push(two_way_listener_op);
        }
        ParsedEventType::LegacyAnimation => {
            // LegacyAnimation events use phase instead of target
            // For LegacyAnimation, target_or_phase contains the phase
            let listener_op = ir::ops::create::create_listener_op(
                job.root.xref,
                SlotHandle::default(),
                event.name,
                None, // tag
                handler_ops,
                event.target_or_phase.clone(), // legacy_animation_phase (phase is stored in target_or_phase for LegacyAnimation)
                None, // event_target (null for LegacyAnimation)
                true, // host_listener
                event.source_span,
            );
            job.root.create.push(listener_op);
        }
    }
}
