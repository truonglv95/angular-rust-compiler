use crate::ngtsc::annotations::component::src::handler::ComponentDecoratorHandler;
use crate::ngtsc::annotations::directive::src::handler::DirectiveDecoratorHandler;
use crate::ngtsc::core::NgCompilerOptions;
use crate::ngtsc::file_system::{AbsoluteFsPath, FileSystem};
use crate::ngtsc::metadata::{
    DecoratorMetadata, DirectiveMetadata, MetadataReader, OxcMetadataReader,
};
use crate::ngtsc::translator::src::import_manager::import_manager::EmitterImportManager;
use angular_compiler::ml_parser::tags::TagDefinition;
use angular_compiler::ml_parser::{
    html_tags::get_html_tag_definition, parser::Parser as HtmlParser,
};
use angular_compiler::output::abstract_emitter::{AbstractEmitterVisitor, EmitterVisitorContext};
use angular_compiler::output::output_ast::ExpressionTrait;
use angular_compiler::render3::r3_injector_compiler::{compile_injector, R3InjectorMetadata};
use angular_compiler::render3::r3_module_compiler::{
    compile_ng_module, R3NgModuleMetadata, R3NgModuleMetadataCommon, R3NgModuleMetadataKind,
    R3NgModuleMetadataLocal, R3SelectorScopeMode,
};
use angular_compiler::render3::util::R3Reference;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::collections::HashSet;
use std::path::PathBuf;

fn expression_to_string(expr: &angular_compiler::output::output_ast::Expression) -> String {
    let mut ctx = EmitterVisitorContext::create_root();
    let mut visitor = AbstractEmitterVisitor::new(false);
    expr.visit_expression(&mut visitor, &mut ctx);
    ctx.to_source()
}

fn get_html_tag_definition_wrapper(name: &str) -> &'static dyn TagDefinition {
    get_html_tag_definition(name)
}

pub enum CompilationTicketKind {
    Fresh,
    Incremental,
}

pub struct CompilationTicket<'a, T: FileSystem> {
    pub kind: CompilationTicketKind,
    pub options: NgCompilerOptions,
    pub fs: &'a T,
}

pub struct NgCompiler<'a, T: FileSystem> {
    pub options: NgCompilerOptions,
    pub fs: &'a T,
    pub is_core: bool,
}

#[derive(Default)]
pub struct CompilationResult {
    pub files: Vec<PathBuf>,
    pub directives: Vec<DirectiveMetadata<'static>>,
    pub diagnostics: Vec<crate::ngtsc::core::Diagnostic>,
}

impl<'a, T: FileSystem> NgCompiler<'a, T> {
    pub fn new(ticket: CompilationTicket<'a, T>) -> Self {
        NgCompiler {
            options: ticket.options,
            fs: ticket.fs,
            is_core: false,
        }
    }

