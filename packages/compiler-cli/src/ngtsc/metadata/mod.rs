use std::path::PathBuf;
use oxc_ast::ast::{Program, Declaration, ModuleDeclaration, Expression, ObjectPropertyKind, PropertyKey};
pub mod property_mapping;

pub use property_mapping::ClassPropertyMapping;
use crate::ngtsc::reflection::{ReflectionHost, TypeScriptReflectionHost, Decorator, ClassDeclaration};

/// Metadata collected for a directive within an NgModule's scope.
use angular_compiler::ml_parser::ast::Node as HtmlNode;

#[derive(Debug, Clone)]
pub struct DirectiveMetadata {
    pub name: String,
    pub selector: Option<String>,
    pub is_component: bool,
    pub inputs: ClassPropertyMapping,
    pub outputs: ClassPropertyMapping,
    pub export_as: Option<Vec<String>>,
    pub is_standalone: bool,
    pub is_signal: bool,
    pub template: Option<String>,
    pub template_url: Option<String>,
    pub styles: Option<Vec<String>>,
    pub style_urls: Option<Vec<String>>,
    pub imports: Option<Vec<String>>,
    pub template_ast: Option<Vec<HtmlNode>>,
    pub source_file: Option<PathBuf>,
    pub change_detection: Option<angular_compiler::core::ChangeDetectionStrategy>,
}

pub trait MetadataReader {
    fn get_directive_metadata(&self, program: &Program, path: &std::path::Path) -> Vec<DirectiveMetadata>;
}

pub struct OxcMetadataReader;

impl MetadataReader for OxcMetadataReader {
    fn get_directive_metadata(&self, program: &Program, path: &std::path::Path) -> Vec<DirectiveMetadata> {
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
                // Ensure it is a class declaration before querying generic host (or host could just return empty)
                if let Declaration::ClassDeclaration(class_decl) = decl {
                    // Use ReflectionHost to get decorators
                    let decorators = host.get_decorators_of_declaration(decl);
                    
                    for decorator in decorators {
                        if decorator.name == "Component" || decorator.name == "Directive" {
                             if let Some(metadata) = extract_directive_metadata(
                                 class_decl,
                                 &decorator,
                                 decorator.name == "Component",
                                 path,
                             ) {
                                 directives.push(metadata);
                             }
                        }
                    }
                }
            }
        }
        
        directives
    }
}

