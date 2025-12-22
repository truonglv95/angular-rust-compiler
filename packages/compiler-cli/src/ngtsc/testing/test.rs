// Testing Module Tests
//
// Tests for the testing utilities module.

#[cfg(test)]
mod tests {
    use super::src::*;
    
    mod test_compiler_host_tests {
        use super::*;
        
        #[test]
        fn should_create_empty_host() {
            let host = TestCompilerHost::new();
            assert!(!host.exists("anything"));
        }
        
        #[test]
        fn should_write_and_read_file() {
            let mut host = TestCompilerHost::new();
            host.write("test.ts", "const x = 1;");
            
            assert!(host.exists("test.ts"));
            assert_eq!(host.read("test.ts"), Some("const x = 1;"));
        }
        
        #[test]
        fn should_return_none_for_missing() {
            let host = TestCompilerHost::new();
            assert!(host.read("missing.ts").is_none());
        }
        
        #[test]
        fn should_list_files() {
            let mut host = TestCompilerHost::new();
            host.write("a.ts", "a");
            host.write("b.ts", "b");
            
            let files = host.get_files();
            assert_eq!(files.len(), 2);
        }
        
        #[test]
        fn should_clear_files() {
            let mut host = TestCompilerHost::new();
            host.write("test.ts", "content");
            host.clear();
            
            assert!(!host.exists("test.ts"));
        }
    }
    
    mod test_environment_tests {
        use super::*;
        
        #[test]
        fn should_create_environment() {
            let env = TestEnvironment::default();
            assert!(env.get_files().is_empty());
        }
        
        #[test]
        fn should_add_file() {
            let mut env = TestEnvironment::default();
            env.add_file("test.ts", "content");
            
            assert!(env.get_files().contains_key("test.ts"));
        }
        
        #[test]
        fn should_add_component() {
            let mut env = TestEnvironment::default();
            env.add_component("Test", "<p>Hello</p>", "p { color: red; }");
            
            assert!(env.get_files().contains_key("test.component.ts"));
        }
    }
    
    mod mock_file_system_tests {
        use super::*;
        
        #[test]
        fn should_create_empty_fs() {
            let fs = MockFileSystem::new();
            assert!(!fs.file_exists("anything"));
        }
        
        #[test]
        fn should_add_and_read_file() {
            let mut fs = MockFileSystem::new();
            fs.add_file("/app/test.ts", "export const x = 1;");
            
            assert!(fs.file_exists("/app/test.ts"));
            assert_eq!(
                fs.read_file("/app/test.ts"),
                Some("export const x = 1;".to_string())
            );
        }
        
        #[test]
        fn should_create_parent_directories() {
            let mut fs = MockFileSystem::new();
            fs.add_file("/a/b/c/file.ts", "content");
            
            assert!(fs.directory_exists("/a"));
            assert!(fs.directory_exists("/a/b"));
            assert!(fs.directory_exists("/a/b/c"));
        }
        
        #[test]
        fn should_list_directory() {
            let mut fs = MockFileSystem::new();
            fs.add_file("/app/a.ts", "a");
            fs.add_file("/app/b.ts", "b");
            fs.add_file("/app/sub/c.ts", "c");
            
            let contents = fs.list_directory("/app");
            assert!(contents.contains(&"a.ts".to_string()));
            assert!(contents.contains(&"b.ts".to_string()));
        }
    }
    
    mod utility_tests {
        use super::*;
        
        #[test]
        fn should_expect_diagnostics() {
            let diagnostics = vec![
                "Error: Property 'foo' does not exist".to_string(),
                "Error: Cannot find 'bar'".to_string(),
            ];
            
            assert!(expect_diagnostics(&diagnostics, &["foo", "bar"]));
            assert!(!expect_diagnostics(&diagnostics, &["baz"]));
        }
        
        #[test]
        fn should_make_component_source() {
            let source = make_component_source("Test", "<p>Hello</p>");
            
            assert!(source.contains("@Component"));
            assert!(source.contains("selector: 'test'"));
            assert!(source.contains("Hello"));
        }
        
        #[test]
        fn should_make_ng_module_source() {
            let source = make_ng_module_source("AppModule", &["AppComponent", "HeaderComponent"]);
            
            assert!(source.contains("@NgModule"));
            assert!(source.contains("AppComponent"));
            assert!(source.contains("HeaderComponent"));
        }
    }
}
