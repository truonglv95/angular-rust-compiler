use crate::linker::partial_linker::PartialLinker;
use crate::linker::ast_value::{AstObject, AstValue};
use crate::linker::ast::AstNode;
use angular_compiler::constant_pool::ConstantPool;
use angular_compiler::output::output_ast as o;
use angular_compiler::render3::r3_factory::{
    compile_factory_function, R3FactoryMetadata, R3ConstructorFactoryMetadata, FactoryTarget, DepsOrInvalid, R3DependencyMetadata
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
    ) -> o::Expression {
        // Extract type
        let type_expr = match meta_obj.get_value("type") {
            Ok(v) => v.node,
            Err(e) => return o::Expression::Literal(o::LiteralExpr {
                value: o::LiteralValue::String(format!("Error: {}", e)),
                type_: None,
                source_span: None,
            }),
        };
        
        let type_str = meta_obj.host.print_node(&type_expr);
        let wrapped_type = o::Expression::ReadVar(o::ReadVarExpr {
            name: type_str,
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
             // We need to resolve this. 
             // Ideally we might want to check the numeric value if possible, or string.
             // But partial declaration usually sends the enum member access.
             // If we can't resolve it easily, we might default to Injectable or check known patterns.
             // For now let's assume it's Injectable (2) if complex, or try to read number.
             match meta_obj.get_number("target") {
                 Ok(val) => match val as u32 {
                     0 => FactoryTarget::Directive,
                     1 => FactoryTarget::Component,
                     2 => FactoryTarget::Injectable,
                     3 => FactoryTarget::Pipe,
                     4 => FactoryTarget::NgModule,
                     _ => FactoryTarget::Injectable,
                 },
                 Err(_) => FactoryTarget::Injectable, // Default/Fallback
             }
        } else {
            FactoryTarget::Injectable
        };

        // Extract dependencies
        let deps = if meta_obj.has("deps") {
            if let Ok(deps_arr) = meta_obj.get_array("deps") {
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
                Some(DepsOrInvalid::Valid(vec![]))
            }
        } else {
            None
        };

        let meta = R3FactoryMetadata::Constructor(R3ConstructorFactoryMetadata {
            name: "Factory".to_string(), // TODO: extract name from type string
            type_: type_ref,
            type_argument_count: 0, 
            deps,
            target,
        });

        let res = compile_factory_function(&meta);
        res.expression
    }
}
