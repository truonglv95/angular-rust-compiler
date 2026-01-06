//! NgModule Metadata Extraction
//!
//! This module extracts metadata from linked JavaScript to enable dynamic
//! NgModule resolution. It handles:
//! - Parsing `ɵmod`, `ɵcmp`, `ɵdir` definitions.
//! - Resolving re-exported directives via imports.
//! - Supporting both `defineComponent` (Ivy) and `ngDeclareComponent` (Partial) formats.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    AssignmentTarget, Declaration, Expression, ObjectPropertyKind, PropertyKey, Statement,
};
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Extracted metadata for a directive or component
#[derive(Debug, Clone)]
pub struct ExtractedDirective {
    pub name: String,
    pub selector: String,
    pub export_as: Option<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub host_attrs: Vec<String>,
    pub is_component: bool,
}

#[derive(Debug, Clone)]
pub struct ExportReference {
    pub exported_name: String,
    pub source_path: Option<String>, // None if defined in current module
    pub original_name: Option<String>, // If re-exported with alias or from another module
}

/// Extracted metadata for an NgModule
#[derive(Debug, Clone)]
pub struct ExtractedNgModule {
    pub name: String,
    pub exports: Vec<ExportReference>,
}

/// Global metadata cache, keyed by module path
static METADATA_CACHE: OnceLock<RwLock<MetadataCache>> = OnceLock::new();

#[derive(Debug, Default)]
pub struct MetadataCache {
    pub modules: HashMap<String, ExtractedNgModule>,
    pub directives: HashMap<String, ExtractedDirective>,
}

impl MetadataCache {
    fn new() -> Self {
        Self {
            modules: HashMap::new(),
            directives: HashMap::new(),
        }
    }
}

/// Get the global metadata cache
pub fn get_metadata_cache() -> &'static RwLock<MetadataCache> {
    METADATA_CACHE.get_or_init(|| RwLock::new(MetadataCache::new()))
}

fn normalize_path(path: &Path) -> String {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                components.pop();
            }
            c => components.push(c),
        }
    }
    components
        .iter()
        .collect::<PathBuf>()
        .to_string_lossy()
        .to_string()
}

