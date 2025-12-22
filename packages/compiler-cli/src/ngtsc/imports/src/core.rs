// Core - Import rewriting utilities
//
// Provides ImportRewriter trait and implementations for rewriting imports.

use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Rewrites imports of symbols being written into generated code.
pub trait ImportRewriter {
    /// Optionally rewrite a reference to an imported symbol.
    fn rewrite_symbol(&self, symbol: &str, specifier: &str) -> String;
    
    /// Optionally rewrite the given module specifier in the context of a given file.
    fn rewrite_specifier(&self, specifier: &str, in_context_of_file: &str) -> String;
    
    /// Optionally rewrite the identifier of a namespace import.
    fn rewrite_namespace_import_identifier(&self, specifier: &str, module_name: &str) -> String;
}

/// `ImportRewriter` that does no rewriting.
#[derive(Debug, Clone, Default)]
pub struct NoopImportRewriter;

impl NoopImportRewriter {
    pub fn new() -> Self {
        Self
    }
}

impl ImportRewriter for NoopImportRewriter {
    fn rewrite_symbol(&self, symbol: &str, _specifier: &str) -> String {
        symbol.to_string()
    }
    
    fn rewrite_specifier(&self, specifier: &str, _in_context_of_file: &str) -> String {
        specifier.to_string()
    }
    
    fn rewrite_namespace_import_identifier(&self, specifier: &str, _module_name: &str) -> String {
        specifier.to_string()
    }
}

/// A mapping of supported symbols that can be imported from within @angular/core,
/// and the names by which they're exported from r3_symbols.
static CORE_SUPPORTED_SYMBOLS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("ɵɵdefineInjectable", "ɵɵdefineInjectable");
    m.insert("ɵɵdefineInjector", "ɵɵdefineInjector");
    m.insert("ɵɵdefineNgModule", "ɵɵdefineNgModule");
    m.insert("ɵɵsetNgModuleScope", "ɵɵsetNgModuleScope");
    m.insert("ɵɵinject", "ɵɵinject");
    m.insert("ɵɵFactoryDeclaration", "ɵɵFactoryDeclaration");
    m.insert("ɵsetClassMetadata", "setClassMetadata");
    m.insert("ɵsetClassMetadataAsync", "setClassMetadataAsync");
    m.insert("ɵɵInjectableDeclaration", "ɵɵInjectableDeclaration");
    m.insert("ɵɵInjectorDeclaration", "ɵɵInjectorDeclaration");
    m.insert("ɵɵNgModuleDeclaration", "ɵɵNgModuleDeclaration");
    m.insert("ɵNgModuleFactory", "NgModuleFactory");
    m.insert("ɵnoSideEffects", "ɵnoSideEffects");
    m
});

const CORE_MODULE: &str = "@angular/core";

/// `ImportRewriter` that rewrites imports from '@angular/core' to be imported
/// from the r3_symbols.ts file instead.
#[derive(Debug, Clone)]
pub struct R3SymbolsImportRewriter {
    r3_symbols_path: String,
}

impl R3SymbolsImportRewriter {
    pub fn new(r3_symbols_path: impl Into<String>) -> Self {
        Self {
            r3_symbols_path: r3_symbols_path.into(),
        }
    }
}

impl ImportRewriter for R3SymbolsImportRewriter {
    fn rewrite_symbol(&self, symbol: &str, specifier: &str) -> String {
        if specifier != CORE_MODULE {
            // This import isn't from core, so ignore it.
            return symbol.to_string();
        }
        
        validate_and_rewrite_core_symbol(symbol)
    }
    
    fn rewrite_specifier(&self, specifier: &str, in_context_of_file: &str) -> String {
        if specifier != CORE_MODULE {
            // This module isn't core, so ignore it.
            return specifier.to_string();
        }
        
        // Calculate relative path from in_context_of_file to r3_symbols_path
        relative_path_between(in_context_of_file, &self.r3_symbols_path)
            .unwrap_or_else(|| self.r3_symbols_path.clone())
    }
    
    fn rewrite_namespace_import_identifier(&self, specifier: &str, _module_name: &str) -> String {
        specifier.to_string()
    }
}

/// Validate that a symbol is supported for core rewriting and return the rewritten name.
pub fn validate_and_rewrite_core_symbol(name: &str) -> String {
    if let Some(&rewritten) = CORE_SUPPORTED_SYMBOLS.get(name) {
        rewritten.to_string()
    } else {
        // Return the symbol as-is for unsupported symbols
        // In TS this throws, but we'll be more lenient
        name.to_string()
    }
}

/// Calculate relative path between two file paths.
fn relative_path_between(from: &str, to: &str) -> Option<String> {
    use std::path::Path;
    
    let from_path = Path::new(from);
    let to_path = Path::new(to);
    
    // Get the parent directory of 'from'
    let from_dir = from_path.parent()?;
    
    // Try to create a relative path
    if let Ok(rel) = to_path.strip_prefix(from_dir) {
        Some(format!("./{}", rel.display()))
    } else {
        // Fall back to counting parent directories
        let from_components: Vec<_> = from_dir.components().collect();
        let to_components: Vec<_> = to_path.components().collect();
        
        // Find common prefix
        let common_len = from_components.iter()
            .zip(to_components.iter())
            .take_while(|(a, b)| a == b)
            .count();
        
        // Build relative path
        let ups = from_components.len() - common_len;
        let downs: Vec<_> = to_components[common_len..].iter()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();
        
        let mut result = String::new();
        for _ in 0..ups {
            result.push_str("../");
        }
        result.push_str(&downs.join("/"));
        
        if result.is_empty() || result.starts_with("..") {
            Some(result)
        } else {
            Some(format!("./{}", result))
        }
    }
}
