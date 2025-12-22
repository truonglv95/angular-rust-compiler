use crate::linker::partial_linker::PartialLinker;
use crate::linker::ast_value::{AstObject, AstValue};
use crate::linker::ast::AstNode;
use angular_compiler::constant_pool::ConstantPool;
use angular_compiler::output::output_ast as o;
use angular_compiler::render3::r3_injector_compiler::{
    compile_injector, R3InjectorMetadata
};
use angular_compiler::render3::util::R3Reference;

pub struct PartialInjectorLinker2;

impl PartialInjectorLinker2 {
    pub fn new() -> Self {
        Self
    }
}

impl<TExpression: AstNode> PartialLinker<TExpression> for PartialInjectorLinker2 {
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

        // Extract providers
        let providers = if meta_obj.has("providers") {
            if let Ok(v) = meta_obj.get_value("providers") {
                let code = v.host.print_node(&v.node);
                Some(o::Expression::RawCode(o::RawCodeExpr {
                    code,
                    source_span: None,
                }))
            } else {
                None
            }
        } else {
            None
        };

        // Extract imports
        let imports = if meta_obj.has("imports") {
             if let Ok(arr) = meta_obj.get_array("imports") {
                 arr.iter().map(|v| {
                     let s = v.host.print_node(&v.node);
                     o::Expression::RawCode(o::RawCodeExpr {
                        code: s,
                        source_span: None,
                     })
                 }).collect()
             } else {
                 vec![]
             }
        } else {
            vec![]
        };

        let meta = R3InjectorMetadata {
            name: "Injector".to_string(), 
            type_: type_ref,
            providers,
            imports,
        };

        let res = compile_injector(&meta);
        res.expression
    }
}
