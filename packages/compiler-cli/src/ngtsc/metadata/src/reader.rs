use super::util::extract_directive_metadata;
use crate::ngtsc::metadata::DecoratorMetadata;
use crate::ngtsc::reflection::{self, ClassDeclaration, Decorator};
use angular_compiler::output::output_ast::{Expression, ExternalExpr, ExternalReference};
use angular_compiler::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};
use angular_compiler::render3::view::api::{
    R3DirectiveDependencyMetadata, R3PipeDependencyMetadata, R3TemplateDependencyKind,
    R3TemplateDependencyMetadata,
};
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    CallExpression, Declaration, Expression as OxcExpression, ObjectPropertyKind, PropertyKey,
    Statement,
};
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct ModuleMetadataReader {
    node_modules_path: PathBuf,
    project_root: PathBuf,
}

impl ModuleMetadataReader {
    pub fn new(project_root: &Path) -> Self {
        Self {
            node_modules_path: project_root.join("node_modules"),
            project_root: project_root.to_path_buf(),
        }
    }

    pub fn resolve_module(&self, module_name: &str) -> Option<PathBuf> {
        let path = PathBuf::from(module_name);
        if path.is_absolute() && path.exists() {
            return Some(path);
        }

        // Heuristic for Angular Material and similar packages
        // 1. Check if it's a scoped package with secondary entry point
        // e.g. @angular/material/checkbox
        if module_name.starts_with("@") {
            // eprintln!(
            //     "DEBUG: [fesm_reader] Resolving scoped module: {}",
            //     module_name
            // );
            let parts: Vec<&str> = module_name.split('/').collect();
            if parts.len() >= 3 {
                // @scope/pkg/entry -> @scope/pkg
                let scope = parts[0];
                let pkg = parts[1];
                let entry = parts[2..].join("/");

                // Try FESM2022 first (Angular 13+)
                let fesm_path = self
                    .node_modules_path
                    .join(scope)
                    .join(pkg)
                    .join("fesm2022")
                    .join(format!("{}.mjs", entry));
                if fesm_path.exists() {
                    eprintln!(
                        "DEBUG: [fesm_reader] Found FESM2022: {}",
                        fesm_path.display()
                    );
                    return Some(fesm_path);
                }

                // Try FESM2020
                let fesm_2020_path = self
                    .node_modules_path
                    .join(scope)
                    .join(pkg)
                    .join("fesm2020")
                    .join(format!("{}.mjs", entry));

                eprintln!(
                    "DEBUG: [fesm_reader] Checking path for {}: {}",
                    module_name,
                    fesm_2020_path.display()
                );

                if fesm_2020_path.exists() {
                    eprintln!(
                        "DEBUG: [fesm_reader] Found FESM2020: {}",
                        fesm_2020_path.display()
                    );
                    return Some(fesm_2020_path);
                }
            } else if parts.len() == 2 {
                // @scope/pkg -> @scope/pkg/fesm2022/pkg.mjs
                let scope = parts[0];
                let pkg = parts[1];

                // Try FESM2022 first
                let fesm_path = self
                    .node_modules_path
                    .join(scope)
                    .join(pkg)
                    .join("fesm2022")
                    .join(format!("{}.mjs", pkg));

                if fesm_path.exists() {
                    // eprintln!(
                    //     "DEBUG: [fesm_reader] Found FESM2022 for {}: {}",
                    //     module_name,
                    //     fesm_path.display()
                    // );
                    return Some(fesm_path);
                }

                // Try FESM2020
                let fesm_2020_path = self
                    .node_modules_path
                    .join(scope)
                    .join(pkg)
                    .join("fesm2020")
                    .join(format!("{}.mjs", pkg));

                if fesm_2020_path.exists() {
                    // eprintln!(
                    //     "DEBUG: [fesm_reader] Found FESM2020 for {}: {}",
                    //     module_name,
                    //     fesm_2020_path.display()
                    // );
                    return Some(fesm_2020_path);
                }
            }
        }

        // Fallback: Try traversing node_modules directly (simplified)
        let direct_path = self.node_modules_path.join(module_name);
        if direct_path.exists() {
            // Check package.json for module/fesm2022
        }

        None
    }

