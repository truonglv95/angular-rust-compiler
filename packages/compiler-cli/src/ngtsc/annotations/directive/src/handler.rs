// Directive Decorator Handler
//
// Handles @Directive decorator processing.

use crate::ngtsc::transform::src::api::{
    DecoratorHandler, HandlerPrecedence, DetectResult, AnalysisOutput, CompileResult,
};
use crate::ngtsc::reflection::ClassDeclaration;
use super::symbol::DirectiveSymbol;
use angular_compiler::render3::r3_identifiers::Identifiers;

/// Field decorators that indicate Angular-specific behavior.
pub const FIELD_DECORATORS: &[&str] = &[
    "Input",
    "Output", 
    "ViewChild",
    "ViewChildren",
    "ContentChild",
    "ContentChildren",
    "HostBinding",
    "HostListener",
];

/// Lifecycle hooks that indicate Angular component/directive.
pub const LIFECYCLE_HOOKS: &[&str] = &[
    "ngOnChanges",
    "ngOnInit",
    "ngOnDestroy",
    "ngDoCheck",
    "ngAfterViewInit",
    "ngAfterViewChecked",
    "ngAfterContentInit",
    "ngAfterContentChecked",
];

/// Data collected during directive analysis.
#[derive(Debug, Clone)]
pub struct DirectiveHandlerData {
    /// Base class reference.
    pub base_class: Option<String>,
    /// Type check metadata.
    pub type_check_meta: super::symbol::DirectiveTypeCheckMeta,
    /// Directive metadata for compilation.
    pub meta: R3DirectiveMetadata,
    /// Class metadata for setClassMetadata.
    pub class_metadata: Option<crate::ngtsc::annotations::common::src::metadata::R3ClassMetadata>,
    /// Whether the directive is poisoned (has errors).
    pub is_poisoned: bool,
    /// Whether this is a structural directive.
    pub is_structural: bool,
    /// Host directives metadata.
    pub host_directives: Option<Vec<HostDirectiveMeta>>,
}

/// R3 Directive metadata for code generation.
#[derive(Debug, Clone)]
pub struct R3DirectiveMetadata {
    /// Directive name.
    pub name: String,
    /// Selector.
    pub selector: Option<String>,
    /// Export as names.
    pub export_as: Option<Vec<String>>,
    /// Inputs.
    pub inputs: Vec<DirectiveInput>,
    /// Outputs.
    pub outputs: Vec<DirectiveOutput>,
    /// Host bindings.
    pub host: HostBindings,
    /// Type reference.
    pub type_ref: String,
    /// Internal type reference.
    pub internal_type: String,
    /// Whether standalone.
    pub is_standalone: bool,
    /// Whether a signal directive.
    pub is_signal: bool,
    /// Providers.
    pub providers: Option<String>,
    /// Queries.
    pub queries: Vec<DirectiveQuery>,
    /// View queries.
    pub view_queries: Vec<DirectiveQuery>,
}

impl R3DirectiveMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        let n = name.into();
        Self {
            name: n.clone(),
            selector: None,
            export_as: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            host: HostBindings::default(),
            type_ref: n.clone(),
            internal_type: n,
            is_standalone: true,
            is_signal: false,
            providers: None,
            queries: Vec::new(),
            view_queries: Vec::new(),
        }
    }
}

/// Directive input metadata.
#[derive(Debug, Clone)]
pub struct DirectiveInput {
    pub class_property_name: String,
    pub binding_property_name: String,
    pub required: bool,
    pub is_signal: bool,
    pub transform: Option<String>,
}

/// Directive output metadata.
#[derive(Debug, Clone)]
pub struct DirectiveOutput {
    pub class_property_name: String,
    pub binding_property_name: String,
}

/// Directive query metadata.
#[derive(Debug, Clone)]
pub struct DirectiveQuery {
    pub property_name: String,
    pub selector: String,
    pub first: bool,
    pub descendants: bool,
    pub read: Option<String>,
    pub is_signal: bool,
}

/// Host bindings for a directive.
#[derive(Debug, Clone, Default)]
pub struct HostBindings {
    pub properties: Vec<(String, String)>,
    pub attributes: Vec<(String, String)>,
    pub listeners: Vec<(String, String)>,
    pub class_attr: Option<String>,
    pub style_attr: Option<String>,
}

/// Host directive metadata.
#[derive(Debug, Clone)]
pub struct HostDirectiveMeta {
    pub directive: String,
    pub inputs: Vec<(String, String)>,
    pub outputs: Vec<(String, String)>,
    pub is_forward_reference: bool,
}

/// Directive decorator handler.
pub struct DirectiveDecoratorHandler {
    #[allow(dead_code)]
    is_core: bool,
    strict_standalone: bool,
    #[allow(dead_code)]
    implicit_standalone: bool,
}

impl DirectiveDecoratorHandler {
    pub fn new(is_core: bool) -> Self {
        Self {
            is_core,
            strict_standalone: false,
            implicit_standalone: true,
        }
    }
    
