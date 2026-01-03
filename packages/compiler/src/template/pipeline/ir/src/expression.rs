//! IR Expressions
//!
//! Corresponds to packages/compiler/src/template/pipeline/ir/src/expression.ts

use crate::output::output_ast::Expression;
use crate::parse_util::ParseSourceSpan;
use crate::render3::r3_ast::Variable;
use crate::template::pipeline::ir::enums::ExpressionKind;
use crate::template::pipeline::ir::handle::{SlotHandle, XrefId};
use crate::template::pipeline::ir::operations::{CreateOp, UpdateOp};
use crate::template::pipeline::ir::ops::shared::VariableOp;
use crate::template::pipeline::ir::traits::{
    ConsumesVarsTrait, DependsOnSlotContextOpTrait, UsesVarOffsetTrait,
};
use bitflags::bitflags;
use std::sync::Arc;

/// An `Expression` subtype representing a logical expression in the intermediate representation.
#[derive(Debug, Clone)]
pub enum IRExpression {
    LexicalRead(LexicalReadExpr),
    Reference(ReferenceExpr),
    Context(ContextExpr),
    NextContext(NextContextExpr),
    GetCurrentView(GetCurrentViewExpr),
    RestoreView(RestoreViewExpr),
    ResetView(ResetViewExpr),
    ReadVariable(ReadVariableExpr),
    PureFunction(PureFunctionExpr),
    PureFunctionParameter(PureFunctionParameterExpr),
    PipeBinding(PipeBindingExpr),
    PipeBindingVariadic(PipeBindingVariadicExpr),
    SafePropertyRead(SafePropertyReadExpr),
    SafeKeyedRead(SafeKeyedReadExpr),
    SafeInvokeFunction(SafeInvokeFunctionExpr),
    SafeTernary(SafeTernaryExpr),
    Empty(EmptyExpr),
    AssignTemporary(AssignTemporaryExpr),
    ReadTemporary(ReadTemporaryExpr),
    SlotLiteral(SlotLiteralExpr),
    ConditionalCase(ConditionalCaseExpr),
    ConstCollected(ConstCollectedExpr),
    TwoWayBindingSet(TwoWayBindingSetExpr),
    ContextLetReference(ContextLetReferenceExpr),
    StoreLet(StoreLetExpr),
    TrackContext(TrackContextExpr),
}

bitflags! {
    /// Flags for visitor context when transforming expressions
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct VisitorContextFlag: u32 {
        const NONE = 0b0000;
        const IN_CHILD_OPERATION = 0b0001;
    }
}

/// Transformer type which converts expressions into general `Expression`s (which may be an
/// identity transformation).
pub type ExpressionTransform = Box<dyn Fn(Expression, VisitorContextFlag) -> Expression>;

/// Check whether a given `Expression` is a logical IR expression type.
pub fn is_ir_expression(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::LexicalRead(_)
            | Expression::Reference(_)
            | Expression::Context(_)
            | Expression::NextContext(_)
            | Expression::GetCurrentView(_)
            | Expression::RestoreView(_)
            | Expression::ResetView(_)
            | Expression::ReadVariable(_)
            | Expression::PureFunction(_)
            | Expression::PureFunctionParameter(_)
            | Expression::PipeBinding(_)
            | Expression::PipeBindingVariadic(_)
            | Expression::SafePropertyRead(_)
            | Expression::SafeKeyedRead(_)
            | Expression::SafeInvokeFunction(_)
            | Expression::SafeTernary(_)
            | Expression::Empty(_)
            | Expression::AssignTemporary(_)
            | Expression::ReadTemporary(_)
            | Expression::SlotLiteral(_)
            | Expression::ConditionalCase(_)
            | Expression::ConstCollected(_)
            | Expression::TwoWayBindingSet(_)
            | Expression::ContextLetReference(_)
            | Expression::StoreLet(_)
            | Expression::TrackContext(_)
    )
}

/// Convert an `Expression` to an `IRExpression` if it is one.
pub fn as_ir_expression(expr: &Expression) -> Option<IRExpression> {
    match expr {
        Expression::LexicalRead(e) => Some(IRExpression::LexicalRead(e.clone())),
        Expression::Reference(e) => Some(IRExpression::Reference(e.clone())),
        Expression::Context(e) => Some(IRExpression::Context(e.clone())),
        Expression::NextContext(e) => Some(IRExpression::NextContext(e.clone())),
        Expression::GetCurrentView(e) => Some(IRExpression::GetCurrentView(e.clone())),
        Expression::RestoreView(e) => Some(IRExpression::RestoreView(e.clone())),
        Expression::ResetView(e) => Some(IRExpression::ResetView(e.clone())),
        Expression::ReadVariable(e) => Some(IRExpression::ReadVariable(e.clone())),
        Expression::PureFunction(e) => Some(IRExpression::PureFunction(e.clone())),
        Expression::PureFunctionParameter(e) => {
            Some(IRExpression::PureFunctionParameter(e.clone()))
        }
        Expression::PipeBinding(e) => Some(IRExpression::PipeBinding(e.clone())),
        Expression::PipeBindingVariadic(e) => Some(IRExpression::PipeBindingVariadic(e.clone())),
        Expression::SafePropertyRead(e) => Some(IRExpression::SafePropertyRead(e.clone())),
        Expression::SafeKeyedRead(e) => Some(IRExpression::SafeKeyedRead(e.clone())),
        Expression::SafeInvokeFunction(e) => Some(IRExpression::SafeInvokeFunction(e.clone())),
        Expression::SafeTernary(e) => Some(IRExpression::SafeTernary(e.clone())),
        Expression::Empty(e) => Some(IRExpression::Empty(e.clone())),
        Expression::AssignTemporary(e) => Some(IRExpression::AssignTemporary(e.clone())),
        Expression::ReadTemporary(e) => Some(IRExpression::ReadTemporary(e.clone())),
        Expression::SlotLiteral(e) => Some(IRExpression::SlotLiteral(e.clone())),
        Expression::ConditionalCase(e) => Some(IRExpression::ConditionalCase(e.clone())),
        Expression::ConstCollected(e) => Some(IRExpression::ConstCollected(e.clone())),
        Expression::TwoWayBindingSet(e) => Some(IRExpression::TwoWayBindingSet(e.clone())),
        Expression::ContextLetReference(e) => Some(IRExpression::ContextLetReference(e.clone())),
        Expression::StoreLet(e) => Some(IRExpression::StoreLet(e.clone())),
        Expression::TrackContext(e) => Some(IRExpression::TrackContext(e.clone())),
        _ => None,
    }
}

/// Base trait for all logical IR expressions.
pub trait IRExpressionTrait {
    fn kind(&self) -> ExpressionKind;
    fn source_span(&self) -> Option<&ParseSourceSpan>;

