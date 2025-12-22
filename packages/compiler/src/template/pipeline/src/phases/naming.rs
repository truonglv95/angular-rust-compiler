//! Generate names for functions and variables across all views.
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/naming.ts

use crate::template::pipeline::ir;
use crate::template::pipeline::src::compilation::{
    CompilationJob, ComponentCompilationJob, CompilationUnit, ViewCompilationUnit,
};
use crate::parse_util::sanitize_identifier;
use crate::template::pipeline::ir::ops::create::{
    RepeaterCreateOp, TemplateOp, ConditionalCreateOp, ProjectionOp,
};
use crate::template::pipeline::ir::ops::update::{
    PropertyOp, StylePropOp, ClassPropOp,
};
use crate::template::pipeline::ir::ops::shared::VariableOp;
use crate::template::pipeline::ir::ops::host::DomPropertyOp;
use crate::template::pipeline::ir::operations::UpdateOp;


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
     // 1. Name assignment (needs pool access)
     let needs_name = {
         let unit = get_view_mut(job, unit_xref);
         unit.fn_name().is_none()
     };
     
     if needs_name {
         let fn_suffix = job.fn_suffix().to_string();
         let candidate = sanitize_identifier(&format!("{}_{}", base_name, fn_suffix));
         // Need pool access.
         // Split access?
         // let unique_name = job.pool.unique_name(...);
         // But to do this we must not hold unit.
         // We dropped unit above.
         let unique_name = job.pool.unique_name(candidate, false);
         
         // Re-acquire unit
         let unit = get_view_mut(job, unit_xref);
         unit.set_fn_name(unique_name);
     }
     
    // 2. Iterate ops - first pass: name variables and collect tasks
    let mut var_names: std::collections::HashMap<ir::XrefId, String> = std::collections::HashMap::new();
    let mut tasks = Vec::new();
    
    {
        let unit = get_view_mut(job, unit_xref);
        let create_len = unit.create.len();
        let update_len = unit.update.len();
        let total = create_len + update_len;
        
        for i in 0..total {
            let is_create = i < create_len;
            let idx = if is_create { i } else { i - create_len };
            
            if is_create {
                let op = unit.create.get_mut(idx).unwrap();
                process_op_with_var_names(&mut **op, base_name.as_str(), state, compatibility, &mut tasks, &mut var_names, true);
            } else {
                let op = unit.update.get_mut(idx).unwrap();
                process_op_with_var_names(&mut **op, base_name.as_str(), state, compatibility, &mut tasks, &mut var_names, false);
            }
        }
    } // Drop unit borrow
    
    // 3. Second pass: propagate variable names into ReadVariableExpr
    {
        let unit = get_view_mut(job, unit_xref);
        use crate::template::pipeline::ir::expression::transform_expressions_in_op;
        use crate::output::output_ast::Expression;
        
        let create_len = unit.create.len();
        let update_len = unit.update.len();
        let total = create_len + update_len;
        
        for i in 0..total {
            let is_create = i < create_len;
            let idx = if is_create { i } else { i - create_len };
            
            if is_create {
                let op = unit.create.get_mut(idx).unwrap();
                transform_expressions_in_op(
                    &mut **op,
                    &mut |mut expr: Expression, _flags| {
                        if let Expression::ReadVariable(ref mut read_var) = expr {
                            if read_var.name.is_none() {
                                if let Some(name) = var_names.get(&read_var.xref) {
                                    read_var.name = Some(name.clone());
                                } else {
                                    panic!("PANIC: Variable {:?} not yet named in view {:?}. Available: {:?}", read_var.xref, unit_xref, var_names.keys());
                                }
                            }
                        }
                        expr
                    },
                    ir::VisitorContextFlag::NONE,
                );
            } else {
                let op = unit.update.get_mut(idx).unwrap();
                transform_expressions_in_op(
                    &mut **op,
                    &mut |mut expr: Expression, _flags| {
                        if let Expression::ReadVariable(ref mut read_var) = expr {
                            if read_var.name.is_none() {
                                if let Some(name) = var_names.get(&read_var.xref) {
                                    read_var.name = Some(name.clone());
                                } else {
                                    panic!("PANIC: Variable {:?} not yet named in view {:?}. Available: {:?}", read_var.xref, unit_xref, var_names.keys());
                                }
                            }
                        }
                        expr
                    },
                    ir::VisitorContextFlag::NONE,
                );
            }
        }
    } // Drop unit borrow
     
     // Recurse
     for task in tasks {
         process_view_safe(job, task.xref, task.name, state, compatibility);
     }
}