    pub fn read_metadata(&self, module_name: &str) -> Option<Vec<R3TemplateDependencyMetadata>> {
        let entry_path = if let Some(p) = self.resolve_module(module_name) {
            p
        } else {
            return None;
        };
        // ...

        let mut queue = vec![entry_path.clone()];
        let mut visited = std::collections::HashSet::new();
        visited.insert(entry_path.clone());

        // Map class name to metadata
        let mut all_definitions = HashMap::new();
        // Names of classes exported by NgModules
        let mut all_exported_classes = Vec::new();

        while let Some(path) = queue.pop() {
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let allocator = Allocator::default();
            let source_type = SourceType::from_path(&path).unwrap_or_default();
            let ret = Parser::new(&allocator, &content, source_type).parse();

            if !ret.errors.is_empty() {
                // eprintln!(
                //     "DEBUG: [fesm_reader] Parser errors for {}: {:?}",
                //     path.display(),
                //     ret.errors
                // );
                continue;
            }
            let program = ret.program;
            let current_dir = path.parent().unwrap_or(Path::new("."));

            for stmt in &program.body {
                // 1. Handle Re-exports to populate queue
                let source_opt = match stmt {
                    Statement::ExportAllDeclaration(e) => Some(&e.source),
                    Statement::ExportNamedDeclaration(e) => e.source.as_ref(),
                    _ => None,
                };

                if let Some(source) = source_opt {
                    let import_path = source.value.as_str();
                    if import_path.starts_with(".") {
                        let resolved_path = current_dir.join(import_path);
                        let p = if resolved_path.exists() {
                            resolved_path
                        } else if resolved_path.with_extension("mjs").exists() {
                            resolved_path.with_extension("mjs")
                        } else if resolved_path.with_extension("js").exists() {
                            resolved_path.with_extension("js")
                        } else {
                            resolved_path
                        };

                        if p.exists() && !visited.contains(&p) {
                            visited.insert(p.clone());
                            queue.push(p);
                        }
                    }
                }

                // 1b. Also handle ImportDeclarations with local relative paths (e.g., chunk files)
                if let Some(oxc_ast::ast::ModuleDeclaration::ImportDeclaration(import_decl)) =
                    stmt.as_module_declaration()
                {
                    let import_path = import_decl.source.value.as_str();
                    if import_path.starts_with(".") {
                        let resolved_path = current_dir.join(import_path);
                        let p = if resolved_path.exists() {
                            resolved_path
                        } else if resolved_path.with_extension("mjs").exists() {
                            resolved_path.with_extension("mjs")
                        } else if resolved_path.with_extension("js").exists() {
                            resolved_path.with_extension("js")
                        } else {
                            resolved_path
                        };

                        if p.exists() && !visited.contains(&p) {
                            visited.insert(p.clone());
                            queue.push(p);
                        }
                    }
                }

                // 2. Handle Class Declarations for Directives/Pipes/NgModules
                let class_decl_opt = match stmt {
                    Statement::ClassDeclaration(c) => Some(c),
                    Statement::ExportNamedDeclaration(e) => {
                        if let Some(Declaration::ClassDeclaration(c)) = &e.declaration {
                            Some(c)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(class_decl) = class_decl_opt {
                    if let Some(id) = &class_decl.id {
                        let class_name = id.name.as_str();

                        // Check static properties
                        for member in &class_decl.body.body {
                            if let oxc_ast::ast::ClassElement::PropertyDefinition(prop) = member {
                                if prop.r#static {
                                    if let Some(key_name) = prop.key.name() {
                                        if let Some(value) = &prop.value {
                                            if let OxcExpression::CallExpression(call) = value {
                                                let property_name_opt =
                                                    if let OxcExpression::StaticMemberExpression(
                                                        callee,
                                                    ) = &call.callee
                                                    {
                                                        Some(callee.property.name.as_str())
                                                    } else {
                                                        None
                                                    };

                                                if let Some(property_name) = property_name_opt {
                                                    match key_name.as_ref() {
                                                        "ɵmod"
                                                            if property_name
                                                                == "ɵɵdefineNgModule"
                                                                || property_name
                                                                    == "ɵɵngDeclareNgModule" =>
                                                        {
                                                            if let Some(args) =
                                                                call.arguments.first()
                                                            {
                                                                if let Some(expr) =
                                                                    args.as_expression()
                                                                {
                                                                    self.extract_exports(
                                                                        expr,
                                                                        &mut all_exported_classes,
                                                                    );
                                                                }
                                                            }
                                                        }
                                                        "ɵcmp" | "ɵdir" => {
                                                            if property_name == "ɵɵdefineComponent"
                                                                || property_name
                                                                    == "ɵɵdefineDirective"
                                                                || property_name
                                                                    == "ɵɵngDeclareComponent"
                                                                || property_name
                                                                    == "ɵɵngDeclareDirective"
                                                            {
                                                                if let Some(args) =
                                                                    call.arguments.first()
                                                                {
                                                                    if let Some(expr) =
                                                                        args.as_expression()
                                                                    {
                                                                        if let Some(meta) = self
                                                                            .parse_directive_meta(
                                                                                expr,
                                                                                key_name.as_ref()
                                                                                    == "ɵcmp",
                                                                            )
                                                                        {
                                                                            all_definitions.insert(class_name.to_string(), (meta.0, meta.1, meta.2, meta.3, meta.4, R3TemplateDependencyKind::Directive));
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        "ɵpipe"
                                                            if property_name == "ɵɵdefinePipe"
                                                                || property_name
                                                                    == "ɵɵngDeclarePipe" =>
                                                        {
                                                            if let Some(args) =
                                                                call.arguments.first()
                                                            {
                                                                if let Some(expr) =
                                                                    args.as_expression()
                                                                {
                                                                    if let Some(name) =
                                                                        self.parse_pipe_meta(expr)
                                                                    {
                                                                        all_definitions.insert(class_name.to_string(), (name, vec![], vec![], None, false, R3TemplateDependencyKind::Pipe));
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        _ => {}
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

        let mut results = Vec::new();

        let classes_to_export = if all_exported_classes.is_empty() {
            all_definitions.keys().cloned().collect::<Vec<_>>()
        } else {
            all_exported_classes
        };

        for export_name in classes_to_export {
            if let Some((selector_or_name, inputs, outputs, export_as, is_component, kind)) =
                all_definitions.get(&export_name)
            {
                let expression = Expression::External(ExternalExpr {
                    value: ExternalReference {
                        module_name: Some(module_name.to_string()),
                        name: Some(export_name.clone()),
                        runtime: None,
                    },
                    type_: None,
                    source_span: None,
                });

                let meta = match kind {
                    R3TemplateDependencyKind::Directive => {
                        // eprintln!("DEBUG: [metadata] Resolving directive export: {}", selector_or_name);
                        R3TemplateDependencyMetadata::Directive(R3DirectiveDependencyMetadata {
                            selector: selector_or_name.clone(),
                            type_: expression,
                            inputs: inputs.clone(),
                            outputs: outputs.clone(),
                            export_as: export_as
                                .as_ref()
                                .map(|s| {
                                    s.split(',')
                                        .map(|p| p.trim().to_string())
                                        .collect::<Vec<String>>()
                                })
                                .unwrap_or_default()
                                .into(),
                            kind: R3TemplateDependencyKind::Directive,
                            is_component: *is_component,
                            source_span: None,
                        })
                    }
                    R3TemplateDependencyKind::Pipe => R3TemplateDependencyMetadata::Pipe(
                        angular_compiler::render3::view::api::R3PipeDependencyMetadata {
                            name: selector_or_name.clone(),
                            type_: expression,
                            kind: R3TemplateDependencyKind::Pipe,
                            source_span: None,
                        },
                    ),
                    _ => continue,
                };

                results.push(meta);
            } else {
                // eprintln!("DEBUG: [metadata] Export listed but definition not found: {}", export_name);
            }
        }

        Some(results)
    }

    fn extract_exports(&self, expr: &OxcExpression, exports: &mut Vec<String>) {
        if let OxcExpression::ObjectExpression(obj) = expr {
            for prop in &obj.properties {
                if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                    if let Some(name) = p.key.name() {
                        if name == "exports" {
                            if let OxcExpression::ArrayExpression(arr) = &p.value {
                                for elem in &arr.elements {
                                    if let Some(e) = elem.as_expression() {
                                        if let OxcExpression::Identifier(id) = e {
                                            exports.push(id.name.to_string());
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

    fn parse_directive_meta(
        &self,
        expr: &OxcExpression,
        is_cmp: bool,
    ) -> Option<(String, Vec<String>, Vec<String>, Option<String>, bool)> {
        // Returns (selector, inputs, outputs, export_as, is_component)
        if let OxcExpression::ObjectExpression(obj) = expr {
            let mut selector = String::new();
            let mut inputs = Vec::new();
            let mut outputs = Vec::new();
            let mut export_as = None;

            for prop in &obj.properties {
                if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                    if let Some(key) = p.key.name() {
                        match key.as_ref() {
                            "selector" => {
                                if let OxcExpression::StringLiteral(s) = &p.value {
                                    selector = s.value.to_string();
                                }
                            }
                            "selectors" => {
                                if let OxcExpression::ArrayExpression(arr) = &p.value {
                                    // selectors: [['mat-checkbox'], ...]
                                    if let Some(first) = arr.elements.first() {
                                        if let Some(OxcExpression::ArrayExpression(inner)) =
                                            first.as_expression()
                                        {
                                            if let Some(sel) = inner.elements.first() {
                                                if let Some(OxcExpression::StringLiteral(s)) =
                                                    sel.as_expression()
                                                {
                                                    selector = s.value.to_string();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "exportAs" | "export_as" => {
                                if let OxcExpression::StringLiteral(s) = &p.value {
                                    export_as = Some(s.value.to_string());
                                } else if let OxcExpression::ArrayExpression(arr) = &p.value {
                                    let mut names = Vec::new();
                                    for elem in &arr.elements {
                                        if let Some(expr) = elem.as_expression() {
                                            if let OxcExpression::StringLiteral(s) = expr {
                                                names.push(s.value.to_string());
                                            }
                                        }
                                    }
                                    if !names.is_empty() {
                                        export_as = Some(names.join(","));
                                    }
                                }
                            }
                            "inputs" => {
                                // inputs: { color: "color", ... } or { color: ["alias", "color"] }
                                if let OxcExpression::ObjectExpression(in_obj) = &p.value {
                                    for in_prop in &in_obj.properties {
                                        if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(
                                            ip,
                                        ) = in_prop
                                        {
                                            // Parse value to get public name
                                            let mut public_name = None;
                                            if let OxcExpression::StringLiteral(s) = &ip.value {
                                                public_name = Some(s.value.to_string());
                                            } else if let OxcExpression::ArrayExpression(arr) =
                                                &ip.value
                                            {
                                                if let Some(first) = arr.elements.first() {
                                                    if let Some(OxcExpression::StringLiteral(s)) =
                                                        first.as_expression()
                                                    {
                                                        public_name = Some(s.value.to_string());
                                                    }
                                                }
                                            }

                                            if let Some(name) = public_name {
                                                inputs.push(name);
                                            } else if let Some(key_name) = ip.key.name() {
                                                // Fallback to key
                                                inputs.push(key_name.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                            "outputs" => {
                                if let OxcExpression::ObjectExpression(out_obj) = &p.value {
                                    for out_prop in &out_obj.properties {
                                        if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(
                                            op,
                                        ) = out_prop
                                        {
                                            if let Some(oname) = op.key.name() {
                                                outputs.push(oname.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            return Some((selector, inputs, outputs, export_as, is_cmp));
        }
        None
    }

    fn parse_pipe_meta(&self, expr: &OxcExpression) -> Option<String> {
        if let OxcExpression::ObjectExpression(obj) = expr {
            for prop in &obj.properties {
                if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                    if let Some(key) = p.key.name() {
                        if key == "name" {
                            if let OxcExpression::StringLiteral(s) = &p.value {
                                return Some(s.value.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn extract_ts_metadata(&self, path: &Path) -> Option<Vec<R3TemplateDependencyMetadata>> {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                // eprintln!("DEBUG: [ts_reader] Failed to read file {}: {}", path.display(), e);
                return None;
            }
        };

        let allocator = Allocator::default();
        let source_type = SourceType::from_path(path).unwrap_or_default();
        let ret = Parser::new(&allocator, &content, source_type).parse();

        if !ret.errors.is_empty() {
            // eprintln!("DEBUG: [ts_reader] Parse errors for {}: {:?}", path.display(), ret.errors);
            return None;
        }

        let program = ret.program;
        let mut results = Vec::new();
        let mut imports = HashMap::new();

        for stmt in &program.body {
            if let Statement::ImportDeclaration(import_decl) = stmt {
                let module_path = import_decl.source.value.as_str();
                if let Some(specifiers) = &import_decl.specifiers {
                    for spec in specifiers {
                        let local_name = match spec {
                            oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                s.local.name.as_str()
                            }
                            oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                s.local.name.as_str()
                            }
                            oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                                s,
                            ) => s.local.name.as_str(),
                        };
                        imports.insert(local_name.to_string(), module_path.to_string());
                    }
                }
            }
        }

        for stmt in &program.body {
            let class_decl = match stmt {
                Statement::ClassDeclaration(c) => Some(c),
                Statement::ExportNamedDeclaration(e) => {
                    if let Some(Declaration::ClassDeclaration(c)) = &e.declaration {
                        Some(c)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(class_decl) = class_decl {
                // Find @Component or @Directive decorator
                for dec in &class_decl.decorators {
                    let dec_name = if let OxcExpression::CallExpression(call) = &dec.expression {
                        if let OxcExpression::Identifier(ident) = &call.callee {
                            Some(ident.name.as_str())
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(name) = dec_name {
                        if name == "Component" || name == "Directive" || name == "Pipe" {
                            // Construct reflection::Decorator
                            let args = if let OxcExpression::CallExpression(call) = &dec.expression
                            {
                                Some(
                                    call.arguments
                                        .iter()
                                        .filter_map(|arg| arg.as_expression())
                                        .collect(),
                                )
                            } else {
                                None
                            };

                            let decorator = Decorator {
                                name: name.to_string(),
                                identifier: None, // We don't have resolved identifier yet
                                import: None,
                                node: dec,
                                args,
                            };

                            // For pipes
                            if name == "Pipe" {
                                // TODO: extract_pipe_metadata
                                continue;
                            }

                            let is_component = name == "Component";

                            if let Some(meta) = extract_directive_metadata(
                                class_decl,
                                &decorator,
                                is_component,
                                path,
                                &imports,
                            ) {
                                if let DecoratorMetadata::Directive(dir_meta) = meta {
                                    // Convert DirectiveMeta to R3DirectiveDependencyMetadata
                                    // For local components, we want ReadVar(ClassName).

                                    let class_name = dir_meta.t2.name;
                                    let selector = dir_meta.t2.selector.unwrap_or_default();

                                    let type_expr = Expression::ReadVar(
                                        angular_compiler::output::output_ast::ReadVarExpr {
                                            name: class_name.clone(),
                                            type_: None,
                                            source_span: None,
                                        },
                                    );

                                    let inputs = dir_meta
                                        .t2
                                        .inputs
                                        .iter()
                                        .map(|(_, val)| val.binding_property_name.clone())
                                        .collect();
                                    let outputs = dir_meta
                                        .t2
                                        .outputs
                                        .iter()
                                        .map(|(_, val)| val.binding_property_name.clone())
                                        .collect();

                                    let r3_meta = R3TemplateDependencyMetadata::Directive(
                                        R3DirectiveDependencyMetadata {
                                            selector,
                                            type_: type_expr,
                                            inputs,
                                            outputs,
                                            export_as: dir_meta
                                                .t2
                                                .export_as
                                                .unwrap_or_default()
                                                .into(),
                                            kind: R3TemplateDependencyKind::Directive,
                                            is_component,
                                            source_span: None,
                                        },
                                    );

                                    results.push(r3_meta);
                                }
                            }
                        }
                    }
                }
            }
        }

        Some(results)
    }

    fn read_from_cache(&self, module_name: &str) -> Option<Vec<R3TemplateDependencyMetadata>> {
        use crate::linker::metadata_extractor::get_metadata_cache;

        eprintln!(
            "DEBUG: [cache_lookup] Attempting lookup for: {}",
            module_name
        );

        // Resolve the entry file path to normalize it for cache lookup
        let entry_path = if let Some(p) = self.resolve_module(module_name) {
            p
        } else {
            return None;
        };
        let entry_path_str = entry_path.to_string_lossy().to_string();

        // The linker cache uses the full absolute path as the key prefix.
        let cache_key_prefix = entry_path_str.clone();

        let cache = get_metadata_cache().read().ok()?;

        // 1. Find the NgModule in this file
        let mut results = Vec::new();
        let mut modules_found = Vec::new();

        for (key, module) in &cache.modules {
            if key.starts_with(&cache_key_prefix) {
                modules_found.push(module.clone());
            }
        }

        if modules_found.is_empty() {
            return None;
        }

        // 2. Collect all exports from found NgModules
        for module in modules_found {
            for export in &module.exports {
                // Look up the directive in the cache
                let lookup_path = export.source_path.as_deref().unwrap_or(&cache_key_prefix);
                let lookup_name = export
                    .original_name
                    .as_deref()
                    .unwrap_or(&export.exported_name);
                let directive_key = format!("{}:{}", lookup_path, lookup_name);

                if let Some(directive) = cache.directives.get(&directive_key) {
                    // Convert ExtractedDirective to R3TemplateDependencyMetadata
                    let expression = Expression::External(ExternalExpr {
                        value: ExternalReference {
                            module_name: Some(module_name.to_string()),
                            name: Some(directive.name.clone()),
                            runtime: None,
                        },
                        type_: None,
                        source_span: None,
                    });

                    results.push(R3TemplateDependencyMetadata::Directive(
                        R3DirectiveDependencyMetadata {
                            selector: directive.selector.clone(),
                            type_: expression,
                            inputs: directive.inputs.clone(),
                            outputs: directive.outputs.clone(),
                            export_as: directive
                                .export_as
                                .as_ref()
                                .map(|s| {
                                    s.split(',')
                                        .map(|p| p.trim().to_string())
                                        .collect::<Vec<String>>()
                                })
                                .unwrap_or_default()
                                .into(),
                            kind: R3TemplateDependencyKind::Directive,
                            is_component: directive.is_component,
                            source_span: None,
                        },
                    ));
                }
            }
        }

        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    }
}