    /// Run the transformer against any nested expressions which may be present in this IR expression
    /// subtype.
    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    );
}

/// Logical expression representing a lexical read of a variable name.
#[derive(Debug, Clone)]
pub struct LexicalReadExpr {
    pub name: Arc<str>,
    pub source_span: Option<ParseSourceSpan>,
}

impl LexicalReadExpr {
    pub fn new(name: String) -> Self {
        LexicalReadExpr {
            name: name.into(),
            source_span: None,
        }
    }
}

impl IRExpressionTrait for LexicalReadExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::LexicalRead
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        _transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        _flags: VisitorContextFlag,
    ) {
        // No nested expressions
    }
}

/// Runtime operation to retrieve the value of a local reference.
#[derive(Debug, Clone)]
pub struct ReferenceExpr {
    pub target: XrefId,
    pub target_slot: SlotHandle,
    pub offset: usize,
    pub source_span: Option<ParseSourceSpan>,
}

impl ReferenceExpr {
    pub fn new(target: XrefId, target_slot: SlotHandle, offset: usize) -> Self {
        ReferenceExpr {
            target,
            target_slot,
            offset,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for ReferenceExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::Reference
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        _transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        _flags: VisitorContextFlag,
    ) {
        // No nested expressions
    }
}

/// A reference to the current view context (usually the `ctx` variable in a template function).
#[derive(Debug, Clone)]
pub struct ContextExpr {
    pub view: XrefId,
    pub source_span: Option<ParseSourceSpan>,
}

impl ContextExpr {
    pub fn new(view: XrefId) -> Self {
        ContextExpr {
            view,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for ContextExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::Context
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        _transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        _flags: VisitorContextFlag,
    ) {
        // No nested expressions
    }
}

/// A reference to the current view context inside a track function.
#[derive(Debug, Clone)]
pub struct TrackContextExpr {
    pub view: XrefId,
    pub source_span: Option<ParseSourceSpan>,
}

impl TrackContextExpr {
    pub fn new(view: XrefId) -> Self {
        TrackContextExpr {
            view,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for TrackContextExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::TrackContext
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        _transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        _flags: VisitorContextFlag,
    ) {
        // No nested expressions
    }
}

/// Runtime operation to navigate to the next view context in the view hierarchy.
#[derive(Debug, Clone)]
pub struct NextContextExpr {
    pub steps: usize,
    pub source_span: Option<ParseSourceSpan>,
}

impl NextContextExpr {
    pub fn new() -> Self {
        NextContextExpr {
            steps: 1,
            source_span: None,
        }
    }
}

impl Default for NextContextExpr {
    fn default() -> Self {
        Self::new()
    }
}

impl IRExpressionTrait for NextContextExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::NextContext
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        _transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        _flags: VisitorContextFlag,
    ) {
        // No nested expressions
    }
}

/// Runtime operation to snapshot the current view context.
/// The result of this operation can be stored in a variable and later used with the `RestoreView`
/// operation.
#[derive(Debug, Clone)]
pub struct GetCurrentViewExpr {
    pub source_span: Option<ParseSourceSpan>,
}

impl GetCurrentViewExpr {
    pub fn new() -> Self {
        GetCurrentViewExpr { source_span: None }
    }
}

impl Default for GetCurrentViewExpr {
    fn default() -> Self {
        Self::new()
    }
}

impl IRExpressionTrait for GetCurrentViewExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::GetCurrentView
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        _transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        _flags: VisitorContextFlag,
    ) {
        // No nested expressions
    }
}

/// Runtime operation to restore a snapshotted view.
#[derive(Debug, Clone)]
pub struct RestoreViewExpr {
    pub view: EitherXrefIdOrExpression,
    pub target_context: Option<XrefId>,
    pub source_span: Option<ParseSourceSpan>,
}

/// Either an XrefId or an Expression (for dynamic view references)
#[derive(Debug, Clone)]
pub enum EitherXrefIdOrExpression {
    XrefId(XrefId),
    Expression(Box<Expression>),
}

impl RestoreViewExpr {
    pub fn new(view: EitherXrefIdOrExpression) -> Self {
        RestoreViewExpr {
            view,
            target_context: None,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for RestoreViewExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::RestoreView
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        if let EitherXrefIdOrExpression::Expression(ref mut expr) = self.view {
            *expr = Box::new(transform(*expr.clone(), flags));
        }
    }
}

/// Runtime operation to reset the current view context after `RestoreView`.
#[derive(Debug, Clone)]
pub struct ResetViewExpr {
    pub expr: Box<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

impl ResetViewExpr {
    pub fn new(expr: Box<Expression>) -> Self {
        ResetViewExpr {
            expr,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for ResetViewExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::ResetView
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        self.expr = Box::new(transform(*self.expr.clone(), flags));
    }
}

/// Read of a variable declared as an `ir.VariableOp` and referenced through its `ir.XrefId`.
#[derive(Debug, Clone)]
pub struct ReadVariableExpr {
    pub xref: XrefId,
    pub name: Option<String>,
    pub source_span: Option<ParseSourceSpan>,
}

impl ReadVariableExpr {
    pub fn new(xref: XrefId) -> Self {
        ReadVariableExpr {
            xref,
            name: None,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for ReadVariableExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::ReadVariable
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        _transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        _flags: VisitorContextFlag,
    ) {
        // No nested expressions
    }
}

/// Defines and calls a function with change-detected arguments.
#[derive(Debug, Clone)]
pub struct PureFunctionExpr {
    pub var_offset: Option<usize>,
    /// The expression which should be memoized as a pure computation.
    /// This expression contains internal `PureFunctionParameterExpr`s, which are placeholders for the
    /// positional argument expressions in `args`.
    pub body: Option<Box<Expression>>,
    /// Positional arguments to the pure function which will memoize the `body` expression, which act
    /// as memoization keys.
    pub args: Vec<Expression>,
    /// Once extracted to the `ConstantPool`, a reference to the function which defines the computation
    /// of `body`.
    pub fn_: Option<Box<Expression>>,
    pub source_span: Option<ParseSourceSpan>,
}

impl PureFunctionExpr {
    pub fn new(expression: Option<Box<Expression>>, args: Vec<Expression>) -> Self {
        PureFunctionExpr {
            var_offset: None,
            body: expression,
            args,
            fn_: None,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for PureFunctionExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::PureFunctionExpr
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        if let Some(ref mut body) = self.body {
            *body = Box::new(transform(
                *body.clone(),
                flags | VisitorContextFlag::IN_CHILD_OPERATION,
            ));
        } else if let Some(ref mut fn_) = self.fn_ {
            *fn_ = Box::new(transform(*fn_.clone(), flags));
        }

        for arg in &mut self.args {
            *arg = transform(arg.clone(), flags);
        }
    }
}

impl ConsumesVarsTrait for PureFunctionExpr {}
impl UsesVarOffsetTrait for PureFunctionExpr {
    fn var_offset(&self) -> Option<usize> {
        self.var_offset
    }

