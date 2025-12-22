// Main Entry Point
//
// Corresponds to packages/compiler-cli/src/main.ts
//
// This module provides the main entry point for the Angular compiler CLI.

use std::collections::HashSet;

use crate::perform_compile::{
    exit_code_from_result, format_diagnostics, perform_compilation, read_configuration,
    ParsedConfiguration, CompilationResult, EmitFlags, Program,
};
use crate::perform_watch::{create_perform_watch_host, perform_watch_compilation, WatchResult};
use crate::transformers::api::{CompilerOptions, CustomTransformers, Diagnostic, DiagnosticCategory, NewLineKind};

/// Parsed configuration for ngc with watch mode support.
#[derive(Debug, Clone, Default)]
pub struct NgcParsedConfiguration {
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
    /// Whether to run in watch mode.
    pub watch: bool,
}

impl From<ParsedConfiguration> for NgcParsedConfiguration {
    fn from(config: ParsedConfiguration) -> Self {
        Self {
            project: config.project,
            root_names: config.root_names,
            options: config.options,
            errors: config.errors,
            emit_flags: config.emit_flags,
            watch: false,
        }
    }
}

/// Result from main diagnostics test.
#[derive(Debug)]
pub struct MainDiagnosticsResult {
    pub exit_code: i32,
    pub diagnostics: Vec<Diagnostic>,
}

/// Program reuse container for incremental compilation.
#[derive(Debug, Default)]
pub struct ProgramReuse {
    pub program: Option<Program>,
}

/// Format diagnostics host for error formatting.
#[derive(Debug)]
pub struct FormatDiagnosticsHost {
    pub current_directory: String,
    pub new_line: String,
}

impl FormatDiagnosticsHost {
    pub fn new(options: Option<&CompilerOptions>) -> Self {
        let current_directory = options
            .and_then(|o| o.base_path.clone())
            .unwrap_or_else(|| std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()));

        let new_line = if let Some(opts) = options {
            if opts.new_line == Some(NewLineKind::LineFeed) {
                "\n".to_string()
            } else if opts.new_line == Some(NewLineKind::CarriageReturnLineFeed) {
                "\r\n".to_string()
            } else {
                "\n".to_string()
            }
        } else {
            "\n".to_string()
        };

        Self {
            current_directory,
            new_line,
        }
    }

    pub fn get_canonical_file_name(&self, file_name: &str) -> String {
        file_name.replace('\\', "/")
    }
}

/// Main entry point for the Angular compiler.
///
/// # Arguments
/// * `args` - Command line arguments
/// * `console_error` - Error output function
/// * `config` - Optional pre-parsed configuration
/// * `custom_transformers` - Optional custom transformers
/// * `program_reuse` - Optional program reuse container for incremental compilation
/// * `modified_resource_files` - Optional set of modified resource files
///
/// # Returns
/// Exit code (0 for success, non-zero for failure)
pub fn main_fn<F>(
    args: &[String],
    console_error: F,
    config: Option<NgcParsedConfiguration>,
    custom_transformers: Option<CustomTransformers>,
    program_reuse: Option<&mut ProgramReuse>,
    modified_resource_files: Option<HashSet<String>>,
) -> i32
where
    F: Fn(&str),
{
    let NgcParsedConfiguration {
        project,
        root_names,
        options,
        errors: config_errors,
        watch,
        emit_flags,
    } = config.unwrap_or_else(|| read_ngc_command_line_and_configuration(args));

    if !config_errors.is_empty() {
        return report_errors_and_exit(&config_errors, Some(&options), &console_error);
    }

    if watch {
        let result = watch_mode(&project, &options, &console_error);
        return report_errors_and_exit(&result.first_compile_result, Some(&options), &console_error);
    }

    let old_program = program_reuse.as_ref().and_then(|pr| pr.program.clone());

    let CompilationResult {
        diagnostics: compile_diags,
        program,
    } = perform_compilation(
        root_names,
        options.clone(),
        emit_flags,
        old_program,
        custom_transformers,
        modified_resource_files,
    );

    if let Some(pr) = program_reuse {
        pr.program = program;
    }

    report_errors_and_exit(&compile_diags, Some(&options), &console_error)
}

