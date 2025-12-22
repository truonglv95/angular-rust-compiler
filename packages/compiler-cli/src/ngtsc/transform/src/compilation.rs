// Compilation - The heart of Angular compilation
//
// The TraitCompiler is responsible for processing all classes in the program. Any time a
// DecoratorHandler matches a class, a "trait" is created to represent that Angular aspect
// of the class (such as the class having a component definition).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::ngtsc::transform::src::api::{
    CompilationMode, CompileResult, ConstantPool, DecoratorHandler, IndexingContext, TypeCheckContext, Xi18nContext,
};
use crate::ngtsc::transform::src::trait_::TraitState;
use crate::ngtsc::transform::src::declaration::DtsTransformRegistry;
use crate::ngtsc::reflection::ClassDeclaration;
use ts::Diagnostic;

// ============================================================================
// ClassRecord
// ============================================================================

/// Records information about a specific class that has matched traits.
#[derive(Clone)]
pub struct ClassRecord<D: Clone, A: Clone, S: Clone, R: Clone> {
    /// Class identifier/name.
    pub class_name: String,
    
    /// All traits which matched on the class.
    pub traits: Vec<TraitInfo<D, A, S, R>>,
    
    /// Meta-diagnostics about the class (e.g., invalid decorator combinations).
    pub meta_diagnostics: Option<Vec<Diagnostic>>,
    
    /// Whether traits contains traits matched from DecoratorHandlers marked as WEAK.
    pub has_weak_handlers: bool,
    
    /// Whether traits contains a trait from a DecoratorHandler matched as PRIMARY.
    pub has_primary_handler: bool,
}

impl<D: Clone, A: Clone, S: Clone, R: Clone> ClassRecord<D, A, S, R> {
    /// Create a new class record.
    pub fn new(class_name: String) -> Self {
        Self {
            class_name,
            traits: Vec::new(),
            meta_diagnostics: None,
            has_weak_handlers: false,
            has_primary_handler: false,
        }
    }
}

// ============================================================================
// TraitInfo
// ============================================================================

/// Simplified trait info for storage.
#[derive(Clone)]
pub struct TraitInfo<D: Clone, A: Clone, S: Clone, R: Clone> {
    /// Current state of the trait.
    pub state: TraitState,
    /// Handler name.
    pub handler_name: String,
    /// Analysis results.
    pub analysis: Option<A>,
    /// Symbol.
    pub symbol: Option<S>,
    /// Resolution results.
    pub resolution: Option<R>,
    /// Analysis diagnostics.
    pub analysis_diagnostics: Option<Vec<Diagnostic>>,
    /// Resolve diagnostics.
    pub resolve_diagnostics: Option<Vec<Diagnostic>>,
    /// Detected metadata
    pub detected_metadata: Option<D>,
}

// ============================================================================
// Source File Type Identifier
// ============================================================================

/// Interface for identifying source file types.
pub trait SourceFileTypeIdentifier {
    /// Check if a file is a shim file.
    fn is_shim(&self, path: &str) -> bool;
    
    /// Check if a file is a resource file.
    fn is_resource(&self, path: &str) -> bool;
}

/// Default implementation that returns false for all checks.
pub struct DefaultSourceFileTypeIdentifier;

impl SourceFileTypeIdentifier for DefaultSourceFileTypeIdentifier {
    fn is_shim(&self, _path: &str) -> bool {
        false
    }
    
    fn is_resource(&self, _path: &str) -> bool {
        false
    }
}

// ============================================================================
// TraitCompiler
// ============================================================================

/// The heart of Angular compilation.
///
/// The TraitCompiler is responsible for processing all classes in the program. Any time a
/// DecoratorHandler matches a class, a "trait" is created to represent that Angular aspect
/// of the class.
pub struct TraitCompiler<D: Clone, A: Clone, S: Clone, R: Clone> {
    /// All registered decorator handlers.
    handlers: Vec<Arc<dyn DecoratorHandler<D, A, S, R>>>,
    
    /// Map of handler names to handlers (for lookup during adoption).
    handlers_by_name: HashMap<String, Arc<dyn DecoratorHandler<D, A, S, R>>>,
    
    /// Map of class identifiers to their records.
    classes: HashMap<String, ClassRecord<D, A, S, R>>,
    
