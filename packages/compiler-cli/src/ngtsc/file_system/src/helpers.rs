use crate::ngtsc::file_system::src::invalid_file_system::InvalidFileSystem;
use crate::ngtsc::file_system::src::types::{AbsoluteFsPath, FileSystem, PathSegment};
use crate::ngtsc::file_system::src::util::normalize_separators;
use std::sync::{Arc, RwLock};

// Use lazy_static or std::sync::OnceLock if available.
// Since we don't have external crates guaranteed, we'll use a crude static with unsafe or a simple RwLock with a lazy initialization pattern if strictly needed,
// but for now let's assume we can use a global static RwLock if initialized.
// However, Rust statics must be const initialized. `InvalidFileSystem` is zero-sized but wrapping it in Box/Arc is not const.
// We will use a `static mut` with `Once` or similar, or better:
// Since `lazy_static` is common, I'll assume it's available or implement a simple version.
// Actually, `std::sync::OnceLock` is stabilized in 1.70. I'll try to use that if I can, but to be safe and 1:1 with "global var", I'll use `std::sync::RwLock` wrapped in a way that allows swapping.
//
// A common pattern without external crates for a global singleton that is swappable:
// static FS: RwLock<Option<Arc<dyn FileSystem + Sync + Send>>> = RwLock::new(None);
// And accessors initialize it to InvalidFileSystem if None.

static FILE_SYSTEM: RwLock<Option<Arc<dyn FileSystem + Sync + Send>>> = RwLock::new(None);

pub fn get_file_system() -> Arc<dyn FileSystem + Sync + Send> {
    let fs_lock = FILE_SYSTEM.read().unwrap();
    if let Some(fs) = &*fs_lock {
        return fs.clone();
    }
    // If not set, return InvalidFileSystem.
    // We shouldn't hold the read lock while acquiring write lock if we were to init it here, but we just return a new invalid one.
    // Or better, return a static reference if possible, but Arc is handy.
    Arc::new(InvalidFileSystem)
}

pub fn set_file_system(file_system: Arc<dyn FileSystem + Sync + Send>) {
    let mut fs_lock = FILE_SYSTEM.write().unwrap();
    *fs_lock = Some(file_system);
}

pub fn absolute_from(path: &str) -> AbsoluteFsPath {
    let fs = get_file_system();
    if !fs.is_rooted(path) {
        panic!(
            "Internal Error: absoluteFrom({}): path is not absolute",
            path
        );
    }
    fs.resolve(&[path])
}

pub fn absolute_from_source_file(_sf: &oxc_ast::ast::Program) -> AbsoluteFsPath {
    // Rust doesn't have the same "patching" capability as JS objects (Sybmol patch).
    // We assume we can get the filename from the source file or it's passed separately.
    // The Oxc Program doesn't carry filename usually.
    // We might need to change the signature to match Rust's data structures.
    // For now, let's assume we pass the filename string associated with the SourceFile.
    unimplemented!("absolute_from_source_file requires a way to attach metadata to SourceFile or passing filename directly")
}

pub fn relative_from(path: &str) -> PathSegment {
    let fs = get_file_system();
    let normalized = normalize_separators(path);
    if fs.is_rooted(&normalized) {
        panic!(
            "Internal Error: relativeFrom({}): path is not relative",
            path
        );
    }
    PathSegment::new(normalized)
}

pub fn dirname(file: &str) -> String {
    get_file_system().dirname(file)
}

pub fn join(base_path: &str, paths: &[&str]) -> String {
    get_file_system().join(base_path, paths)
}

pub fn resolve(base_path: &str, paths: &[&str]) -> AbsoluteFsPath {
    let mut all_paths = vec![base_path];
    all_paths.extend_from_slice(paths);
    get_file_system().resolve(&all_paths)
}

pub fn is_root(path: &AbsoluteFsPath) -> bool {
    get_file_system().is_root(path)
}

pub fn is_rooted(path: &str) -> bool {
    get_file_system().is_rooted(path)
}

pub fn relative(from: &str, to: &str) -> String {
    get_file_system().relative(from, to)
}

pub fn basename(file_path: &str, extension: Option<&str>) -> PathSegment {
    get_file_system().basename(file_path, extension)
}

pub fn is_local_relative_path(relative_path: &str) -> bool {
    !is_rooted(relative_path) && !relative_path.starts_with("..")
}

pub fn to_relative_import(relative_path: &str) -> String {
    if is_local_relative_path(relative_path) {
        format!("./{}", relative_path)
    } else {
        relative_path.to_string()
    }
}
