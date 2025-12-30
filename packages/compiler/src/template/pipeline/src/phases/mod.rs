//! Pipeline Phases Module
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/
//! Contains 60+ transformation phases

pub mod any_cast;
pub mod apply_i18n_expressions;
pub mod assign_i18n_slot_dependencies;
pub mod attach_source_locations;
pub mod attribute_extraction;
pub mod binding_specialization;
pub mod chaining;
pub mod collapse_singleton_interpolations;
pub mod conditionals;
pub mod const_collection;
pub mod convert_animations;
pub mod deduplicate_text_bindings;
pub mod defer_configs;
pub mod defer_resolve_targets;
pub mod diagnostics;
pub mod empty_elements;
pub mod expand_safe_reads;
pub mod generate_advance;
pub mod generate_local_let_references;
pub mod generate_projection_def;
pub mod generate_variables;
pub mod has_const_expression_collection;
pub mod host_style_property_parsing;
pub mod local_refs;
pub mod namespace;
pub mod ng_container;
pub mod nonbindable;
pub mod remove_content_selectors;
pub mod remove_empty_bindings;
pub mod remove_illegal_let_references;
pub mod resolve_contexts;
pub mod resolve_defer_deps_fns;
pub mod resolve_definitions;
pub mod resolve_dollar_event;
pub mod resolve_names;
pub mod resolve_sanitizers;
pub mod save_restore_view;
pub mod store_let_optimization;
pub mod temporary_variables;
pub mod track_variables;
pub mod var_counting;
pub mod variable_optimization;

pub mod convert_i18n_bindings;
pub mod create_i18n_contexts;
pub mod extract_i18n_messages;
pub mod i18n_const_collection;
pub mod i18n_text_extraction;
pub mod naming;
pub mod next_context_merging;
pub mod ordering;
pub mod parse_extracted_styles;
pub mod pipe_creation;
pub mod pipe_variadic;
pub mod propagate_i18n_blocks;
pub mod pure_function_extraction;
pub mod pure_literal_structures;
pub mod reify;
pub mod remove_i18n_contexts;
pub mod remove_unused_i18n_attrs;
pub mod resolve_i18n_element_placeholders;
pub mod resolve_i18n_expression_placeholders;
pub mod slot_allocation;
pub mod strip_nonrequired_parentheses;
pub mod style_binding_specialization;
pub mod track_fn_optimization;
pub mod transform_two_way_binding_set;
pub mod wrap_icus;

use crate::template::pipeline::src::compilation::{CompilationJob, ComponentCompilationJob};

pub fn run(job: &mut ComponentCompilationJob) {
    // Simplified phase order for vars debugging
    pure_literal_structures::phase(job);
    generate_variables::phase(job); // Generate context variables including $implicit

    save_restore_view::save_and_restore_view(job); // Save/restore view for listeners - MUST run AFTER generate_variables so RestoreView is prepended last (appears first)
    resolve_names::phase(job);
    resolve_contexts::phase(job);

    // Remove $any() calls - they have no runtime effects
    any_cast::delete_any_casts(job);

    // Added phases for correctness
    binding_specialization::specialize_bindings(job); // Converts BindingOp -> AttributeOp, PropertyOp, etc.
    attribute_extraction::extract_attributes(job);
    local_refs::lift_local_refs(job); // Lift local refs (#templateName) to consts for templateRefExtractor
    namespace::emit_namespace_changes(job);

    empty_elements::collapse_empty_instructions(job); // Merge ElementStart+ElementEnd -> Element for empty elements
    const_collection::collect_element_consts(job);

    // Resolve sanitizers for security-sensitive properties/attributes (e.g. href, src)
    resolve_sanitizers::resolve_sanitizers(job);

    // Resolve definitions (used directives/pipes)
    resolve_definitions::phase(job);

    // Create pipe operations before slot allocation
    pipe_creation::create_pipes(job);

    slot_allocation::phase(job);
    pure_function_extraction::phase(job); // Extract pure functions to constants like _c0, _c1
    track_fn_optimization::optimize_track_fns(job); // Generate track functions for @for loops
    var_counting::phase(job);
    variable_optimization::optimize_variables(job); // Remove unused variables

    next_context_merging::merge_next_context_expressions(job); // Merge sequential nextContext() calls

    naming::name_functions_and_variables(job);
    generate_advance::phase(job);
    conditionals::generate_conditional_expressions(job); // Collapse conditional expressions to single ternary
    diagnostics::phase(job);
    transform_two_way_binding_set::transform_two_way_binding_set(job);
    reify::reify(job);
    chaining::chain(job);
}

pub fn run_host(job: &mut crate::template::pipeline::src::compilation::HostBindingCompilationJob) {
    binding_specialization::specialize_bindings(job);
    attribute_extraction::extract_attributes(job);
    const_collection::collect_element_consts(job);
    resolve_names::phase(job);
    resolve_contexts::phase(job);

    // Remove $any() calls - they have no runtime effects
    any_cast::delete_any_casts(job);

    // Host bindings may have property bindings that need var counting
    var_counting::phase(job);

    naming::name_functions_and_variables(job);

    // Two-way bindings for host are rare but possible if ngModel is on host
    transform_two_way_binding_set::transform_two_way_binding_set(job);

    reify::reify(job);
    chaining::chain(job);
}
