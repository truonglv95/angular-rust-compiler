// Alias Transform - Export statement aliasing
//
// This module provides the alias transform factory for adding export statements
// with aliases to source files.

use std::collections::HashMap;

/// Type alias for export statements map.
/// Outer key: source file path
/// Inner key: alias name
/// Value: (module name, symbol name)
pub type ExportStatementsMap = HashMap<String, HashMap<String, (String, String)>>;

/// Configuration for the alias transform.
pub struct AliasTransformConfig {
    /// Map of export statements to add per source file.
    pub export_statements: ExportStatementsMap,
}

impl AliasTransformConfig {
    /// Create a new alias transform config.
    pub fn new() -> Self {
        Self {
            export_statements: HashMap::new(),
        }
    }

    /// Add an export statement for a source file.
    ///
    /// # Arguments
    /// * `file_name` - The source file to add the export to
    /// * `alias_name` - The alias name for the export
    /// * `module_name` - The module to import from
    /// * `symbol_name` - The original symbol name to export
    pub fn add_export(
        &mut self,
        file_name: impl Into<String>,
        alias_name: impl Into<String>,
        module_name: impl Into<String>,
        symbol_name: impl Into<String>,
    ) {
        self.export_statements
            .entry(file_name.into())
            .or_default()
            .insert(alias_name.into(), (module_name.into(), symbol_name.into()));
    }

    /// Check if the config has any export statements.
    pub fn has_exports(&self) -> bool {
        !self.export_statements.is_empty()
    }

    /// Get export statements for a specific file.
    pub fn get_exports_for_file(&self, file_name: &str) -> Option<&HashMap<String, (String, String)>> {
        self.export_statements.get(file_name)
    }
}

impl Default for AliasTransformConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents an export to add to a source file.
#[derive(Debug, Clone)]
pub struct ExportAlias {
    /// The alias name for the exported symbol.
    pub alias_name: String,
    /// The original symbol name being exported.
    pub symbol_name: String,
    /// The module to import from.
    pub module_name: String,
}

impl ExportAlias {
    pub fn new(
        alias_name: impl Into<String>,
        symbol_name: impl Into<String>,
        module_name: impl Into<String>,
    ) -> Self {
        Self {
            alias_name: alias_name.into(),
            symbol_name: symbol_name.into(),
            module_name: module_name.into(),
        }
    }
}

/// Apply alias transforms to source file statements.
///
/// This would typically be called during the transformation phase to add
/// re-export statements to source files.
pub fn apply_alias_exports(
    config: &AliasTransformConfig,
    file_name: &str,
) -> Vec<ExportAlias> {
    config
        .get_exports_for_file(file_name)
        .map(|exports| {
            exports
                .iter()
                .map(|(alias_name, (module_name, symbol_name))| {
                    ExportAlias::new(alias_name, symbol_name, module_name)
                })
                .collect()
        })
        .unwrap_or_default()
}
