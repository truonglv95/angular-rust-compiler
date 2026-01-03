use crate::ngtsc::metadata::{
    extract_directive_metadata, DecoratorMetadata, DirectiveMetadata, ModuleMetadataReader,
};
use crate::ngtsc::reflection::{ClassDeclaration, ReflectionHost, TypeScriptReflectionHost};
use crate::ngtsc::transform::src::api::{
    AnalysisOutput, CompileResult, ConstantPool, DecoratorHandler, DetectResult, HandlerPrecedence,
};
use angular_compiler::core::ViewEncapsulation;
use angular_compiler::ml_parser::html_whitespaces::{
    visit_all_with_siblings_nodes, WhitespaceVisitor,
};
use angular_compiler::output::abstract_emitter::EmitterVisitorContext;
use angular_compiler::output::abstract_js_emitter::AbstractJsEmitterVisitor;
use angular_compiler::output::output_ast::{Expression, ExpressionTrait, ReadVarExpr};
use angular_compiler::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};
use angular_compiler::render3::r3_template_transform::{
    html_ast_to_render3_ast, Render3ParseOptions,
};
use angular_compiler::render3::view::api::{
    DeclarationListEmitMode, R3ComponentDeferMetadata, R3ComponentMetadata, R3ComponentTemplate,
    R3DirectiveMetadata, R3HostMetadata, R3LifecycleMetadata, R3TemplateDependencyMetadata,
};
// use angular_compiler::render3::view::template::{parse_template, ParseTemplateOptions};
// use std::collections::HashMap;
use angular_compiler::template::pipeline::src::compilation::TemplateCompilationMode;
use angular_compiler::template::pipeline::src::emit::emit_component;
use angular_compiler::template::pipeline::src::ingest::{ingest_host_binding, HostBindingInput};
use angular_compiler::template::pipeline::src::phases;
use std::any::Any;
// use std::time::Instant;
// use angular_compiler::constant_pool::ConstantPool as CompilerConstantPool; // Distinct from ngtsc ConstantPool if needed

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
        let class_name = node
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("<anonymous>");

        let reflection_host = TypeScriptReflectionHost::new();
        // unsafe transmute because ClassDeclaration is same as Declaration for our purposes here
        let decl =
            oxc_ast::ast::Declaration::ClassDeclaration(unsafe { std::mem::transmute(node) });
        let converted_decorators = reflection_host.get_decorators_of_declaration(&decl);

        for decorator in converted_decorators {
            if decorator.name == "Component" {
                let empty_imports = std::collections::HashMap::new();
                if let Some(metadata) = extract_directive_metadata(
                    node,
                    &decorator,
                    true,
                    std::path::Path::new(""),
                    &empty_imports,
                ) {
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
                } else {
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
            _ => {
                return vec![];
            }
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

        // Find project root by traversing up to find node_modules
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let project_root = {
            let mut search_dir = cwd.clone();
            loop {
                if search_dir.join("node_modules").exists() {
                    break search_dir;
                }
                if !search_dir.pop() {
                    // Fallback to cwd if no node_modules found
                    break cwd;
                }
            }
        };
        // eprintln!("DEBUG: [handler] Project root for ModuleMetadataReader: {}", project_root.display());
        let metadata_reader = ModuleMetadataReader::new(&project_root);

        let (nodes, ng_content_selectors, preserve_whitespaces, styles) = if let Some(ast) =
            comp_meta.template_ast.as_ref()
        {
            let options = Render3ParseOptions {
                collect_comment_nodes: false,
                ..Default::default()
            };

            // Apply whitespace visitor
            let mut visitor = WhitespaceVisitor::new(true, None, false);
            let processed_nodes = visit_all_with_siblings_nodes(&mut visitor, ast);

            let result = html_ast_to_render3_ast(&processed_nodes, &mut binding_parser, &options);
            // Combine inline styles from template with any style URLs
            let mut combined_styles = result.styles;
            combined_styles.extend(result.style_urls);
            (
                result.nodes,
                result.ng_content_selectors,
                true, // TODO: Get from options
                combined_styles,
            )
        } else {
            let parsed_template = angular_compiler::render3::view::template::parse_template(
                &template_str,
                &template_url,
                angular_compiler::render3::view::template::ParseTemplateOptions {
                    preserve_whitespaces: Some(false),
                    ..Default::default()
                },
            );
            // Combine inline styles from template with any style URLs
            let mut combined_styles = parsed_template.styles;
            combined_styles.extend(parsed_template.style_urls);
            (
                parsed_template.nodes,
                parsed_template.ng_content_selectors,
                parsed_template.preserve_whitespaces.unwrap_or(false),
                combined_styles,
            )
        };

        // TODO: Handle parsing errors?
        // if let Some(errors) = parsed_template.errors { ... }

        // Detect dependencies (directives, pipes, modules) from imports
        let mut declarations_map = indexmap::IndexMap::new();

        if let Some(imports) = &dir.imports {
            // eprintln!("DEBUG: [handler] Processing imports for component: {}, total imports: {}", dir.t2.name, imports.len());
            for import_ref in imports {
                let import_name = import_ref.debug_name().to_string();
                let import_name = import_ref.debug_name().to_string();
                let module_path = import_ref
                    .best_guess_owning_module
                    .as_ref()
                    .map(|m| m.specifier.clone());
                // eprintln!("DEBUG: [handler] Processing import: {} (module: {:?})", import_name, module_path);

                let source_span = dir.source_file.as_ref().and_then(|path| {
                    import_ref.span.map(|span| {
                        let file = std::sync::Arc::new(ParseSourceFile::new(
                            "".to_string(),
                            path.to_string_lossy().to_string(),
                        ));
                        ParseSourceSpan {
                            start: ParseLocation::new(
                                std::sync::Arc::clone(&file),
                                span.start as usize,
                                0,
                                0,
                            ),
                            end: ParseLocation::new(file, span.end as usize, 0, 0),
                            details: None,
                        }
                    })
                });

                let local_import_expr = Expression::ReadVar(ReadVarExpr {
                    name: import_name.clone(),
                    type_: None,
                    source_span: None,
                });

                // Strategy:
                // 1. If module_path exists (external module), try dynamic loading first
                // 2. If dynamic fails, fall back to hardcoded for compatibility
                // 3. If no module_path (local), use ReadVar expression directly

                if let Some(path) = &module_path {
                    // External module - try dynamic loading first
                    if let Some(dynamic_deps) = metadata_reader.read_metadata(path) {
                        let mut matched_specific_symbol = false;
                        let mut matched_symbol_is_module = false;

                        for meta in &dynamic_deps {
                            // Check if this metadata corresponds to the imported symbol
                            let meta_name = match &meta {
                                R3TemplateDependencyMetadata::Directive(d) => Some(&d.type_),
                                R3TemplateDependencyMetadata::Pipe(p) => Some(&p.type_),
                                R3TemplateDependencyMetadata::NgModule(m) => Some(&m.type_),
                            }
                            .and_then(|expr| match expr {
                                Expression::External(ext) => ext.value.name.as_ref(),
                                Expression::ReadVar(rv) => Some(&rv.name),
                                _ => None,
                            });

                            if let Some(name) = meta_name {
                                if name == &import_name
                                    || (name == "NgForOf" && import_name == "NgFor")
                                    || (name == "NgIf" && import_name == "NgIf")
                                {
                                    matched_specific_symbol = true;

                                    let key = match &meta {
                                        R3TemplateDependencyMetadata::Directive(d) => {
                                            format!("dir:{}", d.selector)
                                        }
                                        R3TemplateDependencyMetadata::Pipe(p) => {
                                            format!("pipe:{}", p.name)
                                        }
                                        R3TemplateDependencyMetadata::NgModule(_) => {
                                            format!("module:{}", import_name)
                                        }
                                    };

                                    let mut meta_clone = meta.clone();

                                    // Use the local import expression for the type to avoid synthetic aliases (i1, i2...)
                                    // when the symbol is explicitly imported by the user.
                                    let local_type = local_import_expr.clone();
                                    match &mut meta_clone {
                                        R3TemplateDependencyMetadata::Directive(d) => {
                                            d.source_span = source_span.clone();
                                            d.type_ = local_type;
                                        }
                                        R3TemplateDependencyMetadata::Pipe(p) => {
                                            p.source_span = source_span.clone();
                                            p.type_ = local_type;
                                        }
                                        R3TemplateDependencyMetadata::NgModule(m) => {
                                            m.type_ = local_type;
                                            matched_symbol_is_module = true;
                                        }
                                    }
                                    declarations_map.insert(key, meta_clone);
                                }
                            }
                        }

                        // If we didn't match a specific directive/pipe, assume it's an NgModule (like CommonModule)
                        // In this case, we add the module itself AND all its exports (which are in dynamic_deps)
                        // This allows the template to see all exported directives/pipes
                        if !matched_specific_symbol || matched_symbol_is_module {
                            // 1. Add the module itself
                            let module_meta = R3TemplateDependencyMetadata::NgModule(
                                angular_compiler::render3::view::api::R3NgModuleDependencyMetadata {
                                    kind: angular_compiler::render3::view::api::R3TemplateDependencyKind::NgModule,
                                    type_: local_import_expr.clone(),
                                }
                            );
                            declarations_map.insert(format!("module:{}", import_name), module_meta);

                            // 2. Add all exports from the module
                            for mut meta in dynamic_deps {
                                let key = match &meta {
                                    R3TemplateDependencyMetadata::Directive(d) => {
                                        format!("dir:{}", d.selector)
                                    }
                                    R3TemplateDependencyMetadata::Pipe(p) => {
                                        format!("pipe:{}", p.name)
                                    }
                                    R3TemplateDependencyMetadata::NgModule(_) => {
                                        format!("module:unknown")
                                    } // Should not happen for exports usually
                                };

                                match &mut meta {
                                    R3TemplateDependencyMetadata::Directive(d) => {
                                        d.source_span = source_span.clone()
                                    }
                                    R3TemplateDependencyMetadata::Pipe(p) => {
                                        p.source_span = source_span.clone()
                                    }
                                    R3TemplateDependencyMetadata::NgModule(_) => {}
                                }
                                declarations_map.insert(key, meta);
                            }
                        }
                    } else {
                        // Dynamic loading failed - just add module itself as dependency
                        // The directives/pipes should already be compiled and linked
                        // eprintln!("DEBUG: [handler] Dynamic loading returned no results for {}, adding module itself", import_name);

                        // Dynamic loading failed - just add module itself as dependency
                        let module_meta = R3TemplateDependencyMetadata::NgModule(
                            angular_compiler::render3::view::api::R3NgModuleDependencyMetadata {
                                kind: angular_compiler::render3::view::api::R3TemplateDependencyKind::NgModule,
                                type_: local_import_expr.clone(),
                            }
                        );
                        declarations_map.insert(format!("module:{}", import_name), module_meta);
                    }
                } else {
                    // Local component/directive - no external module
                    // eprintln!("DEBUG: [handler] Local import (no module_path): {}", import_name);

                    let mut found = false;
                    // Dynamic resolution for local components
                    if let Some(source_file) = &dir.source_file {
                        if let Some(local_details) =
                            metadata_reader.extract_ts_metadata(source_file)
                        {
                            for meta in local_details {
                                let matches = match &meta {
                                     R3TemplateDependencyMetadata::Directive(d) => {
                                         if let angular_compiler::output::output_ast::Expression::ReadVar(rv) = &d.type_ {
                                             rv.name == import_name
                                         } else {
                                             false
                                         }
                                     },
                                     R3TemplateDependencyMetadata::Pipe(p) => {
                                         if let angular_compiler::output::output_ast::Expression::ReadVar(rv) = &p.type_ {
                                              rv.name == import_name
                                         } else {
                                              false
                                         }
                                     },
                                     R3TemplateDependencyMetadata::NgModule(n) => {
                                          if let angular_compiler::output::output_ast::Expression::ReadVar(rv) = &n.type_ {
                                              rv.name == import_name
                                          } else {
                                              false
                                          }
                                     }
                                };

                                if matches {
                                    // eprintln!("DEBUG: [handler] Resolved local import dynamically: {}", import_name);
                                    let key = match &meta {
                                        R3TemplateDependencyMetadata::Directive(d) => {
                                            format!("dir:{}", d.selector)
                                        }
                                        R3TemplateDependencyMetadata::Pipe(p) => {
                                            format!("pipe:{}", p.name)
                                        }
                                        R3TemplateDependencyMetadata::NgModule(_) => {
                                            format!("module:{}", import_name)
                                        }
                                    };
                                    declarations_map.insert(key, meta.clone());
                                    found = true;
                                    break;
                                }
                            }
                        }
                    }

                    if !found {
                        // Unknown local - create placeholder with ReadVar
                        let meta = R3TemplateDependencyMetadata::Directive(angular_compiler::render3::view::api::R3DirectiveDependencyMetadata {
                            selector: "".to_string(),
                            type_: local_import_expr,
                            inputs: vec![],
                            outputs: vec![],
                            export_as: vec![].into(),
                            kind: angular_compiler::render3::view::api::R3TemplateDependencyKind::Directive,
                            is_component: false,
                            source_span: source_span.clone(),
                        });
                        declarations_map.insert(format!("unknown:{}", import_name), meta);
                    }
                }
            }
        }

        // eprintln!("DEBUG: [handler] Final declarations_map size: {}", declarations_map.len());
        // eprintln!("DEBUG: [handler] Final declarations_map keys: {:?}", declarations_map.keys().collect::<Vec<_>>());

        let mut r3_metadata = R3ComponentMetadata {
            directive: R3DirectiveMetadata {
                name: dir.t2.name.clone(),
                type_: type_ref,
                type_argument_count: 0,
                type_source_span: angular_compiler::parse_util::ParseSourceSpan::new(
                    angular_compiler::parse_util::ParseLocation::new(
                        angular_compiler::parse_util::ParseSourceFile::new(
                            "".to_string(),
                            "".to_string(),
                        )
                        .into(),
                        0,
                        0,
                        0,
                    ),
                    angular_compiler::parse_util::ParseLocation::new(
                        angular_compiler::parse_util::ParseSourceFile::new(
                            "".to_string(),
                            "".to_string(),
                        )
                        .into(),
                        0,
                        0,
                        0,
                    ),
                ),
                selector: dir.t2.selector.clone(),
                queries: dir
                    .queries
                    .iter()
                    .map(|q| angular_compiler::render3::view::api::R3QueryMetadata {
                        property_name: q.property_name.clone(),
                        first: q.first,
                        predicate:
                            angular_compiler::render3::view::api::R3QueryPredicate::Selectors(vec![
                                q.selector.clone(),
                            ]),
                        descendants: q.descendants,
                        emit_distinct_changes_only: true,
                        read: None,
                        static_: q.is_static,
                        is_signal: q.is_signal,
                    })
                    .collect(),
                view_queries: dir
                    .view_queries
                    .iter()
                    .map(|vq| angular_compiler::render3::view::api::R3QueryMetadata {
                        property_name: vq.property_name.clone(),
                        first: vq.first,
                        predicate:
                            angular_compiler::render3::view::api::R3QueryPredicate::Selectors(vec![
                                vq.selector.clone(),
                            ]),
                        descendants: vq.descendants,
                        emit_distinct_changes_only: true,
                        read: None,
                        static_: vq.is_static,
                        is_signal: vq.is_signal,
                    })
                    .collect(),
                host: dir.host.clone(),
                inputs: dir
                    .t2
                    .inputs
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            angular_compiler::render3::view::api::R3InputMetadata {
                                class_property_name: v.class_property_name.clone(),
                                binding_property_name: v.binding_property_name.clone(),
                                is_signal: v.is_signal,
                                required: v.required,
                                transform_function: v.transform.as_ref().map(|t| {
                                    angular_compiler::output::output_ast::Expression::ReadVar(
                                        angular_compiler::output::output_ast::ReadVarExpr {
                                            name: t.node.clone(),
                                            type_: None,
                                            source_span: None,
                                        },
                                    )
                                }),
                            },
                        )
                    })
                    .collect(),
                outputs: dir
                    .t2
                    .outputs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.binding_property_name.clone()))
                    .collect(),
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
            declarations: declarations_map.into_iter().map(|(_, v)| v).collect(),
            declaration_list_emit_mode: DeclarationListEmitMode::Direct,
            styles: {
                let mut combined = comp_meta.styles.clone().unwrap_or_default();
                combined.extend(styles);
                combined
            },
            encapsulation: ViewEncapsulation::Emulated,
            change_detection: comp_meta.change_detection.map(|s| {
                angular_compiler::render3::view::api::ChangeDetectionOrExpression::Strategy(s)
            }),
            animations: None,
            view_providers: None,
            relative_context_file_path: "".to_string(),
            i18n_use_external_ids: false,
            raw_imports: None,
            external_styles: None,
            defer: R3ComponentDeferMetadata::PerComponent {
                dependencies_fn: None,
            },
            relative_template_path: None,
            has_directive_dependencies: false,
        };

        let mut real_constant_pool = angular_compiler::constant_pool::ConstantPool::new(false);

        // 4. Emit component definition using centralized compiler
        let compiled = angular_compiler::render3::view::compiler::compile_component_from_metadata(
            &r3_metadata,
            &mut real_constant_pool,
            &mut binding_parser,
        );

        // Detect required imports based on metadata
        let mut import_manager = crate::ngtsc::translator::src::import_manager::import_manager::EmitterImportManager::new();

        // Ensure @angular/core is mapped
        let _ = import_manager.get_or_generate_alias("@angular/core");

        for decl in &r3_metadata.declarations {
            let type_expr = match decl {
                R3TemplateDependencyMetadata::Directive(d) => &d.type_,
                R3TemplateDependencyMetadata::Pipe(p) => &p.type_,
                R3TemplateDependencyMetadata::NgModule(m) => &m.type_,
            };

            if let Expression::External(ext) = type_expr {
                let val = &ext.value;
                if let Some(module_name) = &val.module_name {
                    import_manager.get_or_generate_alias(module_name);
                }
            }
        }

        let imports_map = import_manager.get_imports_map();
        let additional_imports: Vec<(String, String)> = imports_map
            .iter()
            .map(|(k, v)| (v.clone(), k.clone()))
            .collect();

        // Emit AST to String
        let mut emitter = AbstractJsEmitterVisitor::with_imports(imports_map);
        let mut ctx = EmitterVisitorContext::create_root();
        let context: &mut dyn Any = &mut ctx;

        compiled.expression.visit_expression(&mut emitter, context);

        let initializer = ctx.to_source();

        // Emit statements (hoisted statements)
        let mut emitted_statements = vec![];
        for stmt in &compiled.statements {
            let mut stmt_ctx = EmitterVisitorContext::create_root();
            let stmt_context: &mut dyn Any = &mut stmt_ctx;
            stmt.visit_statement(&mut emitter, stmt_context);
            emitted_statements.push(stmt_ctx.to_source());
        }

        // 4. Convert diagnostics (not easily available from compiled result yet, need to improve return type of compile_component_from_metadata if we want them back)
        // For now, returning empty diagnostics as the centralized compiler doesn't return them directly in the struct yet
        let ts_diagnostics: Vec<ts::Diagnostic> = vec![];

        vec![CompileResult {
            name: "ɵcmp".to_string(),
            initializer: Some(initializer),
            statements: emitted_statements,
            type_desc: "ComponentDef".to_string(),
            deferrable_imports: None,
            diagnostics: ts_diagnostics,
            additional_imports,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ngtsc::metadata::{
        ClassPropertyMapping, ComponentMetadata, DirectiveMeta, T2DirectiveMetadata,
    };
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
        // Check for key Ivy definition parts
        assert!(initializer.contains("defineComponent"));
        assert!(initializer.contains("selectors: [['test-comp']]"));
        assert!(initializer.contains("decls: 2"));
        assert!(initializer.contains("vars: 0"));
    }
}
