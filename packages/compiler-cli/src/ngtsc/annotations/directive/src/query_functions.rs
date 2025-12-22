// Query Functions
//
// Handles parsing of signal-based query initializers like viewChild, contentChildren.

use super::initializer_functions::{InitializerApiFunction, OwningModule, InitializerFunctionName};

/// Query metadata from initializer.
#[derive(Debug, Clone)]
pub struct QueryFunctionMetadata {
    /// Property name.
    pub property_name: String,
    /// The selector (string or type reference).
    pub selector: String,
    /// Whether this is a required query.
    pub is_required: bool,
    /// Whether to get first result only.
    pub first: bool,
    /// Whether to query descendants.
    pub descendants: bool,
    /// Read token, if specified.
    pub read: Option<String>,
    /// Whether this is a signal-based query.
    pub is_signal: bool,
}

impl QueryFunctionMetadata {
    pub fn view_child(property_name: impl Into<String>, selector: impl Into<String>) -> Self {
        Self {
            property_name: property_name.into(),
            selector: selector.into(),
            is_required: false,
            first: true,
            descendants: true,
            read: None,
            is_signal: true,
        }
    }
    
    pub fn view_children(property_name: impl Into<String>, selector: impl Into<String>) -> Self {
        Self {
            property_name: property_name.into(),
            selector: selector.into(),
            is_required: false,
            first: false,
            descendants: true,
            read: None,
            is_signal: true,
        }
    }
    
    pub fn content_child(property_name: impl Into<String>, selector: impl Into<String>) -> Self {
        Self {
            property_name: property_name.into(),
            selector: selector.into(),
            is_required: false,
            first: true,
            descendants: false,
            read: None,
            is_signal: true,
        }
    }
    
    pub fn content_children(property_name: impl Into<String>, selector: impl Into<String>) -> Self {
        Self {
            property_name: property_name.into(),
            selector: selector.into(),
            is_required: false,
            first: false,
            descendants: true,
            read: None,
            is_signal: true,
        }
    }
    
    pub fn with_required(mut self, required: bool) -> Self {
        self.is_required = required;
        self
    }
    
    pub fn with_read(mut self, read: impl Into<String>) -> Self {
        self.read = Some(read.into());
        self
    }
}

/// All query initializer APIs.
pub fn query_initializer_apis() -> Vec<InitializerApiFunction> {
    vec![
        InitializerApiFunction::new(OwningModule::AngularCore, InitializerFunctionName::ViewChild),
        InitializerApiFunction::new(OwningModule::AngularCore, InitializerFunctionName::ViewChildren),
        InitializerApiFunction::new(OwningModule::AngularCore, InitializerFunctionName::ContentChild),
        InitializerApiFunction::new(OwningModule::AngularCore, InitializerFunctionName::ContentChildren),
    ]
}

/// Try to parse a query from an initializer call.
pub fn try_parse_signal_query(
    property_name: &str,
    function_name: InitializerFunctionName,
    selector: &str,
    is_required: bool,
    read: Option<&str>,
) -> Option<QueryFunctionMetadata> {
    let mut query = match function_name {
        InitializerFunctionName::ViewChild => QueryFunctionMetadata::view_child(property_name, selector),
        InitializerFunctionName::ViewChildren => QueryFunctionMetadata::view_children(property_name, selector),
        InitializerFunctionName::ContentChild => QueryFunctionMetadata::content_child(property_name, selector),
        InitializerFunctionName::ContentChildren => QueryFunctionMetadata::content_children(property_name, selector),
        _ => return None,
    };
    
    query.is_required = is_required;
    query.read = read.map(|s| s.to_string());
    
    Some(query)
}
