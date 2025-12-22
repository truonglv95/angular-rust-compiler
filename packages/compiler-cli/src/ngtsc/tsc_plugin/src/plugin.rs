// TSC Plugin
//
// Corresponds to NgTscPlugin class in tsc_plugin.ts

use std::collections::HashSet;

use super::host::{PluginCompilerHost, SimplePluginCompilerHost};

/// Compilation setup result containing files to ignore for diagnostics and emit.
#[derive(Debug, Clone, Default)]
pub struct CompilationSetupResult {
    /// Source files to ignore for diagnostics.
    pub ignore_for_diagnostics: HashSet<String>,
    /// Source files to ignore for emit.
    pub ignore_for_emit: HashSet<String>,
}

/// TSC Plugin trait - mirrors the TscPlugin interface from tsc_plugin.ts
pub trait TscPlugin {
    /// Plugin name.
    fn name(&self) -> &str;
    
    /// Wrap a compiler host with Angular-specific functionality.
    fn wrap_host(
        &mut self,
        host: Box<dyn PluginCompilerHost>,
        input_files: Vec<String>,
        options: CompilerOptions,
    ) -> Box<dyn PluginCompilerHost>;
    
    /// Setup the compilation.
    fn setup_compilation(
        &mut self,
        program: &Program,
        old_program: Option<&Program>,
    ) -> CompilationSetupResult;
    
    /// Get diagnostics for a file (or all files if None).
    fn get_diagnostics(&self, file: Option<&str>) -> Vec<Diagnostic>;
    
    /// Get option diagnostics.
    fn get_option_diagnostics(&self) -> Vec<Diagnostic>;
    
    /// Get the next program after transformations.
    fn get_next_program(&self) -> &Program;
    
    /// Create custom transformers for the emit phase.
    fn create_transformers(&self) -> CustomTransformers;
}

/// Compiler options (placeholder).
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {
    pub strict: bool,
    pub strict_templates: bool,
    pub full_template_type_check: bool,
    pub skip_lib_check: bool,
    pub target: String,
    pub module: String,
    pub base_url: Option<String>,
    pub paths: Option<std::collections::HashMap<String, Vec<String>>>,
}

/// Program representation (placeholder).
#[derive(Debug, Clone, Default)]
pub struct Program {
    pub source_files: Vec<String>,
}

/// Diagnostic message.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub category: DiagnosticCategory,
    pub code: i32,
    pub file: Option<String>,
    pub start: Option<usize>,
    pub length: Option<usize>,
    pub message: String,
}

/// Diagnostic category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCategory {
    Warning,
    Error,
    Suggestion,
    Message,
}

/// Custom transformers for emit.
#[derive(Debug, Clone, Default)]
pub struct CustomTransformers {
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub after_declarations: Vec<String>,
}

/// Angular TypeScript Compiler Plugin.
/// A plugin for `tsc_wrapped` which allows Angular compilation from a plain `ts_library`.
pub struct NgTscPlugin {
    /// Plugin name.
    name: String,
    /// Angular compiler options.
    ng_options: NgCompilerOptions,
    /// The wrapped host.
    host: Option<Box<dyn PluginCompilerHost>>,
    /// Compiler options.
    options: Option<CompilerOptions>,
    /// The current program.
    program: Option<Program>,
    /// Files to ignore for diagnostics.
    ignore_for_diagnostics: HashSet<String>,
    /// Files to ignore for emit.
    ignore_for_emit: HashSet<String>,
}

/// Angular-specific compiler options.
#[derive(Debug, Clone, Default)]
pub struct NgCompilerOptions {
    pub strict_injection_parameters: bool,
    pub strict_input_access_modifiers: bool,
    pub strict_templates: bool,
    pub flat_module_out_file: Option<String>,
    pub flat_module_id: Option<String>,
    pub generate_code_for_libraries: bool,
    pub enable_ivy: bool,
    pub full_template_type_check: bool,
    pub use_incremental_api: bool,
    pub extend_angular_core_with_rxjs: bool,
    pub allow_empty_codegen_files: bool,
    pub disable_expression_lowering: bool,
    pub i18n_out_file: Option<String>,
    pub i18n_out_format: Option<String>,
    pub i18n_out_locale: Option<String>,
    pub i18n_use_external_ids: bool,
    pub compilation_mode: CompilationMode,
}

/// Compilation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompilationMode {
    #[default]
    Full,
    Partial,
    Local,
}

impl NgTscPlugin {
    /// Create a new NgTscPlugin with the given options.
    pub fn new(ng_options: NgCompilerOptions) -> Self {
        Self {
            name: "ngtsc".to_string(),
            ng_options,
            host: None,
            options: None,
            program: None,
            ignore_for_diagnostics: HashSet::new(),
            ignore_for_emit: HashSet::new(),
        }
    }
    
    /// Get the compiler (panics if setupCompilation hasn't been called).
    pub fn compiler(&self) -> Result<&Program, String> {
        self.program.as_ref().ok_or_else(|| {
            "Lifecycle error: setupCompilation() must be called first.".to_string()
        })
    }
}

impl TscPlugin for NgTscPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn wrap_host(
        &mut self,
        host: Box<dyn PluginCompilerHost>,
        input_files: Vec<String>,
        options: CompilerOptions,
    ) -> Box<dyn PluginCompilerHost> {
        self.options = Some(options.clone());
        
        // Wrap the host with Angular-specific functionality
        let wrapped = SimplePluginCompilerHost::new(
            input_files,
            &host.get_current_directory(),
        );
        
        let boxed: Box<dyn PluginCompilerHost> = Box::new(wrapped);
        self.host = Some(Box::new(SimplePluginCompilerHost::new(
            host.input_files().to_vec(),
            &host.get_current_directory(),
        )));
        boxed
    }
    
    fn setup_compilation(
        &mut self,
        program: &Program,
        old_program: Option<&Program>,
    ) -> CompilationSetupResult {
        if self.host.is_none() || self.options.is_none() {
            panic!("Lifecycle error: setupCompilation() before wrapHost().");
        }
        
        self.program = Some(program.clone());
        
        // In the TS implementation, this would:
        // 1. Create a PerfRecorder
        // 2. Create a TsCreateProgramDriver
        // 3. Create a PatchedProgramIncrementalBuildStrategy
        // 4. Determine modified resource files
        // 5. Create a fresh or incremental compilation ticket
        // 6. Create the NgCompiler from the ticket
        
        // For now, return empty sets
        CompilationSetupResult {
            ignore_for_diagnostics: self.ignore_for_diagnostics.clone(),
            ignore_for_emit: self.ignore_for_emit.clone(),
        }
    }
    
    fn get_diagnostics(&self, file: Option<&str>) -> Vec<Diagnostic> {
        // In the TS implementation, this delegates to compiler.getDiagnostics()
        // or compiler.getDiagnosticsForFile()
        Vec::new()
    }
    
    fn get_option_diagnostics(&self) -> Vec<Diagnostic> {
        // In the TS implementation, this delegates to compiler.getOptionDiagnostics()
        Vec::new()
    }
    
    fn get_next_program(&self) -> &Program {
        self.program.as_ref().expect("setupCompilation() must be called first")
    }
    
    fn create_transformers(&self) -> CustomTransformers {
        // In the TS implementation, this:
        // 1. Updates the perf recorder phase to TypeScriptEmit
        // 2. Returns compiler.prepareEmit().transformers
        CustomTransformers::default()
    }
}
