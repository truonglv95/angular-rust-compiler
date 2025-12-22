use std::collections::HashMap;
use indexmap::IndexMap;

use crate::compiler_facade_interface::{
    self as facade, CompilerFacade, CoreEnvironment, R3ComponentMetadataFacade,
    R3DeclareComponentFacade, R3DeclareDirectiveFacade, R3DeclareFactoryFacade,
    R3DeclareInjectableFacade, R3DeclareInjectorFacade, R3DeclareNgModuleFacade,
    R3DeclarePipeFacade, R3DirectiveMetadataFacade, R3FactoryDefMetadataFacade,
    R3InjectableMetadataFacade, R3InjectorMetadataFacade, R3NgModuleMetadataFacade,
    R3PipeMetadataFacade, R3QueryMetadataFacade, R3DeclareDependencyMetadataFacade,
    R3DependencyMetadataFacade,
};
use crate::constant_pool::ConstantPool;
use crate::core::{ChangeDetectionStrategy, ViewEncapsulation};
use crate::injectable_compiler_2::{
    compile_injectable,
    R3InjectableMetadata as InjectableMetadata,
};
use crate::output::output_ast::{
    DeclareVarStmt, Expression, Statement, StmtModifier, WrappedNodeExpr,
};
use crate::output::output_jit::{ExternalReferenceResolver, JitEvaluator};
use crate::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};
use crate::render3::r3_factory::{
    compile_factory_function, R3ConstructorFactoryMetadata, R3FactoryMetadata,
    FactoryTarget,
};
use crate::render3::r3_injector_compiler::{compile_injector, R3InjectorMetadata};
use crate::render3::r3_jit::{ExternalReferenceResolver as R3ExternalReferenceResolver, R3JitReflector};
use crate::render3::r3_module_compiler::{
    compile_ng_module, R3NgModuleMetadata, R3NgModuleMetadataCommon, R3NgModuleMetadataGlobal,
    R3NgModuleMetadataKind, R3SelectorScopeMode,
};
use crate::render3::r3_pipe_compiler::{compile_pipe_from_metadata, R3PipeMetadata};
use crate::render3::util::{
    wrap_reference, R3Reference, create_may_be_forward_ref_expression, get_safe_property_access_string,
};
use crate::render3::view::api::{
    R3ComponentMetadata, R3ComponentTemplate, R3DirectiveMetadata, R3HostMetadata,
    R3LifecycleMetadata, R3QueryMetadata,
    ChangeDetectionOrExpression,
    R3InputMetadata,
};
use crate::render3::view::compiler::{
    compile_component_from_metadata, compile_directive_from_metadata,
    parse_host_bindings, verify_host_bindings, ParsedHostBindings,
};
use crate::render3::view::template::{make_binding_parser, parse_template, ParseTemplateOptions};

pub struct CompilerFacadeImpl {
    jit_evaluator: JitEvaluator,
}

impl CompilerFacadeImpl {
    pub fn new() -> Self {
        Self {
            jit_evaluator: JitEvaluator::new(),
        }
    }

    fn jit_expression(
        &self,
        def: Expression,
        context: CoreEnvironment,
        source_url: String,
        pre_statements: Vec<Statement>,
    ) -> Result<serde_json::Value, String> {
        let mut statements = pre_statements;
        statements.push(Statement::DeclareVar(DeclareVarStmt {
            name: "$def".to_string(),
            value: Some(Box::new(def)),
            type_: None,
            modifiers: StmtModifier::Exported,
            source_span: None,
        }));

        let mut reflector_context: HashMap<String, Box<dyn std::any::Any>> = HashMap::new();
        for (k, v) in context {
            reflector_context.insert(k, Box::new(v));
        }

        let reflector = R3JitReflector::new(reflector_context);
        let reflector_adapter = R3JitReflectorAdapter(reflector);

        let res = self.jit_evaluator.evaluate_statements(
            &source_url,
            &statements,
            &reflector_adapter,
            /* enableSourceMaps */ true,
        );

        // Extract $def from res
        if let Some(_val) = res.get("$def") {
            // Since Box<dyn Any> serves as the value container, we need to decide how to return it.
            // The signature returns serde_json::Value, but the evaluated result is Box<dyn Any>.
            // In the real JIT, this returns the actual runtime class/factory.
            // Here, we can't easily convert Box<dyn Any> to serde_json::Value without knowing what it is.
            // For now, we'll return Null as a placeholder, but with a comment that we found the value.
            // In a real generic implementation we might need a way to serialize the result or return an opaque handle.
            Ok(serde_json::Value::Null) 
        } else {
             Ok(serde_json::Value::Null)
        }
    }
}

impl CompilerFacade for CompilerFacadeImpl {
    fn compile_pipe(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        facade: R3PipeMetadataFacade,
    ) -> Result<serde_json::Value, String> {
        let metadata = R3PipeMetadata {
            name: facade.name.clone(),
            type_: wrap_reference(facade.type_ref),
            type_argument_count: 0,
            deps: None,
            pipe_name: facade.pipe_name,
            pure: facade.pure,
            is_standalone: facade.is_standalone,
        };
        let res = compile_pipe_from_metadata(&metadata);
        self.jit_expression(res.expression, angular_core_env, source_map_url, vec![])
    }

