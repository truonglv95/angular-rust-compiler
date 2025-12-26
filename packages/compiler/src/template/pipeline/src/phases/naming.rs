//! Generate names for functions and variables across all views.
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/naming.ts

use crate::parse_util::sanitize_identifier;
use crate::template::pipeline::ir;
use crate::template::pipeline::ir::operations::UpdateOp;
use crate::template::pipeline::ir::ops::create::{
    ConditionalCreateOp, ProjectionOp, RepeaterCreateOp, TemplateOp,
};
use crate::template::pipeline::ir::ops::host::DomPropertyOp;
use crate::template::pipeline::ir::ops::shared::VariableOp;
use crate::template::pipeline::ir::ops::update::{ClassPropOp, PropertyOp, StylePropOp};
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob, ViewCompilationUnit,
};

struct NamingState {
    index: usize,
}

// Struct needed for recursion tasks
struct RecTask {
    xref: ir::XrefId,
    name: String,
}

pub fn name_functions_and_variables(job: &mut ComponentCompilationJob) {
    let mut state = NamingState { index: 0 };
    let root_xref = job.root.xref;

    // Check compatibility mode (TemplateDefinitionBuilder)
    let compatibility = job.compatibility == ir::CompatibilityMode::TemplateDefinitionBuilder;

    // Determine base name
    let _fn_suffix = job.fn_suffix().to_string();
    let component_name = job.component_name.clone();

    // Logic from naming.ts: ensure unique names for view units
    // In Rust, we might need to access the pool.
    // However, unit.fn_name might already be null.

    // We start traversal from root.
    // The baseName calculation happens inside addNamesToView in TS,
    // but the first call passes `job.componentName`.

    process_view(job, root_xref, component_name, &mut state, compatibility);
}

fn process_view_safe(
    job: &mut ComponentCompilationJob,
    unit_xref: ir::XrefId,
    base_name: String,
    state: &mut NamingState,
    compatibility: bool,
) {
    let needs_name = {
        let unit = get_view_mut(job, unit_xref);
        unit.fn_name().is_none()
    };

    // Update base_name for root view in compatibility mode
    // effective_base_name is used for the *current* view's function name.
    let mut effective_base_name = base_name.clone();
    if compatibility && unit_xref == job.root.xref && !base_name.ends_with("_Template") {
        effective_base_name.push_str("_Template");
    }

    let fn_suffix = job.fn_suffix().to_string();

    // child_base_name is used as the prefix for *embedded* views.
    // For the root view, we want to use the original component name (e.g. "NgForTest") as the prefix,
    // NOT "NgForTest_Template", to avoid double suffixes like "NgForTest_Template_div_0_Template".
    let query_root = job.root.xref;
    let child_base_name = if compatibility && unit_xref == query_root {
        // Use original base name (e.g. "NgForTest")
        base_name.as_str()
    } else {
        // For nested views, us effective_base_name which is "NgForTest_div_0" (no suffix yet)
        effective_base_name.as_str()
    };

    // listener_base_name is used for naming listeners.
    // Listeners should be prefixed with the View's Function Name (e.g. "NgForTest_Template" or "NgForTest_div_0_Template")
    let listener_base_name = if effective_base_name.ends_with(&format!("_{}", fn_suffix))
        || effective_base_name.ends_with("_Template")
    {
        effective_base_name.clone()
    } else {
        format!("{}_{}", effective_base_name, fn_suffix)
    };

    if needs_name {
        // Use listener_base_name as the candidate for function name
        let candidate = sanitize_identifier(&listener_base_name);
        // Need pool access.
        let unique_name = job.pool.unique_name(candidate, false);

        // Re-acquire unit
        let unit = get_view_mut(job, unit_xref);
        unit.set_fn_name(unique_name);
    }

    // 2. Iterate ops - first pass: name variables and recurse immediately (DFS)
    let mut var_names: std::collections::HashMap<ir::XrefId, String> =
        std::collections::HashMap::new();

    // Cache for reusing variable names within this view based on their semantic identity.
    // Key: (VariableKind discriminator, Identifier String or Context View Xref)
    // We use a simplified string key for now since Rust enums are hard to hash directly without implementing Hash.
    let mut var_name_cache: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    let total_ops = {
        let unit = get_view_mut(job, unit_xref);
        unit.create.len() + unit.update.len()
    };

    for i in 0..total_ops {
        let mut tasks = Vec::new();
        {
            let unit = get_view_mut(job, unit_xref);
            let create_len = unit.create.len();
            let is_create = i < create_len;
            let idx = if is_create { i } else { i - create_len };

            if is_create {
                if let Some(op) = unit.create.get_mut(idx) {
                    process_op_with_var_names(
                        &mut **op,
                        listener_base_name.as_str(),
                        child_base_name,
                        state,
                        compatibility,
                        &mut tasks,
                        &mut var_names,
                        true,
                        &mut var_name_cache,
                    );
                }
            } else {
                if let Some(op) = unit.update.get_mut(idx) {
                    process_op_with_var_names(
                        &mut **op,
                        listener_base_name.as_str(),
                        child_base_name,
                        state,
                        compatibility,
                        &mut tasks,
                        &mut var_names,
                        false,
                        &mut var_name_cache,
                    );
                }
            }
        } // Drop unit borrow to allow recursion

        // Recurse immediately (DFS)
        for task in tasks {
            process_view_safe(job, task.xref, task.name, state, compatibility);
        }
    }

    // 3. Second pass: propagate variable names into ReadVariableExpr within the current view
    // This happens after all variables (local and children) have been named/visited.
    {
        let unit = get_view_mut(job, unit_xref);

        for op in unit.create.iter_mut() {
            apply_names_to_op_recursive(op.as_mut(), &var_names);
        }

        for op in unit.update.iter_mut() {
            apply_names_to_op_recursive(op.as_mut(), &var_names);
        }
    }
}

