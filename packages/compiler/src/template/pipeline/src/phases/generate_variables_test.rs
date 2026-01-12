
#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::pipeline::ir;
    use crate::template::pipeline::src::compilation::ComponentCompilationJob;
    use crate::template::pipeline::src::compilation::TemplateCompilationMode;
    use crate::constant_pool::ConstantPool;

    #[test]
    fn test_next_context_generation_nested() {
        // Setup Job
        let pool = ConstantPool::new();
        let mut job = ComponentCompilationJob::new(
            "TestComp".to_string(),
            pool,
            ir::CompatibilityMode::TemplateDefinitionBuilder,
            TemplateCompilationMode::Full,
            "test.ts".to_string(),
            false,
            Default::default(),
            None,
            None,
            false,
            None,
            vec![],
        );

        // View Hierarchy: Root (0) -> Child (1) -> Grandchild (2)
        let root_xref = job.root.xref;
        let child_xref = job.allocate_view(Some(root_xref));
        let grandchild_xref = job.allocate_view(Some(child_xref));

        // Add a reference in Grandchild pointing to Root
        // We simulate this by adding a resolved reference to the Grandchild scope
        // Note: We need to manually construct scopes or rely on phase to build them?
        // generate_variables *builds* the scopes based on view hierarchy and usage.
        // But it relies on `scope.references` being populated by `resolve_names`.
        // So we need to mock what `resolve_names` does.
        
        // However, `generate_variables` logic `getScopeForView` initializes a new Scope.
        // It populates it from `view.references`?? 
        // No, `resolve_names` populates `view.prop_reads` etc? 
        // Let's check `generate_variables` code again.
        // It iterates `scope.references`. 
        // Wait, `generate_variables` DOES NOT populate `scope.references`.
        // `resolve_names` DOES.
        
        // So I should populate `job.views.get(grandchild).references`?
        // Actually `generate_variables` reads `scope.references`. 
        // And `getScopeForView` transfers `view.references` to `scope.references`?
        // No. 
        
        // Let's assume `resolve_names` has run.
        // In the test, we need to populate the data structures that `generate_variables` consumes.
        // Inspecting `generate_variables.rs`: it uses `scope: Scope`.
        // `getScopeForView` creates Scope.
        // Where does it get references from?
        // It seems `generate_variables` *calculates* references? 
        // No, `generate_variables_in_scope_for_view` iterates `scope.references`.
        
        // I need to see `getScopeForView`.
        // (I will check file content)
    }
}