    fn compile_pipe_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        declaration: R3DeclarePipeFacade,
    ) -> Result<serde_json::Value, String> {
        let meta = convert_declare_pipe_facade_to_metadata(declaration);
        let res = compile_pipe_from_metadata(&meta);
        self.jit_expression(res.expression, angular_core_env, source_map_url, vec![])
    }

    fn compile_injector(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        facade: R3InjectorMetadataFacade,
    ) -> Result<serde_json::Value, String> {
        let meta = R3InjectorMetadata {
            name: facade.name.clone(),
            type_: wrap_reference(facade.type_ref),
            providers: if !facade.providers.is_empty() {
                Some(new_wrapped_node_expr(serde_json::Value::Array(facade.providers)))
            } else {
                None
            },
            imports: facade
                .imports
                .iter()
                .map(|i| new_wrapped_node_expr(i.clone()))
                .collect(),
        };
        let res = compile_injector(&meta);
        self.jit_expression(res.expression, angular_core_env, source_map_url, vec![])
    }

    fn compile_injector_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        declaration: R3DeclareInjectorFacade,
    ) -> Result<serde_json::Value, String> {
        let meta = convert_declare_injector_facade_to_metadata(declaration);
        let res = compile_injector(&meta);
        self.jit_expression(res.expression, angular_core_env, source_map_url, vec![])
    }

    fn compile_ng_module(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        facade: R3NgModuleMetadataFacade,
    ) -> Result<serde_json::Value, String> {
        let schemas = if let Some(schemas) = facade.schemas {
             if !schemas.is_empty() {
                 Some(
                     schemas
                         .iter()
                         .map(|s| wrap_reference(s.name.clone()))
                         .collect(),
                 )
             } else {
                 None
             }
        } else {
             None
        };

        let meta = R3NgModuleMetadata::Global(R3NgModuleMetadataGlobal {
            common: R3NgModuleMetadataCommon {
                kind: R3NgModuleMetadataKind::Global,
                type_: wrap_reference(facade.type_ref),
                selector_scope_mode: R3SelectorScopeMode::Inline,
                schemas,
                id: facade.id.map(|id| new_wrapped_node_expr(serde_json::Value::String(id))),
            },
            bootstrap: facade
                .bootstrap
                .iter()
                .map(|b| wrap_reference(b.clone()))
                .collect(),
            declarations: facade
                .declarations
                .iter()
                .map(|d| wrap_reference(d.clone()))
                .collect(),
            public_declaration_types: None,
            imports: facade
                .imports
                .iter()
                .map(|i| wrap_reference(i.clone()))
                .collect(),
            include_import_types: true,
            exports: facade
                .exports
                .iter()
                .map(|e| wrap_reference(e.clone()))
                .collect(),
            contains_forward_decls: false,
        });

        let res = compile_ng_module(&meta);
        self.jit_expression(res.expression, angular_core_env, source_map_url, vec![])
    }

    fn compile_ng_module_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        declaration: R3DeclareNgModuleFacade,
    ) -> Result<serde_json::Value, String> {
        let meta = convert_declare_ng_module_facade_to_metadata(declaration);
        let res = compile_ng_module(&meta);
        self.jit_expression(res.expression, angular_core_env, source_map_url, vec![])
    }

    fn compile_directive(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        facade: R3DirectiveMetadataFacade,
    ) -> Result<serde_json::Value, String> {
        let mut constant_pool = ConstantPool::new(false);
        let binding_parser = make_binding_parser(false);

        let meta = convert_directive_facade_to_metadata(facade);
        let res = compile_directive_from_metadata(&meta, &mut constant_pool, &binding_parser);
        self.jit_expression(
            res.expression,
            angular_core_env,
            source_map_url,
            constant_pool.statements,
        )
    }

    fn compile_directive_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        declaration: R3DeclareDirectiveFacade,
    ) -> Result<serde_json::Value, String> {
        let mut constant_pool = ConstantPool::new(false);
        let binding_parser = make_binding_parser(false);

        let meta = convert_declare_directive_facade_to_metadata(declaration);
        let res = compile_directive_from_metadata(&meta, &mut constant_pool, &binding_parser);
        self.jit_expression(
            res.expression,
            angular_core_env,
            source_map_url,
            constant_pool.statements,
        )
    }

    fn compile_component(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        facade: R3ComponentMetadataFacade,
    ) -> Result<serde_json::Value, String> {
        let mut constant_pool = ConstantPool::new(false);
        let binding_parser = make_binding_parser(false);

        let meta = convert_component_facade_to_metadata(facade, source_map_url.clone());
        let res = compile_component_from_metadata(&meta, &mut constant_pool, &binding_parser);
        self.jit_expression(
            res.expression,
            angular_core_env,
            source_map_url,
            constant_pool.statements,
        )
    }

    fn compile_component_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        declaration: R3DeclareComponentFacade,
    ) -> Result<serde_json::Value, String> {
        let mut constant_pool = ConstantPool::new(false);
        let binding_parser = make_binding_parser(false);

        let meta =
            convert_declare_component_facade_to_metadata(declaration, source_map_url.clone());
        let res = compile_component_from_metadata(&meta, &mut constant_pool, &binding_parser);
        self.jit_expression(
            res.expression,
            angular_core_env,
            source_map_url,
            constant_pool.statements,
        )
    }

    fn compile_factory(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        facade: R3FactoryDefMetadataFacade,
    ) -> Result<serde_json::Value, String> {
        let meta = R3FactoryMetadata::Constructor(R3ConstructorFactoryMetadata {
            name: facade.name.clone(),
            type_: wrap_reference(facade.type_ref),
            type_argument_count: 0,
            deps: None,
            target: FactoryTarget::Injectable,
        });
        let res = compile_factory_function(&meta);
        self.jit_expression(res.expression, angular_core_env, source_map_url, vec![])
    }

    fn compile_factory_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        declaration: R3DeclareFactoryFacade,
    ) -> Result<serde_json::Value, String> {
        let meta = R3FactoryMetadata::Constructor(R3ConstructorFactoryMetadata {
            name: declaration.type_ref.clone(),
            type_: wrap_reference(declaration.type_ref),
            type_argument_count: 0,
            deps: None,
            target: FactoryTarget::Injectable,
        });
        let res = compile_factory_function(&meta);
        self.jit_expression(res.expression, angular_core_env, source_map_url, vec![])
    }

    fn compile_injectable(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        facade: R3InjectableMetadataFacade,
    ) -> Result<serde_json::Value, String> {
        let meta = InjectableMetadata {
            name: facade.name.clone(),
            type_ref: create_injectable_reference(facade.type_ref),
            type_argument_count: facade.type_argument_count,
            provided_in: compute_provided_in(facade.provided_in),
            use_class: convert_to_maybe_forward_ref_expression(facade.use_class),
            use_factory: convert_to_provider_expression(facade.use_factory),
            use_existing: convert_to_maybe_forward_ref_expression(facade.use_existing),
            use_value: convert_to_maybe_forward_ref_expression(facade.use_value),
            deps: convert_r3_dependency_metadata_array(facade.deps),
        };

        if let Ok(res) = compile_injectable(meta, false) {
             let expr = convert_injectable_expression_to_expression(res.expression);
             self.jit_expression(expr, angular_core_env, source_map_url, vec![])
        } else {
             Err("Compile injectable failed".to_string())
        }
    }

    fn compile_injectable_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        facade: R3DeclareInjectableFacade,
    ) -> Result<serde_json::Value, String> {
        let meta = InjectableMetadata {
            name: facade.type_ref.clone(),
            type_ref: create_injectable_reference(facade.type_ref.clone()),
            type_argument_count: 0,
            provided_in: compute_provided_in(facade.provided_in),
            use_class: convert_to_maybe_forward_ref_expression(facade.use_class),
            use_factory: convert_to_provider_expression(facade.use_factory),
            use_existing: convert_to_maybe_forward_ref_expression(facade.use_existing),
            use_value: convert_to_maybe_forward_ref_expression(facade.use_value),
            deps: convert_declare_dependency_metadata_array(facade.deps),
        };

        if let Ok(res) = compile_injectable(meta, false) {
             let expr = convert_injectable_expression_to_expression(res.expression);
             self.jit_expression(expr, angular_core_env, source_map_url, vec![])
        } else {
             Err("Compile injectable declaration failed".to_string())
        }
    }

    fn create_parse_source_span(
        &self,
        _kind: String,
        _type_name: String,
        source_url: String,
    ) -> facade::ParseSourceSpan {
        facade::ParseSourceSpan {
            start: facade::ParseLocation {
                file: facade::ParseSourceFile {
                    content: "".to_string(),
                    url: source_url.clone(),
                },
                offset: 0,
                line: 0,
                col: 0,
            },
            end: facade::ParseLocation {
                file: facade::ParseSourceFile {
                    content: "".to_string(),
                    url: source_url,
                },
                offset: 0,
                line: 0,
                col: 0,
            },
        }
    }
}

