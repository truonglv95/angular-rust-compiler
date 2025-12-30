//! Update Operations
//!
//! Corresponds to packages/compiler/src/template/pipeline/ir/src/ops/update.ts

use crate::output::output_ast::Expression;
use crate::parse_util::ParseSourceSpan;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::handle::{SlotHandle, XrefId};
use crate::template::pipeline::ir::operations::{Op, UpdateOp};
use crate::template::pipeline::ir::traits::{ConsumesVarsTrait, DependsOnSlotContextOpTrait};
use std::sync::Arc;

/// Interpolation structure for text interpolation
#[derive(Debug, Clone)]
pub struct Interpolation {
    /// Static strings in the interpolation
    pub strings: Vec<Arc<str>>,
    /// Dynamic expressions in the interpolation
    pub expressions: Vec<Expression>,
    /// i18n placeholders for the expressions
    pub i18n_placeholders: Vec<Arc<str>>,
}

impl Interpolation {
    pub fn new(
        strings: Vec<Arc<str>>,
        expressions: Vec<Expression>,
        i18n_placeholders: Vec<Arc<str>>,
    ) -> Self {
        // Validate that strings and expressions are compatible
        // strings.len() should equal expressions.len() + 1
        if strings.len() != expressions.len() + 1 {
            panic!(
                "Interpolation strings count ({}) must equal expressions count ({}) + 1",
                strings.len(),
                expressions.len()
            );
        }

        // Validate i18n placeholders if provided
        if !i18n_placeholders.is_empty() && i18n_placeholders.len() != expressions.len() {
            panic!(
                "Expected {} i18n placeholders to match interpolation expression count, but got {}",
                expressions.len(),
                i18n_placeholders.len()
            );
        }

        Interpolation {
            strings,
            expressions,
            i18n_placeholders,
        }
    }
}

