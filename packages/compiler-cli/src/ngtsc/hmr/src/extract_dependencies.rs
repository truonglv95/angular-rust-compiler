// Extract Dependencies
//
// Extract component dependencies for HMR.

/// Extracted HMR dependencies.
#[derive(Debug, Clone)]
pub struct HmrDependencies {
    pub template_dependencies: Vec<String>,
    pub style_dependencies: Vec<String>,
    pub component_dependencies: Vec<String>,
}

/// Extract dependencies for HMR.
pub fn extract_dependencies(_source: &str) -> HmrDependencies {
    HmrDependencies {
        template_dependencies: Vec::new(),
        style_dependencies: Vec::new(),
        component_dependencies: Vec::new(),
    }
}