// Helpers

struct R3JitReflectorAdapter(R3JitReflector);

impl ExternalReferenceResolver for R3JitReflectorAdapter {
    fn resolve_external_reference(
        &self,
        reference: &crate::output::output_ast::ExternalReference,
    ) -> Box<dyn std::any::Any> {
        let inner_ref = crate::output::output_ast::ExternalReference {
            module_name: reference.module_name.clone(),
            name: reference.name.clone(),
            runtime: None, 
        };

        match R3ExternalReferenceResolver::resolve_external_reference(&self.0, &inner_ref) {
            Some(val) => val,
            None => Box::new(()),
        }
    }
}

fn convert_declare_pipe_facade_to_metadata(declaration: R3DeclarePipeFacade) -> R3PipeMetadata {
    R3PipeMetadata {
        name: declaration.type_ref.clone(),
        type_: wrap_reference(declaration.type_ref),
        type_argument_count: 0,
        pipe_name: Some(declaration.name),
        deps: None,
        pure: declaration.pure.unwrap_or(true),
        is_standalone: declaration.is_standalone.unwrap_or(true),
    }
}

fn convert_declare_injector_facade_to_metadata(
    declaration: R3DeclareInjectorFacade,
) -> R3InjectorMetadata {
    R3InjectorMetadata {
        name: declaration.type_ref.clone(),
        type_: wrap_reference(declaration.type_ref),
        providers: if let Some(providers) = declaration.providers {
            Some(new_wrapped_node_expr(serde_json::Value::Array(providers)))
        } else {
            None
        },
        imports: declaration
            .imports
            .unwrap_or_default()
            .iter()
            .map(|i| new_wrapped_node_expr(i.clone()))
            .collect(),
    }
}

