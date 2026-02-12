use crate::ngtsc::translator::src::api::ast_factory::AstFactory;
use crate::ngtsc::translator::src::api::import_generator::{ImportGenerator, ImportRequest};
use crate::ngtsc::translator::src::import_manager::check_unique_identifier_name::{
    IdentifierScope, UniqueIdentifierGenerator,
};
use crate::ngtsc::translator::src::import_manager::reuse_generated_imports::{
    attempt_to_reuse_generated_imports, capture_generated_import, ReuseGeneratedImportsTracker,
};
use crate::ngtsc::translator::src::import_manager::reuse_source_file_imports::{
    attempt_to_reuse_existing_source_file_imports, ReuseExistingSourceFileImportsTracker,
    SourceFileImports,
};
use angular_compiler::output::output_ast::{Expression, ReadPropExpr, ReadVarExpr};
use std::collections::{HashMap, HashSet};

// We define ModuleName as generic string for now.
pub type ModuleName = String;

pub struct ImportManagerConfig {
    pub namespace_import_prefix: String,
    pub disable_original_source_file_reuse: bool,
    pub force_generate_namespaces_for_new_imports: bool,
    // generateUniqueIdentifier is handled by UniqueIdentifierGenerator helper
}

pub struct ImportManager<'a, A: AstFactory, TFile> {
    config: ImportManagerConfig,
    ast_factory: &'a A,

    // Per file tracking
    new_imports: HashMap<TFile, NewImportsForFile>, // TFile must be Hash + Eq
    next_unique_index: usize,

    reuse_generated_imports_tracker: ReuseGeneratedImportsTracker<A::Expression>,
    reuse_source_file_imports_tracker: ReuseExistingSourceFileImportsTracker,
    unique_id_generator: UniqueIdentifierGenerator,
}

struct NewImportsForFile {
    namespace_imports: HashMap<ModuleName, String>, // Module -> Namespace Name
    named_imports: HashMap<ModuleName, Vec<ImportSpecifierInternal>>,
    side_effect_imports: HashSet<ModuleName>,
}

pub struct ImportSpecifierInternal {
    pub name: String,
    pub alias: Option<String>,
}

pub struct FinalizeResult<TFile, TDeclaration> {
    pub affected_files: HashSet<TFile>,
    pub new_imports: HashMap<TFile, Vec<NewImport>>, // TDeclaration usually Statement
    pub updated_imports: HashMap<TDeclaration, Vec<ImportSpecifierInternal>>, // ImportDeclaration -> specifiers
    pub reused_original_alias_declarations: HashSet<TDeclaration>,
}

pub struct NewImport {
    pub module_specifier: String,
    pub namespace_import: Option<String>,
    pub named_imports: Vec<ImportSpecifierInternal>,
    pub side_effect: bool,
}

impl<'a, A: AstFactory, TFile> ImportManager<'a, A, TFile>
where
    TFile: std::hash::Hash + Eq + Clone + IdentifierScope + SourceFileImports,
    A::Expression: Clone,
{
    pub fn new(ast_factory: &'a A, config: ImportManagerConfig) -> Self {
        Self {
            config,
            ast_factory,
            new_imports: HashMap::new(),
            next_unique_index: 0,
            reuse_generated_imports_tracker: ReuseGeneratedImportsTracker::new(),
            reuse_source_file_imports_tracker: ReuseExistingSourceFileImportsTracker::new(),
            unique_id_generator: UniqueIdentifierGenerator::new(),
        }
    }

    fn get_new_imports_tracker_for_file(&mut self, file: &TFile) -> &mut NewImportsForFile {
        self.new_imports
            .entry(file.clone())
            .or_insert(NewImportsForFile {
                namespace_imports: HashMap::new(),
                named_imports: HashMap::new(),
                side_effect_imports: HashSet::new(),
            })
    }

    pub fn finalize(&self) -> FinalizeResult<TFile, A::Statement> {
        // Collect new imports to be generated
        let mut new_imports = HashMap::new();
        let mut affected_files = HashSet::new();

        for (file, tracker) in &self.new_imports {
            affected_files.insert(file.clone());
            let file_new_imports = new_imports.entry(file.clone()).or_insert(Vec::new());

            // Process namespace imports
            for (module, alias) in &tracker.namespace_imports {
                file_new_imports.push(NewImport {
                    module_specifier: module.clone(),
                    namespace_import: Some(alias.clone()),
                    named_imports: Vec::new(),
                    side_effect: false,
                });
            }

            // Process named imports
            for (module, specifiers) in &tracker.named_imports {
                // Merge with existing named import if any? No, this is for *new* imports.
                if !specifiers.is_empty() {
                    // We need to clone specifiers.
                    let specifiers_cloned = specifiers
                        .iter()
                        .map(|s| ImportSpecifierInternal {
                            name: s.name.clone(),
                            alias: s.alias.clone(),
                        })
                        .collect();

                    file_new_imports.push(NewImport {
                        module_specifier: module.clone(),
                        namespace_import: None,
                        named_imports: specifiers_cloned,
                        side_effect: false,
                    });
                }
            }

            // Side effects
            for module in &tracker.side_effect_imports {
                file_new_imports.push(NewImport {
                    module_specifier: module.clone(),
                    namespace_import: None,
                    named_imports: Vec::new(),
                    side_effect: true,
                });
            }
        }

        // Updated imports would come from reuse tracker
        // But in reuse_source_file_imports implementation I used `ExistingImport` which is just data.
        // If we want to map back to AST nodes (A::Statement), we need to store them or be able to look valid ones up.
        // For now, I'll return empty maps for updated/reused until I link specific AST nodes.

        FinalizeResult {
            affected_files,
            new_imports,
            updated_imports: HashMap::new(), // TODO: Populate from reuse tracker
            reused_original_alias_declarations: HashSet::new(), // TODO: Populate
        }
    }
}

