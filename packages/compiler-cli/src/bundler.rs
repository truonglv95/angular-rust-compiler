use crate::compile::parallel::parallel_compile;
use crate::config::angular::AngularConfig;
use anyhow::Result;
use oxc_allocator::Allocator;
use oxc_ast::ast::{Argument, Expression as OxcExpression, Statement};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

pub struct BundleResult {
    pub bundle_js: String,
    pub bundle_name: String,
    pub styles_css: Option<String>,
    pub scripts_js: Option<String>,
    pub polyfills_js: Option<String>, // Generated from angular.json "polyfills" config
    pub index_html: Option<String>,
    pub files: HashMap<String, String>,
    pub raw_files: HashMap<String, String>, // Raw compiled files without import stripping (for dev mode)
    pub chunks: HashMap<String, String>,
    pub chunk_names: HashMap<String, String>, // Map<HashedName, OriginalName>
    pub module_to_chunk: HashMap<String, String>, // Map<SourcePath, HashedName>
    pub external_imports: Vec<String>, // List of external package imports for Vite optimizeDeps
}

/// Configuration for bundling - extracted from angular.json
/// This avoids hardcoding paths like "/dist/" throughout the codebase
#[derive(Clone)]
pub struct BundleConfig {
    pub source_root: String, // e.g., "src"
    pub output_dir: String,  // e.g., "dist"
}

/// Represents result of scanning a file for imports
struct ImportScanResult {
    static_imports: Vec<PathBuf>,
    dynamic_imports: Vec<PathBuf>,
    resources: Vec<PathBuf>,
}

/// Scans a TypeScript/JavaScript file for static and dynamic imports
fn scan_imports(file_path: &Path, root_dir: &Path) -> Result<ImportScanResult> {
    let content = std::fs::read_to_string(file_path)?;

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(file_path)
        .unwrap_or_default()
        .with_typescript(true);
    let ret = Parser::new(&allocator, &content, source_type).parse();

    let mut static_imports = Vec::new();
    let mut dynamic_imports = Vec::new();
    let mut resources = HashSet::new();

    let file_dir = file_path.parent().unwrap_or(root_dir);

    scan_resources(&ret.program, file_dir, root_dir, &mut resources);

    for stmt in &ret.program.body {
        // Static imports: import ... from '...'
        if let Statement::ImportDeclaration(decl) = stmt {
            let specifier = decl.source.value.as_str();

            if let Some(resolved) = resolve_import(specifier, file_dir, root_dir) {
                // eprintln!("Found static import: {:?} -> {:?}", specifier, resolved);
                static_imports.push(resolved);
            } else {
                eprintln!(
                    "Failed to resolve static import: '{}' in '{:?}' (root: {:?})",
                    specifier, file_path, root_dir
                );
            }
        }

        // Export from: export ... from '...'
        if let Statement::ExportNamedDeclaration(decl) = stmt {
            if let Some(source) = &decl.source {
                let specifier = source.value.as_str();
                if let Some(resolved) = resolve_import(specifier, file_dir, root_dir) {
                    static_imports.push(resolved);
                }
            }
        }

        if let Statement::ExportAllDeclaration(decl) = stmt {
            let specifier = decl.source.value.as_str();
            if let Some(resolved) = resolve_import(specifier, file_dir, root_dir) {
                static_imports.push(resolved);
            }
        }
    }

    // Scan for dynamic imports: import('...')
    scan_dynamic_imports_in_program(&ret.program, file_dir, root_dir, &mut dynamic_imports);

    Ok(ImportScanResult {
        static_imports,
        dynamic_imports,
        resources: resources.into_iter().collect(),
    })
}

/// Recursively scan for dynamic import() calls in the AST
fn scan_dynamic_imports_in_program(
    program: &oxc_ast::ast::Program,
    file_dir: &Path,
    root_dir: &Path,
    dynamic_imports: &mut Vec<PathBuf>,
) {
    for stmt in &program.body {
        scan_dynamic_imports_in_stmt(stmt, file_dir, root_dir, dynamic_imports);
    }
}

fn scan_dynamic_imports_in_stmt(
    stmt: &Statement,
    file_dir: &Path,
    root_dir: &Path,
    dynamic_imports: &mut Vec<PathBuf>,
) {
    match stmt {
        Statement::ExpressionStatement(expr_stmt) => {
            scan_dynamic_imports_in_expr(
                &expr_stmt.expression,
                file_dir,
                root_dir,
                dynamic_imports,
            );
        }
        Statement::VariableDeclaration(var_decl) => {
            for decl in &var_decl.declarations {
                if let Some(init) = &decl.init {
                    scan_dynamic_imports_in_expr(init, file_dir, root_dir, dynamic_imports);
                }
            }
        }
        Statement::ReturnStatement(ret_stmt) => {
            if let Some(arg) = &ret_stmt.argument {
                scan_dynamic_imports_in_expr(arg, file_dir, root_dir, dynamic_imports);
            }
        }
        Statement::BlockStatement(block) => {
            for s in &block.body {
                scan_dynamic_imports_in_stmt(s, file_dir, root_dir, dynamic_imports);
            }
        }
        Statement::IfStatement(if_stmt) => {
            scan_dynamic_imports_in_stmt(&if_stmt.consequent, file_dir, root_dir, dynamic_imports);
            if let Some(alt) = &if_stmt.alternate {
                scan_dynamic_imports_in_stmt(alt, file_dir, root_dir, dynamic_imports);
            }
        }
        Statement::ExportDefaultDeclaration(_export_decl) => {
            // ExportDefaultDeclaration handling - skip for now, dynamic imports less common here
        }
        Statement::ExportNamedDeclaration(export_decl) => {
            if let Some(oxc_ast::ast::Declaration::VariableDeclaration(var_decl)) =
                &export_decl.declaration
            {
                for decl in &var_decl.declarations {
                    if let Some(init) = &decl.init {
                        scan_dynamic_imports_in_expr(init, file_dir, root_dir, dynamic_imports);
                    }
                }
            }
        }
        _ => {}
    }
}

fn scan_resources(
    program: &oxc_ast::ast::Program,
    file_dir: &Path,
    root_dir: &Path,
    resources: &mut HashSet<PathBuf>,
) {
    use oxc_ast::ast::*;

    // We look for ClassDeclarations with @Component decorator
    for stmt in &program.body {
        if let Statement::ExportNamedDeclaration(export_decl) = stmt {
            if let Some(Declaration::ClassDeclaration(class_decl)) = &export_decl.declaration {
                scan_class_decorators(class_decl, file_dir, root_dir, resources);
            }
        } else if let Statement::ClassDeclaration(class_decl) = stmt {
            scan_class_decorators(class_decl, file_dir, root_dir, resources);
        }
    }
}

