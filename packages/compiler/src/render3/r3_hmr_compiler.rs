//! Render3 HMR Compiler
//!
//! Corresponds to packages/compiler/src/render3/r3_hmr_compiler.ts
//! Contains Hot Module Replacement (HMR) compilation

use crate::output::output_ast::dynamic_type;
use crate::output::output_ast::{
    ArrowFunctionBody, ArrowFunctionExpr, BinaryOperator, BinaryOperatorExpr, DeclareFunctionStmt,
    DeclareVarStmt, DynamicImportExpr, Expression, ExternalExpr, ExternalReference, FnParam,
    InvokeFunctionExpr, LiteralArrayExpr, LiteralExpr, LiteralValue, ReadKeyExpr, ReadPropExpr,
    ReadVarExpr, Statement, StmtModifier, WritePropExpr,
};

use super::r3_identifiers::Identifiers as R3;
use super::util::dev_only_guarded_expression;

/// Helper to create external expression from ExternalReference
fn external_expr(reference: ExternalReference) -> Expression {
    Expression::External(ExternalExpr {
        value: reference,
        type_: None,
        source_span: None,
    })
}

/// Simple URI encoding helper (replaces urlencoding crate)
fn encode_uri_component(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z'
            | 'a'..='z'
            | '0'..='9'
            | '-'
            | '_'
            | '.'
            | '!'
            | '~'
            | '*'
            | '\''
            | '('
            | ')' => c.to_string(),
            c => c
                .encode_utf8(&mut [0; 4])
                .bytes()
                .map(|b| format!("%{:02X}", b))
                .collect::<String>(),
        })
        .collect()
}

/// Metadata necessary to compile HMR-related code
#[derive(Debug, Clone)]
pub struct R3HmrMetadata {
    /// Component class for which HMR is being enabled
    pub type_: Expression,
    /// Name of the component class
    pub class_name: String,
    /// File path of the component class
    pub file_path: String,
    /// Namespace dependencies (e.g. import * as i0 from '@angular/core')
    pub namespace_dependencies: Vec<Expression>,
    /// Local dependencies (the actual symbols like MatCardModule)
    pub local_dependencies: Vec<Expression>,
    /// Unique HMR ID
    pub hmr_id: String,
}

/// HMR dependency on a namespace import
#[derive(Debug, Clone)]
pub struct R3HmrNamespaceDependency {
    /// Module name of the import
    pub module_name: String,
    /// Name under which to refer to the namespace inside HMR-related code
    pub assigned_name: String,
}

/// Local dependency for HMR
#[derive(Debug, Clone)]
pub struct R3HmrLocalDependency {
    pub name: String,
    pub runtime_representation: Expression,
}