fn convert_declare_ng_module_facade_to_metadata(
    declaration: R3DeclareNgModuleFacade,
) -> R3NgModuleMetadata {
    let schemas = if let Some(schemas) = declaration.schemas {
         Some(
             schemas
                 .iter()
                 .map(|s| create_wrapped_reference_from_value(serde_json::json!(s.name)))
                 .collect()
         )
    } else {
         None
    };

    R3NgModuleMetadata::Global(R3NgModuleMetadataGlobal {
        common: R3NgModuleMetadataCommon {
            kind: R3NgModuleMetadataKind::Global,
            type_: wrap_reference(declaration.type_ref),
            selector_scope_mode: R3SelectorScopeMode::Inline,
            schemas,
            id: declaration.id.map(|id| new_wrapped_node_expr(id)),
        },
        bootstrap: declaration
            .bootstrap
            .unwrap_or_default()
            .iter()
            .map(|b| create_wrapped_reference_from_value(serde_json::Value::String(b.to_string())))
            .collect(),
        declarations: declaration
            .declarations
            .unwrap_or_default()
            .iter()
            .map(|d| create_wrapped_reference_from_value(serde_json::Value::String(d.to_string())))
            .collect(),
        public_declaration_types: None,
        imports: declaration
            .imports
            .unwrap_or_default()
            .iter()
            .map(|i| create_wrapped_reference_from_value(serde_json::Value::String(i.to_string())))
            .collect(),
        include_import_types: true,
        exports: declaration
            .exports
            .unwrap_or_default()
            .iter()
            .map(|e| create_wrapped_reference_from_value(serde_json::Value::String(e.to_string())))
            .collect(),
        contains_forward_decls: false,
    })
}

fn convert_directive_facade_to_metadata(facade: R3DirectiveMetadataFacade) -> R3DirectiveMetadata {
    let prop_metadata = facade.prop_metadata;
    let mut inputs_from_type: IndexMap<String, R3InputMetadata> = IndexMap::new();
    let mut outputs_from_type: IndexMap<String, String> = IndexMap::new();

    for (field, annotations) in &prop_metadata {
        for ann in annotations {
            if is_input(ann) {
                let binding_property_name = ann.get("alias").and_then(|v| v.as_str()).unwrap_or(field).to_string();
                let required = ann.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
                let is_signal = ann.get("isSignal").and_then(|v| v.as_bool()).unwrap_or(false);
                let transform = ann.get("transform").map(|v| new_wrapped_node_expr(v.clone()));

                inputs_from_type.insert(field.clone(), R3InputMetadata {
                    binding_property_name,
                    class_property_name: field.clone(),
                    required,
                    is_signal,
                    transform_function: transform,
                });
            } else if is_output(ann) {
                 let alias = ann.get("alias").and_then(|v| v.as_str()).unwrap_or(field).to_string();
                 outputs_from_type.insert(field.clone(), alias);
            }
        }
    }

    let mut inputs = convert_input_metadata_array(&facade.inputs);
    inputs.extend(inputs_from_type);
    
    // Convert outputs from string array to map
    let mut outputs = IndexMap::new();
    for output in &facade.outputs {
        let (alias, name) = parse_mapping_string(output);
        outputs.insert(name, alias);
    }
    outputs.extend(outputs_from_type);

    // Dummy source span for now
    let source_span = create_dummy_source_span();
    
    // Extract host bindings
    let host_bindings = extract_host_bindings(&prop_metadata, &source_span, &facade.host).unwrap_or_else(|_| ParsedHostBindings::default());

    let host_metadata = R3HostMetadata {
        attributes: host_bindings.attributes,
        listeners: host_bindings.listeners,
        properties: host_bindings.properties,
        special_attributes: convert_special_attributes(host_bindings.special_attributes),
    };

    R3DirectiveMetadata {
        name: facade.name.clone(),
        type_: wrap_reference(facade.type_ref),
        type_argument_count: 0,
        type_source_span: source_span,
        deps: None,
        selector: facade.selector,
        queries: facade.queries.into_iter().map(convert_to_r3_query_metadata).collect(),
        view_queries: facade.view_queries.into_iter().map(convert_to_r3_query_metadata).collect(),
        host: host_metadata,
        lifecycle: R3LifecycleMetadata::default(),
        inputs,
        outputs,
        uses_inheritance: facade.uses_inheritance,
        export_as: facade.export_as,
        providers: facade.providers.map(|p| new_wrapped_node_expr(serde_json::Value::Array(p))),
        is_standalone: facade.is_standalone,
        is_signal: facade.is_signal,
        host_directives: None,
    }
}