    fn set_var_offset(&mut self, offset: Option<usize>) {
        self.var_offset = offset;
    }
}

/// Indicates a positional parameter to a pure function definition.
#[derive(Debug, Clone)]
pub struct PureFunctionParameterExpr {
    pub index: usize,
    pub source_span: Option<ParseSourceSpan>,
}

impl PureFunctionParameterExpr {
    pub fn new(index: usize) -> Self {
        PureFunctionParameterExpr {
            index,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for PureFunctionParameterExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::PureFunctionParameterExpr
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        _transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        _flags: VisitorContextFlag,
    ) {
        // No nested expressions
    }
}

/// Binding to a pipe transformation.
#[derive(Debug, Clone)]
pub struct PipeBindingExpr {
    pub var_offset: Option<usize>,
    pub target: XrefId,
    pub target_slot: SlotHandle,
    pub name: Arc<str>,
    pub args: Vec<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

impl PipeBindingExpr {
    pub fn new(
        target: XrefId,
        target_slot: SlotHandle,
        name: String,
        args: Vec<Expression>,
    ) -> Self {
        PipeBindingExpr {
            var_offset: None,
            target,
            target_slot,
            name: name.into(),
            args,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for PipeBindingExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::PipeBinding
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        for arg in &mut self.args {
            *arg = transform(arg.clone(), flags);
        }
    }
}

impl ConsumesVarsTrait for PipeBindingExpr {}
impl UsesVarOffsetTrait for PipeBindingExpr {
    fn var_offset(&self) -> Option<usize> {
        self.var_offset
    }

    fn set_var_offset(&mut self, offset: Option<usize>) {
        self.var_offset = offset;
    }
}

/// Binding to a pipe transformation with a variable number of arguments.
#[derive(Debug, Clone)]
pub struct PipeBindingVariadicExpr {
    pub var_offset: Option<usize>,
    pub target: XrefId,
    pub target_slot: SlotHandle,
    pub name: Arc<str>,
    pub args: Box<Expression>,
    pub num_args: usize,
    pub source_span: Option<ParseSourceSpan>,
}

impl PipeBindingVariadicExpr {
    pub fn new(
        target: XrefId,
        target_slot: SlotHandle,
        name: Arc<str>,
        args: Box<Expression>,
        num_args: usize,
    ) -> Self {
        PipeBindingVariadicExpr {
            var_offset: None,
            target,
            target_slot,
            name,
            args,
            num_args,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for PipeBindingVariadicExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::PipeBindingVariadic
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        self.args = Box::new(transform(*self.args.clone(), flags));
    }
}

impl ConsumesVarsTrait for PipeBindingVariadicExpr {}
impl UsesVarOffsetTrait for PipeBindingVariadicExpr {
    fn var_offset(&self) -> Option<usize> {
        self.var_offset
    }

    fn set_var_offset(&mut self, offset: Option<usize>) {
        self.var_offset = offset;
    }
}

/// A safe property read requiring expansion into a null check.
#[derive(Debug, Clone)]
pub struct SafePropertyReadExpr {
    pub receiver: Box<Expression>,
    pub name: Arc<str>,
    pub source_span: Option<ParseSourceSpan>,
}

impl SafePropertyReadExpr {
    pub fn new(receiver: Box<Expression>, name: Arc<str>) -> Self {
        SafePropertyReadExpr {
            receiver,
            name,
            source_span: None,
        }
    }

