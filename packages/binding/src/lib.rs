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
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use xxhash_rust::xxh3::xxh3_64;

// ============ Cache Configuration ============
const CACHE_DIR_NAME: &str = ".angular/cache/rust-compiler";
const COMPILER_CACHE_SUBDIR: &str = "compiler";
const LINKER_CACHE_SUBDIR: &str = "linker";

/// Compute xxHash3 (64-bit) of content - extremely fast (~10GB/s)
fn compute_hash(content: &str) -> String {
    format!("{:016x}", xxh3_64(content.as_bytes()))
}

/// Extract template content if templateUrl is present in the TS file.
/// Returns the combined content (ts + template) for hashing.
fn get_combined_content_for_hash(filename: &str, ts_content: &str) -> String {
    use regex::Regex;

    // Look for templateUrl: './something.html' or templateUrl: "./something.html"
    let re = Regex::new(r#"templateUrl\s*:\s*['"]([^'"]+)['"]"#).ok();

    if let Some(regex) = re {
        if let Some(captures) = regex.captures(ts_content) {
            if let Some(template_path) = captures.get(1) {
                // Resolve template path relative to TS file
                let ts_dir = Path::new(filename).parent().unwrap_or(Path::new("."));
                let template_file = ts_dir.join(template_path.as_str());

                // Read template content
                if let Ok(template_content) = fs::read_to_string(&template_file) {
                    // Combine TS + template content for hash
                    return format!("{}\n---TEMPLATE---\n{}", ts_content, template_content);
                }
            }
        }
    }

    // Also check for styleUrl/styleUrls
    let style_re = Regex::new(r#"styleUrl[s]?\s*:\s*['"]([^'"]+)['"]"#).ok();
    if let Some(regex) = style_re {
        let mut combined = ts_content.to_string();
        for captures in regex.captures_iter(ts_content) {
            if let Some(style_path) = captures.get(1) {
                let ts_dir = Path::new(filename).parent().unwrap_or(Path::new("."));
                let style_file = ts_dir.join(style_path.as_str());
                if let Ok(style_content) = fs::read_to_string(&style_file) {
                    combined.push_str("\n---STYLE---\n");
                    combined.push_str(&style_content);
                }
            }
        }
        return combined;
    }

    ts_content.to_string()
}

/// Get cache directory, creating it if necessary
fn get_cache_dir(subdir: &str) -> PathBuf {
    // Find project root by looking for package.json going up from cwd
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut project_root = cwd.clone();

    // Walk up to find package.json
    loop {
        if project_root.join("package.json").exists() {
            break;
        }
        if !project_root.pop() {
            // Fallback to cwd if no package.json found
            project_root = cwd;
            break;
        }
    }

    let cache_dir = project_root.join(CACHE_DIR_NAME).join(subdir);
    if !cache_dir.exists() {
        let _ = fs::create_dir_all(&cache_dir);
    }
    cache_dir
}

// ============ Cached Data Structures ============
#[derive(Serialize, Deserialize)]
struct CachedCompileResult {
    code: String,
    diagnostics: Vec<CachedDiagnostic>,
}

#[derive(Serialize, Deserialize)]
struct CachedDiagnostic {
    file: Option<String>,
    message: String,
    code: u32,
    start: Option<u32>,
    length: Option<u32>,
}

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
        eprintln!("[Rust Binding] Writing to memory: {}", path);
        captured.insert(path.clone(), data.to_vec());
        Ok(())
    }
    fn remove_file(&self, path: &AbsoluteFsPath) -> io::Result<()> {
        let mut captured = self.captured_files.lock().unwrap();
        captured.remove(path);
        Ok(())
    }
    fn symlink(&self, _target: &AbsoluteFsPath, _path: &AbsoluteFsPath) -> io::Result<()> {
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

#[napi(object)]
pub struct FileEntry {
    pub filename: String,
    pub content: String,
}

#[napi(object)]
pub struct BatchEntryResult {
    pub filename: String,
    pub code: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}

#[napi(object)]
pub struct NapiBundleResult {
    pub bundle_js: String,
    pub styles_css: Option<String>,
    pub scripts_js: Option<String>,
    pub index_html: Option<String>,
    pub files: HashMap<String, String>,
    pub chunks: HashMap<String, String>,
}

#[napi]
pub struct Compiler {
    compiler_cache_dir: PathBuf,
    linker_cache_dir: PathBuf,
}

#[napi]
impl Compiler {
    #[napi(constructor)]
    pub fn new() -> Self {
        let compiler_cache_dir = get_cache_dir(COMPILER_CACHE_SUBDIR);
        let linker_cache_dir = get_cache_dir(LINKER_CACHE_SUBDIR);

        eprintln!(
            "[Rust NGC] Cache dir: {}",
            compiler_cache_dir.parent().unwrap().display()
        );

        Compiler {
            compiler_cache_dir,
            linker_cache_dir,
        }
    }

    /// Read cached compile result from disk
    fn read_compiler_cache(&self, _hash: &str) -> Option<CompileResult> {
        // CACHE DISABLED TEMPORARILY
        None
    }

    /// Write compile result to disk cache
    fn write_compiler_cache(&self, _hash: &str, _result: &CompileResult) {
        // CACHE DISABLED TEMPORARILY
    }

    /// Read cached linker result from disk
    fn read_linker_cache(&self, _hash: &str) -> Option<String> {
        // CACHE DISABLED TEMPORARILY
        None
    }

    /// Write linker result to disk cache
    fn write_linker_cache(&self, _hash: &str, _result: &str) {
        // CACHE DISABLED TEMPORARILY
    }

    #[napi]
    pub fn compile(&self, filename: String, content: String) -> CompileResult {
        // 1. Compute hash of content (including template and style files)
        let combined_content = get_combined_content_for_hash(&filename, &content);
        let hash = compute_hash(&combined_content);

        // 2. Check cache
        if let Some(cached) = self.read_compiler_cache(&hash) {
            return cached;
        }

        // 3. Setup Capturing FileSystem
        let fs = CapturingFileSystem::new();
        let abs_filename_str = fs.resolve(&[&filename]).to_string();
        let abs_filename = AbsoluteFsPath::from(Path::new(&abs_filename_str));
        fs.write_file(&abs_filename, content.as_bytes(), None).ok();

        // 4. Setup Compiler Options
        let mut options = NgCompilerOptions::default();
        options.project = abs_filename_str.clone();
        options.out_dir = Some(fs.dirname(&abs_filename_str));

        // 5. Create Program
        let root_names = vec![abs_filename_str.clone()];
        let mut program = NgtscProgram::new(root_names, options, &fs);

        // 6. Load NG Structure
        let mut diagnostics = Vec::new();
        if let Err(e) = program.load_ng_structure(Path::new("/")) {
            return CompileResult {
                code: format!("/* Error loading: {} */", e),
                diagnostics: vec![],
            };
        }

        // 7. Emit
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

        // 8. Retrieve output from Memory
        let output_path_str = abs_filename_str.replace(".ts", ".js");
        let output_path = AbsoluteFsPath::from(Path::new(&output_path_str));

        let code = match fs.read_file(&output_path) {
            Ok(js_content) => js_content,
            Err(_) => {
                format!("/* Output not found in memory for {} */", output_path_str)
            }
        };

        let result = CompileResult { code, diagnostics };

        // 9. Write to cache
        self.write_compiler_cache(&hash, &result);

        result
    }

    #[napi]
    pub fn link_file(&self, filename: String, source_code: String) -> String {
        // 1. Compute hash of source code
        let hash = compute_hash(&source_code);

        // 2. Check cache
        if let Some(cached) = self.read_linker_cache(&hash) {
            return cached;
        }

        // 3. Link
        use angular_compiler_cli::linker::napi::link_file;

        let result = match link_file(source_code, filename) {
            Ok(code) => code,
            Err(e) => format!("/* Linker Error: {} */", e),
        };

        // 4. Write to cache (only if successful)
        if !result.starts_with("/* Linker Error") {
            self.write_linker_cache(&hash, &result);
        }

        result
    }

    #[napi]
    pub fn compile_batch(&self, files: Vec<FileEntry>) -> Vec<BatchEntryResult> {
        if files.is_empty() {
            return vec![];
        }

        // 1. Setup Capturing FileSystem & Root Names
        let fs = CapturingFileSystem::new();
        let mut root_names = Vec::new();
        let mut file_map = HashMap::new();

        for file in &files {
            let abs_filename_str = fs.resolve(&[&file.filename]).to_string();
            let abs_filename = AbsoluteFsPath::from(Path::new(&abs_filename_str));

            // Write to virtual FS
            fs.write_file(&abs_filename, file.content.as_bytes(), None)
                .ok();

            root_names.push(abs_filename_str.clone());
            file_map.insert(abs_filename_str, &file.filename);
        }

        // 2. Setup Compiler Options
        let mut options = NgCompilerOptions::default();
        if !root_names.is_empty() {
            // Find common root
            let mut common_root = fs.dirname(&root_names[0]);
            for path in &root_names[1..] {
                while !path.starts_with(&common_root) {
                    let parent = fs.dirname(&common_root);
                    if parent == common_root {
                        break;
                    } // Reached root
                    common_root = parent;
                }
            }
            eprintln!("[Rust Binding] Common root: {}", common_root);
            options.out_dir = Some(common_root.clone());
            options.root_dir = Some(common_root.clone());
            options.project = root_names[0].clone();
        }
        // Ensure we compile all inputs - default behavior is sufficient

        // 3. Create Program (ONCE)
        let mut program = NgtscProgram::new(root_names.clone(), options, &fs);

        // 4. Load NG Structure
        let mut global_diagnostics = Vec::new();
        if let Err(e) = program.load_ng_structure(Path::new("/")) {
            return files
                .iter()
                .map(|f| BatchEntryResult {
                    filename: f.filename.clone(),
                    code: None,
                    diagnostics: vec![Diagnostic {
                        file: None,
                        message: format!("Global Load Error: {}", e),
                        code: 0,
                        start: None,
                        length: None,
                    }],
                })
                .collect();
        }

        // 5. Emit
        let emit_result = program.emit();

        if let Ok(diags) = &emit_result {
            for diag in diags {
                global_diagnostics.push(Diagnostic {
                    file: diag.file.as_ref().map(|p| p.to_string_lossy().to_string()),
                    message: diag.message.clone(),
                    code: diag.code as u32,
                    start: diag.start.map(|s| s as u32),
                    length: diag.length.map(|l| l as u32),
                });
            }
        } else if let Err(e) = emit_result {
            return files
                .iter()
                .map(|f| BatchEntryResult {
                    filename: f.filename.clone(),
                    code: None,
                    diagnostics: vec![Diagnostic {
                        file: None,
                        message: format!("Global Emit Error: {}", e),
                        code: 0,
                        start: None,
                        length: None,
                    }],
                })
                .collect();
        }

        // 6. Collect Results
        let mut results = Vec::new();

        let mut file_diagnostics: HashMap<String, Vec<Diagnostic>> = HashMap::new();
        for diag in global_diagnostics {
            if let Some(ref f) = diag.file {
                file_diagnostics.entry(f.clone()).or_default().push(diag);
            }
        }

        for (abs_path_str, original_filename) in file_map {
            let output_path_str = abs_path_str.replace(".ts", ".js");
            let output_path = AbsoluteFsPath::from(Path::new(&output_path_str));

            let code = match fs.read_file(&output_path) {
                Ok(c) => Some(c),
                Err(_) => {
                    eprintln!("[Rust Binding] Failed to read from memory: {}", output_path);
                    None
                }
            };
            let diags = file_diagnostics.remove(&abs_path_str).unwrap_or_default();

            results.push(BatchEntryResult {
                filename: original_filename.clone(),
                code,
                diagnostics: diags,
            });
        }

        results
    }

    #[napi]
    pub fn get_dependencies(&self, entry_file: String) -> Vec<String> {
        use angular_compiler_cli::ngtsc::dependency::resolution::resolve_dependencies;
        use std::path::Path;

        match resolve_dependencies(Path::new(&entry_file)) {
            Ok(paths) => paths
                .into_iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            Err(e) => {
                eprintln!("Failed to resolve dependencies: {}", e);
                vec![]
            }
        }
    }
    #[napi]
    pub fn bundle(&self, project_path: String) -> NapiBundleResult {
        use angular_compiler_cli::bundler::bundle_project;
        use std::path::Path;

        match bundle_project(Path::new(&project_path)) {
            Ok(res) => NapiBundleResult {
                bundle_js: res.bundle_js,
                styles_css: res.styles_css,
                scripts_js: res.scripts_js,
                index_html: res.index_html,
                files: res.files,
                chunks: res.chunks,
            },
            Err(e) => {
                eprintln!("Bundle Error: {}", e);
                NapiBundleResult {
                    bundle_js: format!("/* Bundle Error: {} */", e),
                    styles_css: None,
                    scripts_js: None,
                    index_html: None,
                    files: HashMap::new(),
                    chunks: HashMap::new(),
                }
            }
        }
    }
}
