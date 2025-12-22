// HMR Tests
//
// Tests for the hot module replacement module.

#[cfg(test)]
mod tests {
    use crate::ngtsc::hmr::*;
    
    mod hmr_metadata_tests {
        use super::*;
        
        #[test]
        fn should_create_metadata() {
            let metadata = HmrMetadata {
                component_name: "AppComponent".to_string(),
                component_id: "app-component-123".to_string(),
                template_url: Some("app.component.html".to_string()),
                style_urls: vec!["app.component.css".to_string()],
            };
            
            assert_eq!(metadata.component_name, "AppComponent");
            assert!(metadata.template_url.is_some());
        }
    }
    
    mod hmr_bootstrap_tests {
        use super::*;
        
        #[test]
        fn should_generate_bootstrap_code() {
            let code = generate_hmr_bootstrap_code("AppComponent", "app.module.ts");
            
            assert!(code.contains("AppComponent"));
            assert!(code.contains("module.hot"));
        }
        
        #[test]
        fn should_include_accept_handler() {
            let code = generate_hmr_bootstrap_code("TestComponent", "test.module.ts");
            
            assert!(code.contains("accept"));
        }
    }
    
    mod hmr_update_tests {
        use super::*;
        
        #[test]
        fn should_generate_update_code() {
            let metadata = HmrMetadata {
                component_name: "MyComponent".to_string(),
                component_id: "my-comp".to_string(),
                template_url: None,
                style_urls: vec![],
            };
            
            let code = generate_hmr_update_code(&metadata);
            
            assert!(code.contains("MyComponent"));
        }
    }
}
