// TypeCheck API
//
// Public API types for template type-checking.

use std::collections::HashMap;

/// Configuration for type-checking.
#[derive(Debug, Clone)]
pub struct TypeCheckingConfig {
    /// Whether to apply fullTemplateTypeCheck mode.
    pub apply_full_template_type_check_mode: bool,
    /// Whether to check variable types.
    pub check_type_of_inputs: bool,
    /// Whether to check directives.
    pub check_type_of_outputs: bool,
    /// Whether to use strict null checks.
    pub strict_null_checks: bool,
    /// Whether to honor access modifiers.
    pub honor_access_modifiers: bool,
    /// Whether to check queries.
    pub check_type_of_queries: bool,
    /// Whether to check two-way bindings.
    pub check_type_of_two_way_bindings: bool,
    /// DOM schemas to use.
    pub check_type_of_dom_references: bool,
    /// Whether to check pipe types.
    pub check_type_of_pipes: bool,
    /// Whether to suggest fixes for template errors.
    pub suggest_fixes_for_template_errors: bool,
    /// Use any type for controls.
    pub control_flow_preventing_content_projection: ControlFlowPrevention,
}

impl Default for TypeCheckingConfig {
    fn default() -> Self {
        Self {
            apply_full_template_type_check_mode: false,
            check_type_of_inputs: true,
            check_type_of_outputs: true,
            strict_null_checks: true,
            honor_access_modifiers: true,
            check_type_of_queries: true,
            check_type_of_two_way_bindings: true,
            check_type_of_dom_references: true,
            check_type_of_pipes: true,
            suggest_fixes_for_template_errors: false,
            control_flow_preventing_content_projection: ControlFlowPrevention::Warning,
        }
    }
}

/// Options for control flow content projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlowPrevention {
    /// No error or warning.
    Off,
    /// Warning only.
    Warning,
    /// Error.
    Error,
}

/// Type-check block metadata.
#[derive(Debug, Clone)]
pub struct TypeCheckBlockMetadata {
    /// Template node ID to TCB location mapping.
    pub template_mappings: HashMap<String, TcbLocation>,
    /// Inline TCB bound.
    pub is_inline: bool,
}

/// Location in a type-check block.
#[derive(Debug, Clone)]
pub struct TcbLocation {
    /// File path.
    pub file: String,
    /// Start offset.
    pub start: usize,
    /// End offset.
    pub end: usize,
}

/// A template type-check operation.
pub trait TypeCheckOp {
    /// Execute the type-check operation.
    fn execute(&self, context: &mut TypeCheckContext);
}

/// Context for type-checking operations.
#[derive(Debug, Default)]
pub struct TypeCheckContext {
    /// Pending type-check blocks.
    pending_tcbs: Vec<PendingTypeCheckBlock>,
    /// Errors collected during type-checking.
    errors: Vec<TypeCheckError>,
}

impl TypeCheckContext {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a pending type-check block.
    pub fn add_pending_tcb(&mut self, tcb: PendingTypeCheckBlock) {
        self.pending_tcbs.push(tcb);
    }
    
    /// Add an error.
    pub fn add_error(&mut self, error: TypeCheckError) {
        self.errors.push(error);
    }
    
    /// Get all errors.
    pub fn errors(&self) -> &[TypeCheckError] {
        &self.errors
    }
}

/// A pending type-check block to be generated.
#[derive(Debug, Clone)]
pub struct PendingTypeCheckBlock {
    /// Component reference.
    pub component: String,
    /// Template source.
    pub template: String,
    /// Bound target.
    pub bound_target: String,
}

/// Template type-check error.
#[derive(Debug, Clone)]
pub struct TypeCheckError {
    /// Error message.
    pub message: String,
    /// Error code.
    pub code: String,
    /// File path.
    pub file: Option<String>,
    /// Start position.
    pub start: Option<usize>,
    /// Error length.
    pub length: Option<usize>,
}