/// Compiles the expression that initializes HMR for a class
pub fn compile_hmr_initializer(meta: &R3HmrMetadata) -> Expression {
    let module_name = "m";
    let data_name = "d";
    let timestamp_name = "t";
    let id_name = "id";
    let import_callback_name = format!("{}_HmrLoad", meta.class_name);

    let namespaces: Vec<Expression> = meta.namespace_dependencies.clone();

    // m.default
    let default_read = Expression::ReadProp(ReadPropExpr {
        receiver: Box::new(Expression::ReadVar(ReadVarExpr {
            name: module_name.to_string(),
            type_: None,
            source_span: None,
        })),
        name: "default".to_string(),
        type_: None,
        source_span: None,
    });

    // Build locals array
    let locals_arr: Vec<Expression> = meta.local_dependencies.clone();

    // ɵɵreplaceMetadata(Comp, m.default, [...namespaces], [...locals], import.meta, id)
    let replace_metadata_ref = R3::replace_metadata();
    let replace_metadata_expr = external_expr(replace_metadata_ref);

    let replace_call = Expression::InvokeFn(InvokeFunctionExpr {
        fn_: Box::new(replace_metadata_expr),
        args: vec![
            meta.type_.clone(),
            default_read.clone(),
            Expression::LiteralArray(LiteralArrayExpr {
                entries: namespaces,
                type_: None,
                source_span: None,
            }),
            Expression::LiteralArray(LiteralArrayExpr {
                entries: locals_arr,
                type_: None,
                source_span: None,
            }),
            Expression::ReadProp(ReadPropExpr {
                receiver: Box::new(Expression::ReadVar(ReadVarExpr {
                    name: "import".to_string(),
                    type_: None,
                    source_span: None,
                })),
                name: "meta".to_string(),
                type_: None,
                source_span: None,
            }),
            Expression::ReadVar(ReadVarExpr {
                name: id_name.to_string(),
                type_: None,
                source_span: None,
            }),
        ],
        type_: None,
        source_span: None,
        pure: false,
    });

    // (m) => m.default && ɵɵreplaceMetadata(...)
    let replace_callback = Expression::ArrowFn(ArrowFunctionExpr {
        params: vec![FnParam {
            name: module_name.to_string(),
            type_: None,
        }],
        body: ArrowFunctionBody::Expression(Box::new(Expression::BinaryOp(BinaryOperatorExpr {
            operator: BinaryOperator::And,
            lhs: Box::new(default_read.clone()),
            rhs: Box::new(replace_call),
            type_: None,
            source_span: None,
        }))),
        type_: None,
        source_span: None,
    });

    let url_expr = Expression::ReadProp(ReadPropExpr {
        receiver: Box::new(Expression::ReadProp(ReadPropExpr {
            receiver: Box::new(Expression::ReadVar(ReadVarExpr {
                name: "import".to_string(),
                type_: None,
                source_span: None,
            })),
            name: "meta".to_string(),
            type_: None,
            source_span: None,
        })),
        name: "url".to_string(),
        type_: None,
        source_span: None,
    });

    // i0.ɵɵgetReplaceMetadataURL(id, t, import.meta.url)
    // This returns a special metadata-only URL: ./@ng/component?c=<id>&t=<timestamp>
    // that Vite handles to return only component metadata, not the full module
    let get_replace_metadata_url_call = Expression::InvokeFn(InvokeFunctionExpr {
        fn_: Box::new(external_expr(R3::get_replace_metadata_url())),
        args: vec![
            Expression::ReadVar(ReadVarExpr {
                name: id_name.to_string(),
                type_: None,
                source_span: None,
            }),
            Expression::ReadVar(ReadVarExpr {
                name: timestamp_name.to_string(),
                type_: None,
                source_span: None,
            }),
            url_expr,
        ],
        type_: None,
        source_span: None,
        pure: false,
    });

    // import(/* @vite-ignore */ ɵɵgetReplaceMetadataURL(...)).then(replaceCallback)
    let dynamic_import = Expression::InvokeFn(InvokeFunctionExpr {
        fn_: Box::new(Expression::ReadVar(ReadVarExpr {
            name: "__vite_ignore_import".to_string(), // Magic name handled by abstract_emitter
            type_: None,
            source_span: None,
        })),
        args: vec![get_replace_metadata_url_call],
        type_: None,
        source_span: None,
        pure: false,
    });

    let import_then = Expression::InvokeFn(InvokeFunctionExpr {
        fn_: Box::new(Expression::ReadProp(ReadPropExpr {
            receiver: Box::new(dynamic_import),
            name: "then".to_string(),
            type_: None,
            source_span: None,
        })),
        args: vec![replace_callback],
        type_: None,
        source_span: None,
        pure: false,
    });

    // function Cmp_HmrLoad(t) { import(...).then(...); }
    let import_callback = Statement::DeclareFn(DeclareFunctionStmt {
        name: import_callback_name.clone(),
        params: vec![FnParam {
            name: timestamp_name.to_string(),
            type_: None,
        }],
        statements: vec![import_then.to_stmt()],
        type_: None,
        modifiers: StmtModifier::Final,
        source_span: None,
    });

    let id_read = Expression::ReadVar(ReadVarExpr {
        name: id_name.to_string(),
        type_: None,
        source_span: None,
    });

    let update_callback = compile_hmr_update_callback(id_read, &import_callback_name);

    // import.meta.hot
    let hot_read = Expression::ReadProp(ReadPropExpr {
        receiver: Box::new(Expression::ReadProp(ReadPropExpr {
            receiver: Box::new(Expression::ReadVar(ReadVarExpr {
                name: "import".to_string(),
                type_: None,
                source_span: None,
            })),
            name: "meta".to_string(),
            type_: None,
            source_span: None,
        })),
        name: "hot".to_string(),
        type_: None,
        source_span: None,
    });

    // import.meta.hot.on('angular:component-update', updateCallback)
    let hot_listener = Expression::InvokeFn(InvokeFunctionExpr {
        fn_: Box::new(Expression::ReadProp(ReadPropExpr {
            receiver: Box::new(hot_read.clone()),
            name: "on".to_string(),
            type_: None,
            source_span: None,
        })),
        args: vec![
            Expression::Literal(LiteralExpr {
                value: LiteralValue::String("angular:component-update".to_string()),
                type_: None,
                source_span: None,
            }),
            update_callback,
        ],
        type_: None,
        source_span: None,
        pure: false,
    });

    // Encode ID (kept for potential future use)
    let _encoded_id = encode_uri_component(&format!("{}@{}", meta.file_path, meta.class_name));

    // Build the IIFE
    let iife_body: Vec<Statement> = vec![
        // const id = <hmr_id>
        Statement::DeclareVar(DeclareVarStmt {
            name: id_name.to_string(),
            value: Some(Box::new(Expression::Literal(LiteralExpr {
                value: LiteralValue::String(meta.hmr_id.clone()),
                type_: None,
                source_span: None,
            }))),
            type_: None,
            modifiers: StmtModifier::Final,
            source_span: None,
        }),
        // function Cmp_HmrLoad() {...}
        import_callback,
        // (typeof ngDevMode === "undefined" || ngDevMode) && Cmp_HmrLoad(Date.now());
        // Now safe to call immediately because ɵɵgetReplaceMetadataURL returns a metadata-only URL
        // that won't cause infinite loop by re-importing the full module
        dev_only_guarded_expression(Expression::InvokeFn(InvokeFunctionExpr {
            fn_: Box::new(Expression::ReadVar(ReadVarExpr {
                name: import_callback_name.clone(),
                type_: None,
                source_span: None,
            })),
            args: vec![Expression::InvokeFn(InvokeFunctionExpr {
                fn_: Box::new(Expression::ReadProp(ReadPropExpr {
                    receiver: Box::new(Expression::ReadVar(ReadVarExpr {
                        name: "Date".to_string(),
                        type_: None,
                        source_span: None,
                    })),
                    name: "now".to_string(),
                    type_: None,
                    source_span: None,
                })),
                args: vec![],
                type_: None,
                source_span: None,
                pure: false,
            })],
            type_: None,
            source_span: None,
            pure: false,
        }))
        .to_stmt(),
        // (typeof ngDevMode === "undefined" || ngDevMode) && (import.meta.hot && import.meta.hot.on(...))
        Expression::BinaryOp(BinaryOperatorExpr {
            operator: BinaryOperator::And,
            lhs: Box::new(Expression::BinaryOp(BinaryOperatorExpr {
                operator: BinaryOperator::Or,
                lhs: Box::new(Expression::BinaryOp(BinaryOperatorExpr {
                    operator: BinaryOperator::Identical,
                    lhs: Box::new(Expression::TypeOf(crate::output::output_ast::TypeofExpr {
                        expr: Box::new(Expression::ReadVar(ReadVarExpr {
                            name: "ngDevMode".to_string(),
                            type_: None,
                            source_span: None,
                        })),
                        type_: None,
                        source_span: None,
                    })),
                    rhs: Box::new(Expression::Literal(LiteralExpr {
                        value: LiteralValue::String("undefined".to_string()),
                        type_: None,
                        source_span: None,
                    })),
                    type_: None,
                    source_span: None,
                })),
                rhs: Box::new(Expression::ReadVar(ReadVarExpr {
                    name: "ngDevMode".to_string(),
                    type_: None,
                    source_span: None,
                })),
                type_: None,
                source_span: None,
            })),
            rhs: Box::new(Expression::BinaryOp(BinaryOperatorExpr {
                operator: BinaryOperator::And,
                lhs: Box::new(hot_read),
                rhs: Box::new(hot_listener),
                type_: None,
                source_span: None,
            })),
            type_: None,
            source_span: None,
        })
        .to_stmt(),
    ];

    let iife = Expression::ArrowFn(ArrowFunctionExpr {
        params: vec![],
        body: ArrowFunctionBody::Statements(iife_body),
        type_: None,
        source_span: None,
    });

    Expression::InvokeFn(InvokeFunctionExpr {
        fn_: Box::new(iife),
        args: vec![],
        type_: None,
        source_span: None,
        pure: false,
    })
}

