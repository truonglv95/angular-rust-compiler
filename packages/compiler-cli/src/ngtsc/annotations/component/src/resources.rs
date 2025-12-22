// Component Resources
//
// Utilities for extracting and parsing component templates and styles.

use angular_compiler::parse_util::ParseSourceFile;

/// Style URL metadata from decorator.
#[derive(Debug, Clone)]
pub struct StyleUrlMeta {
    /// The URL to the style file.
    pub url: String,
    /// Source type.
    pub source: ResourceTypeForDiagnostics,
}

/// Type of resource for diagnostic purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceTypeForDiagnostics {
    /// Style from template (e.g., :host {}).
    StylesheetFromTemplate,
    /// Style from decorator styleUrls.
    StylesheetFromDecorator,
    /// Template from templateUrl.
    Template,
}

/// Parsed component template.
#[derive(Debug, Clone)]
pub struct ParsedComponentTemplate {
    /// Template nodes for diagnostics.
    pub diag_nodes: Vec<String>,
    /// Source file.
    pub file: Option<ParseSourceFile>,
    /// Interpolation config used.
    pub interpolation_start: String,
    pub interpolation_end: String,
    /// Whether whitespace is preserved.
    pub preserve_whitespaces: bool,
    /// i18n options.
    pub is_inline: bool,
    /// Template content.
    pub template: String,
    /// Errors during parsing.
    pub errors: Vec<String>,
}

impl ParsedComponentTemplate {
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            diag_nodes: Vec::new(),
            file: None,
            interpolation_start: "{{".to_string(),
            interpolation_end: "}}".to_string(),
            preserve_whitespaces: false,
            is_inline: true,
            template: template.into(),
            errors: Vec::new(),
        }
    }
    
    pub fn with_file(mut self, file: ParseSourceFile) -> Self {
        self.file = Some(file);
        self
    }
    
    pub fn with_errors(mut self, errors: Vec<String>) -> Self {
        self.errors = errors;
        self
    }
}

/// Parsed template with source mapping info.
#[derive(Debug, Clone)]
pub struct ParsedTemplateWithSource {
    /// Template content.
    pub content: String,
    /// Source mapping info.
    pub source_mapping: SourceMapping,
    /// Declaration info.
    pub declaration: TemplateDeclaration,
    /// Parsed template.
    pub template: ParsedComponentTemplate,
}

/// Source mapping for template.
#[derive(Debug, Clone)]
pub enum SourceMapping {
    Direct {
        node: String,
    },
    Indirect {
        component_class: String,
        template_url: String,
    },
    External {
        component_class: String,
        template_url: String,
    },
}

/// Template declaration info.
#[derive(Debug, Clone)]
pub struct TemplateDeclaration {
    /// Whether the template is inline.
    pub is_inline: bool,
    /// Preserve whitespace setting.
    pub preserve_whitespaces: bool,
    /// Template URL (for external templates).
    pub template_url: String,
    /// Resolved template URL.
    pub resolved_template_url: String,
}

impl TemplateDeclaration {
    pub fn inline(content: impl Into<String>) -> Self {
        Self {
            is_inline: true,
            preserve_whitespaces: false,
            template_url: String::new(),
            resolved_template_url: String::new(),
        }
    }
    
    pub fn external(template_url: impl Into<String>, resolved: impl Into<String>) -> Self {
        Self {
            is_inline: false,
            preserve_whitespaces: false,
            template_url: template_url.into(),
            resolved_template_url: resolved.into(),
        }
    }
}

/// Template extraction options.
#[derive(Debug, Clone, Default)]
pub struct ExtractTemplateOptions {
    pub use_poisoned_data: bool,
    pub enable_i18n_legacy_message_id_format: bool,
    pub i18n_normalize_line_endings_in_icus: bool,
    pub enable_block_syntax: bool,
    pub enable_let_syntax: bool,
    pub enable_selectorless: bool,
    pub preserve_significant_whitespace: Option<bool>,
}

/// Extract template from declaration.
pub fn extract_template(
    class_name: &str,
    declaration: &TemplateDeclaration,
    template_content: &str,
    options: &ExtractTemplateOptions,
) -> ParsedTemplateWithSource {
    let parsed = ParsedComponentTemplate::new(template_content);
    
    ParsedTemplateWithSource {
        content: template_content.to_string(),
        source_mapping: if declaration.is_inline {
            SourceMapping::Direct {
                node: class_name.to_string(),
            }
        } else {
            SourceMapping::External {
                component_class: class_name.to_string(),
                template_url: declaration.template_url.clone(),
            }
        },
        declaration: declaration.clone(),
        template: parsed,
    }
}

/// Parse template declaration from component metadata.
pub fn parse_template_declaration(
    template: Option<&str>,
    template_url: Option<&str>,
    preserve_whitespaces: bool,
) -> TemplateDeclaration {
    if let Some(url) = template_url {
        TemplateDeclaration::external(url, url)
    } else {
        TemplateDeclaration::inline(template.unwrap_or(""))
    }
}
