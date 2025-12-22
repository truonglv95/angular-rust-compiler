// Scope Tests
//
// Tests for the scope module.

#[cfg(test)]
mod tests {
    use super::src::*;
    
    mod local_scope_registry_tests {
        use super::*;
        
        #[test]
        fn should_create_empty_registry() {
            let registry = LocalModuleScopeRegistry::new();
            assert!(registry.get_scope_of_module("NonExistent").is_none());
        }
        
        #[test]
        fn should_register_module() {
            let mut registry = LocalModuleScopeRegistry::new();
            registry.register_module("TestModule".to_string(), Some("test-selector".to_string()));
            
            // After registration, module should be known
            assert!(registry.get_module_declarations("TestModule").is_some());
        }
        
        #[test]
        fn should_add_declaration_to_module() {
            let mut registry = LocalModuleScopeRegistry::new();
            registry.register_module("TestModule".to_string(), None);
            registry.add_declaration("TestModule", "TestComponent");
            
            let declarations = registry.get_module_declarations("TestModule").unwrap();
            assert!(declarations.contains(&"TestComponent".to_string()));
        }
        
        #[test]
        fn should_add_export_to_module() {
            let mut registry = LocalModuleScopeRegistry::new();
            registry.register_module("TestModule".to_string(), None);
            registry.add_export("TestModule", "ExportedComponent");
            
            let exports = registry.get_module_exports("TestModule").unwrap();
            assert!(exports.contains(&"ExportedComponent".to_string()));
        }
        
        #[test]
        fn should_add_import_to_module() {
            let mut registry = LocalModuleScopeRegistry::new();
            registry.register_module("TestModule".to_string(), None);
            registry.add_import("TestModule", "ImportedModule");
            
            let imports = registry.get_module_imports("TestModule").unwrap();
            assert!(imports.contains(&"ImportedModule".to_string()));
        }
    }
    
    mod standalone_scope_tests {
        use super::*;
        
        #[test]
        fn should_create_standalone_scope() {
            let registry = StandaloneComponentScopeRegistry::new();
            assert!(registry.get_scope("NonExistent").is_none());
        }
        
        #[test]
        fn should_register_standalone_component() {
            let mut registry = StandaloneComponentScopeRegistry::new();
            registry.register_component("StandaloneComp".to_string(), vec![], vec![]);
            
            assert!(registry.get_scope("StandaloneComp").is_some());
        }
        
        #[test]
        fn should_track_imports_for_standalone() {
            let mut registry = StandaloneComponentScopeRegistry::new();
            registry.register_component(
                "StandaloneComp".to_string(),
                vec!["FormsModule".to_string()],
                vec![],
            );
            
            let scope = registry.get_scope("StandaloneComp").unwrap();
            assert!(scope.imports.contains(&"FormsModule".to_string()));
        }
    }
}
