use crate::ngtsc::transform::src::api::{DecoratorHandler, HandlerPrecedence, DetectResult, AnalysisOutput, CompileResult, ConstantPool};
use crate::ngtsc::reflection::{ClassDeclaration, Decorator, ReflectionHost, TypeScriptReflectionHost};
use crate::ngtsc::metadata::{DirectiveMetadata, extract_directive_metadata};
use angular_compiler::render3::view::api::{
    R3ComponentMetadata, R3DirectiveMetadata, R3ComponentTemplate, R3ComponentDeferMetadata, DeclarationListEmitMode,
    R3LifecycleMetadata
};
use angular_compiler::render3::view::compiler::compile_component_from_metadata;
use angular_compiler::render3::view::template::{parse_template, ParseTemplateOptions};
use angular_compiler::render3::r3_template_transform::{html_ast_to_render3_ast, Render3ParseOptions};
use angular_compiler::core::{ViewEncapsulation};
use angular_compiler::output::output_ast::{WrappedNodeExpr, ExpressionVisitor, ExpressionTrait};
use angular_compiler::output::abstract_js_emitter::AbstractJsEmitterVisitor;
use angular_compiler::output::abstract_emitter::EmitterVisitorContext;
use angular_compiler::ml_parser::html_whitespaces::{WhitespaceVisitor, visit_all_with_siblings_nodes};
use std::collections::HashMap;
use std::any::Any;
use angular_compiler::template::pipeline::src::ingest::ingest_component;
use angular_compiler::template::pipeline::src::phases;
use angular_compiler::template::pipeline::src::emit::emit_component;
use angular_compiler::template::pipeline::src::compilation::TemplateCompilationMode;
use angular_compiler::constant_pool::ConstantPool as CompilerConstantPool; // Distinct from ngtsc ConstantPool if needed

pub struct ComponentDecoratorHandler;

impl ComponentDecoratorHandler {
    pub fn new() -> Self {
        Self
    }
}

impl DecoratorHandler<DirectiveMetadata, DirectiveMetadata, (), ()> for ComponentDecoratorHandler {
    fn name(&self) -> &str {
        "ComponentDecoratorHandler"
    }

    fn precedence(&self) -> HandlerPrecedence {
        HandlerPrecedence::Primary
    }

    fn detect(&self, node: &ClassDeclaration, _decorators: &[String]) -> Option<DetectResult<DirectiveMetadata>> {
        let reflection_host = TypeScriptReflectionHost::new();
        // unsafe transmute because ClassDeclaration is same as Declaration for our purposes here
        let decl = oxc_ast::ast::Declaration::ClassDeclaration(unsafe { std::mem::transmute(node) });
        let converted_decorators = reflection_host.get_decorators_of_declaration(&decl);

        for decorator in converted_decorators {
            if decorator.name == "Component" {
                 if let Some(metadata) = extract_directive_metadata(node, &decorator, true) {
                     return Some(DetectResult {
                         trigger: Some("Component".to_string()),
                         decorator: Some("Component".to_string()),
                         metadata,
                     });
                 }
            }
        }
        None
    }

    fn analyze(&self, _node: &ClassDeclaration, metadata: &DirectiveMetadata) -> AnalysisOutput<DirectiveMetadata> {
        AnalysisOutput::of(metadata.clone())
    }

    fn symbol(&self, _node: &ClassDeclaration, _analysis: &DirectiveMetadata) -> Option<()> {
        None
    }

    fn compile_full(
        &self,
        _node: &ClassDeclaration,
        analysis: &DirectiveMetadata,
        _resolution: Option<&()>,
        _constant_pool: &mut ConstantPool,
    ) -> Vec<CompileResult> {
        self.compile_ivy(analysis)
    }
}

