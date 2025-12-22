// Mock File Loading
//
// Mock file system for testing.

use std::collections::HashMap;

/// Mock file system.
pub struct MockFileSystem {
    /// Files in the mock file system.
    files: HashMap<String, Vec<u8>>,
    /// Directories.
    directories: std::collections::HashSet<String>,
}

impl MockFileSystem {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            directories: std::collections::HashSet::new(),
        }
    }
    
    /// Add a text file.
    pub fn add_file(&mut self, path: &str, content: &str) {
        self.files.insert(path.to_string(), content.as_bytes().to_vec());
        // Add parent directories
        if let Some(parent) = get_parent(path) {
            self.add_directory(parent);
        }
    }
    
    /// Add a binary file.
    pub fn add_binary_file(&mut self, path: &str, content: Vec<u8>) {
        self.files.insert(path.to_string(), content);
        if let Some(parent) = get_parent(path) {
            self.add_directory(parent);
        }
    }
    
    /// Add a directory.
    pub fn add_directory(&mut self, path: &str) {
        let mut current = String::new();
        for part in path.split('/').filter(|s| !s.is_empty()) {
            current.push('/');
            current.push_str(part);
            self.directories.insert(current.clone());
        }
    }
    
    /// Read a file as string.
    pub fn read_file(&self, path: &str) -> Option<String> {
        self.files.get(path).and_then(|bytes| {
            String::from_utf8(bytes.clone()).ok()
        })
    }
    
    /// Read a file as bytes.
    pub fn read_binary_file(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(|v| v.as_slice())
    }
    
    /// Check if file exists.
    pub fn file_exists(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }
    
    /// Check if directory exists.
    pub fn directory_exists(&self, path: &str) -> bool {
        self.directories.contains(path)
    }
    
    /// List directory contents.
    pub fn list_directory(&self, path: &str) -> Vec<String> {
        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{}/", path)
        };
        
        let mut results = Vec::new();
        
        for file_path in self.files.keys() {
            if file_path.starts_with(&prefix) {
                let rest = &file_path[prefix.len()..];
                if !rest.contains('/') {
                    results.push(rest.to_string());
                }
            }
        }
        
        for dir_path in &self.directories {
            if dir_path.starts_with(&prefix) {
                let rest = &dir_path[prefix.len()..];
                if !rest.contains('/') && !results.contains(&rest.to_string()) {
                    results.push(rest.to_string());
                }
            }
        }
        
        results
    }
}

impl Default for MockFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Get parent directory path.
fn get_parent(path: &str) -> Option<&str> {
    path.rfind('/').map(|pos| &path[..pos])
}