    pub fn with_strict_standalone(mut self, strict: bool) -> Self {
        self.strict_standalone = strict;
        self
    }
    
    /// Find class fields with Angular features.
    pub fn find_class_field_with_angular_features(
        &self,
        member_names: &[String],
        member_decorators: &[(String, Vec<String>)],
    ) -> Option<String> {
        // Check for lifecycle hooks
        for name in member_names {
            if LIFECYCLE_HOOKS.contains(&name.as_str()) {
                return Some(name.clone());
            }
        }
        
        // Check for field decorators
        for (name, decorators) in member_decorators {
            for dec in decorators {
                if FIELD_DECORATORS.contains(&dec.as_str()) {
                    return Some(name.clone());
                }
            }
        }
        
        None
    }
}

impl DecoratorHandler<DirectiveHandlerData, DirectiveHandlerData, DirectiveSymbol, ()> for DirectiveDecoratorHandler {
    fn name(&self) -> &str {
        "DirectiveDecoratorHandler"
    }
    
    fn precedence(&self) -> HandlerPrecedence {
        HandlerPrecedence::Primary
    }
    
    fn detect(&self, _node: &ClassDeclaration, decorators: &[String]) -> Option<DetectResult<DirectiveHandlerData>> {
        // Look for @Directive decorator
        let has_directive = decorators.iter().any(|d| d == "Directive");
        if has_directive {
            // Would return detect result - simplified for now
            None
        } else {
            None
        }
    }
    
    fn analyze(&self, _node: &ClassDeclaration, _metadata: &DirectiveHandlerData) -> AnalysisOutput<DirectiveHandlerData> {
        AnalysisOutput {
            analysis: None,
            diagnostics: None,
        }
    }
    
    fn symbol(&self, _node: &ClassDeclaration, _analysis: &DirectiveHandlerData) -> Option<DirectiveSymbol> {
        None
    }
    
    fn compile_full(
        &self,
        _node: &ClassDeclaration,
        analysis: &DirectiveHandlerData,
        _resolution: Option<&()>,
        _constant_pool: &mut crate::ngtsc::transform::src::api::ConstantPool,
    ) -> Vec<CompileResult> {
        let meta = &analysis.meta;
        
        // Use R3Identifiers for define_directive
        let define_directive_name = Identifiers::define_directive().name.unwrap_or_default();
        
        let definition = format!(
            "static ɵdir = {}({{ type: {}, selectors: [[\"{}\"]] }});",
            define_directive_name,
            meta.name,
            meta.selector.as_deref().unwrap_or("")
        );
        
        vec![CompileResult {
            name: "ɵdir".to_string(),
            initializer: Some(definition),
            statements: vec![],
            type_desc: "DirectiveDef".to_string(),
            deferrable_imports: None,
        }]
    }
}


use crate::ngtsc::metadata::DirectiveMetadata;

impl DirectiveDecoratorHandler {
    pub fn compile_ivy(&self, analysis: &DirectiveMetadata) -> Vec<CompileResult> {
        let define_directive_name = Identifiers::define_directive().name.unwrap_or_default();
        
        // ɵfac
        let fac_definition = format!(
            "(t) => new (t || {})()",
            analysis.name
        );
        let fac_result = CompileResult {
            name: "ɵfac".to_string(),
            initializer: Some(fac_definition),
            statements: vec![],
            type_desc: "FactoryDef".to_string(),
            deferrable_imports: None,
        };

        // ɵdir
        // Construct definition object with i0 prefix
        let definition = format!(
            "i0.{}({{ type: {}, selectors: [[\"{}\"]]{}{}{} }})",
            define_directive_name,
            analysis.name,
            analysis.selector.as_deref().unwrap_or(""),
            if !analysis.inputs.is_empty() {
                let mut inputs_str = String::from(", inputs: {");
                for (i, (prop, input)) in analysis.inputs.iter().enumerate() {
                    if i > 0 { inputs_str.push_str(", "); }
                    if input.is_signal {
                        inputs_str.push_str(&format!("{}: [1, \"{}\"]", prop, input.binding_property_name));
                    } else {
                        inputs_str.push_str(&format!("{}: \"{}\"", prop, input.binding_property_name));
                    }
                }
                inputs_str.push_str("}");
                inputs_str
            } else {
                String::new()
            },
            if !analysis.outputs.is_empty() {
                let mut outputs_str = String::from(", outputs: {");
                for (i, (prop, binding)) in analysis.outputs.iter().enumerate() {
                    if i > 0 { outputs_str.push_str(", "); }
                    outputs_str.push_str(&format!("{}: \"{}\"", prop, binding.binding_property_name));
                }
                outputs_str.push_str("}");
                outputs_str
            } else {
                String::new()
            },
            "" // Placeholder for other fields if needed
        );
        
        let dir_result = CompileResult {
            name: "ɵdir".to_string(),
            initializer: Some(definition),
            statements: vec![],
            type_desc: "DirectiveDef".to_string(),
            deferrable_imports: None,
        };

        vec![fac_result, dir_result]
    }
}
