use std::collections::HashMap;
use crate::linker::partial_linker::PartialLinker;
use crate::linker::ast::AstNode;

pub struct PartialLinkerSelector<'a, TExpression: AstNode> {
    linkers: HashMap<String, Box<dyn PartialLinker<TExpression> + 'a>>,
}

impl<'a, TExpression: AstNode + 'a> PartialLinkerSelector<'a, TExpression> {
    pub fn new() -> Self {
        let mut linkers: HashMap<String, Box<dyn PartialLinker<TExpression> + 'a>> = HashMap::new();
        // Register linkers here
        use crate::linker::partial_linkers::partial_component_linker_2::PartialComponentLinker2;
        use crate::linker::partial_linkers::partial_directive_linker_2::PartialDirectiveLinker2;
        use crate::linker::partial_linkers::partial_pipe_linker_2::PartialPipeLinker2;
        use crate::linker::partial_linkers::partial_ng_module_linker_2::PartialNgModuleLinker2;
        use crate::linker::partial_linkers::partial_factory_linker_2::PartialFactoryLinker2;
        use crate::linker::partial_linkers::partial_injectable_linker_2::PartialInjectableLinker2;
        use crate::linker::partial_linkers::partial_injector_linker_2::PartialInjectorLinker2;
        
        // Note: The specific linkers must satisfy PartialLinker<TExpression>
        // We assume they are implemented generically.
        linkers.insert("ɵɵngDeclareComponent".to_string(), Box::new(PartialComponentLinker2::new()));
        linkers.insert("ɵɵngDeclareDirective".to_string(), Box::new(PartialDirectiveLinker2::new()));
        linkers.insert("ɵɵngDeclarePipe".to_string(), Box::new(PartialPipeLinker2::new()));
        linkers.insert("ɵɵngDeclareNgModule".to_string(), Box::new(PartialNgModuleLinker2::new()));
        linkers.insert("ɵɵngDeclareFactory".to_string(), Box::new(PartialFactoryLinker2::new()));
        linkers.insert("ɵɵngDeclareInjectable".to_string(), Box::new(PartialInjectableLinker2::new()));
        linkers.insert("ɵɵngDeclareInjector".to_string(), Box::new(PartialInjectorLinker2::new()));
        
        // Aliases for JIT/Decorator mode
        linkers.insert("Component".to_string(), Box::new(PartialComponentLinker2::new()));
        linkers.insert("Directive".to_string(), Box::new(PartialDirectiveLinker2::new()));
        linkers.insert("Pipe".to_string(), Box::new(PartialPipeLinker2::new()));
        linkers.insert("NgModule".to_string(), Box::new(PartialNgModuleLinker2::new()));
        linkers.insert("Injectable".to_string(), Box::new(PartialInjectableLinker2::new()));
        
        Self { linkers }
    }

    pub fn supports_declaration(&self, name: &str) -> bool {
        self.linkers.contains_key(name)
    }

    pub fn get_linker(&self, name: &str, _min_version: &str, _version: &str) -> &dyn PartialLinker<TExpression> {
        self.linkers.get(name).expect(&format!("Linker for {} not found", name)).as_ref()
    }
}

