use crate::compile::parallel::parallel_compile;
use crate::config::angular::AngularConfig;
use anyhow::Result;
use oxc_allocator::Allocator;
use oxc_ast::ast::{Argument, Expression as OxcExpression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

pub struct BundleResult {
    pub bundle_js: String,
    pub styles_css: Option<String>,
    pub scripts_js: Option<String>,
    pub index_html: Option<String>,
    pub files: HashMap<String, String>,
    pub chunks: HashMap<String, String>,
}

/// Represents result of scanning a file for imports
struct ImportScanResult {
    static_imports: Vec<PathBuf>,
    dynamic_imports: Vec<PathBuf>,
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

    let file_dir = file_path.parent().unwrap_or(root_dir);

    for stmt in &ret.program.body {
        // Static imports: import ... from '...'
        if let Statement::ImportDeclaration(decl) = stmt {
            let specifier = decl.source.value.as_str();

            if let Some(resolved) = resolve_import(specifier, file_dir, root_dir) {
                static_imports.push(resolved);
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

/// Resolve import specifier to absolute path
fn resolve_import(specifier: &str, file_dir: &Path, root_dir: &Path) -> Option<PathBuf> {
    // Only handle relative imports (starting with . or ..)
    if !specifier.starts_with('.') {
        eprintln!("ANTIGRAVITY_DEBUG: Ignored external import: {}", specifier);
        return None; // External/node_modules import
    }

    let joined = file_dir.join(specifier);
    let normalized = normalize_path(&joined);

    // Try appending .ts (don't use with_extension as it replaces existing extension)
    let mut with_ts_name = normalized.file_name().unwrap_or_default().to_os_string();
    with_ts_name.push(".ts");
    let with_ts = normalized.with_file_name(with_ts_name);

    if with_ts.exists() {
        return Some(with_ts);
    }

    // Try with /index.ts
    let index_ts = normalized.join("index.ts");
    if index_ts.exists() {
        return Some(index_ts);
    }

    // Try exact path
    if normalized.exists() {
        return Some(normalized);
    }

    None
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
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
) -> Result<(HashSet<PathBuf>, HashSet<PathBuf>)> {
    let mut static_set = HashSet::new();
    let mut dynamic_set = HashSet::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

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
    }

    Ok((static_set, dynamic_set))
}

pub fn bundle_project(project_path: &Path) -> Result<BundleResult> {
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

    // 2. Resolve Entry Point from angular.json
    let main_file = build_options
        .and_then(|o| o.main.as_ref())
        .map(|m| root_dir.join(m))
        .unwrap_or_else(|| root_dir.join("src/main.ts"));

    if !main_file.exists() {
        return Err(anyhow::anyhow!("Entry file not found: {:?}", main_file));
    }

    eprintln!("Building from entry: {:?}", main_file);

    // 3. Build import graph
    let (static_files, dynamic_files) = build_import_graph(&main_file, root_dir)?;

    eprintln!(
        "Static files: {}, Dynamic (lazy) files: {}",
        static_files.len(),
        dynamic_files.len()
    );

    // 4. Compile static files
    let static_files_vec: Vec<PathBuf> = static_files.into_iter().collect();
    let compiled_contents = parallel_compile(&static_files_vec, project_path)?;

    // 5. Build main bundle and files map
    let mut bundle_js = String::new();
    let mut files_map = HashMap::new();

    bundle_js.push_str("import 'zone.js';\n");

    let import_regex = regex::Regex::new(r#"(from\s+['"])([\.\/][^'"]+)(['"])"#).unwrap();

    for (path, content) in &compiled_contents {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let relative_file_path = path.strip_prefix(root_dir).unwrap_or(path);
        let relative_path_str = relative_file_path.to_string_lossy().to_string();

        // parallel_compile already outputs to 'dist/' via out_dir setting
        files_map.insert(relative_path_str.clone(), content.clone());

        if extension == "js" {
            let file_dir = relative_file_path.parent().unwrap_or(Path::new("."));
            let rewritten_content = import_regex.replace_all(content, |caps: &regex::Captures| {
                let prefix = &caps[1];
                let import_path = &caps[2];
                let suffix = &caps[3];

                if import_path.starts_with('.') {
                    let joined = file_dir.join(import_path);
                    let mut new_path = joined.to_string_lossy().to_string();
                    if !new_path.starts_with('.') && !new_path.starts_with('/') {
                        new_path = format!("./{}", new_path);
                    }
                    format!("{}{}{}", prefix, new_path, suffix)
                } else {
                    caps[0].to_string()
                }
            });

            bundle_js.push_str(&format!("// File: {}\n", path.display()));
            bundle_js.push_str(&rewritten_content);
            bundle_js.push_str("\n");
        }
    }

    // 6. Build chunks for dynamic imports
    let mut chunks = HashMap::new();
    for dynamic_entry in &dynamic_files {
        if !dynamic_entry.exists() {
            continue;
        }
        let chunk_name = dynamic_entry
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("chunk");

        let (chunk_static_files, _) = build_import_graph(dynamic_entry, root_dir)?;
        let chunk_files_vec: Vec<PathBuf> = chunk_static_files.into_iter().collect();

        if chunk_files_vec.is_empty() {
            continue;
        }

        let chunk_compiled = parallel_compile(&chunk_files_vec, project_path)?;
        let mut chunk_content = String::new();

        for (path, content) in &chunk_compiled {
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if extension == "js" {
                chunk_content.push_str(&format!("// File: {}\n", path.display()));
                chunk_content.push_str(content);
                chunk_content.push_str("\n");
            }

            let relative_path_str = path
                .strip_prefix(root_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            // parallel_compile already outputs to 'dist/' via out_dir setting
            files_map.insert(relative_path_str, content.clone());
        }

        chunks.insert(format!("chunk-{}.js", chunk_name), chunk_content);
    }

    // 7. Process Styles
    let mut styles_css = None;
    if let Some(options) = build_options {
        if let Some(styles) = &options.styles {
            let mut combined_css = String::new();
            for style in styles {
                let path = root_dir.join(style);
                if path.exists() {
                    let content = std::fs::read_to_string(&path)?;
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

                let script_tag = r#"<script src="bundle.js" type="module"></script>"#;
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

    Ok(BundleResult {
        bundle_js,
        styles_css,
        scripts_js,
        index_html,
        files: files_map,
        chunks,
    })
}
