// Transform module - Core compilation infrastructure
//
// This module provides the transform pipeline for Angular compilation,
// including decorator handling, trait management, and code generation.

pub mod api;
pub mod trait_;
pub mod compilation;
pub mod declaration;
pub mod alias;
pub mod transform;

// Re-export commonly used types
pub use api::{
    CompilationMode, CompileResult, DecoratorHandler, HandlerPrecedence,
    AnalysisOutput, ResolveResult, DetectResult, ConstantPool,
};
pub use trait_::{TraitState, Trait, TraitFactory};
pub use compilation::{TraitCompiler, ClassRecord};
pub use declaration::{DtsTransformRegistry, IvyDeclarationDtsTransform, IvyDeclarationField};
pub use alias::{AliasTransformConfig, ExportAlias};
pub use transform::{IvyCompilationVisitor, IvyTransformationVisitor, IvyTransformConfig};