fn scan_class_decorators(
    class_decl: &oxc_ast::ast::Class,
    file_dir: &Path,
    root_dir: &Path,
    resources: &mut HashSet<PathBuf>,
) {
    use oxc_ast::ast::*;

    for decorator in &class_decl.decorators {
        if let OxcExpression::CallExpression(call_expr) = &decorator.expression {
            if let OxcExpression::Identifier(ident) = &call_expr.callee {
                if ident.name == "Component" {
                    // check arguments[0] which should be object
                    if let Some(arg) = call_expr.arguments.first() {
                        if let Some(OxcExpression::ObjectExpression(obj)) = arg.as_expression() {
                            for prop in &obj.properties {
                                if let ObjectPropertyKind::ObjectProperty(p) = prop {
                                    if let PropertyKey::StaticIdentifier(key) = &p.key {
                                        if key.name == "templateUrl" {
                                            if let OxcExpression::StringLiteral(lit) = &p.value {
                                                if let Some(resolved) = resolve_import(
                                                    lit.value.as_str(),
                                                    file_dir,
                                                    root_dir,
                                                ) {
                                                    resources.insert(resolved);
                                                }
                                            }
                                        } else if key.name == "styleUrls" {
                                            if let OxcExpression::ArrayExpression(arr) = &p.value {
                                                for elem in &arr.elements {
                                                    if let Some(elem_expr) = elem.as_expression() {
                                                        if let OxcExpression::StringLiteral(lit) =
                                                            elem_expr
                                                        {
                                                            if let Some(resolved) = resolve_import(
                                                                lit.value.as_str(),
                                                                file_dir,
                                                                root_dir,
                                                            ) {
                                                                resources.insert(resolved);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn scan_dynamic_imports_in_expr(
    expr: &OxcExpression,
    file_dir: &Path,
    root_dir: &Path,
    dynamic_imports: &mut Vec<PathBuf>,
) {
    match expr {
        OxcExpression::ImportExpression(import_expr) => {
            // This is a dynamic import: import('...')
            if let OxcExpression::StringLiteral(lit) = &import_expr.source {
                let specifier = lit.value.as_str();
                if let Some(resolved) = resolve_import(specifier, file_dir, root_dir) {
                    dynamic_imports.push(resolved);
                }
            }
        }
        OxcExpression::CallExpression(call_expr) => {
            // Recurse into arguments
            for arg in &call_expr.arguments {
                // Handle all argument types that contain expressions
                if let Some(expr) = arg.as_expression() {
                    scan_dynamic_imports_in_expr(expr, file_dir, root_dir, dynamic_imports);
                }
            }
            scan_dynamic_imports_in_expr(&call_expr.callee, file_dir, root_dir, dynamic_imports);
        }
        OxcExpression::ArrowFunctionExpression(arrow) => {
            // Check body
            if arrow.expression {
                // body is expression
                if let Statement::ExpressionStatement(expr_stmt) = &arrow.body.statements[0] {
                    scan_dynamic_imports_in_expr(
                        &expr_stmt.expression,
                        file_dir,
                        root_dir,
                        dynamic_imports,
                    );
                }
            } else {
                for stmt in &arrow.body.statements {
                    scan_dynamic_imports_in_stmt(stmt, file_dir, root_dir, dynamic_imports);
                }
            }
        }
        OxcExpression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                    scan_dynamic_imports_in_expr(&p.value, file_dir, root_dir, dynamic_imports);
                }
            }
        }
        OxcExpression::ArrayExpression(arr) => {
            for elem in &arr.elements {
                if let Some(elem_expr) = elem.as_expression() {
                    scan_dynamic_imports_in_expr(elem_expr, file_dir, root_dir, dynamic_imports);
                }
            }
        }
        OxcExpression::ConditionalExpression(cond) => {
            scan_dynamic_imports_in_expr(&cond.consequent, file_dir, root_dir, dynamic_imports);
            scan_dynamic_imports_in_expr(&cond.alternate, file_dir, root_dir, dynamic_imports);
        }
        OxcExpression::ChainExpression(chain) => {
            if let oxc_ast::ast::ChainElement::CallExpression(call) = &chain.expression {
                for arg in &call.arguments {
                    if let Some(arg_expr) = arg.as_expression() {
                        scan_dynamic_imports_in_expr(arg_expr, file_dir, root_dir, dynamic_imports);
                    }
                }
            }
        }
        OxcExpression::StaticMemberExpression(static_member) => {
            scan_dynamic_imports_in_expr(
                &static_member.object,
                file_dir,
                root_dir,
                dynamic_imports,
            );
        }
        OxcExpression::ComputedMemberExpression(computed_member) => {
            scan_dynamic_imports_in_expr(
                &computed_member.object,
                file_dir,
                root_dir,
                dynamic_imports,
            );
        }
        _ => {}
    }
}

/// Topologically sort files based on imports
fn sort_files_topologically(files: &[PathBuf], root_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut adj: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    let file_set: HashSet<&PathBuf> = files.iter().collect();

    for file in files {
        if !file.exists() {
            continue;
        }
        let scan = scan_imports(file, root_dir)?;
        let mut deps = Vec::new();
        for dep in scan.static_imports {
            if file_set.contains(&dep) {
                deps.push(dep);
            }
        }
        adj.insert(file.clone(), deps);
    }

    let mut visited = HashSet::new();
    let mut temp_visited = HashSet::new();
    let mut order = Vec::new();

    // Use input order for deterministic iteration of independent nodes
    for file in files {
        visit_topo(file, &adj, &mut visited, &mut temp_visited, &mut order);
    }

    Ok(order)
}

fn visit_topo(
    node: &PathBuf,
    adj: &HashMap<PathBuf, Vec<PathBuf>>,
    visited: &mut HashSet<PathBuf>,
    temp_visited: &mut HashSet<PathBuf>,
    order: &mut Vec<PathBuf>,
) {
    if visited.contains(node) {
        return;
    }
    if temp_visited.contains(node) {
        return;
    } // Cycle detected

    temp_visited.insert(node.clone());

    if let Some(deps) = adj.get(node) {
        for dep in deps {
            visit_topo(dep, adj, visited, temp_visited, order);
        }
    }

    temp_visited.remove(node);
    visited.insert(node.clone());
    order.push(node.clone());
}

fn process_bundle_file(
    content: &str,
    file_path: &Path,
    root_dir: &Path,
    internal_files: &HashSet<PathBuf>,
    global_registry: &mut HashMap<(String, String), String>,
    name_counters: &mut HashMap<String, usize>,
    bundle_config: &BundleConfig,
) -> Result<String> {
    process_bundle_file_inner(
        content,
        file_path,
        root_dir,
        internal_files,
        global_registry,
        name_counters,
        false,
        bundle_config,
    )
}

fn process_bundle_file_preserve_exports(
    content: &str,
    file_path: &Path,
    root_dir: &Path,
    internal_files: &HashSet<PathBuf>,
    global_registry: &mut HashMap<(String, String), String>,
    name_counters: &mut HashMap<String, usize>,
    bundle_config: &BundleConfig,
) -> Result<String> {
    let result = process_bundle_file_inner(
        content,
        file_path,
        root_dir,
        internal_files,
        global_registry,
        name_counters,
        true,
        bundle_config,
    );

    result
}

fn process_bundle_file_inner(
    content: &str,
    file_path: &Path,
    root_dir: &Path,
    internal_files: &HashSet<PathBuf>,
    global_registry: &mut HashMap<(String, String), String>,
    name_counters: &mut HashMap<String, usize>,
    preserve_exports: bool,
    bundle_config: &BundleConfig,
) -> Result<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(file_path)
        .unwrap_or_default()
        .with_typescript(true);
    let ret = Parser::new(&allocator, content, source_type).parse();

    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut local_rewrites: HashMap<String, String> = HashMap::new();

    // Track internal namespace imports - these are `import * as ns from './internal'`
    // We need to remove the import and rewrite `ns.Symbol` → `Symbol`
    let mut internal_namespaces: HashSet<String> = HashSet::new();

    // Calculate source directory for correct import resolution (even if file_path is in dist/)
    // First, make file_path absolute if it's relative
    let abs_file_path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        root_dir.join(file_path)
    };

    let out_dir = root_dir.join(&bundle_config.output_dir);
    let output_dir_pattern = format!("/{}/", bundle_config.output_dir);
    let source_file_path = if let Ok(rel) = abs_file_path.strip_prefix(&out_dir) {
        root_dir.join(rel)
    } else {
        // Also try stripping output_dir prefix for relative paths
        let path_str = abs_file_path.to_string_lossy();
        if path_str.contains(&output_dir_pattern) {
            let stripped = path_str.replace(&output_dir_pattern, "/");
            PathBuf::from(stripped)
        } else {
            abs_file_path.clone()
        }
    };
    let file_dir = source_file_path.parent().unwrap_or(root_dir);

    // Track import declaration spans to avoid rewriting identifiers inside imports
    let mut import_spans: Vec<(usize, usize)> = Vec::new();

    // First pass: Process imports and collect rewrites
    for stmt in &ret.program.body {
        match stmt {
            Statement::ImportDeclaration(decl) => {
                let specifier = decl.source.value.as_str();
                let mut is_internal = false;

                // Debug: log all relative imports for troubleshooting
                if specifier.starts_with('.') {
                    let debug_log = format!("--- DEBUG RELATIVE ---\nFile: {:?}\nSpec: {}\nfile_dir: {:?}\nroot_dir: {:?}\n\n", 
                        file_path, specifier, file_dir, root_dir);
                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/strip_debug_rel.log")
                        .and_then(|mut f| std::io::Write::write_all(&mut f, debug_log.as_bytes()));
                }

                if let Some(resolved) = resolve_import(specifier, file_dir, root_dir) {
                    let normalized_resolved = normalize_path(&resolved);

                    let mut log = format!(
                        "--- COMPARE ---\nFile: {:?}\nSpec: {}\nResolved: {:?}\nNorm: {:?}\n",
                        file_path, specifier, resolved, normalized_resolved
                    );
                    log.push_str("Set contains:\n");
                    for p in internal_files {
                        log.push_str(&format!("  {:?}\n", p));
                    }

                    // Safe string slicing for UTF-8 content
                    let content_prefix = content
                        .char_indices()
                        .take_while(|(i, _)| *i < 500)
                        .map(|(_, c)| c)
                        .collect::<String>();
                    let content_prefix = content_prefix.as_str();
                    log.push_str(&format!("Content prefix:\n{}\n", content_prefix));

                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/strip_debug_all.log")
                        .and_then(|mut f| std::io::Write::write_all(&mut f, log.as_bytes()));

                    if internal_files.contains(&normalized_resolved) {
                        is_internal = true;
                    }

                    // Also check if the source file version is in internal_files
                    // This handles the case where compiled files are in output_dir but internal_files contains source paths
                    if !is_internal {
                        let resolved_str = normalized_resolved.to_string_lossy();
                        // Strip output_dir prefix and change .js to .ts for comparison
                        let source_path_str = resolved_str
                            .replace(&output_dir_pattern, "/")
                            .replace(".js", ".ts");
                        let source_path = PathBuf::from(&source_path_str);
                        if internal_files.contains(&source_path) {
                            is_internal = true;
                        }
                    }
                }

                if is_internal {
                    // For internal imports, we need to:
                    // 1. Remove the import statement entirely
                    // 2. Track namespace imports so we can rewrite `ns.Symbol` → `Symbol`

                    if let Some(specifiers) = &decl.specifiers {
                        for spec in specifiers {
                            if let oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns) = spec {
                                 // Track namespace name for later rewriting (e.g., i_3 in `import * as i_3 from '...'`)
                                 internal_namespaces.insert(ns.local.name.to_string());
                             }
                        }
                    }

                    // Remove internal import entirely (both named and namespace imports)
                    edits.push((
                        decl.span.start as usize,
                        decl.span.end as usize,
                        "".to_string(),
                    ));
                } else {
                    // Track this import span
                    import_spans.push((decl.span.start as usize, decl.span.end as usize));

                    // External Import - Rewrite inline with unique aliases
                    if let Some(specifiers) = &decl.specifiers {
                        let mut named_parts: Vec<String> = Vec::new();
                        let mut default_part: Option<String> = None;
                        let mut namespace_part: Option<String> = None;

                        for spec in specifiers {
                            let (imported_name, local_name) = match spec {
                                 oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                     (s.imported.name().as_str().to_string(), s.local.name.as_str().to_string())
                                 },
                                 oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                     ("default".to_string(), s.local.name.as_str().to_string())
                                 },
                                 oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                     ("*".to_string(), s.local.name.as_str().to_string())
                                 },
                             };

                            // Get or create unique alias
                            // Key: (module, imported_name) -> unique_alias
                            let key = (specifier.to_string(), imported_name.clone());
                            let (unique_alias, is_new_entry) = if let Some(a) =
                                global_registry.get(&key)
                            {
                                (a.clone(), false) // Already exists - don't emit import again
                            } else {
                                // Get or create per-identifier counter
                                let count = name_counters.entry(local_name.clone()).or_insert(0);
                                let new_alias = if *count == 0 {
                                    // First occurrence - use original name without suffix
                                    local_name.clone()
                                } else {
                                    // Subsequent occurrences - start from 2
                                    format!("{}{}", local_name, *count + 1)
                                };
                                *count += 1;
                                global_registry.insert(key, new_alias.clone());
                                (new_alias, true) // New entry - emit import
                            };

                            // Collect rewrite mapping if alias differs from local name
                            if local_name != unique_alias {
                                local_rewrites.insert(local_name.clone(), unique_alias.clone());
                            }

                            // Only add to import parts if this is a NEW entry (first time seeing it)
                            if is_new_entry {
                                // Build import parts
                                if imported_name == "default" {
                                    default_part = Some(unique_alias);
                                } else if imported_name == "*" {
                                    namespace_part = Some(unique_alias);
                                } else {
                                    // Only use 'as' if alias differs from imported name
                                    if imported_name == unique_alias {
                                        named_parts.push(imported_name);
                                    } else {
                                        named_parts
                                            .push(format!("{} as {}", imported_name, unique_alias));
                                    }
                                }
                            }
                        }

                        // Build new import statement(s)
                        let mut new_imports = Vec::new();

                        if let Some(alias) = default_part {
                            new_imports.push(format!("import {} from '{}';", alias, specifier));
                        }
                        if let Some(alias) = namespace_part {
                            new_imports.push(format!("import*as {} from '{}';", alias, specifier));
                        }
                        if !named_parts.is_empty() {
                            new_imports.push(format!(
                                "import {{{}}} from '{}';",
                                named_parts.join(","),
                                specifier
                            ));
                        }

                        edits.push((
                            decl.span.start as usize,
                            decl.span.end as usize,
                            new_imports.join("\n"),
                        ));
                    }
                }
            }
            Statement::VariableDeclaration(decl) => {
                if matches!(
                    decl.kind,
                    oxc_ast::ast::VariableDeclarationKind::Const
                        | oxc_ast::ast::VariableDeclarationKind::Let
                ) {
                    let len = if matches!(decl.kind, oxc_ast::ast::VariableDeclarationKind::Const) {
                        5
                    } else {
                        3
                    };
                    edits.push((
                        decl.span.start as usize,
                        decl.span.start as usize + len,
                        "var".to_string(),
                    ));
                }
            }
            Statement::ExportNamedDeclaration(decl) => {
                if let Some(inner_decl) = &decl.declaration {
                    // Remove 'export' keyword only for main bundle, not for chunks
                    if !preserve_exports {
                        edits.push((
                            decl.span.start as usize,
                            inner_decl.span().start as usize,
                            "".to_string(),
                        ));
                    }

                    if let oxc_ast::ast::Declaration::VariableDeclaration(var_decl) = inner_decl {
                        if matches!(
                            var_decl.kind,
                            oxc_ast::ast::VariableDeclarationKind::Const
                                | oxc_ast::ast::VariableDeclarationKind::Let
                        ) {
                            let len = if matches!(
                                var_decl.kind,
                                oxc_ast::ast::VariableDeclarationKind::Const
                            ) {
                                5
                            } else {
                                3
                            };
                            edits.push((
                                var_decl.span.start as usize,
                                var_decl.span.start as usize + len,
                                "var".to_string(),
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Second pass: Walk AST to find identifier references and rewrite them
    if !local_rewrites.is_empty() {
        collect_identifier_edits(
            &ret.program.body,
            &local_rewrites,
            &import_spans,
            &mut edits,
        );
    }

    // Third pass: Rewrite internal namespace member expressions (e.g., `i_3.FeatureComponent` → `FeatureComponent`)
    if !internal_namespaces.is_empty() {
        collect_namespace_member_edits(
            &ret.program.body,
            &internal_namespaces,
            &import_spans,
            &mut edits,
        );
    }

    // Sort edits by start position descending (important for applying in reverse order)
    edits.sort_by(|a, b| b.0.cmp(&a.0));

    // Deduplicate edits at same position (keep first occurrence after sort)
    edits.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

    // Fix: Ensure /* @vite-ignore */ comment exists in HMR dynamic imports.
    // The comment might have been stripped during earlier transformations (e.g. AST parsing).
    // We scan for dynamic imports involving `ɵɵgetReplaceMetadataURL` and re-inject the comment if missing.
    let mut search_idx = 0;
    let mut added_fix = false;
    while let Some(idx) = content[search_idx..].find("ɵɵgetReplaceMetadataURL") {
        let actual_idx = search_idx + idx;

        // Look backwards for "import(" within reasonable range
        let lookback_start = actual_idx.saturating_sub(100);
        let lookback_slice = &content[lookback_start..actual_idx];

        if let Some(import_rel_idx) = lookback_slice.rfind("import(") {
            let import_start = lookback_start + import_rel_idx;
            let import_end = import_start + 7; // "import(" length

            // Check if comment already exists between import_end and actual_idx
            let gap = &content[import_end..actual_idx];
            if !gap.contains("@vite-ignore") {
                edits.push((import_end, import_end, "/* @vite-ignore */ ".to_string()));
                // eprintln!("APPLYING FIX: Re-injected @vite-ignore comment at {}", import_end);
                // eprintln!("APPLYING FIX: Re-injected @vite-ignore comment at {}", import_end);
                added_fix = true;
            } else {
                // eprintln!("FIX SKIPPED: Comment already present: {:?}", gap);
            }
        }
        search_idx = actual_idx + "ɵɵgetReplaceMetadataURL".len();
    }

    if added_fix {
        // Re-sort edits because we added new ones
        edits.sort_by(|a, b| b.0.cmp(&a.0));
    }

    let mut new_content = content.to_string();
    for (start, end, replacement) in edits {
        if start < new_content.len() && end <= new_content.len() {
            new_content.replace_range(start..end, &replacement);
        }
    }

    Ok(new_content)
}

/// Recursively collect identifier edits from AST
fn collect_identifier_edits(
    stmts: &oxc_allocator::Vec<'_, Statement<'_>>,
    rewrites: &HashMap<String, String>,
    import_spans: &[(usize, usize)],
    edits: &mut Vec<(usize, usize, String)>,
) {
    for stmt in stmts {
        collect_identifiers_in_statement(stmt, rewrites, import_spans, edits);
    }
}

/// Recursively collect namespace member expression edits from AST
/// This rewrites `ns.Symbol` → `Symbol` when `ns` is an internal namespace
fn collect_namespace_member_edits(
    stmts: &oxc_allocator::Vec<'_, Statement<'_>>,
    internal_namespaces: &HashSet<String>,
    import_spans: &[(usize, usize)],
    edits: &mut Vec<(usize, usize, String)>,
) {
    for stmt in stmts {
        collect_namespace_members_in_statement(stmt, internal_namespaces, import_spans, edits);
    }
}

fn collect_namespace_members_in_statement(
    stmt: &Statement<'_>,
    internal_namespaces: &HashSet<String>,
    import_spans: &[(usize, usize)],
    edits: &mut Vec<(usize, usize, String)>,
) {
    match stmt {
        Statement::ImportDeclaration(_) => {}
        Statement::ExpressionStatement(expr_stmt) => {
            collect_namespace_members_in_expression(
                &expr_stmt.expression,
                internal_namespaces,
                import_spans,
                edits,
            );
        }
        Statement::VariableDeclaration(decl) => {
            for declarator in &decl.declarations {
                if let Some(init) = &declarator.init {
                    collect_namespace_members_in_expression(
                        init,
                        internal_namespaces,
                        import_spans,
                        edits,
                    );
                }
            }
        }
        Statement::ClassDeclaration(class_decl) => {
            collect_namespace_members_in_class(
                class_decl,
                internal_namespaces,
                import_spans,
                edits,
            );
        }
        Statement::ExportNamedDeclaration(export_decl) => {
            if let Some(decl) = &export_decl.declaration {
                match decl {
                    oxc_ast::ast::Declaration::VariableDeclaration(var_decl) => {
                        for declarator in &var_decl.declarations {
                            if let Some(init) = &declarator.init {
                                collect_namespace_members_in_expression(
                                    init,
                                    internal_namespaces,
                                    import_spans,
                                    edits,
                                );
                            }
                        }
                    }
                    oxc_ast::ast::Declaration::ClassDeclaration(class_decl) => {
                        collect_namespace_members_in_class(
                            class_decl,
                            internal_namespaces,
                            import_spans,
                            edits,
                        );
                    }
                    oxc_ast::ast::Declaration::FunctionDeclaration(func_decl) => {
                        if let Some(body) = &func_decl.body {
                            for stmt in &body.statements {
                                collect_namespace_members_in_statement(
                                    stmt,
                                    internal_namespaces,
                                    import_spans,
                                    edits,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Statement::FunctionDeclaration(func_decl) => {
            if let Some(body) = &func_decl.body {
                for stmt in &body.statements {
                    collect_namespace_members_in_statement(
                        stmt,
                        internal_namespaces,
                        import_spans,
                        edits,
                    );
                }
            }
        }
        Statement::ReturnStatement(ret_stmt) => {
            if let Some(arg) = &ret_stmt.argument {
                collect_namespace_members_in_expression(
                    arg,
                    internal_namespaces,
                    import_spans,
                    edits,
                );
            }
        }
        Statement::IfStatement(if_stmt) => {
            collect_namespace_members_in_expression(
                &if_stmt.test,
                internal_namespaces,
                import_spans,
                edits,
            );
            collect_namespace_members_in_statement(
                &if_stmt.consequent,
                internal_namespaces,
                import_spans,
                edits,
            );
            if let Some(alt) = &if_stmt.alternate {
                collect_namespace_members_in_statement(
                    alt,
                    internal_namespaces,
                    import_spans,
                    edits,
                );
            }
        }
        Statement::BlockStatement(block) => {
            for stmt in &block.body {
                collect_namespace_members_in_statement(
                    stmt,
                    internal_namespaces,
                    import_spans,
                    edits,
                );
            }
        }
        _ => {}
    }
}

fn collect_namespace_members_in_class(
    class_decl: &oxc_ast::ast::Class<'_>,
    internal_namespaces: &HashSet<String>,
    import_spans: &[(usize, usize)],
    edits: &mut Vec<(usize, usize, String)>,
) {
    if let Some(super_class) = &class_decl.super_class {
        collect_namespace_members_in_expression(
            super_class,
            internal_namespaces,
            import_spans,
            edits,
        );
    }

    for element in &class_decl.body.body {
        match element {
            oxc_ast::ast::ClassElement::PropertyDefinition(prop) => {
                if let Some(value) = &prop.value {
                    collect_namespace_members_in_expression(
                        value,
                        internal_namespaces,
                        import_spans,
                        edits,
                    );
                }
            }
            oxc_ast::ast::ClassElement::MethodDefinition(method) => {
                if let Some(body) = &method.value.body {
                    for stmt in &body.statements {
                        collect_namespace_members_in_statement(
                            stmt,
                            internal_namespaces,
                            import_spans,
                            edits,
                        );
                    }
                }
            }
            oxc_ast::ast::ClassElement::StaticBlock(static_block) => {
                for stmt in &static_block.body {
                    collect_namespace_members_in_statement(
                        stmt,
                        internal_namespaces,
                        import_spans,
                        edits,
                    );
                }
            }
            _ => {}
        }
    }
}

fn collect_namespace_members_in_expression(
    expr: &oxc_ast::ast::Expression<'_>,
    internal_namespaces: &HashSet<String>,
    import_spans: &[(usize, usize)],
    edits: &mut Vec<(usize, usize, String)>,
) {
    match expr {
        // Check StaticMemberExpression for namespace references: `ns.Symbol` → `Symbol`
        oxc_ast::ast::Expression::StaticMemberExpression(member) => {
            // Check if the object is an identifier that's an internal namespace
            if let oxc_ast::ast::Expression::Identifier(obj_ident) = &member.object {
                let ns_name = obj_ident.name.as_str();
                if internal_namespaces.contains(ns_name) {
                    // This is an internal namespace member access like `i_3.FeatureComponent`
                    // Rewrite the entire expression to just the property name
                    let property_name = member.property.name.as_str();
                    let start = member.span.start as usize;
                    let end = member.span.end as usize;
                    if !is_inside_import(start, import_spans) {
                        edits.push((start, end, property_name.to_string()));
                    }
                    return; // Don't recurse further into this expression
                }
            }
            // Not an internal namespace, recurse into object
            collect_namespace_members_in_expression(
                &member.object,
                internal_namespaces,
                import_spans,
                edits,
            );
        }
        oxc_ast::ast::Expression::CallExpression(call) => {
            collect_namespace_members_in_expression(
                &call.callee,
                internal_namespaces,
                import_spans,
                edits,
            );
            for arg in &call.arguments {
                match arg {
                    oxc_ast::ast::Argument::SpreadElement(spread) => {
                        collect_namespace_members_in_expression(
                            &spread.argument,
                            internal_namespaces,
                            import_spans,
                            edits,
                        );
                    }
                    _ => {
                        if let Some(expr) = arg.as_expression() {
                            collect_namespace_members_in_expression(
                                expr,
                                internal_namespaces,
                                import_spans,
                                edits,
                            );
                        }
                    }
                }
            }
        }
        oxc_ast::ast::Expression::ComputedMemberExpression(member) => {
            collect_namespace_members_in_expression(
                &member.object,
                internal_namespaces,
                import_spans,
                edits,
            );
            collect_namespace_members_in_expression(
                &member.expression,
                internal_namespaces,
                import_spans,
                edits,
            );
        }
        oxc_ast::ast::Expression::PrivateFieldExpression(member) => {
            collect_namespace_members_in_expression(
                &member.object,
                internal_namespaces,
                import_spans,
                edits,
            );
        }
        oxc_ast::ast::Expression::ArrayExpression(arr) => {
            for elem in &arr.elements {
                match elem {
                    oxc_ast::ast::ArrayExpressionElement::SpreadElement(spread) => {
                        collect_namespace_members_in_expression(
                            &spread.argument,
                            internal_namespaces,
                            import_spans,
                            edits,
                        );
                    }
                    oxc_ast::ast::ArrayExpressionElement::Elision(_) => {}
                    _ => {
                        if let Some(expr) = elem.as_expression() {
                            collect_namespace_members_in_expression(
                                expr,
                                internal_namespaces,
                                import_spans,
                                edits,
                            );
                        }
                    }
                }
            }
        }
        oxc_ast::ast::Expression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                match prop {
                    oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) => {
                        collect_namespace_members_in_expression(
                            &p.value,
                            internal_namespaces,
                            import_spans,
                            edits,
                        );
                    }
                    oxc_ast::ast::ObjectPropertyKind::SpreadProperty(spread) => {
                        collect_namespace_members_in_expression(
                            &spread.argument,
                            internal_namespaces,
                            import_spans,
                            edits,
                        );
                    }
                }
            }
        }
        oxc_ast::ast::Expression::NewExpression(new_expr) => {
            collect_namespace_members_in_expression(
                &new_expr.callee,
                internal_namespaces,
                import_spans,
                edits,
            );
            for arg in &new_expr.arguments {
                if let Some(expr) = arg.as_expression() {
                    collect_namespace_members_in_expression(
                        expr,
                        internal_namespaces,
                        import_spans,
                        edits,
                    );
                }
            }
        }
        oxc_ast::ast::Expression::ArrowFunctionExpression(arrow) => {
            for stmt in &arrow.body.statements {
                collect_namespace_members_in_statement(
                    stmt,
                    internal_namespaces,
                    import_spans,
                    edits,
                );
            }
        }
        oxc_ast::ast::Expression::FunctionExpression(func) => {
            if let Some(body) = &func.body {
                for stmt in &body.statements {
                    collect_namespace_members_in_statement(
                        stmt,
                        internal_namespaces,
                        import_spans,
                        edits,
                    );
                }
            }
        }
        oxc_ast::ast::Expression::ConditionalExpression(cond) => {
            collect_namespace_members_in_expression(
                &cond.test,
                internal_namespaces,
                import_spans,
                edits,
            );
            collect_namespace_members_in_expression(
                &cond.consequent,
                internal_namespaces,
                import_spans,
                edits,
            );
            collect_namespace_members_in_expression(
                &cond.alternate,
                internal_namespaces,
                import_spans,
                edits,
            );
        }
        oxc_ast::ast::Expression::BinaryExpression(bin) => {
            collect_namespace_members_in_expression(
                &bin.left,
                internal_namespaces,
                import_spans,
                edits,
            );
            collect_namespace_members_in_expression(
                &bin.right,
                internal_namespaces,
                import_spans,
                edits,
            );
        }
        oxc_ast::ast::Expression::LogicalExpression(log) => {
            collect_namespace_members_in_expression(
                &log.left,
                internal_namespaces,
                import_spans,
                edits,
            );
            collect_namespace_members_in_expression(
                &log.right,
                internal_namespaces,
                import_spans,
                edits,
            );
        }
        oxc_ast::ast::Expression::UnaryExpression(unary) => {
            collect_namespace_members_in_expression(
                &unary.argument,
                internal_namespaces,
                import_spans,
                edits,
            );
        }
        oxc_ast::ast::Expression::AssignmentExpression(assign) => {
            collect_namespace_members_in_expression(
                &assign.right,
                internal_namespaces,
                import_spans,
                edits,
            );
        }
        oxc_ast::ast::Expression::SequenceExpression(seq) => {
            for expr in &seq.expressions {
                collect_namespace_members_in_expression(
                    expr,
                    internal_namespaces,
                    import_spans,
                    edits,
                );
            }
        }
        oxc_ast::ast::Expression::ParenthesizedExpression(paren) => {
            collect_namespace_members_in_expression(
                &paren.expression,
                internal_namespaces,
                import_spans,
                edits,
            );
        }
        oxc_ast::ast::Expression::TemplateLiteral(template) => {
            for expr in &template.expressions {
                collect_namespace_members_in_expression(
                    expr,
                    internal_namespaces,
                    import_spans,
                    edits,
                );
            }
        }
        oxc_ast::ast::Expression::TaggedTemplateExpression(tagged) => {
            collect_namespace_members_in_expression(
                &tagged.tag,
                internal_namespaces,
                import_spans,
                edits,
            );
            for expr in &tagged.quasi.expressions {
                collect_namespace_members_in_expression(
                    expr,
                    internal_namespaces,
                    import_spans,
                    edits,
                );
            }
        }
        _ => {}
    }
}

fn is_inside_import(pos: usize, import_spans: &[(usize, usize)]) -> bool {
    import_spans
        .iter()
        .any(|(start, end)| pos >= *start && pos < *end)
}

fn collect_identifiers_in_statement(
    stmt: &Statement<'_>,
    rewrites: &HashMap<String, String>,
    import_spans: &[(usize, usize)],
    edits: &mut Vec<(usize, usize, String)>,
) {
    match stmt {
        Statement::ImportDeclaration(_) => {
            // Skip import declarations - they're handled separately
        }
        Statement::ExpressionStatement(expr_stmt) => {
            collect_identifiers_in_expression(&expr_stmt.expression, rewrites, import_spans, edits);
        }
        Statement::VariableDeclaration(decl) => {
            for declarator in &decl.declarations {
                if let Some(init) = &declarator.init {
                    collect_identifiers_in_expression(init, rewrites, import_spans, edits);
                }
            }
        }
        Statement::ClassDeclaration(class_decl) => {
            collect_identifiers_in_class(&class_decl, rewrites, import_spans, edits);
        }
        Statement::ExportNamedDeclaration(export_decl) => {
            if let Some(decl) = &export_decl.declaration {
                match decl {
                    oxc_ast::ast::Declaration::VariableDeclaration(var_decl) => {
                        for declarator in &var_decl.declarations {
                            if let Some(init) = &declarator.init {
                                collect_identifiers_in_expression(
                                    init,
                                    rewrites,
                                    import_spans,
                                    edits,
                                );
                            }
                        }
                    }
                    oxc_ast::ast::Declaration::ClassDeclaration(class_decl) => {
                        collect_identifiers_in_class(&class_decl, rewrites, import_spans, edits);
                    }
                    oxc_ast::ast::Declaration::FunctionDeclaration(func_decl) => {
                        if let Some(body) = &func_decl.body {
                            for stmt in &body.statements {
                                collect_identifiers_in_statement(
                                    stmt,
                                    rewrites,
                                    import_spans,
                                    edits,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Statement::FunctionDeclaration(func_decl) => {
            if let Some(body) = &func_decl.body {
                for stmt in &body.statements {
                    collect_identifiers_in_statement(stmt, rewrites, import_spans, edits);
                }
            }
        }
        Statement::ReturnStatement(ret_stmt) => {
            if let Some(arg) = &ret_stmt.argument {
                collect_identifiers_in_expression(arg, rewrites, import_spans, edits);
            }
        }
        Statement::IfStatement(if_stmt) => {
            collect_identifiers_in_expression(&if_stmt.test, rewrites, import_spans, edits);
            collect_identifiers_in_statement(&if_stmt.consequent, rewrites, import_spans, edits);
            if let Some(alt) = &if_stmt.alternate {
                collect_identifiers_in_statement(alt, rewrites, import_spans, edits);
            }
        }
        Statement::BlockStatement(block) => {
            for stmt in &block.body {
                collect_identifiers_in_statement(stmt, rewrites, import_spans, edits);
            }
        }
        _ => {}
    }
}

fn collect_identifiers_in_class(
    class_decl: &oxc_ast::ast::Class<'_>,
    rewrites: &HashMap<String, String>,
    import_spans: &[(usize, usize)],
    edits: &mut Vec<(usize, usize, String)>,
) {
    // Check superclass
    if let Some(super_class) = &class_decl.super_class {
        collect_identifiers_in_expression(super_class, rewrites, import_spans, edits);
    }

    // Check class body
    for element in &class_decl.body.body {
        match element {
            oxc_ast::ast::ClassElement::PropertyDefinition(prop) => {
                if let Some(value) = &prop.value {
                    collect_identifiers_in_expression(value, rewrites, import_spans, edits);
                }
            }
            oxc_ast::ast::ClassElement::MethodDefinition(method) => {
                if let Some(body) = &method.value.body {
                    for stmt in &body.statements {
                        collect_identifiers_in_statement(stmt, rewrites, import_spans, edits);
                    }
                }
            }
            oxc_ast::ast::ClassElement::StaticBlock(static_block) => {
                for stmt in &static_block.body {
                    collect_identifiers_in_statement(stmt, rewrites, import_spans, edits);
                }
            }
            _ => {}
        }
    }
}

fn collect_identifiers_in_expression(
    expr: &oxc_ast::ast::Expression<'_>,
    rewrites: &HashMap<String, String>,
    import_spans: &[(usize, usize)],
    edits: &mut Vec<(usize, usize, String)>,
) {
    match expr {
        oxc_ast::ast::Expression::Identifier(ident) => {
            let name = ident.name.as_str();
            if let Some(new_name) = rewrites.get(name) {
                let pos = ident.span.start as usize;
                if !is_inside_import(pos, import_spans) {
                    edits.push((pos, ident.span.end as usize, new_name.clone()));
                }
            }
        }
        oxc_ast::ast::Expression::CallExpression(call) => {
            collect_identifiers_in_expression(&call.callee, rewrites, import_spans, edits);
            for arg in &call.arguments {
                match arg {
                    oxc_ast::ast::Argument::SpreadElement(spread) => {
                        collect_identifiers_in_expression(
                            &spread.argument,
                            rewrites,
                            import_spans,
                            edits,
                        );
                    }
                    _ => {
                        if let Some(expr) = arg.as_expression() {
                            collect_identifiers_in_expression(expr, rewrites, import_spans, edits);
                        }
                    }
                }
            }
        }
        oxc_ast::ast::Expression::StaticMemberExpression(member) => {
            collect_identifiers_in_expression(&member.object, rewrites, import_spans, edits);
            // Don't rewrite the property name itself
        }
        oxc_ast::ast::Expression::ComputedMemberExpression(member) => {
            collect_identifiers_in_expression(&member.object, rewrites, import_spans, edits);
            collect_identifiers_in_expression(&member.expression, rewrites, import_spans, edits);
        }
        oxc_ast::ast::Expression::PrivateFieldExpression(member) => {
            collect_identifiers_in_expression(&member.object, rewrites, import_spans, edits);
        }
        oxc_ast::ast::Expression::ArrayExpression(arr) => {
            for elem in &arr.elements {
                match elem {
                    oxc_ast::ast::ArrayExpressionElement::SpreadElement(spread) => {
                        collect_identifiers_in_expression(
                            &spread.argument,
                            rewrites,
                            import_spans,
                            edits,
                        );
                    }
                    oxc_ast::ast::ArrayExpressionElement::Elision(_) => {}
                    _ => {
                        if let Some(expr) = elem.as_expression() {
                            collect_identifiers_in_expression(expr, rewrites, import_spans, edits);
                        }
                    }
                }
            }
        }
        oxc_ast::ast::Expression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                match prop {
                    oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) => {
                        collect_identifiers_in_expression(&p.value, rewrites, import_spans, edits);
                    }
                    oxc_ast::ast::ObjectPropertyKind::SpreadProperty(spread) => {
                        collect_identifiers_in_expression(
                            &spread.argument,
                            rewrites,
                            import_spans,
                            edits,
                        );
                    }
                }
            }
        }
        oxc_ast::ast::Expression::NewExpression(new_expr) => {
            collect_identifiers_in_expression(&new_expr.callee, rewrites, import_spans, edits);
            for arg in &new_expr.arguments {
                if let Some(expr) = arg.as_expression() {
                    collect_identifiers_in_expression(expr, rewrites, import_spans, edits);
                }
            }
        }
        oxc_ast::ast::Expression::ArrowFunctionExpression(arrow) => {
            for stmt in &arrow.body.statements {
                collect_identifiers_in_statement(stmt, rewrites, import_spans, edits);
            }
        }
        oxc_ast::ast::Expression::FunctionExpression(func) => {
            if let Some(body) = &func.body {
                for stmt in &body.statements {
                    collect_identifiers_in_statement(stmt, rewrites, import_spans, edits);
                }
            }
        }
        oxc_ast::ast::Expression::ConditionalExpression(cond) => {
            collect_identifiers_in_expression(&cond.test, rewrites, import_spans, edits);
            collect_identifiers_in_expression(&cond.consequent, rewrites, import_spans, edits);
            collect_identifiers_in_expression(&cond.alternate, rewrites, import_spans, edits);
        }
        oxc_ast::ast::Expression::BinaryExpression(bin) => {
            collect_identifiers_in_expression(&bin.left, rewrites, import_spans, edits);
            collect_identifiers_in_expression(&bin.right, rewrites, import_spans, edits);
        }
        oxc_ast::ast::Expression::LogicalExpression(log) => {
            collect_identifiers_in_expression(&log.left, rewrites, import_spans, edits);
            collect_identifiers_in_expression(&log.right, rewrites, import_spans, edits);
        }
        oxc_ast::ast::Expression::UnaryExpression(unary) => {
            collect_identifiers_in_expression(&unary.argument, rewrites, import_spans, edits);
        }
        oxc_ast::ast::Expression::AssignmentExpression(assign) => {
            match &assign.left {
                oxc_ast::ast::AssignmentTarget::AssignmentTargetIdentifier(ident) => {
                    let name = ident.name.as_str();
                    if let Some(new_name) = rewrites.get(name) {
                        let pos = ident.span.start as usize;
                        if !is_inside_import(pos, import_spans) {
                            edits.push((pos, ident.span.end as usize, new_name.clone()));
                        }
                    }
                }
                _ => {}
            }
            collect_identifiers_in_expression(&assign.right, rewrites, import_spans, edits);
        }
        oxc_ast::ast::Expression::SequenceExpression(seq) => {
            for expr in &seq.expressions {
                collect_identifiers_in_expression(expr, rewrites, import_spans, edits);
            }
        }
        oxc_ast::ast::Expression::TemplateLiteral(template) => {
            for expr in &template.expressions {
                collect_identifiers_in_expression(expr, rewrites, import_spans, edits);
            }
        }
        oxc_ast::ast::Expression::TaggedTemplateExpression(tagged) => {
            collect_identifiers_in_expression(&tagged.tag, rewrites, import_spans, edits);
        }
        oxc_ast::ast::Expression::ParenthesizedExpression(paren) => {
            collect_identifiers_in_expression(&paren.expression, rewrites, import_spans, edits);
        }
        _ => {}
    }
}

/// Resolve import specifier to absolute path
fn resolve_import(specifier: &str, file_dir: &Path, root_dir: &Path) -> Option<PathBuf> {
    // 1. Handle relative imports
    if specifier.starts_with('.') {
        let joined = file_dir.join(specifier);
        return resolve_file_path(&joined);
    }

    // 2. Handle absolute imports (rare in functional code but possibilities exist)
    if specifier.starts_with('/') {
        return resolve_file_path(Path::new(specifier));
    }

    // 3. Handle node_modules resolution

    // a) Try manual exports resolution first (modern)
    if let Some(resolved) = resolve_package_exports(specifier, root_dir) {
        return Some(resolved);
    }

    // b) Check root_dir/node_modules/<specifier> (legacy/fallback)
    let node_modules = root_dir.join("node_modules");
    let package_path = node_modules.join(specifier);

    if package_path.exists() {
        // Check package.json for "module" or "main"
        let package_json = package_path.join("package.json");
        if package_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Try 'fesm2022', 'es2020', 'module', 'main' in that order for Angular/ESM preference
                    let entry_point = json
                        .get("fesm2022")
                        .or_else(|| json.get("es2020"))
                        .or_else(|| json.get("module"))
                        .or_else(|| json.get("main"))
                        .and_then(|v| v.as_str());

                    if let Some(entry) = entry_point {
                        let entry_path = package_path.join(entry);
                        if let Some(resolved) = resolve_file_path(&entry_path) {
                            return Some(resolved);
                        }
                    }
                }
            }
        }

        // Try index.ts, index.js, index.mjs
        if let Some(resolved) = resolve_file_path(&package_path.join("index")) {
            return Some(resolved);
        }
    }

    None
}

/// Helper to try extensions
fn resolve_file_path(path: &Path) -> Option<PathBuf> {
    // 1. Try exact path
    if path.is_file() {
        return Some(normalize_path(path));
    }

    // 2. Try extensions (regardless of whether it has one, e.g. app.config -> app.config.ts)
    let extensions = [".ts", ".js", ".mjs", ".d.ts"];

    for ext in &extensions {
        let mut p = path.as_os_str().to_os_string();
        p.push(ext);
        let p_buf = PathBuf::from(p);
        if p_buf.is_file() {
            return Some(normalize_path(&p_buf));
        }
    }

    // 3. Directory index resolution
    if path.is_dir() {
        for ext in &extensions {
            let p = path.join(format!("index{}", ext));
            if p.is_file() {
                return Some(normalize_path(&p));
            }
        }
    }

    None
}

/// Try to resolve package exports manually (since we don't have oxc_resolver)
fn resolve_package_exports(specifier: &str, root_dir: &Path) -> Option<PathBuf> {
    // 1. Determine package name and subpath
    let parts: Vec<&str> = specifier.split('/').collect();
    let (pkg_name, subpath) = if specifier.starts_with('@') {
        if parts.len() < 2 {
            return None;
        }
        (format!("{}/{}", parts[0], parts[1]), parts[2..].join("/"))
    } else {
        if parts.is_empty() {
            return None;
        }
        (parts[0].to_string(), parts[1..].join("/"))
    };

    let node_modules = root_dir.join("node_modules");
    let package_path = node_modules.join(&pkg_name);
    let package_json_path = package_path.join("package.json");

    if !package_json_path.exists() {
        // Fallback: maybe it's nested in another node_modules (simplified: just check root)
        return None;
    }

    if let Ok(content) = std::fs::read_to_string(&package_json_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            // Check 'exports'
            if let Some(exports) = json.get("exports") {
                let lookup_key = if subpath.is_empty() {
                    ".".to_string()
                } else {
                    format!("./{}", subpath)
                };

                if let Some(export_entry) = exports.get(&lookup_key) {
                    // Handle string or object
                    if let Some(s) = export_entry.as_str() {
                        return Some(normalize_path(&package_path.join(s)));
                    }
                    if let Some(obj) = export_entry.as_object() {
                        let entry = obj
                            .get("fesm2022")
                            .or_else(|| obj.get("es2020")) // Angular logic
                            .or_else(|| obj.get("module"))
                            .or_else(|| obj.get("import"))
                            .or_else(|| obj.get("default"))
                            .and_then(|v| v.as_str());

                        if let Some(e) = entry {
                            return Some(normalize_path(&package_path.join(e)));
                        }
                    }
                }
                // Wildcard matching (simple)
                // e.g. "./prebuilt-themes/*" -> "./prebuilt-themes/*.css"
                // This is a naive implementation
                if let Some(obj) = exports.as_object() {
                    for (key, val) in obj {
                        if key.contains('*') {
                            // simplistic glob match (only supports prefix/*)
                            let prefix = key.replace("*", "");
                            if lookup_key.starts_with(&prefix) {
                                let remainder = lookup_key.strip_prefix(&prefix).unwrap_or("");

                                // Get target pattern
                                let target = if let Some(s) = val.as_str() {
                                    Some(s)
                                } else if let Some(target_obj) = val.as_object() {
                                    target_obj
                                        .get("default")
                                        .and_then(|v| v.as_str())
                                        .or_else(|| {
                                            target_obj.get("module").and_then(|v| v.as_str())
                                        })
                                        .or_else(|| {
                                            target_obj.get("import").and_then(|v| v.as_str())
                                        })
                                        .or_else(|| {
                                            target_obj.get("style").and_then(|v| v.as_str())
                                        })
                                } else {
                                    None
                                };

                                if let Some(t) = target {
                                    let resolved_target = t.replace("*", remainder);
                                    let final_path =
                                        normalize_path(&package_path.join(&resolved_target));
                                    // eprintln!("Resolved wildcard: {} -> {} -> {} -> {:?}", lookup_key, key, resolved_target, final_path);
                                    return Some(final_path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                if let Some(std::path::Component::Normal(_)) = components.last() {
                    components.pop();
                } else {
                    components.push(component);
                }
            }
            std::path::Component::CurDir => {}
            _ => components.push(component),
        }
    }
    components.iter().collect()
}

/// Build the import graph starting from entry point
fn build_import_graph(
    entry: &Path,
    root_dir: &Path,
    ignored_files: Option<&HashSet<PathBuf>>,
) -> Result<(HashSet<PathBuf>, HashSet<PathBuf>, HashSet<PathBuf>)> {
    let mut static_set = HashSet::new();
    let mut dynamic_set = HashSet::new();
    let mut resources_set = HashSet::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // Init visited with ignored files so we don't traverse them
    if let Some(ignored) = ignored_files {
        for f in ignored {
            visited.insert(f.clone());
        }
    }

    queue.push_back(entry.to_path_buf());

    while let Some(file) = queue.pop_front() {
        if visited.contains(&file) {
            continue;
        }
        visited.insert(file.clone());

        if !file.exists() {
            continue;
        }

        static_set.insert(file.clone());

        let scan_result = scan_imports(&file, root_dir)?;

        // Add static imports to queue
        for static_import in scan_result.static_imports {
            if !visited.contains(&static_import) {
                queue.push_back(static_import);
            }
        }

        // Record dynamic imports but don't traverse them (they become chunks)
        for dynamic_import in scan_result.dynamic_imports {
            dynamic_set.insert(dynamic_import);
        }

        // Record resources
        for res in scan_result.resources {
            resources_set.insert(res);
        }
    }

    Ok((static_set, dynamic_set, resources_set))
}

pub fn bundle_project(project_path: &Path, hmr: bool) -> Result<BundleResult> {
    // 1. Load configuration
    let config = AngularConfig::load(project_path)?;
    let (_name, project) = config
        .projects
        .iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No project found"))?;

    let build_options = project
        .architect
        .as_ref()
        .and_then(|a| a.get("build"))
        .and_then(|t| t.options.as_ref());

    let root_dir = project_path.parent().unwrap_or_else(|| Path::new("."));

    // Create BundleConfig from angular.json settings
    let bundle_config = BundleConfig {
        source_root: project
            .source_root
            .clone()
            .unwrap_or_else(|| "src".to_string()),
        output_dir: build_options
            .and_then(|o| o.output_path.clone())
            .unwrap_or_else(|| "dist".to_string()),
    };

    // Collector for all unique external import specifiers for the Vite cache
    let mut external_import_collector: HashSet<String> = HashSet::new();

    // 2. Resolve Entry Point from angular.json
    let main_file = build_options
        .and_then(|o| o.main.as_ref())
        .map(|m| root_dir.join(m))
        .unwrap_or_else(|| root_dir.join("src/main.ts"));

    if !main_file.exists() {
        return Err(anyhow::anyhow!("Entry file not found: {:?}", main_file));
    }

    // Determine bundle name from main_file (entry point)
    let bundle_name = main_file
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.replace(".ts", ".js"))
        .unwrap_or_else(|| "main.js".to_string());

    // eprintln!("Building from entry: {:?}", main_file);

    // 3. Build import graph for MAIN bundle (no ignored files initially)
    let (static_files, dynamic_files, resource_files) =
        build_import_graph(&main_file, root_dir, None)?;

    // eprintln!(
    //     "Main Bundle: Static files: {}, Dynamic (lazy) entry points: {}",
    //     static_files.len(),
    //     dynamic_files.len()
    // );

    // 4. Compile static files (Sources only) and read Libs
    let mut source_files = Vec::new();
    let mut lib_files = Vec::new();

    for path in &static_files {
        if path.to_string_lossy().contains("node_modules") {
            lib_files.push(path.clone());
        } else {
            source_files.push(path.clone());
        }
    }

    // Sort source files topologically
    let sorted_source_files = sort_files_topologically(&source_files, root_dir)?;

    // Compile source files
    let raw_compiled = parallel_compile(&source_files, project_path, hmr)?;
    let mut compiled_map: HashMap<PathBuf, String> = raw_compiled
        .into_iter()
        .map(|(p, c)| (normalize_path(&p), c))
        .collect();

    // Create ordered compiled contents

    // Create ordered compiled contents
    let mut compiled_contents: Vec<(std::path::PathBuf, String)> = Vec::new();

    for path in &sorted_source_files {
        // Construct lookup key: dist/{relative_source_path_with_js_ext}
        let relative = path.strip_prefix(root_dir).unwrap_or(path);
        let dist_key = Path::new("dist").join(relative.with_extension("js"));

        if let Some(content) = compiled_map.remove(&dist_key) {
            compiled_contents.push((path.clone(), content));
        }
    }

    // 5. Build chunks for dynamic imports (Calculate first to allow import rewriting)
    let mut chunks = HashMap::new();
    let mut chunk_names = HashMap::new(); // Map<HashedFilename, OriginalName>
    let mut lazy_map = HashMap::new(); // Map<AbsoluteFsPath, HashedFilename>
    let mut files_map = HashMap::new();
    let mut raw_files_map: HashMap<String, String> = HashMap::new();
    let mut module_to_chunk = HashMap::new();

    use xxhash_rust::xxh3::xxh3_64;

    for dynamic_entry in &dynamic_files {
        if !dynamic_entry.exists() {
            continue;
        }
        let chunk_name = dynamic_entry
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("chunk");

        // Pass main bundle files as ignored files so we don't duplicate them in chunks
        let (chunk_static_files, _, _) =
            build_import_graph(dynamic_entry, root_dir, Some(&static_files))?;
        let chunk_files_vec: Vec<PathBuf> = chunk_static_files.into_iter().collect();

        if chunk_files_vec.is_empty() {
            continue;
        }

        let raw_chunk_compiled = parallel_compile(&chunk_files_vec, project_path, hmr)?;
        let chunk_compiled: Vec<(PathBuf, String)> = raw_chunk_compiled
            .into_iter()
            .map(|(p, c)| (normalize_path(&p), c))
            .collect();
        let mut chunk_content = String::new();

        // Header for chunk
        chunk_content.push_str(&format!("// Quantum Chunk: {}\n", chunk_name));

        // We need a way to register this chunk in the runtime if we were fully implementing Angular's runtime.
        // For now, we concat content but we need to strip internal imports.

        // Build a set of SOURCE file paths for internal import stripping
        // IMPORTANT: Only include source files, NOT node_modules - external deps should not be stripped
        let chunk_file_set: HashSet<PathBuf> = chunk_files_vec
            .iter()
            .filter(|p| !p.to_string_lossy().contains("node_modules"))
            .map(|p| normalize_path(p))
            .collect();

        // Determine entry file path for comparison (convert source path to compiled path)
        // dynamic_entry: src/app/.../module.ts -> dist/src/app/.../module.js
        let entry_compiled_path = {
            let entry_rel = dynamic_entry
                .strip_prefix(root_dir)
                .unwrap_or(dynamic_entry);
            let entry_dist = Path::new(&bundle_config.output_dir).join(entry_rel);
            normalize_path(&entry_dist.with_extension("js"))
        };

        // Collect named imports ONLY from entry file (like ngtsc output)
        // Other files keep their imports inline
        let mut hoisted_named_imports: Vec<String> = Vec::new();
        let mut code_sections: Vec<(String, String)> = Vec::new(); // (source_path, processed_content)

        let mut chunk_registry: HashMap<(String, String), String> = HashMap::new();
        let mut chunk_counters: HashMap<String, usize> = HashMap::new();

        for (path, content) in &chunk_compiled {
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

            if extension == "js" {
                // Use preserve_exports for chunks so lazy-loaded modules export their symbols
                let processed_content = process_bundle_file_preserve_exports(
                    content,
                    path,
                    root_dir,
                    &chunk_file_set,
                    &mut chunk_registry,
                    &mut chunk_counters,
                    &bundle_config,
                )
                .unwrap_or_else(|_| content.clone());

                // Format path as source path (like ngtsc output): src/app/... instead of dist/src/app/...
                let relative_path_str = path
                    .strip_prefix(root_dir)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                let source_path = relative_path_str
                    .strip_prefix(&format!("{}/", bundle_config.output_dir))
                    .unwrap_or(&relative_path_str)
                    .replace(".js", ".ts");

                // Check if this is the entry file - only hoist imports from entry file
                let is_entry_file = normalize_path(path) == entry_compiled_path;

                if is_entry_file {
                    // Separate named imports from code for entry file only
                    let mut file_named_imports = Vec::new();
                    let mut code_lines = Vec::new();

                    for line in processed_content.lines() {
                        let trimmed = line.trim();
                        // Named import pattern: starts with `import {` or `import{`, contains `}`, contains `from`
                        let is_named_import = (trimmed.starts_with("import {")
                            || trimmed.starts_with("import{"))
                            && trimmed.contains('}')
                            && trimmed.contains(" from ");

                        if is_named_import {
                            file_named_imports.push(line.to_string());
                        } else {
                            code_lines.push(line.to_string());
                        }
                    }

                    // Add entry's named imports to hoisted list
                    hoisted_named_imports.extend(file_named_imports);

                    // Add code section without named imports
                    let code_without_imports = code_lines.join("\n");
                    code_sections.push((source_path, code_without_imports));
                } else {
                    // Non-entry files: keep everything as-is
                    code_sections.push((source_path, processed_content));
                }
            }

            let relative_path_str = path
                .strip_prefix(root_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            // parallel_compile already outputs to 'dist/' via out_dir setting
            files_map.insert(relative_path_str, content.clone());
        }

        // Add chunk imports to collector
        for (specifier, _) in chunk_registry.keys() {
            external_import_collector.insert(specifier.clone());
        }

        // Build final chunk content: header + entry file comment + hoisted imports (from entry) + code sections
        // Get entry file source path for comment
        let entry_source_path = dynamic_entry
            .strip_prefix(root_dir)
            .unwrap_or(dynamic_entry)
            .to_string_lossy()
            .to_string();

        // Add entry file comment after header
        chunk_content.push_str(&format!("// {}\n", entry_source_path));

        // Deduplicate and add named imports (hoisted from entry)
        let mut seen_imports: HashSet<String> = HashSet::new();
        for import_line in &hoisted_named_imports {
            if seen_imports.insert(import_line.clone()) {
                chunk_content.push_str(import_line);
                chunk_content.push_str("\n");
            }
        }

        // Add blank line after imports if there are any
        if !hoisted_named_imports.is_empty() {
            chunk_content.push_str("\n");
        }

        // Add code sections with their file comments
        for (source_path, code) in &code_sections {
            chunk_content.push_str(&format!("// {}\n", source_path));
            chunk_content.push_str(code);
            chunk_content.push_str("\n\n");
        }

        // Hashing logic
        let hash = format!("{:016x}", xxh3_64(chunk_content.as_bytes()));
        let short_hash = &hash[0..8].to_uppercase();

        let hashed_name = format!("chunk-{}.js", short_hash);
        chunks.insert(hashed_name.clone(), chunk_content);

        // Store the full relative source path for import resolution
        let source_path = dynamic_entry
            .strip_prefix(root_dir)
            .unwrap_or(dynamic_entry)
            .to_string_lossy()
            .to_string();
        chunk_names.insert(hashed_name.clone(), source_path);

        lazy_map.insert(dynamic_entry.clone(), hashed_name.clone());

        // Map all source files in this chunk to the hashed name
        for file_path in &chunk_files_vec {
            let rel_path = file_path
                .strip_prefix(root_dir)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();
            module_to_chunk.insert(rel_path, hashed_name.clone());
        }
    }

    // Populate raw_files_map BEFORE stripping (for dev mode)
    for (path, content) in &compiled_contents {
        let relative_path = path.strip_prefix(root_dir).unwrap_or(path);
        // Convert source path to dist key: src/main.ts -> dist/src/main.js
        let dist_key = format!(
            "dist/{}",
            relative_path.with_extension("js").to_string_lossy()
        );
        raw_files_map.insert(dist_key, content.clone());
    }

    // 6. Build main bundle and files map
    let mut parts = Vec::new();
    let mut main_registry: HashMap<(String, String), String> = HashMap::new();
    let mut main_counters: HashMap<String, usize> = HashMap::new();

    let import_regex = regex::Regex::new(r#"(from\s+['"])([\.\/][^'"]+)(['"])"#).unwrap();
    let dynamic_import_regex =
        regex::Regex::new(r#"(import\s*\(\s*['"])([^'"]+)(['"]\s*\))"#).unwrap();

    for (path, content) in &compiled_contents {
        // Strip internal imports and rewrite external ones with unique aliases
        let internal_files_set: HashSet<PathBuf> = source_files.iter().cloned().collect();
        let stripped_content = process_bundle_file(
            content,
            path,
            root_dir,
            &internal_files_set,
            &mut main_registry,
            &mut main_counters,
            &bundle_config,
        )
        .unwrap_or_else(|_| content.clone());
        let content = &stripped_content;

        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let relative_file_path = path.strip_prefix(root_dir).unwrap_or(path);
        let relative_path_str = relative_file_path.to_string_lossy().to_string();

        // parallel_compile already outputs to 'dist/' via out_dir setting for sources
        // For libs, we just use relative path in map (simulating they are available)
        files_map.insert(relative_path_str.clone(), content.clone());

        if extension == "js" || extension == "mjs" || extension == "ts" {
            let file_dir = relative_file_path.parent().unwrap_or(Path::new("."));

            // compiled files are in 'dist' (if dist path) or 'src' (if source path).
            let source_dir = file_dir.strip_prefix("dist").unwrap_or(file_dir);

            // Rewrite static imports (Side-effects only)
            let mut rewritten_content = import_regex
                .replace_all(content, |caps: &regex::Captures| {
                    let prefix = &caps[1];
                    let import_path = &caps[2];
                    let suffix = &caps[3];

                    if import_path.starts_with('.') {
                        let joined = source_dir.join(import_path);
                        let mut new_path = joined.to_string_lossy().to_string();
                        if !new_path.starts_with('.') && !new_path.starts_with('/') {
                            new_path = format!("./{}", new_path);
                        }
                        format!("{}{}{}", prefix, new_path, suffix)
                    } else {
                        caps[0].to_string()
                    }
                })
                .to_string();

            // Rewrite dynamic imports (Lazy Loading)
            rewritten_content = dynamic_import_regex
                .replace_all(&rewritten_content, |caps: &regex::Captures| {
                    let prefix = &caps[1];
                    let import_path = &caps[2];
                    let suffix = &caps[3];

                    if import_path.starts_with('.') {
                        let joined = source_dir.join(import_path);
                        let abs_target = root_dir.join(&joined);

                        if let Some(hashed) = lazy_map.get(&abs_target) {
                            return format!("{}./{}{}", prefix, hashed, suffix);
                        }

                        let mut path_str = abs_target.to_string_lossy().to_string();
                        path_str.push_str(".ts");
                        let with_ts = PathBuf::from(path_str);

                        if let Some(hashed) = lazy_map.get(&with_ts) {
                            return format!("{}./{}{}", prefix, hashed, suffix);
                        }

                        if abs_target.exists() {
                            if let Ok(canon) = abs_target.canonicalize() {
                                if let Some(hashed) = lazy_map.get(&canon) {
                                    return format!("{}./{}{}", prefix, hashed, suffix);
                                }
                            }
                        }

                        let mut norm_path_str = abs_target.to_string_lossy().to_string();
                        norm_path_str.push_str(".ts");
                        let norm_ts = PathBuf::from(norm_path_str);

                        if let Some(hashed) = lazy_map.get(&norm_ts) {
                            return format!("{}./{}{}", prefix, hashed, suffix);
                        }

                        caps[0].to_string()
                    } else {
                        caps[0].to_string()
                    }
                })
                .to_string();

            // Rewrite templateUrl and styleUrl
            let template_regex =
                regex::Regex::new(r#"(templateUrl\s*:\s*['"])([^'"]+)(['"])"#).unwrap();
            let style_url_regex =
                regex::Regex::new(r#"(styleUrl\s*:\s*['"])([^'"]+)(['"])"#).unwrap();
            let style_urls_regex =
                regex::Regex::new(r#"(styleUrls\s*:\s*\[\s*['"])([^'"]+)(['"]\s*\])"#).unwrap();

            rewritten_content = template_regex
                .replace_all(&rewritten_content, |caps: &regex::Captures| {
                    let prefix = &caps[1];
                    let path = &caps[2];
                    let suffix = &caps[3];
                    if path.starts_with('.') {
                        let joined = file_dir.join(path);
                        let normalized = normalize_path(&joined);
                        let new_path = normalized.to_string_lossy().to_string();
                        format!("{}{}{}", prefix, new_path, suffix)
                    } else {
                        caps[0].to_string()
                    }
                })
                .to_string();

            rewritten_content = style_url_regex
                .replace_all(&rewritten_content, |caps: &regex::Captures| {
                    let prefix = &caps[1];
                    let path = &caps[2];
                    let suffix = &caps[3];
                    if path.starts_with('.') {
                        let joined = file_dir.join(path);
                        let normalized = normalize_path(&joined);
                        let new_path = normalized.to_string_lossy().to_string();
                        format!("{}{}{}", prefix, new_path, suffix)
                    } else {
                        caps[0].to_string()
                    }
                })
                .to_string();

            rewritten_content = style_urls_regex
                .replace_all(&rewritten_content, |caps: &regex::Captures| {
                    let prefix = &caps[1];
                    let path = &caps[2];
                    let suffix = &caps[3];
                    if path.starts_with('.') {
                        let joined = file_dir.join(path);
                        let normalized = normalize_path(&joined);
                        let new_path = normalized.to_string_lossy().to_string();
                        format!("{}{}{}", prefix, new_path, suffix)
                    } else {
                        caps[0].to_string()
                    }
                })
                .to_string();

            // Format path as source path (like ngtsc output): src/app/... instead of dist/src/app/...
            let source_path = relative_path_str
                .strip_prefix(&format!("{}/", bundle_config.output_dir))
                .unwrap_or(&relative_path_str)
                .replace(".js", ".ts");
            parts.push(format!("// {}\n{}\n", source_path, rewritten_content));
        }
    }

    // Bundle content - no polyfills here, they go to separate polyfills.js
    let mut bundle_js = String::new();
    for part in parts {
        bundle_js.push_str(&part);
    }

    // Map main bundle static files to main bundle name
    for file_path in &static_files {
        let rel_path = file_path
            .strip_prefix(root_dir)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();
        module_to_chunk.insert(rel_path, bundle_name.clone());
    }

    // 6.5. Process Polyfills from angular.json config
    let mut polyfills_js = None;
    if let Some(options) = build_options {
        if let Some(polyfills) = &options.polyfills {
            if !polyfills.is_empty() {
                let mut polyfills_content = String::new();
                polyfills_content.push_str("// Polyfills bundle - generated from angular.json\n");
                for polyfill in polyfills {
                    polyfills_content.push_str(&format!("import '{}';\n", polyfill));
                }
                polyfills_js = Some(polyfills_content);
            }
        }
    }

    // 7. Process Styles
    let mut styles_css = None;
    if let Some(options) = build_options {
        if let Some(styles) = &options.styles {
            let mut combined_css = String::new();
            for style in styles {
                let path = root_dir.join(style);
                if path.exists() {
                    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    let content = if extension == "scss" || extension == "sass" {
                        let mut options = grass::Options::default();
                        options = options.load_path(root_dir);
                        options = options.load_path(root_dir.join("node_modules"));

                        if let Some(parent) = root_dir.parent() {
                            options = options.load_path(parent.join("node_modules"));
                        }

                        match grass::from_path(&path, &options) {
                            Ok(css) => css,
                            Err(e) => {
                                eprintln!("[Bundler] SCSS compilation failed for {}: {}", style, e);
                                // Fallback: try to compile without imports if possible or just return raw
                                std::fs::read_to_string(&path)?
                            }
                        }
                    } else {
                        let raw_content = std::fs::read_to_string(&path)?;
                        if extension == "css" {
                            inline_css_imports(&raw_content, root_dir, &path)
                        } else {
                            raw_content
                        }
                    };

                    files_map.insert(style.clone(), content.clone());
                    combined_css.push_str(&format!("/* {} */\n", style));
                    combined_css.push_str(&content);
                    combined_css.push_str("\n");
                }
            }
            if !combined_css.is_empty() {
                styles_css = Some(combined_css);
            }
        }
    }

    // 8. Process Scripts
    let mut scripts_js = None;
    if let Some(options) = build_options {
        if let Some(scripts) = &options.scripts {
            let mut combined_js = String::new();
            for script in scripts {
                let path = root_dir.join(script);
                if path.exists() {
                    let content = std::fs::read_to_string(&path)?;
                    files_map.insert(script.clone(), content.clone());
                    combined_js.push_str(&format!("// {} \n", script));
                    combined_js.push_str(&content);
                    combined_js.push_str("\n");
                }
            }
            if !combined_js.is_empty() {
                scripts_js = Some(combined_js);
            }
        }
    }

    // 9. Process Index HTML
    let mut index_html = None;
    if let Some(options) = build_options {
        if let Some(index) = &options.index {
            let src_path = root_dir.join(index);
            if src_path.exists() {
                let mut content = std::fs::read_to_string(&src_path)?;

                if styles_css.is_some() {
                    let link_tag = r#"<link rel="stylesheet" href="styles.css">"#;
                    if let Some(pos) = content.find("</head>") {
                        content.insert_str(pos, &format!("{}\n", link_tag));
                    } else {
                        content.push_str(&format!("\n{}", link_tag));
                    }
                }

                // Polyfills must be loaded BEFORE main bundle (sync, not module)
                if polyfills_js.is_some() {
                    let script_tag = r#"<script src="polyfills.js" type="module"></script>"#;
                    if let Some(pos) = content.find("</body>") {
                        content.insert_str(pos, &format!("{}\n", script_tag));
                    } else {
                        content.push_str(&format!("\n{}", script_tag));
                    }
                }

                // Main bundle
                let script_tag =
                    format!(r#"<script src="{}" type="module"></script>"#, bundle_name);
                if let Some(pos) = content.find("</body>") {
                    content.insert_str(pos, &format!("{}\n", script_tag));
                } else {
                    content.push_str(&format!("\n{}", script_tag));
                }

                if scripts_js.is_some() {
                    let script_tag = r#"<script src="scripts.js" defer></script>"#;
                    if let Some(pos) = content.find("</body>") {
                        content.insert_str(pos, &format!("{}\n", script_tag));
                    } else {
                        content.push_str(&format!("\n{}", script_tag));
                    }
                }
                index_html = Some(content);
            }
        }
    }

    // Add main registry imports to collector
    for (specifier, _) in main_registry.keys() {
        external_import_collector.insert(specifier.clone());
    }

    let external_imports: Vec<String> = external_import_collector.into_iter().collect();

    Ok(BundleResult {
        bundle_js,
        bundle_name,
        styles_css,
        scripts_js,
        polyfills_js,
        index_html,
        files: files_map,
        raw_files: raw_files_map,
        chunks,
        chunk_names,
        module_to_chunk,
        external_imports,
    })
}

fn inline_css_imports(
    content: &str,
    root_dir: &std::path::Path,
    current_file: &std::path::Path,
) -> String {
    let import_re = regex::Regex::new(r#"@import\s+['"]([^'"]+)['"];"#).unwrap();
    let url_re =
        regex::Regex::new(r#"url\s*\(\s*(?:'([^']*)'|"([^"]*)"|([^'"\s)]+))\s*\)"#).unwrap();

    let mut processed_lines = Vec::new();

    // Helper to determine if we are in node_modules and construct virtual path
    let rewrite_url = |url: &str| -> Option<String> {
        let path_str = current_file.to_string_lossy();
        // Check if current file is in node_modules
        if path_str.contains("node_modules")
            && !url.starts_with("data:")
            && !url.starts_with("http")
            && !url.starts_with("/")
        {
            // Extract package path from node_modules
            // e.g. .../node_modules/pkg/foo.css -> pkg/foo.css
            // We want to construct /__node_modules/pkg/relative_url

            // Find the last node_modules segment
            // We use rsplit_once to get the path relative to the last node_modules/
            if let Some((_, relative_pkg_path)) = path_str.rsplit_once("node_modules/") {
                // relative_pkg_path is like "primeicons/primeicons.css"
                if let Some(parent_dir) = std::path::Path::new(relative_pkg_path).parent() {
                    // parent_dir is like "primeicons" (if primeicons.css is at root of package)
                    // OR "primeicons/css" (if nested)

                    let asset_path = parent_dir.join(url);
                    // normalize path to use forward slashes
                    let asset_path_str = asset_path.to_string_lossy().replace("\\", "/");
                    return Some(format!("/__node_modules/{}", asset_path_str));
                }
            }
        }
        None
    };

    for line in content.lines() {
        if let Some(caps) = import_re.captures(line) {
            let import_path = &caps[1];

            // Try resolving via Node.js exports first
            let mut resolved_path = None;
            if let Some(exports_resolved) = resolve_package_exports(import_path, root_dir) {
                resolved_path = Some(exports_resolved);
            } else {
                // Fallback to manual resolution
                let mut path = root_dir.join("node_modules").join(import_path);

                // 1. Try exact path (only if it's a file)
                if !path.is_file() {
                    // 2. Try appending .css
                    let with_css = path.with_extension("css");
                    if with_css.is_file() {
                        path = with_css;
                    } else if !path.exists() || path.is_dir() {
                        // Try parent node_modules (monorepo support)
                        if let Some(parent) = root_dir.parent() {
                            let parent_node_modules = parent.join("node_modules").join(import_path);
                            if parent_node_modules.is_file() {
                                path = parent_node_modules;
                            } else {
                                let parent_with_css = parent_node_modules.with_extension("css");
                                if parent_with_css.is_file() {
                                    path = parent_with_css;
                                }
                            }
                        }
                    }
                }

                if path.is_file() {
                    resolved_path = Some(path);
                }
            }

            if let Some(path) = resolved_path {
                if let Ok(imported_content) = std::fs::read_to_string(&path) {
                    processed_lines.push(format!("/* Inlined: {} */", import_path));
                    // Recursive inline with new file context
                    let inlined_nested = inline_css_imports(&imported_content, root_dir, &path);
                    processed_lines.push(inlined_nested);
                    continue;
                }
            }
        }

        // Rewrite URLs
        let mut new_line = line.to_string();
        if url_re.is_match(line) {
            new_line = url_re
                .replace_all(line, |caps: &regex::Captures| {
                    // Group 1: single quoted, Group 2: double quoted, Group 3: unquoted
                    let url = caps
                        .get(1)
                        .or(caps.get(2))
                        .or(caps.get(3))
                        .map(|m| m.as_str())
                        .unwrap_or("");

                    if let Some(rewritten) = rewrite_url(url) {
                        format!("url(\"{}\")", rewritten)
                    } else {
                        caps[0].to_string()
                    }
                })
                .to_string();
        }

        processed_lines.push(new_line);
    }

    processed_lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn create_test_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_resolve_import_node_modules() {
        let temp_dir = std::env::temp_dir().join("ng_bundler_test_resolve");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let root_dir = temp_dir.clone();
        let src_dir = root_dir.join("src");

        // Setup node_modules
        create_test_file(
            &root_dir.join("node_modules/pkg-a/package.json"),
            r#"{"main": "index.js"}"#,
        );
        create_test_file(
            &root_dir.join("node_modules/pkg-a/index.js"),
            "export const a = 1;",
        );

        create_test_file(
            &root_dir.join("node_modules/pkg-b/index.ts"),
            "export const b = 2;",
        );

        // Test resolution
        let resolved_a = resolve_import("pkg-a", &src_dir, &root_dir);
        assert!(resolved_a.is_some());
        assert_eq!(
            resolved_a.unwrap(),
            root_dir.join("node_modules/pkg-a/index.js")
        );

        let resolved_b = resolve_import("pkg-b", &src_dir, &root_dir);
        assert!(resolved_b.is_some());
        assert_eq!(
            resolved_b.unwrap(),
            root_dir.join("node_modules/pkg-b/index.ts")
        );

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_build_import_graph_lazy_split() {
        let temp_dir = std::env::temp_dir().join("ng_bundler_test_graph");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let root_dir = temp_dir.clone();

        // File structure:
        // main.ts -> imports shared.ts
        //         -> dynamic imports lazy.ts
        // lazy.ts -> imports shared.ts
        //         -> imports unique.ts

        create_test_file(
            &root_dir.join("main.ts"),
            r#"
            import { shared } from './shared';
            const lazy = import('./lazy');
        "#,
        );
        create_test_file(
            &root_dir.join("shared.ts"),
            "export const shared = 'shared';",
        );
        create_test_file(
            &root_dir.join("lazy.ts"),
            r#"
            import { shared } from './shared';
            import { unique } from './unique';
        "#,
        );
        create_test_file(
            &root_dir.join("unique.ts"),
            "export const unique = 'unique';",
        );

        // 1. Build Main Graph
        let (main_static, main_dynamic, _) =
            build_import_graph(&root_dir.join("main.ts"), &root_dir, None).unwrap();

        assert!(main_static.contains(&root_dir.join("main.ts")));
        assert!(main_static.contains(&root_dir.join("shared.ts")));
        assert!(!main_static.contains(&root_dir.join("lazy.ts")));

        assert!(main_dynamic.contains(&root_dir.join("lazy.ts"))); // resolved path

        // 2. Build Lazy Graph (simulating chunk generation)
        let lazy_entry = main_dynamic.iter().next().unwrap();

        // Pass main_static as ignored
        // We need to resolve path for shared.ts and main.ts to match those in ignored set
        // main_static contains absolute paths.

        let (lazy_static, _, _) =
            build_import_graph(lazy_entry, &root_dir, Some(&main_static)).unwrap();

        assert!(lazy_static.contains(&root_dir.join("lazy.ts")));
        assert!(lazy_static.contains(&root_dir.join("unique.ts")));

        // Crucial: shared.ts should NOT be in lazy_static because it's in main_static (ignored)
        assert!(!lazy_static.contains(&root_dir.join("shared.ts")));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
