//! Pipe decorator handler
//!
//! Handles @Pipe decorator and generates ɵpipe definition.

use crate::ngtsc::reflection::ClassDeclaration;
use crate::ngtsc::transform::src::api::{
    AnalysisOutput, CompileResult, ConstantPool, DecoratorHandler, DetectResult, HandlerPrecedence,
};
use angular_compiler::output::abstract_emitter::EmitterVisitorContext;
use angular_compiler::output::abstract_js_emitter::AbstractJsEmitterVisitor;
use angular_compiler::output::output_ast::{ExpressionTrait, ReadVarExpr};
use angular_compiler::output::oxc_emitter::OxcEmitter;
use angular_compiler::render3::r3_pipe_compiler::{compile_pipe_from_metadata, R3PipeMetadata};
use angular_compiler::render3::util::R3Reference;
use std::any::Any;
use std::collections::HashMap;

/// Metadata extracted from @Pipe decorator
#[derive(Debug, Clone)]
pub struct PipeMetadata {
    /// Class name
    pub name: String,
    /// Pipe name (used in templates)
    pub pipe_name: String,
    /// Whether the pipe is pure (default: true)
    pub pure: bool,
    /// Whether the pipe is standalone (default: true)
    pub standalone: bool,
}

impl PipeMetadata {
    /// Create default metadata from class name
    pub fn new(class_name: String) -> Self {
        PipeMetadata {
            name: class_name.clone(),
            pipe_name: class_name,
            pure: true,
            standalone: true,
        }
    }

    /// Create from decorator arguments
    pub fn from_args(class_name: String, args: &serde_json::Value) -> Self {
        let pipe_name = args
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&class_name)
            .to_string();

        let pure = args.get("pure").and_then(|v| v.as_bool()).unwrap_or(true);

        let standalone = args
            .get("standalone")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        PipeMetadata {
            name: class_name,
            pipe_name,
            pure,
            standalone,
        }
    }
}

/// Handler for @Pipe decorator
pub struct PipeDecoratorHandler;

impl PipeDecoratorHandler {
    pub fn new() -> Self {
        PipeDecoratorHandler
    }

    /// Detect @Pipe decorator on a class
    pub fn detect_pipe(decorators: &[String]) -> bool {
        decorators.iter().any(|d| d == "Pipe")
    }

    /// Compile pipe definition
    /// Generates: static ɵpipe = ɵɵdefinePipe({ name: 'pipeName', type: PipeClass, pure: true, standalone: true })
    pub fn compile_pipe(metadata: &PipeMetadata) -> CompileResult {
        // 1. Prepare Metadata for Ivy Compiler
        let type_ref = R3Reference {
            value: angular_compiler::output::output_ast::Expression::ReadVar(ReadVarExpr {
                name: metadata.name.clone(),
                type_: None,
                source_span: None,
            }),
            type_expr: angular_compiler::output::output_ast::Expression::ReadVar(ReadVarExpr {
                name: metadata.name.clone(),
                type_: None,
                source_span: None,
            }),
        };

        let r3_meta = R3PipeMetadata {
            name: metadata.name.clone(),
            type_: type_ref,
            type_argument_count: 0,
            pipe_name: Some(metadata.pipe_name.clone()),
            deps: None, // Factory handles deps
            pure: metadata.pure,
            is_standalone: metadata.standalone,
        };

        // 2. Compile to Angular Output AST
        let compiled = compile_pipe_from_metadata(&r3_meta);

        // 3. Emit to String (Fallback/Traditional path)
        let mut imports_map = HashMap::new();
        imports_map.insert("@angular/core".to_string(), "i0".to_string());

        let mut emitter = AbstractJsEmitterVisitor::with_imports(imports_map.clone());
        let mut ctx = EmitterVisitorContext::create_root();
        {
            let context: &mut dyn Any = &mut ctx;
            compiled.expression.visit_expression(&mut emitter, context);
        }
        let initializer = ctx.to_source();

        // 4. AST-Native path: produce OXC AST code via OxcEmitter + oxc_codegen
        let initializer_ast_code = {
            let ast_allocator = oxc_allocator::Allocator::default();
            let oxc_emitter = OxcEmitter::with_imports(&ast_allocator, imports_map);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let oxc_expr = oxc_emitter.emit_expression(&compiled.expression);
                let mut body = oxc_allocator::Vec::new_in(&ast_allocator);
                body.push(oxc_ast::ast::Statement::ExpressionStatement(
                    oxc_allocator::Box::new_in(
                        oxc_ast::ast::ExpressionStatement {
                            span: oxc_span::Span::default(),
                            expression: oxc_expr,
                        },
                        &ast_allocator,
                    ),
                ));
                let program = oxc_ast::ast::Program {
                    span: oxc_span::Span::default(),
                    source_type: oxc_span::SourceType::mjs(),
                    source_text: "",
                    comments: oxc_allocator::Vec::new_in(&ast_allocator),
                    hashbang: None,
                    directives: oxc_allocator::Vec::new_in(&ast_allocator),
                    body,
                    scope_id: std::cell::Cell::new(None),
                };
                let codegen = oxc_codegen::Codegen::new();
                let mut code = codegen.build(&program).code;
                code = code.trim_end().to_string();
                if code.ends_with(';') {
                    code.pop();
                }
                code
            }));
            match result {
                Ok(code) => Some(code),
                Err(_) => None,
            }
        };

        CompileResult {
            name: "ɵpipe".to_string(),
            initializer: Some(initializer),
            initializer_ast_code,
            statements: vec![],
            type_desc: format!(
                "i0.ɵɵPipeDeclaration<{}, \"{}\", {}>",
                metadata.name, metadata.pipe_name, metadata.standalone
            ),
            deferrable_imports: None,
            diagnostics: Vec::new(),
            additional_imports: Vec::new(),
        }
    }
}