fn process_op_with_var_names(
    op: &mut dyn ir::Op,
    base_name: &str,
    state: &mut NamingState,
    compatibility: bool,
    tasks: &mut Vec<RecTask>,
    var_names: &mut std::collections::HashMap<ir::XrefId, String>,
    is_create_op: bool,
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
                        anim_listener.handler_fn_name = Some(sanitize_identifier(
                            &format!("{}_{}_HostBindingHandler", base_name, animation_kind)
                        ));
                    } else {
                        let slot = anim_listener.target_slot.slot.expect("Expected a slot to be assigned");
                        let tag = anim_listener.tag.as_ref().map(|t| t.replace('-', "_")).unwrap_or_default();
                        anim_listener.handler_fn_name = Some(sanitize_identifier(
                            &format!("{}_{}_{}_{}_listener", base_name, tag, animation_kind, slot)
                        ));
                    }
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
                        let phase = listener.legacy_animation_phase.as_ref()
                            .expect("legacy_animation_phase must be set for legacy animation listener");
                        event_name = format!("@{}.{}", listener.name, phase);
                        animation = "animation".to_string();
                    }
                    
                    if listener.host_listener {
                        listener.handler_fn_name = Some(sanitize_identifier(
                            &format!("{}{}{}_HostBindingHandler", base_name, animation, event_name)
                        ));
                    } else {
                        let slot = listener.target_slot.slot.expect("Expected a slot to be assigned");
                        let tag = listener.tag.as_ref().map(|t| t.replace('-', "_")).unwrap_or_default();
                        listener.handler_fn_name = Some(sanitize_identifier(
                            &format!("{}_{}{}{}_{}_listener", base_name, tag, animation, event_name, slot)
                        ));
                    }
                }

                
                // Recurse into handler_ops
                let listener_ptr = op_ptr as *mut ListenerOp;
                let listener = &mut *listener_ptr;
                for handler_op in &mut listener.handler_ops {
                    process_op_with_var_names(&mut **handler_op, base_name, state, compatibility, tasks, var_names, false);
                }
            }
            ir::OpKind::TwoWayListener => {
                use crate::template::pipeline::ir::ops::create::TwoWayListenerOp;
                let two_way_ptr = op_ptr as *mut TwoWayListenerOp;
                let two_way = &mut *two_way_ptr;
                if two_way.handler_fn_name.is_none() {
                    let slot = two_way.target_slot.slot.expect("Expected a slot to be assigned");
                    let tag = two_way.tag.as_ref().map(|t| t.replace('-', "_")).unwrap_or_default();
                    two_way.handler_fn_name = Some(sanitize_identifier(
                        &format!("{}_{}_{}_{}_twoWayListener", base_name, tag, two_way.name, slot)
                    ));
                }
            }
            ir::OpKind::Variable => {
                // VariableOp - need to handle both CreateOp and UpdateOp versions
                // Use safe downcasting to handle potential mismatches (e.g. inside ListenerOp handlers)
                
                if let Some(var_op) = op.as_any_mut().downcast_mut::<VariableOp<Box<dyn ir::CreateOp + Send + Sync>>>() {
                    let name = get_variable_name(&var_op.variable, state, compatibility);
                    var_names.insert(var_op.xref, name.clone());
                    var_op.variable.set_name(Some(name));
                } else if let Some(var_op) = op.as_any_mut().downcast_mut::<VariableOp<Box<dyn UpdateOp + Send + Sync>>>() {
                    let name = get_variable_name(&var_op.variable, state, compatibility);
                    var_names.insert(var_op.xref, name.clone());
                    var_op.variable.set_name(Some(name));
                }
            }
            ir::OpKind::RepeaterCreate => {
                let rep_ptr = op_ptr as *mut RepeaterCreateOp;
                let rep = &mut *rep_ptr;
                if let Some(empty) = rep.empty_view {
                    let slot = rep.base.base.handle.slot.unwrap();
                    tasks.push(RecTask {
                        xref: empty,
                        name: format!("{}_{}Empty_{}", base_name, rep.function_name_suffix, slot + 2),
                    });
                }
                let slot = rep.base.base.handle.slot.unwrap();
                tasks.push(RecTask {
                    xref: rep.base.base.xref,
                    name: format!("{}_{}_{}", base_name, rep.function_name_suffix, slot + 1),
                });
            }
            ir::OpKind::Projection => {
                let proj_ptr = op_ptr as *mut ProjectionOp;
                let proj = &mut *proj_ptr;
                if let Some(fallback) = proj.fallback_view {
                    let slot = proj.handle.slot.unwrap();
                    tasks.push(RecTask {
                        xref: fallback,
                        name: format!("{}_ProjectionFallback_{}", base_name, slot),
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
                let slot = template.base.base.handle.slot.unwrap();
                tasks.push(RecTask {
                    xref: template.base.base.xref,
                    name: format!("{}{}_{}", base_name, suffix, slot),
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
                let slot = cond.base.base.handle.slot.unwrap();
                tasks.push(RecTask {
                    xref: cond.base.base.xref,
                    name: format!("{}{}_{}", base_name, suffix, slot),
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
                let slot = branch.base.base.handle.slot.unwrap();
                tasks.push(RecTask {
                    xref: branch.base.base.xref,
                    name: format!("{}{}_{}", base_name, suffix, slot),
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
                let compat_prefix = if ident_var.identifier == "ctx" { "i" } else { "" };
                let idx = state.index + 1; // Pre-increment in TS: ++state.index
                state.index += 1;
                format!("{}_{}r{}", ident_var.identifier, compat_prefix, idx)
           } else {
                let idx = state.index;
                state.index += 1;
                 format!("{}_i{}", ident_var.identifier, idx)
           }
        }
        _ => {
            // Fallback for other types
             let idx = state.index;
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
