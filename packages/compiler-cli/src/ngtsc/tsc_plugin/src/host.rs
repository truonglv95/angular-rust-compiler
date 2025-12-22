// Plugin Compiler Host
//
// Corresponds to the PluginCompilerHost interface in tsc_plugin.ts


/// A CompilerHost which also returns a list of input files.
/// Mirrors the plugin interface from @bazel/concatjs/internal/tsc_wrapped/plugin_api.
pub trait PluginCompilerHost {
    /// List of input files for the program.
    fn input_files(&self) -> &[String];
    
    /// Returns the file name to module name mapping.
    fn file_name_to_module_name(&self, file_name: &str) -> Option<String>;
    
    /// Checks if a file exists.
    fn file_exists(&self, file_name: &str) -> bool;
    
    /// Reads a file.
    fn read_file(&self, file_name: &str) -> Option<String>;
    
    /// Gets the source file for the given path.
    fn get_source_file(&self, file_name: &str) -> Option<String>;
    
    /// Gets the default lib file name.
    fn get_default_lib_file_name(&self) -> String;
    
    /// Gets the current directory.
    fn get_current_directory(&self) -> String;
    
    /// Gets the canonical file name.
    fn get_canonical_file_name(&self, file_name: &str) -> String;
    
    /// Checks if file names are case-sensitive.
    fn use_case_sensitive_file_names(&self) -> bool;
    
    /// Gets the new line character.
    fn get_new_line(&self) -> String;
}

/// Simple implementation of PluginCompilerHost
pub struct SimplePluginCompilerHost {
    input_files: Vec<String>,
    current_directory: String,
    case_sensitive: bool,
}

impl SimplePluginCompilerHost {
    pub fn new(input_files: Vec<String>, current_directory: &str) -> Self {
        Self {
            input_files,
            current_directory: current_directory.to_string(),
            case_sensitive: true,
        }
    }
}

impl PluginCompilerHost for SimplePluginCompilerHost {
    fn input_files(&self) -> &[String] {
        &self.input_files
    }
    
    fn file_name_to_module_name(&self, file_name: &str) -> Option<String> {
        // Simple implementation: strip extension
        if file_name.ends_with(".ts") {
            Some(file_name[..file_name.len() - 3].to_string())
        } else {
            Some(file_name.to_string())
        }
    }
    
    fn file_exists(&self, file_name: &str) -> bool {
        std::path::Path::new(file_name).exists()
    }
    
    fn read_file(&self, file_name: &str) -> Option<String> {
        std::fs::read_to_string(file_name).ok()
    }
    
    fn get_source_file(&self, file_name: &str) -> Option<String> {
        self.read_file(file_name)
    }
    
    fn get_default_lib_file_name(&self) -> String {
        "lib.d.ts".to_string()
    }
    
    fn get_current_directory(&self) -> String {
        self.current_directory.clone()
    }
    
    fn get_canonical_file_name(&self, file_name: &str) -> String {
        if self.case_sensitive {
            file_name.to_string()
        } else {
            file_name.to_lowercase()
        }
    }
    
    fn use_case_sensitive_file_names(&self) -> bool {
        self.case_sensitive
    }
    
    fn get_new_line(&self) -> String {
        "\n".to_string()
    }
}