/// A logical operation to perform string interpolation on a text node.
#[derive(Debug, Clone)]
pub struct InterpolateTextOp {
    /// Reference to the text node to which the interpolation is bound
    pub target: XrefId,
    /// The interpolated value
    pub interpolation: Interpolation,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl InterpolateTextOp {
    pub fn new(target: XrefId, interpolation: Interpolation, source_span: ParseSourceSpan) -> Self {
        InterpolateTextOp {
            target,
            interpolation,
            source_span,
        }
    }
}

impl Op for InterpolateTextOp {
    fn kind(&self) -> OpKind {
        OpKind::InterpolateText
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for InterpolateTextOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for InterpolateTextOp {}
impl DependsOnSlotContextOpTrait for InterpolateTextOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

// Safe to implement Send + Sync since Expression types don't actually need to be thread-safe
// in the template pipeline context (they're processed in a single thread)
unsafe impl Send for InterpolateTextOp {}
unsafe impl Sync for InterpolateTextOp {}

/// Create an InterpolateTextOp
pub fn create_interpolate_text_op(
    target: XrefId,
    interpolation: Interpolation,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(InterpolateTextOp::new(target, interpolation, source_span))
}

/// Op to store the current value of a `@let` declaration
#[derive(Debug, Clone)]
pub struct StoreLetOp {
    /// Name that the user set when declaring the `@let`
    pub declared_name: Arc<str>,
    /// XrefId of the slot in which the call may write its value
    pub target: XrefId,
    /// Value of the `@let` declaration
    pub value: Expression,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl StoreLetOp {
    pub fn new(
        target: XrefId,
        declared_name: Arc<str>,
        value: Expression,
        source_span: ParseSourceSpan,
    ) -> Self {
        StoreLetOp {
            target,
            declared_name,
            value,
            source_span,
        }
    }
}

impl Op for StoreLetOp {
    fn kind(&self) -> OpKind {
        OpKind::StoreLet
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for StoreLetOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for StoreLetOp {}
impl DependsOnSlotContextOpTrait for StoreLetOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

// Safe to implement Send + Sync since Expression types don't actually need to be thread-safe
// in the template pipeline context (they're processed in a single thread)
unsafe impl Send for StoreLetOp {}
unsafe impl Sync for StoreLetOp {}

/// Create a StoreLetOp
pub fn create_store_let_op(
    target: XrefId,
    declared_name: Arc<str>,
    value: Expression,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(StoreLetOp::new(target, declared_name, value, source_span))
}

use crate::template::pipeline::ir::expression::ConditionalCaseExpr;

/// Conditional operation - displays an embedded view according to a condition
#[derive(Debug, Clone)]
pub struct ConditionalOp {
    /// The insertion point, which is the first template in the creation block belonging to this condition
    pub target: XrefId,
    /// The main test expression (for a switch), or `null` (for an if, which has no test expression)
    pub test: Option<Expression>,
    /// Each possible embedded view that could be displayed has a condition (or is default)
    pub conditions: Vec<ConditionalCaseExpr>,
    /// After processing, this will be a single collapsed expression that evaluates the conditions
    pub processed: Option<Expression>,
    /// Control flow conditionals can accept a context value (this is a result of specifying an alias)
    pub context_value: Option<Expression>,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl ConditionalOp {
    pub fn new(
        target: XrefId,
        test: Option<Expression>,
        conditions: Vec<ConditionalCaseExpr>,
        source_span: ParseSourceSpan,
    ) -> Self {
        ConditionalOp {
            target,
            test,
            conditions,
            processed: None,
            context_value: None,
            source_span,
        }
    }
}

impl Op for ConditionalOp {
    fn kind(&self) -> OpKind {
        OpKind::Conditional
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for ConditionalOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for ConditionalOp {}
impl DependsOnSlotContextOpTrait for ConditionalOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

// Safe to implement Send + Sync since Expression types don't actually need to be thread-safe
// in the template pipeline context (they're processed in a single thread)
unsafe impl Send for ConditionalOp {}
unsafe impl Sync for ConditionalOp {}

/// Create a ConditionalOp
pub fn create_conditional_op(
    target: XrefId,
    test: Option<Expression>,
    conditions: Vec<ConditionalCaseExpr>,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(ConditionalOp::new(target, test, conditions, source_span))
}

/// Repeater operation - displays repeated embedded views
#[derive(Debug, Clone)]
pub struct RepeaterOp {
    /// The RepeaterCreate op associated with this repeater
    pub target: XrefId,
    /// Target slot handle
    pub target_slot: SlotHandle,
    /// The collection provided to the for loop as its expression
    pub collection: Expression,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl RepeaterOp {
    pub fn new(
        target: XrefId,
        target_slot: SlotHandle,
        collection: Expression,
        source_span: ParseSourceSpan,
    ) -> Self {
        RepeaterOp {
            target,
            target_slot,
            collection,
            source_span,
        }
    }
}

impl Op for RepeaterOp {
    fn kind(&self) -> OpKind {
        OpKind::Repeater
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for RepeaterOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl DependsOnSlotContextOpTrait for RepeaterOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

// Safe to implement Send + Sync since Expression types don't actually need to be thread-safe
unsafe impl Send for RepeaterOp {}
unsafe impl Sync for RepeaterOp {}

/// Create a RepeaterOp
pub fn create_repeater_op(
    target: XrefId,
    target_slot: SlotHandle,
    collection: Expression,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(RepeaterOp::new(
        target,
        target_slot,
        collection,
        source_span,
    ))
}

use crate::core::SecurityContext;
use crate::i18n::i18n_ast::Message;
use crate::template::pipeline::ir::enums::{
    AnimationBindingKind, AnimationKind, BindingKind, DeferOpModifierKind, I18nExpressionFor,
    I18nParamResolutionTime, TemplateKind,
};

/// Union type for binding expressions - can be Expression or Interpolation
#[derive(Debug, Clone)]
pub enum BindingExpression {
    Expression(Expression),
    Interpolation(Interpolation),
}

/// An intermediate binding op, that has not yet been processed into an individual property,
/// attribute, style, etc.
#[derive(Debug, Clone)]
pub struct BindingOp {
    /// Reference to the element on which the property is bound
    pub target: XrefId,
    /// The kind of binding represented by this op
    pub binding_kind: BindingKind,
    /// The name of this binding
    pub name: Arc<str>,
    /// Expression which is bound to the property
    pub expression: BindingExpression,
    /// The unit of the bound value
    pub unit: Option<String>,
    /// The security context of the binding
    pub security_context: Vec<SecurityContext>,
    /// Whether the binding is a TextAttribute (e.g. `some-attr="some-value"`)
    pub is_text_attribute: bool,
    /// Whether this is a structural template attribute
    pub is_structural_template_attribute: bool,
    /// Whether this binding is on a structural template
    pub template_kind: Option<TemplateKind>,
    /// i18n context XrefId
    pub i18n_context: Option<XrefId>,
    /// i18n message
    pub i18n_message: Option<Message>,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl BindingOp {
    pub fn new(
        target: XrefId,
        binding_kind: BindingKind,
        name: Arc<str>,
        expression: BindingExpression,
        unit: Option<String>,
        security_context: Vec<SecurityContext>,
        is_text_attribute: bool,
        is_structural_template_attribute: bool,
        template_kind: Option<TemplateKind>,
        i18n_context: Option<XrefId>,
        i18n_message: Option<Message>,
        source_span: ParseSourceSpan,
    ) -> Self {
        BindingOp {
            target,
            binding_kind,
            name,
            expression,
            unit,
            security_context,
            is_text_attribute,
            is_structural_template_attribute,
            template_kind,
            i18n_context,
            i18n_message,
            source_span,
        }
    }
}

impl Op for BindingOp {
    fn kind(&self) -> OpKind {
        OpKind::Binding
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for BindingOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for BindingOp {}
impl DependsOnSlotContextOpTrait for BindingOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

// Safe to implement Send + Sync since Expression types don't actually need to be thread-safe
unsafe impl Send for BindingOp {}
unsafe impl Sync for BindingOp {}

/// Create a BindingOp
pub fn create_binding_op(
    target: XrefId,
    binding_kind: BindingKind,
    name: Arc<str>,
    expression: BindingExpression,
    unit: Option<String>,
    security_context: Vec<SecurityContext>,
    is_text_attribute: bool,
    is_structural_template_attribute: bool,
    template_kind: Option<TemplateKind>,
    i18n_message: Option<Message>,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(BindingOp::new(
        target,
        binding_kind,
        name,
        expression,
        unit,
        security_context,
        is_text_attribute,
        is_structural_template_attribute,
        template_kind,
        None, // i18n_context - set later
        i18n_message,
        source_span,
    ))
}

/// A logical operation representing binding to a property in the update IR.
#[derive(Debug, Clone)]
pub struct PropertyOp {
    /// Reference to the element on which the property is bound.
    pub target: XrefId,
    /// Name of the bound property.
    pub name: Arc<str>,
    /// Expression which is bound to the property.
    pub expression: BindingExpression,
    /// Whether this property is an animation trigger.
    pub binding_kind: BindingKind,
    /// The security context of the binding.
    pub security_context: Vec<SecurityContext>,
    /// The sanitizer for this property.
    pub sanitizer: Option<Expression>,
    /// Whether this is a structural template attribute.
    pub is_structural_template_attribute: bool,
    /// The kind of template targeted by the binding, or None if this binding does not target a template.
    pub template_kind: Option<TemplateKind>,
    /// i18n context XrefId.
    pub i18n_context: Option<XrefId>,
    /// i18n message.
    pub i18n_message: Option<Message>,
    /// Source span.
    pub source_span: ParseSourceSpan,
}

impl PropertyOp {
    pub fn new(
        target: XrefId,
        name: Arc<str>,
        expression: BindingExpression,
        binding_kind: BindingKind,
        security_context: Vec<SecurityContext>,
        is_structural_template_attribute: bool,
        template_kind: Option<TemplateKind>,
        i18n_context: Option<XrefId>,
        i18n_message: Option<Message>,
        source_span: ParseSourceSpan,
    ) -> Self {
        PropertyOp {
            target,
            name,
            expression,
            binding_kind,
            security_context,
            sanitizer: None,
            is_structural_template_attribute,
            template_kind,
            i18n_context,
            i18n_message,
            source_span,
        }
    }
}

impl Op for PropertyOp {
    fn kind(&self) -> OpKind {
        OpKind::Property
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for PropertyOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for PropertyOp {}
impl DependsOnSlotContextOpTrait for PropertyOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

unsafe impl Send for PropertyOp {}
unsafe impl Sync for PropertyOp {}

/// Create a PropertyOp.
pub fn create_property_op(
    target: XrefId,
    name: Arc<str>,
    expression: BindingExpression,
    binding_kind: BindingKind,
    security_context: Vec<SecurityContext>,
    is_structural_template_attribute: bool,
    template_kind: Option<TemplateKind>,
    i18n_context: Option<XrefId>,
    i18n_message: Option<Message>,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(PropertyOp::new(
        target,
        name,
        expression,
        binding_kind,
        security_context,
        is_structural_template_attribute,
        template_kind,
        i18n_context,
        i18n_message,
        source_span,
    ))
}

/// A logical operation representing the property binding side of a two-way binding in the update IR.
#[derive(Debug, Clone)]
pub struct TwoWayPropertyOp {
    /// Reference to the element on which the property is bound.
    pub target: XrefId,
    /// Name of the property.
    pub name: Arc<str>,
    /// Expression which is bound to the property.
    pub expression: Expression,
    /// The security context of the binding.
    pub security_context: Vec<SecurityContext>,
    /// The sanitizer for this property.
    pub sanitizer: Option<Expression>,
    /// Whether this is a structural template attribute.
    pub is_structural_template_attribute: bool,
    /// The kind of template targeted by the binding, or None if this binding does not target a template.
    pub template_kind: Option<TemplateKind>,
    /// i18n context XrefId.
    pub i18n_context: Option<XrefId>,
    /// i18n message.
    pub i18n_message: Option<Message>,
    /// Source span.
    pub source_span: ParseSourceSpan,
}

impl TwoWayPropertyOp {
    pub fn new(
        target: XrefId,
        name: Arc<str>,
        expression: Expression,
        security_context: Vec<SecurityContext>,
        is_structural_template_attribute: bool,
        template_kind: Option<TemplateKind>,
        i18n_context: Option<XrefId>,
        i18n_message: Option<Message>,
        source_span: ParseSourceSpan,
    ) -> Self {
        TwoWayPropertyOp {
            target,
            name,
            expression,
            security_context,
            sanitizer: None,
            is_structural_template_attribute,
            template_kind,
            i18n_context,
            i18n_message,
            source_span,
        }
    }
}

impl Op for TwoWayPropertyOp {
    fn kind(&self) -> OpKind {
        OpKind::TwoWayProperty
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for TwoWayPropertyOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for TwoWayPropertyOp {}
impl DependsOnSlotContextOpTrait for TwoWayPropertyOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

unsafe impl Send for TwoWayPropertyOp {}
unsafe impl Sync for TwoWayPropertyOp {}

/// Create a TwoWayPropertyOp.
pub fn create_two_way_property_op(
    target: XrefId,
    name: Arc<str>,
    expression: Expression,
    security_context: Vec<SecurityContext>,
    is_structural_template_attribute: bool,
    template_kind: Option<TemplateKind>,
    i18n_context: Option<XrefId>,
    i18n_message: Option<Message>,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(TwoWayPropertyOp::new(
        target,
        name,
        expression,
        security_context,
        is_structural_template_attribute,
        template_kind,
        i18n_context,
        i18n_message,
        source_span,
    ))
}

/// A logical operation representing setting an attribute on an element in the update IR.
#[derive(Debug, Clone)]
pub struct AttributeOp {
    /// The `XrefId` of the template-like element the attribute will belong to.
    pub target: XrefId,
    /// The namespace of the attribute (or None if none).
    pub namespace: Option<String>,
    /// The name of the attribute.
    pub name: Arc<str>,
    /// The value of the attribute.
    pub expression: BindingExpression,
    /// The security context of the binding.
    pub security_context: Vec<SecurityContext>,
    /// The sanitizer for this attribute.
    pub sanitizer: Option<Expression>,
    /// Whether the binding is a TextAttribute (e.g. `some-attr="some-value"`).
    pub is_text_attribute: bool,
    /// Whether this is a structural template attribute.
    pub is_structural_template_attribute: bool,
    /// The kind of template targeted by the binding, or None if this binding does not target a template.
    pub template_kind: Option<TemplateKind>,
    /// The i18n context, if this is an i18n attribute.
    pub i18n_context: Option<XrefId>,
    /// i18n message.
    pub i18n_message: Option<Message>,
    /// Source span.
    pub source_span: ParseSourceSpan,
}

impl AttributeOp {
    pub fn new(
        target: XrefId,
        namespace: Option<String>,
        name: Arc<str>,
        expression: BindingExpression,
        security_context: Vec<SecurityContext>,
        is_text_attribute: bool,
        is_structural_template_attribute: bool,
        template_kind: Option<TemplateKind>,
        i18n_message: Option<Message>,
        source_span: ParseSourceSpan,
    ) -> Self {
        AttributeOp {
            target,
            namespace,
            name,
            expression,
            security_context,
            sanitizer: None,
            is_text_attribute,
            is_structural_template_attribute,
            template_kind,
            i18n_context: None,
            i18n_message,
            source_span,
        }
    }
}

impl Op for AttributeOp {
    fn kind(&self) -> OpKind {
        OpKind::Attribute
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for AttributeOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for AttributeOp {}
impl DependsOnSlotContextOpTrait for AttributeOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

unsafe impl Send for AttributeOp {}
unsafe impl Sync for AttributeOp {}

/// Create an AttributeOp.
pub fn create_attribute_op(
    target: XrefId,
    namespace: Option<String>,
    name: Arc<str>,
    expression: BindingExpression,
    security_context: Vec<SecurityContext>,
    is_text_attribute: bool,
    is_structural_template_attribute: bool,
    template_kind: Option<TemplateKind>,
    i18n_message: Option<Message>,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(AttributeOp::new(
        target,
        namespace,
        name,
        expression,
        security_context,
        is_text_attribute,
        is_structural_template_attribute,
        template_kind,
        i18n_message,
        source_span,
    ))
}

/// A logical operation representing binding to a style property in the update IR.
#[derive(Debug, Clone)]
pub struct StylePropOp {
    /// Reference to the element on which the property is bound.
    pub target: XrefId,
    /// Name of the bound property.
    pub name: Arc<str>,
    /// Expression which is bound to the property.
    pub expression: BindingExpression,
    /// The unit of the bound value.
    pub unit: Option<String>,
    /// Source span.
    pub source_span: ParseSourceSpan,
}

impl StylePropOp {
    pub fn new(
        target: XrefId,
        name: Arc<str>,
        expression: BindingExpression,
        unit: Option<String>,
        source_span: ParseSourceSpan,
    ) -> Self {
        StylePropOp {
            target,
            name,
            expression,
            unit,
            source_span,
        }
    }
}

impl Op for StylePropOp {
    fn kind(&self) -> OpKind {
        OpKind::StyleProp
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for StylePropOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for StylePropOp {}
impl DependsOnSlotContextOpTrait for StylePropOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

unsafe impl Send for StylePropOp {}
unsafe impl Sync for StylePropOp {}

/// Create a StylePropOp.
pub fn create_style_prop_op(
    target: XrefId,
    name: Arc<str>,
    expression: BindingExpression,
    unit: Option<String>,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(StylePropOp::new(
        target,
        name,
        expression,
        unit,
        source_span,
    ))
}

/// A logical operation representing binding to a class property in the update IR.
#[derive(Debug, Clone)]
pub struct ClassPropOp {
    /// Reference to the element on which the property is bound.
    pub target: XrefId,
    /// Name of the bound property.
    pub name: Arc<str>,
    /// Expression which is bound to the property.
    pub expression: Expression,
    /// Source span.
    pub source_span: ParseSourceSpan,
}

impl ClassPropOp {
    pub fn new(
        target: XrefId,
        name: Arc<str>,
        expression: Expression,
        source_span: ParseSourceSpan,
    ) -> Self {
        ClassPropOp {
            target,
            name,
            expression,
            source_span,
        }
    }
}

impl Op for ClassPropOp {
    fn kind(&self) -> OpKind {
        OpKind::ClassProp
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for ClassPropOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for ClassPropOp {}
impl DependsOnSlotContextOpTrait for ClassPropOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

unsafe impl Send for ClassPropOp {}
unsafe impl Sync for ClassPropOp {}

/// Create a ClassPropOp.
pub fn create_class_prop_op(
    target: XrefId,
    name: Arc<str>,
    expression: Expression,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(ClassPropOp::new(target, name, expression, source_span))
}

/// A logical operation representing binding to a style map in the update IR.
#[derive(Debug, Clone)]
pub struct StyleMapOp {
    /// Reference to the element on which the property is bound.
    pub target: XrefId,
    /// Expression which is bound to the property.
    pub expression: BindingExpression,
    /// Source span.
    pub source_span: ParseSourceSpan,
}

impl StyleMapOp {
    pub fn new(
        target: XrefId,
        expression: BindingExpression,
        source_span: ParseSourceSpan,
    ) -> Self {
        StyleMapOp {
            target,
            expression,
            source_span,
        }
    }
}

impl Op for StyleMapOp {
    fn kind(&self) -> OpKind {
        OpKind::StyleMap
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for StyleMapOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for StyleMapOp {}
impl DependsOnSlotContextOpTrait for StyleMapOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

unsafe impl Send for StyleMapOp {}
unsafe impl Sync for StyleMapOp {}

/// Create a StyleMapOp.
pub fn create_style_map_op(
    target: XrefId,
    expression: BindingExpression,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(StyleMapOp::new(target, expression, source_span))
}

/// A logical operation representing binding to a class map in the update IR.
#[derive(Debug, Clone)]
pub struct ClassMapOp {
    /// Reference to the element on which the property is bound.
    pub target: XrefId,
    /// Expression which is bound to the property.
    pub expression: BindingExpression,
    /// Source span.
    pub source_span: ParseSourceSpan,
}

impl ClassMapOp {
    pub fn new(
        target: XrefId,
        expression: BindingExpression,
        source_span: ParseSourceSpan,
    ) -> Self {
        ClassMapOp {
            target,
            expression,
            source_span,
        }
    }
}

impl Op for ClassMapOp {
    fn kind(&self) -> OpKind {
        OpKind::ClassMap
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for ClassMapOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for ClassMapOp {}
impl DependsOnSlotContextOpTrait for ClassMapOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

unsafe impl Send for ClassMapOp {}
unsafe impl Sync for ClassMapOp {}

/// Create a ClassMapOp.
pub fn create_class_map_op(
    target: XrefId,
    expression: BindingExpression,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(ClassMapOp::new(target, expression, source_span))
}

/// Logical operation to advance the runtime's internal slot pointer in the update IR.
#[derive(Debug, Clone)]
pub struct AdvanceOp {
    /// Delta by which to advance the pointer.
    pub delta: usize,
    /// Source span of the binding that caused the advance.
    pub source_span: ParseSourceSpan,
}

impl AdvanceOp {
    pub fn new(delta: usize, source_span: ParseSourceSpan) -> Self {
        AdvanceOp { delta, source_span }
    }
}

impl Op for AdvanceOp {
    fn kind(&self) -> OpKind {
        OpKind::Advance
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for AdvanceOp {
    fn xref(&self) -> XrefId {
        // AdvanceOp doesn't have an xref - this shouldn't be called
        panic!("AdvanceOp does not have an xref")
    }
}

unsafe impl Send for AdvanceOp {}
unsafe impl Sync for AdvanceOp {}

/// Create an AdvanceOp.
pub fn create_advance_op(
    delta: usize,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(AdvanceOp::new(delta, source_span))
}

/// A logical operation representing binding to an animation in the update IR.
#[derive(Debug, Clone)]
pub struct AnimationBindingOp {
    /// The name of the extracted attribute.
    pub name: Arc<str>,
    /// Reference to the element on which the property is bound.
    pub target: XrefId,
    /// Animation kind.
    pub animation_kind: AnimationKind,
    /// Expression which is bound to the property.
    pub expression: BindingExpression,
    /// i18n message XrefId.
    pub i18n_message: Option<XrefId>,
    /// The security context of the binding.
    pub security_context: Vec<SecurityContext>,
    /// The sanitizer for this property.
    pub sanitizer: Option<Expression>,
    /// Source span.
    pub source_span: ParseSourceSpan,
    /// Animation binding kind.
    pub animation_binding_kind: AnimationBindingKind,
}

impl AnimationBindingOp {
    pub fn new(
        name: Arc<str>,
        target: XrefId,
        animation_kind: AnimationKind,
        expression: BindingExpression,
        security_context: Vec<SecurityContext>,
        source_span: ParseSourceSpan,
        animation_binding_kind: AnimationBindingKind,
    ) -> Self {
        AnimationBindingOp {
            name,
            target,
            animation_kind,
            expression,
            i18n_message: None,
            security_context,
            sanitizer: None,
            source_span,
            animation_binding_kind,
        }
    }
}

impl Op for AnimationBindingOp {
    fn kind(&self) -> OpKind {
        OpKind::AnimationBinding
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for AnimationBindingOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

unsafe impl Send for AnimationBindingOp {}
unsafe impl Sync for AnimationBindingOp {}

/// Create an AnimationBindingOp.
pub fn create_animation_binding_op(
    name: Arc<str>,
    target: XrefId,
    animation_kind: AnimationKind,
    expression: BindingExpression,
    security_context: Vec<SecurityContext>,
    source_span: ParseSourceSpan,
    animation_binding_kind: AnimationBindingKind,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(AnimationBindingOp::new(
        name,
        target,
        animation_kind,
        expression,
        security_context,
        source_span,
        animation_binding_kind,
    ))
}

/// Operation that controls when a `@defer` loads, using a custom expression as the condition.
#[derive(Debug, Clone)]
pub struct DeferWhenOp {
    /// The `defer` create op associated with this when condition.
    pub target: XrefId,
    /// A user-provided expression that triggers the defer op.
    pub expr: Expression,
    /// Modifier set on the trigger by the user (e.g. `hydrate`, `prefetch` etc).
    pub modifier: DeferOpModifierKind,
    /// Source span.
    pub source_span: ParseSourceSpan,
}

impl DeferWhenOp {
    pub fn new(
        target: XrefId,
        expr: Expression,
        modifier: DeferOpModifierKind,
        source_span: ParseSourceSpan,
    ) -> Self {
        DeferWhenOp {
            target,
            expr,
            modifier,
            source_span,
        }
    }
}

impl Op for DeferWhenOp {
    fn kind(&self) -> OpKind {
        OpKind::DeferWhen
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for DeferWhenOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for DeferWhenOp {}
impl DependsOnSlotContextOpTrait for DeferWhenOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

unsafe impl Send for DeferWhenOp {}
unsafe impl Sync for DeferWhenOp {}

/// Create a DeferWhenOp.
pub fn create_defer_when_op(
    target: XrefId,
    expr: Expression,
    modifier: DeferOpModifierKind,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(DeferWhenOp::new(target, expr, modifier, source_span))
}

/// An op that represents an expression in an i18n message.
#[derive(Debug, Clone)]
pub struct I18nExpressionOp {
    /// The i18n context that this expression belongs to.
    pub context: XrefId,
    /// The Xref of the op that we need to `advance` to.
    pub target: XrefId,
    /// In an i18n block, this should be the i18n start op.
    /// In an i18n attribute, this will be the xref of the attribute configuration instruction.
    pub i18n_owner: XrefId,
    /// A handle for the slot that this expression modifies.
    pub handle: SlotHandle,
    /// The expression value.
    pub expression: Expression,
    /// ICU placeholder XrefId.
    pub icu_placeholder: Option<XrefId>,
    /// The i18n placeholder associated with this expression.
    pub i18n_placeholder: Option<String>,
    /// The time that this expression is resolved.
    pub resolution_time: I18nParamResolutionTime,
    /// Whether this i18n expression applies to a template or to a binding.
    pub usage: I18nExpressionFor,
    /// If this is an I18nExpressionContext.Binding, this expression is associated with a named attribute.
    pub name: Arc<str>,
    /// Source span.
    pub source_span: ParseSourceSpan,
}

impl I18nExpressionOp {
    pub fn new(
        context: XrefId,
        target: XrefId,
        i18n_owner: XrefId,
        handle: SlotHandle,
        expression: Expression,
        icu_placeholder: Option<XrefId>,
        i18n_placeholder: Option<String>,
        resolution_time: I18nParamResolutionTime,
        usage: I18nExpressionFor,
        name: Arc<str>,
        source_span: ParseSourceSpan,
    ) -> Self {
        I18nExpressionOp {
            context,
            target,
            i18n_owner,
            handle,
            expression,
            icu_placeholder,
            i18n_placeholder,
            resolution_time,
            usage,
            name,
            source_span,
        }
    }
}

impl Op for I18nExpressionOp {
    fn kind(&self) -> OpKind {
        OpKind::I18nExpression
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for I18nExpressionOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for I18nExpressionOp {}
impl DependsOnSlotContextOpTrait for I18nExpressionOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

unsafe impl Send for I18nExpressionOp {}
unsafe impl Sync for I18nExpressionOp {}

/// Create an i18n expression op.
pub fn create_i18n_expression_op(
    context: XrefId,
    target: XrefId,
    i18n_owner: XrefId,
    handle: SlotHandle,
    expression: Expression,
    icu_placeholder: Option<XrefId>,
    i18n_placeholder: Option<String>,
    resolution_time: I18nParamResolutionTime,
    usage: I18nExpressionFor,
    name: Arc<str>,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(I18nExpressionOp::new(
        context,
        target,
        i18n_owner,
        handle,
        expression,
        icu_placeholder,
        i18n_placeholder,
        resolution_time,
        usage,
        name,
        source_span,
    ))
}

/// An op that represents applying a set of i18n expressions.
#[derive(Debug, Clone)]
pub struct I18nApplyOp {
    /// In an i18n block, this should be the i18n start op.
    /// In an i18n attribute, this will be the xref of the attribute configuration instruction.
    pub owner: XrefId,
    /// A handle for the slot that i18n apply instruction should apply to.
    pub handle: SlotHandle,
    /// Source span.
    pub source_span: ParseSourceSpan,
}

impl I18nApplyOp {
    pub fn new(owner: XrefId, handle: SlotHandle, source_span: ParseSourceSpan) -> Self {
        I18nApplyOp {
            owner,
            handle,
            source_span,
        }
    }
}

impl Op for I18nApplyOp {
    fn kind(&self) -> OpKind {
        OpKind::I18nApply
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for I18nApplyOp {
    fn xref(&self) -> XrefId {
        // I18nApplyOp doesn't have a meaningful xref - return owner instead
        self.owner
    }
}

unsafe impl Send for I18nApplyOp {}
unsafe impl Sync for I18nApplyOp {}

/// Creates an op to apply i18n expression ops.
pub fn create_i18n_apply_op(
    owner: XrefId,
    handle: SlotHandle,
    source_span: ParseSourceSpan,
) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(I18nApplyOp::new(owner, handle, source_span))
}

/// A specialized PropertyOp that may bind a form field to a control.
#[derive(Debug, Clone)]
pub struct ControlOp {
    /// Reference to the element on which the property is bound.
    pub target: XrefId,
    /// Expression which is bound to the property.
    pub expression: BindingExpression,
    /// Binding kind.
    pub binding_kind: BindingKind,
    /// The security context of the binding.
    pub security_context: Vec<SecurityContext>,
    /// The sanitizer for this property.
    pub sanitizer: Option<Expression>,
    /// Whether this is a structural template attribute.
    pub is_structural_template_attribute: bool,
    /// The kind of template targeted by the binding, or None if this binding does not target a template.
    pub template_kind: Option<TemplateKind>,
    /// i18n context XrefId.
    pub i18n_context: Option<XrefId>,
    /// i18n message.
    pub i18n_message: Option<Message>,
    /// Source span.
    pub source_span: ParseSourceSpan,
}

impl ControlOp {
    pub fn from_binding_op(op: &BindingOp) -> Self {
        ControlOp {
            target: op.target,
            expression: op.expression.clone(),
            binding_kind: op.binding_kind,
            security_context: op.security_context.clone(),
            sanitizer: None,
            is_structural_template_attribute: op.is_structural_template_attribute,
            template_kind: op.template_kind,
            i18n_context: op.i18n_context,
            i18n_message: op.i18n_message.clone(),
            source_span: op.source_span.clone(),
        }
    }
}

impl Op for ControlOp {
    fn kind(&self) -> OpKind {
        OpKind::Control
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.source_span)
    }
}

impl UpdateOp for ControlOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

impl ConsumesVarsTrait for ControlOp {}
impl DependsOnSlotContextOpTrait for ControlOp {
    fn target(&self) -> XrefId {
        self.target
    }

    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
}

unsafe impl Send for ControlOp {}
unsafe impl Sync for ControlOp {}

/// Creates a ControlOp.
pub fn create_control_op(op: &BindingOp) -> Box<dyn UpdateOp + Send + Sync> {
    Box::new(ControlOp::from_binding_op(op))
}
