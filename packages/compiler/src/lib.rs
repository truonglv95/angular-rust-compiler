#![deny(clippy::all)]

/**
 * Angular Rust Compiler - NAPI-RS Implementation
 *
 * High-performance Angular compiler written in Rust with Node.js bindings
 */

#[cfg(feature = "napi-bindings")]
use napi::bindgen_prelude::*;
#[cfg(feature = "napi-bindings")]
use napi_derive::napi;

#[cfg(feature = "napi-bindings")]
use crate::expression_parser::{Lexer, Parser as ExprParser, serialize};

// Core modules (root level - mirrors packages/compiler/src/*.ts)
mod assertions;
pub mod chars;
pub mod combined_visitor;
pub mod compiler;
pub mod compiler_facade_interface;
mod config;
pub mod constant_pool;
pub mod core;
pub mod directive_matching;
mod error;
mod injectable_compiler_2;
mod jit_compiler_facade;
pub mod parse_util;
mod resource_loader;
pub mod shadow_css;
pub mod style_url_resolver;
pub mod util;
mod version;

// Parser modules (mirrors Angular structure)
pub mod expression_parser;
pub mod ml_parser;
pub mod template_parser;
pub mod template;  // Template compilation pipeline

// Compilation modules
pub mod render3;
pub mod i18n;
pub mod output;
pub mod schema;

// Re-exports
pub use config::CompilerConfig as RustCompilerConfig;
pub use util::Version;
pub use version::VERSION;

use error::Result as CompilerResult;


/// Compiler configuration
#[cfg_attr(feature = "napi-bindings", napi(object))]
#[cfg_attr(not(feature = "napi-bindings"), derive(Debug))]
pub struct CompilerConfig {
    /// Enable debug mode
    pub debug: Option<bool>,
    /// Preserve whitespaces
    pub preserve_whitespaces: Option<bool>,
    /// Strict mode
    pub strict: Option<bool>,
}

/// Component metadata
#[cfg_attr(feature = "napi-bindings", napi(object))]
#[cfg_attr(not(feature = "napi-bindings"), derive(Debug))]
pub struct ComponentMetadata {
    /// Component template
    pub template: String,
    /// Component selector
    pub selector: Option<String>,
    /// Component name
    pub name: String,
    /// Styles
    pub styles: Option<Vec<String>>,
}

/// Compilation result
#[cfg_attr(feature = "napi-bindings", napi(object))]
#[cfg_attr(not(feature = "napi-bindings"), derive(Debug))]
pub struct CompilationResult {
    /// Generated JavaScript code
    pub js_code: String,
    /// Source map (optional)
    pub source_map: Option<String>,
    /// Compilation time in milliseconds
    pub compilation_time: f64,
    /// Success flag
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Benchmark statistics
#[cfg_attr(feature = "napi-bindings", napi(object))]
#[cfg_attr(not(feature = "napi-bindings"), derive(Debug))]
pub struct BenchmarkStats {
    pub avg: f64,
    pub min: f64,
    pub max: f64,
    pub median: f64,
}

/// Parse HTML template
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn parse_template(template: String) -> Result<String> {
    let start = std::time::Instant::now();

    // Use real HTML parser
    use ml_parser::parser::Parser;
    use ml_parser::html_tags::get_html_tag_definition;
    use ml_parser::tags::TagDefinition;

    fn tag_def(name: &str) -> &'static dyn TagDefinition {
        get_html_tag_definition(name)
    }

    let parser = Parser::new(tag_def);
    let parse_result = parser.parse(&template, "template.html", None);

    let elapsed = start.elapsed().as_micros() as f64 / 1000.0;

    // Return summary
    let result = serde_json::json!({
        "success": parse_result.errors.is_empty(),
        "nodes": parse_result.root_nodes.len(),
        "errors": parse_result.errors.len(),
        "time": elapsed
    });

    Ok(result.to_string())
}