    /// Map of source file paths to class identifiers within them.
    file_to_classes: HashMap<String, HashSet<String>>,
    
    /// Files that were analyzed but contained no traits.
    files_without_traits: HashSet<String>,
    
    /// Map for re-exports (filename -> alias -> (module, symbol)).
    reexport_map: HashMap<String, HashMap<String, (String, String)>>,
    
    /// The compilation mode.
    compilation_mode: CompilationMode,
    
    /// Whether to compile non-exported classes.
    compile_non_exported_classes: bool,
    
    /// DTS transform registry.
    dts_transforms: DtsTransformRegistry,
    
    /// Whether to emit declaration files only.
    emit_declaration_only: bool,
}

impl<D: Clone, A: Clone, S: Clone, R: Clone> TraitCompiler<D, A, S, R> {
    /// Create a new TraitCompiler.
    pub fn new(
        handlers: Vec<Arc<dyn DecoratorHandler<D, A, S, R>>>,
        compilation_mode: CompilationMode,
    ) -> Self {
        let mut handlers_by_name = HashMap::new();
        for handler in &handlers {
            handlers_by_name.insert(handler.name().to_string(), handler.clone());
        }
        
        Self {
            handlers,
            handlers_by_name,
            classes: HashMap::new(),
            file_to_classes: HashMap::new(),
            files_without_traits: HashSet::new(),
            reexport_map: HashMap::new(),
            compilation_mode,
            compile_non_exported_classes: true,
            dts_transforms: DtsTransformRegistry::new(),
            emit_declaration_only: false,
        }
    }

    /// Create a TraitCompiler with additional options.
    pub fn with_options(
        handlers: Vec<Arc<dyn DecoratorHandler<D, A, S, R>>>,
        compilation_mode: CompilationMode,
        compile_non_exported_classes: bool,
        emit_declaration_only: bool,
    ) -> Self {
        let mut compiler = Self::new(handlers, compilation_mode);
        compiler.compile_non_exported_classes = compile_non_exported_classes;
        compiler.emit_declaration_only = emit_declaration_only;
        compiler
    }

    /// Analyze a source file synchronously.
    pub fn analyze_sync(&mut self, sf_path: &str, is_declaration_file: bool) {
        self.analyze(sf_path, is_declaration_file);
    }

    /// Analyze a source file.
    fn analyze(&mut self, sf_path: &str, is_declaration_file: bool) {
        // We shouldn't analyze declaration, shim, or resource files.
        if is_declaration_file {
            return;
        }

        // Track the file
        if !self.file_to_classes.contains_key(sf_path) {
            self.files_without_traits.insert(sf_path.to_string());
        }
    }

    /// Get the record for a class, if it exists.
    pub fn record_for(&self, class_name: &str) -> Option<&ClassRecord<D, A, S, R>> {
        self.classes.get(class_name)
    }

    /// Get all analyzed records grouped by source file.
    pub fn get_analyzed_records(&self) -> HashMap<String, Vec<String>> {
        let mut result = HashMap::new();
        
        for (sf, classes) in &self.file_to_classes {
            result.insert(sf.clone(), classes.iter().cloned().collect());
        }
        
        for sf in &self.files_without_traits {
            result.entry(sf.clone()).or_insert_with(Vec::new);
        }
        
        result
    }

    /// Register a class with a trait.
    pub fn register_class_trait(
        &mut self,
        class_name: &str,
        handler_name: &str,
        detected_metadata: D,
    ) {
        let trait_info = TraitInfo {
            state: TraitState::Pending,
            handler_name: handler_name.to_string(),
            analysis: None,
            symbol: None,
            resolution: None,
            analysis_diagnostics: None,
            resolve_diagnostics: None,
            detected_metadata: Some(detected_metadata),
        };
        
        self.classes
            .entry(class_name.to_string())
            .or_insert_with(|| ClassRecord::new(class_name.to_string()))
            .traits
            .push(trait_info);
    }

