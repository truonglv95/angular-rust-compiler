#![deny(clippy::all)]

use angular_compiler_cli::ngtsc::core::NgCompilerOptions;
use angular_compiler_cli::ngtsc::file_system::src::node_js_file_system::NodeJSFileSystem;
use angular_compiler_cli::ngtsc::file_system::src::types::{
    AbsoluteFsPath, FileStats, PathManipulation, PathSegment,
};
use angular_compiler_cli::ngtsc::file_system::FileSystem;
use angular_compiler_cli::ngtsc::file_system::ReadonlyFileSystem;
use angular_compiler_cli::ngtsc::program::NgtscProgram;
use napi_derive::napi;
use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::Mutex;

/// A FileSystem that reads from disk (NodeJS) but captures writes in memory.
struct CapturingFileSystem {
    delegate: NodeJSFileSystem,
    captured_files: Mutex<HashMap<AbsoluteFsPath, Vec<u8>>>,
}

impl CapturingFileSystem {
    fn new() -> Self {
        CapturingFileSystem {
            delegate: NodeJSFileSystem::new(),
            captured_files: Mutex::new(HashMap::new()),
        }
    }
}

impl ReadonlyFileSystem for CapturingFileSystem {
    fn is_case_sensitive(&self) -> bool {
        self.delegate.is_case_sensitive()
    }
    fn exists(&self, path: &AbsoluteFsPath) -> bool {
        // Check captured first? Or overlay?
        // For output files, we should check captured.
        let captured = self.captured_files.lock().unwrap();
        if captured.contains_key(path) {
            true
        } else {
            self.delegate.exists(path)
        }
    }
    fn read_file(&self, path: &AbsoluteFsPath) -> io::Result<String> {
        let captured = self.captured_files.lock().unwrap();
        if let Some(content) = captured.get(path) {
            return String::from_utf8(content.clone())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
        }
        self.delegate.read_file(path)
    }
    fn read_file_buffer(&self, path: &AbsoluteFsPath) -> io::Result<Vec<u8>> {
        let captured = self.captured_files.lock().unwrap();
        if let Some(content) = captured.get(path) {
            return Ok(content.clone());
        }
        self.delegate.read_file_buffer(path)
    }
    // Delegate others
    fn readdir(&self, path: &AbsoluteFsPath) -> io::Result<Vec<PathSegment>> {
        self.delegate.readdir(path)
    }
    fn lstat(&self, path: &AbsoluteFsPath) -> io::Result<FileStats> {
        self.delegate.lstat(path)
    }
    fn stat(&self, path: &AbsoluteFsPath) -> io::Result<FileStats> {
        self.delegate.stat(path)
    }
    fn realpath(&self, path: &AbsoluteFsPath) -> io::Result<AbsoluteFsPath> {
        self.delegate.realpath(path)
    }
    fn get_default_lib_location(&self) -> AbsoluteFsPath {
        self.delegate.get_default_lib_location()
    }
}

impl PathManipulation for CapturingFileSystem {
    fn dirname(&self, file: &str) -> String {
        self.delegate.dirname(file)
    }
    fn join(&self, base_path: &str, paths: &[&str]) -> String {
        self.delegate.join(base_path, paths)
    }
    fn resolve(&self, paths: &[&str]) -> AbsoluteFsPath {
        self.delegate.resolve(paths)
    }
    fn basename(&self, path: &str, ext: Option<&str>) -> PathSegment {
        self.delegate.basename(path, ext)
    }
    fn extname(&self, path: &str) -> String {
        self.delegate.extname(path)
    }
    fn is_root(&self, path: &AbsoluteFsPath) -> bool {
        self.delegate.is_root(path)
    }
    fn is_rooted(&self, path: &str) -> bool {
        self.delegate.is_rooted(path)
    }
    fn normalize(&self, path: &str) -> String {
        self.delegate.normalize(path)
    }
    fn relative(&self, from: &str, to: &str) -> String {
        self.delegate.relative(from, to)
    }
    fn pwd(&self) -> AbsoluteFsPath {
        self.delegate.pwd()
    }
    fn chdir(&self, path: &AbsoluteFsPath) {
        self.delegate.chdir(path)
    }
}

