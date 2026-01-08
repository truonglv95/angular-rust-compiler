//! FileLinker Implementation
//!
//! Orchestrates the linking process for a single file.

use crate::linker::ast::AstHost;

use crate::linker::ast_value::AstValue;
use crate::linker::partial_linker::PartialLinker;
use crate::ngtsc::translator::src::api::ast_factory::AstFactory;
use angular_compiler::constant_pool::ConstantPool;
// use crate::ngtsc::translator... TranslatorOptions, ImportGenerator?

/// Environment dependencies for the linker.
pub struct LinkerEnvironment<'a, A: AstFactory> {
    pub host: Box<dyn AstHost<A::Expression> + 'a>,
    pub factory: &'a A,
    // translator options, logger, etc.
}

impl<'a, A: AstFactory> LinkerEnvironment<'a, A> {
    pub fn new(host: Box<dyn AstHost<A::Expression> + 'a>, factory: &'a A) -> Self {
        Self { host, factory }
    }
}

use crate::linker::ast::AstNode;
use crate::linker::partial_linkers::partial_linker_selector::PartialLinkerSelector;

pub struct FileLinker<'a, A: AstFactory>
where
    A::Expression: Clone + AstNode,
{
    environment: LinkerEnvironment<'a, A>,
    linker_selector: PartialLinkerSelector<'a, A::Expression>,
}

impl<'a, A: AstFactory> FileLinker<'a, A>
where
    A::Expression: Clone + AstNode,
{
    pub fn new(environment: LinkerEnvironment<'a, A>) -> Self {
        Self {
            environment,
            linker_selector: PartialLinkerSelector::new(),
        }
    }

    pub fn is_partial_declaration(&self, callee_name: &str) -> bool {
        self.linker_selector.supports_declaration(callee_name)
    }

    pub fn link_partial_declaration(
        &self,
        name: &str,
        args: &[A::Expression],
        source_url: &str,
    ) -> Result<A::Expression, String> {
        if !self.linker_selector.supports_declaration(name) {
            return Err(format!("Declaration {} not supported", name));
        }

        // This is a simplified version. Real partial linking involves more complex argument parsing.
        // We assume args[0] is the metadata object.
        if args.len() < 1 {
            return Err("Missing metadata object".to_string());
        }

        let meta_expr = &args[0];
        // Create AstValue helper
        let value = AstValue::new(meta_expr.clone(), self.environment.host.as_ref());

        let obj = value
            .get_object()
            .map_err(|_| "Metadata is not an object".to_string())?;

        // We need a proper version handling. For now, empty string or stub.
        let linker = self.linker_selector.get_linker(name, "0.0.0", "0.0.0");

        let mut constant_pool = ConstantPool::new(false); // Mock Constant Pool (needs implementation or passing in)

        let _definition = linker.link_partial_declaration(
            &mut constant_pool,
            &obj,
            source_url,
            "0.0.0",
            None,
            None,
            None,
        );

        // Translate definition (output AST) to native AST using environment.translator?
        // Since we don't have translator instance yet, we stub.
        // environment.translator.translate(definition)

        Err("Translator not available yet".to_string())
    }
}