/// Parse HTML template and return full AST
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn parse_template_full(template: String) -> Result<String> {
    let start = std::time::Instant::now();

    // Use real HTML parser
    use ml_parser::parser::Parser;
    use ml_parser::html_tags::get_html_tag_definition;
    use ml_parser::tags::TagDefinition;

    fn tag_def(name: &str) -> &'static dyn TagDefinition {
        get_html_tag_definition(name)
    }

    let parser = Parser::new(tag_def);
    let parse_result = parser.parse(&template, "template.html", None);

    let elapsed = start.elapsed().as_micros() as f64 / 1000.0;

    // Serialize nodes to simplified format (limited to prevent overflow)
    let node_count = parse_result.root_nodes.len();
    let nodes_json: Vec<serde_json::Value> = parse_result.root_nodes.iter()
        .take(10) // Limit to first 10 root nodes
        .map(|node| node_to_json(node))
        .collect();

    let errors_json: Vec<serde_json::Value> = parse_result.errors.iter().map(|err| {
        serde_json::json!({
            "message": err.msg.clone(),
            "line": err.span.start.line,
            "col": err.span.start.col,
            "offset": err.span.start.offset
        })
    }).collect();

    let result = serde_json::json!({
        "success": parse_result.errors.is_empty(),
        "nodeCount": node_count,
        "nodes": nodes_json,
        "errors": errors_json,
        "time": elapsed
    });

    Ok(result.to_string())
}

// Helper to convert AST Node to JSON (limited depth to prevent stack overflow)
fn node_to_json(node: &ml_parser::ast::Node) -> serde_json::Value {
    node_to_json_depth(node, 0, 10) // Max depth 10
}

fn node_to_json_depth(node: &ml_parser::ast::Node, depth: usize, max_depth: usize) -> serde_json::Value {
    use ml_parser::ast::Node;

    if depth >= max_depth {
        return serde_json::json!({"type": "MaxDepthReached"});
    }

    match node {
        Node::Element(el) => {
            // Limit children serialization to prevent stack overflow
            let children = if depth < max_depth - 1 && el.children.len() < 20 {
                el.children.iter()
                    .take(10) // Only serialize first 10 children
                    .map(|c| node_to_json_depth(c, depth + 1, max_depth))
                    .collect()
            } else {
                vec![]
            };

            serde_json::json!({
                "type": "Element",
                "name": el.name,
                "attrs": el.attrs.iter().take(10).map(|a| serde_json::json!({
                    "name": a.name,
                    "value": if a.value.len() > 50 {
                        format!("{}...", &a.value[..50.min(a.value.len())])
                    } else {
                        a.value.clone()
                    }
                })).collect::<Vec<_>>(),
                "childCount": el.children.len(),
                "children": children
            })
        }
        Node::Text(text) => {
            let value = if text.value.len() > 100 {
                format!("{}... ({} chars)", &text.value[..50], text.value.len())
            } else {
                text.value.clone()
            };
            serde_json::json!({
                "type": "Text",
                "value": value
            })
        }
        Node::Comment(comment) => {
            let value = if let Some(ref v) = comment.value {
                if v.len() > 100 {
                    format!("{}...", &v[..50])
                } else {
                    v.clone()
                }
            } else {
                String::new()
            };
            serde_json::json!({
                "type": "Comment",
                "value": value
            })
        }
        Node::Block(block) => {
            // Limit children serialization
            let children = if depth < max_depth - 1 && block.children.len() < 20 {
                block.children.iter()
                    .take(10)
                    .map(|c| node_to_json_depth(c, depth + 1, max_depth))
                    .collect()
            } else {
                vec![]
            };

            serde_json::json!({
                "type": "Block",
                "name": block.name,
                "paramCount": block.parameters.len(),
                "childCount": block.children.len(),
                "children": children
            })
        }
        Node::LetDeclaration(let_decl) => {
            serde_json::json!({
                "type": "LetDeclaration",
                "name": let_decl.name,
                "value": if let_decl.value.len() > 100 {
                    format!("{}...", &let_decl.value[..50])
                } else {
                    let_decl.value.clone()
                }
            })
        }
        Node::Component(comp) => {
            serde_json::json!({
                "type": "Component",
                "name": comp.component_name,
                "childCount": comp.children.len()
            })
        }
        Node::Expansion(exp) => {
            serde_json::json!({
                "type": "Expansion",
                "switchValue": exp.switch_value,
                "caseCount": exp.cases.len()
            })
        }
        Node::ExpansionCase(case) => {
            serde_json::json!({
                "type": "ExpansionCase",
                "value": case.value
            })
        }
        Node::Attribute(attr) => {
            serde_json::json!({
                "type": "Attribute",
                "name": attr.name,
                "value": attr.value
            })
        }
        Node::Directive(dir) => {
            serde_json::json!({
                "type": "Directive",
                "name": dir.name
            })
        }
        Node::BlockParameter(param) => {
            serde_json::json!({
                "type": "BlockParameter",
                "expression": param.expression
            })
        }
    }
}

