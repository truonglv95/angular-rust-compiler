// Reexport - Types for tracking reexports
//
// A Reexport represents a symbol being re-exported from a module.

/// Represents a symbol being re-exported from a module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reexport {
    /// The original symbol name.
    pub symbol_name: String,
    /// The alias under which the symbol is exported.
    pub as_alias: String,
    /// The module from which the symbol is re-exported.
    pub from_module: String,
}

impl Reexport {
    pub fn new(
        symbol_name: impl Into<String>,
        as_alias: impl Into<String>,
        from_module: impl Into<String>,
    ) -> Self {
        Self {
            symbol_name: symbol_name.into(),
            as_alias: as_alias.into(),
            from_module: from_module.into(),
        }
    }
}
