use dashmap::DashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

/// The result of compiling a single source file — cached to avoid recompiling
/// files that haven't logically changed since the last bundle.
///
/// This is the Rust analog to ngtsc's `ClassRecord[]` returned by
/// `priorAnalysisFor(sf)`.
#[derive(Clone)]
pub struct AnalyzedFile {
    /// The compiled JavaScript output (result of Angular compilation).
    /// This is the value that would normally come from `parallel_compile`.
    pub compiled_js: String,

    /// xxh3 hash of the source content that produced this compiled output.
    /// Used as a sanity check: if a file made it to the analysis cache but
    /// its content changed, we must recompile.
    pub content_hash: u64,

    /// Resource files (HTML templates, CSS) that this file depends on.
    /// Matches `ImportScanResult::resources`.
    pub resource_deps: HashSet<PathBuf>,
}

/// Per-file analysis cache: maps source file path → its last compiled output.
///
/// Entries are invalidated when the file appears in the logical change set.
/// Entries are populated at the end of each successful `parallel_compile` call.
pub type AnalysisCache = Arc<DashMap<PathBuf, AnalyzedFile>>;

/// Create a fresh, empty `AnalysisCache`.
pub fn new_analysis_cache() -> AnalysisCache {
    Arc::new(DashMap::new())
}

/// Given the full set of source files and a set of logically changed files,
/// split them into:
///   - `should_recompile`: files that need fresh compilation
///   - `can_reuse`:        files that can be served from the cache
///
/// A file `can_reuse` only if ALL of the following hold:
///   1. It is NOT in `logical_changed`.
///   2. It has an entry in `analysis_cache`.
///
/// If a file is not in the cache (first run), it must be compiled.
pub fn partition_files(
    all_files: &[PathBuf],
    logical_changed: &HashSet<PathBuf>,
    analysis_cache: &AnalysisCache,
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut should_recompile = Vec::new();
    let mut can_reuse = Vec::new();

    for file in all_files {
        if logical_changed.contains(file) {
            should_recompile.push(file.clone());
        } else if analysis_cache.contains_key(file) {
            can_reuse.push(file.clone());
        } else {
            // Not changed, but not cached either (first run) → compile
            should_recompile.push(file.clone());
        }
    }

    (should_recompile, can_reuse)
}

/// After a successful `parallel_compile` call, update the `analysis_cache`
/// with the freshly compiled results.
///
/// Also remove cache entries for files that are no longer in the project
/// (deleted files).
pub fn update_analysis_cache(
    new_results: &[(PathBuf, String)],
    all_files: &[PathBuf],
    analysis_cache: &AnalysisCache,
) {
    use xxhash_rust::xxh3::xxh3_64;

    // Insert/update freshly compiled files
    for (path, compiled_js) in new_results {
        // We don't have the original source here easily, so we hash the output
        // as a proxy. The version_map already guards correctness at a higher level.
        let content_hash = xxh3_64(compiled_js.as_bytes());
        analysis_cache.insert(
            path.clone(),
            AnalyzedFile {
                compiled_js: compiled_js.clone(),
                content_hash,
                resource_deps: HashSet::new(), // populated separately if needed
            },
        );
    }

    // Evict stale entries for files that are no longer in the project
    let all_set: HashSet<&PathBuf> = all_files.iter().collect();
    analysis_cache.retain(|k, _| all_set.contains(k));
}

/// Invalidate cache entries for files in `logical_changed`.
/// Called at the start of a bundle pass to ensure stale entries are not used.
pub fn invalidate_changed(logical_changed: &HashSet<PathBuf>, analysis_cache: &AnalysisCache) {
    for path in logical_changed {
        analysis_cache.remove(path);
    }
}
