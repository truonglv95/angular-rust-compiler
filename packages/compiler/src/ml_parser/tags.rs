//! ML Parser Tags
//!
//! Corresponds to packages/compiler/src/ml_parser/tags.ts (69 lines)

/// Tag content types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagContentType {
    RawText,
    EscapableRawText,
    ParsableData,
}

/// Tag definition interface
pub trait TagDefinition {
    fn closed_by_parent(&self) -> bool;
    fn implicit_namespace_prefix(&self) -> Option<&str>;
    fn is_void(&self) -> bool;
    fn ignore_first_lf(&self) -> bool;
    fn can_self_close(&self) -> bool;
    fn prevent_namespace_inheritance(&self) -> bool;
    
    fn is_closed_by_child(&self, name: &str) -> bool;
    fn get_content_type(&self, prefix: Option<&str>) -> TagContentType;
}

/// Split namespace and name from element name
/// 
/// Format: `:namespace:name`
/// Returns: (namespace, name) or (None, name)
pub fn split_ns_name(element_name: &str, fatal: bool) -> Result<(Option<String>, String), String> {
    if !element_name.starts_with(':') {
        return Ok((None, element_name.to_string()));
    }
    
    let colon_index = element_name[1..].find(':');
    
    match colon_index {
        None => {
            if fatal {
                Err(format!("Unsupported format \"{}\" expecting \":namespace:name\"", element_name))
            } else {
                Ok((None, element_name.to_string()))
            }
        }
        Some(idx) => {
            let actual_idx = idx + 1; // Adjust for the slice starting at index 1
            let namespace = element_name[1..=idx].to_string();
            let name = element_name[actual_idx + 1..].to_string();
            Ok((Some(namespace), name))
        }
    }
}

/// Check if tag is `<ng-container>` (works same regardless of namespace)
pub fn is_ng_container(tag_name: &str) -> bool {
    split_ns_name(tag_name, false)
        .map(|(_, name)| name == "ng-container")
        .unwrap_or(false)
}

/// Check if tag is `<ng-content>` (works same regardless of namespace)
pub fn is_ng_content(tag_name: &str) -> bool {
    split_ns_name(tag_name, false)
        .map(|(_, name)| name == "ng-content")
        .unwrap_or(false)
}

/// Check if tag is `<ng-template>` (works same regardless of namespace)
pub fn is_ng_template(tag_name: &str) -> bool {
    split_ns_name(tag_name, false)
        .map(|(_, name)| name == "ng-template")
        .unwrap_or(false)
}

/// Get namespace prefix from full name
pub fn get_ns_prefix(full_name: Option<&str>) -> Option<String> {
    full_name.and_then(|name| {
        split_ns_name(name, false)
            .ok()
            .and_then(|(prefix, _)| prefix)
    })
}

/// Merge namespace prefix and local name
pub fn merge_ns_and_name(prefix: Option<&str>, local_name: &str) -> String {
    match prefix {
        Some(p) if !p.is_empty() => format!(":{}:{}", p, local_name),
        _ => local_name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_ns_name_no_namespace() {
        let result = split_ns_name("div", false).unwrap();
        assert_eq!(result, (None, "div".to_string()));
    }

    #[test]
    fn test_split_ns_name_with_namespace() {
        let result = split_ns_name(":svg:circle", false).unwrap();
        assert_eq!(result, (Some("svg".to_string()), "circle".to_string()));
    }

    #[test]
    fn test_split_ns_name_invalid_fatal() {
        let result = split_ns_name(":invalid", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_split_ns_name_invalid_non_fatal() {
        let result = split_ns_name(":invalid", false).unwrap();
        assert_eq!(result, (None, ":invalid".to_string()));
    }

    #[test]
    fn test_is_ng_container() {
        assert!(is_ng_container("ng-container"));
        assert!(is_ng_container(":svg:ng-container"));
        assert!(!is_ng_container("div"));
    }

    #[test]
    fn test_is_ng_content() {
        assert!(is_ng_content("ng-content"));
        assert!(!is_ng_content("div"));
    }

    #[test]
    fn test_is_ng_template() {
        assert!(is_ng_template("ng-template"));
        assert!(!is_ng_template("div"));
    }

    #[test]
    fn test_get_ns_prefix() {
        assert_eq!(get_ns_prefix(Some(":svg:circle")), Some("svg".to_string()));
        assert_eq!(get_ns_prefix(Some("div")), None);
        assert_eq!(get_ns_prefix(None), None);
    }

    #[test]
    fn test_merge_ns_and_name() {
        assert_eq!(merge_ns_and_name(Some("svg"), "circle"), ":svg:circle");
        assert_eq!(merge_ns_and_name(None, "div"), "div");
        assert_eq!(merge_ns_and_name(Some(""), "div"), "div");
    }
}

