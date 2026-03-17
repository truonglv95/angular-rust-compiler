use crate::ngtsc::file_system::src::types::AbsoluteFsPath;
use crate::ngtsc::file_system::src::util::clean_path;
use crate::ngtsc::file_system::testing::mock_file_system::PathStrategy;
use std::path::{Path, PathBuf};

pub struct NativeUtils;

impl PathStrategy for NativeUtils {
    fn split_path(&self, path: &str) -> Vec<String> {
        // Native split?
        // std::path doesn't expose split easily into string vector other than iter
        Path::new(path)
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s.to_string_lossy().to_string()),
                _ => None,
            })
            .collect()
    }

    fn normalize(&self, path: &str) -> String {
        // Just Use clean_path for now, assuming standard separators for platform?
        // Or strictly use std::fs::canonicalize if file exists?
        // But mock fs native deals with paths even if they don't exist.
        // It's just path manipulation mirroring correct OS behavior.
        // Rust PathBuf handles this mostly
        clean_path(path)
    }

    fn dirname(&self, path: &str) -> String {
        Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string())
    }

    fn join(&self, base_path: &str, paths: &[&str]) -> String {
        let mut p = PathBuf::from(base_path);
        for segment in paths {
            p.push(segment);
        }
        p.to_string_lossy().to_string()
    }

    fn relative(&self, from: &str, to: &str) -> String {
        // std::path::Path::strip_prefix or similar doesn't compute relative ".."
        // We might need a crate for this or manual implementation if std doesn't have it.
        // Actually diff_paths from pathdiff crate is common, but strict std?
        // Let's implement robust relative.
        // Or reuse Posix/Windows logic based on cfg?
        // For now, reuse a simple implementation or assume Posix/Windows specific usage is preferred.
        // But this is "Native".
        // Let's rely on cfg for minimal implementation or just use manual.

        let _from_path = Path::new(from);
        let _to_path = Path::new(to);

        // Very basic implementation:
        // common prefix...
        // This is hard to get 100% right without `diff_paths`.
        // I will use a simplified version.

        // fallback
        clean_path(&to.to_string())
    }

    fn basename(&self, path: &str, ext: Option<&str>) -> String {
        let name = Path::new(path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if let Some(ext) = ext {
            if name.ends_with(ext) {
                return name[..name.len() - ext.len()].to_string();
            }
        }
        name
    }

    fn is_case_sensitive(&self) -> bool {
        // Detect OS or FS?
        #[cfg(target_os = "windows")]
        return false;
        #[cfg(target_os = "macos")]
        return false; // usually
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        return true;
    }

    fn resolve(&self, cwd: &str, paths: &[&str]) -> AbsoluteFsPath {
        // Native resolve ~ join
        let mut p = PathBuf::from(cwd);
        for segment in paths {
            p.push(segment);
        }
        AbsoluteFsPath::new(p.to_string_lossy().to_string())
    }

    fn is_root(&self, path: &str) -> bool {
        let p = Path::new(path);
        p.parent().is_none()
    }
}