/// Main function for testing that returns diagnostics.
pub fn main_diagnostics_for_test(
    args: &[String],
    config: Option<NgcParsedConfiguration>,
    program_reuse: Option<&mut ProgramReuse>,
    modified_resource_files: Option<HashSet<String>>,
) -> MainDiagnosticsResult {
    let NgcParsedConfiguration {
        root_names,
        options,
        errors: config_errors,
        emit_flags,
        ..
    } = config.unwrap_or_else(|| read_ngc_command_line_and_configuration(args));

    if !config_errors.is_empty() {
        return MainDiagnosticsResult {
            exit_code: exit_code_from_result(&config_errors),
            diagnostics: config_errors,
        };
    }

    let old_program = program_reuse.as_ref().and_then(|pr| pr.program.clone());

    let CompilationResult {
        diagnostics: compile_diags,
        program,
    } = perform_compilation(
        root_names,
        options,
        emit_flags,
        old_program,
        None,
        modified_resource_files,
    );

    if let Some(pr) = program_reuse {
        pr.program = program;
    }

    MainDiagnosticsResult {
        exit_code: exit_code_from_result(&compile_diags),
        diagnostics: compile_diags,
    }
}

/// Read ngc command line arguments and configuration.
pub fn read_ngc_command_line_and_configuration(args: &[String]) -> NgcParsedConfiguration {
    let mut options = CompilerOptions::default();

    // Parse command line arguments
    let parsed_args = parse_ngc_args(args);

    if let Some(i18n_file) = &parsed_args.i18n_file {
        options.i18n_in_file = Some(i18n_file.clone());
    }
    if let Some(i18n_format) = &parsed_args.i18n_format {
        options.i18n_in_format = Some(i18n_format.clone());
    }
    if let Some(locale) = &parsed_args.locale {
        options.i18n_in_locale = Some(locale.clone());
    }
    if let Some(missing_translation) = &parsed_args.missing_translation {
        options.i18n_in_missing_translations = Some(missing_translation.clone());
    }

    let config = read_command_line_and_configuration(
        args,
        Some(options),
        &["i18nFile", "i18nFormat", "locale", "missingTranslation", "watch"],
    );

    NgcParsedConfiguration {
        project: config.project,
        root_names: config.root_names,
        options: config.options,
        errors: config.errors,
        emit_flags: config.emit_flags,
        watch: parsed_args.watch,
    }
}

/// Parsed ngc command line arguments.
#[derive(Debug, Default)]
struct NgcArgs {
    i18n_file: Option<String>,
    i18n_format: Option<String>,
    locale: Option<String>,
    missing_translation: Option<String>,
    watch: bool,
}

/// Parse ngc-specific command line arguments.
fn parse_ngc_args(args: &[String]) -> NgcArgs {
    let mut result = NgcArgs::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--i18nFile" | "-i18nFile" => {
                if i + 1 < args.len() {
                    result.i18n_file = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--i18nFormat" | "-i18nFormat" => {
                if i + 1 < args.len() {
                    result.i18n_format = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--locale" | "-locale" => {
                if i + 1 < args.len() {
                    result.locale = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--missingTranslation" | "-missingTranslation" => {
                if i + 1 < args.len() {
                    result.missing_translation = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--watch" | "-w" => {
                result.watch = true;
            }
            _ => {}
        }
        i += 1;
    }
    result
}

/// Read command line and configuration.
pub fn read_command_line_and_configuration(
    args: &[String],
    existing_options: Option<CompilerOptions>,
    ng_cmd_line_options: &[&str],
) -> ParsedConfiguration {
    // Parse TypeScript command line
    let cmd_config = parse_command_line(args);
    let project = cmd_config.project.unwrap_or_else(|| ".".to_string());

    // Filter out Angular-specific errors
    let cmd_errors: Vec<Diagnostic> = cmd_config
        .errors
        .into_iter()
        .filter(|e| {
            let msg = &e.message;
            !ng_cmd_line_options.iter().any(|o| msg.contains(o))
        })
        .collect();

    if !cmd_errors.is_empty() {
        return ParsedConfiguration {
            project,
            root_names: Vec::new(),
            options: cmd_config.options,
            errors: cmd_errors,
            emit_flags: EmitFlags::DEFAULT,
        };
    }

    let config = read_configuration(&project, Some(cmd_config.options));
    let mut options = config.options;

    // Merge with existing options
    if let Some(existing) = existing_options {
        merge_options(&mut options, &existing);
    }

    // Handle locale
    if let Some(locale) = &options.locale {
        options.i18n_in_locale = Some(locale.clone());
    }

    ParsedConfiguration {
        project,
        root_names: config.root_names,
        options,
        errors: config.errors,
        emit_flags: config.emit_flags,
    }
}

/// Parsed command line result.
#[derive(Debug, Default)]
struct CommandLineConfig {
    project: Option<String>,
    options: CompilerOptions,
    errors: Vec<Diagnostic>,
}

/// Parse TypeScript-style command line.
fn parse_command_line(args: &[String]) -> CommandLineConfig {
    let mut result = CommandLineConfig::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "-p" || arg == "--project" {
            if i + 1 < args.len() {
                result.project = Some(args[i + 1].clone());
                i += 1;
            }
        }
        // Add more options as needed
        i += 1;
    }
    result
}