fn apply_names_to_op_recursive(
    op: &mut dyn ir::Op,
    var_names: &std::collections::HashMap<ir::XrefId, String>,
) {
    use crate::output::output_ast::Expression;
    use crate::template::pipeline::ir::expression::transform_expressions_in_op;

    transform_expressions_in_op(
        op,
        &mut |mut expr: Expression, _flags| {
            if let Expression::ReadVariable(ref mut read_var) = expr {
                if read_var.name.is_none() {
                    if let Some(name) = var_names.get(&read_var.xref) {
                        read_var.name = Some(name.clone());
                    } else {
                        // Variable from parent scope - generate fallback name
                        let fallback = format!("_r{}", read_var.xref.0);
                        read_var.name = Some(fallback);
                    }
                }
            }
            expr
        },
        ir::VisitorContextFlag::NONE,
    );

    unsafe {
        let op_ptr = op as *mut dyn ir::Op;
        match op.kind() {
            ir::OpKind::Listener => {
                use crate::template::pipeline::ir::ops::create::ListenerOp;
                let listener = &mut *(op_ptr as *mut ListenerOp);
                for handler_op in &mut listener.handler_ops {
                    apply_names_to_op_recursive(handler_op.as_mut(), var_names);
                }
            }
            ir::OpKind::AnimationListener => {
                use crate::template::pipeline::ir::ops::create::AnimationListenerOp;
                let listener = &mut *(op_ptr as *mut AnimationListenerOp);
                for handler_op in &mut listener.handler_ops {
                    apply_names_to_op_recursive(handler_op.as_mut(), var_names);
                }
            }
            ir::OpKind::TwoWayListener => {
                use crate::template::pipeline::ir::ops::create::TwoWayListenerOp;
                let listener = &mut *(op_ptr as *mut TwoWayListenerOp);
                for handler_op in &mut listener.handler_ops {
                    apply_names_to_op_recursive(handler_op.as_mut(), var_names);
                }
            }
            _ => {}
        }
    }
}

