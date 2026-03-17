use dashmap::DashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

/// Maps each source file to a version fingerprint (mtime + size).
/// Files with different fingerprints have been physically changed.
pub type FileVersionMap = Arc<DashMap<PathBuf, FileVersion>>;

/// Compact version descriptor for a single file.
///
/// Using mtime + size (not content hash) is intentional: reading 549 files
/// to content-hash them is slow. mtime + size is an O(1) syscall and
/// matches what TypeScript's language service uses internally.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FileVersion {
    /// Seconds since UNIX epoch of the file's last modification.
    pub mtime_secs: u64,
    /// Nanosecond sub-second of mtime, for sub-second precision.
    pub mtime_nanos: u32,
    /// File size in bytes. Used as a cheap change discriminator when
    /// mtime precision is low (e.g. FAT32 2-second granularity).
    pub size_bytes: u64,
}

/// Compute the `FileVersion` for a path.
/// Returns `None` if the file cannot be stat'd (e.g. deleted).
pub fn file_version(path: &Path) -> Option<FileVersion> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    let since_epoch = mtime
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    Some(FileVersion {
        mtime_secs: since_epoch.as_secs(),
        mtime_nanos: since_epoch.subsec_nanos(),
        size_bytes: meta.len(),
    })
}

/// Canonicalize a path for use as a DashMap key.
/// Falls back to the original path if canonicalization fails (e.g. file deleted).
fn canonical_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Walk a slice of source files and return those whose `FileVersion`
/// differs from the last recorded version in `version_map`.
///
/// Also returns the set of newly-seen files (first time encountered).
///
/// The caller must call `update_versions` after a successful bundle
/// to persist the new versions.
pub fn compute_changed_files(
    all_files: &[PathBuf],
    version_map: &FileVersionMap,
) -> HashSet<PathBuf> {
    let mut changed = HashSet::new();

    // DEBUG: Log map state on second+ run
    if !version_map.is_empty() {
        let first_stored = version_map.iter().next().map(|r| r.key().clone());
        let first_lookup = all_files.first().map(|p| canonical_key(p));
        eprintln!(
            "[INCREMENTAL DEBUG] version_map has {} entries. first_stored={:?} first_lookup={:?}",
            version_map.len(),
            first_stored,
            first_lookup
        );
    }

    for path in all_files {
        let key = canonical_key(path);
        let current = match file_version(&key) {
            Some(v) => v,
            None => {
                // File disappeared — treat as changed so dependents are
                // invalidated too.
                changed.insert(path.clone());
                continue;
            }
        };

        match version_map.get(&key) {
            Some(prev) if *prev == current => {
                // Identical version → unchanged.
            }
            _ => {
                // Not in map (new file) or version mismatch → changed.
                changed.insert(path.clone());
            }
        }
    }

    changed
}

/// Persist the versions for all files after a successful bundle.
/// Call this at the **end** of `bundle_project` so the next incremental
/// pass has accurate baseline versions.
pub fn update_versions(all_files: &[PathBuf], version_map: &FileVersionMap) {
    for path in all_files {
        let key = canonical_key(path);
        if let Some(v) = file_version(&key) {
            version_map.insert(key, v);
        }
    }
    // Remove stale entries for files that no longer exist
    let all_set: HashSet<PathBuf> = all_files.iter().map(|p| canonical_key(p)).collect();
    version_map.retain(|k, _| all_set.contains(k));
}