/// Compiles the HMR update callback
pub fn compile_hmr_update_callback(id_expr: Expression, import_callback_name: &str) -> Expression {
    let data_name = "d";

    // (d) => d.id === id && Cmp_HmrLoad(d.timestamp)
    let d_id = Expression::ReadProp(ReadPropExpr {
        receiver: Box::new(Expression::ReadVar(ReadVarExpr {
            name: data_name.to_string(),
            type_: None,
            source_span: None,
        })),
        name: "id".to_string(),
        type_: None,
        source_span: None,
    });

    let d_timestamp = Expression::ReadProp(ReadPropExpr {
        receiver: Box::new(Expression::ReadVar(ReadVarExpr {
            name: data_name.to_string(),
            type_: None,
            source_span: None,
        })),
        name: "timestamp".to_string(),
        type_: None,
        source_span: None,
    });

    let hmr_load_call = Expression::InvokeFn(InvokeFunctionExpr {
        fn_: Box::new(Expression::ReadVar(ReadVarExpr {
            name: import_callback_name.to_string(),
            type_: None,
            source_span: None,
        })),
        args: vec![d_timestamp],
        type_: None,
        source_span: None,
        pure: false,
    });

    Expression::ArrowFn(ArrowFunctionExpr {
        params: vec![FnParam {
            name: data_name.to_string(),
            type_: None,
        }],
        body: ArrowFunctionBody::Expression(Box::new(Expression::BinaryOp(BinaryOperatorExpr {
            operator: BinaryOperator::And,
            lhs: Box::new(Expression::BinaryOp(BinaryOperatorExpr {
                operator: BinaryOperator::Identical,
                lhs: Box::new(d_id),
                rhs: Box::new(id_expr),
                type_: None,
                source_span: None,
            })),
            rhs: Box::new(hmr_load_call),
            type_: None,
            source_span: None,
        }))),
        type_: None,
        source_span: None,
    })
}