/// Extract directive metadata from a class declaration and its decorator.
pub fn extract_directive_metadata(
    class_decl: &ClassDeclaration,
    decorator: &Decorator,
    is_component: bool,
    source_file: &std::path::Path,
) -> Option<DirectiveMetadata> {
    let name = class_decl.id.as_ref().map(|id| id.name.to_string()).unwrap_or_default();
    
    let mut selector = None;
    let mut inputs = ClassPropertyMapping::new();
    let mut outputs = ClassPropertyMapping::new();
    let mut export_as = None;
    let mut is_standalone = true; // Default to true per user request
    let is_signal = false;
    let mut template = None;
    let mut template_url = None;
    let mut styles = None;
    let mut style_urls = None;
    let mut imports = None;
    let mut change_detection = None;

    // Scan class body for @Input and signals (input(), input.required())
    for element in &class_decl.body.body {
        if let oxc_ast::ast::ClassElement::PropertyDefinition(prop) = element {
            if let PropertyKey::StaticIdentifier(key) = &prop.key {
                let prop_name = key.name.as_str();

                // 1. Check for @Input decorator
                let mut is_input = false;
                let mut binding_name = prop_name.to_string();
                
                for decorator in &prop.decorators {
                    if let Expression::CallExpression(call) = &decorator.expression {
                        if let Expression::Identifier(ident) = &call.callee {
                            if ident.name == "Input" {
                                is_input = true;
                                // Check for alias: @Input('alias')
                                if let Some(arg) = call.arguments.first() {
                                    if let Some(Expression::StringLiteral(s)) = arg.as_expression() {
                                        binding_name = s.value.to_string();
                                    }
                                }
                            }
                        }
                    } else if let Expression::Identifier(ident) = &decorator.expression {
                        if ident.name == "Input" {
                            is_input = true;
                        }
                    }
                }

                if is_input {
                    inputs.insert(property_mapping::InputOrOutput {
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

                        // Check callee: input or input.required
                        match &call.callee {
                            Expression::Identifier(ident) if ident.name == "input" => {
                                is_signal_input = true;
                            },
                            Expression::StaticMemberExpression(member) => {
                                if let Expression::Identifier(obj) = &member.object {
                                    if obj.name == "input" && member.property.name == "required" {
                                        is_signal_input = true;
                                    }
                                }
                            },
                            _ => {}
                        }

                        if is_signal_input {
                            // Check options for alias: input(initial, { alias: '...' }) or input.required({ alias: '...' })
                            // For input(), options is 2nd arg. For input.required(), options is 1st arg.
                            
                            let options_arg = if call.callee.is_member_expression() {
                                // input.required(options?)
                                call.arguments.first()
                            } else {
                                // input(initial?, options?)
                                call.arguments.get(1)
                            };

                            if let Some(arg) = options_arg {
                                if let Some(Expression::ObjectExpression(obj)) = arg.as_expression() {
                                    for p in &obj.properties {
                                        if let ObjectPropertyKind::ObjectProperty(op) = p {
                                            if let PropertyKey::StaticIdentifier(k) = &op.key {
                                                if k.name == "alias" {
                                                    if let Expression::StringLiteral(s) = &op.value {
                                                        signal_alias = s.value.to_string();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            inputs.insert(property_mapping::InputOrOutput {
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

                for decorator in &prop.decorators {
                    if let Expression::CallExpression(call) = &decorator.expression {
                        if let Expression::Identifier(ident) = &call.callee {
                            if ident.name == "Output" {
                                is_output = true;
                                // Check for alias: @Output('alias')
                                if let Some(arg) = call.arguments.first() {
                                    if let Some(Expression::StringLiteral(s)) = arg.as_expression() {
                                        output_binding_name = s.value.to_string();
                                    }
                                }
                            }
                        }
                    } else if let Expression::Identifier(ident) = &decorator.expression {
                        if ident.name == "Output" {
                            is_output = true;
                        }
                    }
                }

                if is_output {
                    outputs.insert(property_mapping::InputOrOutput {
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
                            // output({ alias: '...' }) - options is 1st arg
                            if let Some(arg) = call.arguments.first() {
                                if let Some(Expression::ObjectExpression(obj)) = arg.as_expression() {
                                    for p in &obj.properties {
                                        if let ObjectPropertyKind::ObjectProperty(op) = p {
                                            if let PropertyKey::StaticIdentifier(k) = &op.key {
                                                if k.name == "alias" {
                                                    if let Expression::StringLiteral(s) = &op.value {
                                                        output_alias = s.value.to_string();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            outputs.insert(property_mapping::InputOrOutput {
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

    if let Some(arg) = decorator.args.as_ref().and_then(|args| args.first()) {
        if let Expression::ObjectExpression(obj_expr) = arg {
            for prop in &obj_expr.properties {
                if let ObjectPropertyKind::ObjectProperty(prop) = prop {
                    if let PropertyKey::StaticIdentifier(key) = &prop.key {
                        match key.name.as_str() {
                            "selector" => {
                                if let Expression::StringLiteral(val) = &prop.value {
                                    selector = Some(val.value.to_string());
                                }
                            },
                            "inputs" => {
                                if let Expression::ArrayExpression(arr) = &prop.value {
                                    for elem in &arr.elements {
                                        if let Some(expr) = elem.as_expression() {
                                            if let Expression::StringLiteral(s) = expr {
                                                let parts: Vec<&str> = s.value.split(':').map(|p| p.trim()).collect();
                                                let (class_prop, binding_prop) = if parts.len() == 2 {
                                                    (parts[0], parts[1])
                                                } else {
                                                    (s.value.as_str(), s.value.as_str())
                                                };
                                                
                                                inputs.insert(property_mapping::InputOrOutput {
                                                    class_property_name: class_prop.to_string(),
                                                    binding_property_name: binding_prop.to_string(),
                                                    is_signal: false,
                                                });
                                            }
                                        }
                                    }
                                } else if let Expression::ObjectExpression(obj) = &prop.value {
                                        for p in &obj.properties {
                                            if let ObjectPropertyKind::ObjectProperty(prop) = p {
                                                let key_name = match &prop.key {
                                                    PropertyKey::StaticIdentifier(ident) => Some(ident.name.to_string()),
                                                    PropertyKey::StringLiteral(s) => Some(s.value.to_string()),
                                                    _ => None
                                                };
                                                
                                                if let (Some(key), Expression::StringLiteral(val)) = (key_name, &prop.value) {
                                                    inputs.insert(property_mapping::InputOrOutput {
                                                        class_property_name: key,
                                                        binding_property_name: val.value.to_string(),
                                                        is_signal: false,
                                                    });
                                                }
                                            }
                                        }
                                }
                            },
                            "outputs" => {
                                if let Expression::ArrayExpression(arr) = &prop.value {
                                    for elem in &arr.elements {
                                        if let Some(expr) = elem.as_expression() {
                                            if let Expression::StringLiteral(s) = expr {
                                                    let parts: Vec<&str> = s.value.split(':').map(|p| p.trim()).collect();
                                                let (class_prop, binding_prop) = if parts.len() == 2 {
                                                    (parts[0], parts[1])
                                                } else {
                                                    (s.value.as_str(), s.value.as_str())
                                                };
                                                
                                                outputs.insert(property_mapping::InputOrOutput {
                                                    class_property_name: class_prop.to_string(),
                                                    binding_property_name: binding_prop.to_string(),
                                                    is_signal: false,
                                                });
                                            }
                                        }
                                    }
                                } else if let Expression::ObjectExpression(obj) = &prop.value {
                                        for p in &obj.properties {
                                            if let ObjectPropertyKind::ObjectProperty(prop) = p {
                                                let key_name = match &prop.key {
                                                    PropertyKey::StaticIdentifier(ident) => Some(ident.name.to_string()),
                                                    PropertyKey::StringLiteral(s) => Some(s.value.to_string()),
                                                    _ => None
                                                };
                                                
                                                if let (Some(key), Expression::StringLiteral(val)) = (key_name, &prop.value) {
                                                    outputs.insert(property_mapping::InputOrOutput {
                                                        class_property_name: key,
                                                        binding_property_name: val.value.to_string(),
                                                        is_signal: false,
                                                    });
                                                }
                                            }
                                        }
                                }
                            },
                            "exportAs" => {
                                if let Expression::StringLiteral(val) = &prop.value {
                                    export_as = Some(val.value.to_string().split(',').map(|s| s.trim().to_string()).collect());
                                }
                            },
                            "templateUrl" => {
                                if let Expression::StringLiteral(val) = &prop.value {
                                    template_url = Some(val.value.to_string());
                                }
                            },
                            "template" => {
                                if let Expression::StringLiteral(val) = &prop.value {
                                    template = Some(val.value.to_string());
                                }
                            },
                            "styleUrls" => {
                                if let Expression::ArrayExpression(arr) = &prop.value {
                                    let collected: Vec<String> = arr.elements.iter().filter_map(|e| {
                                        if let Some(expr) = e.as_expression() {
                                            if let Expression::StringLiteral(s) = expr {
                                                return Some(s.value.to_string());
                                            }
                                        }
                                        None
                                    }).collect();
                                    style_urls = Some(collected);
                                }
                            },
                            "styleUrl" => {
                                if let Expression::StringLiteral(val) = &prop.value {
                                    style_urls = Some(vec![val.value.to_string()]);
                                }
                            },
                            "styles" => {
                                if let Expression::ArrayExpression(arr) = &prop.value {
                                    let collected: Vec<String> = arr.elements.iter().filter_map(|e| {
                                        if let Some(expr) = e.as_expression() {
                                            if let Expression::StringLiteral(s) = expr {
                                                return Some(s.value.to_string());
                                            }
                                        }
                                        None
                                    }).collect();
                                    styles = Some(collected);
                                }
                            },
                            "imports" => {
                                // Presence of imports implies standalone: true, which is now default anyway
                                is_standalone = true;
                                if let Expression::ArrayExpression(arr) = &prop.value {
                                    let collected: Vec<String> = arr.elements.iter().filter_map(|e| {
                                        if let Some(expr) = e.as_expression() {
                                            if let Expression::Identifier(ident) = expr {
                                                return Some(ident.name.to_string());
                                            }
                                        }
                                        None
                                    }).collect();
                                    imports = Some(collected);
                                }
                            },
                            "changeDetection" => {
                                // Handle ChangeDetectionStrategy.OnPush or ChangeDetectionStrategy.Default
                                // It's usually a member expression like ChangeDetectionStrategy.OnPush
                                if let Expression::StaticMemberExpression(member) = &prop.value {
                                    if member.property.name == "OnPush" {
                                        change_detection = Some(angular_compiler::core::ChangeDetectionStrategy::OnPush);
                                    } else if member.property.name == "Default" {
                                        change_detection = Some(angular_compiler::core::ChangeDetectionStrategy::Default);
                                    }
                                } else if let Expression::NumericLiteral(num) = &prop.value {
                                    // Fallback for numeric literal (0 = OnPush, 1 = Default)
                                    if num.value as i32 == 0 {
                                        change_detection = Some(angular_compiler::core::ChangeDetectionStrategy::OnPush);
                                    } else {
                                        change_detection = Some(angular_compiler::core::ChangeDetectionStrategy::Default);
                                    }
                                }
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Some(DirectiveMetadata {
        name,
        selector,
        is_component,
        inputs,
        outputs,
        export_as,
        is_standalone,
        is_signal,
        template,
        template_url,
        styles,
        style_urls,
        imports, // Add this
        template_ast: None,
        source_file: Some(source_file.to_path_buf()),
        change_detection,
    })
}

#[cfg(test)]
mod selector_test;
