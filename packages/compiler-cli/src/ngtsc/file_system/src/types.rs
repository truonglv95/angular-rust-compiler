use std::io;
use std::path::Path;

/// A `string` representing a specific type of path, with a particular brand `B`.
///
/// In Rust, we use a newtype wrapper to achieve branding.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BrandedPath<B>(String, std::marker::PhantomData<B>);

impl<B> BrandedPath<B> {
    pub fn new(path: String) -> Self {
        BrandedPath(path, std::marker::PhantomData)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl<B> AsRef<str> for BrandedPath<B> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<B> std::fmt::Display for BrandedPath<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A fully qualified path in the file system, in POSIX form.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AbsoluteFsPath(String);

impl AbsoluteFsPath {
    pub fn new(path: String) -> Self {
        AbsoluteFsPath(path)
    }

    pub fn from<P: AsRef<Path>>(path: P) -> Self {
        AbsoluteFsPath(path.as_ref().to_string_lossy().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for AbsoluteFsPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<Path> for AbsoluteFsPath {
    fn as_ref(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl std::fmt::Display for AbsoluteFsPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A path that's relative to another (unspecified) root.
///
/// This does not necessarily have to refer to a physical file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathSegment(String);

impl PathSegment {
    pub fn new(path: String) -> Self {
        PathSegment(path)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PathSegment {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PathSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Information about an object in the FileSystem.
/// This is analogous to the `fs.Stats` class in Node.js.
#[derive(Debug, Clone)]
pub struct FileStats {
    pub is_file: bool,
    pub is_directory: bool,
    pub is_symbolic_link: bool,
}

impl FileStats {
    pub fn is_file(&self) -> bool {
        self.is_file
    }

    pub fn is_directory(&self) -> bool {
        self.is_directory
    }

    pub fn is_symbolic_link(&self) -> bool {
        self.is_symbolic_link
    }
}

pub trait IntoPathString {
    fn into_path_string(self) -> String;
}

impl IntoPathString for String {
    fn into_path_string(self) -> String {
        self
    }
}

impl IntoPathString for &str {
    fn into_path_string(self) -> String {
        self.to_string()
    }
}

impl IntoPathString for AbsoluteFsPath {
    fn into_path_string(self) -> String {
        self.0
    }
}

impl IntoPathString for PathSegment {
    fn into_path_string(self) -> String {
        self.0
    }
}

/// An abstraction over the path manipulation aspects of a file-system.
pub trait PathManipulation {
    fn extname(&self, path: &str) -> String;
    fn is_root(&self, path: &AbsoluteFsPath) -> bool;
    fn is_rooted(&self, path: &str) -> bool;
    fn dirname(&self, file: &str) -> String;
    fn join(&self, base_path: &str, paths: &[&str]) -> String;

    /// Compute the relative path between `from` and `to`.
    fn relative(&self, from: &str, to: &str) -> String; // Returns either PathSegment or AbsoluteFsPath as string

    fn basename(&self, file_path: &str, extension: Option<&str>) -> PathSegment;
    fn normalize(&self, path: &str) -> String;
    fn resolve(&self, paths: &[&str]) -> AbsoluteFsPath;
    fn pwd(&self) -> AbsoluteFsPath;
    fn chdir(&self, path: &AbsoluteFsPath);
}

/// An abstraction over the read-only aspects of a file-system.
pub trait ReadonlyFileSystem: PathManipulation + Sync + Send {
    fn is_case_sensitive(&self) -> bool;
    fn exists(&self, path: &AbsoluteFsPath) -> bool;
    fn read_file(&self, path: &AbsoluteFsPath) -> io::Result<String>;
    fn read_file_buffer(&self, path: &AbsoluteFsPath) -> io::Result<Vec<u8>>;
    fn readdir(&self, path: &AbsoluteFsPath) -> io::Result<Vec<PathSegment>>;
    fn lstat(&self, path: &AbsoluteFsPath) -> io::Result<FileStats>;
    fn stat(&self, path: &AbsoluteFsPath) -> io::Result<FileStats>;
    fn realpath(&self, file_path: &AbsoluteFsPath) -> io::Result<AbsoluteFsPath>;
    fn get_default_lib_location(&self) -> AbsoluteFsPath;
}

/// A basic interface to abstract the underlying file-system.
pub trait FileSystem: ReadonlyFileSystem + Sync + Send {
    fn write_file(
        &self,
        path: &AbsoluteFsPath,
        data: &[u8],
        exclusive: Option<bool>,
    ) -> io::Result<()>;
    fn remove_file(&self, path: &AbsoluteFsPath) -> io::Result<()>;
    fn symlink(&self, target: &AbsoluteFsPath, path: &AbsoluteFsPath) -> io::Result<()>;
    fn copy_file(&self, from: &AbsoluteFsPath, to: &AbsoluteFsPath) -> io::Result<()>;
    fn move_file(&self, from: &AbsoluteFsPath, to: &AbsoluteFsPath) -> io::Result<()>;
    fn ensure_dir(&self, path: &AbsoluteFsPath) -> io::Result<()>;
    fn remove_deep(&self, path: &AbsoluteFsPath) -> io::Result<()>;
}