    /// Resolve all analyzed traits.
    pub fn resolve(&mut self) {
        let class_names: Vec<String> = self.classes.keys().cloned().collect();
        
        for class_name in class_names {
            if let Some(record) = self.classes.get_mut(&class_name) {
                for trait_info in &mut record.traits {
                    match trait_info.state {
                        TraitState::Skipped => continue,
                        TraitState::Pending => {
                            // Skip pending traits - they should be analyzed first
                            continue;
                        }
                        TraitState::Resolved => {
                            // Already resolved
                            continue;
                        }
                        TraitState::Analyzed => {}
                    }
                    
                    if trait_info.analysis.is_none() {
                        continue;
                    }
                    
                    // Mark as resolved
                    trait_info.state = TraitState::Resolved;
                }
            }
        }
    }

    /// Compile a class and return compilation results.
    pub fn compile(
        &self,
        class_name: &str,
        class_decl: &ClassDeclaration,
        constant_pool: &mut ConstantPool,
    ) -> Option<Vec<CompileResult>> {
        let record = self.classes.get(class_name)?;
        let mut results = Vec::new();
        
        for trait_info in &record.traits {
            if trait_info.state != TraitState::Resolved {
                continue;
            }
            
            // Check for errors in diagnostics
            if has_diagnostic_errors(&trait_info.analysis_diagnostics) {
                continue;
            }
            if has_diagnostic_errors(&trait_info.resolve_diagnostics) {
                continue;
            }
            
            // Get handler
            let handler = self.handlers_by_name.get(&trait_info.handler_name)?;
            
            // Compile based on mode
            let analysis = trait_info.analysis.as_ref()?;
            let resolution = trait_info.resolution.as_ref();
            
            let compile_results = match self.compilation_mode {
                CompilationMode::Local => {
                    handler.compile_local(class_decl, analysis, resolution, constant_pool)
                }
                CompilationMode::Partial => {
                    handler.compile_partial(class_decl, analysis, resolution)
                }
                CompilationMode::Full => {
                    handler.compile_full(class_decl, analysis, resolution, constant_pool)
                }
            };
            
            // Deduplicate results by name
            for result in compile_results {
                if !results.iter().any(|r: &CompileResult| r.name == result.name) {
                    results.push(result);
                }
            }
        }
        
        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    }

    /// Type check components in a source file.
    pub fn type_check(&self, _sf_path: &str, _ctx: &mut TypeCheckContext) {
        if self.compilation_mode == CompilationMode::Local {
            return;
        }
        // TODO: Implement type checking
    }

    /// Index components in all analyzed classes.
    pub fn index(&self, _ctx: &mut IndexingContext) {
        // TODO: Implement indexing
    }

    /// Extract i18n messages from all analyzed classes.
    pub fn xi18n(&self, _bundle: &mut Xi18nContext) {
        // TODO: Implement i18n extraction
    }

    /// Get all diagnostics from the compilation.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        for record in self.classes.values() {
            if let Some(ref meta_diags) = record.meta_diagnostics {
                diagnostics.extend(meta_diags.iter().cloned());
            }
            
            for trait_info in &record.traits {
                if trait_info.state == TraitState::Analyzed || trait_info.state == TraitState::Resolved {
                    if let Some(ref diags) = trait_info.analysis_diagnostics {
                        diagnostics.extend(diags.iter().cloned());
                    }
                }
                if trait_info.state == TraitState::Resolved {
                    if let Some(ref diags) = trait_info.resolve_diagnostics {
                        diagnostics.extend(diags.iter().cloned());
                    }
                }
            }
        }
        
        diagnostics
    }

    /// Get the re-export map for generating export statements.
    pub fn export_statements(&self) -> &HashMap<String, HashMap<String, (String, String)>> {
        &self.reexport_map
    }

    /// Get the DTS transform registry.
    pub fn dts_transforms(&self) -> &DtsTransformRegistry {
        &self.dts_transforms
    }

    /// Get mutable access to the DTS transform registry.
    pub fn dts_transforms_mut(&mut self) -> &mut DtsTransformRegistry {
        &mut self.dts_transforms
    }
}

/// Helper function to check if diagnostics contain errors
fn has_diagnostic_errors(diagnostics: &Option<Vec<Diagnostic>>) -> bool {
    diagnostics.as_ref()
        .map(|diags| !diags.is_empty()) // Consider any diagnostic as potential error for now
        .unwrap_or(false)
}
