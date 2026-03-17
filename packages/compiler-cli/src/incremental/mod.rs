/// Incremental build support for the Angular Rust compiler.
///
/// This module provides a 3-layer system that mirrors Angular ngtsc's approach:
///
/// # Layer 1 — Physical Change Detection (`version_map`)
/// Tracks per-file mtime+size so we know exactly which files changed
/// since the last successful bundle, without reading file contents.
///
/// # Layer 2 — Logical Change Propagation (`dep_graph`)
/// Builds a reverse dependency graph from `ScanCache` (already populated
/// during import scanning). From the physical changed set, computes the
/// **transitive** set of affected files via BFS.
///
/// # Layer 3 — Analysis Cache (`analysis_cache`)
/// Caches the compiled JavaScript output per source file. Files not in
/// the logical change set have their cached output reused, so
/// `parallel_compile` is only invoked for the minimal affected set.
///
/// ## Usage in `bundle_project`
///
/// ```text
/// 1. compute_changed_files(all_files, version_map) → physically_changed
/// 2. FileDependencyGraph::from_scan_cache(scan_cache) → graph
/// 3. graph.compute_logical_changes(physically_changed, ∅) → logical_changed
/// 4. partition_files(all_files, logical_changed, analysis_cache)
///       → (should_recompile, can_reuse)
/// 5. parallel_compile(should_recompile) → new_results
/// 6. merge: new_results ∪ {analysis_cache[f] for f in can_reuse}
/// 7. update_analysis_cache(new_results, all_files, analysis_cache)
/// 8. update_versions(all_files, version_map)
/// ```
pub mod analysis_cache;
pub mod dep_graph;
pub mod version_map;

// Re-export the most commonly used items for ergonomic imports.
pub use analysis_cache::{
    invalidate_changed, new_analysis_cache, partition_files, update_analysis_cache, AnalysisCache,
    AnalyzedFile,
};
pub use dep_graph::{new_shared_dep_graph, FileDependencyGraph, SharedDepGraph};
pub use version_map::{
    compute_changed_files, file_version, update_versions, FileVersion, FileVersionMap,
};

use dashmap::DashMap;
use std::sync::Arc;

/// Create a new `FileVersionMap`.
pub fn new_version_map() -> FileVersionMap {
    Arc::new(DashMap::new())
}
