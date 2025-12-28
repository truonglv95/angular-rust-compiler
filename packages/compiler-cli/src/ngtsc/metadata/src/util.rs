//! Utility functions for metadata extraction.
//!
//! This module contains functions for extracting Angular metadata from TypeScript AST.
//! Matches TypeScript's util.ts

use oxc_ast::ast::Program;
use oxc_ast::ast::{Declaration, Expression, ModuleDeclaration, ObjectPropertyKind, PropertyKey};

use super::api::{
    ComponentMetadata, DecoratorMetadata, DirectiveMeta, DirectiveTypeCheckMeta, InjectableMeta,
    MatchSource, MetaKind, PipeMeta, Reference, T2DirectiveMetadata,
};
use super::property_mapping::InputOrOutput;
use crate::ngtsc::reflection::{
    ClassDeclaration, Decorator, ReflectionHost, TypeScriptReflectionHost,
};

/// Extract directive metadata from a class declaration and its decorator.
/// The lifetime `'a` is tied to the OXC AST allocator.
pub fn extract_directive_metadata<'a>(
    class_decl: &'a ClassDeclaration<'a>,
    decorator: &Decorator<'a>,
    is_component: bool,
    source_file: &std::path::Path,
) -> Option<DecoratorMetadata<'a>> {
    let name = class_decl
        .id
        .as_ref()
        .map(|id| id.name.to_string())
        .unwrap_or_default();

    let mut meta = DirectiveMeta {
        kind: MetaKind::Directive,
        match_source: MatchSource::Selector,
        t2: T2DirectiveMetadata {
            name,
            is_component,
            ..Default::default()
        },
        component: if is_component {
            Some(ComponentMetadata::default())
        } else {
            None
        },
        is_standalone: true,
        source_file: Some(source_file.to_path_buf()),
        type_check: DirectiveTypeCheckMeta::default(),
        // Store the OXC decorator reference directly
        decorator: Some(decorator.node),
        ..Default::default()
    };

    // Scan class body for @Input and signals (input(), input.required())
    for element in &class_decl.body.body {
        if let oxc_ast::ast::ClassElement::PropertyDefinition(prop) = element {
            if let PropertyKey::StaticIdentifier(key) = &prop.key {
                let prop_name = key.name.as_str();

                // 1. Check for @Input decorator
                let mut is_input = false;
                let mut binding_name = prop_name.to_string();

                for dec in &prop.decorators {
                    if let Expression::CallExpression(call) = &dec.expression {
                        if let Expression::Identifier(ident) = &call.callee {
                            if ident.name == "Input" {
                                is_input = true;
                                if let Some(arg) = call.arguments.first() {
                                    if let Some(Expression::StringLiteral(s)) = arg.as_expression()
                                    {
                                        binding_name = s.value.to_string();
                                    }
                                }
                            }
                        }
                    } else if let Expression::Identifier(ident) = &dec.expression {
                        if ident.name == "Input" {
                            is_input = true;
                        }
                    }
                }

                if is_input {
                    meta.t2.inputs.insert(InputOrOutput {
                        class_property_name: prop_name.to_string(),
                        binding_property_name: binding_name.clone(),
                        is_signal: false,
                    });
                }

                // 2. Check for signal input: input() or input.required()
                if let Some(value) = &prop.value {
                    if let Expression::CallExpression(call) = value {
                        let mut is_signal_input = false;
                        let mut signal_alias = prop_name.to_string();

                        match &call.callee {
                            Expression::Identifier(ident) if ident.name == "input" => {
                                is_signal_input = true;
                            }
                            Expression::StaticMemberExpression(member) => {
                                if let Expression::Identifier(obj) = &member.object {
                                    if obj.name == "input" && member.property.name == "required" {
                                        is_signal_input = true;
                                    }
                                }
                            }
                            _ => {}
                        }

                        if is_signal_input {
                            let options_arg = if call.callee.is_member_expression() {
                                call.arguments.first()
                            } else {
                                call.arguments.get(1)
                            };

                            if let Some(arg) = options_arg {
                                if let Some(Expression::ObjectExpression(obj)) = arg.as_expression()
                                {
                                    for p in &obj.properties {
                                        if let ObjectPropertyKind::ObjectProperty(op) = p {
                                            if let PropertyKey::StaticIdentifier(k) = &op.key {
                                                if k.name == "alias" {
                                                    if let Expression::StringLiteral(s) = &op.value
                                                    {
                                                        signal_alias = s.value.to_string();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            meta.t2.inputs.insert(InputOrOutput {
                                class_property_name: prop_name.to_string(),
                                binding_property_name: signal_alias,
                                is_signal: true,
                            });
                        }
                    }
                }

                // 3. Check for @Output decorator
                let mut is_output = false;
                let mut output_binding_name = prop_name.to_string();

                for dec in &prop.decorators {
                    if let Expression::CallExpression(call) = &dec.expression {
                        if let Expression::Identifier(ident) = &call.callee {
                            if ident.name == "Output" {
                                is_output = true;
                                if let Some(arg) = call.arguments.first() {
                                    if let Some(Expression::StringLiteral(s)) = arg.as_expression()
                                    {
                                        output_binding_name = s.value.to_string();
                                    }
                                }
                            }
                        }
                    } else if let Expression::Identifier(ident) = &dec.expression {
                        if ident.name == "Output" {
                            is_output = true;
                        }
                    }
                }

                if is_output {
                    meta.t2.outputs.insert(InputOrOutput {
                        class_property_name: prop_name.to_string(),
                        binding_property_name: output_binding_name.clone(),
                        is_signal: false,
                    });
                }

                // 4. Check for output() signal-like function
                if let Some(value) = &prop.value {
                    if let Expression::CallExpression(call) = value {
                        let mut is_output_fn = false;
                        let mut output_alias = prop_name.to_string();

                        if let Expression::Identifier(ident) = &call.callee {
                            if ident.name == "output" {
                                is_output_fn = true;
                            }
                        }

                        if is_output_fn {
                            if let Some(arg) = call.arguments.first() {
                                if let Some(Expression::ObjectExpression(obj)) = arg.as_expression()
                                {
                                    for p in &obj.properties {
                                        if let ObjectPropertyKind::ObjectProperty(op) = p {
                                            if let PropertyKey::StaticIdentifier(k) = &op.key {
                                                if k.name == "alias" {
                                                    if let Expression::StringLiteral(s) = &op.value
                                                    {
                                                        output_alias = s.value.to_string();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            meta.t2.outputs.insert(InputOrOutput {
                                class_property_name: prop_name.to_string(),
                                binding_property_name: output_alias,
                                is_signal: true,
                            });
                        }
                    }
                }
            }
        }
    }

    // Parse decorator arguments
    if let Some(arg) = decorator.args.as_ref().and_then(|args| args.first()) {
        if let Expression::ObjectExpression(obj_expr) = arg {
            for prop in &obj_expr.properties {
                if let ObjectPropertyKind::ObjectProperty(prop) = prop {
                    if let PropertyKey::StaticIdentifier(key) = &prop.key {
                        match key.name.as_str() {
                            "selector" => {
                                if let Some(val) = extract_string_value(&prop.value) {
                                    meta.t2.selector = Some(val);
                                }
                            }
                            "inputs" => match &prop.value {
                                Expression::ArrayExpression(arr) => {
                                    for elem in &arr.elements {
                                        if let Some(expr) = elem.as_expression() {
                                            if let Expression::StringLiteral(s) = expr {
                                                let parts: Vec<&str> =
                                                    s.value.split(':').map(|p| p.trim()).collect();
                                                let (class_prop, binding_prop) = if parts.len() == 2
                                                {
                                                    (parts[0], parts[1])
                                                } else {
                                                    (s.value.as_str(), s.value.as_str())
                                                };

                                                meta.t2.inputs.insert(InputOrOutput {
                                                    class_property_name: class_prop.to_string(),
                                                    binding_property_name: binding_prop.to_string(),
                                                    is_signal: false,
                                                });
                                            }
                                        }
                                    }
                                }
                                Expression::ObjectExpression(obj) => {
                                    for p in &obj.properties {
                                        if let ObjectPropertyKind::ObjectProperty(p) = p {
                                            let class_prop = match &p.key {
                                                PropertyKey::StaticIdentifier(key) => {
                                                    Some(key.name.to_string())
                                                }
                                                PropertyKey::StringLiteral(key) => {
                                                    Some(key.value.to_string())
                                                }
                                                _ => None,
                                            };
                                            if let Some(cp) = class_prop {
                                                if let Expression::StringLiteral(val) = &p.value {
                                                    meta.t2.inputs.insert(InputOrOutput {
                                                        class_property_name: cp,
                                                        binding_property_name: val
                                                            .value
                                                            .to_string(),
                                                        is_signal: false,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            },
                            "outputs" => match &prop.value {
                                Expression::ArrayExpression(arr) => {
                                    for elem in &arr.elements {
                                        if let Some(expr) = elem.as_expression() {
                                            if let Expression::StringLiteral(s) = expr {
                                                let parts: Vec<&str> =
                                                    s.value.split(':').map(|p| p.trim()).collect();
                                                let (class_prop, binding_prop) = if parts.len() == 2
                                                {
                                                    (parts[0], parts[1])
                                                } else {
                                                    (s.value.as_str(), s.value.as_str())
                                                };

                                                meta.t2.outputs.insert(InputOrOutput {
                                                    class_property_name: class_prop.to_string(),
                                                    binding_property_name: binding_prop.to_string(),
                                                    is_signal: false,
                                                });
                                            }
                                        }
                                    }
                                }
                                Expression::ObjectExpression(obj) => {
                                    for p in &obj.properties {
                                        if let ObjectPropertyKind::ObjectProperty(p) = p {
                                            let class_prop = match &p.key {
                                                PropertyKey::StaticIdentifier(key) => {
                                                    Some(key.name.to_string())
                                                }
                                                PropertyKey::StringLiteral(key) => {
                                                    Some(key.value.to_string())
                                                }
                                                _ => None,
                                            };
                                            if let Some(cp) = class_prop {
                                                if let Expression::StringLiteral(val) = &p.value {
                                                    meta.t2.outputs.insert(InputOrOutput {
                                                        class_property_name: cp,
                                                        binding_property_name: val
                                                            .value
                                                            .to_string(),
                                                        is_signal: false,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            },
                            "exportAs" => {
                                if let Some(val) = extract_string_value(&prop.value) {
                                    meta.t2.export_as = Some(
                                        val.split(',').map(|s| s.trim().to_string()).collect(),
                                    );
                                }
                            }
                            "templateUrl" => {
                                if let Some(val) = extract_string_value(&prop.value) {
                                    if let Some(comp) = meta.component.as_mut() {
                                        comp.template_url = Some(val);
                                    }
                                }
                            }
                            "template" => {
                                if let Some(val) = extract_string_value(&prop.value) {
                                    if let Some(comp) = meta.component.as_mut() {
                                        comp.template = Some(val);
                                    }
                                }
                            }
                            "styleUrls" => {
                                if let Expression::ArrayExpression(arr) = &prop.value {
                                    let collected: Vec<String> = arr
                                        .elements
                                        .iter()
                                        .filter_map(|e| {
                                            if let Some(expr) = e.as_expression() {
                                                if let Expression::StringLiteral(s) = expr {
                                                    return Some(s.value.to_string());
                                                }
                                            }
                                            None
                                        })
                                        .collect();
                                    if let Some(comp) = meta.component.as_mut() {
                                        comp.style_urls = Some(collected);
                                    }
                                }
                            }
                            "styleUrl" => {
                                if let Some(val) = extract_string_value(&prop.value) {
                                    if let Some(comp) = meta.component.as_mut() {
                                        comp.style_urls = Some(vec![val]);
                                    }
                                }
                            }
                            "styles" => {
                                if let Expression::ArrayExpression(arr) = &prop.value {
                                    let collected: Vec<String> = arr
                                        .elements
                                        .iter()
                                        .filter_map(|e| {
                                            if let Some(expr) = e.as_expression() {
                                                if let Expression::StringLiteral(s) = expr {
                                                    return Some(s.value.to_string());
                                                }
                                            }
                                            None
                                        })
                                        .collect();
                                    if let Some(comp) = meta.component.as_mut() {
                                        comp.styles = Some(collected);
                                    }
                                }
                            }
                            "imports" => {
                                meta.is_standalone = true;
                                if let Expression::ArrayExpression(arr) = &prop.value {
                                    let collected: Vec<Reference> = arr
                                        .elements
                                        .iter()
                                        .filter_map(|e| {
                                            if let Some(expr) = e.as_expression() {
                                                if let Expression::Identifier(ident) = expr {
                                                    return Some(Reference::from_name_with_span(
                                                        ident.name.to_string(),
                                                        Some(source_file.to_path_buf()),
                                                        ident.span,
                                                    ));
                                                }
                                            }
                                            None
                                        })
                                        .collect();
                                    meta.imports = Some(collected);
                                }
                            }
                            "standalone" => {
                                if let Expression::BooleanLiteral(b) = &prop.value {
                                    meta.is_standalone = b.value;
                                }
                            }
                            "changeDetection" => {
                                if let Some(comp) = meta.component.as_mut() {
                                    if let Expression::StaticMemberExpression(member) = &prop.value
                                    {
                                        if member.property.name == "OnPush" {
                                            comp.change_detection = Some(angular_compiler::core::ChangeDetectionStrategy::OnPush);
                                        } else if member.property.name == "Default" {
                                            comp.change_detection = Some(angular_compiler::core::ChangeDetectionStrategy::Default);
                                        }
                                    } else if let Expression::NumericLiteral(num) = &prop.value {
                                        if num.value as i32 == 0 {
                                            comp.change_detection = Some(angular_compiler::core::ChangeDetectionStrategy::OnPush);
                                        } else {
                                            comp.change_detection = Some(angular_compiler::core::ChangeDetectionStrategy::Default);
                                        }
                                    }
                                }
                            }
                            "queries" => {
                                if let Expression::ArrayExpression(arr) = &prop.value {
                                    let collected: Vec<String> = arr
                                        .elements
                                        .iter()
                                        .filter_map(|e| {
                                            if let Some(expr) = e.as_expression() {
                                                if let Expression::StringLiteral(s) = expr {
                                                    return Some(s.value.to_string());
                                                }
                                            }
                                            None
                                        })
                                        .collect();
                                    meta.queries = collected;
                                }
                            }
                            "host" => {
                                if let Expression::ObjectExpression(obj) = &prop.value {
                                    for host_prop in &obj.properties {
                                        if let ObjectPropertyKind::ObjectProperty(p) = host_prop {
                                            let key = match &p.key {
                                                PropertyKey::StaticIdentifier(id) => {
                                                    Some(id.name.to_string())
                                                }
                                                PropertyKey::StringLiteral(s) => {
                                                    Some(s.value.to_string())
                                                }
                                                _ => None,
                                            };
                                            if let Some(key) = key {
                                                if let Some(val) = extract_string_value(&p.value) {
                                                    if key.starts_with('(') && key.ends_with(')') {
                                                        meta.host.listeners.insert(
                                                            key[1..key.len() - 1].to_string(),
                                                            val,
                                                        );
                                                    } else if key.starts_with('[')
                                                        && key.ends_with(']')
                                                    {
                                                        meta.host.properties.insert(
                                                            key[1..key.len() - 1].to_string(),
                                                            val,
                                                        );
                                                    } else if key == "class" {
                                                        meta.host.special_attributes.class_attr =
                                                            Some(val);
                                                    } else if key == "style" {
                                                        meta.host.special_attributes.style_attr =
                                                            Some(val);
                                                    } else {
                                                        meta.host.attributes.insert(
                                                            key,
                                                            *angular_compiler::output::output_ast::literal(
                                                                angular_compiler::output::output_ast::LiteralValue::String(val),
                                                            ),
                                                        );
                                                    }
                                                }
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
        }
    }

    Some(DecoratorMetadata::Directive(meta))
}

/// Extract pipe metadata from a class declaration and its @Pipe decorator.
pub fn extract_pipe_metadata<'a>(
    class_decl: &'a ClassDeclaration<'a>,
    decorator: &Decorator<'a>,
    source_file: &std::path::Path,
) -> Option<DecoratorMetadata<'a>> {
    let name = class_decl
        .id
        .as_ref()
        .map(|id| id.name.to_string())
        .unwrap_or_default();

    let mut meta = PipeMeta {
        kind: MetaKind::Pipe,
        name: name.clone(),
        pipe_name: name,
        source_file: Some(source_file.to_path_buf()),
        ..Default::default()
    };

    // Extract @Pipe({ name: '...', pure: ..., standalone: ... })
    if let Some(args) = &decorator.args {
        if let Some(first_arg) = args.first() {
            if let Expression::ObjectExpression(obj) = first_arg {
                for prop in &obj.properties {
                    if let ObjectPropertyKind::ObjectProperty(obj_prop) = prop {
                        let key = match &obj_prop.key {
                            PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
                            _ => None,
                        };

                        match key {
                            Some("name") => {
                                if let Expression::StringLiteral(s) = &obj_prop.value {
                                    meta.pipe_name = s.value.to_string();
                                }
                            }
                            Some("pure") => {
                                if let Expression::BooleanLiteral(b) = &obj_prop.value {
                                    meta.is_pure = b.value;
                                }
                            }
                            Some("standalone") => {
                                if let Expression::BooleanLiteral(b) = &obj_prop.value {
                                    meta.is_standalone = b.value;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Some(DecoratorMetadata::Pipe(meta))
}

/// Extract injectable metadata from a class declaration and its @Injectable decorator.
pub fn extract_injectable_metadata<'a>(
    class_decl: &'a ClassDeclaration<'a>,
    decorator: &Decorator<'a>,
    source_file: &std::path::Path,
) -> Option<DecoratorMetadata<'a>> {
    let name = class_decl
        .id
        .as_ref()
        .map(|id| id.name.to_string())
        .unwrap_or_default();

    let mut provided_in: Option<String> = None;

    // Extract @Injectable({ providedIn: '...' })
    if let Some(args) = &decorator.args {
        if let Some(first_arg) = args.first() {
            if let Expression::ObjectExpression(obj) = first_arg {
                for prop in &obj.properties {
                    if let ObjectPropertyKind::ObjectProperty(obj_prop) = prop {
                        let key = match &obj_prop.key {
                            PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
                            _ => None,
                        };

                        if key == Some("providedIn") {
                            if let Expression::StringLiteral(s) = &obj_prop.value {
                                provided_in = Some(s.value.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    Some(DecoratorMetadata::Injectable(InjectableMeta {
        name,
        provided_in,
        source_file: Some(source_file.to_path_buf()),
    }))
}

/// Get all Angular decorator metadata from a program.
/// The lifetime `'a` is tied to the OXC AST allocator.
pub fn get_all_metadata<'a>(
    program: &'a Program<'a>,
    path: &std::path::Path,
) -> Vec<DecoratorMetadata<'a>> {
    let mut directives = Vec::new();
    let host = TypeScriptReflectionHost::new();

    for stmt in &program.body {
        let declaration = if let Some(decl) = stmt.as_declaration() {
            Some(decl)
        } else if let Some(mod_decl) = stmt.as_module_declaration() {
            if let ModuleDeclaration::ExportNamedDeclaration(export_decl) = mod_decl {
                export_decl.declaration.as_ref()
            } else {
                None
            }
        } else {
            None
        };

        if let Some(decl) = declaration {
            if let Declaration::ClassDeclaration(class_decl) = decl {
                let class_name = class_decl
                    .id
                    .as_ref()
                    .map(|id| id.name.as_str())
                    .unwrap_or("<anonymous>");
                // println!("DEBUG: Metadata reader found class: {}", class_name);
                let decorators = host.get_decorators_of_declaration(decl);

                for decorator in decorators {
                    if decorator.name == "Component" || decorator.name == "Directive" {
                        if class_name == "HostBindingTest" {
                            println!(
                                "DEBUG: Found Component/Directive decorator for HostBindingTest"
                            );
                        }
                        if let Some(metadata) = extract_directive_metadata(
                            class_decl,
                            &decorator,
                            decorator.name == "Component",
                            path,
                        ) {
                            if class_name == "HostBindingTest" {
                                println!(
                                    "DEBUG: SUCCESSFULLY extracted metadata for HostBindingTest"
                                );
                            }
                            directives.push(metadata);
                        } else {
                            if class_name == "HostBindingTest" {
                                println!("DEBUG: FAILED to extract metadata for HostBindingTest");
                            }
                        }
                    } else if decorator.name == "Pipe" {
                        if let Some(metadata) = extract_pipe_metadata(class_decl, &decorator, path)
                        {
                            directives.push(metadata);
                        }
                    } else if decorator.name == "Injectable" {
                        if let Some(metadata) =
                            extract_injectable_metadata(class_decl, &decorator, path)
                        {
                            directives.push(metadata);
                        }
                    }
                }
            }
        }
    }

    directives
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
