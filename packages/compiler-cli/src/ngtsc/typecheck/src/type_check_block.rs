// TypeCheck Block Generation
//
// Generates type-check blocks for templates.

use std::fmt::Write;
use super::super::api::{TypeCheckError, TypeCheckingConfig};

/// Generates a type-check block (TCB) for a component template.
pub struct TypeCheckBlockGenerator {
    /// Configuration.
    config: TypeCheckingConfig,
    /// Output buffer.
    output: String,
    /// Indentation level.
    indent: usize,
}

impl TypeCheckBlockGenerator {
    pub fn new(config: TypeCheckingConfig) -> Self {
        Self {
            config,
            output: String::new(),
            indent: 0,
        }
    }
    
    /// Generate TCB for a component.
    pub fn generate(&mut self, component_name: &str, template: &str) -> Result<String, TypeCheckError> {
        self.output.clear();
        
        // Generate function signature
        self.write_line(&format!("function _tcb_{}(ctx: {}) {{", component_name, component_name));
        self.indent += 1;
        
        // For now, generate placeholder
        self.write_line("// Template type-check block");
        self.write_line(&format!("// Template: {}", template.replace('\n', " ")));
        
        // Close function
        self.indent -= 1;
        self.write_line("}");
        
        Ok(self.output.clone())
    }
    
    /// Generate element type-check.
    pub fn generate_element(&mut self, tag: &str, attrs: &[(String, String)]) {
        self.write_line(&format!("// Element: <{}>", tag));
        for (name, value) in attrs {
            self.write_line(&format!("// Attr: {}=\"{}\"", name, value));
        }
    }
    
    /// Generate directive type-check.
    pub fn generate_directive(&mut self, directive_name: &str, inputs: &[(String, String)]) {
        self.write_line(&format!("const _dir = new {}();", directive_name));
        for (input, value) in inputs {
            self.write_line(&format!("_dir.{} = {};", input, value));
        }
    }
    
    /// Generate pipe type-check.
    pub fn generate_pipe(&mut self, pipe_name: &str, args: &[String]) {
        let args_str = args.join(", ");
        self.write_line(&format!("const _pipe = new {}();", pipe_name));
        self.write_line(&format!("_pipe.transform({});", args_str));
    }
    
    fn write_line(&mut self, line: &str) {
        let indent = "  ".repeat(self.indent);
        writeln!(self.output, "{}{}", indent, line).ok();
    }
}

/// Out-of-band checker for template errors.
pub struct OutOfBandDiagnosticRecorder {
    /// Collected diagnostics.
    diagnostics: Vec<TypeCheckError>,
}

impl OutOfBandDiagnosticRecorder {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }
    
    /// Record a missing pipe error.
    pub fn missing_pipe(&mut self, component: &str, pipe_name: &str) {
        self.diagnostics.push(TypeCheckError {
            message: format!("The pipe '{}' could not be found", pipe_name),
            code: "NG8004".to_string(),
            file: Some(component.to_string()),
            start: None,
            length: None,
        });
    }
    
    /// Record a missing directive error.
    pub fn missing_directive(&mut self, component: &str, selector: &str) {
        self.diagnostics.push(TypeCheckError {
            message: format!("There is no directive with selector '{}'", selector),
            code: "NG8002".to_string(),
            file: Some(component.to_string()),
            start: None,
            length: None,
        });
    }
    
    /// Get all diagnostics.
    pub fn diagnostics(&self) -> &[TypeCheckError] {
        &self.diagnostics
    }
}

impl Default for OutOfBandDiagnosticRecorder {
    fn default() -> Self {
        Self::new()
    }
}
