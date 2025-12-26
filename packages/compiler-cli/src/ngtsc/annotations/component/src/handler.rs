use crate::ngtsc::metadata::{extract_directive_metadata, DecoratorMetadata, DirectiveMetadata};
use crate::ngtsc::reflection::{ClassDeclaration, ReflectionHost, TypeScriptReflectionHost};
use crate::ngtsc::transform::src::api::{
    AnalysisOutput, CompileResult, ConstantPool, DecoratorHandler, DetectResult, HandlerPrecedence,
};
use angular_compiler::render3::view::api::{
    DeclarationListEmitMode, R3ComponentDeferMetadata, R3ComponentMetadata, R3ComponentTemplate,
    R3DirectiveMetadata, R3LifecycleMetadata,
};
// use angular_compiler::render3::view::compiler::compile_component_from_metadata;
use angular_compiler::core::ViewEncapsulation;
use angular_compiler::ml_parser::html_whitespaces::{
    visit_all_with_siblings_nodes, WhitespaceVisitor,
};
use angular_compiler::output::abstract_emitter::EmitterVisitorContext;
use angular_compiler::output::abstract_js_emitter::AbstractJsEmitterVisitor;
use angular_compiler::output::output_ast::ExpressionTrait;
use angular_compiler::render3::r3_template_transform::{
    html_ast_to_render3_ast, Render3ParseOptions,
};
use angular_compiler::render3::view::template::{parse_template, ParseTemplateOptions};
// use std::collections::HashMap;
use angular_compiler::template::pipeline::src::compilation::TemplateCompilationMode;
use angular_compiler::template::pipeline::src::emit::emit_component;
use angular_compiler::template::pipeline::src::ingest::ingest_component;
use angular_compiler::template::pipeline::src::phases;
use std::any::Any;
// use angular_compiler::constant_pool::ConstantPool as CompilerConstantPool; // Distinct from ngtsc ConstantPool if needed

/// Get metadata for known Angular directives (NgFor, NgIf, etc.)
/// This is a workaround until proper static analysis of imported modules is implemented.
fn get_known_directive_metadata(
    name: &str,
) -> Option<angular_compiler::render3::view::api::R3DirectiveDependencyMetadata> {
    use angular_compiler::output::output_ast::{Expression, ReadVarExpr};
    use angular_compiler::render3::view::api::{
        R3DirectiveDependencyMetadata, R3TemplateDependencyKind,
    };

    match name {
        "NgForOf" | "NgFor" => Some(R3DirectiveDependencyMetadata {
            selector: "[ngFor][ngForOf]".to_string(),
            type_: Expression::ReadVar(ReadVarExpr {
                name: name.to_string(),
                type_: None,
                source_span: None,
            }),
            inputs: vec![
                "ngForOf".to_string(),
                "ngForTrackBy".to_string(),
                "ngForTemplate".to_string(),
            ],
            outputs: vec![],
            export_as: vec![].into(),
            kind: R3TemplateDependencyKind::Directive,
            is_component: false,
        }),
        "NgIf" => Some(R3DirectiveDependencyMetadata {
            selector: "[ngIf]".to_string(),
            type_: Expression::ReadVar(ReadVarExpr {
                name: name.to_string(),
                type_: None,
                source_span: None,
            }),
            inputs: vec![
                "ngIf".to_string(),
                "ngIfThen".to_string(),
                "ngIfElse".to_string(),
            ],
            outputs: vec![],
            export_as: vec![].into(),
            kind: R3TemplateDependencyKind::Directive,
            is_component: false,
        }),
        "NgSwitch" => Some(R3DirectiveDependencyMetadata {
            selector: "[ngSwitch]".to_string(),
            type_: Expression::ReadVar(ReadVarExpr {
                name: name.to_string(),
                type_: None,
                source_span: None,
            }),
            inputs: vec!["ngSwitch".to_string()],
            outputs: vec![],
            export_as: vec![].into(),
            kind: R3TemplateDependencyKind::Directive,
            is_component: false,
        }),
        "NgSwitchCase" => Some(R3DirectiveDependencyMetadata {
            selector: "[ngSwitchCase]".to_string(),
            type_: Expression::ReadVar(ReadVarExpr {
                name: name.to_string(),
                type_: None,
                source_span: None,
            }),
            inputs: vec!["ngSwitchCase".to_string()],
            outputs: vec![],
            export_as: vec![].into(),
            kind: R3TemplateDependencyKind::Directive,
            is_component: false,
        }),
        "NgSwitchDefault" => Some(R3DirectiveDependencyMetadata {
            selector: "[ngSwitchDefault]".to_string(),
            type_: Expression::ReadVar(ReadVarExpr {
                name: name.to_string(),
                type_: None,
                source_span: None,
            }),
            inputs: vec![],
            outputs: vec![],
            export_as: vec![].into(),
            kind: R3TemplateDependencyKind::Directive,
            is_component: false,
        }),
        "NgClass" => Some(R3DirectiveDependencyMetadata {
            selector: "[ngClass]".to_string(),
            type_: Expression::ReadVar(ReadVarExpr {
                name: name.to_string(),
                type_: None,
                source_span: None,
            }),
            inputs: vec!["ngClass".to_string()],
            outputs: vec![],
            export_as: vec![].into(),
            kind: R3TemplateDependencyKind::Directive,
            is_component: false,
        }),
        "NgStyle" => Some(R3DirectiveDependencyMetadata {
            selector: "[ngStyle]".to_string(),
            type_: Expression::ReadVar(ReadVarExpr {
                name: name.to_string(),
                type_: None,
                source_span: None,
            }),
            inputs: vec!["ngStyle".to_string()],
            outputs: vec![],
            export_as: vec![].into(),
            kind: R3TemplateDependencyKind::Directive,
            is_component: false,
        }),
        "RouterOutlet" => Some(R3DirectiveDependencyMetadata {
            selector: "router-outlet".to_string(),
            type_: Expression::ReadVar(ReadVarExpr {
                name: name.to_string(),
                type_: None,
                source_span: None,
            }),
            inputs: vec!["name".to_string()],
            outputs: vec!["activate".to_string(), "deactivate".to_string()],
            export_as: vec![].into(),
            kind: R3TemplateDependencyKind::Directive,
            is_component: true,
        }),
        _ => None,
    }
}