    /// An alias for name, which allows other logic to handle property reads and keyed reads together.
    pub fn index(&self) -> &str {
        &self.name
    }
}

impl IRExpressionTrait for SafePropertyReadExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::SafePropertyRead
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        self.receiver = Box::new(transform(*self.receiver.clone(), flags));
    }
}

/// A safe keyed read requiring expansion into a null check.
#[derive(Debug, Clone)]
pub struct SafeKeyedReadExpr {
    pub receiver: Box<Expression>,
    pub index: Box<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

impl SafeKeyedReadExpr {
    pub fn new(
        receiver: Box<Expression>,
        index: Box<Expression>,
        source_span: Option<ParseSourceSpan>,
    ) -> Self {
        SafeKeyedReadExpr {
            receiver,
            index,
            source_span,
        }
    }
}

impl IRExpressionTrait for SafeKeyedReadExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::SafeKeyedRead
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        self.receiver = Box::new(transform(*self.receiver.clone(), flags));
        self.index = Box::new(transform(*self.index.clone(), flags));
    }
}

/// A safe function call requiring expansion into a null check.
#[derive(Debug, Clone)]
pub struct SafeInvokeFunctionExpr {
    pub receiver: Box<Expression>,
    pub args: Vec<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

impl SafeInvokeFunctionExpr {
    pub fn new(receiver: Box<Expression>, args: Vec<Expression>) -> Self {
        SafeInvokeFunctionExpr {
            receiver,
            args,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for SafeInvokeFunctionExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::SafeInvokeFunction
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        self.receiver = Box::new(transform(*self.receiver.clone(), flags));
        for arg in &mut self.args {
            *arg = transform(arg.clone(), flags);
        }
    }
}

/// An intermediate expression that will be expanded from a safe read into an explicit ternary.
#[derive(Debug, Clone)]
pub struct SafeTernaryExpr {
    pub guard: Box<Expression>,
    pub expr: Box<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

impl SafeTernaryExpr {
    pub fn new(guard: Box<Expression>, expr: Box<Expression>) -> Self {
        SafeTernaryExpr {
            guard,
            expr,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for SafeTernaryExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::SafeTernaryExpr
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        self.guard = Box::new(transform(*self.guard.clone(), flags));
        self.expr = Box::new(transform(*self.expr.clone(), flags));
    }
}

/// An empty expression that will be stripped before generating the final output.
#[derive(Debug, Clone)]
pub struct EmptyExpr {
    pub source_span: Option<ParseSourceSpan>,
}

impl EmptyExpr {
    pub fn new() -> Self {
        EmptyExpr { source_span: None }
    }
}

impl Default for EmptyExpr {
    fn default() -> Self {
        Self::new()
    }
}

impl IRExpressionTrait for EmptyExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::EmptyExpr
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        _transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        _flags: VisitorContextFlag,
    ) {
        // No nested expressions
    }
}

/// An assignment to a temporary variable.
#[derive(Debug, Clone)]
pub struct AssignTemporaryExpr {
    pub name: Option<String>,
    pub expr: Box<Expression>,
    pub xref: XrefId,
    pub source_span: Option<ParseSourceSpan>,
}

impl AssignTemporaryExpr {
    pub fn new(expr: Box<Expression>, xref: XrefId) -> Self {
        AssignTemporaryExpr {
            name: None,
            expr,
            xref,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for AssignTemporaryExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::AssignTemporaryExpr
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        self.expr = Box::new(transform(*self.expr.clone(), flags));
    }
}

/// A reference to a temporary variable.
#[derive(Debug, Clone)]
pub struct ReadTemporaryExpr {
    pub name: Option<String>,
    pub xref: XrefId,
    pub source_span: Option<ParseSourceSpan>,
}

impl ReadTemporaryExpr {
    pub fn new(xref: XrefId) -> Self {
        ReadTemporaryExpr {
            name: None,
            xref,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for ReadTemporaryExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::ReadTemporaryExpr
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        _transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        _flags: VisitorContextFlag,
    ) {
        // No nested expressions
    }
}

/// An expression that will cause a literal slot index to be emitted.
#[derive(Debug, Clone)]
pub struct SlotLiteralExpr {
    pub slot: SlotHandle,
    pub source_span: Option<ParseSourceSpan>,
}

impl SlotLiteralExpr {
    pub fn new(slot: SlotHandle) -> Self {
        SlotLiteralExpr {
            slot,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for SlotLiteralExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::SlotLiteralExpr
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        _transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        _flags: VisitorContextFlag,
    ) {
        // No nested expressions
    }
}

/// Conditional case expression - represents one branch of a conditional
/// Create an expression for one branch of a conditional.
#[derive(Debug, Clone)]
pub struct ConditionalCaseExpr {
    /// The expression to be tested for this case. Might be null, as in an `else` case.
    pub expr: Option<Box<Expression>>,
    /// The Xref of the view to be displayed if this condition is true.
    pub target: XrefId,
    /// The slot handle of the target view
    pub target_slot: SlotHandle,
    /// Expression alias if present (for @if with expression aliases)
    pub alias: Option<Variable>,
    pub source_span: Option<ParseSourceSpan>,
}

impl ConditionalCaseExpr {
    pub fn new(
        expr: Option<Box<Expression>>,
        target: XrefId,
        target_slot: SlotHandle,
        alias: Option<Variable>,
    ) -> Self {
        ConditionalCaseExpr {
            expr,
            target,
            target_slot,
            alias,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for ConditionalCaseExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::ConditionalCase
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        if let Some(ref mut expr) = self.expr {
            *expr = Box::new(transform(*expr.clone(), flags));
        }
    }
}

/// An expression that will be automatically extracted to the component const array.
#[derive(Debug, Clone)]
pub struct ConstCollectedExpr {
    pub expr: Box<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

impl ConstCollectedExpr {
    pub fn new(expr: Box<Expression>) -> Self {
        ConstCollectedExpr {
            expr,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for ConstCollectedExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::ConstCollected
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        self.expr = Box::new(transform(*self.expr.clone(), flags));
    }
}

/// Operation that sets the value of a two-way binding.
#[derive(Debug, Clone)]
pub struct TwoWayBindingSetExpr {
    pub target: Box<Expression>,
    pub value: Box<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

impl TwoWayBindingSetExpr {
    pub fn new(target: Box<Expression>, value: Box<Expression>) -> Self {
        TwoWayBindingSetExpr {
            target,
            value,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for TwoWayBindingSetExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::TwoWayBindingSet
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        self.target = Box::new(transform(*self.target.clone(), flags));
        self.value = Box::new(transform(*self.value.clone(), flags));
    }
}

/// A reference to a `@let` declaration read from the context view.
#[derive(Debug, Clone)]
pub struct ContextLetReferenceExpr {
    pub target: XrefId,
    pub target_slot: SlotHandle,
    pub source_span: Option<ParseSourceSpan>,
}

impl ContextLetReferenceExpr {
    pub fn new(target: XrefId, target_slot: SlotHandle) -> Self {
        ContextLetReferenceExpr {
            target,
            target_slot,
            source_span: None,
        }
    }
}

impl IRExpressionTrait for ContextLetReferenceExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::ContextLetReference
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }

    fn transform_internal_expressions(
        &mut self,
        _transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        _flags: VisitorContextFlag,
    ) {
        // No nested expressions
    }
}

/// A call storing the value of a `@let` declaration.
#[derive(Debug, Clone)]
pub struct StoreLetExpr {
    pub target: XrefId,
    pub value: Box<Expression>,
    pub source_span: ParseSourceSpan,
}

impl StoreLetExpr {
    pub fn new(target: XrefId, value: Box<Expression>, source_span: ParseSourceSpan) -> Self {
        StoreLetExpr {
            target,
            value,
            source_span,
        }
    }
}

impl IRExpressionTrait for StoreLetExpr {
    fn kind(&self) -> ExpressionKind {
        ExpressionKind::StoreLet
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }

    fn transform_internal_expressions(
        &mut self,
        transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
        flags: VisitorContextFlag,
    ) {
        self.value = Box::new(transform(*self.value.clone(), flags));
    }
}

impl ConsumesVarsTrait for StoreLetExpr {}
impl DependsOnSlotContextOpTrait for StoreLetExpr {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Transform all expressions in an Interpolation structure
fn transform_expressions_in_interpolation(
    interpolation: &mut crate::template::pipeline::ir::ops::update::Interpolation,
    transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
    flags: VisitorContextFlag,
) {
    for expr in &mut interpolation.expressions {
        *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
    }
}

/// Transform all `Expression`s in the AST of `expr` with the `transform` function.
/// All such operations will be replaced with the result of applying `transform`, which may be an
/// identity transformation.
pub fn transform_expressions_in_expression(
    mut expr: Expression,
    transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
    flags: VisitorContextFlag,
) -> Expression {
    use crate::output::output_ast::Expression as OutputExpr;

    // Transform nested expressions first
    // println!("DEBUG: transform visiting {:?}", expr);
    match &mut expr {
        OutputExpr::ReadProp(prop) => {
            // println!("DEBUG: traversing ReadProp {:?}", prop.name);
            prop.receiver = Box::new(transform_expressions_in_expression(
                *prop.receiver.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::InvokeFn(invoke) => {
            // InvokeFn - traverse fn_ and args
            invoke.fn_ = Box::new(transform_expressions_in_expression(
                *invoke.fn_.clone(),
                transform,
                flags,
            ));
            for arg in &mut invoke.args {
                *arg = transform_expressions_in_expression(arg.clone(), transform, flags);
            }
        }
        OutputExpr::BinaryOp(bin) => {
            bin.lhs = Box::new(transform_expressions_in_expression(
                *bin.lhs.clone(),
                transform,
                flags,
            ));
            bin.rhs = Box::new(transform_expressions_in_expression(
                *bin.rhs.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::Unary(un) => {
            un.expr = Box::new(transform_expressions_in_expression(
                *un.expr.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::ReadProp(prop) => {
            prop.receiver = Box::new(transform_expressions_in_expression(
                *prop.receiver.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::ReadKey(key) => {
            key.receiver = Box::new(transform_expressions_in_expression(
                *key.receiver.clone(),
                transform,
                flags,
            ));
            key.index = Box::new(transform_expressions_in_expression(
                *key.index.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::InvokeFn(invoke) => {
            invoke.fn_ = Box::new(transform_expressions_in_expression(
                *invoke.fn_.clone(),
                transform,
                flags,
            ));
            for arg in &mut invoke.args {
                *arg = transform_expressions_in_expression(arg.clone(), transform, flags);
            }
        }
        OutputExpr::LiteralArray(arr) => {
            for entry in &mut arr.entries {
                *entry = transform_expressions_in_expression(entry.clone(), transform, flags);
            }
        }
        OutputExpr::LiteralMap(map) => {
            for entry in &mut map.entries {
                entry.value = Box::new(transform_expressions_in_expression(
                    *entry.value.clone(),
                    transform,
                    flags,
                ));
            }
        }
        OutputExpr::Conditional(cond) => {
            cond.condition = Box::new(transform_expressions_in_expression(
                *cond.condition.clone(),
                transform,
                flags,
            ));
            cond.true_case = Box::new(transform_expressions_in_expression(
                *cond.true_case.clone(),
                transform,
                flags,
            ));
            if let Some(ref mut false_case) = cond.false_case {
                *false_case = Box::new(transform_expressions_in_expression(
                    *false_case.clone(),
                    transform,
                    flags,
                ));
            }
        }
        OutputExpr::TypeOf(ty) => {
            ty.expr = Box::new(transform_expressions_in_expression(
                *ty.expr.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::Void(void) => {
            void.expr = Box::new(transform_expressions_in_expression(
                *void.expr.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::Parens(parens) => {
            parens.expr = Box::new(transform_expressions_in_expression(
                *parens.expr.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::NotExpr(not) => {
            not.condition = Box::new(transform_expressions_in_expression(
                *not.condition.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::TaggedTemplate(tagged) => {
            tagged.tag = Box::new(transform_expressions_in_expression(
                *tagged.tag.clone(),
                transform,
                flags,
            ));
            for expr in &mut tagged.template.expressions {
                *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
            }
        }
        OutputExpr::ArrowFn(arrow) => match &mut arrow.body {
            crate::output::output_ast::ArrowFunctionBody::Expression(expr) => {
                *expr = Box::new(transform_expressions_in_expression(
                    *expr.clone(),
                    transform,
                    flags,
                ));
            }
            crate::output::output_ast::ArrowFunctionBody::Statements(stmts) => {
                for stmt in stmts {
                    transform_expressions_in_statement(stmt, transform, flags);
                }
            }
        },
        OutputExpr::TemplateLiteral(tmpl) => {
            for expr in &mut tmpl.expressions {
                *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
            }
        }
        OutputExpr::Localized(localized) => {
            for expr in &mut localized.expressions {
                *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
            }
        }
        // IR Expression variants with nested expressions
        OutputExpr::SafePropertyRead(ir_expr) => {
            ir_expr.receiver = Box::new(transform_expressions_in_expression(
                *ir_expr.receiver.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::SafeKeyedRead(ir_expr) => {
            ir_expr.receiver = Box::new(transform_expressions_in_expression(
                *ir_expr.receiver.clone(),
                transform,
                flags,
            ));
            ir_expr.index = Box::new(transform_expressions_in_expression(
                *ir_expr.index.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::SafeInvokeFunction(ir_expr) => {
            ir_expr.receiver = Box::new(transform_expressions_in_expression(
                *ir_expr.receiver.clone(),
                transform,
                flags,
            ));
            for arg in &mut ir_expr.args {
                *arg = transform_expressions_in_expression(arg.clone(), transform, flags);
            }
        }
        OutputExpr::SafeTernary(ir_expr) => {
            ir_expr.guard = Box::new(transform_expressions_in_expression(
                *ir_expr.guard.clone(),
                transform,
                flags,
            ));
            ir_expr.expr = Box::new(transform_expressions_in_expression(
                *ir_expr.expr.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::PipeBinding(ir_expr) => {
            for arg in &mut ir_expr.args {
                *arg = transform_expressions_in_expression(arg.clone(), transform, flags);
            }
        }
        OutputExpr::PipeBindingVariadic(ir_expr) => {
            ir_expr.args = Box::new(transform_expressions_in_expression(
                *ir_expr.args.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::AssignTemporary(ir_expr) => {
            ir_expr.expr = Box::new(transform_expressions_in_expression(
                *ir_expr.expr.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::StoreLet(ir_expr) => {
            ir_expr.value = Box::new(transform_expressions_in_expression(
                *ir_expr.value.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::PureFunction(pf) => {
            if let Some(ref mut fn_) = pf.fn_ {
                *fn_ = Box::new(transform_expressions_in_expression(
                    *fn_.clone(),
                    transform,
                    flags,
                ));
            }
            for arg in &mut pf.args {
                *arg = transform_expressions_in_expression(arg.clone(), transform, flags);
            }
            if let Some(ref mut body) = pf.body {
                *body = Box::new(transform_expressions_in_expression(
                    *body.clone(),
                    transform,
                    flags,
                ));
            }
        }
        OutputExpr::ConditionalCase(ir_expr) => {
            if let Some(ref mut expr) = ir_expr.expr {
                *expr = Box::new(transform_expressions_in_expression(
                    *expr.clone(),
                    transform,
                    flags,
                ));
            }
        }
        // Output AST variants that need traversal
        OutputExpr::WriteVar(var) => {
            var.value = Box::new(transform_expressions_in_expression(
                *var.value.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::WriteKey(key) => {
            key.receiver = Box::new(transform_expressions_in_expression(
                *key.receiver.clone(),
                transform,
                flags,
            ));
            key.index = Box::new(transform_expressions_in_expression(
                *key.index.clone(),
                transform,
                flags,
            ));
            key.value = Box::new(transform_expressions_in_expression(
                *key.value.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::WriteProp(prop) => {
            prop.receiver = Box::new(transform_expressions_in_expression(
                *prop.receiver.clone(),
                transform,
                flags,
            ));
            prop.value = Box::new(transform_expressions_in_expression(
                *prop.value.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::Instantiate(inst) => {
            inst.class_expr = Box::new(transform_expressions_in_expression(
                *inst.class_expr.clone(),
                transform,
                flags,
            ));
            for arg in &mut inst.args {
                *arg = transform_expressions_in_expression(arg.clone(), transform, flags);
            }
        }
        OutputExpr::CommaExpr(comma) => {
            for part in &mut comma.parts {
                *part = transform_expressions_in_expression(part.clone(), transform, flags);
            }
        }
        // Leaf nodes or handled above
        OutputExpr::InvokeFn(expr) => {
            expr.fn_ = Box::new(transform_expressions_in_expression(
                *expr.fn_.clone(),
                transform,
                flags,
            ));
            for arg in &mut expr.args {
                *arg = transform_expressions_in_expression(arg.clone(), transform, flags);
            }
        }
        OutputExpr::BinaryOp(expr) => {
            expr.lhs = Box::new(transform_expressions_in_expression(
                *expr.lhs.clone(),
                transform,
                flags,
            ));
            expr.rhs = Box::new(transform_expressions_in_expression(
                *expr.rhs.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::ReadProp(expr) => {
            expr.receiver = Box::new(transform_expressions_in_expression(
                *expr.receiver.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::ReadKey(expr) => {
            expr.receiver = Box::new(transform_expressions_in_expression(
                *expr.receiver.clone(),
                transform,
                flags,
            ));
            expr.index = Box::new(transform_expressions_in_expression(
                *expr.index.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::Parens(expr) => {
            expr.expr = Box::new(transform_expressions_in_expression(
                *expr.expr.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::NotExpr(expr) => {
            expr.condition = Box::new(transform_expressions_in_expression(
                *expr.condition.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::Conditional(expr) => {
            expr.condition = Box::new(transform_expressions_in_expression(
                *expr.condition.clone(),
                transform,
                flags,
            ));
            expr.true_case = Box::new(transform_expressions_in_expression(
                *expr.true_case.clone(),
                transform,
                flags,
            ));
            if let Some(ref mut false_case) = expr.false_case {
                *false_case = Box::new(transform_expressions_in_expression(
                    *false_case.clone(),
                    transform,
                    flags,
                ));
            }
        }
        OutputExpr::IfNull(expr) => {
            expr.condition = Box::new(transform_expressions_in_expression(
                *expr.condition.clone(),
                transform,
                flags,
            ));
            expr.null_case = Box::new(transform_expressions_in_expression(
                *expr.null_case.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::AssertNotNull(expr) => {
            expr.condition = Box::new(transform_expressions_in_expression(
                *expr.condition.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::Cast(expr) => {
            expr.value = Box::new(transform_expressions_in_expression(
                *expr.value.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::TypeOf(expr) => {
            expr.expr = Box::new(transform_expressions_in_expression(
                *expr.expr.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::Void(expr) => {
            expr.expr = Box::new(transform_expressions_in_expression(
                *expr.expr.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::Unary(expr) => {
            expr.expr = Box::new(transform_expressions_in_expression(
                *expr.expr.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::ConstCollected(expr) => {
            expr.expr = Box::new(transform_expressions_in_expression(
                *expr.expr.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::TwoWayBindingSet(expr) => {
            expr.target = Box::new(transform_expressions_in_expression(
                *expr.target.clone(),
                transform,
                flags,
            ));
            expr.value = Box::new(transform_expressions_in_expression(
                *expr.value.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::ResetView(expr) => {
            // ResetView wraps an expression - need to traverse the nested expr to visit LexicalReads
            expr.expr = Box::new(transform_expressions_in_expression(
                *expr.expr.clone(),
                transform,
                flags,
            ));
        }
        OutputExpr::RestoreView(expr) => {
            // RestoreView may have nested view expression
            if let EitherXrefIdOrExpression::Expression(ref mut view_expr) = expr.view {
                *view_expr = Box::new(transform_expressions_in_expression(
                    *view_expr.clone(),
                    transform,
                    flags,
                ));
            }
        }

        OutputExpr::ArrowFn(expr) => {
            // Handle arrow function body if it is an expression
            if let crate::output::output_ast::ArrowFunctionBody::Expression(ref mut body_expr) =
                expr.body
            {
                *body_expr = Box::new(transform_expressions_in_expression(
                    *body_expr.clone(),
                    transform,
                    flags,
                ));
            }
        }

        OutputExpr::LiteralArray(expr) => {
            for entry in &mut expr.entries {
                *entry = transform_expressions_in_expression(entry.clone(), transform, flags);
            }
        }
        OutputExpr::LiteralMap(expr) => {
            for entry in &mut expr.entries {
                entry.value = Box::new(transform_expressions_in_expression(
                    *entry.value.clone(),
                    transform,
                    flags,
                ));
            }
        }

        // Leaf nodes or unhandled
        OutputExpr::LexicalRead(l) => {
            // LexicalRead - leaf node
        }
        OutputExpr::WrappedNode(_)
        | OutputExpr::Literal(_)
        | OutputExpr::External(_)
        | OutputExpr::ExternalRef(_)
        | OutputExpr::FnParam(_)
        | OutputExpr::DynamicImport(_)
        | OutputExpr::Fn(_)
        | OutputExpr::RegularExpressionLiteral(_)
        | OutputExpr::RawCode(_)
        | _ => {}
    }

    // Apply the transform function to the expression itself
    transform(expr, flags)
}

/// Transform all `Expression`s in the AST of `stmt` with the `transform` function.
/// All such operations will be replaced with the result of applying `transform`, which may be an
/// identity transformation.
pub fn transform_expressions_in_statement(
    stmt: &mut crate::output::output_ast::Statement,
    transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
    flags: VisitorContextFlag,
) {
    use crate::output::output_ast::Statement;

    match stmt {
        Statement::Expression(expr_stmt) => {
            expr_stmt.expr = Box::new(transform_expressions_in_expression(
                *expr_stmt.expr.clone(),
                transform,
                flags,
            ));
        }
        Statement::Return(return_stmt) => {
            return_stmt.value = Box::new(transform_expressions_in_expression(
                *return_stmt.value.clone(),
                transform,
                flags,
            ));
        }
        Statement::DeclareVar(declare_var) => {
            if let Some(ref mut value) = declare_var.value {
                *value = Box::new(transform_expressions_in_expression(
                    *value.clone(),
                    transform,
                    flags,
                ));
            }
        }
        Statement::IfStmt(if_stmt) => {
            if_stmt.condition = Box::new(transform_expressions_in_expression(
                *if_stmt.condition.clone(),
                transform,
                flags,
            ));
            for case_stmt in &mut if_stmt.true_case {
                transform_expressions_in_statement(case_stmt, transform, flags);
            }
            for case_stmt in &mut if_stmt.false_case {
                transform_expressions_in_statement(case_stmt, transform, flags);
            }
        }
        Statement::DeclareFn(declare_fn) => {
            for stmt in &mut declare_fn.statements {
                transform_expressions_in_statement(stmt, transform, flags);
            }
        }
    }
}

/// Visits all `Expression`s in the AST of `op` with the `visitor` function.
pub fn visit_expressions_in_op(
    op: &mut (dyn crate::template::pipeline::ir::operations::Op),
    visitor: &mut dyn FnMut(&Expression, VisitorContextFlag),
) {
    // This is a placeholder - full implementation would need to:
    // 1. Match on op.kind() to determine the concrete type
    // 2. Downcast to the appropriate struct
    // 3. Visit expressions in that struct
    //
    // For now, we'll use transform_expressions_in_op with an identity transform
    transform_expressions_in_op(
        op,
        &mut |expr, flags| {
            visitor(&expr, flags);
            expr
        },
        VisitorContextFlag::NONE,
    );
}

/// Transform all `Expression`s in the AST of `op` with the `transform` function.
/// All such operations will be replaced with the result of applying `transform`, which may be an
/// identity transformation.
///
/// Note: This function operates on trait objects, which makes it complex. In practice, you may want
/// to call this through concrete op types rather than through the Op trait.
pub fn transform_expressions_in_op(
    op: &mut dyn crate::template::pipeline::ir::operations::Op,
    transform: &mut dyn FnMut(Expression, VisitorContextFlag) -> Expression,
    flags: VisitorContextFlag,
) {
    use crate::template::pipeline::ir::enums::OpKind;

    // This is a simplified implementation that works with trait objects
    // In practice, you'd want to match on concrete op types and transform their fields
    // For now, this is a placeholder that can be extended

    match op.kind() {
        OpKind::Binding
        | OpKind::AnimationString
        | OpKind::AnimationBinding => {
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::update::BindingOp>()
            {
                match &mut op.expression {
                     crate::template::pipeline::ir::ops::update::BindingExpression::Expression(expr) => {
                         *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                     }
                     crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(interp) => {
                         for expr in &mut interp.expressions {
                             *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                         }
                     }
                 }
            }
        }
        OpKind::ClassProp => {
            // ClassPropOp has Expression (not BindingExpression)
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::update::ClassPropOp>()
            {
                op.expression =
                    transform_expressions_in_expression(op.expression.clone(), transform, flags);
            }
        }
        OpKind::StyleProp => {
            // StylePropOp has BindingExpression
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::update::StylePropOp>()
            {
                match &mut op.expression {
                    crate::template::pipeline::ir::ops::update::BindingExpression::Expression(expr) => {
                        *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                    }
                    crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(interp) => {
                        for expr in &mut interp.expressions {
                            *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                        }
                    }
                }
            }
        }
        OpKind::Property | OpKind::DomProperty | OpKind::Attribute | OpKind::Control => {
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::update::PropertyOp>()
            {
                match &mut op.expression {
                     crate::template::pipeline::ir::ops::update::BindingExpression::Expression(expr) => {
                         *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                     }
                     crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(interp) => {
                         for expr in &mut interp.expressions {
                             *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                         }
                     }
                 }
                if let Some(ref mut sanitizer) = op.sanitizer {
                    *sanitizer =
                        transform_expressions_in_expression(sanitizer.clone(), transform, flags);
                }
            } else if let Some(op) =
                op.as_any_mut()
                    .downcast_mut::<crate::template::pipeline::ir::ops::update::AttributeOp>()
            {
                match &mut op.expression {
                     crate::template::pipeline::ir::ops::update::BindingExpression::Expression(expr) => {
                         *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                     }
                     crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(interp) => {
                         for expr in &mut interp.expressions {
                             *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                         }
                     }
                 }
                if let Some(ref mut sanitizer) = op.sanitizer {
                    *sanitizer =
                        transform_expressions_in_expression(sanitizer.clone(), transform, flags);
                }
            }
        }
        OpKind::TwoWayProperty => {
            if let Some(op) =
                op.as_any_mut()
                    .downcast_mut::<crate::template::pipeline::ir::ops::update::TwoWayPropertyOp>()
            {
                op.expression =
                    transform_expressions_in_expression(op.expression.clone(), transform, flags);
                if let Some(ref mut sanitizer) = op.sanitizer {
                    *sanitizer =
                        transform_expressions_in_expression(sanitizer.clone(), transform, flags);
                }
            }
        }
        OpKind::I18nExpression => {
            if let Some(op) =
                op.as_any_mut()
                    .downcast_mut::<crate::template::pipeline::ir::ops::update::I18nExpressionOp>()
            {
                op.expression =
                    transform_expressions_in_expression(op.expression.clone(), transform, flags);
            }
        }
        OpKind::StyleProp => {
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::update::StylePropOp>()
            {
                match &mut op.expression {
                     crate::template::pipeline::ir::ops::update::BindingExpression::Expression(expr) => {
                         *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                     }
                     crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(interp) => {
                         for expr in &mut interp.expressions {
                             *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                         }
                     }
                 }
            }
        }
        OpKind::ClassProp => {
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::update::ClassPropOp>()
            {
                op.expression =
                    transform_expressions_in_expression(op.expression.clone(), transform, flags);
            }
        }
        OpKind::StyleMap => {
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::update::StyleMapOp>()
            {
                match &mut op.expression {
                     crate::template::pipeline::ir::ops::update::BindingExpression::Expression(expr) => {
                         *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                     }
                     crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(interp) => {
                         for expr in &mut interp.expressions {
                             *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                         }
                     }
                 }
            }
        }
        OpKind::ClassMap => {
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::update::ClassMapOp>()
            {
                match &mut op.expression {
                     crate::template::pipeline::ir::ops::update::BindingExpression::Expression(expr) => {
                         *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                     }
                     crate::template::pipeline::ir::ops::update::BindingExpression::Interpolation(interp) => {
                         for expr in &mut interp.expressions {
                             *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                         }
                     }
                 }
            }
        }
        OpKind::InterpolateText => {
            if let Some(op) =
                op.as_any_mut()
                    .downcast_mut::<crate::template::pipeline::ir::ops::update::InterpolateTextOp>()
            {
                for expr in &mut op.interpolation.expressions {
                    *expr = transform_expressions_in_expression(expr.clone(), transform, flags);
                }
            }
        }
        OpKind::Statement => {
            // StatementOp - wraps a statement
            use crate::template::pipeline::ir::ops::StatementOp;
            use crate::template::pipeline::ir::CreateOp;
            use crate::template::pipeline::ir::UpdateOp;

            // Try CreateOp version
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<StatementOp<Box<dyn CreateOp + Send + Sync>>>()
            {
                // CreateOp StatementOp - transform statement expressions
                transform_expressions_in_statement(&mut op.statement, transform, flags);
            }
            // Try UpdateOp version
            else if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<StatementOp<Box<dyn UpdateOp + Send + Sync>>>()
            {
                // UpdateOp StatementOp - transform statement expressions
                transform_expressions_in_statement(&mut op.statement, transform, flags);
            }
        }

        OpKind::Conditional => {
            if let Some(op) =
                op.as_any_mut()
                    .downcast_mut::<crate::template::pipeline::ir::ops::update::ConditionalOp>()
            {
                if let Some(ref mut test) = op.test {
                    *test = transform_expressions_in_expression(test.clone(), transform, flags);
                }
                for condition in &mut op.conditions {
                    if let Some(ref mut expr) = condition.expr {
                        *expr = Box::new(transform_expressions_in_expression(
                            *expr.clone(),
                            transform,
                            flags,
                        ));
                    }
                }
                if let Some(ref mut processed) = op.processed {
                    *processed =
                        transform_expressions_in_expression(processed.clone(), transform, flags);
                }
                if let Some(ref mut context_value) = op.context_value {
                    *context_value = transform_expressions_in_expression(
                        context_value.clone(),
                        transform,
                        flags,
                    );
                }
            }
        }
        OpKind::RepeaterCreate => {
            if let Some(op) =
                op.as_any_mut()
                    .downcast_mut::<crate::template::pipeline::ir::ops::create::RepeaterCreateOp>()
            {
                *op.track =
                    transform_expressions_in_expression(*op.track.clone(), transform, flags);
                if let Some(ref mut track_by_fn) = op.track_by_fn {
                    *track_by_fn = Box::new(transform_expressions_in_expression(
                        *track_by_fn.clone(),
                        transform,
                        flags,
                    ));
                }
            }
        }
        OpKind::Listener => {
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::create::ListenerOp>()
            {
                for handler_op in &mut op.handler_ops {
                    transform_expressions_in_op(&mut **handler_op, transform, flags);
                }
            }
        }
        OpKind::AnimationListener => {
            use crate::template::pipeline::ir::ops::create::AnimationListenerOp;
            if let Some(op) = op.as_any_mut().downcast_mut::<AnimationListenerOp>() {
                for handler_op in &mut op.handler_ops {
                    transform_expressions_in_op(&mut **handler_op, transform, flags);
                }
            }
        }
        OpKind::TwoWayListener => {
            use crate::template::pipeline::ir::ops::create::TwoWayListenerOp;
            if let Some(op) = op.as_any_mut().downcast_mut::<TwoWayListenerOp>() {
                for handler_op in &mut op.handler_ops {
                    transform_expressions_in_op(&mut **handler_op, transform, flags);
                }
            }
        }
        OpKind::Variable => {
            // Try CreateOp version
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<VariableOp<Box<dyn CreateOp + Send + Sync>>>()
            {
                op.initializer = Box::new(transform_expressions_in_expression(
                    *op.initializer.clone(),
                    transform,
                    flags,
                ));
            }
            // Try UpdateOp version
            else if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<VariableOp<Box<dyn UpdateOp + Send + Sync>>>()
            {
                op.initializer = Box::new(transform_expressions_in_expression(
                    *op.initializer.clone(),
                    transform,
                    flags,
                ));
            } else {
            }
        }

        OpKind::Repeater => {
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::update::RepeaterOp>()
            {
                op.collection =
                    transform_expressions_in_expression(op.collection.clone(), transform, flags);
            }
        }
        OpKind::Defer => {
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::create::DeferOp>()
            {
                if let Some(ref mut loading_config) = op.loading_config {
                    *loading_config = transform_expressions_in_expression(
                        loading_config.clone(),
                        transform,
                        flags,
                    );
                }
                if let Some(ref mut placeholder_config) = op.placeholder_config {
                    *placeholder_config = transform_expressions_in_expression(
                        placeholder_config.clone(),
                        transform,
                        flags,
                    );
                }
                if let Some(ref mut resolver_fn) = op.resolver_fn {
                    *resolver_fn =
                        transform_expressions_in_expression(resolver_fn.clone(), transform, flags);
                }
                if let Some(ref mut own_resolver_fn) = op.own_resolver_fn {
                    *own_resolver_fn = transform_expressions_in_expression(
                        own_resolver_fn.clone(),
                        transform,
                        flags,
                    );
                }
            }
        }
        OpKind::DeferWhen => {
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::update::DeferWhenOp>()
            {
                op.expr = transform_expressions_in_expression(op.expr.clone(), transform, flags);
            }
        }
        OpKind::StoreLet => {
            if let Some(op) = op
                .as_any_mut()
                .downcast_mut::<crate::template::pipeline::ir::ops::update::StoreLetOp>()
            {
                op.value = transform_expressions_in_expression(op.value.clone(), transform, flags);
            }
        }
        OpKind::I18nMessage => {
            // This op has params map
            // TODO: Downcast and transform params
        }
        // These operations contain no expressions:
        OpKind::Advance
        | OpKind::Container
        | OpKind::ContainerEnd
        | OpKind::ContainerStart
        | OpKind::DeferOn
        | OpKind::DisableBindings
        | OpKind::Element
        | OpKind::ElementEnd
        | OpKind::ElementStart
        | OpKind::EnableBindings
        | OpKind::I18n
        | OpKind::I18nApply
        | OpKind::I18nContext
        | OpKind::I18nEnd
        | OpKind::I18nStart
        | OpKind::IcuEnd
        | OpKind::IcuStart
        | OpKind::Namespace
        | OpKind::Pipe
        | OpKind::Projection
        | OpKind::ProjectionDef
        | OpKind::Template
        | OpKind::Text
        | OpKind::I18nAttributes
        | OpKind::IcuPlaceholder
        | OpKind::DeclareLet
        | OpKind::SourceLocation
        | OpKind::ConditionalCreate
        | OpKind::ConditionalBranchCreate
        | OpKind::ControlCreate
        | OpKind::Animation
        | OpKind::ListEnd
        | OpKind::ExtractedAttribute => {
            // These operations contain no expressions or are handled separately
            // Note: Some of these (Binding, StyleProp, etc.) actually have expressions
            // but are handled in the first match arm above
        }
    }
}

/// Checks whether the given expression is a string literal.
pub fn is_string_literal(expr: &Expression) -> bool {
    use crate::output::output_ast::{Expression as OutputExpr, LiteralValue};

    if let OutputExpr::Literal(lit) = expr {
        matches!(lit.value, LiteralValue::String(_))
    } else {
        false
    }
}
