// Reference Emitter - Generates expressions for references
//
// The ReferenceEmitter uses strategies to produce expressions that 
// refer to References in the context of a particular file.

use super::references::Reference;

/// Flags which alter how imports are generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportFlags {
    bits: u8,
}

impl ImportFlags {
    pub const NONE: Self = Self { bits: 0x00 };
    
    /// Force the generation of a new import even if an identifier exists.
    pub const FORCE_NEW_IMPORT: Self = Self { bits: 0x01 };
    
    /// Import should not be deferred using dynamic imports.
    pub const NO_DEFERRED_IMPORTS: Self = Self { bits: 0x02 };
    
    /// Allow emitting references to type-only declarations.
    pub const ALLOW_TYPE_IMPORTS: Self = Self { bits: 0x04 };
    
    /// Allow relative imports from declaration files.
    pub const ALLOW_RELATIVE_DTS_IMPORTS: Self = Self { bits: 0x08 };
    
    /// Allow references from ambient imports.
    pub const ALLOW_AMBIENT_REFERENCES: Self = Self { bits: 0x10 };
    
    pub fn new() -> Self {
        Self::NONE
    }
    
    pub fn contains(&self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }
    
    pub fn insert(&mut self, other: Self) {
        self.bits |= other.bits;
    }
}

impl std::ops::BitOr for ImportFlags {
    type Output = Self;
    
    fn bitor(self, rhs: Self) -> Self::Output {
        Self { bits: self.bits | rhs.bits }
    }
}

/// Represents the import source of a generated expression.
#[derive(Debug, Clone)]
pub enum ImportedFile {
    /// Known source file path.
    Known(String),
    /// Unknown source (computation would be required to determine).
    Unknown,
    /// Not an import at all.
    NotAnImport,
}

/// Result kind for reference emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceEmitKind {
    Success,
    Failed,
}

/// Represents a successfully emitted reference.
#[derive(Debug, Clone)]
pub struct EmittedReference {
    /// The generated expression string.
    pub expression: String,
    /// Information about the imported file.
    pub imported_file: ImportedFile,
}

impl EmittedReference {
    pub fn new(expression: impl Into<String>, imported_file: ImportedFile) -> Self {
        Self {
            expression: expression.into(),
            imported_file,
        }
    }
}

/// Represents a failure to emit a reference.
#[derive(Debug, Clone)]
pub struct FailedEmitResult {
    /// The reference that failed to emit.
    pub ref_name: String,
    /// The context file where emission was attempted.
    pub context: String,
    /// Reason for the failure.
    pub reason: String,
}

impl FailedEmitResult {
    pub fn new(ref_name: impl Into<String>, context: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            ref_name: ref_name.into(),
            context: context.into(),
            reason: reason.into(),
        }
    }
}

/// Result of emitting a reference.
#[derive(Debug, Clone)]
pub enum ReferenceEmitResult {
    Success(EmittedReference),
    Failed(FailedEmitResult),
}

impl ReferenceEmitResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }
    
    pub fn unwrap(self) -> EmittedReference {
        match self {
            Self::Success(e) => e,
            Self::Failed(f) => panic!("Failed to emit reference: {}", f.reason),
        }
    }
}

/// Strategy for generating expressions that refer to References.
pub trait ReferenceEmitStrategy: Send + Sync {
    /// Emit an expression which refers to the given Reference.
    fn emit(
        &self,
        reference: &Reference,
        context_file: &str,
        import_flags: ImportFlags,
    ) -> Option<ReferenceEmitResult>;
}

/// Generates expressions which refer to References in a given context.
#[derive(Default)]
pub struct ReferenceEmitter {
    strategies: Vec<Box<dyn ReferenceEmitStrategy>>,
}

impl ReferenceEmitter {
    pub fn new(strategies: Vec<Box<dyn ReferenceEmitStrategy>>) -> Self {
        Self { strategies }
    }
    
    /// Emit a reference expression using registered strategies.
    pub fn emit(
        &self,
        reference: &Reference,
        context_file: &str,
        import_flags: ImportFlags,
    ) -> ReferenceEmitResult {
        for strategy in &self.strategies {
            if let Some(result) = strategy.emit(reference, context_file, import_flags) {
                return result;
            }
        }
        
        ReferenceEmitResult::Failed(FailedEmitResult::new(
            reference.debug_name(),
            context_file,
            "No strategy was able to emit a reference",
        ))
    }
}

