use std::path::{Path, PathBuf};

use super::capturing_fs::CapturingFileSystem;
use crate::ngtsc::core::NgCompilerOptions;
use crate::ngtsc::file_system::NodeJSFileSystem;
use crate::ngtsc::program::NgtscProgram;
use std::time::Instant;

pub fn parallel_compile(
    files: &[PathBuf],
    project_path: &Path,
) -> anyhow::Result<Vec<(PathBuf, String)>> {
    let start = Instant::now();
    println!(
        "Compiling {} files in parallel (via NgCompiler)...",
        files.len()
    );

    let root_names: Vec<String> = files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let options = NgCompilerOptions {
        project: project_path.to_string_lossy().to_string(),
        out_dir: Some("dist".to_string()), // Virtual output dir
        ..NgCompilerOptions::default()
    };

    // Use CapturingFileSystem to intercept writes
    let fs = NodeJSFileSystem::new(); // case-sensitive by default or system dependent
    let capturing_fs = CapturingFileSystem::new(fs);

    let mut program = NgtscProgram::new(root_names, options, &capturing_fs);

    // Initial analysis
    eprintln!("Analyzing structure... Files: {}", files.len());
    if !files.is_empty() {
        eprintln!("First file: {:?}", files[0]);
    }
    program
        .load_ng_structure(project_path)
        .map_err(|e| anyhow::anyhow!(e))?;

    // Emit (compilation)
    eprintln!("Emitting code...");

    let diagnostics = program.emit().map_err(|e| anyhow::anyhow!(e))?;

    if !diagnostics.is_empty() {
        for diag in diagnostics {
            eprintln!("Diagnostic: {:?}", diag);
        }
    }

    eprintln!("Compilation finished in {:?}", start.elapsed());

    // Collect outputs from capturing_fs
    let mut results = Vec::new();
    let files_map = capturing_fs.files.lock().unwrap();

    // Sort output for deterministic bundling (optional but good)
    let mut keys: Vec<_> = files_map.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(content) = files_map.get(key) {
            results.push((key.clone(), content.clone()));
        }
    }

    // Fallback: if no files emitted (e.g. all errors), results is empty.

    Ok(results)
}