pub struct ComponentDecoratorHandler;

impl ComponentDecoratorHandler {
    pub fn new() -> Self {
        Self
    }
}

impl DecoratorHandler<DirectiveMetadata<'static>, DirectiveMetadata<'static>, (), ()>
    for ComponentDecoratorHandler
{
    fn name(&self) -> &str {
        "ComponentDecoratorHandler"
    }

    fn precedence(&self) -> HandlerPrecedence {
        HandlerPrecedence::Primary
    }

    fn detect(
        &self,
        node: &ClassDeclaration,
        _decorators: &[String],
    ) -> Option<DetectResult<DirectiveMetadata<'static>>> {
        let reflection_host = TypeScriptReflectionHost::new();
        // unsafe transmute because ClassDeclaration is same as Declaration for our purposes here
        let decl =
            oxc_ast::ast::Declaration::ClassDeclaration(unsafe { std::mem::transmute(node) });
        let converted_decorators = reflection_host.get_decorators_of_declaration(&decl);

        for decorator in converted_decorators {
            if decorator.name == "Component" {
                if let Some(metadata) =
                    extract_directive_metadata(node, &decorator, true, std::path::Path::new(""))
                {
                    // Clear the decorator reference to avoid lifetime issues
                    let owned_metadata = match metadata {
                        DecoratorMetadata::Directive(mut d) => {
                            d.decorator = None; // Clear the lifetime-bound reference
                            DecoratorMetadata::Directive(d)
                        }
                        other => other,
                    };
                    // Safety: We cleared the decorator reference, so there's no dangling pointer
                    let static_metadata: DirectiveMetadata<'static> =
                        unsafe { std::mem::transmute(owned_metadata) };
                    return Some(DetectResult {
                        trigger: Some("Component".to_string()),
                        decorator: Some("Component".to_string()),
                        metadata: static_metadata,
                    });
                }
            }
        }
        None
    }

    fn analyze(
        &self,
        _node: &ClassDeclaration,
        metadata: &DirectiveMetadata<'static>,
    ) -> AnalysisOutput<DirectiveMetadata<'static>> {
        AnalysisOutput::of(metadata.clone())
    }

    fn symbol(
        &self,
        _node: &ClassDeclaration,
        _analysis: &DirectiveMetadata<'static>,
    ) -> Option<()> {
        None
    }

    fn compile_full(
        &self,
        _node: &ClassDeclaration,
        analysis: &DirectiveMetadata<'static>,
        _resolution: Option<&()>,
        _constant_pool: &mut ConstantPool,
    ) -> Vec<CompileResult> {
        self.compile_ivy(analysis)
    }
}

