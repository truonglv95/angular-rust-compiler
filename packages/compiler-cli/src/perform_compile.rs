//! Perform Compile
//!
//! Corresponds to packages/compiler-cli/src/perform_compile.ts
//! Config parsing and compilation entry point.

use std::path::Path;
use crate::ngtsc::core::NgCompilerOptions;
use crate::ngtsc::program::NgtscProgram;
use crate::ngtsc::file_system::NodeJSFileSystem;

#[derive(Debug)]
pub struct PerformCompileResult {
    pub diagnostics: Vec<String>,
    pub program: Option<()>, 
    pub emit_result: Option<()>,
}

pub fn perform_compilation(
    project: Option<&str>,
    _root_names: Option<Vec<String>>,
    _options: Option<NgCompilerOptions>,
) -> PerformCompileResult {
    println!("Performing compilation...");
    
    let fs = NodeJSFileSystem::new();
    let root_names = if let Some(p) = project {
        println!("Using project file: {}", p);
        // TODO: Parse tsconfig to get root names
        vec!["src/main.ts".to_string()] 
    } else {
        vec![]
    };

    let options = NgCompilerOptions::default();
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
