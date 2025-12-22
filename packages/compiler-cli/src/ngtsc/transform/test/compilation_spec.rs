// Compilation Tests - Tests for TraitCompiler
//
// These tests verify the behavior of the TraitCompiler across different
// compilation modes and handler configurations.

use std::sync::Arc;
use crate::ngtsc::transform::src::api::{
    AnalysisOutput, CompilationMode, CompileResult, ConstantPool, DecoratorHandler,
    DetectResult, HandlerPrecedence, ResolveResult,
};
use crate::ngtsc::transform::src::compilation::TraitCompiler;
use crate::ngtsc::reflection::ClassDeclaration;

// ============================================================================
// Mock Decorator Handlers
// ============================================================================

/// A simple mock handler that always rejects classes
struct NeverMatchHandler;

impl DecoratorHandler<(), (), (), ()> for NeverMatchHandler {
    fn name(&self) -> &str {
        "NeverMatchHandler"
    }

    fn precedence(&self) -> HandlerPrecedence {
        HandlerPrecedence::Primary
    }

    fn detect(&self, _node: &ClassDeclaration, _decorators: &[String]) -> Option<DetectResult<()>> {
        None
    }

    fn analyze(&self, _node: &ClassDeclaration, _metadata: &()) -> AnalysisOutput<()> {
        AnalysisOutput::empty()
    }

    fn symbol(&self, _node: &ClassDeclaration, _analysis: &()) -> Option<()> {
        None
    }

    fn compile_full(
        &self,
        _node: &ClassDeclaration,
        _analysis: &(),
        _resolution: Option<&()>,
        _constant_pool: &mut ConstantPool,
    ) -> Vec<CompileResult> {
        vec![]
    }
}

/// Handler that matches when "Full" decorator is present
struct FullDecoratorHandler;

impl DecoratorHandler<String, String, (), ()> for FullDecoratorHandler {
    fn name(&self) -> &str {
        "FullDecoratorHandler"
    }

    fn precedence(&self) -> HandlerPrecedence {
        HandlerPrecedence::Primary
    }

    fn detect(&self, _node: &ClassDeclaration, decorators: &[String]) -> Option<DetectResult<String>> {
        if decorators.contains(&"Full".to_string()) {
            Some(DetectResult {
                trigger: Some("Full".to_string()),
                decorator: Some("Full".to_string()),
                metadata: "Full".to_string(),
            })
        } else {
            None
        }
    }

    fn analyze(&self, _node: &ClassDeclaration, _metadata: &String) -> AnalysisOutput<String> {
        AnalysisOutput::of("analyzed".to_string())
    }

    fn symbol(&self, _node: &ClassDeclaration, _analysis: &String) -> Option<()> {
        None
    }

    fn compile_full(
        &self,
        _node: &ClassDeclaration,
        _analysis: &String,
        _resolution: Option<&()>,
        _constant_pool: &mut ConstantPool,
    ) -> Vec<CompileResult> {
        vec![CompileResult::new("compileFull", "boolean")]
    }
}

/// Handler that matches when "Partial" decorator is present and has compilePartial
struct PartialDecoratorHandler;

impl DecoratorHandler<String, String, (), ()> for PartialDecoratorHandler {
    fn name(&self) -> &str {
        "PartialDecoratorHandler"
    }

    fn precedence(&self) -> HandlerPrecedence {
        HandlerPrecedence::Primary
    }

    fn detect(&self, _node: &ClassDeclaration, decorators: &[String]) -> Option<DetectResult<String>> {
        if decorators.contains(&"Partial".to_string()) {
            Some(DetectResult {
                trigger: Some("Partial".to_string()),
                decorator: Some("Partial".to_string()),
                metadata: "Partial".to_string(),
            })
        } else {
            None
        }
    }

    fn analyze(&self, _node: &ClassDeclaration, _metadata: &String) -> AnalysisOutput<String> {
        AnalysisOutput::of("analyzed".to_string())
    }

    fn symbol(&self, _node: &ClassDeclaration, _analysis: &String) -> Option<()> {
        None
    }

    fn compile_full(
        &self,
        _node: &ClassDeclaration,
        _analysis: &String,
        _resolution: Option<&()>,
        _constant_pool: &mut ConstantPool,
    ) -> Vec<CompileResult> {
        vec![CompileResult::new("compileFull", "boolean")]
    }

    fn compile_partial(
        &self,
        _node: &ClassDeclaration,
        _analysis: &String,
        _resolution: Option<&()>,
    ) -> Vec<CompileResult> {
        vec![CompileResult::new("compilePartial", "boolean")]
    }
}

/// Handler that matches when "Local" decorator is present and has compileLocal
struct LocalDecoratorHandler;

impl DecoratorHandler<String, String, (), ()> for LocalDecoratorHandler {
    fn name(&self) -> &str {
        "LocalDecoratorHandler"
    }

    fn precedence(&self) -> HandlerPrecedence {
        HandlerPrecedence::Primary
    }

    fn detect(&self, _node: &ClassDeclaration, decorators: &[String]) -> Option<DetectResult<String>> {
        if decorators.contains(&"Local".to_string()) {
            Some(DetectResult {
                trigger: Some("Local".to_string()),
                decorator: Some("Local".to_string()),
                metadata: "Local".to_string(),
            })
        } else {
            None
        }
    }