    pub fn analyze_async(&mut self, root_names: &[String]) -> Result<CompilationResult, String> {
        // eprintln!("DEBUG: NgCompiler::analyze_async called with {} root files", root_names.len());
        let mut result = CompilationResult::default();
        let metadata_reader = OxcMetadataReader;

        for file in root_names {
            let path = PathBuf::from(file);
            let abs_path = AbsoluteFsPath::from(&path);
            // eprintln!("DEBUG: analyze_async processing file: {:?}", abs_path);

            let content = match self.fs.read_file(&abs_path) {
                Ok(c) => c,
                Err(e) => {
                    // eprintln!("DEBUG: analyze_async failed to read file: {:?}", abs_path);
                    return Err(format!(
                        "File not found or unreadable: {:?} ({})",
                        abs_path, e
                    ));
                }
            };

            let allocator = Allocator::default();
            let source_type = SourceType::from_path(&path).unwrap_or_default();

            let ret = Parser::new(&allocator, &content, source_type).parse();

            if !ret.errors.is_empty() {
                for error in ret.errors {
                    // eprintln!("DEBUG: Error parsing {:?}: {:?}", path, error);
                }
                return Err(format!("Failed to parse {:?}", path));
            } else {
                let mut directives = metadata_reader.get_directive_metadata(&ret.program, &path);
                // eprintln!("DEBUG: Extracted {} decorators from {:?}", directives.len(), abs_path);

                // Parse templates for components that have inline templates
                for directive in &mut directives {
                    // Only components have templates and styles
                    if let DecoratorMetadata::Directive(ref mut dir) = directive {
                        // eprintln!("DEBUG: Found directive/component: {}", dir.t2.name);
                        if !dir.t2.is_component {
                            continue;
                        }

                        let template_str = if let Some(comp) = &dir.component {
                            if let Some(template) = &comp.template {
                                Some(template.clone())
                            } else if let Some(template_url) = &comp.template_url {
                                let component_dir = self.fs.dirname(abs_path.as_str());
                                let template_path =
                                    self.fs.resolve(&[&component_dir, template_url]);
                                // eprintln!("DEBUG: Resolving template URL '{}' relative to '{}' -> '{}'", template_url, component_dir, template_path);
                                match self.fs.read_file(&template_path) {
                                    Ok(content) => {
                                        // eprintln!("DEBUG: Successfully read template file: {}", template_path);
                                        Some(content)
                                    }
                                    Err(e) => {
                                        // eprintln!("DEBUG: Failed to read template file: {} (Error: {})", template_path, e);
                                        None
                                    }
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(template) = template_str {
                            let parser = HtmlParser::new(get_html_tag_definition_wrapper);
                            let parse_result = parser.parse(&template, "template.html", None);

                            if !parse_result.errors.is_empty() {
                            } else {
                                if let Some(comp) = &mut dir.component {
                                    comp.template_ast = Some(parse_result.root_nodes);
                                }
                            }
                        }

                        let style_urls = dir.component.as_ref().and_then(|c| c.style_urls.clone());
                        if let Some(style_urls) = style_urls {
                            let component_dir = self.fs.dirname(abs_path.as_str());
                            let mut resolved_styles = if let Some(comp) = &mut dir.component {
                                comp.styles.take().unwrap_or_default()
                            } else {
                                Vec::new()
                            };

                            for url in style_urls {
                                let style_path = self.fs.resolve(&[&component_dir, &url]);
                                match self.fs.read_file(&style_path) {
                                    Ok(content) => resolved_styles.push(content),
                                    Err(_) => {}
                                }
                            }
                            if let Some(comp) = &mut dir.component {
                                comp.styles = Some(resolved_styles);
                            }
                        }
                    }
                }

                result.directives.extend(directives);
                result.files.push(path);
            }
        }
        Ok(result)
    }

    pub fn emit(
        &self,
        compilation_result: &CompilationResult,
    ) -> Result<Vec<crate::ngtsc::core::Diagnostic>, String> {
        use oxc_ast::ast::*;
        let mut result_diagnostics: Vec<crate::ngtsc::core::Diagnostic> = Vec::new();
        let fs = self.fs;

        let component_handler =
            crate::ngtsc::annotations::component::src::handler::ComponentDecoratorHandler::new();
        let directive_handler =
            crate::ngtsc::annotations::directive::src::handler::DirectiveDecoratorHandler::new(
                false,
            );

        use rayon::prelude::*;
        use std::collections::{HashMap, HashSet};

        // Track which files have components (they get special handling)
        let mut component_files: HashSet<PathBuf> = HashSet::new();
        let mut result_diagnostics = Vec::new();

        // Group directives by source file to efficient processing
        let mut file_to_directives: HashMap<PathBuf, Vec<&DecoratorMetadata>> = HashMap::new();
        let mut directives_without_source: Vec<&DecoratorMetadata> = Vec::new();

        for directive in &compilation_result.directives {
            if let Some(src) = directive.source_file() {
                file_to_directives
                    .entry(src.clone())
                    .or_default()
                    .push(directive);
            } else {
                directives_without_source.push(directive);
            }
        }

        struct FileResult {
            path: PathBuf,
            diagnostics: Vec<crate::ngtsc::core::Diagnostic>,
        }

        struct UnsafeSyncWrapper<T>(T);
        unsafe impl<T> Sync for UnsafeSyncWrapper<T> {}
        unsafe impl<T> Send for UnsafeSyncWrapper<T> {}

        // Parallel processing of files with components
        let file_results: Vec<FileResult> = file_to_directives
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().map(UnsafeSyncWrapper).collect::<Vec<_>>()))
            .collect::<Vec<_>>() // Collect into Vec to allow par_iter
            .into_par_iter()
            .map(|(src_file, directives_wrapper)| {
                let directives: Vec<&DecoratorMetadata> = directives_wrapper.into_iter().map(|w| w.0).collect();

                // Setup output path
                let mut out_path = if let Some(out_dir) = &self.options.out_dir {
                    let absolute_project_root = if let Some(root_dir) = &self.options.root_dir {
                        let p = PathBuf::from(root_dir);
                        std::fs::canonicalize(&p).unwrap_or(p)
                    } else {
                        let project_path = std::path::Path::new(&self.options.project);
                        let project_root = project_path.parent().unwrap_or(std::path::Path::new("."));
                        std::fs::canonicalize(project_root).unwrap_or(project_root.to_path_buf())
                    };

                    let absolute_src_file = std::fs::canonicalize(&src_file)
                        .unwrap_or(src_file.clone());

                    let relative_path = absolute_src_file
                        .strip_prefix(&absolute_project_root)
                        .unwrap_or_else(|_| {
                            std::path::Path::new(src_file.file_name().unwrap())
                        });

                    let mut p = PathBuf::from(out_dir);
                    p.push(relative_path);

                    let out_file_path = p.with_extension("js");
                    out_file_path
                } else {
                    let mut p = PathBuf::from(&src_file);
                    let out_file_path = p.with_extension("js");
                    out_file_path
                };

                // Ensure parent dir exists
                if let Some(parent) = out_path.parent() {
                    let _ = fs.ensure_dir(&AbsoluteFsPath::from(parent));
                }

                let source_path = AbsoluteFsPath::from(src_file.as_path());
                let mut diagnostics = Vec::new();

                // Read parse and transform
                let output_content = match fs.read_file(&source_path) {
                    Ok(source_content) => {
                        let allocator = Allocator::default();
                        let source_type = SourceType::ts();
                        let file_path = src_file.to_string_lossy().to_string();

                        let parser = Parser::new(&allocator, &source_content, source_type);
                        let mut parse_result = parser.parse();

                        if parse_result.errors.is_empty() {
                            // Step 1: Run semantic analysis for scoping
                            let semantic = oxc_semantic::SemanticBuilder::new()
                                .with_excess_capacity(0.0)
                                .build(&parse_result.program);

                            // Step 2: Apply TypeScript transformer to strip types
                            let transform_options = oxc_transformer::TransformOptions::default();
                            let transformer = oxc_transformer::Transformer::new(
                                &allocator,
                                std::path::Path::new(&file_path),
                                &transform_options,
                            );
                            let _ = transformer.build_with_scoping(
                                semantic.semantic.into_scoping(),
                                &mut parse_result.program,
                            );

                            // Step 3: AST-based Ivy transformation for ALL directives in this file
                            // Track failed properties that couldn't be parsed (accumulated across all directives)
                            let mut failed_properties: Vec<super::ast_transformer::FailedProperty> = Vec::new();
                            let mut last_def_name = "ɵcmp".to_string(); // Default, will be updated for each directive

                            // Create a shared ImportManager for this file to coordinate aliases (i0, i1, etc.)
                            let mut import_manager = EmitterImportManager::new();
                            // Ensure @angular/core is always i0
                            let _ = import_manager.get_or_generate_alias("@angular/core");

                            for directive in directives {
                                let (compiled_results, directive_name) = match directive {
                                    DecoratorMetadata::Directive(dir) => {
                                        let results = if dir.t2.is_component {
                                            component_handler.compile_ivy(&directive, Some(&mut import_manager))
                                        } else {
                                            directive_handler.compile_ivy(&directive, Some(&mut import_manager))
                                        };
                                        (results, dir.t2.name.clone())
                                    }
                                    DecoratorMetadata::Pipe(pipe) => {
                                        let initializer = format!(
                                            "/*@__PURE__*/ i0.ɵɵdefinePipe({{ name: '{}', type: {}, pure: {}{} }})",
                                            pipe.pipe_name, pipe.name, pipe.is_pure,
                                            if pipe.is_standalone { ", standalone: true" } else { "" }
                                        );
                                        let results = vec![crate::ngtsc::transform::src::api::CompileResult {
                                            name: "ɵpipe".to_string(),
                                            initializer: Some(initializer),
                                            statements: vec![],
                                            type_desc: format!("i0.ɵɵPipeDeclaration<{}, '{}', {}>", pipe.name, pipe.pipe_name, pipe.is_standalone),
                                            deferrable_imports: None,
                                            diagnostics: Vec::new(),
                                            additional_imports: Vec::new(),
                                        }];
                                        (results, pipe.name.clone())
                                    }
                                    DecoratorMetadata::Injectable(inj) => {
                                        let provided_in = inj.provided_in.as_deref().unwrap_or("null");
                                        let provided_in_value = if provided_in == "null" { "null".to_string() } else { format!("'{}'", provided_in) };
                                        let fac_initializer = format!("function {}_Factory(__ngFactoryType__) {{ return new (__ngFactoryType__ || {})(); }}", inj.name, inj.name);
                                        let prov_initializer = format!("/*@__PURE__*/ i0.ɵɵdefineInjectable({{ token: {}, factory: {}.ɵfac, providedIn: {} }})", inj.name, inj.name, provided_in_value);
                                        let results = vec![
                                            crate::ngtsc::transform::src::api::CompileResult {
                                                name: "ɵfac".to_string(),
                                                initializer: Some(fac_initializer),
                                                statements: vec![],
                                                type_desc: format!("i0.ɵɵFactoryDeclaration<{}, never>", inj.name),
                                                deferrable_imports: None,
                                                diagnostics: Vec::new(),
                                                additional_imports: Vec::new(),
                                            },
                                            crate::ngtsc::transform::src::api::CompileResult {
                                                name: "ɵprov".to_string(),
                                                initializer: Some(prov_initializer),
                                                statements: vec![],
                                                type_desc: format!("i0.ɵɵInjectableDeclaration<{}>", inj.name),
                                                deferrable_imports: None,
                                                diagnostics: Vec::new(),
                                                additional_imports: Vec::new(),
                                            }
                                        ];
                                        (results, inj.name.clone())
                                    }
                                    DecoratorMetadata::NgModule(ngm) => {
                                        use angular_compiler::output::output_ast::{Expression, LiteralArrayExpr};

                                        // Helper to create expression from string
                                        let str_to_expr = |s: &str| Expression::External(angular_compiler::output::output_ast::ExternalExpr {
                                            value: angular_compiler::output::output_ast::ExternalReference {
                                                module_name: None,
                                                name: Some(s.to_string()),
                                                runtime: None,
                                            },
                                            type_: None,
                                            source_span: None,
                                        });

                                        let type_ref = R3Reference {
                                            value: str_to_expr(&ngm.name),
                                            type_expr: str_to_expr(&ngm.name),
                                        };

                                        // Create NgModule Local Compilation Metadata
                                        let common = R3NgModuleMetadataCommon {
                                            kind: R3NgModuleMetadataKind::Local,
                                            type_: type_ref.clone(),
                                            selector_scope_mode: R3SelectorScopeMode::Inline, // Or SideEffect if desired? Inline is simpler for local.
                                            schemas: if ngm.schemas.is_empty() {
                                                None
                                            } else {
                                                Some(ngm.schemas.iter().map(|s| R3Reference {
                                                    value: str_to_expr(s),
                                                    type_expr: str_to_expr(s),
                                                }).collect())
                                            },
                                            id: None,
                                        };

                                        // We use the raw_ declarations/imports/exports if available, otherwise fallback to empty lists for safety.
                                        // But actually, for local compilation, we need the EXPRESSIONS (arrays).
                                        // Since we don't have the original expressions easily parsed yet here,
                                        // we'll rely on what we have. If we have raw_* strings, we can use them directly?
                                        // Wait, R3NgModuleMetadataLocal expects Option<Expression>.
                                        // If we have just names, we can construct an array literal.

                                        // Helper: Vec<String> -> Expression::LiteralArray
                                        // Helper: Vec<String> -> Expression::LiteralArray
                                        let to_array_expr = |items: &[String]| -> Option<Expression> {
                                            if items.is_empty() {
                                                None
                                            } else {
                                                Some(Expression::LiteralArray(LiteralArrayExpr {
                                                    entries: items.iter().map(|i| str_to_expr(i)).collect(),
                                                    type_: None,
                                                    source_span: None,
                                                }))
                                            }
                                        };

                                        let ng_module_meta = R3NgModuleMetadataLocal {
                                            common,
                                            bootstrap_expression: None, // We don't support bootstrap component yet
                                            declarations_expression: ngm.declarations_expression.clone().or_else(|| to_array_expr(&ngm.declarations)),
                                            imports_expression: ngm.imports_expression.clone().or_else(|| to_array_expr(&ngm.imports)),
                                            exports_expression: ngm.exports_expression.clone().or_else(|| to_array_expr(&ngm.exports)),
                                        };

                                        let res_mod = compile_ng_module(&R3NgModuleMetadata::Local(ng_module_meta));

                                        // Injector Compilation
                                        // Extract imports Vec<Expression> from imports_expression if possible
                                        let injector_imports: Vec<Expression> = if let Some(Expression::LiteralArray(arr)) = &ngm.imports_expression {
                                            arr.entries.clone()
                                        } else {
                                            ngm.imports.iter().map(|i| str_to_expr(i)).collect()
                                        };

                                        let injector_meta = R3InjectorMetadata {
                                            name: ngm.name.clone(),
                                            type_: type_ref,
                                            providers: ngm.providers_expression.clone().or_else(||
                                                // Fallback if needed, though usually None is fine if empty
                                                if ngm.may_declare_providers {
                                                     Some(Expression::LiteralArray(LiteralArrayExpr {
                                                        entries: vec![],
                                                        type_: None,
                                                        source_span: None,
                                                    }))
                                                } else {
                                                    None
                                                }
                                            ),
                                            imports: injector_imports,
                                        };

                                        let res_inj = compile_injector(&injector_meta);

                                        // Combine results
                                        let mut results = vec![];

                                        // Result for ɵmod
                                        let mod_init = expression_to_string(&res_mod.expression)
                                            .replace("@angular/core.", "i0.");
                                        results.push(crate::ngtsc::transform::src::api::CompileResult {
                                            name: "ɵmod".to_string(),
                                            initializer: Some(mod_init),
                                            statements: vec![],
                                            type_desc: format!("i0.ɵɵNgModuleDeclaration<{}, never, never, never>", ngm.name),
                                            deferrable_imports: None,
                                            diagnostics: Vec::new(),
                                            additional_imports: Vec::new(),
                                        });

                                        // Result for ɵinj
                                        let inj_init = expression_to_string(&res_inj.expression)
                                            .replace("@angular/core.", "i0.");
                                        results.push(crate::ngtsc::transform::src::api::CompileResult {
                                            name: "ɵinj".to_string(),
                                            initializer: Some(inj_init),
                                            statements: vec![],
                                            type_desc: format!("i0.ɵɵInjectorDeclaration<{}>", ngm.name),
                                            deferrable_imports: None,
                                            diagnostics: Vec::new(),
                                            additional_imports: Vec::new(),
                                        });

                                        eprintln!("[RUST_DEBUG] Matched NgModule: {}, Results: {}", ngm.name, results.len());

                                        (results, ngm.name.clone())
                                    }
                                };

                                // Collect diagnostics
                                for r in &compiled_results {
                                    diagnostics.extend(r.diagnostics.iter().map(|d| crate::ngtsc::core::Diagnostic {
                                        file: d.file.clone().map(PathBuf::from),
                                        message: d.message_text.to_string(),
                                        code: d.code as usize,
                                        start: Some(d.start),
                                        length: Some(d.length),
                                    }));
                                }

                                if compiled_results.is_empty() {
                                    continue;
                                }

                                // Apply to AST
                                // Primary result
                                let mut hoisted_statements = String::new();

                                // Merge results if multiple (e.g. fac and cmp)
                                for res in &compiled_results {
                                    for stmt in &res.statements {
                                        hoisted_statements.push_str(stmt);
                                        hoisted_statements.push('\n');
                                    }
                                }

                                // Prepare expressions for transform_component_ast
                                let fac_expr_str_default = format!("function {}_Factory(t) {{ return new (t || {})(); }}", directive_name, directive_name);

                                // Collect definitions (non-fac)
                                let mut definitions_vec: Vec<(String, String)> = Vec::new();
                                let mut additional_imports: Vec<(String, String)> = Vec::new();
                                let mut last_def_name = "ɵcmp".to_string();

                                for res in &compiled_results {
                                    additional_imports.extend(res.additional_imports.clone());
                                    if res.name == "ɵfac" {
                                        continue;
                                    }
                                    let init = res.initializer.as_deref().unwrap_or("null");
                                    // Wrap in PURE annotation
                                    let expr = format!("/*@__PURE__*/ {}", init);
                                    definitions_vec.push((res.name.clone(), expr));
                                    last_def_name = res.name.clone();
                                }

                                // Make additional_imports unique
                                additional_imports.sort();
                                additional_imports.dedup();

                                let fac_initializer = compiled_results.iter().find(|r| r.name == "ɵfac").and_then(|r| r.initializer.as_deref()).unwrap_or(&fac_expr_str_default);

                                let hoisted_statements_arena: &str = allocator.alloc_str(&hoisted_statements);
                                let fac_expr_arena: &str = allocator.alloc_str(fac_initializer); // Use correct fac logic

                                // Allocate definitions strings in arena
                                let definitions_arena: Vec<(&str, &str)> = definitions_vec.iter()
                                    .map(|(name, expr)| (
                                        allocator.alloc_str(name) as &str,
                                        allocator.alloc_str(expr) as &str
                                    ))
                                    .collect();

                                // Only transform if we have something valid (definitions or fac)
                                if !definitions_arena.is_empty() || fac_initializer != "null" {
                                    let failed = super::ast_transformer::transform_component_ast(
                                        &allocator,
                                        &mut parse_result.program,
                                        &directive_name,
                                        hoisted_statements_arena,
                                        fac_expr_arena, // fac
                                        &definitions_arena,
                                        &additional_imports,
                                    );

                                    failed_properties.extend(failed);
                                }
                            }

                            // Step 4: Codegen final JavaScript
                            let codegen = oxc_codegen::Codegen::new().with_options(oxc_codegen::CodegenOptions {
                                single_quote: true,
                                ..oxc_codegen::CodegenOptions::default()
                            });
                            let mut code = codegen.build(&parse_result.program).code;

                            // Replace placeholders with real expressions
                            for prop in &failed_properties {
                                // Codegen for string literals with single_quote: true will be 'placeholder'
                                let placeholder_pattern = format!("'{}'", prop.placeholder);
                                if code.contains(&placeholder_pattern) {
                                    code = code.replace(&placeholder_pattern, &prop.expr);
                                }
                            }

                            // Fix property names (ɵUNIQUE_FAC -> ɵfac, ɵUNIQUE_DIR -> last_def_name)
                            code = code.replace("ɵUNIQUE_FAC", "ɵfac");
                            code = code.replace("ɵUNIQUE_DIR", &last_def_name);

                            Some(code)
                        } else {
                            // Parse error
                            None
                        }
                    },
                    Err(_) => None
                };

                if let Some(content) = output_content {
                    let out_path_abs = AbsoluteFsPath::from(out_path.as_path());
                    match fs.write_file(&out_path_abs, content.as_bytes(), None) {
                        Ok(_) => (),
                        Err(_) => (),
                    }
                }

                FileResult {
                    path: src_file,
                    diagnostics
                }
            })
            .collect();

        // Collect results
        for res in file_results {
            component_files.insert(res.path);
        }

        // Handle directives without source (fallback, sequential)
        for directive in directives_without_source {
            self.process_directive_fallback(
                &directive,
                self.fs,
                &mut component_files,
                &mut result_diagnostics,
                &compilation_result.files,
            );
        }
        // Second pass: transpile non-component TypeScript files
        // Use parallel iterator
        compilation_result.files.par_iter().for_each(|file| {
            // Skip files in node_modules, spec files, and declaration files
            let src_path = file.to_string_lossy();

            if src_path.contains("node_modules")
                || src_path.ends_with(".spec.ts")
                || src_path.ends_with(".d.ts")
            {
                return; // Use return for for_each
            }

            // Skip files that have components (already emitted)
            if component_files.contains(file) {
                return;
            }

            if let Some(out_dir) = &self.options.out_dir {
                // Calculate output path preserving directory structure
                let absolute_project_root = if let Some(root_dir) = &self.options.root_dir {
                    let p = PathBuf::from(root_dir);
                    std::fs::canonicalize(&p).unwrap_or(p)
                } else {
                    let project_path = std::path::Path::new(&self.options.project);
                    let project_root = project_path.parent().unwrap_or(std::path::Path::new("."));
                    std::fs::canonicalize(project_root).unwrap_or(project_root.to_path_buf())
                };

                let absolute_src_file =
                    std::fs::canonicalize(file.as_path()).unwrap_or(file.as_path().to_path_buf());

                let relative_path = absolute_src_file
                    .strip_prefix(&absolute_project_root)
                    .unwrap_or_else(|_| std::path::Path::new(file.file_name().unwrap()));

                let mut out_path = PathBuf::from(out_dir);
                out_path.push(relative_path);
                out_path.set_extension("js");

                // Ensure parent dir exists
                if let Some(parent) = out_path.parent() {
                    let _ = fs.ensure_dir(&AbsoluteFsPath::from(parent));
                }

                // Read the source file
                let source_path = AbsoluteFsPath::from(file.as_path());

                match fs.read_file(&source_path) {
                    Ok(source_content) => {
                        // Transpilation: parse TypeScript, transform to strip types, then codegen
                        let allocator = Allocator::default();
                        let source_type = SourceType::ts();
                        let file_path = file.to_string_lossy().to_string();

                        let parser = Parser::new(&allocator, &source_content, source_type);
                        let mut parse_result = parser.parse();

                        if !parse_result.errors.is_empty() {
                            return;
                        }

                        // Run semantic analysis to get scoping information

                        let semantic = oxc_semantic::SemanticBuilder::new()
                            .with_excess_capacity(0.0)
                            .build(&parse_result.program);

                        // Apply TypeScript transformer to strip types

                        let transform_options = oxc_transformer::TransformOptions::default();
                        let transformer = oxc_transformer::Transformer::new(
                            &allocator,
                            std::path::Path::new(&file_path),
                            &transform_options,
                        );
                        let _ = transformer.build_with_scoping(
                            semantic.semantic.into_scoping(),
                            &mut parse_result.program,
                        );

                        // Use OXC codegen to emit JavaScript without types

                        let codegen =
                            oxc_codegen::Codegen::new().with_options(oxc_codegen::CodegenOptions {
                                single_quote: true,
                                ..oxc_codegen::CodegenOptions::default()
                            });
                        let mut js_output = codegen.build(&parse_result.program).code;

                        // Add signature line for main.ts
                        if file_path.ends_with("main.ts") {
                            js_output.push_str("\nconsole.log('%cAngular Rust compiler powered by Truonglv4', 'color: #00ff00; font-weight: bold;');\n");
                        }

                        let out_path_abs = AbsoluteFsPath::from(out_path.as_path());

                        match fs.write_file(&out_path_abs, js_output.as_bytes(), None) {
                            Ok(_) => (),
                            Err(_) => (),
                        }
                    }
                    Err(_) => {}
                }
            }
        });

        Ok(result_diagnostics)
    }

    fn process_directive_fallback(
        &self,
        directive: &DecoratorMetadata<'static>,
        fs: &T,
        component_files: &mut HashSet<PathBuf>,
        result_diagnostics: &mut Vec<crate::ngtsc::core::Diagnostic>,
        compilation_files: &[PathBuf],
    ) {
        let component_handler = ComponentDecoratorHandler::new();
        let directive_handler = DirectiveDecoratorHandler::new(self.is_core);

        let (compiled_results, directive_name, source_file) = match directive {
            DecoratorMetadata::Directive(dir) => {
                let results = if dir.t2.is_component {
                    component_handler.compile_ivy(directive, None)
                } else {
                    directive_handler.compile_ivy(directive, None)
                };
                (results, dir.t2.name.clone(), dir.source_file.clone())
            }
            _ => return,
        };

        // Collect diagnostics
        for r in &compiled_results {
            result_diagnostics.extend(r.diagnostics.iter().map(|d| {
                crate::ngtsc::core::Diagnostic {
                    file: d.file.clone().map(PathBuf::from),
                    message: d.message_text.to_string(),
                    code: d.code as usize,
                    start: Some(d.start),
                    length: Some(d.length),
                }
            }));
        }

        // Wait, I can't easily change the `if let` structure without rewriting more lines.
        // `compiled_results` is used in loop at line 592 (by reference).
        // Then line 604 consumes it `into_iter`.

        // Strategy:
        // 1. Find main result by reference first.
        // 2. If found, proceed.
        // 3. Inside, iterate over `compiled_results` (which was not consumed if I change line 604 to `iter()`).

        let main_result = compiled_results
            .iter()
            .find(|r| r.name == "ɵcmp" || r.name == "ɵdir");

        if let Some(r) = main_result {
            if let Some(initializer) = &r.initializer {
                // Logic here
            }
        }

        // BUT the replacement target is the `if let` block.
        // So I will replace the `if let` block with the new logic.

        // New block:
        let main_result_ref = compiled_results
            .iter()
            .find(|r| r.name == "ɵcmp" || r.name == "ɵdir");
        if let Some(main_res) = main_result_ref {
            if let Some(initializer) = &main_res.initializer {
                let mut import_map: std::collections::HashMap<String, String> =
                    std::collections::HashMap::new();

                // Collect imports from ALL results (fac, dir, etc)
                for res in &compiled_results {
                    for (alias, module) in &res.additional_imports {
                        import_map.insert(alias.clone(), module.clone());
                    }
                }

                // Generate imports string
                let mut import_stmts = String::new();
                let mut sorted_aliases: Vec<_> = import_map.keys().collect();
                sorted_aliases.sort();

                for alias in sorted_aliases {
                    let module = import_map.get(alias).unwrap();
                    import_stmts.push_str(&format!("import * as {} from '{}';\n", alias, module));
                }

                let final_content = if let Some(src_file) = source_file.as_ref() {
                    let source_path =
                        crate::ngtsc::file_system::AbsoluteFsPath::from(src_file.as_path());
                    match fs.read_file(&source_path) {
                        Ok(content) => {
                            let source_text = content.clone();
                            let stripped = strip_angular_decorator(&source_text);
                            // We strictly prepend the imports.
                            format!("{}\n{}\n\n{}.ɵfac = function {}_Factory(t) {{ return new (t || {})(); }};\n{}.ɵcmp = /*@__PURE__*/ {};",
                                   import_stmts, stripped, directive_name, directive_name, directive_name, directive_name, initializer)
                        }
                        Err(_) => format!(
                            "// Error reading file\nexport class {} {{}}",
                            directive_name
                        ),
                    }
                } else {
                    format!("export class {} {{}}", directive_name)
                };

                if let Some(src_file) = source_file {
                    let file_name = src_file.file_name().unwrap_or_default();
                    let file_name_str = file_name.to_string_lossy();
                    let js_name = file_name_str.replace(".ts", ".js");

                    for out_path in compilation_files {
                        if out_path.ends_with(&js_name) {
                            let _ = fs.write_file(
                                &crate::ngtsc::file_system::AbsoluteFsPath::from(out_path),
                                final_content.as_bytes(),
                                None,
                            );
                        }
                    }
                    component_files.insert(src_file);
                }
            }
        }
    }
}
/// Strip Angular decorators (@Component, @Directive, @Injectable, etc.) from transpiled code
fn strip_angular_decorator(code: &str) -> String {
    // Pattern to match: export @Decorator({...}) class ClassName
    // Should produce: export class ClassName
    let mut result = code.to_string();

    // List of Angular decorators to strip
    let decorators = [
        "@Component",
        "@Directive",
        "@Injectable",
        "@Pipe",
        "@NgModule",
    ];

    for decorator in &decorators {
        // Find the decorator and its configuration block
        while let Some(start) = result.find(decorator) {
            // Find the opening parenthesis
            if let Some(paren_start) = result[start..].find('(') {
                let paren_start = start + paren_start;

                // Count balanced parentheses to find the end
                let mut depth = 0;
                let mut end_pos = paren_start;
                for (i, c) in result[paren_start..].char_indices() {
                    match c {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                end_pos = paren_start + i + 1; // include the closing paren
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                // Remove the decorator and its config, keeping any whitespace after
                let before = &result[..start];
                let after = &result[end_pos..].trim_start();
                result = format!("{}{}", before, after);
            } else {
                break;
            }
        }
    }

    result
}

/// Extract import statements from code and return them separately
fn extract_and_remove_imports(code: &str) -> (Vec<String>, String) {
    let mut imports = Vec::new();
    let mut remaining_lines = Vec::new();

    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ") {
            imports.push(line.to_string());
        } else {
            remaining_lines.push(line);
        }
    }

    (imports, remaining_lines.join("\n"))
}
