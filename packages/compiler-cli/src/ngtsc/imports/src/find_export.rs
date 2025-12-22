// Find Export - Utilities for finding exported names of nodes
//
// Find the name, if any, by which a node is exported from a given file.

use std::collections::HashMap;

/// Information about an export from a module.
#[derive(Debug, Clone)]
pub struct ExportInfo {
    /// The exported name.
    pub name: String,
    /// The local name in the source file.
    pub local_name: String,
    /// Whether this is a re-export from another module.
    pub is_reexport: bool,
}

/// Result of finding an exported name.
pub type ExportMap = HashMap<String, ExportInfo>;

/// Find the name by which a node is exported from a given file.
///
/// # Arguments
/// * `target_name` - The local name of the node to find
/// * `exports` - Map of export names to export info
///
/// # Returns
/// The export name if found, prioritizing non-alias exports.
pub fn find_exported_name_of_node(
    target_name: &str,
    exports: &ExportMap,
) -> Option<String> {
    let mut found_export_name: Option<String> = None;
    
    for (export_name, info) in exports {
        if info.local_name != target_name {
            continue;
        }
        
        // A non-alias export (where export name matches local name) is always preferred
        if export_name == target_name {
            return Some(export_name.clone());
        }
        
        found_export_name = Some(export_name.clone());
    }
    
    found_export_name
}

/// Check if a declaration is a named declaration (has a name property).
pub fn is_named_declaration(name: Option<&str>) -> bool {
    name.is_some()
}
