//! AST-based Ivy transformation utilities
//!
//! This module provides functions to transform OXC AST for Angular Ivy compilation:
//! - Remove Angular decorators (@Component, @Directive, etc.)
//! - Add Ivy static properties (ɵfac, ɵcmp)
//! - Merge/add import statements

use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::*;
use oxc_ast::AstBuilder;
use oxc_span::SPAN;

/// Represents a static property that failed to be parsed and needs to be injected via string manipulation
#[derive(Debug, Clone)]
pub struct FailedProperty {
    pub class_name: String,
    pub prop_name: String,
    pub placeholder: String,
    pub expr: String,
}

/// Transform a parsed program to add Ivy compilation results
/// Returns Vec of properties that couldn't be parsed and need to be injected into class bodies
pub fn transform_component_ast<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    component_name: &str,
    hoisted_statements: &'a str,
    fac_expr_str: &'a str,
    definitions: &[(&str, &str)],
    additional_imports: &[(String, String)],
) -> Vec<FailedProperty> {
    // 1. Remove Angular decorators from the class
    remove_angular_decorators(program, component_name);

    // 2. Ensure namespace imports exist (must be before hoisted statements)
    // Always add @angular/core as i0
    let mut imports = additional_imports.to_vec();
    if !imports.iter().any(|(alias, _)| alias == "i0") {
        imports.push(("i0".to_string(), "@angular/core".to_string()));
    }

    ensure_imports(allocator, program, &imports);

    // 3. Add hoisted statements to program body (after all imports)
    add_hoisted_statements(allocator, program, hoisted_statements);

    // 4. Add ɵfac and ɵcmp/ɵdir as static properties to the class
    let raw_suffix = add_static_properties_to_class(
        allocator,
        program,
        component_name,
        fac_expr_str,
        definitions,
    );

    raw_suffix
}

/// Remove Angular decorators (@Component, @Directive, etc.) from class declarations
pub fn remove_angular_decorators<'a>(program: &mut Program<'a>, class_name: &str) {
    for stmt in &mut program.body {
        match stmt {
            Statement::ExportDefaultDeclaration(export_decl) => {
                if let ExportDefaultDeclarationKind::ClassDeclaration(class) =
                    &mut export_decl.declaration
                {
                    if class.id.as_ref().map(|id| id.name.as_str()) == Some(class_name) {
                        remove_decorators_from_class(class);
                    }
                }
            }
            Statement::ExportNamedDeclaration(export_decl) => {
                if let Some(Declaration::ClassDeclaration(class)) = &mut export_decl.declaration {
                    if class.id.as_ref().map(|id| id.name.as_str()) == Some(class_name) {
                        remove_decorators_from_class(class);
                    }
                }
            }
            Statement::ClassDeclaration(class) => {
                if class.id.as_ref().map(|id| id.name.as_str()) == Some(class_name) {
                    remove_decorators_from_class(class);
                }
            }
            _ => {}
        }
    }
}

/// Remove Angular decorators from a class
fn remove_decorators_from_class(class: &mut Class) {
    // Angular decorators to remove from class declaration
    let class_decorators = ["Component", "Directive", "Injectable", "Pipe", "NgModule"];

    // Filter out Angular decorators form class
    class.decorators.retain(|decorator| {
        let decorator_name = get_decorator_name(decorator);
        let remove = class_decorators.contains(&decorator_name.as_str());
        eprintln!(
            "[RUST_DEBUG] Decorator: '{}', Remove: {}",
            decorator_name, remove
        );
        !remove
    });

    // Angular decorators to remove from class members
    let member_decorators = [
        "Input",
        "Output",
        "HostBinding",
        "HostListener",
        "ViewChild",
        "ViewChildren",
        "ContentChild",
        "ContentChildren",
    ];

    // Remove decorators from class body elements
    for element in class.body.body.iter_mut() {
        match element {
            ClassElement::PropertyDefinition(prop) => {
                prop.decorators.retain(|decorator| {
                    let decorator_name = get_decorator_name(decorator);
                    !member_decorators.contains(&decorator_name.as_str())
                });
            }
            ClassElement::MethodDefinition(method) => {
                method.decorators.retain(|decorator| {
                    let decorator_name = get_decorator_name(decorator);
                    !member_decorators.contains(&decorator_name.as_str())
                });

                // If it's a constructor, also remove decorators from parameters
                if method.kind == MethodDefinitionKind::Constructor {
                    for param in &mut method.value.params.items {
                        param.decorators.retain(|decorator| {
                            let decorator_name = get_decorator_name(decorator);
                            // Also remove dependency injection decorators
                            let di_decorators =
                                ["Optional", "Self", "SkipSelf", "Host", "Attribute"];
                            let should_keep = !member_decorators.contains(&decorator_name.as_str())
                                && !di_decorators.contains(&decorator_name.as_str());
                            should_keep
                        });
                    }
                }
            }
            ClassElement::AccessorProperty(accessor) => {
                accessor.decorators.retain(|decorator| {
                    let decorator_name = get_decorator_name(decorator);
                    !member_decorators.contains(&decorator_name.as_str())
                });
            }
            _ => {}
        }
    }
}

