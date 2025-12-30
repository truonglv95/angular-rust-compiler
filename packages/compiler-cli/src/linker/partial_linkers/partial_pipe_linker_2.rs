use crate::linker::ast::AstNode;
use crate::linker::ast_value::AstObject;
use crate::linker::partial_linker::PartialLinker;
use angular_compiler::constant_pool::ConstantPool;
use angular_compiler::output::output_ast as o;
use angular_compiler::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};
use angular_compiler::render3::r3_pipe_compiler::{compile_pipe_from_metadata, R3PipeMetadata};
use angular_compiler::render3::util::R3Reference;

pub struct PartialPipeLinker2;

impl PartialPipeLinker2 {
    pub fn new() -> Self {
        Self
    }

    fn to_r3_pipe_metadata<TExpression: AstNode>(
        &self,
        meta_obj: &AstObject<TExpression>,
    ) -> Result<R3PipeMetadata, String> {
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

        // Create a dummy source file for source spans
        let dummy_file =
            std::sync::Arc::new(ParseSourceFile::new("".to_string(), "unknown".to_string()));
        let _dummy_span = ParseSourceSpan::new(
            ParseLocation::new(std::sync::Arc::clone(&dummy_file), 0, 0, 0),
            ParseLocation::new(dummy_file, 0, 0, 0),
        );

        let pipe_name = meta_obj
            .get_string("name")
            .unwrap_or_else(|_| "Pipe".to_string());
        let pure = meta_obj.get_bool("pure").unwrap_or(true);
        let is_standalone = meta_obj.get_bool("isStandalone").unwrap_or(false);

        Ok(R3PipeMetadata {
            name: "Pipe".to_string(), // internal class name
            type_: type_ref,
            type_argument_count: 0,
            pipe_name: Some(pipe_name),
            deps: None,
            pure,
            is_standalone,
        })
    }
}

impl<TExpression: AstNode> PartialLinker<TExpression> for PartialPipeLinker2 {
    fn link_partial_declaration(
        &self,
        _constant_pool: &mut ConstantPool,
        meta_obj: &AstObject<TExpression>,
        _source_url: &str,
        _version: &str,
        _target_name: Option<&str>,
    ) -> o::Expression {
        match self.to_r3_pipe_metadata(meta_obj) {
            Ok(meta) => {
                let res = compile_pipe_from_metadata(&meta);
                res.expression
            }
            Err(e) => o::Expression::Literal(o::LiteralExpr {
                value: o::LiteralValue::String(format!("Error: {}", e)),
                type_: None,
                source_span: None,
            }),
        }
    }
}