fn convert_declare_directive_facade_to_metadata(
    declaration: R3DeclareDirectiveFacade,
) -> R3DirectiveMetadata {
    // TODO: hostDirectives
    let host_bindings = convert_host_declaration_to_metadata(declaration.host);

    R3DirectiveMetadata {
        name: declaration.type_ref.clone(),
        type_: wrap_reference(declaration.type_ref),
        type_argument_count: 0,
        type_source_span: create_dummy_source_span(),
        deps: None,
        selector: declaration.selector,
        queries: declaration.queries.unwrap_or_default().into_iter().map(convert_query_declaration_to_metadata).collect(),
        view_queries: declaration.view_queries.unwrap_or_default().into_iter().map(convert_query_declaration_to_metadata).collect(),
        host: host_bindings,
        lifecycle: R3LifecycleMetadata::default(),
        inputs: convert_inputs_declaration(declaration.inputs),
        outputs: convert_outputs_declaration(declaration.outputs),
        uses_inheritance: declaration.uses_inheritance.unwrap_or(false),
        export_as: declaration.export_as,
        providers: declaration.providers.map(|p| new_wrapped_node_expr(serde_json::Value::Array(p))),
        is_standalone: declaration.is_standalone.unwrap_or(false),
        is_signal: declaration.is_signal.unwrap_or(false),
        host_directives: None,
    }
}

fn extract_directive_metadata_from_component(facade: &R3ComponentMetadataFacade) -> R3DirectiveMetadata {
    // Reuse convert_directive_facade_to_metadata logic by temporarily constructing a R3DirectiveMetadataFacade
    // OR just duplicate logic. Duplication is safer to avoid cloning cost of big struct.
    
    // For now, let's just use the previous simple implementation but add host bindings extraction
    let source_span = create_dummy_source_span();
    let host_bindings = extract_host_bindings(&facade.prop_metadata, &source_span, &facade.host).unwrap_or_else(|_| ParsedHostBindings::default());
    let host_metadata = R3HostMetadata {
        attributes: host_bindings.attributes,
        listeners: host_bindings.listeners,
        properties: host_bindings.properties,
        special_attributes: convert_special_attributes(host_bindings.special_attributes),
    };

    R3DirectiveMetadata {
        name: facade.name.clone(),
        type_: wrap_reference(facade.type_ref.clone()),
        type_argument_count: 0,
        type_source_span: source_span,
        deps: None,
        selector: facade.selector.clone(),
        queries: facade.queries.clone().into_iter().map(convert_to_r3_query_metadata).collect(),
        view_queries: facade.view_queries.clone().into_iter().map(convert_to_r3_query_metadata).collect(),
        host: host_metadata,
        lifecycle: R3LifecycleMetadata::default(),
        inputs: IndexMap::new(), // TODO: extract inputs from props
        outputs: IndexMap::new(), // TODO: extract outputs from props
        uses_inheritance: facade.uses_inheritance,
        export_as: facade.export_as.clone(),
        providers: facade.providers.clone().map(|p| new_wrapped_node_expr(serde_json::Value::Array(p))),
        is_standalone: facade.is_standalone,
        is_signal: facade.is_signal,
        host_directives: None,
    }
}

fn extract_declare_directive_metadata_from_component(declaration: &R3DeclareComponentFacade) -> R3DirectiveMetadata {
     let host_bindings = convert_host_declaration_to_metadata(declaration.host.clone());

    R3DirectiveMetadata {
        name: declaration.type_ref.clone(),
        type_: wrap_reference(declaration.type_ref.clone()),
        type_argument_count: 0,
        type_source_span: create_dummy_source_span(),
        deps: None,
        selector: declaration.selector.clone(),
        queries: declaration.queries.clone().unwrap_or_default().into_iter().map(convert_query_declaration_to_metadata).collect(),
        view_queries: declaration.view_queries.clone().unwrap_or_default().into_iter().map(convert_query_declaration_to_metadata).collect(),
        host: host_bindings,
        lifecycle: R3LifecycleMetadata::default(),
        inputs: convert_inputs_declaration(declaration.inputs.clone()),
        outputs: convert_outputs_declaration(declaration.outputs.clone()),
        uses_inheritance: declaration.uses_inheritance.unwrap_or(false),
        export_as: declaration.export_as.clone(),
        providers: declaration.providers.clone().map(|p| new_wrapped_node_expr(serde_json::Value::Array(p))),
        is_standalone: declaration.is_standalone.unwrap_or(false),
        is_signal: declaration.is_signal.unwrap_or(false),
        host_directives: None,
    }
}

fn convert_component_facade_to_metadata(
    facade: R3ComponentMetadataFacade,
    source_map_url: String,
) -> R3ComponentMetadata {
    // Manually extract directive metadata
    let directive = extract_directive_metadata_from_component(&facade);
    
    let template = parse_template(
        &facade.template,
        &source_map_url,
        ParseTemplateOptions {
            preserve_whitespaces: Some(facade.preserve_whitespaces),
            ..Default::default()
        },
    );

    R3ComponentMetadata {
        directive,
        template: R3ComponentTemplate {
            nodes: template.nodes,
            ng_content_selectors: template.ng_content_selectors,
            preserve_whitespaces: facade.preserve_whitespaces,
        },
        declarations: vec![],
        defer: crate::render3::view::api::R3ComponentDeferMetadata::PerComponent {
            dependencies_fn: None,
        },
        declaration_list_emit_mode:
            crate::render3::view::api::DeclarationListEmitMode::RuntimeResolved,
        styles: facade.styles,
        external_styles: None,
        encapsulation: ViewEncapsulation::None,
        animations: facade.animations.map(|a| new_wrapped_node_expr(serde_json::Value::Array(a))),
        view_providers: facade.view_providers.map(|vp| new_wrapped_node_expr(serde_json::Value::Array(vp))),
        relative_context_file_path: source_map_url,
        i18n_use_external_ids: false,
        change_detection: facade.change_detection.map(|_cd| {
            ChangeDetectionOrExpression::Strategy(ChangeDetectionStrategy::Default) 
        }),
        relative_template_path: None,
        has_directive_dependencies: false,
        raw_imports: None,
    }
}

