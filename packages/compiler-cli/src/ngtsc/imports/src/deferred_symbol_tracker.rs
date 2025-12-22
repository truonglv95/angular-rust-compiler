// Deferred Symbol Tracker
//
// Allows registering symbols as deferrable and tracking their usage.
// Used to determine whether it's safe to drop regular imports in favor
// of dynamic imports for defer blocks.

use std::collections::{HashMap, HashSet};

/// Represents an assumption that a symbol is used eagerly somewhere.
const ASSUME_EAGER: &str = "AssumeEager";

/// Symbol state: either assumed eager or has a set of identifier locations.
#[derive(Debug, Clone)]
pub enum SymbolState {
    /// Symbol is assumed to be used eagerly (not yet analyzed).
    AssumeEager,
    /// Set of identifier locations where the symbol is used.
    Locations(HashSet<String>),
}

/// Maps imported symbol name to its state.
pub type SymbolMap = HashMap<String, SymbolState>;

/// Tracks deferrable imports and their usage.
///
/// This information is later used to determine whether it's safe to drop
/// a regular import in favor of using a dynamic import for defer blocks.
#[derive(Debug, Default)]
pub struct DeferredSymbolTracker {
    /// Map of import declaration ID to its symbol map.
    imports: HashMap<String, SymbolMap>,
    
    /// Map of component class to import declarations used in deferredImports.
    explicitly_deferred_imports: HashMap<String, Vec<String>>,
    
    /// Whether to only allow explicit defer dependency imports.
    only_explicit_defer_dependency_imports: bool,
}

impl DeferredSymbolTracker {
    pub fn new(only_explicit_defer_dependency_imports: bool) -> Self {
        Self {
            imports: HashMap::new(),
            explicitly_deferred_imports: HashMap::new(),
            only_explicit_defer_dependency_imports,
        }
    }
    
    /// Extract imported symbols from an import declaration.
    ///
    /// Recognizes these import shapes:
    /// - Case 1: `import {a, b as B} from 'a'`
    /// - Case 2: `import X from 'a'`
    /// - Case 3: `import * as x from 'a'`
    pub fn extract_imported_symbols(&self, symbols: &[&str]) -> SymbolMap {
        let mut symbol_map = SymbolMap::new();
        for symbol in symbols {
            symbol_map.insert(symbol.to_string(), SymbolState::AssumeEager);
        }
        symbol_map
    }
    
    /// Register an import declaration with its symbols.
    pub fn register_import(&mut self, import_id: &str, symbols: &[&str]) {
        let symbol_map = self.extract_imported_symbols(symbols);
        self.imports.insert(import_id.to_string(), symbol_map);
    }
    
    /// Mark an identifier as a candidate for defer loading.
    ///
    /// # Arguments
    /// * `identifier` - The symbol name
    /// * `import_id` - The import declaration ID
    /// * `component_class` - The component class ID
    /// * `is_explicitly_deferred` - Whether this is from @Component.deferredImports
    pub fn mark_as_deferrable_candidate(
        &mut self,
        identifier: &str,
        import_id: &str,
        component_class: &str,
        is_explicitly_deferred: bool,
    ) {
        if self.only_explicit_defer_dependency_imports && !is_explicitly_deferred {
            return;
        }
        
        if is_explicitly_deferred {
            self.explicitly_deferred_imports
                .entry(component_class.to_string())
                .or_insert_with(Vec::new)
                .push(import_id.to_string());
        }
        
        if let Some(symbol_map) = self.imports.get_mut(import_id) {
            if let Some(state) = symbol_map.get_mut(identifier) {
                // If AssumeEager, convert to empty set (meaning no eager refs remaining)
                if matches!(state, SymbolState::AssumeEager) {
                    *state = SymbolState::Locations(HashSet::new());
                }
            }
        }
    }
    
    /// Check if all symbols from an import can be deferred.
    pub fn can_defer(&self, import_id: &str) -> bool {
        if let Some(symbol_map) = self.imports.get(import_id) {
            for state in symbol_map.values() {
                match state {
                    SymbolState::AssumeEager => return false,
                    SymbolState::Locations(refs) if !refs.is_empty() => return false,
                    _ => {}
                }
            }
            true
        } else {
            false
        }
    }
    
    /// Get set of import declaration IDs that are safe to defer.
    pub fn get_deferrable_import_decls(&self) -> HashSet<String> {
        let mut deferrable = HashSet::new();
        for import_id in self.imports.keys() {
            if self.can_defer(import_id) {
                deferrable.insert(import_id.clone());
            }
        }
        deferrable
    }
    
    /// Get non-removable deferred imports for a component.
    pub fn get_non_removable_deferred_imports(
        &self,
        _source_file: &str,
        component_class: &str,
    ) -> Vec<String> {
        let mut affected = Vec::new();
        if let Some(import_ids) = self.explicitly_deferred_imports.get(component_class) {
            for import_id in import_ids {
                if !self.can_defer(import_id) {
                    affected.push(import_id.clone());
                }
            }
        }
        affected
    }
}