impl<'a, A: AstFactory, TFile> ImportGenerator<TFile, A::Expression> for ImportManager<'a, A, TFile>
where
    TFile: std::hash::Hash + Eq + Clone + IdentifierScope + SourceFileImports,
    A::Expression: Clone,
{
    fn add_import(&mut self, request: ImportRequest<TFile>) -> A::Expression {
        // Reuse generated
        if let Some(reused) =
            attempt_to_reuse_generated_imports(&self.reuse_generated_imports_tracker, &request)
        {
            // Need to handle if reused is a namespace import but we wanted a named import -> return PropertyAccess
            if request.export_symbol_name.is_some() {
                // If reused is Identifier(ns), we need ns.Symbol
                // reuse_generated_imports returns TExpression.
                // If TExpression is opaque, we rely on reuse logic returning the final expression?
                // But reuse logic returned just the cached one.
                // We should improve reuse_generated_imports to handle this or handle it here.
                // For now assuming direct reuse only.
                return reused;
            }
            return reused;
        }

        // Reuse source file
        if !self.config.disable_original_source_file_reuse {
            if let Some(reused) = attempt_to_reuse_existing_source_file_imports(
                &mut self.reuse_source_file_imports_tracker,
                &request.requested_file,
                &request,
                self.ast_factory,
            ) {
                return reused;
            }
        }

        let file = request.requested_file.clone();

        // Namespace Import
        if request.export_symbol_name.is_none()
            || self.config.force_generate_namespaces_for_new_imports
        {
            // Logic to generate namespace import
            let mut ns_name = format!(
                "{}{}",
                self.config.namespace_import_prefix, self.next_unique_index
            );
            self.next_unique_index += 1;

            // Check unique
            if let Some(unique) = self
                .unique_id_generator
                .generate_unique_identifier(&file, &ns_name)
            {
                ns_name = unique;
            }

            // Store in tracker
            let tracker = self
                .new_imports
                .entry(file.clone())
                .or_insert(NewImportsForFile {
                    namespace_imports: HashMap::new(),
                    named_imports: HashMap::new(),
                    side_effect_imports: HashSet::new(),
                });
            tracker
                .namespace_imports
                .insert(request.export_module_specifier.clone(), ns_name.clone());

            let ns_expr = self.ast_factory.create_identifier(&ns_name);
            capture_generated_import(
                &request,
                &mut self.reuse_generated_imports_tracker,
                ns_expr.clone(),
            );

            if let Some(symbol) = request.export_symbol_name {
                return self.ast_factory.create_property_access(ns_expr, &symbol);
            }
            return ns_expr;
        }

        // Named Import
        let symbol_name = request.export_symbol_name.as_ref().unwrap();

        // Generate unique alias if needed
        let unique_name = if let Some(alias) = &request.unsafe_alias_override {
            alias.clone()
        } else {
            self.unique_id_generator
                .generate_unique_identifier(&file, symbol_name)
                .unwrap_or(symbol_name.clone())
        };

        let needs_alias = &unique_name != symbol_name || request.unsafe_alias_override.is_some();
        let specifier_alias = if needs_alias {
            Some(unique_name.clone())
        } else {
            None
        };

        let tracker = self
            .new_imports
            .entry(file.clone())
            .or_insert(NewImportsForFile {
                namespace_imports: HashMap::new(),
                named_imports: HashMap::new(),
                side_effect_imports: HashSet::new(),
            });

        let exports = tracker
            .named_imports
            .entry(request.export_module_specifier.clone())
            .or_insert(Vec::new());

        exports.push(ImportSpecifierInternal {
            name: symbol_name.clone(),
            alias: specifier_alias,
        });

        let expr = self.ast_factory.create_identifier(&unique_name);
        capture_generated_import(
            &request,
            &mut self.reuse_generated_imports_tracker,
            expr.clone(),
        );
        expr
    }
}