/// Get the name of a decorator
fn get_decorator_name(decorator: &Decorator) -> String {
    match &decorator.expression {
        Expression::CallExpression(call) => match &call.callee {
            Expression::Identifier(id) => id.name.to_string(),
            _ => String::new(),
        },
        Expression::Identifier(id) => id.name.to_string(),
        _ => String::new(),
    }
}

/// Add hoisted statements (function declarations, etc.) to program body
/// Insert them after all import statements to maintain correct order
fn add_hoisted_statements<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    hoisted_statements: &'a str,
) {
    if hoisted_statements.trim().is_empty() {
        return;
    }

    use oxc_parser::Parser;
    use oxc_span::SourceType;

    let source_type = SourceType::mjs();
    let parser = Parser::new(allocator, hoisted_statements, source_type);
    let parse_result = parser.parse();

    if parse_result.errors.is_empty() {
        // Find the last import statement index
        let last_import_index = program
            .body
            .iter()
            .rposition(|stmt| matches!(stmt, Statement::ImportDeclaration(_)))
            .map(|idx| idx + 1)
            .unwrap_or(0);

        // Add statements after the last import (or at the beginning if no imports)
        for (idx, stmt) in parse_result.program.body.into_iter().enumerate() {
            program.body.insert(last_import_index + idx, stmt);
        }
    }
}

