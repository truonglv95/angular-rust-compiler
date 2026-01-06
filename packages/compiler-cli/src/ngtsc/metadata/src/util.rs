//! Utility functions for metadata extraction.
//!
//! This module contains functions for extracting Angular metadata from TypeScript AST.
//! Matches TypeScript's util.ts

use crate::ngtsc::imports::OwningModule;
use oxc_ast::ast::Program;
use oxc_ast::ast::{Declaration, Expression, ModuleDeclaration, ObjectPropertyKind, PropertyKey};
use std::collections::HashMap;

use super::api::{
    ComponentMetadata, DecoratorMetadata, DirectiveMeta, DirectiveTypeCheckMeta, InjectableMeta,
    MatchSource, MetaKind, PipeMeta, Reference, T2DirectiveMetadata,
};
use super::property_mapping::{DecoratorInputTransform, InputOrOutput};
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
    imports_map: &HashMap<String, String>,
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
        file_imports: Some(imports_map.clone()),
        ..Default::default()
    };

    // Extract constructor parameters
    for element in &class_decl.body.body {
        if let oxc_ast::ast::ClassElement::MethodDefinition(method) = element {
            if method.kind == oxc_ast::ast::MethodDefinitionKind::Constructor {
                for param in &method.value.params.items {
                    let param_name = match &param.pattern.kind {
                        oxc_ast::ast::BindingPatternKind::BindingIdentifier(id) => {
                            Some(id.name.to_string())
                        }
                        _ => None,
                    };

                    // Extract type name from type annotation
                    let type_name =
                        param.pattern.type_annotation.as_ref().and_then(|ann| {
                            match &ann.type_annotation {
                                oxc_ast::ast::TSType::TSTypeReference(ref_type) => {
                                    match &ref_type.type_name {
                                        oxc_ast::ast::TSTypeName::IdentifierReference(ident) => {
                                            Some(ident.name.to_string())
                                        }
                                        _ => None,
                                    }
                                }
                                _ => None,
                            }
                        });

                    // Try to determine module from imports (simplified - would need full import analysis)
                    let from_module = type_name.as_ref().and_then(|tn| match tn.as_str() {
                        "ElementRef" | "Renderer2" | "Injector" | "ChangeDetectorRef" => {
                            Some("@angular/core".to_string())
                        }
                        "NgControl" | "FormControl" | "FormGroup" => {
                            Some("@angular/forms".to_string())
                        }
                        _ => None,
                    });

                    let mut attribute = None;
                    let mut optional = false;
                    let mut host = false;
                    let mut self_ = false;
                    let mut skip_self = false;

                    for dec in &param.decorators {
                        if let Expression::CallExpression(call) = &dec.expression {
                            if let Expression::Identifier(ident) = &call.callee {
                                match ident.name.as_str() {
                                    "Optional" => optional = true,
                                    "Host" => host = true,
                                    "Self" => self_ = true,
                                    "SkipSelf" => skip_self = true,
                                    "Attribute" => {
                                        if let Some(arg) = call.arguments.first() {
                                            if let Some(Expression::StringLiteral(s)) =
                                                arg.as_expression()
                                            {
                                                attribute = Some(s.value.to_string());
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        } else if let Expression::Identifier(ident) = &dec.expression {
                            match ident.name.as_str() {
                                "Optional" => optional = true,
                                "Host" => host = true,
                                "Self" => self_ = true,
                                "SkipSelf" => skip_self = true,
                                _ => {}
                            }
                        }
                    }

                    meta.constructor_params.push(super::api::ConstructorParam {
                        name: param_name,
                        type_name,
                        from_module,
                        attribute,
                        optional,
                        host,
                        self_,
                        skip_self,
                    });
                }
                break; // Only one constructor
            }
        }
    }

    // Scan class body for @Input and signals (input(), input.required())
    for element in &class_decl.body.body {
        if let oxc_ast::ast::ClassElement::PropertyDefinition(prop) = element {
            if let PropertyKey::StaticIdentifier(key) = &prop.key {
                let prop_name = key.name.as_str();

                // 1. Check for @Input decorator
                let mut is_input = false;
                let mut binding_name = prop_name.to_string();
                let mut transform_info: Option<DecoratorInputTransform> = None;

                for dec in &prop.decorators {
                    if let Expression::CallExpression(call) = &dec.expression {
                        if let Expression::Identifier(ident) = &call.callee {
                            if ident.name == "Input" {
                                is_input = true;
                                if let Some(arg) = call.arguments.first() {
                                    if let Some(expr) = arg.as_expression() {
                                        match expr {
                                            Expression::StringLiteral(s) => {
                                                binding_name = s.value.to_string();
                                            }
                                            Expression::ObjectExpression(obj) => {
                                                for p in &obj.properties {
                                                    if let ObjectPropertyKind::ObjectProperty(
                                                        prop,
                                                    ) = p
                                                    {
                                                        let key_name = match &prop.key {
                                                            PropertyKey::StaticIdentifier(id) => {
                                                                Some(id.name.as_str())
                                                            }
                                                            PropertyKey::StringLiteral(s) => {
                                                                Some(s.value.as_str())
                                                            }
                                                            _ => None,
                                                        };

                                                        if let Some(key) = key_name {
                                                            match key {
                                                                "alias" => {
                                                                    if let Expression::StringLiteral(s) = &prop.value {
                                                                        binding_name = s.value.to_string();
                                                                    }
                                                                }
                                                                "transform" => {
                                                                   let node_str = match &prop.value {
                                                                       Expression::Identifier(id) => id.name.to_string(),
                                                                       _ => "TRANSFORM_EXPR".to_string(),
                                                                   };
                                                                   transform_info = Some(DecoratorInputTransform {
                                                                       node: node_str.clone(),
                                                                       type_ref: node_str,
                                                                   });
                                                                }
                                                                _ => {}
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
                    } else if let Expression::Identifier(ident) = &dec.expression {
                        if ident.name == "Input" {
                            is_input = true;
                        }
                    }
                }

                if is_input {
                    // eprintln!("DEBUG: [util] Found @Input decorator on property: {}, binding: {}", prop_name, binding_name);
                    meta.t2.inputs.insert(InputOrOutput {
                        class_property_name: prop_name.to_string(),
                        binding_property_name: binding_name.clone(),
                        is_signal: false,
                        required: false,
                        transform: transform_info,
                    });
                }

                // 2. Check for signal input/model: input(), input.required(), model(), model.required()
                if let Some(value) = &prop.value {
                    if let Expression::CallExpression(call) = value {
                        let mut is_signal = false;
                        let mut is_model = false;
                        let mut is_required = false;
                        let mut alias = prop_name.to_string();

                        match &call.callee {
                            Expression::Identifier(ident) => {
                                if ident.name == "input" {
                                    is_signal = true;
                                } else if ident.name == "model" {
                                    is_signal = true;
                                    is_model = true;
                                }
                            }
                            Expression::StaticMemberExpression(member) => {
                                if let Expression::Identifier(obj) = &member.object {
                                    if (obj.name == "input" || obj.name == "model")
                                        && member.property.name == "required"
                                    {
                                        is_signal = true;
                                        is_required = true;
                                        if obj.name == "model" {
                                            is_model = true;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }

                        if is_signal {
                            // Extract options from arguments
                            // input(initialValue, options) or input.required(options)
                            // model(initialValue, options) or model.required(options)

                            let options_arg = if is_required {
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
                                                    if let Some(val) =
                                                        extract_string_value(&op.value)
                                                    {
                                                        alias = val;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            meta.t2.inputs.insert(InputOrOutput {
                                class_property_name: prop_name.to_string(),
                                binding_property_name: alias.clone(),
                                is_signal: true,
                                required: is_required,
                                transform: None, // Parsing signal inputs with transform is a separate task
                            });

                            if is_model {
                                meta.t2.outputs.insert(InputOrOutput {
                                    class_property_name: prop_name.to_string(),
                                    binding_property_name: format!("{}Change", alias),
                                    is_signal: false, // Model outputs are regular outputs event-wise
                                    required: false,
                                    transform: None,
                                });
                            }
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
                    // eprintln!("DEBUG: [util] Found @Output decorator on property: {}, binding: {}", prop_name, output_binding_name);
                    meta.t2.outputs.insert(InputOrOutput {
                        class_property_name: prop_name.to_string(),
                        binding_property_name: output_binding_name.clone(),
                        is_signal: false,
                        required: false,
                        transform: None,
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
                                required: false,
                                transform: None,
                            });
                        }
                    }
                }

                // 5. Check for signal queries (viewChild, contentChild, etc.)
                if let Some(value) = &prop.value {
                    if let Expression::CallExpression(call) = value {
                        let mut query_name = None;
                        let mut is_required = false;

                        match &call.callee {
                            Expression::Identifier(ident) => {
                                query_name = Some(ident.name.as_str());
                            }
                            Expression::StaticMemberExpression(member) => {
                                if let Expression::Identifier(obj) = &member.object {
                                    if member.property.name == "required" {
                                        query_name = Some(obj.name.as_str());
                                        is_required = true;
                                    }
                                }
                            }
                            _ => {}
                        }

                        if let Some(name) = query_name {
                            let (is_query, is_view, first) = match name {
                                "viewChild" => (true, true, true),
                                "viewChildren" => (true, true, false), // .required() for viewChildren? maybe
                                "contentChild" => (true, false, true),
                                "contentChildren" => (true, false, false),
                                _ => (false, false, false),
                            };

                            if is_query {
                                // Extract selector (first argument, required)
                                let mut selector: Option<String> = None;

                                if let Some(arg) = call.arguments.first() {
                                    match arg.as_expression() {
                                        Some(Expression::StringLiteral(s)) => {
                                            selector = Some(s.value.to_string());
                                        }
                                        // Handle options object as first arg?
                                        // viewChild(options) is possible if options has 'read', but selector is usually mandatory/implicit.
                                        // For now support string selector.
                                        _ => {}
                                    }
                                }

                                if let Some(sel) = selector {
                                    eprintln!("DEBUG: Found signal query: prop={}, selector={}, is_signal=true, is_required={}", prop_name, sel, is_required);
                                    let query_meta = super::api::QueryMetadata {
                                        property_name: prop_name.to_string(),
                                        selector: sel,
                                        first,
                                        descendants: true, // Default for signals?
                                        is_static: false,  // Signals are dynamic
                                        read: None,
                                        is_signal: true,
                                    };

                                    if is_view {
                                        meta.view_queries.push(query_meta);
                                    } else {
                                        meta.queries.push(query_meta);
                                    }
                                }
                            }
                        }
                    }
                }

                // 6. Check for @ViewChild / @ContentChild decorators
                for dec in &prop.decorators {
                    if let Expression::CallExpression(call) = &dec.expression {
                        if let Expression::Identifier(ident) = &call.callee {
                            let name = ident.name.as_str();
                            let (is_query, is_view, first) = match name {
                                "ViewChild" => (true, true, true),
                                "ViewChildren" => (true, true, false),
                                "ContentChild" => (true, false, true),
                                "ContentChildren" => (true, false, false),
                                _ => (false, false, false),
                            };

                            if is_query {
                                // Extract the selector (first argument)
                                let mut selector: Option<String> = None;
                                if let Some(arg) = call.arguments.first() {
                                    selector = match arg.as_expression() {
                                        Some(Expression::StringLiteral(s)) => {
                                            Some(s.value.to_string())
                                        }
                                        _ => None,
                                    };
                                }

                                // Extract options (second argument)
                                let mut read = None;
                                let mut descendants = !first;

                                if let Some(arg) = call.arguments.get(1) {
                                    if let Some(Expression::ObjectExpression(obj)) =
                                        arg.as_expression()
                                    {
                                        for p in &obj.properties {
                                            if let ObjectPropertyKind::ObjectProperty(prop) = p {
                                                let key = match &prop.key {
                                                    PropertyKey::StaticIdentifier(id) => {
                                                        Some(id.name.as_str())
                                                    }
                                                    _ => None,
                                                };

                                                match key {
                                                    Some("read") => {
                                                        if let Expression::Identifier(id) =
                                                            &prop.value
                                                        {
                                                            read = Some(id.name.to_string());
                                                        }
                                                    }
                                                    Some("descendants") => {
                                                        if let Expression::BooleanLiteral(b) =
                                                            &prop.value
                                                        {
                                                            descendants = b.value;
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Some(sel) = selector {
                                    let query_meta = super::api::QueryMetadata {
                                        property_name: prop_name.to_string(),
                                        selector: sel,
                                        first,
                                        descendants,
                                        is_static: false, // TODO: Parse static option
                                        read,
                                        is_signal: false,
                                    };

                                    if is_view {
                                        meta.view_queries.push(query_meta);
                                    } else {
                                        meta.queries.push(query_meta);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if let oxc_ast::ast::ClassElement::MethodDefinition(method) = element {
            // Check for @HostListener decorator on methods
            if let PropertyKey::StaticIdentifier(method_key) = &method.key {
                let method_name = method_key.name.as_str();

                match method_name {
                    "ngOnInit" => meta.lifecycle.uses_on_init = true,
                    "ngOnChanges" => meta.lifecycle.uses_on_changes = true,
                    "ngDoCheck" => meta.lifecycle.uses_do_check = true,
                    "ngAfterContentInit" => meta.lifecycle.uses_after_content_init = true,
                    "ngAfterContentChecked" => meta.lifecycle.uses_after_content_checked = true,
                    "ngAfterViewInit" => meta.lifecycle.uses_after_view_init = true,
                    "ngAfterViewChecked" => meta.lifecycle.uses_after_view_checked = true,
                    "ngOnDestroy" => meta.lifecycle.uses_on_destroy = true,
                    _ => {}
                }

                for dec in &method.decorators {
                    let mut is_host_listener = false;
                    let mut event_name = method_name.to_string();
                    let mut args = Vec::new();

                    if let Expression::CallExpression(call) = &dec.expression {
                        if let Expression::Identifier(ident) = &call.callee {
                            if ident.name == "HostListener" {
                                is_host_listener = true;

                                // Extract event name (first argument)
                                if let Some(arg) = call.arguments.first() {
                                    if let Some(Expression::StringLiteral(s)) = arg.as_expression()
                                    {
                                        event_name = s.value.to_string();
                                    }
                                }

                                // Extract args (second argument, optional)
                                if let Some(arg) = call.arguments.get(1) {
                                    if let Some(Expression::ArrayExpression(arr)) =
                                        arg.as_expression()
                                    {
                                        for elem in &arr.elements {
                                            if let Some(expr) = elem.as_expression() {
                                                if let Expression::StringLiteral(s) = expr {
                                                    args.push(s.value.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Expression::Identifier(ident) = &dec.expression {
                        if ident.name == "HostListener" {
                            is_host_listener = true;
                            // No arguments, use method name as event name
                        }
                    }

                    if is_host_listener {
                        // Build handler expression: methodName(args...)
                        let handler_expr = if args.is_empty() {
                            format!("{}($event)", method_name)
                        } else {
                            // Replace $event.target, $event, etc. with actual args
                            let args_str = args.join(", ");
                            format!("{}({})", method_name, args_str)
                        };

                        meta.host.listeners.insert(event_name, handler_expr);
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
                                                    required: false,
                                                    transform: None,
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
                                                        required: false,
                                                        transform: None,
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
                                                    required: false,
                                                    transform: None,
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
                                                        required: false,
                                                        transform: None,
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
                                                    let ident_name = ident.name.as_str();
                                                    let owning_module = imports_map
                                                        .get(ident_name)
                                                        .map(|specifier| {
                                                            OwningModule::new(
                                                                specifier.clone(),
                                                                source_file.to_string_lossy(),
                                                            )
                                                        });

                                                    if let Some(owning_module) = owning_module {
                                                        // We can't use with_owning_module directly because it doesn't take span/source_file manually
                                                        let mut r = Reference::from_name_with_span(
                                                            ident_name.to_string(),
                                                            Some(source_file.to_path_buf()),
                                                            ident.span,
                                                        );
                                                        r.best_guess_owning_module =
                                                            Some(owning_module);
                                                        return Some(r);
                                                    } else {
                                                        return Some(
                                                            Reference::from_name_with_span(
                                                                ident_name.to_string(),
                                                                Some(source_file.to_path_buf()),
                                                                ident.span,
                                                            ),
                                                        );
                                                    }
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
                                    // Legacy queries array parsing - disabled for now as we moved to Vec<QueryMetadata>
                                    // and this code was parsing Strings.
                                    /*
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
                                    */
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
                            "hostDirectives" => {
                                // TODO: Full parsing of HostDirectives array
                                // For now, we just acknowledge it exists to avoid errors if strict checking?
                                // Actually, we need to populate meta.host_directives
                                if let Expression::ArrayExpression(arr) = &prop.value {
                                    let mut directives = Vec::new();
                                    for elem in &arr.elements {
                                        if let Some(expr) = elem.as_expression() {
                                            // Can be Identifier (just the directive) or Object ({directive: ..., inputs: ..., outputs: ...})
                                            let mut directive_ref = None;
                                            let mut inputs = None;
                                            let mut outputs = None;

                                            match expr {
                                                Expression::Identifier(ident) => {
                                                    directive_ref =
                                                        Some(Reference::from_name_with_span(
                                                            ident.name.to_string(),
                                                            Some(source_file.to_path_buf()),
                                                            ident.span,
                                                        ));
                                                }
                                                Expression::ObjectExpression(obj) => {
                                                    for p in &obj.properties {
                                                        if let ObjectPropertyKind::ObjectProperty(
                                                            prop,
                                                        ) = p
                                                        {
                                                            let key = match &prop.key {
                                                                PropertyKey::StaticIdentifier(
                                                                    id,
                                                                ) => Some(id.name.as_str()),
                                                                _ => None,
                                                            };
                                                            match key {
                                                                 Some("directive") => {
                                                                     if let Expression::Identifier(ident) = &prop.value {
                                                                         directive_ref = Some(Reference::from_name_with_span(
                                                                             ident.name.to_string(),
                                                                             Some(source_file.to_path_buf()),
                                                                             ident.span,
                                                                         ));
                                                                     }
                                                                 }
                                                                 Some("inputs") => {
                                                                     if let Expression::ArrayExpression(arr) = &prop.value {
                                                                         let mut map = std::collections::HashMap::new();
                                                                         for elem in &arr.elements {
                                                                             if let Some(expr) = elem.as_expression() {
                                                                                 if let Expression::StringLiteral(s) = expr {
                                                                                     let val = s.value.as_str();
                                                                                     if let Some((left, right)) = val.split_once(':') {
                                                                                         map.insert(left.trim().to_string(), right.trim().to_string());
                                                                                     } else {
                                                                                         map.insert(val.to_string(), val.to_string());
                                                                                     }
                                                                                 }
                                                                             }
                                                                         }
                                                                         inputs = Some(map);
                                                                     }
                                                                 }
                                                                 Some("outputs") => {
                                                                     if let Expression::ArrayExpression(arr) = &prop.value {
                                                                         let mut map = std::collections::HashMap::new();
                                                                         for elem in &arr.elements {
                                                                             if let Some(expr) = elem.as_expression() {
                                                                                 if let Expression::StringLiteral(s) = expr {
                                                                                     let val = s.value.as_str();
                                                                                     if let Some((left, right)) = val.split_once(':') {
                                                                                         map.insert(left.trim().to_string(), right.trim().to_string());
                                                                                     } else {
                                                                                         map.insert(val.to_string(), val.to_string());
                                                                                     }
                                                                                 }
                                                                             }
                                                                         }
                                                                         outputs = Some(map);
                                                                     }
                                                                 }
                                                                 _ => {}
                                                             }
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }

                                            directives.push(super::api::HostDirectiveMeta {
                                                directive: directive_ref,
                                                is_forward_reference: false,
                                                inputs,
                                                outputs,
                                            });
                                        }
                                    }
                                    meta.host_directives = Some(directives);
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

    // 1. Build imports map
    let mut imports_map = HashMap::new();
    for stmt in &program.body {
        if let Some(mod_decl) = stmt.as_module_declaration() {
            if let ModuleDeclaration::ImportDeclaration(import_decl) = mod_decl {
                let source = import_decl.source.value.as_str();
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
                        imports_map.insert(local_name.to_string(), source.to_string());
                    }
                }
            }
        }
    }

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
                let decorators = host.get_decorators_of_declaration(decl);

                for decorator in decorators {
                    if decorator.name == "Component" || decorator.name == "Directive" {
                        if let Some(metadata) = extract_directive_metadata(
                            class_decl,
                            &decorator,
                            decorator.name == "Component",
                            path,
                            &imports_map,
                        ) {
                            directives.push(metadata);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ngtsc::reflection::{ReflectionHost, TypeScriptReflectionHost};
    use oxc_allocator::Allocator;
    use oxc_ast::ast;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use std::collections::HashMap;

    struct TestProgram<'a> {
        _allocator: &'a Allocator,
        program: ast::Program<'a>,
    }

    impl<'a> TestProgram<'a> {
        fn new(allocator: &'a Allocator, source: &'a str) -> Self {
            let source_type = SourceType::default()
                .with_typescript(true)
                .with_module(true);
            let ret = Parser::new(allocator, source, source_type).parse();

            if !ret.errors.is_empty() {
                panic!("Parse errors: {:?}", ret.errors);
            }

            Self {
                _allocator: allocator,
                program: ret.program,
            }
        }

        fn find_class(&self, name: &str) -> Option<&ast::Class<'a>> {
            for stmt in &self.program.body {
                if let ast::Statement::ExportNamedDeclaration(decl) = stmt {
                    if let Some(ast::Declaration::ClassDeclaration(class)) = &decl.declaration {
                        if let Some(id) = &class.id {
                            if id.name == name {
                                return Some(class);
                            }
                        }
                    }
                }
            }
            None
        }

        fn find_declaration(&self, name: &str) -> Option<&ast::Declaration<'a>> {
            for stmt in &self.program.body {
                if let ast::Statement::ExportNamedDeclaration(decl) = stmt {
                    if let Some(declaration) = &decl.declaration {
                        if let ast::Declaration::ClassDeclaration(class) = declaration {
                            if let Some(id) = &class.id {
                                if id.name == name {
                                    return Some(declaration);
                                }
                            }
                        }
                    }
                }
            }
            None
        }
    }

    #[test]
    fn test_extract_model_inputs() {
        let source = r#"
            import {Component, model, input} from '@angular/core';

            @Component({
                selector: 'test-comp',
                template: ''
            })
            export class TestComponent {
                // Standard model
                checked = model(false);
                
                // Model with alias
                maybe = model(0, {alias: 'val'});
                
                // Required model
                disabled = model.required<boolean>({alias: 'isDisabled'});
                
                // Regular input for comparison
                @Input() regular: string;
            }
        "#;

        let allocator = Allocator::default();
        let program = TestProgram::new(&allocator, source);
        let class_decl = program
            .find_class("TestComponent")
            .expect("Class not found");

        let host = TypeScriptReflectionHost::new();
        // find declaration to get decorators
        let decl = program
            .find_declaration("TestComponent")
            .expect("Declaration not found");
        let decorators = host.get_decorators_of_declaration(decl);
        let decorator = decorators
            .iter()
            .find(|d| d.name == "Component")
            .expect("Component decorator not found");

        let path = std::path::Path::new("test.ts");
        let imports = HashMap::new(); // Empty imports map for this test

        let metadata = extract_directive_metadata(class_decl, decorator, true, path, &imports)
            .expect("Metadata extraction failed");

        if let DecoratorMetadata::Directive(dir) = metadata {
            // Verify 'checked' model
            let checked = dir
                .t2
                .inputs
                .get("checked")
                .expect("checked input not found");
            assert!(checked.is_signal);
            assert_eq!(checked.binding_property_name, "checked");
            assert!(!checked.required);

            let checked_out = dir
                .t2
                .outputs
                .get("checked")
                .expect("checked output not found");
            assert_eq!(checked_out.binding_property_name, "checkedChange");

            // Verify 'maybe' model with alias
            let maybe = dir.t2.inputs.get("maybe").expect("maybe input not found");
            assert!(maybe.is_signal);
            assert_eq!(maybe.binding_property_name, "val"); // Alias used
            assert!(!maybe.required);

            let maybe_out = dir.t2.outputs.get("maybe").expect("maybe output not found");
            assert_eq!(maybe_out.binding_property_name, "valChange"); // Alias + Change

            // Verify 'disabled' required model with alias
            let disabled = dir
                .t2
                .inputs
                .get("disabled")
                .expect("disabled input not found");
            assert!(disabled.is_signal);
            assert_eq!(disabled.binding_property_name, "isDisabled");
            assert!(disabled.required);

            let disabled_out = dir
                .t2
                .outputs
                .get("disabled")
                .expect("disabled output not found");
            assert_eq!(disabled_out.binding_property_name, "isDisabledChange");
        } else {
            panic!("Expected Directive metadata");
        }
    }

    #[test]
    fn test_extract_signal_queries() {
        let source = r#"
            import {Component, viewChild, viewChildren, contentChild, contentChildren, ViewChild} from '@angular/core';

            @Component({
                selector: 'test-comp',
                template: ''
            })
            export class TestComponent {
                // Signal View Child
                vChild = viewChild('vRef');
                
                // Signal View Children
                vChildren = viewChildren('vRef');
                
                // Signal Content Child
                cChild = contentChild('cRef');
                
                // Signal Content Children
                cChildren = contentChildren('cRef');
                
                // Decorator View Child (Legacy)
                @ViewChild('decRef') decChild: any;
            }
        "#;

        let allocator = Allocator::default();
        let test_program = TestProgram::new(&allocator, source);
        let class_decl = test_program.find_class("TestComponent").unwrap();

        let host = TypeScriptReflectionHost::new();
        // find declaration to get decorators
        let decl = test_program
            .find_declaration("TestComponent")
            .expect("Declaration not found");
        let decorators = host.get_decorators_of_declaration(decl);
        let decorator = decorators
            .iter()
            .find(|d| d.name == "Component")
            .expect("Component decorator not found");

        let path = std::path::Path::new("test.ts");
        let imports = HashMap::new(); // Empty imports map for this test

        let meta = extract_directive_metadata(
            class_decl, decorator, true, // is_component
            path, &imports,
        )
        .expect("Metadata extraction failed");

        if let DecoratorMetadata::Directive(dir) = meta {
            // Verify Signal View Child
            let v_child = dir
                .view_queries
                .iter()
                .find(|q| q.property_name == "vChild")
                .expect("vChild not found");
            assert_eq!(v_child.selector, "vRef");
            assert!(v_child.first);
            assert!(v_child.is_signal);

            // Verify Signal View Children
            let v_children = dir
                .view_queries
                .iter()
                .find(|q| q.property_name == "vChildren")
                .expect("vChildren not found");
            assert_eq!(v_children.selector, "vRef");
            assert!(!v_children.first);
            assert!(v_children.is_signal);

            // Verify Signal Content Child
            let c_child = dir
                .queries
                .iter()
                .find(|q| q.property_name == "cChild")
                .expect("cChild not found");
            assert_eq!(c_child.selector, "cRef");
            assert!(c_child.first);
            assert!(c_child.is_signal);

            // Verify Signal Content Children
            let c_children = dir
                .queries
                .iter()
                .find(|q| q.property_name == "cChildren")
                .expect("cChildren not found");
            assert_eq!(c_children.selector, "cRef");
            assert!(!c_children.first);
            assert!(c_children.is_signal);

            // Verify Decorator View Child
            let dec_child = dir
                .view_queries
                .iter()
                .find(|q| q.property_name == "decChild")
                .expect("decChild not found");
            assert_eq!(dec_child.selector, "decRef");
            assert!(dec_child.first);
            assert!(!dec_child.is_signal);
        } else {
            panic!("Expected Directive metadata");
        }
    }

    #[test]
    fn test_extract_host_directives() {
        let source = r#"
            import {Component, Directive} from '@angular/core';

            @Directive({
                selector: 'host-dir',
                standalone: true
            })
            export class HostDir {}

            @Component({
                selector: 'test-comp',
                template: '',
                hostDirectives: [
                    HostDir,
                    {
                        directive: HostDir,
                        inputs: ['input1: alias1', 'input2'],
                        outputs: ['output1: alias1', 'output2']
                    }
                ]
            })
            export class TestComponent {}
        "#;

        let allocator = Allocator::default();
        let program = TestProgram::new(&allocator, source);
        let class_decl = program
            .find_class("TestComponent")
            .expect("Class not found");

        let host = TypeScriptReflectionHost::new();
        let decl = program
            .find_declaration("TestComponent")
            .expect("Declaration not found");
        let decorators = host.get_decorators_of_declaration(decl);
        let decorator = decorators
            .iter()
            .find(|d| d.name == "Component")
            .expect("Component decorator not found");

        let path = std::path::Path::new("test.ts");
        let imports = HashMap::new();

        let metadata = extract_directive_metadata(class_decl, decorator, true, path, &imports)
            .expect("Metadata extraction failed");

        if let DecoratorMetadata::Directive(dir) = metadata {
            let host_dirs = dir.host_directives.expect("hostDirectives not found");
            assert_eq!(host_dirs.len(), 2);

            // First: HostDir (simple)
            let hd1 = &host_dirs[0];
            assert_eq!(hd1.directive.as_ref().unwrap().debug_name(), "HostDir");
            assert!(hd1.inputs.is_none());
            assert!(hd1.outputs.is_none());

            // Second: Object with inputs/outputs
            let hd2 = &host_dirs[1];
            assert_eq!(hd2.directive.as_ref().unwrap().debug_name(), "HostDir");

            let inputs = hd2.inputs.as_ref().expect("inputs not found");
            assert_eq!(inputs.get("input1"), Some(&"alias1".to_string()));
            assert_eq!(inputs.get("input2"), Some(&"input2".to_string()));

            let outputs = hd2.outputs.as_ref().expect("outputs not found");
            assert_eq!(outputs.get("output1"), Some(&"alias1".to_string()));
            assert_eq!(outputs.get("output2"), Some(&"output2".to_string()));
        } else {
            panic!("Expected Directive metadata");
        }
    }
}
