//! Perform Compile
//!
//! Corresponds to packages/compiler-cli/src/perform_compile.ts
//! Config parsing and compilation entry point.

use std::collections::HashSet;
use std::path::Path;
use crate::ngtsc::core::NgCompilerOptions;
use crate::ngtsc::program::NgtscProgram;
use crate::ngtsc::file_system::NodeJSFileSystem;
use crate::transformers::api::{CompilerOptions, Diagnostic, DiagnosticCategory};

/// Emit flags for controlling output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EmitFlags(u32);

impl EmitFlags {
    pub const DEFAULT: EmitFlags = EmitFlags(0);
    pub const DTS_ONLY: EmitFlags = EmitFlags(1);
    pub const JS: EmitFlags = EmitFlags(2);
    pub const METADATA: EmitFlags = EmitFlags(4);
    pub const ALL: EmitFlags = EmitFlags(7);
}

/// Parsed configuration from tsconfig.json.
#[derive(Debug, Clone, Default)]
pub struct ParsedConfiguration {
    /// Path to the project (tsconfig.json).
    pub project: String,
    /// Root source file names.
    pub root_names: Vec<String>,
    /// Compiler options.
    pub options: CompilerOptions,
    /// Configuration errors.
    pub errors: Vec<Diagnostic>,
    /// Emit flags.
    pub emit_flags: EmitFlags,
}

/// Compilation result.
#[derive(Debug)]
pub struct CompilationResult {
    /// Diagnostics from compilation.
    pub diagnostics: Vec<Diagnostic>,
    /// The program (if compilation succeeded).
    pub program: Option<Program>,
}

/// Program representation.
#[derive(Debug, Clone, Default)]
pub struct Program {
    /// Source files in the program.
    pub source_files: Vec<String>,
}

/// Old result structure for backward compatibility.
#[derive(Debug)]
pub struct PerformCompileResult {
    pub diagnostics: Vec<String>,
    pub program: Option<()>, 
    pub emit_result: Option<()>,
}

/// Read configuration from project file.
pub fn read_configuration(project: &str, cmd_options: Option<CompilerOptions>) -> ParsedConfiguration {
    let project_path = Path::new(project);
    let mut root_names = Vec::new();
    let mut errors = Vec::new();
    
    // Determine the tsconfig file path and base directory
    let (tsconfig_path, base_dir) = if project_path.is_dir() {
        let tsconfig = project_path.join("tsconfig.json");
        (tsconfig, project_path.to_path_buf())
    } else if project_path.exists() && project_path.extension().map_or(false, |ext| ext == "json") {
        let base = project_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        (project_path.to_path_buf(), base)
    } else {
        errors.push(Diagnostic {
            category: DiagnosticCategory::Error,
            code: -1,
            message: format!("Project path does not exist or is not a valid tsconfig: {}", project),
            file: None,
            start: None,
            length: None,
        });
        return ParsedConfiguration {
            project: project.to_string(),
            root_names,
            options: cmd_options.unwrap_or_default(),
            errors,
            emit_flags: EmitFlags::DEFAULT,
        };
    };
    
    // Parse tsconfig.json
    if tsconfig_path.exists() {
        match std::fs::read_to_string(&tsconfig_path) {
            Ok(content) => {
                // Strip comments for JSON5 compatibility (simplified)
                let content = strip_json_comments(&content);
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(config) => {
                        // Get include patterns (default to ["**/*.ts"])
                        let include_patterns: Vec<String> = config.get("include")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_else(|| vec!["**/*.ts".to_string()]);
                        
                        // Get exclude patterns (default to ["node_modules"])
                        let exclude_patterns: Vec<String> = config.get("exclude")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_else(|| vec!["**/node_modules/**".to_string()]);
                        
                        // Get explicit files list if specified
                        let files: Vec<String> = config.get("files")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        
                        // If files is specified, use it; otherwise glob from include patterns
                        if !files.is_empty() {
                            for file in files {
                                let file_path = base_dir.join(&file);
                                if file_path.exists() {
                                    root_names.push(file_path.to_string_lossy().to_string());
                                }
                            }
                        } else {
                            // Use glob to find files matching include patterns
                            let discovered = discover_files(&base_dir, &include_patterns, &exclude_patterns);
                            root_names = discovered;
                        }
                        
                        println!("Discovered {} TypeScript files", root_names.len());
                    }
                    Err(e) => {
                        errors.push(Diagnostic {
                            category: DiagnosticCategory::Error,
                            code: -1,
                            message: format!("Failed to parse tsconfig.json: {}", e),
                            file: Some(tsconfig_path.to_string_lossy().to_string()),
                            start: None,
                            length: None,
                        });
                    }
                }
            }
            Err(e) => {
                errors.push(Diagnostic {
                    category: DiagnosticCategory::Error,
                    code: -1,
                    message: format!("Failed to read tsconfig.json: {}", e),
                    file: Some(tsconfig_path.to_string_lossy().to_string()),
                    start: None,
                    length: None,
                });
            }
        }
    } else {
        errors.push(Diagnostic {
            category: DiagnosticCategory::Error,
            code: -1,
            message: format!("Cannot find tsconfig.json at {}", tsconfig_path.display()),
            file: None,
            start: None,
            length: None,
        });
    }
    
    ParsedConfiguration {
        project: project.to_string(),
        root_names,
        options: cmd_options.unwrap_or_default(),
        errors,
        emit_flags: EmitFlags::DEFAULT,
    }
}