fn convert_declare_component_facade_to_metadata(
    declaration: R3DeclareComponentFacade,
    source_map_url: String,
) -> R3ComponentMetadata {
    // Manually extract directive metadata
    let directive = extract_declare_directive_metadata_from_component(&declaration);

    let template = parse_template(
        &declaration.template,
        &source_map_url,
        ParseTemplateOptions {
            preserve_whitespaces: declaration.preserve_whitespaces,
            ..Default::default()
        },
    );

    R3ComponentMetadata {
        directive,
        template: R3ComponentTemplate {
            nodes: template.nodes,
            ng_content_selectors: template.ng_content_selectors,
            preserve_whitespaces: declaration.preserve_whitespaces.unwrap_or(false),
        },
        declarations: vec![],
        defer: crate::render3::view::api::R3ComponentDeferMetadata::PerComponent {
            dependencies_fn: None,
        },
        declaration_list_emit_mode:
            crate::render3::view::api::DeclarationListEmitMode::RuntimeResolved,
        styles: declaration.styles.unwrap_or_default(),
        external_styles: None,
        encapsulation: ViewEncapsulation::None,
        animations: declaration.animations.map(|a| new_wrapped_node_expr(serde_json::Value::Array(a))),
        view_providers: declaration.view_providers.map(|vp| new_wrapped_node_expr(serde_json::Value::Array(vp))),
        relative_context_file_path: source_map_url,
        i18n_use_external_ids: false,
        change_detection: declaration.change_detection.map(|_cd| {
             ChangeDetectionOrExpression::Strategy(ChangeDetectionStrategy::Default)
        }),
        relative_template_path: None,
        has_directive_dependencies: false,
        raw_imports: None,
    }
}


fn new_wrapped_node_expr(value: serde_json::Value) -> Expression {
    Expression::WrappedNode(WrappedNodeExpr {
        node: Box::new(value),
        type_: None,
        source_span: None,
    })
}

fn create_wrapped_reference_from_value(value: serde_json::Value) -> R3Reference {
    let expr = new_wrapped_node_expr(value);
    R3Reference {
        value: expr.clone(),
        type_expr: expr,
    }
}

fn create_dummy_source_span() -> ParseSourceSpan {
    ParseSourceSpan {
        start: ParseLocation {
            file: ParseSourceFile {
                content: "".to_string(),
                url: "".to_string(),
            },
            offset: 0,
            line: 0,
            col: 0,
        },
        end: ParseLocation {
            file: ParseSourceFile {
                content: "".to_string(),
                url: "".to_string(),
            },
            offset: 0,
            line: 0,
            col: 0,
        },
        details: None,
    }
}

// Helper for Injectable conversion
fn create_injectable_reference(type_ref: String) -> crate::injectable_compiler_2::R3Reference {
     crate::injectable_compiler_2::R3Reference {
         value: create_injectable_expr_json(type_ref.clone()),
         type_ref: create_injectable_expr_json(type_ref),
     }
}

fn create_injectable_expr_json(val: String) -> crate::injectable_compiler_2::Expression {
    crate::injectable_compiler_2::Expression {
        value: serde_json::Value::String(val),
    }
}

#[allow(dead_code)]
fn create_injectable_expr_from_value(val: serde_json::Value) -> crate::injectable_compiler_2::Expression {
    crate::injectable_compiler_2::Expression {
        value: val,
    }
}


fn convert_host_declaration_to_metadata(
    host: Option<HashMap<String, String>>,
) -> R3HostMetadata {
    let host = host.unwrap_or_default();

    // Re-implement or call parse_host_bindings if available for declaration?
    // Declaration facade has host as HashMap, so we can use parse_host_bindings directly.
    let parsed = parse_host_bindings(&host);

    R3HostMetadata {
        attributes: parsed.attributes,
        listeners: parsed.listeners,
        properties: parsed.properties,
        special_attributes: convert_special_attributes(parsed.special_attributes),
    }
}

fn convert_query_declaration_to_metadata(
    declaration: R3QueryMetadataFacade,
) -> R3QueryMetadata {
    R3QueryMetadata {
        property_name: declaration.property_name,
        first: declaration.first,
        predicate: crate::render3::view::api::R3QueryPredicate::Expression(convert_query_predicate(declaration.predicate)),
        descendants: declaration.descendants,
        read: declaration.read.map(|r| new_wrapped_node_expr(r)),
        static_: declaration.is_static,
        emit_distinct_changes_only: declaration.emit_distinct_changes_only,
        is_signal: declaration.is_signal.unwrap_or(false),
    }
}