impl Default for PipeDecoratorHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DecoratorHandler<PipeMetadata, PipeMetadata, (), ()> for PipeDecoratorHandler {
    fn name(&self) -> &str {
        "PipeDecoratorHandler"
    }

    fn precedence(&self) -> HandlerPrecedence {
        HandlerPrecedence::Primary
    }

    fn detect(
        &self,
        node: &ClassDeclaration,
        decorators: &[String],
    ) -> Option<DetectResult<PipeMetadata>> {
        if !Self::detect_pipe(decorators) {
            return None;
        }

        // Get class name - use id().map() to get name from OxC Class
        let class_name = node
            .id
            .as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| "AnonymousPipe".to_string());

        // Create basic metadata - actual args parsing happens elsewhere
        let metadata = PipeMetadata::new(class_name.clone());

        Some(DetectResult {
            trigger: Some(class_name),
            decorator: Some("Pipe".to_string()),
            metadata,
        })
    }

    fn analyze(
        &self,
        _node: &ClassDeclaration,
        metadata: &PipeMetadata,
    ) -> AnalysisOutput<PipeMetadata> {
        AnalysisOutput {
            analysis: Some(metadata.clone()),
            diagnostics: None,
        }
    }

    fn symbol(&self, _node: &ClassDeclaration, _analysis: &PipeMetadata) -> Option<()> {
        None
    }

    fn compile_full(
        &self,
        _node: &ClassDeclaration,
        analysis: &PipeMetadata,
        _resolution: Option<&()>,
        _constant_pool: &mut ConstantPool,
    ) -> Vec<CompileResult> {
        vec![Self::compile_pipe(analysis)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_pipe() {
        assert!(PipeDecoratorHandler::detect_pipe(&["Pipe".to_string()]));
        assert!(!PipeDecoratorHandler::detect_pipe(&[
            "Component".to_string()
        ]));
    }

    #[test]
    fn test_pipe_metadata_from_args() {
        let args = serde_json::json!({
            "name": "fullName",
            "pure": true,
            "standalone": true
        });

        let metadata = PipeMetadata::from_args("FullNamePipe".to_string(), &args);

        assert_eq!(metadata.name, "FullNamePipe");
        assert_eq!(metadata.pipe_name, "fullName");
        assert!(metadata.pure);
        assert!(metadata.standalone);
    }

    #[test]
    fn test_compile_pipe() {
        let metadata = PipeMetadata {
            name: "FullNamePipe".to_string(),
            pipe_name: "fullName".to_string(),
            pure: true,
            standalone: true,
        };

        let result = PipeDecoratorHandler::compile_pipe(&metadata);

        assert_eq!(result.name, "ɵpipe");
        assert!(result.initializer.is_some());
        let init = result.initializer.unwrap();
        assert!(init.contains("ɵɵdefinePipe"));
        assert!(init.contains("fullName"));
        assert!(init.contains("FullNamePipe"));
    }
}