/// Metadata for HMR update callback compilation
#[derive(Debug, Clone)]
pub struct R3HmrUpdateCallbackMeta {
    /// Name of the component class
    pub class_name: String,
    /// Namespace dependencies with assigned names (e.g., ɵhmr0, ɵhmr1)
    pub namespace_dependencies: Vec<R3HmrNamespaceDependency>,
    /// Local dependencies (imported symbols like MatCardModule)  
    pub local_dependencies: Vec<R3HmrLocalDependency>,
}

/// Component definition field (ɵfac, ɵcmp)
#[derive(Debug, Clone)]
pub struct HmrComponentField {
    /// Field name (e.g., "ɵfac", "ɵcmp")
    pub name: String,
    /// Field initializer expression
    pub initializer: Option<Expression>,
    /// Additional statements for this field
    pub statements: Vec<Statement>,
}

/// Compiles the HMR update callback module that can replace component metadata at runtime.
///
/// This generates:
/// ```javascript
/// export default function Component_UpdateMetadata(Component, ɵɵnamespaces, Dep1, Dep2, ...) {
///     const ɵhmr0 = ɵɵnamespaces[0];
///     const ɵhmr1 = ɵɵnamespaces[1];
///     ...
///     Component.ɵfac = function Component_Factory(...) { ... };
///     Component.ɵcmp = ɵhmr0.ɵɵdefineComponent({ ... });
///     // ɵsetClassMetadata and ɵsetClassDebugInfo calls
/// }
/// ```
pub fn compile_hmr_update_callback_module(
    meta: &R3HmrUpdateCallbackMeta,
    definitions: Vec<HmrComponentField>,
    constant_statements: Vec<Statement>,
) -> Vec<Statement> {
    let namespaces_param = "ɵɵnamespaces";

    // Build function parameters: (ComponentClass, ɵɵnamespaces, ...localDeps)
    let mut params = vec![
        FnParam {
            name: meta.class_name.clone(),
            type_: Some(dynamic_type()),
        },
        FnParam {
            name: namespaces_param.to_string(),
            type_: Some(dynamic_type()),
        },
    ];

    // Add local dependencies as parameters
    for local in &meta.local_dependencies {
        params.push(FnParam {
            name: local.name.clone(),
            type_: None,
        });
    }

    // Build function body
    let mut body: Vec<Statement> = vec![];

    // Declare namespace extraction variables: const ɵhmr0 = ɵɵnamespaces[0];
    for (i, ns_dep) in meta.namespace_dependencies.iter().enumerate() {
        body.push(Statement::DeclareVar(DeclareVarStmt {
            name: ns_dep.assigned_name.clone(),
            value: Some(Box::new(Expression::ReadKey(ReadKeyExpr {
                receiver: Box::new(Expression::ReadVar(ReadVarExpr {
                    name: namespaces_param.to_string(),
                    type_: None,
                    source_span: None,
                })),
                index: Box::new(Expression::Literal(LiteralExpr {
                    value: LiteralValue::Number(i as f64),
                    type_: None,
                    source_span: None,
                })),
                type_: None,
                source_span: None,
            }))),
            type_: Some(dynamic_type()),
            modifiers: StmtModifier::Final,
            source_span: None,
        }));
    }

    // Add constant statements
    body.extend(constant_statements);

    // Add field definitions: Component.ɵfac = ...; Component.ɵcmp = ...;
    for field in definitions {
        if let Some(initializer) = field.initializer {
            // Component.fieldName = initializer;
            body.push(
                Expression::WriteProp(WritePropExpr {
                    receiver: Box::new(Expression::ReadVar(ReadVarExpr {
                        name: meta.class_name.clone(),
                        type_: None,
                        source_span: None,
                    })),
                    name: field.name,
                    value: Box::new(initializer),
                    type_: None,
                    source_span: None,
                })
                .to_stmt(),
            );

            // Add additional statements for this field
            body.extend(field.statements);
        }
    }

    // Create the function declaration
    let func_decl = Statement::DeclareFn(DeclareFunctionStmt {
        name: format!("{}_UpdateMetadata", meta.class_name),
        params,
        statements: body,
        type_: None,
        modifiers: StmtModifier::Exported, // Will be made default export in emitter
        source_span: None,
    });

    vec![func_decl]
}