/// Merge two compiler options, preferring the second.
fn merge_options(target: &mut CompilerOptions, source: &CompilerOptions) {
    if source.i18n_in_file.is_some() {
        target.i18n_in_file = source.i18n_in_file.clone();
    }
    if source.i18n_in_format.is_some() {
        target.i18n_in_format = source.i18n_in_format.clone();
    }
    if source.i18n_in_locale.is_some() {
        target.i18n_in_locale = source.i18n_in_locale.clone();
    }
    if source.i18n_in_missing_translations.is_some() {
        target.i18n_in_missing_translations = source.i18n_in_missing_translations.clone();
    }
}

/// Run in watch mode.
pub fn watch_mode<F>(
    project: &str,
    options: &CompilerOptions,
    console_error: &F,
) -> WatchResult
where
    F: Fn(&str),
{
    let host = create_perform_watch_host(
        project,
        |diagnostics| {
            print_diagnostics(diagnostics, Some(options), console_error);
        },
        options.clone(),
    );

    perform_watch_compilation(host)
}

/// Get format diagnostics host.
pub fn get_format_diagnostics_host(options: Option<&CompilerOptions>) -> FormatDiagnosticsHost {
    FormatDiagnosticsHost::new(options)
}

/// Report errors and exit.
fn report_errors_and_exit<F>(
    all_diagnostics: &[Diagnostic],
    options: Option<&CompilerOptions>,
    console_error: &F,
) -> i32
where
    F: Fn(&str),
{
    let errors_and_warnings: Vec<_> = all_diagnostics
        .iter()
        .filter(|d| d.category != DiagnosticCategory::Message)
        .cloned()
        .collect();

    print_diagnostics(&errors_and_warnings, options, console_error);
    exit_code_from_result(all_diagnostics)
}

/// Print diagnostics.
fn print_diagnostics<F>(
    diagnostics: &[Diagnostic],
    options: Option<&CompilerOptions>,
    console_error: &F,
)
where
    F: Fn(&str),
{
    if diagnostics.is_empty() {
        return;
    }
    let format_host = get_format_diagnostics_host(options);
    console_error(&format_diagnostics(diagnostics, &format_host));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ngc_args_empty() {
        let args: Vec<String> = vec![];
        let result = parse_ngc_args(&args);
        assert!(!result.watch);
        assert!(result.i18n_file.is_none());
    }

    #[test]
    fn test_parse_ngc_args_watch() {
        let args = vec!["--watch".to_string()];
        let result = parse_ngc_args(&args);
        assert!(result.watch);
    }

    #[test]
    fn test_parse_ngc_args_watch_short() {
        let args = vec!["-w".to_string()];
        let result = parse_ngc_args(&args);
        assert!(result.watch);
    }

    #[test]
    fn test_parse_ngc_args_i18n() {
        let args = vec![
            "--i18nFile".to_string(),
            "messages.xlf".to_string(),
            "--locale".to_string(),
            "en-US".to_string(),
        ];
        let result = parse_ngc_args(&args);
        assert_eq!(result.i18n_file, Some("messages.xlf".to_string()));
        assert_eq!(result.locale, Some("en-US".to_string()));
    }

    #[test]
    fn test_format_diagnostics_host() {
        let host = FormatDiagnosticsHost::new(None);
        assert_eq!(host.new_line, "\n");
        assert_eq!(host.get_canonical_file_name("foo\\bar.ts"), "foo/bar.ts");
    }

    #[test]
    fn test_ngc_parsed_configuration_from_parsed_configuration() {
        let config = ParsedConfiguration {
            project: "test".to_string(),
            root_names: vec!["main.ts".to_string()],
            options: CompilerOptions::default(),
            errors: vec![],
            emit_flags: EmitFlags::DEFAULT,
        };
        let ngc_config: NgcParsedConfiguration = config.into();
        assert_eq!(ngc_config.project, "test");
        assert!(!ngc_config.watch);
    }
}