/// Strip JSON comments (simple implementation for single-line comments)
fn strip_json_comments(input: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut prev_char = '\0';
    
    for line in input.lines() {
        let trimmed = line.trim();
        // Skip lines that start with // or /*
        if !trimmed.starts_with("//") && !trimmed.starts_with("/*") {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

/// Discover files matching include patterns and excluding exclude patterns
fn discover_files(base_dir: &std::path::Path, include: &[String], exclude: &[String]) -> Vec<String> {
    let mut files = Vec::new();
    
    for pattern in include {
        let full_pattern = base_dir.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();
        
        match glob::glob(&pattern_str) {
            Ok(paths) => {
                for entry in paths {
                    match entry {
                        Ok(path) => {
                            let path_str = path.to_string_lossy().to_string();
                            
                            // Check if path matches any exclude pattern
                            let should_exclude = exclude.iter().any(|excl| {
                                // Special case for node_modules to ensure efficient and correct exclusion
                                if excl.contains("node_modules") && path_str.contains("node_modules") {
                                    return true;
                                }

                                let excl_pattern = base_dir.join(excl);
                                let excl_str = excl_pattern.to_string_lossy();
                                match glob::Pattern::new(&excl_str) {
                                    Ok(p) => p.matches(&path_str),
                                    Err(_) => path_str.contains(excl),
                                }
                            });
                            
                            if !should_exclude && path.is_file() {
                                files.push(path_str);
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Invalid glob pattern '{}': {}", pattern, e);
            }
        }
    }
    
    files
}

/// Perform compilation with full options.
pub fn perform_compilation(
    root_names: Vec<String>,
    _options: CompilerOptions,
    _emit_flags: EmitFlags,
    _old_program: Option<Program>,
    _custom_transformers: Option<crate::transformers::api::CustomTransformers>,
    _modified_resource_files: Option<HashSet<String>>,
) -> CompilationResult {
    println!("Performing compilation with {} root files...", root_names.len());
    
    let fs = NodeJSFileSystem::new();
    let ng_options = NgCompilerOptions::default();
    let mut program = NgtscProgram::new(root_names.clone(), ng_options, &fs);

    let mut diagnostics = Vec::new();
    
    // Trigger analysis
    if let Err(e) = program.load_ng_structure(Path::new(".")) {
        diagnostics.push(Diagnostic {
            category: DiagnosticCategory::Error,
            code: -1,
            message: e,
            file: None,
            start: None,
            length: None,
        });
        return CompilationResult {
            diagnostics,
            program: None,
        };
    }

    if let Err(e) = program.emit() {
        diagnostics.push(Diagnostic {
            category: DiagnosticCategory::Error,
            code: -1,
            message: e,
            file: None,
            start: None,
            length: None,
        });
        return CompilationResult {
            diagnostics,
            program: None,
        };
    }

    CompilationResult {
        diagnostics,
        program: Some(Program {
            source_files: root_names,
        }),
    }
}

/// Simple compilation entry point.
pub fn perform_compilation_simple(
    project: Option<&str>,
    _root_names: Option<Vec<String>>,
    _options: Option<NgCompilerOptions>,
) -> PerformCompileResult {
    println!("Performing compilation...");
    
    let fs = NodeJSFileSystem::new();
    
    // Parse tsconfig.json and discover files automatically
    let (root_names, options) = if let Some(p) = project {
        println!("Using project file: {}", p);
        let parsed = read_configuration(p, None);
        
        if !parsed.errors.is_empty() {
            for err in &parsed.errors {
                eprintln!("Configuration error: {}", err.message);
            }
        }
        
        // Get outDir from tsconfig if available
        let mut opts = NgCompilerOptions::default();
        
        // Parse tsconfig again to get compilerOptions
        let tsconfig_path = Path::new(p);
        if let Ok(content) = std::fs::read_to_string(tsconfig_path) {
            let content = strip_json_comments(&content);
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(compiler_opts) = config.get("compilerOptions") {
                    if let Some(out_dir) = compiler_opts.get("outDir").and_then(|v| v.as_str()) {
                        // Resolve outDir relative to tsconfig location
                        let base_dir = tsconfig_path.parent().unwrap_or(Path::new("."));
                        let resolved_out_dir = base_dir.join(out_dir);
                        opts.out_dir = Some(resolved_out_dir.to_string_lossy().to_string());
                    }
                }
            }
        }
        
        // Default to rust-output if no outDir is specified
        if opts.out_dir.is_none() {
             let base_dir = Path::new(p).parent().unwrap_or(Path::new("."));
             let resolved = base_dir.join("rust-output");
            opts.out_dir = Some(resolved.to_string_lossy().to_string());
        }
        
        (parsed.root_names, opts)
    } else {
        (vec![], NgCompilerOptions::default())
    };
    
    let mut program = NgtscProgram::new(root_names, options, &fs);

    // Trigger analysis
    if let Err(e) = program.load_ng_structure(Path::new(".")) {
        return PerformCompileResult {
            diagnostics: vec![e],
            program: None,
            emit_result: None,
        };
    }

    if let Err(e) = program.emit() {
        return PerformCompileResult {
            diagnostics: vec![e],
            program: None,
            emit_result: None,
        };
    }

    PerformCompileResult {
        diagnostics: vec![],
        program: Some(()),
        emit_result: Some(()),
    }
}

/// Format diagnostics for display.
pub fn format_diagnostics(diagnostics: &[Diagnostic], _host: &crate::main_entry::FormatDiagnosticsHost) -> String {
    let mut output = String::new();
    for diag in diagnostics {
        let category = match diag.category {
            DiagnosticCategory::Error => "error",
            DiagnosticCategory::Warning => "warning",
            DiagnosticCategory::Suggestion => "suggestion",
            DiagnosticCategory::Message => "message",
        };
        if let Some(file) = &diag.file {
            if let Some(start) = diag.start {
                output.push_str(&format!("{} TS{}: {} ({}:{})\n", category, diag.code, diag.message, file, start));
            } else {
                output.push_str(&format!("{} TS{}: {} ({})\n", category, diag.code, diag.message, file));
            }
        } else {
            output.push_str(&format!("{} TS{}: {}\n", category, diag.code, diag.message));
        }
    }
    output
}

/// Get exit code from compilation result.
pub fn exit_code_from_result(diagnostics: &[Diagnostic]) -> i32 {
    let has_errors = diagnostics.iter().any(|d| d.category == DiagnosticCategory::Error);
    if has_errors { 1 } else { 0 }
}
