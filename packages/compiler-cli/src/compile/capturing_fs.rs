use crate::ngtsc::file_system::{
    AbsoluteFsPath, FileStats, FileSystem, PathManipulation, PathSegment, ReadonlyFileSystem,
};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct CapturingFileSystem<T: FileSystem> {
    delegate: T,
    pub files: Arc<Mutex<HashMap<PathBuf, String>>>,
}

impl<T: FileSystem> CapturingFileSystem<T> {
    pub fn new(delegate: T) -> Self {
        Self {
            delegate,
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<T: FileSystem> PathManipulation for CapturingFileSystem<T> {
    fn extname(&self, path: &str) -> String {
        self.delegate.extname(path)
    }

    fn is_root(&self, path: &AbsoluteFsPath) -> bool {
        self.delegate.is_root(path)
    }

    fn is_rooted(&self, path: &str) -> bool {
        self.delegate.is_rooted(path)
    }

    fn dirname(&self, file: &str) -> String {
        self.delegate.dirname(file)
    }

    fn join(&self, base_path: &str, paths: &[&str]) -> String {
        self.delegate.join(base_path, paths)
    }

    fn relative(&self, from: &str, to: &str) -> String {
        self.delegate.relative(from, to)
    }

    fn basename(&self, file_path: &str, extension: Option<&str>) -> PathSegment {
        self.delegate.basename(file_path, extension)
    }

    fn normalize(&self, path: &str) -> String {
        self.delegate.normalize(path)
    }

    fn resolve(&self, paths: &[&str]) -> AbsoluteFsPath {
        self.delegate.resolve(paths)
    }

    fn pwd(&self) -> AbsoluteFsPath {
        self.delegate.pwd()
    }

    fn chdir(&self, path: &AbsoluteFsPath) {
        self.delegate.chdir(path)
    }
}

impl<T: FileSystem> ReadonlyFileSystem for CapturingFileSystem<T> {
    fn is_case_sensitive(&self) -> bool {
        self.delegate.is_case_sensitive()
    }

    fn exists(&self, path: &AbsoluteFsPath) -> bool {
        // Check memory first? No, we only catch writes.
        // Actually, if we wrote a file, subsequent reads should ideally see it if the compiler reads back what it wrote.
        // But for this simple pipeline, we probably don't need read-after-write consistency for intermediate artifacts yet.
        self.delegate.exists(path)
    }

    fn read_file(&self, path: &AbsoluteFsPath) -> io::Result<String> {
        self.delegate.read_file(path)
    }

    fn read_file_buffer(&self, path: &AbsoluteFsPath) -> io::Result<Vec<u8>> {
        self.delegate.read_file_buffer(path)
    }

    fn readdir(&self, path: &AbsoluteFsPath) -> io::Result<Vec<PathSegment>> {
        self.delegate.readdir(path)
    }

    fn lstat(&self, path: &AbsoluteFsPath) -> io::Result<FileStats> {
        self.delegate.lstat(path)
    }

    fn stat(&self, path: &AbsoluteFsPath) -> io::Result<FileStats> {
        self.delegate.stat(path)
    }

    fn realpath(&self, file_path: &AbsoluteFsPath) -> io::Result<AbsoluteFsPath> {
        self.delegate.realpath(file_path)
    }

    fn get_default_lib_location(&self) -> AbsoluteFsPath {
        self.delegate.get_default_lib_location()
    }
}

impl<T: FileSystem> FileSystem for CapturingFileSystem<T> {
    fn write_file(
        &self,
        path: &AbsoluteFsPath,
        data: &[u8],
        _exclusive: Option<bool>,
    ) -> io::Result<()> {
        let path_buf = path.as_path().to_path_buf();
        let content = String::from_utf8_lossy(data).to_string();
        self.files.lock().unwrap().insert(path_buf, content);
        Ok(())
    }

    fn remove_file(&self, path: &AbsoluteFsPath) -> io::Result<()> {
        self.delegate.remove_file(path)
    }

    fn symlink(&self, target: &AbsoluteFsPath, path: &AbsoluteFsPath) -> io::Result<()> {
        self.delegate.symlink(target, path)
    }

    fn copy_file(&self, from: &AbsoluteFsPath, to: &AbsoluteFsPath) -> io::Result<()> {
        self.delegate.copy_file(from, to)
    }

    fn move_file(&self, from: &AbsoluteFsPath, to: &AbsoluteFsPath) -> io::Result<()> {
        self.delegate.move_file(from, to)
    }

    fn ensure_dir(&self, _path: &AbsoluteFsPath) -> io::Result<()> {
        // We can just ignore directory creation for memory writes
        Ok(())
    }

    fn remove_deep(&self, path: &AbsoluteFsPath) -> io::Result<()> {
        self.delegate.remove_deep(path)
    }
}