/// Parse Angular expressions (using real Rust lexer)
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn parse_expressions(template: String) -> Result<String> {
    let start = std::time::Instant::now();

    // Use real Rust lexer
    let lexer = Lexer::new();
    let tokens = lexer.tokenize(&template);

    let elapsed = start.elapsed().as_micros() as f64 / 1000.0;

    // Return summary
    let result = serde_json::json!({
        "count": tokens.len(),
        "time": elapsed
    });

    Ok(result.to_string())
}

/// Tokenize expression (direct lexer access)
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn tokenize_expression(expression: String) -> Result<String> {
    let lexer = Lexer::new();
    let tokens = lexer.tokenize(&expression);

    let result = serde_json::json!({
        "count": tokens.len()
    });

    Ok(result.to_string())
}

/// Parse expression to AST
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn parse_expression_to_ast(expression: String) -> Result<String> {
    let start = std::time::Instant::now();

    let parser = ExprParser::new();
    let ast = parser.parse_binding(&expression, 0)?;

    let elapsed = start.elapsed().as_micros() as f64 / 1000.0;

    // Serialize AST back to string (like Angular serializer)
    let serialized = serialize(&ast);
    let ast_type = get_ast_type(&ast);

    let result = serde_json::json!({
        "type": ast_type,
        "expression": serialized,
        "original": expression,
        "time": elapsed
    });

    Ok(result.to_string())
}

// Helper to get AST type name
fn get_ast_type(ast: &expression_parser::AST) -> &'static str {
    use expression_parser::AST;

    match ast {
        AST::Binary(_) => "Binary",
        AST::PropertyRead(_) => "PropertyRead",
        AST::SafePropertyRead(_) => "SafePropertyRead",
        AST::PropertyWrite(_) => "PropertyWrite",
        AST::KeyedRead(_) => "KeyedRead",
        AST::KeyedWrite(_) => "KeyedWrite",
        AST::SafeKeyedRead(_) => "SafeKeyedRead",
        AST::Conditional(_) => "Conditional",
        AST::BindingPipe(_) => "BindingPipe",
        AST::LiteralArray(_) => "LiteralArray",
        AST::LiteralMap(_) => "LiteralMap",
        AST::LiteralPrimitive(_) => "LiteralPrimitive",
        AST::Interpolation(_) => "Interpolation",
        AST::Call(_) => "Call",
        AST::SafeCall(_) => "SafeCall",
        AST::Chain(_) => "Chain",
        AST::PrefixNot(_) => "PrefixNot",
        AST::Unary(_) => "Unary",
        AST::TypeofExpression(_) => "TypeofExpression",
        AST::VoidExpression(_) => "VoidExpression",
        AST::NonNullAssert(_) => "NonNullAssert",
        AST::TemplateLiteral(_) => "TemplateLiteral",
        AST::TaggedTemplateLiteral(_) => "TaggedTemplateLiteral",
        AST::ParenthesizedExpression(_) => "ParenthesizedExpression",
        AST::RegularExpressionLiteral(_) => "RegularExpressionLiteral",
        AST::ImplicitReceiver(_) => "ImplicitReceiver",
        AST::ThisReceiver(_) => "ThisReceiver",
        AST::EmptyExpr(_) => "EmptyExpr",
    }
}

/// Process template through pipeline
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn process_pipeline(template_ast: String) -> Result<String> {
    let start = std::time::Instant::now();

    // TODO: Implement real pipeline processing
    let result = format!("Processed pipeline for: {}", template_ast);

    let elapsed = start.elapsed().as_micros() as f64 / 1000.0;

    Ok(format!("{{\"ir\": \"{}\", \"time\": {}ms}}", result, elapsed))
}

/// Generate JavaScript code
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn generate_code(ir: String) -> Result<String> {
    let start = std::time::Instant::now();

    // TODO: Implement real code generation
    let result = format!("function() {{ /* Generated from: {} */ }}", ir);

    let elapsed = start.elapsed().as_micros() as f64 / 1000.0;

    Ok(format!("{{\"code\": \"{}\", \"time\": {}ms}}", result, elapsed))
}

