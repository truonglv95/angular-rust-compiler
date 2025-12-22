use angular_compiler::constant_pool::ConstantPool;
use angular_compiler::output::output_ast as o;
use crate::linker::ast::AstNode;
use crate::linker::ast_value::AstObject;

/// Trait implemented by all partial linkers (component, directive, etc.).
pub trait PartialLinker<TExpression: AstNode> {
    /// Links a partial declaration metadata object to a full definition expression.
    fn link_partial_declaration(
        &self,
        constant_pool: &mut ConstantPool,
        meta_obj: &AstObject<TExpression>,
        source_url: &str,
        version: &str,
        target_name: Option<&str>,
    ) -> o::Expression;
}