// =========================================================================================
// EmitterImportManager
// A simplified ImportManager for use specifically with AbstractJsEmitter where no AST context exists.
//
// Aliases are assigned incrementally on first registration and cached.
// The `generate_import_statements` and `get_imports_map` methods sort by MODULE NAME
// to ensure deterministic OUTPUT ordering, but alias assignments are stable.
// =========================================================================================

pub struct EmitterImportManager {
    /// Map of module name -> cached alias (e.g. "@angular/core" -> "i0")
    /// Once assigned, alias never changes for this compilation run.
    imports: HashMap<String, String>,
    /// Counter for generating unique aliases
    next_id: usize,
    /// Parse source local imports to reuse
    /// Map (Module Name, Export Name) -> Local Variable Name
    local_imports: HashMap<(String, String), String>,
}

impl EmitterImportManager {
    pub fn new() -> Self {
        Self {
            imports: HashMap::new(),
            next_id: 0,
            local_imports: HashMap::new(),
        }
    }

    /// Get the alias for a module, generating one if it doesn't exist.
    /// Once assigned, the alias is cached and never changes.
    pub fn get_or_generate_alias(&mut self, module_name: &str) -> String {
        if let Some(alias) = self.imports.get(module_name) {
            return alias.clone();
        }

        let alias = format!("i{}", self.next_id);
        self.next_id += 1;
        self.imports.insert(module_name.to_string(), alias.clone());
        alias
    }

    /// Get the current map of imports to aliases
    pub fn get_imports_map(&self) -> HashMap<String, String> {
        self.imports.clone()
    }

    /// Generate the import statements to be prepended to the file.
    /// Imports are sorted by module name for deterministic OUTPUT.
    pub fn generate_import_statements(&self) -> String {
        let mut statements = String::new();
        let mut sorted_imports: Vec<_> = self.imports.iter().collect();
        // Sort by module name (a.0)
        sorted_imports.sort_by_key(|(module, _)| *module);

        for (module, alias) in sorted_imports {
            statements.push_str(&format!("import * as {} from '{}';\n", alias, module));
        }
        statements
    }

    pub fn add_local_import(&mut self, module: &str, name: &str, local_name: &str) {
        self.local_imports.insert(
            (module.to_string(), name.to_string()),
            local_name.to_string(),
        );
    }
}

