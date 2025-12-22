// Resource Loader API
//
// Resolves and loads resource files that are referenced in Angular metadata.

use std::future::Future;
use std::pin::Pin;

/// Type of component resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// Resources referenced via `styles` or `styleUrls`.
    Style,
    /// Resources referenced via `template` or `templateUrl`.
    Template,
}

/// Contextual information for resource loading.
#[derive(Debug, Clone)]
pub struct ResourceLoaderContext {
    /// The type of the component resource.
    pub resource_type: ResourceType,
    
    /// The absolute path to the file containing the resource reference.
    pub containing_file: String,
    
    /// For style resources, the order/position within the containing file.
    pub order: Option<u32>,
    
    /// The name of the class that defines the component using the resource.
    pub class_name: String,
}

impl ResourceLoaderContext {
    pub fn new_template(containing_file: impl Into<String>, class_name: impl Into<String>) -> Self {
        Self {
            resource_type: ResourceType::Template,
            containing_file: containing_file.into(),
            order: None,
            class_name: class_name.into(),
        }
    }
    
    pub fn new_style(
        containing_file: impl Into<String>,
        class_name: impl Into<String>,
        order: u32,
    ) -> Self {
        Self {
            resource_type: ResourceType::Style,
            containing_file: containing_file.into(),
            order: Some(order),
            class_name: class_name.into(),
        }
    }
}

/// Async preload future type.
pub type PreloadFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;

/// Async preprocess future type.
pub type PreprocessFuture = Pin<Box<dyn Future<Output = Result<String, String>> + Send>>;

/// Resolves and loads resource files referenced in Angular metadata.
pub trait ResourceLoader: Send + Sync {
    /// Whether this resource loader can preload resources.
    fn can_preload(&self) -> bool;
    
    /// Whether the resource loader can preprocess inline resources.
    fn can_preprocess(&self) -> bool;
    
    /// Resolve the URL of a resource relative to the file containing the reference.
    fn resolve(&self, file: &str, base_path: &str) -> Result<String, String>;
    
    /// Preload the specified resource asynchronously.
    fn preload(&self, resolved_url: &str, context: &ResourceLoaderContext) -> Option<PreloadFuture>;
    
    /// Preprocess the content of an inline resource asynchronously.
    fn preprocess_inline(&self, data: &str, context: &ResourceLoaderContext) -> PreprocessFuture;
    
    /// Load the resource at the given URL synchronously.
    fn load(&self, resolved_url: &str) -> Result<String, String>;
}

/// A no-op resource loader that doesn't support any operations.
#[derive(Debug, Default)]
pub struct NoopResourceLoader;

impl NoopResourceLoader {
    pub fn new() -> Self {
        Self
    }
}

impl ResourceLoader for NoopResourceLoader {
    fn can_preload(&self) -> bool {
        false
    }
    
    fn can_preprocess(&self) -> bool {
        false
    }
    
    fn resolve(&self, file: &str, _base_path: &str) -> Result<String, String> {
        Ok(file.to_string())
    }
    
    fn preload(&self, _resolved_url: &str, _context: &ResourceLoaderContext) -> Option<PreloadFuture> {
        None
    }
    
    fn preprocess_inline(&self, data: &str, _context: &ResourceLoaderContext) -> PreprocessFuture {
        let data = data.to_string();
        Box::pin(async move { Ok(data) })
    }
    
    fn load(&self, resolved_url: &str) -> Result<String, String> {
        Err(format!("Cannot load resource: {}", resolved_url))
    }
}
