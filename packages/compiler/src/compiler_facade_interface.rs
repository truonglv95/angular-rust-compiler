//! Compiler Facade Interface
//!
//! Corresponds to packages/compiler/src/compiler_facade_interface.ts (405 lines)
//!
//! A set of interfaces which are shared between `@angular/core` and `@angular/compiler`
//! to allow for late binding of `@angular/compiler` for JIT purposes.
//!
//! This defines the contract between Angular runtime and the compiler.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Factory target types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FactoryTarget {
    Directive = 0,
    Component = 1,
    Injectable = 2,
    Pipe = 3,
    NgModule = 4,
}

/// View encapsulation modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ViewEncapsulation {
    Emulated = 0,
    None = 2,
    ShadowDom = 3,
}

/// Change detection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ChangeDetectionStrategy {
    OnPush = 0,
    Default = 1,
}

/// Template dependency kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum R3TemplateDependencyKind {
    Directive = 0,
    Pipe = 1,
    NgModule = 2,
}

/// Opaque value (can be anything)
pub type OpaqueValue = serde_json::Value;

/// Type reference (function/class)
pub type TypeRef = String;

/// Provider
pub type Provider = OpaqueValue;

/// Core environment (passed from Angular core)
pub type CoreEnvironment = HashMap<String, OpaqueValue>;

/// Parse source span (for error reporting)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseSourceSpan {
    pub start: ParseLocation,
    pub end: ParseLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseLocation {
    pub file: ParseSourceFile,
    pub offset: usize,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseSourceFile {
    pub content: String,
    pub url: String,
}

/// Dependency metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DependencyMetadataFacade {
    pub token: OpaqueValue,
    pub attribute: Option<String>,
    pub host: bool,
    pub optional: bool,
    #[serde(rename = "self")]
    pub self_dep: bool,
    pub skip_self: bool,
}

/// Declare dependency metadata (with optional fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DeclareDependencyMetadataFacade {
    pub token: OpaqueValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "self")]
    pub self_dep: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_self: Option<bool>,
}

/// Pipe metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3PipeMetadataFacade {
    pub name: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    pub pipe_name: Option<String>,
    pub pure: bool,
    pub is_standalone: bool,
}

/// Declare pipe metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DeclarePipeFacade {
    pub version: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    pub name: String,
    pub pure: Option<bool>,
    pub is_standalone: Option<bool>,
}

/// Injectable metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3InjectableMetadataFacade {
    pub name: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    pub type_argument_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provided_in: Option<ProvidedIn>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_class: Option<OpaqueValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_factory: Option<OpaqueValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_existing: Option<OpaqueValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_value: Option<OpaqueValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deps: Option<Vec<R3DependencyMetadataFacade>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProvidedIn {
    Type(TypeRef),
    Scope(String),  // 'root', 'platform', 'any'
}

/// Declare injectable metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DeclareInjectableFacade {
    pub version: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provided_in: Option<ProvidedIn>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_class: Option<OpaqueValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_factory: Option<OpaqueValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_existing: Option<OpaqueValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_value: Option<OpaqueValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deps: Option<Vec<R3DeclareDependencyMetadataFacade>>,
}

/// NgModule metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3NgModuleMetadataFacade {
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    pub bootstrap: Vec<TypeRef>,
    pub declarations: Vec<TypeRef>,
    pub imports: Vec<TypeRef>,
    pub exports: Vec<TypeRef>,
    pub schemas: Option<Vec<SchemaMetadata>>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    pub name: String,
}

/// Declare NgModule metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DeclareNgModuleFacade {
    pub version: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap: Option<Vec<TypeRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declarations: Option<Vec<TypeRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<Vec<TypeRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exports: Option<Vec<TypeRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<Vec<SchemaMetadata>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<OpaqueValue>,
}

/// Injector metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3InjectorMetadataFacade {
    pub name: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    pub providers: Vec<Provider>,
    pub imports: Vec<OpaqueValue>,
}

