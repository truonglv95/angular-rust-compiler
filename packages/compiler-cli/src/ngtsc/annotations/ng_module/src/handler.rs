// NgModule Decorator Handler
//
// Handles @NgModule decorator processing and compilation.

use super::symbol::NgModuleSymbol;
use crate::ngtsc::annotations::common::src::metadata::R3ClassMetadata;
use crate::ngtsc::reflection::ClassDeclaration;
use crate::ngtsc::transform::src::api::{
    AnalysisOutput, CompileResult, DecoratorHandler, DetectResult, HandlerPrecedence,
};
use angular_compiler::output::abstract_emitter::EmitterVisitorContext;
use angular_compiler::output::abstract_js_emitter::AbstractJsEmitterVisitor;
use angular_compiler::output::output_ast::{ExpressionTrait, ReadVarExpr};
use angular_compiler::output::oxc_emitter::OxcEmitter;
use angular_compiler::render3::r3_factory::{
    compile_factory_function, FactoryTarget, R3ConstructorFactoryMetadata,
    R3FactoryMetadata as R3FacMeta,
};
use angular_compiler::render3::r3_injector_compiler::{
    compile_injector, R3InjectorMetadata as R3InjMeta,
};
use angular_compiler::render3::r3_module_compiler::{
    compile_ng_module, R3NgModuleMetadata as R3ModuleMeta, R3NgModuleMetadataCommon,
    R3NgModuleMetadataGlobal, R3NgModuleMetadataKind, R3SelectorScopeMode,
};
use angular_compiler::render3::util::R3Reference;
use std::any::Any;
use std::collections::HashMap;

/// NgModule analysis data.
#[derive(Debug, Clone)]
pub struct NgModuleAnalysis {
    /// Module metadata for compilation.
    pub module_meta: R3NgModuleMetadata,
    /// Injector metadata.
    pub injector_meta: R3InjectorMetadata,
    /// Factory metadata.
    pub factory_meta: R3FactoryMetadata,
    /// Class metadata for setClassMetadata.
    pub class_metadata: Option<R3ClassMetadata>,
    /// Declarations in this module.
    pub declarations: Vec<String>,
    /// Raw declarations expression.
    pub raw_declarations: Option<String>,
    /// Whether declarations contain forward references.
    pub declarations_have_forward_refs: bool,
    /// Imports.
    pub imports: Vec<String>,
    /// Raw imports expression.
    pub raw_imports: Option<String>,
    /// Exports.
    pub exports: Vec<String>,
    /// Raw exports expression.
    pub raw_exports: Option<String>,
    /// Module ID.
    pub id: Option<String>,
    /// Factory symbol name.
    pub factory_symbol_name: String,
    /// Providers requiring factory.
    pub providers_requiring_factory: Vec<String>,
    /// Raw providers expression.
    pub providers: Option<String>,
    /// Whether remote scopes may need cycle protection.
    pub remote_scopes_may_require_cycle_protection: bool,
}

/// R3 NgModule metadata for compilation.
#[derive(Debug, Clone)]
pub struct R3NgModuleMetadata {
    /// Type reference.
    pub type_ref: String,
    /// Internal type.
    pub internal_type: String,
    /// Bootstrap components.
    pub bootstrap: Vec<String>,
    /// Declarations.
    pub declarations: Vec<String>,
    /// Imports (other modules).
    pub imports: Vec<String>,
    /// Exports.
    pub exports: Vec<String>,
    /// Schemas.
    pub schemas: Vec<String>,
    /// Module ID.
    pub id: Option<String>,
    /// Compilation mode.
    pub contains_forward_decls: bool,
    /// Whether selectorless directives are enabled.
    pub selectorless_enabled: bool,
}

impl R3NgModuleMetadata {
    pub fn new(type_ref: impl Into<String>) -> Self {
        let t = type_ref.into();
        Self {
            type_ref: t.clone(),
            internal_type: t,
            bootstrap: Vec::new(),
            declarations: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            schemas: Vec::new(),
            id: None,
            contains_forward_decls: false,
            selectorless_enabled: false,
        }
    }
}

/// R3 Injector metadata.
#[derive(Debug, Clone)]
pub struct R3InjectorMetadata {
    /// Type reference.
    pub type_ref: String,
    /// Providers.
    pub providers: Option<String>,
    /// Imports for the injector.
    pub imports: Vec<String>,
}

