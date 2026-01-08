use crate::linker::ast::AstNode;
use crate::linker::ast_value::{AstObject, AstValue};
use crate::linker::partial_linker::PartialLinker;
use angular_compiler::constant_pool::ConstantPool;
use angular_compiler::output::output_ast as o;
use angular_compiler::render3::r3_module_compiler::{
    compile_ng_module, R3NgModuleMetadata, R3NgModuleMetadataCommon, R3NgModuleMetadataGlobal,
    R3NgModuleMetadataKind, R3SelectorScopeMode,
};
use angular_compiler::render3::util::R3Reference;

pub struct PartialNgModuleLinker2;

impl PartialNgModuleLinker2 {
    pub fn new() -> Self {
        Self
    }

    fn to_r3_reference<TExpression: AstNode>(value: &AstValue<TExpression>) -> R3Reference {
        let type_str = value.print();
        let wrapped_node = o::Expression::ReadVar(o::ReadVarExpr {
            name: type_str,
            type_: None,
            source_span: None,
        });

        R3Reference {
            value: wrapped_node.clone(),
            type_expr: wrapped_node,
        }
    }

    fn to_r3_ng_module_metadata<TExpression: AstNode>(
        &self,
        meta_obj: &AstObject<TExpression>,
    ) -> Result<R3NgModuleMetadata, String> {
        let type_expr = meta_obj.get_value("type")?.node;
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

        let bootstrap: Vec<R3Reference> = meta_obj
            .get_array("bootstrap")
            .unwrap_or_default()
            .iter()
            .map(|v| Self::to_r3_reference(v))
            .collect();

        let declarations: Vec<R3Reference> = meta_obj
            .get_array("declarations")
            .unwrap_or_default()
            .iter()
            .map(|v| Self::to_r3_reference(v))
            .collect();

        let imports: Vec<R3Reference> = meta_obj
            .get_array("imports")
            .unwrap_or_default()
            .iter()
            .map(|v| Self::to_r3_reference(v))
            .collect();

        let exports: Vec<R3Reference> = meta_obj
            .get_array("exports")
            .unwrap_or_default()
            .iter()
            .map(|v| Self::to_r3_reference(v))
            .collect();

        let schemas: Option<Vec<R3Reference>> = meta_obj
            .get_array("schemas")
            .ok()
            .map(|arr| arr.iter().map(|v| Self::to_r3_reference(v)).collect());

        let id = meta_obj.get_value("id").ok().map(|v| {
            let id_str = v.print();
            o::Expression::ReadVar(o::ReadVarExpr {
                name: id_str,
                type_: None,
                source_span: None,
            })
        });

        Ok(R3NgModuleMetadata::Global(R3NgModuleMetadataGlobal {
            common: R3NgModuleMetadataCommon {
                kind: R3NgModuleMetadataKind::Global,
                type_: type_ref,
                selector_scope_mode: R3SelectorScopeMode::Inline, // Assuming Inline for linked code
                schemas,
                id,
            },
            bootstrap,
            declarations,
            public_declaration_types: None, // Not available in partial metadata
            imports,
            include_import_types: true, // Default to true?
            exports,
            contains_forward_decls: false,
        }))
    }
}

impl<TExpression: AstNode> PartialLinker<TExpression> for PartialNgModuleLinker2 {
    fn link_partial_declaration(
        &self,
        _constant_pool: &mut ConstantPool,
        meta_obj: &AstObject<TExpression>,
        _source_url: &str,
        _version: &str,
        _target_name: Option<&str>,
        _imports: Option<&std::collections::HashMap<String, String>>,
        _class_metadata: Option<&AstObject<TExpression>>,
    ) -> o::Expression {
        match self.to_r3_ng_module_metadata(meta_obj) {
            Ok(meta) => {
                let res = compile_ng_module(&meta);
                if !res.statements.is_empty() {
                    // Wrap in IIFE if there are statements
                    // (function() { statements; return expression; })()
                    let mut stmts = res.statements;
                    stmts.push(o::Statement::Return(o::ReturnStatement {
                        value: Box::new(res.expression),
                        source_span: None,
                    }));

                    o::Expression::InvokeFn(o::InvokeFunctionExpr {
                        fn_: Box::new(o::Expression::Fn(o::FunctionExpr {
                            params: vec![],
                            statements: stmts,
                            type_: None,
                            source_span: None,
                            name: None,
                        })),
                        args: vec![],
                        type_: None,
                        source_span: None,
                        pure: false,
                    })
                } else {
                    res.expression
                }
            }
            Err(e) => o::Expression::Literal(o::LiteralExpr {
                value: o::LiteralValue::String(format!("Error: {}", e)),
                type_: None,
                source_span: None,
            }),
        }
    }
}