fn inputs_partial_metadata_to_input_metadata(
    inputs: IndexMap<String, serde_json::Value>,
) -> IndexMap<String, R3InputMetadata> {
    let mut result = IndexMap::new();
    for (minified_class_name, value) in inputs {
        if let Some(s) = value.as_str() {
             result.insert(minified_class_name.clone(), parse_legacy_input_partial_output(s));
        } else if let Some(arr) = value.as_array() {
              result.insert(minified_class_name.clone(), parse_legacy_input_partial_output_array(arr));
        } else if let Some(obj) = value.as_object() {
             let public_name = obj.get("publicName").and_then(|v| v.as_str()).unwrap_or(&minified_class_name).to_string();
             let is_required = obj.get("isRequired").and_then(|v| v.as_bool()).unwrap_or(false);
             let is_signal = obj.get("isSignal").and_then(|v| v.as_bool()).unwrap_or(false);
             let transform_function = obj.get("transformFunction").map(|v| new_wrapped_node_expr(v.clone()));

             result.insert(minified_class_name.clone(), R3InputMetadata {
                 binding_property_name: public_name,
                 class_property_name: minified_class_name,
                 required: is_required,
                 is_signal,
                 transform_function,
             });
        }
    }
    result
}

fn parse_legacy_input_partial_output(value: &str) -> R3InputMetadata {
    R3InputMetadata {
        binding_property_name: value.to_string(),
        class_property_name: value.to_string(),
        transform_function: None,
        required: false,
        is_signal: false,
    }
}

fn parse_legacy_input_partial_output_array(value: &[serde_json::Value]) -> R3InputMetadata {
     let binding_property_name = value.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
     let class_property_name = value.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
     let transform_function = value.get(2).map(|v| new_wrapped_node_expr(v.clone()));

     R3InputMetadata {
        binding_property_name,
        class_property_name,
        transform_function,
        required: false,
        is_signal: false,
     }
}

fn compute_provided_in(provided_in: Option<crate::compiler_facade_interface::ProvidedIn>) -> crate::injectable_compiler_2::MaybeForwardRefExpression {
    let value = match provided_in {
        Some(crate::compiler_facade_interface::ProvidedIn::Type(type_ref)) => {
             serde_json::Value::String(type_ref)
        },
        Some(crate::compiler_facade_interface::ProvidedIn::Scope(scope)) => {
             serde_json::Value::String(scope)
        },
        None => serde_json::Value::Null,
    };
    
    crate::injectable_compiler_2::MaybeForwardRefExpression {
        expression: crate::injectable_compiler_2::Expression { value },
        forward_ref: false, 
    }
}

fn convert_to_maybe_forward_ref_expression(
    val: Option<serde_json::Value>
) -> Option<crate::injectable_compiler_2::MaybeForwardRefExpression> {
    val.map(|v| crate::injectable_compiler_2::MaybeForwardRefExpression {
        expression: crate::injectable_compiler_2::Expression { value: v },
        forward_ref: false
    })
}

fn convert_to_provider_expression(
    val: Option<serde_json::Value>
) -> Option<crate::injectable_compiler_2::Expression> {
    val.map(|v| crate::injectable_compiler_2::Expression { value: v })
}

fn convert_r3_dependency_metadata_array(
    deps: Option<Vec<R3DependencyMetadataFacade>>
) -> Option<Vec<crate::injectable_compiler_2::R3DependencyMetadata>> {
    deps.map(|d| d.into_iter().map(convert_r3_dependency_metadata).collect())
}

fn convert_declare_dependency_metadata_array(
    deps: Option<Vec<R3DeclareDependencyMetadataFacade>>
) -> Option<Vec<crate::injectable_compiler_2::R3DependencyMetadata>> {
    deps.map(|d| d.into_iter().map(convert_r3_declare_dependency_metadata).collect())
}

fn convert_r3_dependency_metadata(
    dep: R3DependencyMetadataFacade
) -> crate::injectable_compiler_2::R3DependencyMetadata {
    crate::injectable_compiler_2::R3DependencyMetadata {
        token: crate::injectable_compiler_2::Expression { value: dep.token },
        attribute: dep.attribute,
        host: dep.host,
        optional: dep.optional,
        self_dep: dep.self_dep,
        skip_self: dep.skip_self,
    }
}

fn convert_r3_declare_dependency_metadata(
     dep: R3DeclareDependencyMetadataFacade
) -> crate::injectable_compiler_2::R3DependencyMetadata {
    crate::injectable_compiler_2::R3DependencyMetadata {
        token: crate::injectable_compiler_2::Expression { value: dep.token },
        attribute: dep.attribute.and_then(|t| if t { Some("unknown".to_string()) } else { None }), // TODO: Attribute logic in TS handles unknown?
        host: dep.host.unwrap_or(false),
        optional: dep.optional.unwrap_or(false),
        self_dep: dep.self_dep.unwrap_or(false),
        skip_self: dep.skip_self.unwrap_or(false),
    }
}

// Helpers

fn convert_injectable_expression_to_expression(
    expr: crate::injectable_compiler_2::Expression
) -> Expression {
    new_wrapped_node_expr(expr.value)
}

fn convert_special_attributes(attrs: crate::render3::view::compiler::HostSpecialAttributes) -> crate::render3::view::api::R3HostSpecialAttributes {
    crate::render3::view::api::R3HostSpecialAttributes {
        class_attr: attrs.class_attr,
        style_attr: attrs.style_attr,
    }
}