impl R3InjectorMetadata {
    pub fn new(type_ref: impl Into<String>) -> Self {
        Self {
            type_ref: type_ref.into(),
            providers: None,
            imports: Vec::new(),
        }
    }
}

/// R3 Factory metadata for NgModule.
#[derive(Debug, Clone)]
pub struct R3FactoryMetadata {
    /// Type name.
    pub name: String,
    /// Type reference.
    pub type_ref: String,
    /// Dependencies.
    pub deps: Option<Vec<crate::ngtsc::annotations::common::src::di::R3DependencyMetadata>>,
    /// Target (NgModule).
    pub target: crate::ngtsc::annotations::common::src::factory::FactoryTarget,
}

impl R3FactoryMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        let n = name.into();
        Self {
            name: n.clone(),
            type_ref: n,
            deps: None,
            target: crate::ngtsc::annotations::common::src::factory::FactoryTarget::NgModule,
        }
    }
}

/// NgModule resolution data.
#[derive(Debug, Clone)]
pub struct NgModuleResolution {
    /// Injector imports for compilation.
    pub injector_imports: Vec<String>,
}

/// NgModule decorator handler.
pub struct NgModuleDecoratorHandler {
    #[allow(dead_code)]
    is_core: bool,
}

impl NgModuleDecoratorHandler {
    pub fn new(is_core: bool) -> Self {
        Self { is_core }
    }

    /// Extract declarations from analysis.
    #[allow(dead_code)]
    fn resolve_type_list(
        &self,
        raw_expr: Option<&str>,
        _allow_forward_refs: bool,
    ) -> (Vec<String>, bool) {
        // Simplified - would parse expression to get references
        let references = Vec::new();
        let has_forward_refs = raw_expr.map_or(false, |e| e.contains("forwardRef"));
        (references, has_forward_refs)
    }
}