impl ComponentDecoratorHandler {
    pub fn compile_ivy(&self, analysis: &DirectiveMetadata) -> Vec<CompileResult> {
        // Manually construct R3Reference since From isn't implemented
        // Use ReadVarExpr to avoid cloning issues with WrappedNodeExpr
        let type_expr = angular_compiler::output::output_ast::Expression::ReadVar(angular_compiler::output::output_ast::ReadVarExpr {
            name: analysis.name.clone(),
            type_: None,
            source_span: None,
        });

        let type_ref = angular_compiler::render3::util::R3Reference {
            value: type_expr.clone(),
            type_expr: type_expr,
        };

        // Parse Template
        let template_str = analysis.template.clone().unwrap_or_else(|| {
            // Placeholder: Should read from template_url if template is missing
            // For now, default to empty string if neither present (or handle error)
            "".to_string()
        });
        
        let template_url = analysis.template_url.clone().unwrap_or_else(|| "inline-template.html".to_string());

        let expression_parser = angular_compiler::expression_parser::parser::Parser::new();
        let schema_registry = angular_compiler::schema::dom_element_schema_registry::DomElementSchemaRegistry::new();
        let mut binding_parser = angular_compiler::template_parser::binding_parser::BindingParser::new(
             &expression_parser,
             &schema_registry,
             vec![], 
        );

        let (nodes, ng_content_selectors, preserve_whitespaces, styles) = if let Some(ast) = &analysis.template_ast {
            let options = Render3ParseOptions {
                collect_comment_nodes: false,
                ..Default::default()
            };
            
            // Apply whitespace visitor
            let mut visitor = WhitespaceVisitor::new(false, None, false);
            let processed_nodes = visit_all_with_siblings_nodes(&mut visitor, ast);
            
            let result = html_ast_to_render3_ast(&processed_nodes, &mut binding_parser, &options);
            (result.nodes, result.ng_content_selectors, false, result.styles)
        } else {
            let parsed_template = parse_template(
                &template_str,
                &template_url,
                ParseTemplateOptions {
                    preserve_whitespaces: Some(false),
                    ..Default::default()
                }
            );
            (parsed_template.nodes, parsed_template.ng_content_selectors, parsed_template.preserve_whitespaces.unwrap_or(false), parsed_template.styles)
        };

        // TODO: Handle parsing errors?
        // if let Some(errors) = parsed_template.errors { ... }


        let r3_metadata = R3ComponentMetadata {
            directive: R3DirectiveMetadata {
                name: analysis.name.clone(),
                type_: type_ref,
                type_argument_count: 0,
                type_source_span: angular_compiler::parse_util::ParseSourceSpan::new(
                    angular_compiler::parse_util::ParseLocation::new(angular_compiler::parse_util::ParseSourceFile::new("".to_string(), "".to_string()), 0, 0, 0),
                    angular_compiler::parse_util::ParseLocation::new(angular_compiler::parse_util::ParseSourceFile::new("".to_string(), "".to_string()), 0, 0, 0)
                ), 
                selector: analysis.selector.clone(),
                queries: vec![],
                view_queries: vec![],
                host: angular_compiler::render3::view::api::R3HostMetadata::default(),
                inputs: analysis.inputs.iter().map(|(k, v)| (k.clone(), angular_compiler::render3::view::api::R3InputMetadata {
                    class_property_name: v.class_property_name.clone(),
                    binding_property_name: v.binding_property_name.clone(),
                    is_signal: v.is_signal,
                    required: false,
                    transform_function: None,
                })).collect(),
                outputs: analysis.outputs.iter().map(|(k, v)| (k.clone(), v.binding_property_name.clone())).collect(),
                lifecycle: R3LifecycleMetadata::default(),
                providers: None,
                uses_inheritance: false, 
                export_as: analysis.export_as.clone(),
                is_standalone: analysis.is_standalone,
                is_signal: analysis.is_signal,
                host_directives: None,
                deps: None,
            },
            template: R3ComponentTemplate {
                ng_content_selectors: ng_content_selectors,
                nodes: nodes.clone(), // Clone for pipeline ingestion
                preserve_whitespaces: preserve_whitespaces,
            },
            declarations: analysis.imports.iter().flatten().map(|import_name| {
                angular_compiler::render3::view::api::R3TemplateDependencyMetadata::Directive(angular_compiler::render3::view::api::R3DirectiveDependencyMetadata {
                    selector: "".to_string(), // Selector would need full analysis, empty for now to just emit type
                    type_: angular_compiler::output::output_ast::Expression::ReadVar(angular_compiler::output::output_ast::ReadVarExpr {
                        name: import_name.clone(),
                        type_: None,
                        source_span: None,
                    }),
                    inputs: vec![],
                    outputs: vec![],
                    export_as: vec![].into(),
                    kind: angular_compiler::render3::view::api::R3TemplateDependencyKind::Directive,
                    is_component: false, // Default to false if unknown, or true? selector matching handles it. dependencies array just needs type.
                })
            }).collect(),
            declaration_list_emit_mode: DeclarationListEmitMode::Direct,
            styles: {
                let mut combined = analysis.styles.clone().unwrap_or_default();
                combined.extend(styles);
                combined
            },
            encapsulation: ViewEncapsulation::Emulated,
            change_detection: None,
            animations: None,
            view_providers: None,
            relative_context_file_path: "".to_string(),
            i18n_use_external_ids: false,
            raw_imports: None,
            external_styles: None,
            defer: R3ComponentDeferMetadata::PerComponent { dependencies_fn: None },
            relative_template_path: None,
            has_directive_dependencies: false,
        };

        let mut real_constant_pool = angular_compiler::constant_pool::ConstantPool::new(false);
        
        // Use template pipeline instead of placeholder compile_component_from_metadata
        // 1. Ingest template into compilation job
        let mut job = ingest_component(
            analysis.name.clone(),
            nodes,  // Template AST nodes
            real_constant_pool,
            TemplateCompilationMode::Full,
            r3_metadata.relative_context_file_path.clone(),
            r3_metadata.i18n_use_external_ids,
            r3_metadata.defer.clone(),
            None,  // all_deferrable_deps_fn
            r3_metadata.relative_template_path.clone(),
            false, // enable_debug_locations
        );
        
        // 2. Run all pipeline phases
        phases::run(&mut job);
        
        // 3. Emit component definition
        let compiled = emit_component(&job, &r3_metadata);

        
        // Emit AST to String
        let mut emitter = AbstractJsEmitterVisitor::new();
        let mut ctx = EmitterVisitorContext::create_root();
        let context: &mut dyn Any = &mut ctx;
        
        compiled.expression.visit_expression(&mut emitter, context);
        
        let initializer = ctx.to_source();
        
        // Emit statements (hoisted functions like _forTrack)
        let mut emitted_statements = vec![];
        for stmt in &compiled.statements {
            let mut stmt_ctx = EmitterVisitorContext::create_root();
            let stmt_context: &mut dyn Any = &mut stmt_ctx;
            stmt.visit_statement(&mut emitter, stmt_context);
            emitted_statements.push(stmt_ctx.to_source());
        }

        vec![CompileResult {
            name: "ɵcmp".to_string(),
            initializer: Some(initializer),
            statements: emitted_statements, 
            type_desc: "ComponentDef".to_string(),
            deferrable_imports: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ngtsc::transform::src::api::HandlerPrecedence;
    use crate::ngtsc::metadata::ClassPropertyMapping;

    #[test]
    fn test_handler_basic_properties() {
        let handler = ComponentDecoratorHandler::new();
        assert_eq!(handler.name(), "ComponentDecoratorHandler");
        assert!(matches!(handler.precedence(), HandlerPrecedence::Primary));
    }
    
    #[test]
    fn test_compile_full_basic() {
        // Mock a DirectiveMetadata
        let metadata = DirectiveMetadata {
            name: "TestComponent".to_string(),
            selector: Some("test-comp".to_string()),
            is_component: true,
            inputs: ClassPropertyMapping::new(),
            outputs: ClassPropertyMapping::new(),
            export_as: None,
            is_standalone: true,
            is_signal: false,
            template: Some("<div>Hello World</div>".to_string()),
            template_url: None,
            styles: None,
            style_urls: None,
            imports: None,
            template_ast: None,
        };
        
        let handler = ComponentDecoratorHandler::new();
        
        let results = handler.compile_ivy(&metadata);
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.name, "ɵcmp");
        assert!(result.initializer.is_some());
        
        let initializer = result.initializer.as_ref().unwrap();
        println!("DEBUG: Initializer: {}", initializer);
        // Check for key Ivy definition parts
        assert!(initializer.contains("defineComponent"));
        assert!(initializer.contains("selectors: [['', 'test-comp', '']]"));
        assert!(initializer.contains("decls: 2"));
        assert!(initializer.contains("vars: 0"));
    }
}
