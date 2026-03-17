use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use crate::bundler::ScanCache;

/// Reverse dependency graph: for each file, the set of files that import it.
///
/// This enables O(1) lookup of "who depends on file X" which is the core
/// operation needed for logical change propagation.
///
/// The graph is intentionally **not** `Arc<DashMap>` — it is rebuilt from
/// `ScanCache` at the start of each incremental bundle pass (cheap, since
/// ScanCache already has all the import info). It is stored in the `Compiler`
/// struct behind an `Arc<Mutex<>>` so it can be swapped atomically.
#[derive(Default)]
pub struct FileDependencyGraph {
    /// `dependents`: file → set of files that **import** this file.
    /// (The "reverse" / "upstream" direction.)
    pub dependents: HashMap<PathBuf, HashSet<PathBuf>>,

    /// `dependencies`: file → set of files this file **imports**.
    /// (The "forward" direction, mirroring `ScanCache`.)
    pub dependencies: HashMap<PathBuf, HashSet<PathBuf>>,

    /// `resource_owners`: resource path (.html/.css) → set of TS files that
    /// declare it as a dependency via @Component templateUrl/styleUrls.
    pub resource_owners: HashMap<PathBuf, HashSet<PathBuf>>,
}

impl FileDependencyGraph {
    /// Build a new graph by inverting the `ScanCache` import graph.
    ///
    /// This is O(E) where E = total number of import edges in the project,
    /// and typically runs in < 1ms because ScanCache is already populated.
    pub fn from_scan_cache(scan_cache: &ScanCache) -> Self {
        let mut graph = FileDependencyGraph::default();

        for entry in scan_cache.iter() {
            let file = entry.key().clone();
            let result = entry.value();

            let mut deps: HashSet<PathBuf> = HashSet::new();

            for dep in &result.static_imports {
                deps.insert(dep.clone());
                graph
                    .dependents
                    .entry(dep.clone())
                    .or_default()
                    .insert(file.clone());
            }
            for dep in &result.dynamic_imports {
                deps.insert(dep.clone());
                graph
                    .dependents
                    .entry(dep.clone())
                    .or_default()
                    .insert(file.clone());
            }
            for resource in &result.resources {
                graph
                    .resource_owners
                    .entry(resource.clone())
                    .or_default()
                    .insert(file.clone());
            }

            graph.dependencies.insert(file, deps);
        }

        graph
    }

    /// Compute the **logical change set**: the transitive closure of all files
    /// that are affected by the given `physically_changed` and
    /// `changed_resources` sets.
    ///
    /// Algorithm: BFS from each changed file through the **reverse** graph
    /// (`dependents`), collecting all reachable files.
    ///
    /// Mirrors `FileDependencyGraph.updateWithPhysicalChanges()` in ngtsc.
    pub fn compute_logical_changes(
        &self,
        physically_changed: &HashSet<PathBuf>,
        changed_resources: &HashSet<PathBuf>,
    ) -> HashSet<PathBuf> {
        let mut logical_changed: HashSet<PathBuf> = HashSet::new();
        let mut queue: VecDeque<PathBuf> = VecDeque::new();

        // Seed: every physically changed TS file
        for path in physically_changed {
            if logical_changed.insert(path.clone()) {
                queue.push_back(path.clone());
            }
        }

        // Seed: files that depend on changed resources (HTML/CSS)
        for resource in changed_resources {
            if let Some(owners) = self.resource_owners.get(resource) {
                for owner in owners {
                    if logical_changed.insert(owner.clone()) {
                        queue.push_back(owner.clone());
                    }
                }
            }
        }

        // BFS: propagate through the reverse graph
        while let Some(file) = queue.pop_front() {
            if let Some(deps) = self.dependents.get(&file) {
                for dependent in deps {
                    if logical_changed.insert(dependent.clone()) {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }

        logical_changed
    }

    /// Return the number of edges in the forward dependency graph.
    /// Useful for diagnostics / logging.
    pub fn edge_count(&self) -> usize {
        self.dependencies.values().map(|s| s.len()).sum()
    }
}

/// Shared, heap-allocated `FileDependencyGraph` that can be stored in the
/// `Compiler` struct and replaced atomically after each bundle.
pub type SharedDepGraph = Arc<std::sync::RwLock<FileDependencyGraph>>;

/// Create a new shared dep graph, initially empty.
pub fn new_shared_dep_graph() -> SharedDepGraph {
    Arc::new(std::sync::RwLock::new(FileDependencyGraph::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn pb(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    /// Helper: build a `ScanCache`-like `DashMap` from a simple list of edges.
    fn make_scan_cache(edges: &[(&str, &[&str])]) -> ScanCache {
        let cache = Arc::new(DashMap::new());
        for (file, imports) in edges {
            cache.insert(
                pb(file),
                crate::bundler::ImportScanResult {
                    static_imports: imports.iter().map(|s| pb(s)).collect(),
                    dynamic_imports: vec![],
                    resources: vec![],
                },
            );
        }
        cache
    }

    #[test]
    fn test_transitive_propagation() {
        // A → B → C  (A imports B, B imports C)
        let cache = make_scan_cache(&[("A.ts", &["B.ts"]), ("B.ts", &["C.ts"]), ("C.ts", &[])]);

        let graph = FileDependencyGraph::from_scan_cache(&cache);

        // Changing C should logically change C, B, and A
        let changed = graph.compute_logical_changes(&HashSet::from([pb("C.ts")]), &HashSet::new());

        assert!(changed.contains(&pb("C.ts")));
        assert!(changed.contains(&pb("B.ts")));
        assert!(changed.contains(&pb("A.ts")));
    }

    #[test]
    fn test_no_propagation_for_unchanged() {
        // A → B → C, only C changes
        let cache = make_scan_cache(&[("A.ts", &["B.ts"]), ("B.ts", &["C.ts"]), ("C.ts", &[])]);

        let graph = FileDependencyGraph::from_scan_cache(&cache);

        // If A changes (leaf in reverse graph), only A is affected
        let changed = graph.compute_logical_changes(&HashSet::from([pb("A.ts")]), &HashSet::new());

        assert!(changed.contains(&pb("A.ts")));
        assert!(!changed.contains(&pb("B.ts")));
        assert!(!changed.contains(&pb("C.ts")));
    }

    #[test]
    fn test_resource_propagation() {
        // A uses template.html; B imports A
        let cache = Arc::new(DashMap::new());
        cache.insert(
            pb("A.ts"),
            crate::bundler::ImportScanResult {
                static_imports: vec![],
                dynamic_imports: vec![],
                resources: vec![pb("template.html")],
            },
        );
        cache.insert(
            pb("B.ts"),
            crate::bundler::ImportScanResult {
                static_imports: vec![pb("A.ts")],
                dynamic_imports: vec![],
                resources: vec![],
            },
        );

        let graph = FileDependencyGraph::from_scan_cache(&cache);

        let changed =
            graph.compute_logical_changes(&HashSet::new(), &HashSet::from([pb("template.html")]));

        // A owns template.html; B imports A → both should be logically changed
        assert!(changed.contains(&pb("A.ts")));
        assert!(changed.contains(&pb("B.ts")));
    }
}
