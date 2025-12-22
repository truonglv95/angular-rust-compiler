// Compiler Host
//
// Abstraction for the compilation host environment.

use std::collections::HashMap;

/// Compilation host interface.
pub trait CompilerHost {
    /// Read a file.
    fn read_file(&self, file_name: &str) -> Option<String>;
    
    /// Write a file.
    fn write_file(&mut self, file_name: &str, content: &str);
    
    /// Check if file exists.
    fn file_exists(&self, file_name: &str) -> bool;
    
    /// Get directory for a file.
    fn get_directory_path(&self, file_name: &str) -> String;
    
    /// Get current directory.
    fn get_current_directory(&self) -> String;
    
    /// Resolve module name.
    fn resolve_module_name(&self, module_name: &str, containing_file: &str) -> Option<String>;
}

/// In-memory compiler host for testing.
pub struct InMemoryCompilerHost {
    files: HashMap<String, String>,
    current_dir: String,
}

impl InMemoryCompilerHost {
    pub fn new(current_dir: impl Into<String>) -> Self {
        Self {
            files: HashMap::new(),
            current_dir: current_dir.into(),
        }
    }
    
    /// Add a file.
    pub fn add_file(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }
}

impl CompilerHost for InMemoryCompilerHost {
    fn read_file(&self, file_name: &str) -> Option<String> {
        self.files.get(file_name).cloned()
    }
    
    fn write_file(&mut self, file_name: &str, content: &str) {
        self.files.insert(file_name.to_string(), content.to_string());
    }
    
    fn file_exists(&self, file_name: &str) -> bool {
        self.files.contains_key(file_name)
    }
    
    fn get_directory_path(&self, file_name: &str) -> String {
        if let Some(pos) = file_name.rfind('/') {
            file_name[..pos].to_string()
        } else {
            ".".to_string()
        }
    }
    
    fn get_current_directory(&self) -> String {
        self.current_dir.clone()
    }
    
    fn resolve_module_name(&self, module_name: &str, containing_file: &str) -> Option<String> {
        let dir = self.get_directory_path(containing_file);
        let resolved = if module_name.starts_with('.') {
            format!("{}/{}.ts", dir, module_name.trim_start_matches("./"))
        } else {
            format!("node_modules/{}/index.ts", module_name)
        };
        
        if self.file_exists(&resolved) {
            Some(resolved)
        } else {
            None
        }
    }
}
