// HMR Metadata
//
// Metadata for hot module replacement.

/// HMR metadata.
#[derive(Debug, Clone)]
pub struct HmrMetadata {
    pub component_name: String,
    pub component_id: String,
    pub template_url: Option<String>,
    pub style_urls: Vec<String>,
}

/// Generate HMR bootstrap code.
pub fn generate_hmr_bootstrap_code(component: &str, module: &str) -> String {
    format!(
        r#"
if (module.hot) {{
    module.hot.accept('{}', function() {{
        // Re-render {}
        console.log('HMR: reloading {}');
    }});
}}
"#,
        module, component, component
    )
}

/// Generate HMR update code.
pub fn generate_hmr_update_code(metadata: &HmrMetadata) -> String {
    format!(
        "// HMR update for {}\nwindow.__ng_hmr_update__('{}');",
        metadata.component_name, metadata.component_id
    )
}