/// Query metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3QueryMetadataFacade {
    pub property_name: String,
    pub predicate: OpaqueValue,
    pub descendants: bool,
    pub first: bool,
    pub read: Option<OpaqueValue>,
    #[serde(rename = "static")]
    pub is_static: bool,
    pub emit_distinct_changes_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_signal: Option<bool>,
}

/// Host directive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3HostDirectiveMetadataFacade {
    pub directive: TypeRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<String>>,
}

/// Input metadata (can be string or object)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputMetadata {
    Simple(String),
    Detailed {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        alias: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        required: Option<bool>,
    },
}

/// Directive metadata facade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DirectiveMetadataFacade {
    pub name: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    pub type_source_span: ParseSourceSpan,
    pub selector: Option<String>,
    pub queries: Vec<R3QueryMetadataFacade>,
    pub host: HashMap<String, String>,
    pub prop_metadata: HashMap<String, Vec<OpaqueValue>>,
    pub lifecycle: DirectiveLifecycle,
    pub inputs: Vec<InputMetadata>,
    pub outputs: Vec<String>,
    pub uses_inheritance: bool,
    pub export_as: Option<Vec<String>>,
    pub providers: Option<Vec<Provider>>,
    pub view_queries: Vec<R3QueryMetadataFacade>,
    pub is_standalone: bool,
    pub host_directives: Option<Vec<R3HostDirectiveMetadataFacade>>,
    pub is_signal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectiveLifecycle {
    pub uses_on_changes: bool,
}

/// Component metadata facade (extends Directive)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3ComponentMetadataFacade {
    // Directive fields
    pub name: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    pub type_source_span: ParseSourceSpan,
    pub selector: Option<String>,
    pub queries: Vec<R3QueryMetadataFacade>,
    pub host: HashMap<String, String>,
    pub prop_metadata: HashMap<String, Vec<OpaqueValue>>,
    pub lifecycle: DirectiveLifecycle,
    pub inputs: Vec<InputMetadata>,
    pub outputs: Vec<String>,
    pub uses_inheritance: bool,
    pub export_as: Option<Vec<String>>,
    pub providers: Option<Vec<Provider>>,
    pub view_queries: Vec<R3QueryMetadataFacade>,
    pub is_standalone: bool,
    pub host_directives: Option<Vec<R3HostDirectiveMetadataFacade>>,
    pub is_signal: bool,
    
    // Component-specific fields
    pub template: String,
    pub preserve_whitespaces: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animations: Option<Vec<OpaqueValue>>,
    pub declarations: Vec<R3TemplateDependencyFacade>,
    pub styles: Vec<String>,
    pub encapsulation: ViewEncapsulation,
    pub change_detection: Option<ChangeDetectionStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_providers: Option<Vec<Provider>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpolation: Option<(String, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_blocks: Option<serde_json::Value>,
}

/// Template dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3TemplateDependencyFacade {
    pub kind: R3TemplateDependencyKind,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
}

/// Declare directive facade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DeclareDirectiveFacade {
    pub version: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    pub selector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<Vec<Provider>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries: Option<Vec<R3QueryMetadataFacade>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_queries: Option<Vec<R3QueryMetadataFacade>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_as: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses_inheritance: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_standalone: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_signal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_directives: Option<Vec<R3HostDirectiveMetadataFacade>>,
}

/// Declare component facade (extends Directive)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DeclareComponentFacade {
    // Directive fields
    pub version: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    pub selector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<Vec<Provider>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries: Option<Vec<R3QueryMetadataFacade>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_queries: Option<Vec<R3QueryMetadataFacade>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_as: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses_inheritance: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_standalone: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_signal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_directives: Option<Vec<R3HostDirectiveMetadataFacade>>,
    
    // Component-specific fields
    pub template: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_inline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub styles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<R3DeclareTemplateDependencyFacade>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_providers: Option<Vec<Provider>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animations: Option<Vec<OpaqueValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_detection: Option<ChangeDetectionStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encapsulation: Option<ViewEncapsulation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpolation: Option<(String, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_whitespaces: Option<bool>,
}

