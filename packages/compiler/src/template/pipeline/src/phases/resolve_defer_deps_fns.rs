//! Resolve Defer Dependency Functions Phase
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/resolve_defer_deps_fns.ts
//! Resolve the dependency function of a deferred block.

use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::ops::create::DeferOp;
use crate::template::pipeline::src::compilation::{CompilationUnit, ComponentCompilationJob};

/// Resolve the dependency function of a deferred block.
pub fn resolve_defer_deps_fns(job: &mut ComponentCompilationJob) {
    // Process root unit
    process_unit(&mut job.root, &mut job.pool);

    // Process all view units
    for (_, unit) in job.views.iter_mut() {
        process_unit(unit, &mut job.pool);
    }
}

fn process_unit(
    unit: &mut crate::template::pipeline::src::compilation::ViewCompilationUnit,
    pool: &mut crate::constant_pool::ConstantPool,
) {
    // Get full path name before borrowing mutable ops
    let full_path_name = unit
        .fn_name
        .as_ref()
        .map(|name| name.replace("_Template", ""))
        .unwrap_or_else(|| "Unknown".to_string());

    for op in unit.create_mut().iter_mut() {
        if op.kind() != OpKind::Defer {
            continue;
        }

        unsafe {
            let op_ptr = op.as_mut() as *mut dyn ir::CreateOp;
            let defer_ptr = op_ptr as *mut DeferOp;
            let defer = &mut *defer_ptr;

            // Skip if resolver_fn is already set
            if defer.resolver_fn.is_some() {
                continue;
            }

            // If own_resolver_fn is set, extract it to a shared function
            if let Some(own_resolver_fn) = &defer.own_resolver_fn {
                // Check that slot is assigned
                if defer.handle.get_slot().is_none() {
                    panic!("AssertionError: slot must be assigned before extracting defer deps functions");
                }

                // Generate function name
                let slot = defer.handle.get_slot().unwrap();
                let fn_name = format!("{}_Defer_{}_DepsFn", full_path_name, slot);

                // Get shared function reference
                let resolver_fn = pool.get_shared_function_reference(
                    own_resolver_fn.clone(),
                    fn_name,
                    false, // use_unique_name = false for TDB compatibility
                );

                defer.resolver_fn = Some(resolver_fn);
            }
        }
    }
}
