// NgModule Decorator Handler
//
// Handles @NgModule decorator processing and compilation.

use super::symbol::NgModuleSymbol;
use crate::ngtsc::annotations::common::src::metadata::R3ClassMetadata;
use crate::ngtsc::reflection::ClassDeclaration;
use crate::ngtsc::transform::src::api::{
    AnalysisOutput, CompileResult, DecoratorHandler, DetectResult, HandlerPrecedence,
};
use angular_compiler::render3::r3_identifiers::Identifiers;

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
        let meta = &analysis.module_meta;

        // Use R3Identifiers
        let define_ng_module_name = Identifiers::define_ng_module().name.unwrap_or_default();
        let define_injector_name = Identifiers::define_injector().name.unwrap_or_default();

        // Generate ɵmod definition
        let mod_def = format!(
            "static ɵmod = {}({{ type: {} }});",
            define_ng_module_name, meta.type_ref
        );

        // Generate ɵinj definition
        let inj_def = format!("static ɵinj = {}({{}});", define_injector_name);

        // Generate factory
        let fac_def = format!(
            "static ɵfac = function {}Factory(t) {{ return new (t || {})(); }};",
            analysis.factory_meta.name, analysis.factory_meta.name
        );

        vec![
            CompileResult {
                name: "ɵmod".to_string(),
                initializer: Some(mod_def),
                statements: vec![],
                type_desc: "NgModuleDef".to_string(),
                deferrable_imports: None,
                diagnostics: Vec::new(),
                additional_imports: Vec::new(),
            },
            CompileResult {
                name: "ɵinj".to_string(),
                initializer: Some(inj_def),
                statements: vec![],
                type_desc: "InjectorDef".to_string(),
                deferrable_imports: None,
                diagnostics: Vec::new(),
                additional_imports: Vec::new(),
            },
            CompileResult {
                name: "ɵfac".to_string(),
                initializer: Some(fac_def),
                statements: vec![],
                type_desc: "Factory".to_string(),
                deferrable_imports: None,
                diagnostics: Vec::new(),
                additional_imports: Vec::new(),
            },
        ]
    }
}
