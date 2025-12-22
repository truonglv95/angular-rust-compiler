// Transformer Utilities
//
// Helper functions for transformers.

/// Normalize a path for consistent comparison.
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect::<Vec<_>>()
        .join("/")
}

/// Get relative path from one file to another.
pub fn get_relative_path(from: &str, to: &str) -> String {
    let from_parts: Vec<&str> = from.split('/').collect();
    let to_parts: Vec<&str> = to.split('/').collect();
    
    // Find common prefix
    let mut common_len = 0;
    for (i, (a, b)) in from_parts.iter().zip(to_parts.iter()).enumerate() {
        if a == b {
            common_len = i + 1;
        } else {
            break;
        }
    }
    
    // Build relative path
    let ups = from_parts.len() - common_len - 1;
    let mut result = vec![".."; ups];
    result.extend(&to_parts[common_len..]);
    
    if result.is_empty() {
        ".".to_string()
    } else {
        result.join("/")
    }
}

/// Check if a path is relative.
pub fn is_relative_path(path: &str) -> bool {
    path.starts_with('.') || (!path.starts_with('/') && !path.contains("://"))
}

/// Get file extension.
pub fn get_extension(path: &str) -> Option<&str> {
    path.rsplit_once('.').map(|(_, ext)| ext)
}

/// Replace file extension.
pub fn replace_extension(path: &str, new_ext: &str) -> String {
    if let Some((base, _)) = path.rsplit_once('.') {
        format!("{}.{}", base, new_ext)
    } else {
        format!("{}.{}", path, new_ext)
    }
}

/// Check if file is a TypeScript source file.
pub fn is_ts_file(path: &str) -> bool {
    matches!(get_extension(path), Some("ts") | Some("tsx"))
}

/// Check if file is a declaration file.
pub fn is_dts_file(path: &str) -> bool {
    path.ends_with(".d.ts")
}
