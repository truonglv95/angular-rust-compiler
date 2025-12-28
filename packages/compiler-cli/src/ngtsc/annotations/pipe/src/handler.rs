//! Pipe decorator handler
//!
//! Handles @Pipe decorator and generates ɵpipe definition.

use crate::ngtsc::reflection::ClassDeclaration;
use crate::ngtsc::transform::src::api::{
    AnalysisOutput, CompileResult, ConstantPool, DecoratorHandler, DetectResult, HandlerPrecedence,
};

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
        // Build initializer string like Angular does
        let initializer = format!(
            "i0.ɵɵdefinePipe({{ name: '{}', type: {}, pure: {}{} }})",
            metadata.pipe_name,
            metadata.name,
            metadata.pure,
            if metadata.standalone {
                ", standalone: true"
            } else {
                ""
            }
        );

        CompileResult {
            name: "ɵpipe".to_string(),
            initializer: Some(initializer),
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
