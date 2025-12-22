// Debug Info
//
// Extract debug information for class declarations.

/// Debug information for a class.
#[derive(Debug, Clone)]
pub struct R3ClassDebugInfo {
    /// The class type expression.
    pub type_expr: String,
    /// The class name as a string literal.
    pub class_name: String,
    /// The relative file path, if available.
    pub file_path: Option<String>,
    /// The line number where the class is defined.
    pub line_number: u32,
    /// Whether orphan rendering is forbidden.
    pub forbid_orphan_rendering: bool,
}

impl R3ClassDebugInfo {
    pub fn new(class_name: impl Into<String>, line_number: u32) -> Self {
        Self {
            type_expr: String::new(),
            class_name: class_name.into(),
            file_path: None,
            line_number,
            forbid_orphan_rendering: false,
        }
    }
    
    pub fn with_file_path(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into());
        self
    }
    
    pub fn with_forbid_orphan_rendering(mut self, forbid: bool) -> Self {
        self.forbid_orphan_rendering = forbid;
        self
    }
}

/// Extract debug information from a class declaration.
pub fn extract_class_debug_info(
    class_name: &str,
    source_file: &str,
    line_number: u32,
    root_dirs: &[String],
    forbid_orphan_rendering: bool,
) -> Option<R3ClassDebugInfo> {
    let file_path = get_project_relative_path(source_file, root_dirs);
    
    Some(R3ClassDebugInfo {
        type_expr: class_name.to_string(),
        class_name: class_name.to_string(),
        file_path,
        line_number,
        forbid_orphan_rendering,
    })
}

/// Get a project-relative path for a source file.
fn get_project_relative_path(file_path: &str, root_dirs: &[String]) -> Option<String> {
    for root_dir in root_dirs {
        if file_path.starts_with(root_dir) {
            let relative = file_path.strip_prefix(root_dir)?;
            let relative = relative.trim_start_matches('/');
            return Some(relative.to_string());
        }
    }
    None
}