impl FileSystem for CapturingFileSystem {
    fn write_file(
        &self,
        path: &AbsoluteFsPath,
        data: &[u8],
        _exclusive: Option<bool>,
    ) -> io::Result<()> {
        let mut captured = self.captured_files.lock().unwrap();
        captured.insert(path.clone(), data.to_vec());
        Ok(())
    }
    fn remove_file(&self, path: &AbsoluteFsPath) -> io::Result<()> {
        let mut captured = self.captured_files.lock().unwrap();
        captured.remove(path);
        // We generally shouldn't delete from disk in compilation
        Ok(())
    }
    fn symlink(&self, target: &AbsoluteFsPath, path: &AbsoluteFsPath) -> io::Result<()> {
        // Ignore symlinks for now
        Ok(())
    }
    fn copy_file(&self, from: &AbsoluteFsPath, to: &AbsoluteFsPath) -> io::Result<()> {
        let content = self.read_file_buffer(from)?;
        self.write_file(to, &content, None)
    }
    fn move_file(&self, from: &AbsoluteFsPath, to: &AbsoluteFsPath) -> io::Result<()> {
        self.copy_file(from, to)?;
        self.remove_file(from)
    }
    fn ensure_dir(&self, _path: &AbsoluteFsPath) -> io::Result<()> {
        Ok(())
    }
    fn remove_deep(&self, _path: &AbsoluteFsPath) -> io::Result<()> {
        Ok(())
    }
}

#[napi(object)]
pub struct Diagnostic {
    pub file: Option<String>,
    pub message: String,
    pub code: u32,
    pub start: Option<u32>,
    pub length: Option<u32>,
}

#[napi(object)]
pub struct CompileResult {
    pub code: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[napi]
pub struct Compiler {}

#[napi]
impl Compiler {
    #[napi(constructor)]
    pub fn new() -> Self {
        Compiler {}
    }

    #[napi]
    pub fn compile(&self, filename: String, _content: String) -> CompileResult {
        // 1. Setup Capturing FileSystem (delegates reads to disk so imports work!)
        let fs = CapturingFileSystem::new();

        // Note: We ignore `_content` here and read from disk to ensure consistency
        // with how imports are resolved. In a real advanced implementation, we would
        // overlay `_content` into the `captured_files` map immediately.
        // Let's do that actually, to support "unsaved" changes if Rspack sends them.
        let abs_filename_str = fs.resolve(&[&filename]).to_string();
        let abs_filename = AbsoluteFsPath::from(Path::new(&abs_filename_str));

        // Overlay the in-memory content (optional, but good for Rspack)
        fs.write_file(&abs_filename, _content.as_bytes(), None).ok();

        // 2. Setup Compiler Options
        let mut options = NgCompilerOptions::default();
        // Configure options to ensure in-place compilation results
        // By setting project to the file, and out_dir to the file's dir, we force 1:1 emission in place.
        options.project = abs_filename_str.clone();
        options.out_dir = Some(fs.dirname(&abs_filename_str));

        // 3. Create Program
        let root_names = vec![abs_filename_str.clone()];
        let mut program = NgtscProgram::new(root_names, options, &fs);

        // 4. Compile
        // load_ng_structure needs a valid path to tsconfig usually, or just root.
        // We'll pass the dirname of the file as project root or just "/"
        let mut diagnostics = Vec::new();
        if let Err(e) = program.load_ng_structure(Path::new("/")) {
            return CompileResult {
                code: format!("/* Error loading: {} */", e),
                diagnostics: vec![],
            };
        }

        match program.emit() {
            Ok(emit_diagnostics) => {
                for diag in emit_diagnostics {
                    diagnostics.push(Diagnostic {
                        file: diag.file.map(|p| p.to_string_lossy().to_string()),
                        message: diag.message,
                        code: diag.code as u32,
                        start: diag.start.map(|s| s as u32),
                        length: diag.length.map(|l| l as u32),
                    });
                }
            }
            Err(e) => {
                return CompileResult {
                    code: format!("/* Error emitting: {} */", e),
                    diagnostics: vec![],
                };
            }
        }

        // 5. Retrieve output from Memory
        // The output path is usually same as input but .js extension
        let output_path_str = abs_filename_str.replace(".ts", ".js");
        let output_path = AbsoluteFsPath::from(Path::new(&output_path_str));

        let code = match fs.read_file(&output_path) {
            Ok(js_content) => js_content,
            Err(_) => {
                format!("/* Output not found in memory for {} */", output_path_str)
            }
        };

        CompileResult { code, diagnostics }
    }

    #[napi]
    pub fn link_file(&self, filename: String, source_code: String) -> String {
        // Delegate to linker in compiler-cli
        // We need to enable feature napi-bindings for this to be available
        use angular_compiler_cli::linker::napi::link_file;

        match link_file(source_code, filename) {
            Ok(code) => code,
            Err(e) => format!("/* Linker Error: {} */", e),
        }
    }
}
