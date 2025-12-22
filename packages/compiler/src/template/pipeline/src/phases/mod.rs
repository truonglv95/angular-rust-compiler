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
pub mod defer_configs;
pub mod defer_resolve_targets;
pub mod resolve_defer_deps_fns;
pub mod remove_content_selectors;
pub mod remove_empty_bindings;
pub mod deduplicate_text_bindings;
pub mod ng_container;
pub mod namespace;
pub mod nonbindable;
pub mod empty_elements;
pub mod generate_local_let_references;
pub mod has_const_expression_collection;
pub mod remove_illegal_let_references;
pub mod expand_safe_reads;
pub mod store_let_optimization;
pub mod track_variables;
pub mod resolve_dollar_event;
pub mod resolve_contexts;
pub mod resolve_names;
pub mod resolve_sanitizers;
pub mod save_restore_view;
pub mod local_refs;
pub mod generate_advance;
pub mod generate_projection_def;
pub mod generate_variables;
pub mod host_style_property_parsing;
pub mod temporary_variables;
pub mod var_counting;
pub mod variable_optimization;

pub mod naming;
pub mod next_context_merging;
pub mod ordering;
pub mod parse_extracted_styles;
pub mod reify;
pub mod pipe_creation;
pub mod pipe_variadic;
pub mod pure_function_extraction;
pub mod pure_literal_structures;
pub mod slot_allocation;
pub mod style_binding_specialization;
pub mod strip_nonrequired_parentheses;
pub mod track_fn_optimization;
pub mod transform_two_way_binding_set;
pub mod wrap_icus;
pub mod propagate_i18n_blocks;
pub mod remove_unused_i18n_attrs;
pub mod remove_i18n_contexts;
pub mod resolve_i18n_expression_placeholders;
pub mod create_i18n_contexts;
pub mod convert_i18n_bindings;
pub mod i18n_text_extraction;
pub mod extract_i18n_messages;
pub mod resolve_i18n_element_placeholders;
pub mod i18n_const_collection;


use crate::template::pipeline::src::compilation::ComponentCompilationJob;

pub fn run(job: &mut ComponentCompilationJob) {
    // Simplified phase order for vars debugging
    pure_literal_structures::phase(job);
    generate_variables::phase(job);
    resolve_names::phase(job);
    resolve_contexts::phase(job);
    
    // Added phases for correctness
    binding_specialization::specialize_bindings(job); // Converts BindingOp -> AttributeOp, PropertyOp, etc.
    attribute_extraction::extract_attributes(job);
    namespace::emit_namespace_changes(job);
    const_collection::collect_element_consts(job);
    
    // Resolve sanitizers for security-sensitive properties/attributes (e.g. href, src)
    resolve_sanitizers::resolve_sanitizers(job);
    
    slot_allocation::phase(job);
    pure_function_extraction::phase(job); // Extract pure functions to constants like _c0, _c1
    track_fn_optimization::optimize_track_fns(job); // Generate track functions for @for loops
    var_counting::phase(job);
    variable_optimization::optimize_variables(job); // Remove unused variables
    naming::name_functions_and_variables(job);
    generate_advance::phase(job);
    reify::reify(job);
}