impl ComponentDecoratorHandler {
    pub fn compile_ivy(&self, analysis: &DirectiveMetadata<'static>) -> Vec<CompileResult> {
        // Extract DirectiveMeta from DecoratorMetadata enum (must be a component)
        let dir = match analysis {
            DecoratorMetadata::Directive(d) if d.t2.is_component => d,
            _ => return vec![], // Not a component, cannot compile
        };

        // Manually construct R3Reference since From isn't implemented
        let type_expr = angular_compiler::output::output_ast::Expression::ReadVar(
            angular_compiler::output::output_ast::ReadVarExpr {
                name: dir.t2.name.clone(),
                type_: None,
                source_span: None,
            },
        );

        let type_ref = angular_compiler::render3::util::R3Reference {
            value: type_expr.clone(),
            type_expr: type_expr,
        };

        let comp_meta = dir.component.as_ref().unwrap();

        // Parse Template
        let template_str = comp_meta.template.clone().unwrap_or_else(|| "".to_string());
        let template_url = comp_meta
            .template_url
            .clone()
            .unwrap_or_else(|| "inline-template.html".to_string());

        let expression_parser = angular_compiler::expression_parser::parser::Parser::new();
        let schema_registry =
            angular_compiler::schema::dom_element_schema_registry::DomElementSchemaRegistry::new();
        let mut binding_parser =
            angular_compiler::template_parser::binding_parser::BindingParser::new(
                &expression_parser,
                &schema_registry,
                vec![],
            );

        let (nodes, ng_content_selectors, preserve_whitespaces, styles) = if let Some(ast) =
            comp_meta.template_ast.as_ref()
        {
            println!(
                "DEBUG: compile_ivy for {} - template_ast has {} nodes",
                dir.t2.name,
                ast.len()
            );
            let options = Render3ParseOptions {
                collect_comment_nodes: false,
                ..Default::default()
            };

            // Apply whitespace visitor
            let mut visitor = WhitespaceVisitor::new(true, None, false);
            let processed_nodes = visit_all_with_siblings_nodes(&mut visitor, ast);
            println!(
                "DEBUG: compile_ivy for {} - after whitespace visitor: {} nodes",
                dir.t2.name,
                processed_nodes.len()
            );

            let result = html_ast_to_render3_ast(&processed_nodes, &mut binding_parser, &options);
            println!(
                "DEBUG: compile_ivy for {} - after R3 transform: {} nodes",
                dir.t2.name,
                result.nodes.len()
            );
            (
                result.nodes,
                result.ng_content_selectors,
                false,
                result.styles,
            )
        } else {
            let parsed_template = parse_template(
                &template_str,
                &template_url,
                ParseTemplateOptions {
                    preserve_whitespaces: Some(false),
                    ..Default::default()
                },
            );
            (
                parsed_template.nodes,
                parsed_template.ng_content_selectors,
                parsed_template.preserve_whitespaces.unwrap_or(false),
                parsed_template.styles,
            )
        };

        // TODO: Handle parsing errors?
        // if let Some(errors) = parsed_template.errors { ... }

        let r3_metadata = R3ComponentMetadata {
            directive: R3DirectiveMetadata {
                name: dir.t2.name.clone(),
                type_: type_ref,
                type_argument_count: 0,
                type_source_span: angular_compiler::parse_util::ParseSourceSpan::new(
                    angular_compiler::parse_util::ParseLocation::new(angular_compiler::parse_util::ParseSourceFile::new("".to_string(), "".to_string()), 0, 0, 0),
                    angular_compiler::parse_util::ParseLocation::new(angular_compiler::parse_util::ParseSourceFile::new("".to_string(), "".to_string()), 0, 0, 0)
                ),
                selector: dir.t2.selector.clone(),
                queries: vec![],
                view_queries: vec![],
                host: angular_compiler::render3::view::api::R3HostMetadata::default(),
                inputs: dir.t2.inputs.iter().map(|(k, v)| (k.clone(), angular_compiler::render3::view::api::R3InputMetadata {
                    class_property_name: v.class_property_name.clone(),
                    binding_property_name: v.binding_property_name.clone(),
                    is_signal: v.is_signal,
                    required: false,
                    transform_function: None,
                })).collect(),
                outputs: dir.t2.outputs.iter().map(|(k, v)| (k.clone(), v.binding_property_name.clone())).collect(),
                lifecycle: R3LifecycleMetadata::default(),
                providers: None,
                uses_inheritance: false,
                export_as: dir.t2.export_as.clone(),
                is_standalone: dir.is_standalone,
                is_signal: dir.is_signal,
                host_directives: None,
                deps: None,
            },
            template: R3ComponentTemplate {
                ng_content_selectors: ng_content_selectors,
                nodes: nodes.clone(), // Clone for pipeline ingestion
                preserve_whitespaces: preserve_whitespaces,
            },
            declarations: dir.imports.iter().flatten().map(|import_ref| {
                let import_name = import_ref.debug_name().to_string();
                // Try to get known directive metadata first, fall back to empty values
                if let Some(known_metadata) = get_known_directive_metadata(&import_name) {
                    angular_compiler::render3::view::api::R3TemplateDependencyMetadata::Directive(known_metadata)
                } else {
                    angular_compiler::render3::view::api::R3TemplateDependencyMetadata::Directive(angular_compiler::render3::view::api::R3DirectiveDependencyMetadata {
                        selector: "".to_string(), // Selector would need full analysis
                        type_: angular_compiler::output::output_ast::Expression::ReadVar(angular_compiler::output::output_ast::ReadVarExpr {
                            name: import_name.clone(),
                            type_: None,
                            source_span: None,
                        }),
                        inputs: vec![],
                        outputs: vec![],
                        export_as: vec![].into(),
                        kind: angular_compiler::render3::view::api::R3TemplateDependencyKind::Directive,
                        is_component: false,
                    })
                }
            }).collect(),
            declaration_list_emit_mode: DeclarationListEmitMode::Direct,
            styles: {
                let mut combined = comp_meta.styles.clone().unwrap_or_default();
                combined.extend(styles);
                combined
            },
            encapsulation: ViewEncapsulation::Emulated,
            change_detection: comp_meta.change_detection.map(|s| angular_compiler::render3::view::api::ChangeDetectionOrExpression::Strategy(s)),
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

        let real_constant_pool = angular_compiler::constant_pool::ConstantPool::new(false);

        // Use template pipeline instead of placeholder compile_component_from_metadata
        // 1. Ingest template into compilation job
        let mut job = ingest_component(
            dir.t2.name.clone(),
            nodes, // Template AST nodes
            real_constant_pool,
            TemplateCompilationMode::Full,
            r3_metadata.relative_context_file_path.clone(),
            r3_metadata.i18n_use_external_ids,
            r3_metadata.defer.clone(),
            None, // all_deferrable_deps_fn
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
    use crate::ngtsc::metadata::{ClassPropertyMapping, DirectiveMeta};
    use crate::ngtsc::transform::src::api::HandlerPrecedence;

    #[test]
    fn test_handler_basic_properties() {
        let handler = ComponentDecoratorHandler::new();
        assert_eq!(handler.name(), "ComponentDecoratorHandler");
        assert!(matches!(handler.precedence(), HandlerPrecedence::Primary));
    }

    #[test]
    fn test_compile_full_basic() {
        // Mock a DirectiveMetadata using the new structure
        let metadata = DecoratorMetadata::Directive(DirectiveMeta {
            t2: T2DirectiveMetadata {
                name: "TestComponent".to_string(),
                selector: Some("test-comp".to_string()),
                is_component: true,
                ..Default::default()
            },
            component: Some(ComponentMetadata {
                template: Some("<div>Hello World</div>".to_string()),
                ..Default::default()
            }),
            is_standalone: true,
            is_signal: false,
            source_file: None,
            ..Default::default()
        });

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
        assert!(initializer.contains("selectors: [['test-comp']]"));
        assert!(initializer.contains("decls: 2"));
        assert!(initializer.contains("vars: 0"));
    }
}
