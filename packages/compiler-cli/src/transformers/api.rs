// Transformers API
//
// Public API for Angular transformers.

/// Compiler options for Angular compilation.
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {
    /// Base URL for resolving module specifiers.
    pub base_url: Option<String>,
    /// Base path for the project.
    pub base_path: Option<String>,
    /// Root directory for source files.
    pub root_dir: Option<String>,
    /// Output directory for compiled files.
    pub out_dir: Option<String>,
    /// Enable source maps.
    pub source_map: bool,
    /// Enable declaration file generation.
    pub declaration: bool,
    /// Strict null checks.
    pub strict_null_checks: bool,
    /// Full template type check.
    pub full_template_type_check: bool,
    /// Strict templates.
    pub strict_templates: bool,
    /// Enable Ivy.
    pub enable_ivy: bool,
    /// Compilation mode.
    pub compilation_mode: CompilationMode,
    /// i18n options.
    pub i18n: I18nOptions,
    /// i18n input file.
    pub i18n_in_file: Option<String>,
    /// i18n input format.
    pub i18n_in_format: Option<String>,
    /// i18n input locale.
    pub i18n_in_locale: Option<String>,
    /// i18n missing translations handling.
    pub i18n_in_missing_translations: Option<String>,
    /// Locale setting.
    pub locale: Option<String>,
    /// New line kind.
    pub new_line: Option<NewLineKind>,
}

/// New line kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewLineKind {
    LineFeed,
    CarriageReturnLineFeed,
}

/// Compilation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompilationMode {
    /// Full AOT compilation.
    #[default]
    Full,
    /// Partial compilation (library mode).
    Partial,
    /// Local compilation (for faster iteration).
    Local,
}

/// i18n options.
#[derive(Debug, Clone, Default)]
pub struct I18nOptions {
    /// Format for i18n messages.
    pub format: Option<I18nFormat>,
    /// Output file for messages.
    pub out_file: Option<String>,
    /// Locale.
    pub locale: Option<String>,
}

/// i18n message format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I18nFormat {
    Xlf,
    Xlf2,
    Xmb,
    Json,
}

/// Diagnostic message.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Message category.
    pub category: DiagnosticCategory,
    /// Message code.
    pub code: i32,
    /// Message text.
    pub message: String,
    /// File path.
    pub file: Option<String>,
    /// Start position.
    pub start: Option<usize>,
    /// Length.
    pub length: Option<usize>,
}

/// Diagnostic category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCategory {
    Warning,
    Error,
    Suggestion,
    Message,
}

/// Program interface.
pub trait Program {
    /// Get all source files.
    fn get_source_files(&self) -> Vec<String>;
    
    /// Emit compiled output.
    fn emit(&self) -> EmitResult;
    
    /// Get diagnostics.
    fn get_diagnostics(&self) -> Vec<Diagnostic>;
}

/// Result of emit operation.
#[derive(Debug, Clone, Default)]
pub struct EmitResult {
    /// Whether emit was skipped.
    pub emit_skipped: bool,
    /// Diagnostics from emit.
    pub diagnostics: Vec<Diagnostic>,
    /// Emitted files.
    pub emitted_files: Vec<String>,
}

/// Custom transformers for the compilation.
#[derive(Debug, Clone, Default)]
pub struct CustomTransformers {
    /// Transformers to run before the main emit.
    pub before: Vec<String>,
    /// Transformers to run after the main emit.
    pub after: Vec<String>,
    /// Transformers to run after declaration emit.
    pub after_declarations: Vec<String>,
}