impl DecoratorHandler<NgModuleAnalysis, NgModuleAnalysis, NgModuleSymbol, NgModuleResolution>
    for NgModuleDecoratorHandler
{
    fn name(&self) -> &str {
        "NgModuleDecoratorHandler"
    }

    fn precedence(&self) -> HandlerPrecedence {
        HandlerPrecedence::Primary
    }

    fn detect(
        &self,
        _node: &ClassDeclaration,
        decorators: &[String],
    ) -> Option<DetectResult<NgModuleAnalysis>> {
        let has_ng_module = decorators.iter().any(|d| d == "NgModule");
        if has_ng_module {
            None // Would return detect result
        } else {
            None
        }
    }

    fn analyze(
        &self,
        _node: &ClassDeclaration,
        _metadata: &NgModuleAnalysis,
    ) -> AnalysisOutput<NgModuleAnalysis> {
        AnalysisOutput {
            analysis: None,
            diagnostics: None,
        }
    }

    fn symbol(
        &self,
        _node: &ClassDeclaration,
        analysis: &NgModuleAnalysis,
    ) -> Option<NgModuleSymbol> {
        let has_providers = analysis.providers.is_some();
        Some(NgModuleSymbol::new(
            &analysis.factory_symbol_name,
            has_providers,
        ))
    }

    fn compile_full(
        &self,
        _node: &ClassDeclaration,
        analysis: &NgModuleAnalysis,
        _resolution: Option<&NgModuleResolution>,
        _constant_pool: &mut crate::ngtsc::transform::src::api::ConstantPool,
    ) -> Vec<CompileResult> {
        let mut imports_map = HashMap::new();
        imports_map.insert("@angular/core".to_string(), "i0".to_string());

        let type_ref = R3Reference {
            value: angular_compiler::output::output_ast::Expression::ReadVar(ReadVarExpr {
                name: analysis.module_meta.type_ref.clone(),
                type_: None,
                source_span: None,
            }),
            type_expr: angular_compiler::output::output_ast::Expression::ReadVar(ReadVarExpr {
                name: analysis.module_meta.type_ref.clone(),
                type_: None,
                source_span: None,
            }),
        };

        // 1. Compile NgModule (ɵmod)
        let r3_module_meta = R3ModuleMeta::Global(R3NgModuleMetadataGlobal {
            common: R3NgModuleMetadataCommon {
                kind: R3NgModuleMetadataKind::Global,
                type_: type_ref.clone(),
                selector_scope_mode: R3SelectorScopeMode::Inline,
                schemas: None,
                id: None,
            },
            bootstrap: Vec::new(),
            declarations: Vec::new(),
            public_declaration_types: None,
            imports: Vec::new(),
            include_import_types: true,
            exports: Vec::new(),
            contains_forward_decls: false,
        });
        let compiled_mod = compile_ng_module(&r3_module_meta);

        // 2. Compile Injector (ɵinj)
        let r3_inj_meta = R3InjMeta {
            name: analysis.module_meta.type_ref.clone(),
            type_: type_ref.clone(),
            providers: None,
            imports: Vec::new(),
        };
        let compiled_inj = compile_injector(&r3_inj_meta);

        // 3. Compile Factory (ɵfac)
        let r3_fac_meta = R3FacMeta::Constructor(R3ConstructorFactoryMetadata {
            name: analysis.module_meta.type_ref.clone(),
            type_: type_ref,
            type_argument_count: 0,
            target: FactoryTarget::NgModule,
            deps: None,
        });
        let compiled_fac = compile_factory_function(&r3_fac_meta);

        // Helper for emitting AST
        let emit_ast = |expr: &angular_compiler::output::output_ast::Expression,
                        imports: &HashMap<String, String>|
         -> Option<String> {
            let ast_allocator = oxc_allocator::Allocator::default();
            let oxc_emitter = OxcEmitter::with_imports(&ast_allocator, imports.clone());
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let oxc_expr = oxc_emitter.emit_expression(expr);
                let mut body = oxc_allocator::Vec::new_in(&ast_allocator);
                body.push(oxc_ast::ast::Statement::ExpressionStatement(
                    oxc_allocator::Box::new_in(
                        oxc_ast::ast::ExpressionStatement {
                            span: oxc_span::Span::default(),
                            expression: oxc_expr,
                        },
                        &ast_allocator,
                    ),
                ));
                let program = oxc_ast::ast::Program {
                    span: oxc_span::Span::default(),
                    source_type: oxc_span::SourceType::mjs(),
                    source_text: "",
                    comments: oxc_allocator::Vec::new_in(&ast_allocator),
                    hashbang: None,
                    directives: oxc_allocator::Vec::new_in(&ast_allocator),
                    body,
                    scope_id: std::cell::Cell::new(None),
                };
                let codegen = oxc_codegen::Codegen::new();
                let mut code = codegen.build(&program).code;
                code = code.trim_end().to_string();
                if code.ends_with(';') {
                    code.pop();
                }
                code
            }));
            match result {
                Ok(code) => Some(code),
                Err(_) => None,
            }
        };

        // Helper for emitting String fallback
        let emit_string = |expr: &angular_compiler::output::output_ast::Expression,
                           imports: &HashMap<String, String>|
         -> String {
            let mut emitter = AbstractJsEmitterVisitor::with_imports(imports.clone());
            let mut ctx = EmitterVisitorContext::create_root();
            {
                let context: &mut dyn Any = &mut ctx;
                expr.visit_expression(&mut emitter, context);
            }
            ctx.to_source()
        };

        let mod_initializer = emit_string(&compiled_mod.expression, &imports_map);
        let inj_initializer = emit_string(&compiled_inj.expression, &imports_map);
        let fac_initializer = emit_string(&compiled_fac.expression, &imports_map);

        let mod_ast_code = emit_ast(&compiled_mod.expression, &imports_map);
        let inj_ast_code = emit_ast(&compiled_inj.expression, &imports_map);
        let fac_ast_code = emit_ast(&compiled_fac.expression, &imports_map);

        vec![
            CompileResult {
                name: "ɵmod".to_string(),
                initializer: Some(mod_initializer),
                initializer_ast_code: mod_ast_code,
                statements: vec![],
                type_desc: "NgModuleDef".to_string(),
                deferrable_imports: None,
                diagnostics: Vec::new(),
                additional_imports: Vec::new(),
            },
            CompileResult {
                name: "ɵinj".to_string(),
                initializer: Some(inj_initializer),
                initializer_ast_code: inj_ast_code,
                statements: vec![],
                type_desc: "InjectorDef".to_string(),
                deferrable_imports: None,
                diagnostics: Vec::new(),
                additional_imports: Vec::new(),
            },
            CompileResult {
                name: "ɵfac".to_string(),
                initializer: Some(fac_initializer),
                initializer_ast_code: fac_ast_code,
                statements: vec![],
                type_desc: "Factory".to_string(),
                deferrable_imports: None,
                diagnostics: Vec::new(),
                additional_imports: Vec::new(),
            },
        ]
    }
}
