//! AST-based Ivy transformation utilities
//!
//! This module provides functions to transform OXC AST for Angular Ivy compilation:
//! - Remove Angular decorators (@Component, @Directive, etc.)
//! - Add Ivy static properties (ɵfac, ɵcmp)
//! - Merge/add import statements

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::AstBuilder;
use oxc_span::SPAN;

/// Transform a parsed program to add Ivy compilation results
pub fn transform_component_ast<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    component_name: &str,
    hoisted_statements: &'a str,
    fac_expr_str: &'a str,
    def_expr_str: &'a str,
    def_name: &str,
) {
    // 1. Remove Angular decorators from the class
    remove_angular_decorators(program);

    // 2. Ensure namespace import exists (must be before hoisted statements)
    ensure_angular_core_import(allocator, program);

    // 3. Add hoisted statements to program body (after all imports)
    add_hoisted_statements(allocator, program, hoisted_statements);

    // 4. Add ɵfac and ɵcmp/ɵdir as static properties to the class
    add_static_properties_to_class(
        allocator,
        program,
        component_name,
        fac_expr_str,
        def_expr_str,
        def_name,
    );
}

/// Remove Angular decorators (@Component, @Directive, etc.) from class declarations
fn remove_angular_decorators(program: &mut Program) {
    for stmt in program.body.iter_mut() {
        match stmt {
            Statement::ExportDefaultDeclaration(export_decl) => {
                if let ExportDefaultDeclarationKind::ClassDeclaration(class) =
                    &mut export_decl.declaration
                {
                    remove_decorators_from_class(class);
                }
            }
            Statement::ExportNamedDeclaration(export_decl) => {
                if let Some(Declaration::ClassDeclaration(class)) = &mut export_decl.declaration {
                    remove_decorators_from_class(class);
                }
            }
            Statement::ClassDeclaration(class) => {
                remove_decorators_from_class(class);
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
        !class_decorators.contains(&decorator_name.as_str())
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

fn add_static_properties_to_class<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    component_name: &str,
    fac_expr_str: &'a str,
    def_expr_str: &'a str,
    def_name: &str,
) {
    println!(
        "DEBUG: add_static_properties_to_class for {}",
        component_name
    );
    // Find the class declaration and add static properties
    for stmt in program.body.iter_mut() {
        match stmt {
            Statement::ExportNamedDeclaration(export_decl) => {
                // Decorators are not on ExportNamedDeclaration in this version of OXC.
                // They are on the Declaration.

                if let Some(Declaration::ClassDeclaration(class)) = &mut export_decl.declaration {
                    if let Some(class_id) = &class.id {
                        if class_id.name.as_str() == component_name {
                            add_properties_to_class_body(
                                allocator,
                                class,
                                Some(fac_expr_str),
                                Some(def_expr_str),
                                def_name,
                            );
                            break;
                        }
                    }
                }
            }

            Statement::ClassDeclaration(class) => {
                if let Some(class_id) = &class.id {
                    println!("DEBUG: Found class {}", class_id.name);
                    if class_id.name.as_str() == component_name {
                        println!("DEBUG: Match found for {}", component_name);
                        add_properties_to_class_body(
                            allocator,
                            class,
                            Some(fac_expr_str),
                            Some(def_expr_str),
                            def_name,
                        );
                        break;
                    }
                }
            }
            _ => {}
        }
    }
}

fn add_properties_to_class_body<'a>(
    allocator: &'a Allocator,
    class: &mut Class<'a>,
    fac_expr_str: Option<&'a str>,
    def_expr_str: Option<&'a str>,
    def_name: &str,
) {
    let ast = AstBuilder::new(allocator);

    // Create ɵfac property if exists
    if let Some(expr_str) = fac_expr_str {
        if let Some(prop_def) = create_static_property(allocator, &ast, "ɵfac", expr_str) {
            class
                .body
                .body
                .push(ClassElement::PropertyDefinition(prop_def));
        }
    }

    // Create definition property (ɵcmp/ɵdir) if exists
    if let Some(expr_str) = def_expr_str {
        if let Some(prop_def) = create_static_property(allocator, &ast, def_name, expr_str) {
            class
                .body
                .body
                .push(ClassElement::PropertyDefinition(prop_def));
        }
    }
}

/// Create a PropertyDefinition by parsing a template class and extracting it
/// We parse a template class and extract the PropertyDefinition directly.
/// Since PropertyDefinition is allocated in the arena, it will survive after parse_result is dropped.
/// We use unsafe code to extract it, but this is safe because:
/// 1. The PropertyDefinition is allocated in the arena (not on the stack)
/// 2. The arena lifetime matches our return type lifetime
/// 3. We're extracting from a temporary parse result that will be dropped
fn create_static_property<'a>(
    allocator: &'a Allocator,
    _ast: &AstBuilder<'a>,
    prop_name: &str,
    expr_str: &'a str,
) -> Option<oxc_allocator::Box<'a, PropertyDefinition<'a>>> {
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::ptr;

    // Parse a template class: class _ { static prop_name = expr_str; }
    let template = allocator.alloc_str(&format!(
        "class _ {{ static {} = {}; }}",
        prop_name, expr_str
    ));
    let source_type = SourceType::mjs();
    let parser = Parser::new(allocator, template, source_type);
    let parse_result = parser.parse();

    if parse_result.errors.is_empty() {
        // Find the class and extract PropertyDefinition
        for stmt in &parse_result.program.body {
            if let Statement::ClassDeclaration(class_decl) = stmt {
                for element in &class_decl.body.body {
                    if let ClassElement::PropertyDefinition(prop) = element {
                        // PropertyDefinition is allocated in the arena, so we can safely extract it
                        // We'll use unsafe to extract the Box, but this is safe because:
                        // 1. The PropertyDefinition is in the arena, not in parse_result's stack
                        // 2. The lifetime is correct (both are 'a)
                        // 3. We're just moving the Box pointer, not the data
                        unsafe {
                            // Extract the PropertyDefinition by converting the reference to a raw pointer
                            // and then back to a Box. This is safe because the data is in the arena.
                            let prop_ptr =
                                prop as *const oxc_allocator::Box<'a, PropertyDefinition<'a>>;
                            return Some(ptr::read(prop_ptr));
                        }
                    }
                }
            }
        }
    }
    None
}

/// Ensure import * as i0 from '@angular/core' exists
fn ensure_angular_core_import<'a>(allocator: &'a Allocator, program: &mut Program<'a>) {
    let ast = AstBuilder::new(allocator);

    // Check if i0 namespace import already exists
    let has_i0_import = program.body.iter().any(|stmt| {
        if let Statement::ImportDeclaration(import) = stmt {
            if import.source.value.as_str() == "@angular/core" {
                if let Some(specifiers) = &import.specifiers {
                    return specifiers.iter().any(|s| {
                        matches!(s, ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns) 
                            if ns.local.name.as_str() == "i0")
                    });
                }
            }
        }
        false
    });

    if !has_i0_import {
        // Create: import * as i0 from '@angular/core';
        let i0_binding = ast.binding_identifier(SPAN, ast.atom("i0"));
        let namespace_specifier = ast.import_namespace_specifier(SPAN, i0_binding);

        let source = ast.string_literal(SPAN, ast.atom("@angular/core"), None);

        // module_declaration_import_declaration(span, specifiers, source, phase, with_clause, import_kind)
        let import_decl = ast.module_declaration_import_declaration(
            SPAN,
            Some(
                ast.vec1(ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                    ast.alloc(namespace_specifier),
                )),
            ),
            source,
            None,                                           // phase
            None::<oxc_allocator::Box<'_, WithClause<'_>>>, // with clause
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

#[cfg(test)]
mod tests {

    #[test]
    fn test_decorator_name_extraction() {
        // Test would go here
    }
}