/// Adds static properties to the class. Returns Vec<FailedProperty> with properties
/// that couldn't be parsed and need to be injected into class bodies via string manipulation.
fn add_static_properties_to_class<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    component_name: &str,
    fac_expr_str: &'a str,
    definitions: &[(&str, &str)],
) -> Vec<FailedProperty> {
    // Track which properties failed to be added as class members
    let mut failed_fac = false;
    let mut failed_defs = Vec::new();
    let mut class_found = false;
    let mut class_stmt_idx = None;

    // Find the class declaration and add static properties
    for (idx, stmt) in program.body.iter_mut().enumerate() {
        match stmt {
            Statement::ExportNamedDeclaration(export_decl) => {
                if let Some(Declaration::ClassDeclaration(class)) = &mut export_decl.declaration {
                    if let Some(class_id) = &class.id {
                        let c_name = class_id.name.as_str();
                        if c_name == component_name {
                            let (f_ok, failed_definitions) = add_properties_to_class_body(
                                allocator,
                                class,
                                fac_expr_str,
                                definitions,
                            );
                            failed_fac = !f_ok;
                            failed_defs = failed_definitions;
                            class_found = true;
                            class_stmt_idx = Some(idx);
                            break;
                        }
                    }
                }
            }

            Statement::ClassDeclaration(class) => {
                if let Some(class_id) = &class.id {
                    if class_id.name.as_str() == component_name {
                        let (f_ok, failed_definitions) = add_properties_to_class_body(
                            allocator,
                            class,
                            fac_expr_str,
                            definitions,
                        );
                        failed_fac = !f_ok;
                        failed_defs = failed_definitions;
                        class_found = true;
                        class_stmt_idx = Some(idx);
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    let mut failed_properties = Vec::new();

    if class_found {
        use oxc_parser::Parser;
        use oxc_span::SourceType;
        let source_type = SourceType::mjs();
        let mut stmts_to_insert = Vec::new();

        // Helper function to strip PURE annotation for AST parsing attempts
        fn strip_pure(s: &str) -> &str {
            if s.starts_with("/*@__PURE__*/") {
                s.trim_start_matches("/*@__PURE__*/").trim_start()
            } else {
                s
            }
        }

        // Handle failed ɵfac
        if failed_fac && !fac_expr_str.is_empty() {
            let fac_stripped = strip_pure(fac_expr_str);
            let fac_stmt_str = format!("{}.ɵfac = {};", component_name, fac_stripped);
            let fac_stmt_ptr = allocator.alloc_str(&fac_stmt_str);
            let parser = Parser::new(allocator, fac_stmt_ptr, source_type);
            let result = parser.parse();
            if result.errors.is_empty() {
                for stmt in result.program.body {
                    stmts_to_insert.push(stmt);
                }
                add_placeholder_to_class(
                    allocator,
                    program,
                    component_name,
                    "ɵfac",
                    fac_expr_str,
                    &mut failed_properties,
                );
            }
        }

        // Handle failed definitions
        for (def_name, def_expr_str) in failed_defs.iter().zip(definitions.iter()) {
            // zip might mismatch if failed_defs indices don't align?
            // Actually add_properties_to_class_body returns list of boolean failures?
            // Let's change return type of add_properties... to return definitions that failed.
        }

        // Insert any successfully parsed statements
        if let Some(idx) = class_stmt_idx {
            for (i, stmt) in stmts_to_insert.into_iter().enumerate() {
                program.body.insert(idx + 1 + i, stmt);
            }
        }
    }

    failed_properties
}

/// Helper to add a placeholder static property to a class in the AST
fn add_placeholder_to_class<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    class_name: &str,
    prop_name: &str,
    expr: &str,
    failed_properties: &mut Vec<FailedProperty>,
) {
    let ast = AstBuilder::new(allocator);
    let placeholder = format!("__NG_RAW_{}_{}__", class_name, prop_name);
    let placeholder_expr = format!("'{}'", placeholder);

    // Find class again and push PropertyDefinition
    for stmt in program.body.iter_mut() {
        let class = match stmt {
            Statement::ExportNamedDeclaration(export_decl) => {
                if let Some(Declaration::ClassDeclaration(class)) = &mut export_decl.declaration {
                    if class.id.as_ref().map(|id| id.name.as_str()) == Some(class_name) {
                        Some(class)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Statement::ClassDeclaration(class) => {
                if class.id.as_ref().map(|id| id.name.as_str()) == Some(class_name) {
                    Some(class)
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(class) = class {
            if let Some(prop_def) = create_static_property(
                allocator,
                &ast,
                prop_name,
                allocator.alloc_str(&placeholder_expr),
            ) {
                class
                    .body
                    .body
                    .push(ClassElement::PropertyDefinition(prop_def));

                failed_properties.push(FailedProperty {
                    class_name: class_name.to_string(),
                    prop_name: prop_name.to_string(),
                    placeholder,
                    expr: expr.to_string(),
                });
            }
            return;
        }
    }
}

/// Returns (fac_success, def_success) indicating which properties were successfully added
fn add_properties_to_class_body<'a>(
    allocator: &'a Allocator,
    class: &mut Class<'a>,
    fac_expr_str: &'a str,
    definitions: &[(&str, &str)],
) -> (bool, Vec<String>) {
    // Returns (fac_ok, failed_definitions_names)
    let ast = AstBuilder::new(allocator);
    let mut f_ok = false;
    let mut f_ok = false;
    let mut failed_defs = Vec::new();

    // Find the last non-constructor method to insert static properties after it
    // This ensures ɵfac and ɵdir are at the very end, after all methods
    let mut insert_position = class.body.body.len();

    // Search from the end to find the last method (not constructor)
    for (idx, element) in class.body.body.iter().enumerate().rev() {
        if let ClassElement::MethodDefinition(method) = element {
            // Skip constructor, only count regular methods
            if method.kind != oxc_ast::ast::MethodDefinitionKind::Constructor {
                insert_position = idx + 1;
                break;
            }
        }
    }

    // If no regular methods found, find the last element of any type (constructor, property, etc.)
    // This handles classes with only constructor and properties
    if insert_position == class.body.body.len() {
        // Find the last element (could be constructor, property, etc.)
        for (idx, _) in class.body.body.iter().enumerate().rev() {
            insert_position = idx + 1;
            break;
        }
    }

    if !fac_expr_str.is_empty() {
        if let Some(mut prop_def) =
            create_static_property(allocator, &ast, "ɵfac", allocator.alloc_str(fac_expr_str))
        {
            prop_def.span = oxc_ast::ast::Span::default();
            class
                .body
                .body
                .insert(insert_position, ClassElement::PropertyDefinition(prop_def));
            f_ok = true;
            insert_position += 1; // Update for next insertion
        }
    }

    for (def_name, def_expr_str) in definitions {
        eprintln!(
            "[RUST_DEBUG] Attempting to add property: '{}' to class",
            def_name
        );
        if !def_expr_str.is_empty() {
            if let Some(mut prop_def) =
                create_static_property(allocator, &ast, def_name, allocator.alloc_str(def_expr_str))
            {
                eprintln!("[RUST_DEBUG] Successfully parsed property: '{}'", def_name);
                prop_def.span = oxc_ast::ast::Span::default();
                class
                    .body
                    .body
                    .insert(insert_position, ClassElement::PropertyDefinition(prop_def));
                insert_position += 1;
            } else {
                eprintln!("[RUST_DEBUG] Failed to parse property: '{}'", def_name);
                eprintln!("[RUST_DEBUG] Expression content: '{}'", def_expr_str);
                failed_defs.push(def_name.to_string());
            }
        }
    }

    (f_ok, failed_defs)
}

/// Create a PropertyDefinition by parsing the expression standalone, then building the property
/// This approach works around OXC parsing issues with complex expressions in class property context.
fn create_static_property<'a>(
    allocator: &'a Allocator,
    ast: &AstBuilder<'a>,
    prop_name: &str,
    expr_str: &'a str,
) -> Option<oxc_allocator::Box<'a, PropertyDefinition<'a>>> {
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::ptr;

    let template_str = format!("class _ {{ static {} = {}; }}", prop_name, expr_str);
    let template = allocator.alloc_str(&template_str);
    let source_type = SourceType::mjs();
    let parser = Parser::new(allocator, template, source_type);
    let parse_result = parser.parse();

    if !parse_result.errors.is_empty() {
    } else {
        // Find the class and extract PropertyDefinition
        for stmt in &parse_result.program.body {
            if let Statement::ClassDeclaration(class_decl) = stmt {
                for element in &class_decl.body.body {
                    if let ClassElement::PropertyDefinition(prop) = element {
                        return Some(prop.clone_in(allocator));
                    }
                }
            }
        }
    }

    // Fallback: Parse expression standalone and build PropertyDefinition manually
    // Wrap in parentheses to handle edge cases like IIFE
    let expr_template = allocator.alloc_str(&format!("({})", expr_str));
    let expr_parser = Parser::new(allocator, expr_template, source_type);
    let expr_parse_result = expr_parser.parse();

    if !expr_parse_result.errors.is_empty() {
    } else {
        // Extract the expression from the parsed program
        for stmt in &expr_parse_result.program.body {
            if let Statement::ExpressionStatement(expr_stmt) = stmt {
                // Got the expression, now build PropertyDefinition manually
                unsafe {
                    let expr_ptr = &expr_stmt.expression as *const Expression<'a>;
                    let expr_copy = ptr::read(expr_ptr);

                    // Create the identifier for property name
                    let key = PropertyKey::StaticIdentifier(
                        ast.alloc(ast.identifier_name(SPAN, ast.atom(prop_name))),
                    );

                    // Create PropertyDefinition with correct OXC API argument order:
                    // (span, type, decorators, key, type_annotation, value, computed, static, ...)
                    let prop_def = ast.alloc_property_definition(
                        SPAN,
                        PropertyDefinitionType::PropertyDefinition,
                        ast.vec(), // decorators
                        key,
                        None::<oxc_allocator::Box<'a, oxc_ast::ast::TSTypeAnnotation<'a>>>, // type_annotation
                        Some(expr_copy), // value: Option<Expression>
                        false,           // computed
                        true,            // static_
                        false,           // declare
                        false,           // r#override
                        false,           // optional
                        false,           // definite
                        false,           // readonly
                        None,            // accessibility
                    );

                    return Some(prop_def);
                }
            }
        }
    }

    // Third fallback: Strip PURE annotation and try again
    let stripped = if expr_str.starts_with("/*@__PURE__*/") {
        expr_str.trim_start_matches("/*@__PURE__*/").trim_start()
    } else {
        expr_str
    };

    let stripped_template = allocator.alloc_str(&format!(
        "class _ {{ static {} = {}; }}",
        prop_name, stripped
    ));
    let stripped_parser = Parser::new(allocator, stripped_template, source_type);
    let stripped_result = stripped_parser.parse();

    if stripped_result.errors.is_empty() {
        for stmt in &stripped_result.program.body {
            if let Statement::ClassDeclaration(class_decl) = stmt {
                for element in &class_decl.body.body {
                    if let ClassElement::PropertyDefinition(prop) = element {
                        if let PropertyKey::StaticIdentifier(id) = &prop.key {
                            if id.name.as_str() == prop_name {
                                return Some(prop.clone_in(allocator));
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Ensure required imports exist
fn ensure_imports<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    imports: &[(String, String)],
) {
    let ast = AstBuilder::new(allocator);

    for (alias, module_path) in imports {
        if let Some(symbol_name) = alias.strip_prefix("named:") {
            // Handle Named Import: import { Symbol } from 'module_path';
            let has_import = program.body.iter().any(|stmt| {
                if let Statement::ImportDeclaration(import) = stmt {
                    if import.source.value.as_str() == module_path {
                        if let Some(specifiers) = &import.specifiers {
                            return specifiers.iter().any(|s| match s {
                                ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                                    spec.local.name.as_str() == symbol_name
                                }
                                _ => false,
                            });
                        }
                    }
                }
                false
            });

            if !has_import {
                // Create: import { Symbol } from 'module_path';
                let local = ast.binding_identifier(SPAN, ast.atom(symbol_name));
                let imported = ModuleExportName::IdentifierName(
                    ast.identifier_name(SPAN, ast.atom(symbol_name)),
                );

                let import_specifier =
                    ast.import_specifier(SPAN, imported, local, ImportOrExportKind::Value);

                let source = ast.string_literal(SPAN, ast.atom(module_path), None);

                let import_decl = ast.module_declaration_import_declaration(
                    SPAN,
                    Some(ast.vec1(ImportDeclarationSpecifier::ImportSpecifier(
                        ast.alloc(import_specifier),
                    ))),
                    source,
                    None,
                    None::<oxc_allocator::Box<'_, WithClause<'_>>>,
                    ImportOrExportKind::Value,
                );

                // Insert after the last import statement
                let last_import_index = program
                    .body
                    .iter()
                    .rposition(|stmt| matches!(stmt, Statement::ImportDeclaration(_)))
                    .map(|idx| idx + 1)
                    .unwrap_or(0);

                let stmt = Statement::from(import_decl);
                program.body.insert(last_import_index, stmt);
            }
        } else {
            // Handle Namespace Import: import * as alias from 'module_path';
            let has_import = program.body.iter().any(|stmt| {
                if let Statement::ImportDeclaration(import) = stmt {
                    if import.source.value.as_str() == module_path {
                        if let Some(specifiers) = &import.specifiers {
                            return specifiers.iter().any(|s| {
                                matches!(s, ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns)
                                    if ns.local.name.as_str() == alias)
                            });
                        }
                    }
                }
                false
            });

            if !has_import {
                // Create: import * as alias from 'module_path';
                let local = ast.binding_identifier(SPAN, ast.atom(alias));
                let import_namespace = ast.import_namespace_specifier(SPAN, local);

                let source = ast.string_literal(SPAN, ast.atom(module_path), None);

                let import_decl = ast.module_declaration_import_declaration(
                    SPAN,
                    Some(
                        ast.vec1(ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                            ast.alloc(import_namespace),
                        )),
                    ),
                    source,
                    None,
                    None::<oxc_allocator::Box<'_, WithClause<'_>>>,
                    ImportOrExportKind::Value,
                );

                // Insert after the last import statement
                let last_import_index = program
                    .body
                    .iter()
                    .rposition(|stmt| matches!(stmt, Statement::ImportDeclaration(_)))
                    .map(|idx| idx + 1)
                    .unwrap_or(0);

                let stmt = Statement::from(import_decl);
                program.body.insert(last_import_index, stmt);
            }
        }
    }
}

/// Ensure import * as i0 from '@angular/core' exists
fn ensure_angular_core_import<'a>(allocator: &'a Allocator, program: &mut Program<'a>) {
    ensure_imports(
        allocator,
        program,
        &[("i0".to_string(), "@angular/core".to_string())],
    );
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_decorator_name_extraction() {
        // Test would go here
    }
}