fn process_op_with_var_names(
    op: &mut dyn ir::Op,
    base_name: &str,
    child_base_name: &str,
    state: &mut NamingState,
    compatibility: bool,
    tasks: &mut Vec<RecTask>,
    var_names: &mut std::collections::HashMap<ir::XrefId, String>,
    is_create_op: bool,
    var_name_cache: &mut std::collections::HashMap<String, String>,
) {
    unsafe {
        let op_ptr = op as *mut dyn ir::Op;

        match op.kind() {
            ir::OpKind::Property => {
                let prop_ptr = op_ptr as *mut PropertyOp;
                let prop = &mut *prop_ptr;
                if prop.binding_kind == ir::BindingKind::LegacyAnimation {
                    prop.name = format!("@{}", prop.name);
                }
            }

            ir::OpKind::DomProperty => {
                let dom_prop_ptr = op_ptr as *mut DomPropertyOp;
                let _dom_prop = &mut *dom_prop_ptr;
                // DomPropertyOp - no special handling needed
            }
            ir::OpKind::Animation => {
                use crate::template::pipeline::ir::ops::create::AnimationOp;
                let anim_ptr = op_ptr as *mut AnimationOp;
                let anim = &mut *anim_ptr;
                if anim.handler_fn_name.is_none() {
                    let animation_kind = anim.name.replace('.', "");
                    let fn_name = format!("{}_{}_cb", base_name, animation_kind);
                    anim.handler_fn_name = Some(sanitize_identifier(&fn_name));
                }
            }
            ir::OpKind::AnimationListener => {
                use crate::template::pipeline::ir::ops::create::AnimationListenerOp;
                let anim_listener_ptr = op_ptr as *mut AnimationListenerOp;
                let anim_listener = &mut *anim_listener_ptr;
                if anim_listener.handler_fn_name.is_none() {
                    let animation_kind = anim_listener.name.replace('.', "");
                    if anim_listener.host_listener {
                        anim_listener.handler_fn_name = Some(sanitize_identifier(&format!(
                            "{}_{}_HostBindingHandler",
                            base_name, animation_kind
                        )));
                    } else {
                        let slot = anim_listener
                            .target_slot
                            .get_slot()
                            .expect("Expected a slot to be assigned");
                        let tag = anim_listener
                            .tag
                            .as_ref()
                            .map(|t| t.replace('-', "_"))
                            .unwrap_or_default();
                        anim_listener.handler_fn_name = Some(sanitize_identifier(&format!(
                            "{}_{}_{}_{}_listener",
                            base_name, tag, animation_kind, slot
                        )));
                    }
                }

                // Recurse into handler_ops
                let anim_listener_ptr = op_ptr as *mut AnimationListenerOp;
                let anim_listener = &mut *anim_listener_ptr;
                for handler_op in &mut anim_listener.handler_ops {
                    process_op_with_var_names(
                        &mut **handler_op,
                        base_name,
                        child_base_name,
                        state,
                        compatibility,
                        tasks,
                        var_names,
                        false,
                        var_name_cache,
                    );
                }
            }
            ir::OpKind::Listener => {
                use crate::template::pipeline::ir::ops::create::ListenerOp;
                let listener_ptr = op_ptr as *mut ListenerOp;
                let listener = &mut *listener_ptr;
                if listener.handler_fn_name.is_none() {
                    let mut animation = String::new();
                    let mut event_name = listener.name.clone();
                    if listener.is_legacy_animation_listener {
                        let phase = listener.legacy_animation_phase.as_ref().expect(
                            "legacy_animation_phase must be set for legacy animation listener",
                        );
                        event_name = format!("@{}.{}", listener.name, phase);
                        animation = "_animation".to_string();
                    }

                    if listener.host_listener {
                        listener.handler_fn_name = Some(sanitize_identifier(&format!(
                            "{}{}{}_HostBindingHandler",
                            base_name, animation, event_name
                        )));
                    } else {
                        let slot = listener
                            .target_slot
                            .get_slot()
                            .expect("Expected a slot to be assigned");
                        let tag = listener
                            .tag
                            .as_ref()
                            .map(|t| t.replace('-', "_"))
                            .unwrap_or_default();
                        listener.handler_fn_name = Some(sanitize_identifier(&format!(
                            "{}_{}{}_{}_{}_listener",
                            base_name, tag, animation, event_name, slot
                        )));
                    }
                }

                // Recurse into handler_ops
                let listener_ptr = op_ptr as *mut ListenerOp;
                let listener = &mut *listener_ptr;
                for handler_op in &mut listener.handler_ops {
                    process_op_with_var_names(
                        &mut **handler_op,
                        base_name,
                        child_base_name,
                        state,
                        compatibility,
                        tasks,
                        var_names,
                        false,
                        var_name_cache,
                    );
                }
            }
            ir::OpKind::TwoWayListener => {
                use crate::template::pipeline::ir::ops::create::TwoWayListenerOp;
                let two_way_ptr = op_ptr as *mut TwoWayListenerOp;
                let two_way = &mut *two_way_ptr;
                if two_way.handler_fn_name.is_none() {
                    let slot = two_way
                        .target_slot
                        .get_slot()
                        .expect("Expected a slot to be assigned");
                    let tag = two_way
                        .tag
                        .as_ref()
                        .map(|t| t.replace('-', "_"))
                        .unwrap_or_default();
                    two_way.handler_fn_name = Some(sanitize_identifier(&format!(
                        "{}_{}_{}_{}_twoWayListener",
                        base_name, tag, two_way.name, slot
                    )));
                }

                // Recurse into handler_ops
                let two_way_ptr = op_ptr as *mut TwoWayListenerOp;
                let two_way = &mut *two_way_ptr;
                for handler_op in &mut two_way.handler_ops {
                    process_op_with_var_names(
                        &mut **handler_op,
                        base_name,
                        child_base_name,
                        state,
                        compatibility,
                        tasks,
                        var_names,
                        false,
                        var_name_cache,
                    );
                }
            }
            ir::OpKind::Variable => {
                // VariableOp - downcast based on whether this is a create or update op
                // Create ops use VariableOp<Box<dyn ir::CreateOp + Send + Sync>>
                // Update ops use VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>

                if is_create_op {
                    if let Some(var_op) = op
                        .as_any_mut()
                        .downcast_mut::<VariableOp<Box<dyn ir::CreateOp + Send + Sync>>>()
                    {
                        let name = {
                            // Generate cache key
                            let key = match &var_op.variable {
                                ir::SemanticVariable::Identifier(ident) => {
                                    Some(format!("IDENT:{}", ident.identifier))
                                }
                                ir::SemanticVariable::Context(ctx) => {
                                    Some(format!("CTX:{:?}", ctx.view))
                                }
                                _ => None,
                            };

                            if let Some(k) = key {
                                if let Some(cached) = var_name_cache.get(&k) {
                                    cached.clone()
                                } else {
                                    let new_name =
                                        get_variable_name(&var_op.variable, state, compatibility);
                                    var_name_cache.insert(k, new_name.clone());
                                    new_name
                                }
                            } else {
                                get_variable_name(&var_op.variable, state, compatibility)
                            }
                        };

                        if var_op.xref.0 == 134 {}
                        var_names.insert(var_op.xref, name.clone());
                        var_op.variable.set_name(Some(name));
                    } else {
                        if let Some(var_op) = op
                            .as_any_mut()
                            .downcast_mut::<VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>>()
                        {
                            let name = {
                                // Generate cache key
                                let key = match &var_op.variable {
                                    ir::SemanticVariable::Identifier(ident) => {
                                        Some(format!("IDENT:{}", ident.identifier))
                                    }
                                    ir::SemanticVariable::Context(ctx) => {
                                        Some(format!("CTX:{:?}", ctx.view))
                                    }
                                    _ => None,
                                };

                                if let Some(k) = key {
                                    if let Some(cached) = var_name_cache.get(&k) {
                                        cached.clone()
                                    } else {
                                        let new_name = get_variable_name(
                                            &var_op.variable,
                                            state,
                                            compatibility,
                                        );
                                        var_name_cache.insert(k, new_name.clone());
                                        new_name
                                    }
                                } else {
                                    get_variable_name(&var_op.variable, state, compatibility)
                                }
                            };
                            if var_op.xref.0 == 134 {}
                            var_names.insert(var_op.xref, name.clone());
                            var_op.variable.set_name(Some(name));
                        } else {
                        }
                    }
                } else {
                    if let Some(var_op) = op
                        .as_any_mut()
                        .downcast_mut::<VariableOp<Box<dyn ir::UpdateOp + Send + Sync>>>()
                    {
                        let name = {
                            // Generate cache key
                            let key = match &var_op.variable {
                                ir::SemanticVariable::Identifier(ident) => {
                                    Some(format!("IDENT:{}", ident.identifier))
                                }
                                ir::SemanticVariable::Context(ctx) => {
                                    Some(format!("CTX:{:?}", ctx.view))
                                }
                                _ => None,
                            };

                            if let Some(k) = key {
                                if let Some(cached) = var_name_cache.get(&k) {
                                    cached.clone()
                                } else {
                                    let new_name =
                                        get_variable_name(&var_op.variable, state, compatibility);
                                    var_name_cache.insert(k, new_name.clone());
                                    new_name
                                }
                            } else {
                                get_variable_name(&var_op.variable, state, compatibility)
                            }
                        };
                        if var_op.xref.0 == 134 {}
                        var_names.insert(var_op.xref, name.clone());
                        var_op.variable.set_name(Some(name));
                    } else {
                    }
                }
            }
            ir::OpKind::RepeaterCreate => {
                let rep_ptr = op_ptr as *mut RepeaterCreateOp;
                let rep = &mut *rep_ptr;
                if let Some(empty) = rep.empty_view {
                    let index = rep.base.base.handle.get_slot().unwrap();
                    tasks.push(RecTask {
                        xref: empty,
                        name: format!(
                            "{}_{}Empty_{}",
                            child_base_name,
                            rep.function_name_suffix,
                            index + 2
                        ),
                    });
                }
                let index = rep.base.base.handle.get_slot().unwrap();
                tasks.push(RecTask {
                    xref: rep.base.base.xref,
                    name: format!(
                        "{}_{}_{}",
                        child_base_name,
                        rep.function_name_suffix,
                        index + 1
                    ),
                });
            }
            ir::OpKind::Projection => {
                let proj_ptr = op_ptr as *mut ProjectionOp;
                let proj = &mut *proj_ptr;
                if let Some(fallback) = proj.fallback_view {
                    let index = proj.handle.get_slot().unwrap();
                    tasks.push(RecTask {
                        xref: fallback,
                        name: format!("{}_ProjectionFallback_{}", child_base_name, index),
                    });
                }
            }
            ir::OpKind::Template => {
                let template_ptr = op_ptr as *mut TemplateOp;
                let template = &mut *template_ptr;
                let suffix = if template.function_name_suffix.is_empty() {
                    String::new()
                } else {
                    format!("_{}", template.function_name_suffix)
                };
                let index = template.base.base.handle.get_slot().unwrap();
                tasks.push(RecTask {
                    xref: template.base.base.xref,
                    name: format!("{}{}_{}", child_base_name, suffix, index),
                });
            }
            ir::OpKind::ConditionalCreate => {
                let cond_ptr = op_ptr as *mut ConditionalCreateOp;
                let cond = &mut *cond_ptr;
                let suffix = if cond.function_name_suffix.is_empty() {
                    String::new()
                } else {
                    format!("_{}", cond.function_name_suffix)
                };
                let slot = cond.base.base.handle.get_slot().unwrap();
                tasks.push(RecTask {
                    xref: cond.base.base.xref,
                    name: format!("{}{}_{}", child_base_name, suffix, slot),
                });
            }
            ir::OpKind::ConditionalBranchCreate => {
                use crate::template::pipeline::ir::ops::create::ConditionalBranchCreateOp;
                let branch_ptr = op_ptr as *mut ConditionalBranchCreateOp;
                let branch = &mut *branch_ptr;
                let suffix = if branch.function_name_suffix.is_empty() {
                    String::new()
                } else {
                    format!("_{}", branch.function_name_suffix)
                };
                let slot = branch.base.base.handle.get_slot().unwrap();
                tasks.push(RecTask {
                    xref: branch.base.base.xref,
                    name: format!("{}{}_{}", child_base_name, suffix, slot),
                });
            }
            ir::OpKind::StyleProp => {
                let style_ptr = op_ptr as *mut StylePropOp;
                let style = &mut *style_ptr;
                style.name = normalize_style_prop_name(&style.name);
                if compatibility {
                    style.name = strip_important(&style.name);
                }
            }
            ir::OpKind::ClassProp => {
                let class_ptr = op_ptr as *mut ClassPropOp;
                let class = &mut *class_ptr;
                if compatibility {
                    class.name = strip_important(&class.name);
                }
            }
            ir::OpKind::Attribute => {
                // AttributeOp - no special handling needed currently
            }
            _ => {}
        }
    }
}

