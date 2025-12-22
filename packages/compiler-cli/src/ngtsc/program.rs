use std::path::Path;
use crate::ngtsc::core::NgCompilerOptions;
// use crate::compiler::CompilationResult; // Removed to resolve conflict with ngtsc::core::CompilationResult
// Let's use the one from ngtsc::core if exported, or fully qualify.
// Actually, let's remove this import and use the one NgCompiler uses.
// But NgCompiler uses `angular_compiler::CompilationResult` typically?
// The error: expected `angular_compiler::CompilationResult`, found `core::compiler::CompilationResult`
// This means `self.result` expects `angular_compiler` (from crate::compiler re-export?), but `res` (from NgCompiler) is `core`.
// I should change `program.rs` to use `crate::ngtsc::core::compiler::CompilationResult` (if that's what core one is).
// Or check where NgCompiler comes from.
// Import:
use crate::ngtsc::core::{NgCompiler, CompilationTicket, CompilationTicketKind, CompilationResult};
use crate::ngtsc::file_system::FileSystem;

pub struct NgtscProgram<'a, T: FileSystem> {
    root_names: Vec<String>,
    options: NgCompilerOptions,
    compiler: NgCompiler<'a, T>,
    result: Option<CompilationResult>,
}

impl<'a, T: FileSystem> NgtscProgram<'a, T> {
    pub fn new(root_names: Vec<String>, options: NgCompilerOptions, fs: &'a T) -> Self {
        let ticket = CompilationTicket {
            kind: CompilationTicketKind::Fresh,
            options: options.clone(),
            fs,
        };
        let compiler = NgCompiler::new(ticket);

        NgtscProgram {
            root_names,
            options,
            compiler,
            result: None,
        }
    }

    pub fn load_ng_structure(&mut self, _path: &Path) -> Result<(), String> {
        // We trigger analysis with the root files we know about
        let res = self.compiler.analyze_async(&self.root_names)?;
        self.result = Some(res);
        Ok(())
    }

    pub fn emit(&self) -> Result<(), String> {
        // Ensure analysis happens if not already done (simplified)
        // In reality, load_ng_structure is called before emit.
        if let Some(result) = &self.result {
             self.compiler.emit(result)
        } else {
            Err("Compilation result not available. Did you call load_ng_structure?".to_string())
        }
    }
}