fn convert_to_r3_query_metadata(facade: R3QueryMetadataFacade) -> R3QueryMetadata {
    R3QueryMetadata {
        property_name: facade.property_name,
        first: facade.first,
        predicate: crate::render3::view::api::R3QueryPredicate::Expression(convert_query_predicate(facade.predicate)),
        descendants: facade.descendants,
        read: facade.read.map(|r| new_wrapped_node_expr(r)),
        static_: facade.is_static,
        emit_distinct_changes_only: facade.emit_distinct_changes_only,
        is_signal: facade.is_signal.unwrap_or(false),
    }
}

fn convert_query_predicate(predicate: serde_json::Value) -> crate::render3::util::MaybeForwardRefExpression {
     if let serde_json::Value::Array(_) = predicate {
         create_may_be_forward_ref_expression(new_wrapped_node_expr(predicate), crate::render3::util::ForwardRefHandling::None)
     } else {
         create_may_be_forward_ref_expression(new_wrapped_node_expr(predicate), crate::render3::util::ForwardRefHandling::Wrapped)
     }
}

fn convert_input_metadata_array(inputs: &[crate::compiler_facade_interface::InputMetadata]) -> IndexMap<String, R3InputMetadata> {
    let mut result = IndexMap::new();
    for input in inputs {
        match input {
            crate::compiler_facade_interface::InputMetadata::Simple(name) => {
                 let (binding, class_prop) = parse_mapping_string(name);
                 result.insert(class_prop.clone(), R3InputMetadata {
                     binding_property_name: binding,
                     class_property_name: class_prop,
                     required: false,
                     is_signal: false,
                     transform_function: None,
                 });
            },
            crate::compiler_facade_interface::InputMetadata::Detailed { name, alias, required } => {
                 result.insert(name.clone(), R3InputMetadata {
                     binding_property_name: alias.clone().unwrap_or(name.clone()),
                     class_property_name: name.clone(),
                     required: required.unwrap_or(false),
                     is_signal: false,
                     transform_function: None,
                 });
            }
        }
    }
    result
}

fn convert_inputs_declaration(inputs: Option<serde_json::Value>) -> IndexMap<String, R3InputMetadata> {
    if let Some(serde_json::Value::Object(map)) = inputs {
        let mut index_map = IndexMap::new();
        for (k, v) in map {
             index_map.insert(k, v);
        }
        inputs_partial_metadata_to_input_metadata(index_map)
    } else {
        IndexMap::new()
    }
}

fn convert_outputs_declaration(outputs: Option<serde_json::Value>) -> IndexMap<String, String> {
    if let Some(serde_json::Value::Object(map)) = outputs {
        let mut result = IndexMap::new();
         for (k, v) in map {
             if let Some(s) = v.as_str() {
                 result.insert(k, s.to_string());
             }
         }
         result
    } else {
        IndexMap::new()
    }
}

fn is_host_binding(value: &serde_json::Value) -> bool {
    if let serde_json::Value::Object(map) = value {
        if let Some(serde_json::Value::String(name)) = map.get("ngMetadataName") {
            return name == "HostBinding";
        }
    }
    false
}

fn is_host_listener(value: &serde_json::Value) -> bool {
    if let serde_json::Value::Object(map) = value {
        if let Some(serde_json::Value::String(name)) = map.get("ngMetadataName") {
            return name == "HostListener";
        }
    }
    false
}

fn is_input(value: &serde_json::Value) -> bool {
    if let serde_json::Value::Object(map) = value {
        if let Some(serde_json::Value::String(name)) = map.get("ngMetadataName") {
            return name == "Input";
        }
    }
    false
}

fn is_output(value: &serde_json::Value) -> bool {
    if let serde_json::Value::Object(map) = value {
        if let Some(serde_json::Value::String(name)) = map.get("ngMetadataName") {
            return name == "Output";
        }
    }
    false
}

fn parse_mapping_string(value: &str) -> (String, String) {
    let parts: Vec<&str> = value.splitn(2, ':').map(|s| s.trim()).collect();
    if parts.len() == 2 {
        (parts[1].to_string(), parts[0].to_string())
    } else {
        (parts[0].to_string(), parts[0].to_string())
    }
}

fn extract_host_bindings(
    prop_metadata: &HashMap<String, Vec<serde_json::Value>>,
    source_span: &ParseSourceSpan,
    host: &HashMap<String, String>,
) -> Result<ParsedHostBindings, String> {
    let mut bindings = parse_host_bindings(host);

    let errors = verify_host_bindings(&bindings, source_span);
    if !errors.is_empty() {
        let msg = errors.iter().map(|e| e.msg.clone()).collect::<Vec<String>>().join("\n");
        return Err(msg);
    }
    
    for (field, annotations) in prop_metadata {
        for ann in annotations {
             if is_host_binding(ann) {
                 let host_property_name = ann.get("hostPropertyName").and_then(|v| v.as_str()).unwrap_or(field);
                 bindings.properties.insert(
                     host_property_name.to_string(),
                     get_safe_property_access_string("this", field),
                 );
             } else if is_host_listener(ann) {
                 let event_name = ann.get("eventName").and_then(|v| v.as_str()).unwrap_or(field);
                 let args = ann.get("args").and_then(|v| v.as_array()).map(|arr| {
                     arr.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect::<Vec<String>>()
                 }).unwrap_or_default();
                 bindings.listeners.insert(
                     event_name.to_string(),
                     format!("{}({})", field, args.join(",")),
                 );
             }
        }
    }

    Ok(bindings)
}