impl<TFile> ImportGenerator<TFile, Expression> for EmitterImportManager {
    fn add_import(&mut self, request: ImportRequest<TFile>) -> Expression {
        let module = request.export_module_specifier;
        let symbol_opt = request.export_symbol_name;

        // Check local imports first if a specific symbol is requested
        if let Some(symbol) = &symbol_opt {
            if let Some(local) = self.local_imports.get(&(module.clone(), symbol.clone())) {
                {
                    use std::io::Write;
                    let path =
                        std::path::Path::new("/Users/truong/Desktop/rust-compiler/hmr_debug.log");
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                    {
                        let _ = writeln!(
                            f,
                            "LOOKUP MATCH: module='{}', symbol='{}' -> local='{}'",
                            module, symbol, local
                        );
                    }
                }
                return Expression::ReadVar(ReadVarExpr {
                    name: local.clone(),
                    type_: None,
                    source_span: None,
                });
            } else {
                {
                    use std::io::Write;
                    let path =
                        std::path::Path::new("/Users/truong/Desktop/rust-compiler/hmr_debug.log");
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                    {
                        let _ = writeln!(
                            f,
                            "LOOKUP MISS: module='{}', symbol='{}'. Available keys: {:?}",
                            module,
                            symbol,
                            self.local_imports.keys().collect::<Vec<_>>()
                        );
                    }
                }
            }
        }

        let alias = self.get_or_generate_alias(&module);

        if let Some(symbol) = symbol_opt {
            Expression::ReadProp(ReadPropExpr {
                receiver: Box::new(Expression::ReadVar(ReadVarExpr {
                    name: alias,
                    type_: None,
                    source_span: None,
                })),
                name: symbol,
                type_: None,
                source_span: None,
            })
        } else {
            Expression::ReadVar(ReadVarExpr {
                name: alias,
                type_: None,
                source_span: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    #[test]
    fn test_hmr_local_import_registration() {
        let source_code = r#"
            import { Component } from '@angular/core';
            import { provideNativeDateAdapter } from '@angular/material/core';
            import { MatExpansionModule } from '@angular/material/expansion';
        "#;

        let allocator = Allocator::default();
        let source_type = SourceType::ts();
        let parser = Parser::new(&allocator, source_code, source_type);
        let parse_result = parser.parse();

        // Create import manager (which is in super)
        let mut import_manager = EmitterImportManager::new();

        // Replicating the logic from compiler.rs
        for stmt in &parse_result.program.body {
            if let oxc_ast::ast::Statement::ImportDeclaration(decl) = stmt {
                if decl.import_kind == oxc_ast::ast::ImportOrExportKind::Type {
                    continue;
                }

                let module_name = decl.source.value.as_str();
                if module_name == "@angular/core" {
                    continue;
                }

                if let Some(specifiers) = &decl.specifiers {
                    for spec in specifiers {
                        if let oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(s) = spec {
                            if s.import_kind == oxc_ast::ast::ImportOrExportKind::Type {
                                continue;
                            }
                            let local_name = s.local.name.as_str();
                            let imported_name = match &s.imported {
                                oxc_ast::ast::ModuleExportName::IdentifierName(id) => {
                                    id.name.as_str()
                                }
                                oxc_ast::ast::ModuleExportName::IdentifierReference(id) => {
                                    id.name.as_str()
                                }
                                oxc_ast::ast::ModuleExportName::StringLiteral(lit) => {
                                    lit.value.as_str()
                                }
                            };
                            import_manager.add_local_import(module_name, imported_name, local_name);
                        }
                    }
                }
            }
        }

        // Verify lookup
        // ImportRequest is generic over TFile. EmitterImportManager ignores TFile.
        // We can use () as TFile.
        let req = ImportRequest {
            export_module_specifier: "@angular/material/core".to_string(),
            export_symbol_name: Some("provideNativeDateAdapter".to_string()),
            requested_file: (),
            unsafe_alias_override: None,
        };

        let expr = import_manager.add_import(req);

        if let Expression::ReadVar(var) = expr {
            assert_eq!(
                var.name, "provideNativeDateAdapter",
                "Should reuse local import name"
            );
        } else {
            panic!("Expected ReadVar, got {:?}", expr);
        }

        // Verify fallback
        let req_alias = ImportRequest {
            export_module_specifier: "@angular/material/expansion".to_string(),
            export_symbol_name: Some("MatExpansionModule".to_string()),
            requested_file: (),
            unsafe_alias_override: None,
        };
        let expr_alias = import_manager.add_import(req_alias);
        if let Expression::ReadVar(var) = expr_alias {
            assert_eq!(
                var.name, "MatExpansionModule",
                "Should reuse local import name for MatExpansionModule"
            );
        } else {
            panic!(
                "Expected ReadVar for MatExpansionModule, got {:?}",
                expr_alias
            );
        }

        // Verify external (not in file)
        let req_external = ImportRequest {
            export_module_specifier: "@angular/common".to_string(),
            export_symbol_name: Some("CommonModule".to_string()),
            requested_file: (),
            unsafe_alias_override: None,
        };
        let expr_ext = import_manager.add_import(req_external);

        if let Expression::ReadProp(prop) = expr_ext {
            assert_eq!(prop.name, "CommonModule");
        } else {
            panic!("Expected ReadProp for external import, got {:?}", expr_ext);
        }
    }
}