/// Template dependency facade
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum R3DeclareTemplateDependencyFacade {
    Directive(R3DeclareDirectiveDependencyFacade),
    Pipe(R3DeclarePipeDependencyFacade),
    NgModule(R3DeclareNgModuleDependencyFacade),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DeclareDirectiveDependencyFacade {
    pub kind: R3TemplateDependencyKind,  // Should be Directive
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    pub selector: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_as: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DeclarePipeDependencyFacade {
    pub kind: R3TemplateDependencyKind,  // Should be Pipe
    pub name: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DeclareNgModuleDependencyFacade {
    pub kind: R3TemplateDependencyKind,  // Should be NgModule
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
}

/// Factory metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3FactoryDefMetadataFacade {
    pub name: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    pub deps: Option<Vec<R3DependencyMetadataFacade>>,
    pub target: FactoryTarget,
}

/// Declare factory metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DeclareFactoryFacade {
    pub version: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deps: Option<Vec<R3DeclareDependencyMetadataFacade>>,
    pub target: FactoryTarget,
}

/// Declare injector metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DeclareInjectorFacade {
    pub version: String,
    #[serde(rename = "type")]
    pub type_ref: TypeRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<Vec<Provider>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<Vec<OpaqueValue>>,
}

/// Compiler Facade - Main interface
///
/// Corresponds to TypeScript `CompilerFacade` interface
/// Defines all compilation methods available to Angular runtime
pub trait CompilerFacade {
    fn compile_pipe(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        meta: R3PipeMetadataFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_pipe_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        declaration: R3DeclarePipeFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_injectable(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        meta: R3InjectableMetadataFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_injectable_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        meta: R3DeclareInjectableFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_injector(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        meta: R3InjectorMetadataFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_injector_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        declaration: R3DeclareInjectorFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_ng_module(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        meta: R3NgModuleMetadataFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_ng_module_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        declaration: R3DeclareNgModuleFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_directive(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        meta: R3DirectiveMetadataFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_directive_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        declaration: R3DeclareDirectiveFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_component(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        meta: R3ComponentMetadataFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_component_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        declaration: R3DeclareComponentFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_factory(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        meta: R3FactoryDefMetadataFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn compile_factory_declaration(
        &self,
        angular_core_env: CoreEnvironment,
        source_map_url: String,
        meta: R3DeclareFactoryFacade,
    ) -> Result<OpaqueValue, String>;
    
    fn create_parse_source_span(
        &self,
        kind: String,
        type_name: String,
        source_url: String,
    ) -> ParseSourceSpan;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_target_values() {
        assert_eq!(FactoryTarget::Directive as u8, 0);
        assert_eq!(FactoryTarget::Component as u8, 1);
        assert_eq!(FactoryTarget::Injectable as u8, 2);
        assert_eq!(FactoryTarget::Pipe as u8, 3);
        assert_eq!(FactoryTarget::NgModule as u8, 4);
    }

    #[test]
    fn test_view_encapsulation_values() {
        assert_eq!(ViewEncapsulation::Emulated as u8, 0);
        assert_eq!(ViewEncapsulation::None as u8, 2);
        assert_eq!(ViewEncapsulation::ShadowDom as u8, 3);
    }

    #[test]
    fn test_serde_pipe_metadata() {
        let pipe = R3PipeMetadataFacade {
            name: "TestPipe".to_string(),
            type_ref: "TestPipe".to_string(),
            pipe_name: Some("testPipe".to_string()),
            pure: true,
            is_standalone: false,
        };
        
        let json = serde_json::to_string(&pipe).unwrap();
        assert!(json.contains("TestPipe"));
        
        let deserialized: R3PipeMetadataFacade = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "TestPipe");
    }
}