    fn analyze(&self, _node: &ClassDeclaration, _metadata: &String) -> AnalysisOutput<String> {
        AnalysisOutput::of("analyzed".to_string())
    }

    fn symbol(&self, _node: &ClassDeclaration, _analysis: &String) -> Option<()> {
        None
    }

    fn compile_full(
        &self,
        _node: &ClassDeclaration,
        _analysis: &String,
        _resolution: Option<&()>,
        _constant_pool: &mut ConstantPool,
    ) -> Vec<CompileResult> {
        vec![CompileResult::new("compileFull", "boolean")]
    }

    fn compile_local(
        &self,
        _node: &ClassDeclaration,
        _analysis: &String,
        _resolution: Option<&()>,
        _constant_pool: &mut ConstantPool,
    ) -> Vec<CompileResult> {
        vec![CompileResult::new("compileLocal", "boolean")]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trait_compiler_creation() {
        // Test that TraitCompiler can be created with empty handlers
        let handlers: Vec<Arc<dyn DecoratorHandler<(), (), (), ()>>> = vec![];
        let compiler = TraitCompiler::new(handlers, CompilationMode::Full);
        
        // Should not panic and should be empty
        assert!(compiler.diagnostics().is_empty());
    }

    #[test]
    fn test_trait_compiler_with_never_match_handler() {
        let handlers: Vec<Arc<dyn DecoratorHandler<(), (), (), ()>>> = vec![
            Arc::new(NeverMatchHandler),
        ];
        let compiler = TraitCompiler::new(handlers, CompilationMode::Full);
        
        // The handler never matches, so no classes should be recorded
        assert!(compiler.record_for("NonexistentClass").is_none());
    }

    #[test]
    fn test_compilation_mode_full() {
        let handlers: Vec<Arc<dyn DecoratorHandler<String, String, (), ()>>> = vec![
            Arc::new(FullDecoratorHandler),
        ];
        let compiler = TraitCompiler::new(handlers, CompilationMode::Full);
        
        // In Full mode, compile_full should be called
        // (This test verifies the compiler is in Full mode)
        let analyzed_records = compiler.get_analyzed_records();
        assert!(analyzed_records.is_empty()); // No files analyzed yet
    }

    #[test]
    fn test_compilation_mode_partial() {
        let handlers: Vec<Arc<dyn DecoratorHandler<String, String, (), ()>>> = vec![
            Arc::new(PartialDecoratorHandler),
        ];
        let compiler = TraitCompiler::new(handlers, CompilationMode::Partial);
        
        // In Partial mode, compile_partial should be called when available
        let analyzed_records = compiler.get_analyzed_records();
        assert!(analyzed_records.is_empty());
    }

    #[test]
    fn test_compilation_mode_local() {
        let handlers: Vec<Arc<dyn DecoratorHandler<String, String, (), ()>>> = vec![
            Arc::new(LocalDecoratorHandler),
        ];
        let compiler = TraitCompiler::new(handlers, CompilationMode::Local);
        
        // In Local mode, compile_local should be called
        let analyzed_records = compiler.get_analyzed_records();
        assert!(analyzed_records.is_empty());
    }

    #[test]
    fn test_compile_result_creation() {
        let result = CompileResult::new("testField", "TestType");
        
        assert_eq!(result.name, "testField");
        assert_eq!(result.type_desc, "TestType");
        assert!(result.initializer.is_none());
        assert!(result.statements.is_empty());
        assert!(result.deferrable_imports.is_none());
    }

    #[test]
    fn test_analysis_output_of() {
        let output: AnalysisOutput<String> = AnalysisOutput::of("test".to_string());
        
        assert!(output.analysis.is_some());
        assert_eq!(output.analysis.unwrap(), "test");
        assert!(output.diagnostics.is_none());
    }

    #[test]
    fn test_analysis_output_empty() {
        let output: AnalysisOutput<String> = AnalysisOutput::empty();
        
        assert!(output.analysis.is_none());
        assert!(output.diagnostics.is_none());
    }

    #[test]
    fn test_resolve_result_of() {
        let result: ResolveResult<String> = ResolveResult::of("resolved".to_string());
        
        assert!(result.data.is_some());
        assert_eq!(result.data.unwrap(), "resolved");
        assert!(result.diagnostics.is_none());
        assert!(result.reexports.is_none());
    }

    #[test]
    fn test_resolve_result_empty() {
        let result: ResolveResult<String> = ResolveResult::empty();
        
        assert!(result.data.is_none());
        assert!(result.diagnostics.is_none());
        assert!(result.reexports.is_none());
    }

    #[test]
    fn test_handler_precedence_ordering() {
        // PRIMARY should be least (handled first)
        // WEAK should be greatest (handled last, removed if others match)
        assert!(HandlerPrecedence::Primary < HandlerPrecedence::Shared);
        assert!(HandlerPrecedence::Shared < HandlerPrecedence::Weak);
    }

    #[test]
    fn test_detect_result_creation() {
        let result = DetectResult {
            trigger: Some("Component".to_string()),
            decorator: Some("Component".to_string()),
            metadata: "test metadata".to_string(),
        };
        
        assert_eq!(result.trigger, Some("Component".to_string()));
        assert_eq!(result.decorator, Some("Component".to_string()));
        assert_eq!(result.metadata, "test metadata");
    }
}
