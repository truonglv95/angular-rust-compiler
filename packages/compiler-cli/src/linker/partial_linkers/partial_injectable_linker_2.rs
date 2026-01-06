use crate::linker::ast::AstNode;
use crate::linker::ast_value::AstObject;
use crate::linker::partial_linker::PartialLinker;
use angular_compiler::constant_pool::ConstantPool;
use angular_compiler::output::output_ast as o;
use angular_compiler::render3::r3_factory::{
    compile_factory_function, DepsOrInvalid, FactoryTarget, R3ConstructorFactoryMetadata,
    R3FactoryMetadata,
};
use angular_compiler::render3::r3_identifiers::Identifiers as R3;
use angular_compiler::render3::util::R3Reference;

pub struct PartialInjectableLinker2;

impl PartialInjectableLinker2 {
    pub fn new() -> Self {
        Self
    }
}

impl<TExpression: AstNode> PartialLinker<TExpression> for PartialInjectableLinker2 {
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

        // Create R3FactoryMetadata to generate the factory function
        // For simple classes, deps are usually handled via constructor or reflection in JIT,
        // but in partial definition they are explicit.

        // TODO: Parse actual deps from metadata if available
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

                        parsed_deps.push(
                            angular_compiler::render3::r3_factory::R3DependencyMetadata {
                                token,
                                attribute_name_type,
                                host,
                                optional,
                                self_,
                                skip_self,
                            },
                        );
                    }
                }
                Some(DepsOrInvalid::Valid(parsed_deps))
            } else {
                Some(DepsOrInvalid::Valid(vec![]))
            }
        } else {
            None
        };

        // Construct ɵɵdefineInjectable definition map
        let mut definition_entries = vec![];

        // token: type
        definition_entries.push(o::LiteralMapEntry {
            key: "token".to_string(),
            value: Box::new(type_ref.value.clone()),
            quoted: false,
        });

        // factory: Check for useFactory first, then deps, then fallback to ɵfac
        let factory_fn = if meta_obj.has("useFactory") {
            // useFactory specified - wrap and invoke as IIFE
            // Angular partial declaration has useFactory which is the factory function itself
            // We need to generate: factory: () => (useFactory)()
            // This creates a proper IIFE that calls the factory function
            if let Ok(use_factory_val) = meta_obj.get_value("useFactory") {
                let factory_code = meta_obj.host.print_node(&use_factory_val.node);

                // Always wrap useFactory in parentheses and invoke as IIFE
                // Example: useFactory: () => inject(Foo) -> factory: () => (() => inject(Foo))()
                // Example: useFactory: createLocation -> factory: () => (createLocation)()
                o::Expression::ArrowFn(o::ArrowFunctionExpr {
                    params: vec![],
                    body: o::ArrowFunctionBody::Expression(Box::new(o::Expression::InvokeFn(
                        o::InvokeFunctionExpr {
                            fn_: Box::new(o::Expression::RawCode(o::RawCodeExpr {
                                // Wrap in parentheses to create proper IIFE
                                code: format!("({})", factory_code),
                                source_span: None,
                            })),
                            args: vec![],
                            type_: None,
                            source_span: None,
                            pure: false,
                        },
                    ))),
                    type_: None,
                    source_span: None,
                })
            } else {
                // Fallback to ɵfac if can't parse useFactory
                o::Expression::ReadProp(o::ReadPropExpr {
                    receiver: Box::new(type_ref.value.clone()),
                    name: "ɵfac".to_string(),
                    type_: None,
                    source_span: None,
                })
            }
        } else if deps.is_some() {
            let factory_meta = R3FactoryMetadata::Constructor(R3ConstructorFactoryMetadata {
                name: type_str.clone(),
                type_: type_ref.clone(),
                type_argument_count: 0,
                deps,
                target: FactoryTarget::Injectable,
            });
            compile_factory_function(&factory_meta).expression
        } else {
            // deps not specified, no useFactory - just reference class's own ɵfac directly
            // This is exactly what Angular linker does:
            //   factory: _PathLocationStrategy.ɵfac
            o::Expression::ReadProp(o::ReadPropExpr {
                receiver: Box::new(type_ref.value.clone()),
                name: "ɵfac".to_string(),
                type_: None,
                source_span: None,
            })
        };

        // factory: factory_fn
        definition_entries.push(o::LiteralMapEntry {
            key: "factory".to_string(),
            value: Box::new(factory_fn),
            quoted: false,
        });

        // providedIn
        if meta_obj.has("providedIn") {
            if let Ok(val) = meta_obj.get_value("providedIn") {
                // If it's a string literal, use it
                // If it's an identifier, wrap it
                if val.is_string() {
                    definition_entries.push(o::LiteralMapEntry {
                        key: "providedIn".to_string(),
                        value: Box::new(o::Expression::Literal(o::LiteralExpr {
                            value: o::LiteralValue::String(val.get_string().unwrap()),
                            type_: None,
                            source_span: None,
                        })),
                        quoted: false,
                    });
                } else {
                    // Assume expressions (like 'root' or valid identifier)
                    let s = val.host.print_node(&val.node);
                    // If it prints as "root", make it a string literal "root" IF it was a identifier??
                    // No, generally providedIn: 'root' comes as string literal in AST.
                    // If providedIn: SomeModule, it comes as identifier.
                    // We should trust the AST parsing.
                    // But AST host print_node might return source string.
                    // If it's a string literal "root", output should be "root".

                    // Optimization: check if it looks like a string in source (quoted)
                    definition_entries.push(o::LiteralMapEntry {
                        key: "providedIn".to_string(),
                        value: Box::new(o::Expression::ReadVar(o::ReadVarExpr {
                            name: s.replace("\"", "").replace("'", ""), // Hacky cleanup if strictly "root"
                            type_: None,
                            source_span: None,
                        })),
                        quoted: false,
                    });
                }
            }
        }

        let define_injectable_call = o::Expression::InvokeFn(o::InvokeFunctionExpr {
            fn_: Box::new(o::Expression::External(o::ExternalExpr {
                value: R3::define_injectable(),
                type_: None,
                source_span: None,
            })),
            args: vec![o::Expression::LiteralMap(o::LiteralMapExpr {
                entries: definition_entries,
                type_: None,
                source_span: None,
            })],
            type_: None,
            source_span: None,
            pure: true,
        });

        define_injectable_call
    }
}