/// Strategy: Use a local identifier if one exists.
#[derive(Debug, Default)]
pub struct LocalIdentifierStrategy;

impl LocalIdentifierStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl ReferenceEmitStrategy for LocalIdentifierStrategy {
    fn emit(
        &self,
        reference: &Reference,
        context_file: &str,
        import_flags: ImportFlags,
    ) -> Option<ReferenceEmitResult> {
        if import_flags.contains(ImportFlags::FORCE_NEW_IMPORT) {
            return None;
        }
        
        // Check if reference is in the same source file
        let is_same_file = reference.source_file
            .as_ref()
            .map(|sf| sf.to_string_lossy() == context_file)
            .unwrap_or(false);
        
        if is_same_file {
            Some(ReferenceEmitResult::Success(EmittedReference::new(
                reference.debug_name(),
                ImportedFile::NotAnImport,
            )))
        } else {
            None
        }
    }
}

/// Strategy: Generate an import using a relative path.
#[derive(Debug, Default)]
pub struct RelativePathStrategy;

impl RelativePathStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl ReferenceEmitStrategy for RelativePathStrategy {
    fn emit(
        &self,
        reference: &Reference,
        context_file: &str,
        _import_flags: ImportFlags,
    ) -> Option<ReferenceEmitResult> {
        let source_file = reference.source_file.as_ref()?;
        let source_file_str = source_file.to_string_lossy();
        
        if source_file_str == context_file {
            return None;
        }
        
        // Generate a relative import path
        let relative_path = calculate_relative_path(context_file, &source_file_str);
        let import_expr = format!("import('{}').{}", relative_path, reference.debug_name());
        
        Some(ReferenceEmitResult::Success(EmittedReference::new(
            import_expr,
            ImportedFile::Known(source_file_str.to_string()),
        )))
    }
}

/// Strategy: Generate an import using an absolute module specifier.
#[derive(Debug, Default)]
pub struct AbsoluteModuleStrategy;

impl AbsoluteModuleStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl ReferenceEmitStrategy for AbsoluteModuleStrategy {
    fn emit(
        &self,
        reference: &Reference,
        _context_file: &str,
        _import_flags: ImportFlags,
    ) -> Option<ReferenceEmitResult> {
        if let Some(owning_module) = reference.owned_by_module_guess() {
            let import_expr = format!("{}#{}", owning_module, reference.debug_name());
            Some(ReferenceEmitResult::Success(EmittedReference::new(
                import_expr,
                ImportedFile::Unknown,
            )))
        } else {
            None
        }
    }
}

/// Strategy: Use logical project paths.
#[derive(Debug)]
pub struct LogicalProjectStrategy {
    base_path: String,
}

impl LogicalProjectStrategy {
    pub fn new(base_path: impl Into<String>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }
}

impl ReferenceEmitStrategy for LogicalProjectStrategy {
    fn emit(
        &self,
        reference: &Reference,
        _context_file: &str,
        _import_flags: ImportFlags,
    ) -> Option<ReferenceEmitResult> {
        let source_file = reference.source_file.as_ref()?;
        let source_file_str = source_file.to_string_lossy();
        
        if source_file_str.starts_with(&self.base_path) {
            let logical_path = source_file_str.strip_prefix(&self.base_path)
                .unwrap_or(&source_file_str);
            let import_expr = format!("@project{}#{}", logical_path, reference.debug_name());
            Some(ReferenceEmitResult::Success(EmittedReference::new(
                import_expr,
                ImportedFile::Known(source_file_str.to_string()),
            )))
        } else {
            None
        }
    }
}

/// Calculate relative path between two file paths.
fn calculate_relative_path(from: &str, to: &str) -> String {
    use std::path::Path;
    
    let from_path = Path::new(from);
    let to_path = Path::new(to);
    
    let from_dir = from_path.parent().unwrap_or(Path::new(""));
    
    if let Ok(rel) = to_path.strip_prefix(from_dir) {
        return format!("./{}", rel.display());
    }
    
    // Simple implementation - count parent directories
    let from_parts: Vec<_> = from_dir.components().collect();
    let to_parts: Vec<_> = to_path.components().collect();
    
    let common = from_parts.iter()
        .zip(to_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();
    
    let ups = from_parts.len() - common;
    let mut result = String::new();
    
    for _ in 0..ups {
        result.push_str("../");
    }
    
    for part in &to_parts[common..] {
        result.push_str(&part.as_os_str().to_string_lossy());
        result.push('/');
    }
    
    result.pop(); // Remove trailing slash
    result
}
