//! Resource Loader
//!
//! Corresponds to packages/compiler/src/resource_loader.ts
//! Loads external templates and styles

pub trait ResourceLoader {
    fn get(&self, url: &str) -> Result<String, String>;
}

pub struct DefaultResourceLoader;

impl ResourceLoader for DefaultResourceLoader {
    fn get(&self, _url: &str) -> Result<String, String> {
        Err("ResourceLoader not implemented in JIT mode".to_string())
    }
}
