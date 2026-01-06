use crate::linker::ast::AstNode;
use crate::linker::ast_value::AstObject;
use crate::linker::partial_linker::PartialLinker;
use angular_compiler::constant_pool::ConstantPool;
use angular_compiler::output::output_ast as o;
use angular_compiler::render3::r3_factory::{
    compile_factory_function, DepsOrInvalid, FactoryTarget, R3ConstructorFactoryMetadata,
    R3DependencyMetadata, R3FactoryMetadata,
};
use angular_compiler::render3::util::R3Reference;

pub struct PartialFactoryLinker2;

impl PartialFactoryLinker2 {
    pub fn new() -> Self {
        Self
    }
}

impl<TExpression: AstNode> PartialLinker<TExpression> for PartialFactoryLinker2 {
    fn link_partial_declaration(
        &self,
        _constant_pool: &mut ConstantPool,
        meta_obj: &AstObject<TExpression>,
        _source_url: &str,
        _version: &str,
        _target_name: Option<&str>,
        _imports: Option<&std::collections::HashMap<String, String>>,
    ) -> o::Expression {
        // Extract type
        let type_expr = match meta_obj.get_value("type") {
            Ok(v) => v.node,
            Err(e) => {
                return o::Expression::Literal(o::LiteralExpr {
                    value: o::LiteralValue::String(format!("Error: {}", e)),
                    type_: None,
                    source_span: None,
                })
            }
        };

        let type_str = meta_obj.host.print_node(&type_expr);
        let wrapped_type = o::Expression::ReadVar(o::ReadVarExpr {
            name: type_str.clone(),
            type_: None,
            source_span: None,
        });

        let type_ref = R3Reference {
            value: wrapped_type.clone(),
            type_expr: wrapped_type,
        };

        // Extract target
        let target = if meta_obj.has("target") {
            // target is often an enum access like i0.ɵɵFactoryTarget.Injectable
            // First try to read as number
            if let Ok(val) = meta_obj.get_number("target") {
                match val as u32 {
                    0 => FactoryTarget::Directive,
                    1 => FactoryTarget::Component,
                    2 => FactoryTarget::Injectable,
                    3 => FactoryTarget::Pipe,
                    4 => FactoryTarget::NgModule,
                    _ => FactoryTarget::Injectable,
                }
            } else if let Ok(target_val) = meta_obj.get_value("target") {
                // Try to parse from expression string like "i0.ɵɵFactoryTarget.Directive"
                let target_str = meta_obj.host.print_node(&target_val.node);
                if target_str.contains("Directive") {
                    FactoryTarget::Directive
                } else if target_str.contains("Component") {
                    FactoryTarget::Component
                } else if target_str.contains("Pipe") {
                    FactoryTarget::Pipe
                } else if target_str.contains("NgModule") {
                    FactoryTarget::NgModule
                } else {
                    // Default to Injectable if unknown
                    FactoryTarget::Injectable
                }
            } else {
                FactoryTarget::Injectable
            }
        } else {
            FactoryTarget::Injectable
        };

        // Extract dependencies
        // deps can be:
        // - absent: None -> use inherited factory (getInheritedFactory)
        // - null: None -> use inherited factory (getInheritedFactory)
        // - array: Some(Valid(deps)) -> inject deps
        let deps = if meta_obj.has("deps") {
            // Check if deps is null (inherited deps)
            if let Ok(deps_val) = meta_obj.get_value("deps") {
                let deps_str = meta_obj.host.print_node(&deps_val.node);
                if deps_str == "null" {
                    // deps: null means inherit from base class
                    None
                } else if let Ok(deps_arr) = meta_obj.get_array("deps") {
                    let mut parsed_deps = Vec::new();
                    for dep_entry in deps_arr {
                        if let Ok(dep_obj) = dep_entry.get_object() {
                            // Each dep is { token: SomeToken, optional?: bool, self?: bool, ... }
                            let token = if dep_obj.has("token") {
                                if let Ok(token_val) = dep_obj.get_value("token") {
                                    let token_str = meta_obj.host.print_node(&token_val.node);
                                    // Use RawCodeExpr to preserve the token exactly as written
                                    // (e.g., "i0.NgZone" should not be treated as a single identifier)
                                    Some(o::Expression::RawCode(o::RawCodeExpr {
                                        code: token_str,
                                        source_span: None,
                                    }))
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            let optional = dep_obj.get_bool("optional").unwrap_or(false);
                            let self_ = dep_obj.get_bool("self").unwrap_or(false);
                            let skip_self = dep_obj.get_bool("skipSelf").unwrap_or(false);
                            let host = dep_obj.get_bool("host").unwrap_or(false);

                            // Check for attribute injection
                            let attribute_name_type = if dep_obj.has("attribute") {
                                if let Ok(attr_val) = dep_obj.get_value("attribute") {
                                    let attr_str = meta_obj.host.print_node(&attr_val.node);
                                    Some(o::Expression::Literal(o::LiteralExpr {
                                        value: o::LiteralValue::String(attr_str),
                                        type_: None,
                                        source_span: None,
                                    }))
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            parsed_deps.push(R3DependencyMetadata {
                                token,
                                attribute_name_type,
                                host,
                                optional,
                                self_,
                                skip_self,
                            });
                        }
                    }
                    Some(DepsOrInvalid::Valid(parsed_deps))
                } else {
                    // deps exists but is not an array and not null
                    Some(DepsOrInvalid::Valid(vec![]))
                }
            } else {
                None
            }
        } else {
            None
        };

        // Extract simple class name from type_str (e.g. "i0.MyComponent" -> "MyComponent")
        let simple_name = type_str.split('.').last().unwrap_or(&type_str).to_string();

        let meta = R3FactoryMetadata::Constructor(R3ConstructorFactoryMetadata {
            name: simple_name,
            type_: type_ref,
            type_argument_count: 0,
            deps,
            target,
        });

        let res = compile_factory_function(&meta);
        res.expression
    }
}