/// Extract NgModule and directive metadata from linked JavaScript code.
pub fn extract_metadata_from_linked(
    module_path: &str,
    linked_code: &str,
) -> (Vec<ExtractedNgModule>, Vec<ExtractedDirective>) {
    // Normalize module_path by stripping query strings (e.g., ?v=1acbc39a from Vite)
    let normalized_module_path = module_path.split('?').next().unwrap_or(module_path);

    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true);
    let ret = Parser::new(&allocator, linked_code, source_type).parse();

    let mut modules = Vec::new();
    let mut directives = Vec::new();

    // Map content: LocalName -> (SourcePath, OriginalName)
    let mut imports: HashMap<String, (String, String)> = HashMap::new();

    // First pass: Collect imports
    for stmt in &ret.program.body {
        if let Statement::ImportDeclaration(decl) = stmt {
            let source_str = decl.source.value.as_str();

            // Resolve absolute path if relative
            let resolved_path = if source_str.starts_with('.') {
                if let Some(parent) = Path::new(module_path).parent() {
                    let p = parent.join(source_str);
                    normalize_path(&p)
                } else {
                    source_str.to_string()
                }
            } else {
                source_str.to_string()
            };

            if let Some(specifiers) = &decl.specifiers {
                for spec in specifiers {
                    match spec {
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(s) => {
                            let imported_name = s.imported.name().as_str().to_string();
                            let local_name = s.local.name.as_str().to_string();
                            imports.insert(local_name, (resolved_path.clone(), imported_name));
                        }
                        oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                            let local_name = s.local.name.as_str().to_string();
                            imports
                                .insert(local_name, (resolved_path.clone(), "default".to_string()));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Second pass: Process definitions
    for stmt in &ret.program.body {
        match stmt {
            Statement::ExportNamedDeclaration(decl) => {
                if let Some(Declaration::ClassDeclaration(class_decl)) = &decl.declaration {
                    process_class_declaration(class_decl, &mut modules, &mut directives, &imports);
                }
            }
            Statement::ClassDeclaration(class_decl) => {
                process_class_declaration(class_decl, &mut modules, &mut directives, &imports);
            }
            Statement::ExpressionStatement(expr_stmt) => {
                if let Expression::AssignmentExpression(assign) = &expr_stmt.expression {
                    if let AssignmentTarget::StaticMemberExpression(member) = &assign.left {
                        if let Expression::Identifier(obj_ident) = &member.object {
                            let class_name = obj_ident.name.to_string();
                            let key_name = member.property.name.as_str();
                            eprintln!(
                                "[Metadata] Visiting static member: {}.{}",
                                class_name, key_name
                            );
                            process_definition(
                                &class_name,
                                key_name,
                                &assign.right,
                                &mut modules,
                                &mut directives,
                                &imports,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Store in cache
    if !modules.is_empty() || !directives.is_empty() {
        // Debug logging
        #[cfg(not(test))]
        {
            // Only log relevant Angular Material modules to avoid noise
            if module_path.contains("@angular/material") {
                eprintln!("[Rust] Processed module: {}", module_path);
                for m in &modules {
                    eprintln!("[Rust]   Extracted NgModule: {}", m.name);
                    for e in &m.exports {
                        eprintln!(
                            "[Rust]     Export: {} (Source: {:?})",
                            e.exported_name, e.source_path
                        );
                    }
                }
                for d in &directives {
                    eprintln!(
                        "[Rust]   Extracted Directive: {} (Selector: {})",
                        d.name, d.selector
                    );
                }

                // Log if we found MatLabel
                if directives.iter().any(|d| d.selector == "mat-label") {
                    eprintln!("[Rust]   !!! FOUND MatLabel DIRECTIVE !!!");
                }
            }
        }

        if let Ok(mut cache) = get_metadata_cache().write() {
            for module in &modules {
                cache.modules.insert(
                    format!("{}:{}", normalized_module_path, module.name),
                    module.clone(),
                );
            }
            for directive in &directives {
                cache.directives.insert(
                    format!("{}:{}", normalized_module_path, directive.name),
                    directive.clone(),
                );
            }
        }
    }

    (modules, directives)
}

fn process_class_declaration(
    class_decl: &oxc_ast::ast::Class,
    modules: &mut Vec<ExtractedNgModule>,
    directives: &mut Vec<ExtractedDirective>,
    imports: &HashMap<String, (String, String)>,
) {
    if let Some(ident) = &class_decl.id {
        let class_name = ident.name.to_string();
        for elem in &class_decl.body.body {
            if let oxc_ast::ast::ClassElement::PropertyDefinition(prop) = elem {
                if prop.r#static {
                    if let PropertyKey::StaticIdentifier(key) = &prop.key {
                        let key_name = key.name.as_str();
                        eprintln!(
                            "[Metadata] Visiting class property: {}.{}",
                            class_name, key_name
                        );
                        if let Some(value) = &prop.value {
                            process_definition(
                                &class_name,
                                key_name,
                                value,
                                modules,
                                directives,
                                imports,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn process_definition(
    class_name: &str,
    prop_name: &str,
    value: &Expression,
    modules: &mut Vec<ExtractedNgModule>,
    directives: &mut Vec<ExtractedDirective>,
    imports: &HashMap<String, (String, String)>,
) {
    if prop_name == "ɵmod" {
        // NgModule
        if let Some(exports) = extract_ng_module_exports(value, imports) {
            modules.push(ExtractedNgModule {
                name: class_name.to_string(),
                exports,
            });
        }
    } else if prop_name == "ɵcmp" || prop_name == "ɵdir" {
        // Component or Directive
        if let Some((selector, export_as, host_attrs)) = extract_directive_metadata(value) {
            directives.push(ExtractedDirective {
                name: class_name.to_string(),
                selector,
                export_as,
                inputs: vec![],
                outputs: vec![],
                host_attrs,
                is_component: prop_name == "ɵcmp",
            });
        }
    }
}

fn extract_ng_module_exports(
    expr: &Expression,
    imports: &HashMap<String, (String, String)>,
) -> Option<Vec<ExportReference>> {
    // Looks for exports: [...] in ObjectExpression of ɵɵdefineNgModule or ɵɵngDeclareNgModule
    if let Expression::CallExpression(call) = expr {
        if let Some(arg) = call.arguments.first() {
            if let oxc_ast::ast::Argument::ObjectExpression(obj) = arg {
                for prop in &obj.properties {
                    if let ObjectPropertyKind::ObjectProperty(p) = prop {
                        if let PropertyKey::StaticIdentifier(key) = &p.key {
                            if key.name == "exports" {
                                if let Expression::ArrayExpression(arr) = &p.value {
                                    let mut exports = Vec::new();
                                    for elem in &arr.elements {
                                        if let oxc_ast::ast::ArrayExpressionElement::Identifier(
                                            ident,
                                        ) = elem
                                        {
                                            let local_name = ident.name.to_string();
                                            if let Some((source, original)) =
                                                imports.get(&local_name)
                                            {
                                                // Re-exported import
                                                exports.push(ExportReference {
                                                    exported_name: local_name,
                                                    source_path: Some(source.clone()),
                                                    original_name: Some(original.clone()),
                                                });
                                            } else {
                                                // Local definition
                                                exports.push(ExportReference {
                                                    exported_name: local_name,
                                                    source_path: None,
                                                    original_name: None,
                                                });
                                            }
                                        }
                                    }
                                    return Some(exports);
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

fn extract_directive_metadata(expr: &Expression) -> Option<(String, Option<String>, Vec<String>)> {
    let mut selector = String::new();
    let mut export_as = None;
    let mut host_attrs = Vec::new();

    eprintln!(
        "[Metadata]   Extracting metadata from expression: {:?}",
        expr
    );
    if let Expression::CallExpression(call) = expr {
        if let Some(arg) = call.arguments.first() {
            eprintln!("[Metadata]     First argument type: {:?}", arg);
            if let oxc_ast::ast::Argument::ObjectExpression(obj) = arg {
                for prop in &obj.properties {
                    if let ObjectPropertyKind::ObjectProperty(p) = prop {
                        if let PropertyKey::StaticIdentifier(key) = &p.key {
                            eprintln!("[Metadata]       Found property: {}", key.name);
                            if key.name == "selectors" {
                                // JIT/Ivy Component format: selectors: [["tag", ...]]
                                if let Expression::ArrayExpression(arr) = &p.value {
                                    selector = parse_selectors_array_ast(arr);
                                }
                            } else if key.name == "selector" {
                                // ngDeclareDirective format (Partial): selector: "tag"
                                if let Expression::StringLiteral(lit) = &p.value {
                                    selector = lit.value.to_string();
                                }
                            } else if key.name == "exportAs" {
                                if let Expression::StringLiteral(lit) = &p.value {
                                    export_as = Some(lit.value.to_string());
                                    eprintln!(
                                        "[Metadata]         Matched exportAs: {:?}",
                                        export_as
                                    );
                                } else if let Expression::ArrayExpression(arr) = &p.value {
                                    let mut names = Vec::new();
                                    for elem in &arr.elements {
                                        if let Some(expr) = elem.as_expression() {
                                            if let Some(val) = extract_string_value(expr) {
                                                names.push(val);
                                            }
                                        }
                                    }
                                    if !names.is_empty() {
                                        export_as = Some(names.join(","));
                                        eprintln!(
                                            "[Metadata]         Matched exportAs (array): {:?}",
                                            export_as
                                        );
                                    }
                                }
                            } else if key.name == "hostAttrs" {
                                if let Expression::ArrayExpression(arr) = &p.value {
                                    host_attrs = parse_host_attrs_ast(arr);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if selector.is_empty() {
        None
    } else {
        Some((selector, export_as, host_attrs))
    }
}

fn parse_selectors_array_ast(arr: &oxc_ast::ast::ArrayExpression) -> String {
    let mut parts = Vec::new();
    eprintln!(
        "[Metadata]   Parsing selectors array. Elements: {}",
        arr.elements.len()
    );
    for elem in &arr.elements {
        if let Some(expr) = elem.as_expression() {
            if let oxc_ast::ast::Expression::ArrayExpression(inner_arr) = expr {
                let s = parse_single_selector_array_ast(inner_arr);
                if !s.is_empty() {
                    parts.push(s);
                }
            }
        }
    }
    let res = parts.join(", ");
    eprintln!("[Metadata]   Parsed selectors: {}", res);
    res
}

fn parse_single_selector_array_ast(arr: &oxc_ast::ast::ArrayExpression) -> String {
    let mut str_parts = Vec::new();
    for elem in &arr.elements {
        if let oxc_ast::ast::ArrayExpressionElement::StringLiteral(lit) = elem {
            str_parts.push(lit.value.as_str());
        }
    }

    if str_parts.is_empty() {
        return String::new();
    }

    let tag = str_parts[0];
    let mut attrs = Vec::new();

    for i in (1..str_parts.len()).step_by(2) {
        let attr_name = str_parts[i];
        let attr_value = if i + 1 < str_parts.len() {
            str_parts[i + 1]
        } else {
            ""
        };

        if !attr_name.is_empty() {
            if attr_value.is_empty() {
                attrs.push(attr_name.to_string());
            } else {
                attrs.push(format!("{}={}", attr_name, attr_value));
            }
        }
    }

    if attrs.is_empty() {
        tag.to_string()
    } else {
        format!("{}[{}]", tag, attrs.join("]["))
    }
}

fn parse_host_attrs_ast(arr: &oxc_ast::ast::ArrayExpression) -> Vec<String> {
    let mut attrs = Vec::new();
    for elem in &arr.elements {
        if let oxc_ast::ast::ArrayExpressionElement::StringLiteral(lit) = elem {
            attrs.push(lit.value.to_string());
        }
    }
    attrs
}

/// Look up an NgModule's exported directives from the cache
pub fn get_module_exports(module_path: &str, module_name: &str) -> Option<Vec<ExtractedDirective>> {
    let cache = get_metadata_cache().read().ok()?;
    let module = cache
        .modules
        .get(&format!("{}:{}", module_path, module_name))?;

    let mut result = Vec::new();
    for export in &module.exports {
        let lookup_path = export.source_path.as_deref().unwrap_or(module_path);
        let lookup_name = export
            .original_name
            .as_deref()
            .unwrap_or(&export.exported_name);

        let key = format!("{}:{}", lookup_path, lookup_name);

        // Debug
        #[cfg(not(test))]
        if module_path.contains("@angular/material") {
            // eprintln!("[Rust]   Looking up export {} -> key {}", export.exported_name, key);
        }

        if let Some(directive) = cache.directives.get(&key) {
            result.push(directive.clone());
        }
    }

    Some(result)
}

/// Look up a directive's metadata from the cache
pub fn get_directive_metadata(
    module_path: &str,
    directive_name: &str,
) -> Option<ExtractedDirective> {
    let cache = get_metadata_cache().read().ok()?;
    cache
        .directives
        .get(&format!("{}:{}", module_path, directive_name))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_metadata_from_ast() {
        let code = r#"
            class MatButtonModule {
                static ɵmod = ɵɵdefineNgModule({
                    type: MatButtonModule,
                    exports: [MatButton, MatFabButton]
                });
            }
            
            class MatButton {
                static ɵcmp = ɵɵdefineComponent({
                    selectors: [["button", "mat-button", ""]],
                    exportAs: "matButton",
                    hostAttrs: [1, "my-class"]
                });
            }
        "#;

        let (modules, directives) = extract_metadata_from_linked("@angular/material/button", code);

        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].name, "MatButtonModule");
        // Check exports (local refs)
        assert!(modules[0]
            .exports
            .iter()
            .any(|e| e.exported_name == "MatButton" && e.source_path.is_none()));

        assert_eq!(directives.len(), 1);
        let dir = &directives[0];
        assert_eq!(dir.name, "MatButton");
        assert_eq!(dir.selector, "button[mat-button]");
        assert_eq!(dir.export_as, Some("matButton".to_string()));
        assert_eq!(dir.host_attrs, vec!["my-class".to_string()]);
    }

    #[test]
    fn test_extract_metadata_with_array_export_as() {
        let allocator = Allocator::default();
        let program = Parser::new(
            &allocator,
            r#"
            ɵɵngDeclareComponent({
                minVersion: "14.0.0",
                version: "15.0.0",
                ngImport: i0,
                type: MyComponent,
                isStandalone: true,
                selector: "div[my-dir]",
                exportAs: ["dir1", "dir2"],
            });
        "#,
            SourceType::default(),
        )
        .parse()
        .program;

        let mut directives = Vec::new();
        let mut modules = Vec::new();
        let imports = std::collections::HashMap::new();

        for stmt in &program.body {
            if let Statement::ExpressionStatement(expr_stmt) = stmt {
                if let Expression::CallExpression(call) = &expr_stmt.expression {
                    if let Expression::Identifier(_) = &call.callee {
                        process_definition(
                            "MyComponent",
                            "ɵcmp",
                            &expr_stmt.expression,
                            &mut modules,
                            &mut directives,
                            &imports,
                        );
                    }
                }
            }
        }

        assert_eq!(directives.len(), 1);
        let dir = &directives[0];
        assert_eq!(dir.export_as, Some("dir1,dir2".to_string()));
    }

    #[test]
    fn test_extract_ng_declare_directive() {
        let code = r#"
            class MatLabel {
              static ɵdir = i0.ɵɵngDeclareDirective({
                minVersion: "14.0.0",
                version: "21.0.3",
                type: MatLabel,
                isStandalone: true,
                selector: "mat-label",
                ngImport: i0
              });
            }
        "#;
        let (modules, directives) = extract_metadata_from_linked("any", code);
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].name, "MatLabel");
        assert_eq!(directives[0].selector, "mat-label");
    }

    #[test]
    fn test_extract_re_export() {
        let code = r#"
            import { MatLabel } from './_form-field-chunk.mjs';
            class MatFormFieldModule {
                static ɵmod = i0.ɵɵngDeclareNgModule({
                    type: MatFormFieldModule,
                    exports: [MatLabel]
                });
            }
        "#;
        // Mock absolute path (using / for test)
        let module_path = "/root/node_modules/material/form-field.mjs";
        let (modules, directives) = extract_metadata_from_linked(module_path, code);

        assert_eq!(modules.len(), 1);
        let export = &modules[0].exports[0];
        assert_eq!(export.exported_name, "MatLabel");

        // Path normalization in test (mock fs is strict, but our normalize_path is simple string manip)
        // /root/node_modules/material + ./_form-field-chunk.mjs -> /root/node_modules/material/_form-field-chunk.mjs
        assert_eq!(
            export.source_path,
            Some("/root/node_modules/material/_form-field-chunk.mjs".to_string())
        );
        assert_eq!(export.original_name, Some("MatLabel".to_string()));
    }
}

/// Helper to extract string value from Expression (StringLiteral or TemplateLiteral)
fn extract_string_value(expr: &oxc_ast::ast::Expression) -> Option<String> {
    use oxc_ast::ast::Expression;
    match expr {
        Expression::StringLiteral(s) => Some(s.value.to_string()),
        Expression::TemplateLiteral(t) => {
            // Join all quasis into a single string.
            // Note: We ignore expressions in template literals as they're not common in Angular templates/selectors
            let mut result = String::new();
            for quasi in &t.quasis {
                result.push_str(&quasi.value.raw);
            }
            Some(result)
        }
        _ => None,
    }
}
