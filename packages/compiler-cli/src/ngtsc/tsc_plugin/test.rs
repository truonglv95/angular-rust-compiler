// TSC Plugin Tests
//
// Tests for the TSC Plugin module.

#[cfg(test)]
mod tests {
    use crate::ngtsc::tsc_plugin::*;
    
    mod plugin_compiler_host_tests {
        use super::*;
        
        #[test]
        fn should_create_simple_host() {
            let host = SimplePluginCompilerHost::new(
                vec!["src/main.ts".to_string(), "src/app.ts".to_string()],
                "/project",
            );
            
            assert_eq!(host.input_files().len(), 2);
            assert_eq!(host.get_current_directory(), "/project");
        }
        
        #[test]
        fn should_get_canonical_file_name_case_sensitive() {
            let host = SimplePluginCompilerHost::new(vec![], "/project");
            assert_eq!(host.get_canonical_file_name("Foo.ts"), "Foo.ts");
        }
        
        #[test]
        fn should_convert_file_name_to_module_name() {
            let host = SimplePluginCompilerHost::new(vec![], "/project");
            assert_eq!(
                host.file_name_to_module_name("src/app.component.ts"),
                Some("src/app.component".to_string())
            );
        }
    }
    
    mod ng_tsc_plugin_tests {
        use super::*;
        
        #[test]
        fn should_create_plugin() {
            let plugin = NgTscPlugin::new(NgCompilerOptions::default());
            assert_eq!(plugin.name(), "ngtsc");
        }
        
        #[test]
        fn should_error_if_compiler_accessed_before_setup() {
            let plugin = NgTscPlugin::new(NgCompilerOptions::default());
            let result = plugin.compiler();
            assert!(result.is_err());
        }
        
        #[test]
        fn should_return_empty_diagnostics() {
            let plugin = NgTscPlugin::new(NgCompilerOptions::default());
            assert!(plugin.get_diagnostics(None).is_empty());
            assert!(plugin.get_option_diagnostics().is_empty());
        }
        
        #[test]
        fn should_create_default_transformers() {
            let plugin = NgTscPlugin::new(NgCompilerOptions::default());
            let transformers = plugin.create_transformers();
            assert!(transformers.before.is_empty());
            assert!(transformers.after.is_empty());
        }
    }
    
    mod compilation_mode_tests {
        use super::*;
        
        #[test]
        fn should_default_to_full_compilation() {
            let options = NgCompilerOptions::default();
            assert_eq!(options.compilation_mode, CompilationMode::Full);
        }
    }
}
