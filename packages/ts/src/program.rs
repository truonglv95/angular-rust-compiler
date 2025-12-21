use crate::{Diagnostic, DiagnosticWithLocation, ScriptTarget, ModuleKind};
use crate::node::SourceFile;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompilerOptions {
    pub allow_js: Option<bool>,
    pub allow_synthetic_default_imports: Option<bool>,
    pub allow_unreachable_code: Option<bool>,
    pub allow_unused_labels: Option<bool>,
    pub always_strict: Option<bool>,
    pub base_url: Option<String>,
    pub check_js: Option<bool>,
    pub declaration: Option<bool>,
    pub declaration_map: Option<bool>,
    pub emit_declaration_only: Option<bool>,
    pub declaration_dir: Option<String>,
    pub disable_size_limit: Option<bool>,
    pub downlevel_iteration: Option<bool>,
    pub emit_bom: Option<bool>,
    pub emit_decorator_metadata: Option<bool>,
    pub experimental_decorators: Option<bool>,
    pub force_consistent_casing_in_file_names: Option<bool>,
    pub import_helpers: Option<bool>,
    pub inline_source_map: Option<bool>,
    pub inline_sources: Option<bool>,
    pub isolated_modules: Option<bool>,
    pub jsx: Option<JsxEmit>,
    pub lib: Option<Vec<String>>,
    pub locale: Option<String>,
    pub map_root: Option<String>,
    pub max_node_module_js_depth: Option<u32>,
    pub module: Option<ModuleKind>,
    pub module_resolution: Option<ModuleResolutionKind>,
    pub module_suffixes: Option<Vec<String>>,
    pub module_detection: Option<ModuleDetectionKind>,
    pub no_emit: Option<bool>,
    pub no_emit_helpers: Option<bool>,
    pub no_emit_on_error: Option<bool>,
    pub no_error_truncation: Option<bool>,
    pub no_fallthrough_cases_in_switch: Option<bool>,
    pub no_implicit_any: Option<bool>,
    pub no_implicit_returns: Option<bool>,
    pub no_implicit_this: Option<bool>,
    pub no_unused_locals: Option<bool>,
    pub no_unused_parameters: Option<bool>,
    pub no_implicit_use_strict: Option<bool>,
    pub no_property_access_from_index_signature: Option<bool>,
    pub assume_changes_only_affect_direct_dependencies: Option<bool>,
    pub no_lib: Option<bool>,
    pub no_resolve: Option<bool>,
    pub no_unchecked_indexed_access: Option<bool>,
    pub out: Option<String>,
    pub out_dir: Option<String>,
    pub out_file: Option<String>,
    pub paths: Option<std::collections::HashMap<String, Vec<String>>>,
    pub preserve_const_enums: Option<bool>,
    pub preserve_symlinks: Option<bool>,
    pub project: Option<String>,
    pub react_namespace: Option<String>,
    pub jsx_factory: Option<String>,
    pub jsx_fragment_factory: Option<String>,
    pub jsx_import_source: Option<String>,
    pub composite: Option<bool>,
    pub incremental: Option<bool>,
    pub ts_build_info_file: Option<String>,
    pub remove_comments: Option<bool>,
    pub root_dir: Option<String>,
    pub root_dirs: Option<Vec<String>>,
    pub skip_lib_check: Option<bool>,
    pub skip_default_lib_check: Option<bool>,
    pub source_map: Option<bool>,
    pub source_root: Option<String>,
    pub strict: Option<bool>,
    pub strict_function_types: Option<bool>,
    pub strict_bind_call_apply: Option<bool>,
    pub strict_null_checks: Option<bool>,
    pub strict_property_initialization: Option<bool>,
    pub strip_internal: Option<bool>,
    pub suppress_excess_property_errors: Option<bool>,
    pub suppress_implicit_any_index_errors: Option<bool>,
    pub target: Option<ScriptTarget>,
    pub trace_resolution: Option<bool>,
    pub resolve_json_module: Option<bool>,
    pub types: Option<Vec<String>>,
    pub type_roots: Option<Vec<String>>,
    pub es_module_interop: Option<bool>,
    pub use_define_for_class_fields: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsxEmit {
    None,
    Preserve,
    React,
    ReactNative,
    ReactJSX,
    ReactJSXDev,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleResolutionKind {
    Classic,
    NodeJs,
    Node16,
    NodeNext,
    Bundler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleDetectionKind {
    Legacy,
    Auto,
    Force,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewLineKind {
    CarriageReturnLineFeed,
    LineFeed,
}

pub trait CompilerHost {
    fn get_source_file(&self, file_name: &str, language_version: ScriptTarget) -> Option<Box<dyn SourceFile>>;
    fn get_default_lib_file_name(&self, options: &CompilerOptions) -> String;
    fn get_current_directory(&self) -> String;
    fn get_canonical_file_name(&self, file_name: &str) -> String;
    fn use_case_sensitive_file_names(&self) -> bool;
    fn get_new_line(&self) -> String;
    fn file_exists(&self, file_name: &str) -> bool;
    fn read_file(&self, file_name: &str) -> Option<String>;
}

pub trait Program {
    fn get_root_file_names(&self) -> Vec<String>;
    fn get_source_files(&self) -> Vec<Box<dyn SourceFile>>;
    fn get_options_diagnostics(&self) -> Vec<Diagnostic>;
    fn get_global_diagnostics(&self) -> Vec<Diagnostic>;
    fn get_syntactic_diagnostics(&self, source_file: Option<&dyn SourceFile>) -> Vec<DiagnosticWithLocation>;
    fn get_semantic_diagnostics(&self, source_file: Option<&dyn SourceFile>) -> Vec<Diagnostic>;
    fn get_declaration_diagnostics(&self, source_file: Option<&dyn SourceFile>) -> Vec<DiagnosticWithLocation>;
    fn get_type_checker(&self) -> Box<dyn crate::type_checker::TypeChecker>;
}
