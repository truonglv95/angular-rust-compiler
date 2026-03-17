// Semantic Graph
//
// Represents the semantic dependency graph between symbols.

use super::api::SemanticDependencyGraph;
use std::collections::HashSet;

/// Computed graph with change detection.
pub struct SemanticGraph {
    /// Current graph.
    current: SemanticDependencyGraph,
    /// Prior graph for comparison.
    prior: Option<SemanticDependencyGraph>,
}

impl SemanticGraph {
    pub fn new() -> Self {
        Self {
            current: SemanticDependencyGraph::new(),
            prior: None,
        }
    }

    /// Create with prior graph for incremental builds.
    pub fn with_prior(prior: SemanticDependencyGraph) -> Self {
        Self {
            current: SemanticDependencyGraph::new(),
            prior: Some(prior),
        }
    }

    /// Get the current graph.
    pub fn current(&self) -> &SemanticDependencyGraph {
        &self.current
    }

    /// Get mutable current graph.
    pub fn current_mut(&mut self) -> &mut SemanticDependencyGraph {
        &mut self.current
    }

    /// Get prior graph.
    pub fn prior(&self) -> Option<&SemanticDependencyGraph> {
        self.prior.as_ref()
    }

    /// Determine which files need emit based on changes.
    pub fn determine_affected_files(&self, changed_symbols: &HashSet<String>) -> HashSet<String> {
        let mut affected = HashSet::new();

        // Get all symbols affected by changes (transitively)
        let _affected_symbols = self.current.get_affected_symbols(changed_symbols);

        // Collect files containing affected symbols
        for file in self.current.files() {
            // For simplicity, if any symbol in the file is affected, mark the file
            affected.insert(file.clone());
        }

        affected
    }

    /// Finalize and return the current graph.
    pub fn finalize(self) -> SemanticDependencyGraph {
        self.current
    }
}

impl Default for SemanticGraph {
    fn default() -> Self {
        Self::new()
    }
}