/// Compile component (full pipeline)
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn compile_component(metadata: ComponentMetadata, config: Option<CompilerConfig>) -> Result<CompilationResult> {
    let start = std::time::Instant::now();

    let _config = config.unwrap_or(CompilerConfig {
        debug: Some(false),
        preserve_whitespaces: Some(false),
        strict: Some(true),
    });

    // Full compilation pipeline
    let template_ast = parse_template_internal(&metadata.template)?;
    let expressions = parse_expressions_internal(&template_ast)?;
    let ir = process_pipeline_internal(&expressions)?;
    let js_code = generate_code_internal(&ir)?;

    let elapsed = start.elapsed().as_micros() as f64 / 1000.0;

    Ok(CompilationResult {
        js_code,
        source_map: None,
        compilation_time: elapsed,
        success: true,
        error: None,
    })
}

/// Benchmark compilation performance
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn benchmark_compilation(
    metadata: ComponentMetadata,
    iterations: Option<u32>,
) -> Result<BenchmarkStats> {
    let iterations = iterations.unwrap_or(10);
    let mut times: Vec<f64> = Vec::new();

    for _ in 0..iterations {
        let start = std::time::Instant::now();
        let _ = compile_component(metadata.clone(), None)?;
        let elapsed = start.elapsed().as_micros() as f64 / 1000.0;
        times.push(elapsed);
    }

    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let sum: f64 = times.iter().sum();

    Ok(BenchmarkStats {
        avg: sum / times.len() as f64,
        min: times[0],
        max: times[times.len() - 1],
        median: times[times.len() / 2],
    })
}

/// Get compiler version
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Check if Rust compiler is available
#[cfg(feature = "napi-bindings")]
#[napi]
pub fn is_available() -> bool {
    true
}

// Internal implementations
fn parse_template_internal(template: &str) -> CompilerResult<String> {
    // TODO: Implement real HTML parser
    Ok(format!("{{\"type\":\"template\",\"content\":\"{}\"}}", template))
}

fn parse_expressions_internal(template_ast: &str) -> CompilerResult<String> {
    // TODO: Implement real expression parser
    Ok(format!("{{\"type\":\"expressions\",\"ast\":{}}}", template_ast))
}

fn process_pipeline_internal(expressions: &str) -> CompilerResult<String> {
    // TODO: Implement real pipeline processing
    Ok(format!("{{\"type\":\"ir\",\"data\":{}}}", expressions))
}

fn generate_code_internal(ir: &str) -> CompilerResult<String> {
    // TODO: Implement real code generation
    Ok(format!(
        r#"
        function Component() {{
            // Generated from IR: {}
            return {{
                render: function() {{
                    // Template rendering code
                }}
            }};
        }}
        "#,
        ir
    ))
}

// Helper trait to make ComponentMetadata cloneable for benchmarks
impl Clone for ComponentMetadata {
    fn clone(&self) -> Self {
        ComponentMetadata {
            template: self.template.clone(),
            selector: self.selector.clone(),
            name: self.name.clone(),
            styles: self.styles.clone(),
        }
    }
}

#[cfg(test)]
mod tests {


    #[cfg(feature = "napi-bindings")]
    #[test]
    fn test_parse_template() {
        let result = parse_template("<div>Hello</div>".to_string());
        assert!(result.is_ok());
    }

    #[cfg(feature = "napi-bindings")]
    #[test]
    fn test_compile_component() {
        let metadata = ComponentMetadata {
            template: "<div>{{message}}</div>".to_string(),
            selector: Some("app-test".to_string()),
            name: "TestComponent".to_string(),
            styles: None,
        };

        let result = compile_component(metadata, None);
        assert!(result.is_ok());

        let compiled = result.unwrap();
        assert!(compiled.success);
        assert!(!compiled.js_code.is_empty());
    }

    #[cfg(feature = "napi-bindings")]
    #[test]
    fn test_version() {
        let version = get_version();
        assert!(!version.is_empty());
    }

    #[cfg(feature = "napi-bindings")]
    #[test]
    fn test_availability() {
        assert!(is_available());
    }
}