// Map the original function name to this new Safe implementation
fn process_view(
    job: &mut ComponentCompilationJob,
    unit_xref: ir::XrefId,
    base_name: String,
    state: &mut NamingState,
    compatibility: bool,
) {
    process_view_safe(job, unit_xref, base_name, state, compatibility);
}

fn get_view_mut(job: &mut ComponentCompilationJob, xref: ir::XrefId) -> &mut ViewCompilationUnit {
    if xref == job.root.xref {
        &mut job.root
    } else {
        job.views.get_mut(&xref).expect("View not found")
    }
}

fn get_variable_name(
    variable: &ir::SemanticVariable,
    state: &mut NamingState,
    compatibility: bool,
) -> String {
    if let Some(name) = variable.name() {
        return name.to_string();
    }

    match variable {
        ir::SemanticVariable::Context(_) => {
            let idx = state.index;
            state.index += 1;
            format!("ctx_r{}", idx)
        }
        ir::SemanticVariable::Identifier(ident_var) => {
            if compatibility {
                let compat_prefix = if ident_var.identifier == "ctx" {
                    "i"
                } else {
                    ""
                };
                let idx = state.index + 1; // Pre-increment in TS: ++state.index
                state.index += 1;
                format!("{}_{}r{}", ident_var.identifier, compat_prefix, idx)
            } else {
                let idx = state.index;
                state.index += 1;
                format!("{}_i{}", ident_var.identifier, idx)
            }
        }
        ir::SemanticVariable::SavedView(_) => {
            // For now, use fallback for saved views, or skip naming if they are internal
            let idx = state.index + 1;
            state.index += 1;
            format!("_r{}", idx)
        }
        _ => {
            // Fallback for other types
            let idx = state.index + 1;
            state.index += 1;
            format!("_r{}", idx)
        }
    }
}

fn normalize_style_prop_name(name: &str) -> String {
    if name.starts_with("--") {
        name.to_string()
    } else {
        hyphenate(name)
    }
}

fn hyphenate(value: &str) -> String {
    let mut result = String::new();
    for c in value.chars() {
        if c.is_uppercase() {
            result.push('-');
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

fn strip_important(name: &str) -> String {
    if let Some(idx) = name.find("!important") {
        name[..idx].to_string()
    } else {
        name.to_string()
    }
}
