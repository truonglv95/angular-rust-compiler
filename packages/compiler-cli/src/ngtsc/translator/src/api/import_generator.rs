
#[derive(Debug, Clone)]
pub struct ImportRequest<TFile> {
    /// Name of the export to be imported.
    /// May be `None` if a namespace import is requested.
    pub export_symbol_name: Option<String>,

    /// Module specifier to be imported.
    /// May be a module name, or a file-relative path.
    pub export_module_specifier: String,

    /// File for which the import is requested for. This may
    /// be used by import generators to re-use existing imports.
    ///
    /// Import managers may also allow this to be nullable if
    /// imports are never re-used. E.g. in the linker generator.
    pub requested_file: TFile,

    /// Specifies an alias under which the symbol can be referenced within
    /// the file (e.g. `import { symbol as alias } from 'module'`).
    ///
    /// !!!Warning!!! passing in this alias is considered unsafe, because the import manager won't
    /// try to avoid conflicts with existing identifiers in the file if it is specified. As such,
    /// this option should only be used if the caller has verified that the alias won't conflict
    /// with anything in the file.
    pub unsafe_alias_override: Option<String>,
}

pub trait ImportGenerator<TFile, TExpression> {
    fn add_import(&mut self, request: ImportRequest<TFile>) -> TExpression;
}
