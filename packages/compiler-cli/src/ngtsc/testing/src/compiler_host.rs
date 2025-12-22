// Test Compiler Host
//
// Compiler host implementation for testing.

use std::collections::HashMap;

/// Test compiler host with in-memory file system.
pub struct TestCompilerHost {
    /// Files in memory.
    files: HashMap<String, String>,
    /// Current directory.
    cwd: String,
}

impl TestCompilerHost {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            cwd: "/".to_string(),
        }
    }
    
    /// Set a file's content.
    pub fn write(&mut self, path: &str, content: &str) {
        self.files.insert(path.to_string(), content.to_string());
    }
    
    /// Get a file's content.
    pub fn read(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(|s| s.as_str())
    }
    
    /// Check if file exists.
    pub fn exists(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }
    
    /// Set current working directory.
    pub fn set_cwd(&mut self, cwd: &str) {
        self.cwd = cwd.to_string();
    }
    
    /// Get all file paths.
    pub fn get_files(&self) -> Vec<&str> {
        self.files.keys().map(|s| s.as_str()).collect()
    }
    
    /// Clear all files.
    pub fn clear(&mut self) {
        self.files.clear();
    }
}

impl Default for TestCompilerHost {
    fn default() -> Self {
        Self::new()
    }
}
