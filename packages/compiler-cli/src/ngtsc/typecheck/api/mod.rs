// TypeCheck API Module

pub mod api;
pub mod checker;
pub mod symbols;

// Re-exports
pub use api::{
    TypeCheckingConfig, ControlFlowPrevention,
    TypeCheckBlockMetadata, TcbLocation,
    TypeCheckOp, TypeCheckContext,
    PendingTypeCheckBlock, TypeCheckError,
};
pub use checker::{TemplateTypeChecker, TypeCheckResult};
pub use symbols::{
    TemplateSymbol, DirectiveSymbolInfo, PipeSymbolInfo,
    VariableSymbolInfo, VariableKind, ElementSymbolInfo,
    ReferenceSymbolInfo, ExpressionSymbolInfo,
    InputBinding, OutputBinding,
};
