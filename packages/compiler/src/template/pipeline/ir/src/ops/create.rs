//! Create Operations
//!
//! Corresponds to packages/compiler/src/template/pipeline/ir/src/ops/create.ts
//! Defines create operations for template IR

use crate::core::SecurityContext;
use crate::i18n::i18n_ast::TagPlaceholder;
use crate::output::output_ast::Expression as OutputExpression;
use crate::parse_util::ParseSourceSpan;
use crate::template::pipeline::ir::enums::{
    AnimationKind, BindingKind, DeferOpModifierKind, I18nContextKind, I18nParamValueFlags,
    Namespace, OpKind, TemplateKind,
};
use crate::template::pipeline::ir::handle::{ConstIndex, SlotHandle, XrefId};
use crate::template::pipeline::ir::operations::{CreateOp, Op, OpList as IrOpList, UpdateOp};
use crate::template::pipeline::ir::ops::update::BindingExpression;
use crate::template::pipeline::ir::traits::ConsumesSlotOpTrait;
use crate::template::pipeline::ir::traits::ConsumesVarsTrait;
use std::collections::HashMap;
use std::sync::Arc;

/// Local reference on an element
#[derive(Debug, Clone)]
pub struct LocalRef {
    /// User-defined name of the local ref variable
    pub name: Arc<str>,
    /// Target of the local reference variable (often empty string)
    pub target: Arc<str>,
}

/// Base fields shared by element and container operations
#[derive(Debug, Clone)]
pub struct ElementOrContainerOpBase {
    /// XrefId allocated for this element
    pub xref: XrefId,
    /// Slot handle
    pub handle: SlotHandle,
    /// Attributes index in consts array
    pub attributes: Option<ConstIndex>,
    /// Local references index in consts array (populated by local_refs phase)
    pub local_refs_index: Option<ConstIndex>,
    /// Local references
    pub local_refs: Vec<LocalRef>,
    /// Whether marked ngNonBindable
    pub non_bindable: bool,
    /// Start source span
    pub start_source_span: ParseSourceSpan,
    /// Whole source span
    pub whole_source_span: ParseSourceSpan,
}

/// Base fields for element operations
#[derive(Debug, Clone)]
pub struct ElementOpBase {
    /// Base fields
    pub base: ElementOrContainerOpBase,
    /// HTML tag name
    pub tag: Option<String>,
    /// Namespace
    pub namespace: Namespace,
    /// Whether this element has matched directives
    pub has_directives: bool,
}

/// Logical operation representing the start of an element in the creation IR.
#[derive(Debug, Clone)]
pub struct ElementStartOp {
    /// Base element fields
    pub base: ElementOpBase,
    /// i18n placeholder data
    pub i18n_placeholder: Option<TagPlaceholder>,
}

impl ElementStartOp {
    pub fn new(
        tag: String,
        xref: XrefId,
        namespace: Namespace,
        i18n_placeholder: Option<TagPlaceholder>,
        start_source_span: ParseSourceSpan,
        whole_source_span: ParseSourceSpan,
    ) -> Self {
        ElementStartOp {
            base: ElementOpBase {
                base: ElementOrContainerOpBase {
                    xref,
                    handle: SlotHandle::default(),
                    attributes: None,
                    local_refs_index: None,
                    local_refs: Vec::new(),
                    non_bindable: false,
                    start_source_span: start_source_span.clone(),
                    whole_source_span: whole_source_span.clone(),
                },
                tag: Some(tag),
                namespace,
                has_directives: false,
            },
            i18n_placeholder,
        }
    }
}

impl Op for ElementStartOp {
    fn kind(&self) -> OpKind {
        OpKind::ElementStart
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.base.base.start_source_span)
    }
}

impl CreateOp for ElementStartOp {
    fn xref(&self) -> XrefId {
        self.base.base.xref
    }
}

impl ConsumesSlotOpTrait for ElementStartOp {
    fn handle(&self) -> &SlotHandle {
        &self.base.base.handle
    }

    fn num_slots_used(&self) -> usize {
        // After lift_local_refs phase, local_refs is cleared and moved to local_refs_index
        // So we need to check local_refs_index instead of local_refs
        if self.base.base.local_refs_index.is_some() || !self.base.base.local_refs.is_empty() {
            2
        } else {
            1
        }
    }

    fn xref(&self) -> XrefId {
        self.base.base.xref
    }
}

/// Logical operation representing an element with no children in the creation IR.
#[derive(Debug, Clone)]
pub struct ElementOp {
    /// Base element fields
    pub base: ElementOpBase,
    /// i18n placeholder data
    pub i18n_placeholder: Option<TagPlaceholder>,
}

impl Op for ElementOp {
    fn kind(&self) -> OpKind {
        OpKind::Element
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.base.base.start_source_span)
    }
}

impl CreateOp for ElementOp {
    fn xref(&self) -> XrefId {
        self.base.base.xref
    }
}

impl ConsumesSlotOpTrait for ElementOp {
    fn handle(&self) -> &SlotHandle {
        &self.base.base.handle
    }

    fn num_slots_used(&self) -> usize {
        // After lift_local_refs phase, local_refs is cleared and moved to local_refs_index
        // So we need to check local_refs_index instead of local_refs
        if self.base.base.local_refs_index.is_some() || !self.base.base.local_refs.is_empty() {
            2
        } else {
            1
        }
    }

    fn xref(&self) -> XrefId {
        self.base.base.xref
    }
}

/// Logical operation representing an embedded view declaration in the creation IR.
#[derive(Debug, Clone)]
pub struct TemplateOp {
    /// Base element fields
    pub base: ElementOpBase,
    /// Template kind
    pub template_kind: TemplateKind,
    /// Number of declaration slots
    pub decls: Option<usize>,
    /// Number of binding variable slots
    pub vars: Option<usize>,
    /// Function name suffix
    pub function_name_suffix: String,
    /// i18n placeholder data
    pub i18n_placeholder: Option<TagPlaceholder>, // Can also be BlockPlaceholder, simplified for now
}

impl TemplateOp {
    pub fn new(
        xref: XrefId,
        template_kind: TemplateKind,
        tag: Option<String>,
        function_name_suffix: String,
        namespace: Namespace,
        i18n_placeholder: Option<TagPlaceholder>,
        start_source_span: ParseSourceSpan,
        whole_source_span: ParseSourceSpan,
    ) -> Self {
        TemplateOp {
            base: ElementOpBase {
                base: ElementOrContainerOpBase {
                    xref,
                    handle: SlotHandle::default(),
                    attributes: None,
                    local_refs_index: None,
                    local_refs: Vec::new(),
                    non_bindable: false,
                    start_source_span: start_source_span.clone(),
                    whole_source_span: whole_source_span.clone(),
                },
                tag,
                namespace,
                has_directives: false,
            },
            template_kind,
            decls: None,
            vars: None,
            function_name_suffix,
            i18n_placeholder,
        }
    }
}

impl Op for TemplateOp {
    fn kind(&self) -> OpKind {
        OpKind::Template
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.base.base.start_source_span)
    }
}

impl CreateOp for TemplateOp {
    fn xref(&self) -> XrefId {
        self.base.base.xref
    }
}

impl ConsumesSlotOpTrait for TemplateOp {
    fn handle(&self) -> &SlotHandle {
        &self.base.base.handle
    }

    fn num_slots_used(&self) -> usize {
        if self.base.base.local_refs_index.is_some() {
            2
        } else {
            1
        }
    }

    fn xref(&self) -> XrefId {
        self.base.base.xref
    }
}

/// Logical operation representing the end of an element structure in the creation IR.
#[derive(Debug, Clone)]
pub struct ElementEndOp {
    /// The XrefId of the element declared via ElementStart
    pub xref: XrefId,
    /// Source span
    pub source_span: Option<ParseSourceSpan>,
}

impl ElementEndOp {
    pub fn new(xref: XrefId, source_span: Option<ParseSourceSpan>) -> Self {
        ElementEndOp { xref, source_span }
    }
}

impl Op for ElementEndOp {
    fn kind(&self) -> OpKind {
        OpKind::ElementEnd
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl CreateOp for ElementEndOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

// Helper functions to create operations (matching TypeScript API)
pub fn create_element_start_op(
    tag: String,
    xref: XrefId,
    namespace: Namespace,
    i18n_placeholder: Option<TagPlaceholder>,
    start_source_span: ParseSourceSpan,
    whole_source_span: ParseSourceSpan,
    has_directives: bool,
) -> Box<dyn CreateOp + Send + Sync> {
    let mut op = ElementStartOp::new(
        tag,
        xref,
        namespace,
        i18n_placeholder,
        start_source_span,
        whole_source_span,
    );
    op.base.has_directives = has_directives;
    Box::new(op)
}

pub fn create_template_op(
    xref: XrefId,
    template_kind: TemplateKind,
    tag: Option<String>,
    function_name_suffix: String,
    namespace: Namespace,
    i18n_placeholder: Option<TagPlaceholder>,
    start_source_span: ParseSourceSpan,
    whole_source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(TemplateOp::new(
        xref,
        template_kind,
        tag,
        function_name_suffix,
        namespace,
        i18n_placeholder,
        start_source_span,
        whole_source_span,
    ))
}

pub fn create_element_end_op(
    xref: XrefId,
    source_span: Option<ParseSourceSpan>,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(ElementEndOp::new(xref, source_span))
}

pub fn create_element_op(
    tag: String,
    xref: XrefId,
    namespace: Namespace,
    i18n_placeholder: Option<TagPlaceholder>,
    start_source_span: ParseSourceSpan,
    whole_source_span: ParseSourceSpan,
    has_directives: bool,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(ElementOp {
        base: ElementOpBase {
            base: ElementOrContainerOpBase {
                xref,
                handle: SlotHandle::default(),
                attributes: None,
                local_refs_index: None,
                local_refs: Vec::new(),
                non_bindable: false,
                start_source_span: start_source_span.clone(),
                whole_source_span: whole_source_span.clone(),
            },
            tag: Some(tag),
            namespace,
            has_directives,
        },
        i18n_placeholder,
    })
}

/// Logical operation representing the start of a container in the creation IR.
#[derive(Debug, Clone)]
pub struct ContainerStartOp {
    /// Base container fields
    pub base: ElementOrContainerOpBase,
}

impl ContainerStartOp {
    pub fn new(
        xref: XrefId,
        start_source_span: ParseSourceSpan,
        whole_source_span: ParseSourceSpan,
    ) -> Self {
        ContainerStartOp {
            base: ElementOrContainerOpBase {
                xref,
                handle: SlotHandle::default(),
                attributes: None,
                local_refs_index: None,
                local_refs: Vec::new(),
                non_bindable: false,
                start_source_span: start_source_span.clone(),
                whole_source_span: whole_source_span.clone(),
            },
        }
    }
}

impl Op for ContainerStartOp {
    fn kind(&self) -> OpKind {
        OpKind::ContainerStart
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.base.start_source_span)
    }
}

impl CreateOp for ContainerStartOp {
    fn xref(&self) -> XrefId {
        self.base.xref
    }
}

impl ConsumesSlotOpTrait for ContainerStartOp {
    fn handle(&self) -> &SlotHandle {
        &self.base.handle
    }

    fn num_slots_used(&self) -> usize {
        1
    }

    fn xref(&self) -> XrefId {
        self.base.xref
    }
}

/// Logical operation representing an empty container in the creation IR.
#[derive(Debug, Clone)]
pub struct ContainerOp {
    /// Base container fields
    pub base: ElementOrContainerOpBase,
}

impl Op for ContainerOp {
    fn kind(&self) -> OpKind {
        OpKind::Container
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.base.start_source_span)
    }
}

impl CreateOp for ContainerOp {
    fn xref(&self) -> XrefId {
        self.base.xref
    }
}

impl ConsumesSlotOpTrait for ContainerOp {
    fn handle(&self) -> &SlotHandle {
        &self.base.handle
    }

    fn num_slots_used(&self) -> usize {
        1
    }

    fn xref(&self) -> XrefId {
        self.base.xref
    }
}

/// Logical operation representing the end of a container structure in the creation IR.
#[derive(Debug, Clone)]
pub struct ContainerEndOp {
    /// The XrefId of the container declared via ContainerStart
    pub xref: XrefId,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl ContainerEndOp {
    pub fn new(xref: XrefId, source_span: ParseSourceSpan) -> Self {
        ContainerEndOp { xref, source_span }
    }
}

impl Op for ContainerEndOp {
    fn kind(&self) -> OpKind {
        OpKind::ContainerEnd
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

impl CreateOp for ContainerEndOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

/// Logical operation causing binding to be disabled in descendents of a non-bindable container.
#[derive(Debug, Clone)]
pub struct DisableBindingsOp {
    /// XrefId of the element that was marked non-bindable
    pub xref: XrefId,
}

impl DisableBindingsOp {
    pub fn new(xref: XrefId) -> Self {
        DisableBindingsOp { xref }
    }
}

impl Op for DisableBindingsOp {
    fn kind(&self) -> OpKind {
        OpKind::DisableBindings
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        None
    }
}

impl CreateOp for DisableBindingsOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

/// Logical operation causing binding to be re-enabled after visiting descendants of a non-bindable container.
#[derive(Debug, Clone)]
pub struct EnableBindingsOp {
    /// XrefId of the element that was marked non-bindable
    pub xref: XrefId,
}

impl EnableBindingsOp {
    pub fn new(xref: XrefId) -> Self {
        EnableBindingsOp { xref }
    }
}

impl Op for EnableBindingsOp {
    fn kind(&self) -> OpKind {
        OpKind::EnableBindings
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        None
    }
}

impl CreateOp for EnableBindingsOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

/// Logical operation representing a text node in the creation IR.
#[derive(Debug, Clone)]
pub struct TextOp {
    /// XrefId used to reference this text node in other IR structures
    pub xref: XrefId,
    /// Slot handle
    pub handle: SlotHandle,
    /// The static initial value of the text node
    pub initial_value: String,
    /// The placeholder for this text in its parent ICU. If this text is not part of an ICU, the placeholder is null.
    pub icu_placeholder: Option<String>,
    /// Source span
    pub source_span: Option<ParseSourceSpan>,
}

impl TextOp {
    pub fn new(
        xref: XrefId,
        initial_value: String,
        icu_placeholder: Option<String>,
        source_span: Option<ParseSourceSpan>,
    ) -> Self {
        TextOp {
            xref,
            handle: SlotHandle::default(),
            initial_value,
            icu_placeholder,
            source_span,
        }
    }
}

impl Op for TextOp {
    fn kind(&self) -> OpKind {
        OpKind::Text
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl CreateOp for TextOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

impl ConsumesSlotOpTrait for TextOp {
    fn handle(&self) -> &SlotHandle {
        &self.handle
    }

    fn num_slots_used(&self) -> usize {
        1
    }

    fn xref(&self) -> XrefId {
        self.xref
    }
}

// I18n operations
use crate::i18n::i18n_ast::Message as I18nMessage;

/// Base fields for i18n operations
#[derive(Debug, Clone)]
pub struct I18nOpBase {
    /// XrefId allocated for this i18n block
    pub xref: XrefId,
    /// Slot handle
    pub handle: SlotHandle,
    /// Root XrefId for this i18n block
    pub root: XrefId,
    /// The i18n message
    pub message: I18nMessage,
    /// The index in the consts array where the message i18n message is stored
    pub message_index: Option<ConstIndex>,
    /// Sub-template index (null initially)
    pub sub_template_index: Option<usize>,
    /// The i18n context generated from this block. Initially null, until the context is created
    pub context: Option<XrefId>,
}

/// Logical operation representing the start of an i18n block in the creation IR.
#[derive(Debug, Clone)]
pub struct I18nStartOp {
    /// Base i18n fields
    pub base: I18nOpBase,
    /// Source span
    pub source_span: Option<ParseSourceSpan>,
}

impl I18nStartOp {
    pub fn new(
        xref: XrefId,
        message: I18nMessage,
        root: Option<XrefId>,
        source_span: Option<ParseSourceSpan>,
    ) -> Self {
        I18nStartOp {
            base: I18nOpBase {
                xref,
                handle: SlotHandle::default(),
                root: root.unwrap_or(xref),
                message,
                message_index: None,
                sub_template_index: None,
                context: None,
            },
            source_span,
        }
    }
}

impl Op for I18nStartOp {
    fn kind(&self) -> OpKind {
        OpKind::I18nStart
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl CreateOp for I18nStartOp {
    fn xref(&self) -> XrefId {
        self.base.xref
    }
}

impl ConsumesSlotOpTrait for I18nStartOp {
    fn handle(&self) -> &SlotHandle {
        &self.base.handle
    }

    fn num_slots_used(&self) -> usize {
        1
    }

    fn xref(&self) -> XrefId {
        self.base.xref
    }
}

/// Logical operation representing the end of an i18n block in the creation IR.
#[derive(Debug, Clone)]
pub struct I18nEndOp {
    /// The XrefId of the I18nStartOp that created this block
    pub xref: XrefId,
    /// Source span
    pub source_span: Option<ParseSourceSpan>,
}

impl I18nEndOp {
    pub fn new(xref: XrefId, source_span: Option<ParseSourceSpan>) -> Self {
        I18nEndOp { xref, source_span }
    }
}

impl Op for I18nEndOp {
    fn kind(&self) -> OpKind {
        OpKind::I18nEnd
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl CreateOp for I18nEndOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

// Helper functions for container operations
pub fn create_container_start_op(
    xref: XrefId,
    start_source_span: ParseSourceSpan,
    whole_source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(ContainerStartOp::new(
        xref,
        start_source_span,
        whole_source_span,
    ))
}

pub fn create_disable_bindings_op(xref: XrefId) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(DisableBindingsOp::new(xref))
}

pub fn create_enable_bindings_op(xref: XrefId) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(EnableBindingsOp::new(xref))
}

pub fn create_text_op(
    xref: XrefId,
    initial_value: String,
    icu_placeholder: Option<String>,
    source_span: Option<ParseSourceSpan>,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(TextOp::new(
        xref,
        initial_value,
        icu_placeholder,
        source_span,
    ))
}

pub fn create_i18n_start_op(
    xref: XrefId,
    message: I18nMessage,
    root: Option<XrefId>,
    source_span: Option<ParseSourceSpan>,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(I18nStartOp::new(xref, message, root, source_span))
}

pub fn create_i18n_end_op(
    xref: XrefId,
    source_span: Option<ParseSourceSpan>,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(I18nEndOp::new(xref, source_span))
}

// Repeater operations
use crate::i18n::i18n_ast::BlockPlaceholder;

/// Variable names for repeater operations
#[derive(Debug, Clone)]
pub struct RepeaterVarNames {
    /// Set of $index variable names
    pub dollar_index: Vec<String>,
    /// The $implicit variable name
    pub dollar_implicit: String,
}

/// An op that creates a repeater (e.g. a for loop).
#[derive(Debug)]
pub struct RepeaterCreateOp {
    /// Base element fields
    pub base: ElementOpBase,
    /// Number of declaration slots
    pub decls: Option<usize>,
    /// Number of binding variable slots
    pub vars: Option<usize>,
    /// The Xref of the empty view function
    pub empty_view: Option<XrefId>,
    /// The track expression to use while iterating
    pub track: Box<OutputExpression>,
    /// Track ops if necessary
    pub track_by_ops: Option<IrOpList<Box<dyn UpdateOp + Send + Sync>>>,
    /// Track by function (null initially)
    pub track_by_fn: Option<Box<OutputExpression>>,
    /// Context variables available in this block
    pub var_names: RepeaterVarNames,
    /// Whether the repeater track function relies on the component instance
    pub uses_component_instance: bool,
    /// Function name suffix
    pub function_name_suffix: String,
    /// Tag name for the empty block
    pub empty_tag: Option<String>,
    /// Attributes of various kinds on the empty block
    pub empty_attributes: Option<ConstIndex>,
    /// The i18n placeholder for the repeated item template
    pub i18n_placeholder: Option<BlockPlaceholder>,
    /// The i18n placeholder for the empty template
    pub empty_i18n_placeholder: Option<BlockPlaceholder>,
}

impl RepeaterCreateOp {
    pub fn new(
        primary_view: XrefId,
        empty_view: Option<XrefId>,
        tag: Option<String>,
        track: Box<OutputExpression>,
        var_names: RepeaterVarNames,
        empty_tag: Option<String>,
        i18n_placeholder: Option<BlockPlaceholder>,
        empty_i18n_placeholder: Option<BlockPlaceholder>,
        start_source_span: ParseSourceSpan,
        whole_source_span: ParseSourceSpan,
    ) -> Self {
        RepeaterCreateOp {
            base: ElementOpBase {
                base: ElementOrContainerOpBase {
                    xref: primary_view,
                    handle: SlotHandle::default(),
                    attributes: None,
                    local_refs_index: None,
                    local_refs: Vec::new(),
                    non_bindable: false,
                    start_source_span: start_source_span.clone(),
                    whole_source_span: whole_source_span.clone(),
                },
                tag,
                namespace: Namespace::HTML,
                has_directives: false,
            },
            decls: None,
            vars: None,
            empty_view,
            track,
            track_by_ops: None,
            track_by_fn: None,
            var_names,
            uses_component_instance: false,
            function_name_suffix: "For".to_string(),
            empty_tag,
            empty_attributes: None,
            i18n_placeholder,
            empty_i18n_placeholder,
        }
    }
}

impl Op for RepeaterCreateOp {
    fn kind(&self) -> OpKind {
        OpKind::RepeaterCreate
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.base.base.start_source_span)
    }
}

impl CreateOp for RepeaterCreateOp {
    fn xref(&self) -> XrefId {
        self.base.base.xref
    }
}

impl ConsumesSlotOpTrait for RepeaterCreateOp {
    fn handle(&self) -> &SlotHandle {
        &self.base.base.handle
    }

    fn num_slots_used(&self) -> usize {
        if self.empty_view.is_some() {
            3
        } else {
            2
        }
    }

    fn xref(&self) -> XrefId {
        self.base.base.xref
    }
}

impl ConsumesVarsTrait for RepeaterCreateOp {}

// Safe to implement Send + Sync since Expression types don't actually need to be thread-safe
unsafe impl Send for RepeaterCreateOp {}
unsafe impl Sync for RepeaterCreateOp {}

// Conditional operations
/// An op that creates a conditional (e.g. a if or switch).
#[derive(Debug)]
pub struct ConditionalCreateOp {
    /// Base element fields
    pub base: ElementOpBase,
    /// Template kind
    pub template_kind: TemplateKind,
    /// Number of declaration slots
    pub decls: Option<usize>,
    /// Number of binding variable slots
    pub vars: Option<usize>,
    /// Function name suffix
    pub function_name_suffix: String,
    /// i18n placeholder data
    pub i18n_placeholder: Option<TagPlaceholder>, // Can also be BlockPlaceholder
}

impl ConditionalCreateOp {
    pub fn new(
        xref: XrefId,
        template_kind: TemplateKind,
        tag: Option<String>,
        function_name_suffix: String,
        namespace: Namespace,
        i18n_placeholder: Option<TagPlaceholder>,
        start_source_span: ParseSourceSpan,
        whole_source_span: ParseSourceSpan,
    ) -> Self {
        ConditionalCreateOp {
            base: ElementOpBase {
                base: ElementOrContainerOpBase {
                    xref,
                    handle: SlotHandle::default(),
                    attributes: None,
                    local_refs_index: None,
                    local_refs: Vec::new(),
                    non_bindable: false,
                    start_source_span: start_source_span.clone(),
                    whole_source_span: whole_source_span.clone(),
                },
                tag,
                namespace,
                has_directives: false,
            },
            template_kind,
            decls: None,
            vars: None,
            function_name_suffix,
            i18n_placeholder,
        }
    }
}

impl Op for ConditionalCreateOp {
    fn kind(&self) -> OpKind {
        OpKind::ConditionalCreate
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.base.base.start_source_span)
    }
}

impl CreateOp for ConditionalCreateOp {
    fn xref(&self) -> XrefId {
        self.base.base.xref
    }
}

impl ConsumesSlotOpTrait for ConditionalCreateOp {
    fn handle(&self) -> &SlotHandle {
        &self.base.base.handle
    }

    fn num_slots_used(&self) -> usize {
        1
    }

    fn xref(&self) -> XrefId {
        self.base.base.xref
    }
}

/// An op that creates a conditional branch (e.g. an else or case).
#[derive(Debug)]
pub struct ConditionalBranchCreateOp {
    /// Base element fields
    pub base: ElementOpBase,
    /// Template kind
    pub template_kind: TemplateKind,
    /// Number of declaration slots
    pub decls: Option<usize>,
    /// Number of binding variable slots
    pub vars: Option<usize>,
    /// Function name suffix
    pub function_name_suffix: String,
    /// i18n placeholder data
    pub i18n_placeholder: Option<TagPlaceholder>, // Can also be BlockPlaceholder
}

impl ConditionalBranchCreateOp {
    pub fn new(
        xref: XrefId,
        template_kind: TemplateKind,
        tag: Option<String>,
        function_name_suffix: String,
        namespace: Namespace,
        i18n_placeholder: Option<TagPlaceholder>,
        start_source_span: ParseSourceSpan,
        whole_source_span: ParseSourceSpan,
    ) -> Self {
        ConditionalBranchCreateOp {
            base: ElementOpBase {
                base: ElementOrContainerOpBase {
                    xref,
                    handle: SlotHandle::default(),
                    attributes: None,
                    local_refs_index: None,
                    local_refs: Vec::new(),
                    non_bindable: false,
                    start_source_span: start_source_span.clone(),
                    whole_source_span: whole_source_span.clone(),
                },
                tag,
                namespace,
                has_directives: false,
            },
            template_kind,
            decls: None,
            vars: None,
            function_name_suffix,
            i18n_placeholder,
        }
    }
}

impl Op for ConditionalBranchCreateOp {
    fn kind(&self) -> OpKind {
        OpKind::ConditionalBranchCreate
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.base.base.start_source_span)
    }
}

impl CreateOp for ConditionalBranchCreateOp {
    fn xref(&self) -> XrefId {
        self.base.base.xref
    }
}

impl ConsumesSlotOpTrait for ConditionalBranchCreateOp {
    fn handle(&self) -> &SlotHandle {
        &self.base.base.handle
    }

    fn num_slots_used(&self) -> usize {
        1
    }

    fn xref(&self) -> XrefId {
        self.base.base.xref
    }
}

// Projection operation
/// Logical operation representing a content projection in the creation IR.
#[derive(Debug)]
pub struct ProjectionOp {
    /// XrefId allocated for this projection
    pub xref: XrefId,
    /// Slot handle
    pub handle: SlotHandle,
    /// Projection slot index (set by generate_projection_def phase)
    pub projection_slot_index: usize,
    /// Source order - the order in which this ng-content was encountered during template ingestion.
    /// This captures the true template source order, used for correct projection slot assignment.
    pub source_order: usize,
    /// Attributes (null initially)
    pub attributes: Option<Box<OutputExpression>>,
    /// Local references
    pub local_refs: Vec<String>,
    /// Selector for this projection
    pub selector: String,
    /// i18n placeholder
    pub i18n_placeholder: Option<TagPlaceholder>,
    /// Source span
    pub source_span: ParseSourceSpan,
    /// Fallback view XrefId
    pub fallback_view: Option<XrefId>,
    /// Fallback view i18n placeholder
    pub fallback_view_i18n_placeholder: Option<BlockPlaceholder>,
}

impl ProjectionOp {
    pub fn new(
        xref: XrefId,
        selector: String,
        i18n_placeholder: Option<TagPlaceholder>,
        fallback_view: Option<XrefId>,
        source_span: ParseSourceSpan,
    ) -> Self {
        ProjectionOp {
            xref,
            handle: SlotHandle::default(),
            projection_slot_index: 0,
            source_order: 0,
            attributes: None,
            local_refs: Vec::new(),
            selector,
            i18n_placeholder,
            source_span,
            fallback_view,
            fallback_view_i18n_placeholder: None,
        }
    }
}

impl Op for ProjectionOp {
    fn kind(&self) -> OpKind {
        OpKind::Projection
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

impl CreateOp for ProjectionOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

impl ConsumesSlotOpTrait for ProjectionOp {
    fn handle(&self) -> &SlotHandle {
        &self.handle
    }

    fn num_slots_used(&self) -> usize {
        if self.fallback_view.is_some() {
            2
        } else {
            1
        }
    }

    fn xref(&self) -> XrefId {
        self.xref
    }
}

// Safe to implement Send + Sync since Expression types don't actually need to be thread-safe
unsafe impl Send for ProjectionOp {}
unsafe impl Sync for ProjectionOp {}

// Listener operation
/// Logical operation representing an event listener on an element in the creation IR.
#[derive(Debug)]
pub struct ListenerOp {
    /// Target XrefId (unique ID of this listener op)
    pub target: XrefId,
    /// Element XrefId (ID of the element/template this listener belongs to)
    pub element: XrefId,
    /// Target slot handle
    pub target_slot: SlotHandle,
    /// Whether this listener is from a host binding
    pub host_listener: bool,
    /// Name of the event which is being listened to
    pub name: Arc<str>,
    /// Tag name of the element on which this listener is placed
    pub tag: Option<String>,
    /// A list of UpdateOps representing the body of the event listener
    pub handler_ops: IrOpList<Box<dyn UpdateOp + Send + Sync>>,
    /// Name of the function
    pub handler_fn_name: Option<String>,
    /// Whether this listener is known to consume `$event` in its body
    pub consumes_dollar_event: bool,
    /// Whether the listener is listening for an animation event
    pub is_legacy_animation_listener: bool,
    /// The animation phase of the listener
    pub legacy_animation_phase: Option<String>,
    /// Some event listeners can have a target, e.g. in `document:dragover`
    pub event_target: Option<String>,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl ListenerOp {
    pub fn new(
        target: XrefId,
        element: XrefId,
        target_slot: SlotHandle,
        name: Arc<str>,
        tag: Option<String>,
        handler_ops: IrOpList<Box<dyn UpdateOp + Send + Sync>>,
        legacy_animation_phase: Option<String>,
        event_target: Option<String>,
        host_listener: bool,
        source_span: ParseSourceSpan,
        consumes_dollar_event: bool,
    ) -> Self {
        ListenerOp {
            target,
            element,
            target_slot,
            tag,
            host_listener,
            name,
            handler_ops,
            handler_fn_name: None,
            consumes_dollar_event,
            is_legacy_animation_listener: legacy_animation_phase.is_some(),
            legacy_animation_phase,
            event_target,
            source_span,
        }
    }
}

impl Op for ListenerOp {
    fn kind(&self) -> OpKind {
        OpKind::Listener
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

impl CreateOp for ListenerOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

// Helper functions
pub fn create_repeater_create_op(
    primary_view: XrefId,
    empty_view: Option<XrefId>,
    tag: Option<String>,
    track: Box<OutputExpression>,
    var_names: RepeaterVarNames,
    empty_tag: Option<String>,
    i18n_placeholder: Option<BlockPlaceholder>,
    empty_i18n_placeholder: Option<BlockPlaceholder>,
    start_source_span: ParseSourceSpan,
    whole_source_span: ParseSourceSpan,
) -> Box<dyn CreateOp> {
    Box::new(RepeaterCreateOp::new(
        primary_view,
        empty_view,
        tag,
        track,
        var_names,
        empty_tag,
        i18n_placeholder,
        empty_i18n_placeholder,
        start_source_span,
        whole_source_span,
    ))
}

pub fn create_conditional_create_op(
    xref: XrefId,
    template_kind: TemplateKind,
    tag: Option<String>,
    function_name_suffix: String,
    namespace: Namespace,
    i18n_placeholder: Option<TagPlaceholder>,
    start_source_span: ParseSourceSpan,
    whole_source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(ConditionalCreateOp::new(
        xref,
        template_kind,
        tag,
        function_name_suffix,
        namespace,
        i18n_placeholder,
        start_source_span,
        whole_source_span,
    ))
}

pub fn create_conditional_branch_create_op(
    xref: XrefId,
    template_kind: TemplateKind,
    tag: Option<String>,
    function_name_suffix: String,
    namespace: Namespace,
    i18n_placeholder: Option<TagPlaceholder>,
    start_source_span: ParseSourceSpan,
    whole_source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(ConditionalBranchCreateOp::new(
        xref,
        template_kind,
        tag,
        function_name_suffix,
        namespace,
        i18n_placeholder,
        start_source_span,
        whole_source_span,
    ))
}

pub fn create_projection_op(
    xref: XrefId,
    selector: String,
    i18n_placeholder: Option<TagPlaceholder>,
    fallback_view: Option<XrefId>,
    source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    // ProjectionOp does not contain Expression, so it's safe
    Box::new(ProjectionOp::new(
        xref,
        selector,
        i18n_placeholder,
        fallback_view,
        source_span,
    ))
}

pub fn create_listener_op(
    target: XrefId,
    element: XrefId,
    target_slot: SlotHandle,
    name: Arc<str>,
    tag: Option<String>,
    handler_ops: IrOpList<Box<dyn UpdateOp + Send + Sync>>,
    legacy_animation_phase: Option<String>,
    event_target: Option<String>,
    host_listener: bool,
    source_span: ParseSourceSpan,
    consumes_dollar_event: bool,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(ListenerOp::new(
        target,
        element,
        target_slot,
        name,
        tag,
        handler_ops,
        legacy_animation_phase,
        event_target,
        host_listener,
        source_span,
        consumes_dollar_event,
    ))
}

/// Defer operation
#[derive(Debug, Clone)]
pub struct DeferOp {
    /// XrefId allocated for this defer op
    pub xref: XrefId,
    /// Slot handle
    pub handle: SlotHandle,
    /// The xref of the main view
    pub main_view: XrefId,
    /// Main slot
    pub main_slot: SlotHandle,
    /// Secondary loading block associated with this defer op
    pub loading_view: Option<XrefId>,
    /// Loading slot
    pub loading_slot: Option<SlotHandle>,
    /// Secondary placeholder block associated with this defer op
    pub placeholder_view: Option<XrefId>,
    /// Placeholder slot
    pub placeholder_slot: Option<SlotHandle>,
    /// Secondary error block associated with this defer op
    pub error_view: Option<XrefId>,
    /// Error slot
    pub error_slot: Option<SlotHandle>,
    /// Placeholder minimum time
    pub placeholder_minimum_time: Option<f64>,
    /// Loading minimum time
    pub loading_minimum_time: Option<f64>,
    /// Loading after time
    pub loading_after_time: Option<f64>,
    /// Placeholder config expression
    pub placeholder_config: Option<OutputExpression>,
    /// Loading config expression
    pub loading_config: Option<OutputExpression>,
    /// Dependency resolution function for this specific deferred block
    pub own_resolver_fn: Option<OutputExpression>,
    /// Resolver function reference after extraction to constant pool
    pub resolver_fn: Option<OutputExpression>,
    /// Defer block flags (TODO: define TDeferDetailsFlags)
    pub flags: Option<u8>, // TDeferDetailsFlags - using u8 for now
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl DeferOp {
    pub fn new(
        xref: XrefId,
        main_view: XrefId,
        main_slot: SlotHandle,
        own_resolver_fn: Option<OutputExpression>,
        resolver_fn: Option<OutputExpression>,
        source_span: ParseSourceSpan,
    ) -> Self {
        DeferOp {
            xref,
            handle: SlotHandle::default(),
            main_view,
            main_slot,
            loading_view: None,
            loading_slot: None,
            placeholder_view: None,
            placeholder_slot: None,
            error_view: None,
            error_slot: None,
            placeholder_minimum_time: None,
            loading_minimum_time: None,
            loading_after_time: None,
            placeholder_config: None,
            loading_config: None,
            own_resolver_fn,
            resolver_fn,
            flags: None,
            source_span,
        }
    }
}

impl Op for DeferOp {
    fn kind(&self) -> OpKind {
        OpKind::Defer
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

impl CreateOp for DeferOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

impl ConsumesSlotOpTrait for DeferOp {
    fn handle(&self) -> &SlotHandle {
        &self.handle
    }

    fn num_slots_used(&self) -> usize {
        2
    }

    fn xref(&self) -> XrefId {
        self.xref
    }
}

unsafe impl Send for DeferOp {}
unsafe impl Sync for DeferOp {}

/// Create a DeferOp
pub fn create_defer_op(
    xref: XrefId,
    main_view: XrefId,
    main_slot: SlotHandle,
    own_resolver_fn: Option<OutputExpression>,
    resolver_fn: Option<OutputExpression>,
    source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(DeferOp::new(
        xref,
        main_view,
        main_slot,
        own_resolver_fn,
        resolver_fn,
        source_span,
    ))
}

/// ICU Start operation
#[derive(Debug, Clone)]
pub struct IcuStartOp {
    /// The ID of the ICU
    pub xref: XrefId,
    /// The i18n message for this ICU
    pub message: crate::i18n::i18n_ast::Message,
    /// Placeholder used to reference this ICU in other i18n messages
    pub message_placeholder: String,
    /// A reference to the i18n context for this op. Initially null, until the context is created
    pub context: Option<XrefId>,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl IcuStartOp {
    pub fn new(
        xref: XrefId,
        message: crate::i18n::i18n_ast::Message,
        message_placeholder: String,
        source_span: ParseSourceSpan,
    ) -> Self {
        IcuStartOp {
            xref,
            message,
            message_placeholder,
            context: None,
            source_span,
        }
    }
}

impl Op for IcuStartOp {
    fn kind(&self) -> OpKind {
        OpKind::IcuStart
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

impl CreateOp for IcuStartOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

/// Create an ICU start op
pub fn create_icu_start_op(
    xref: XrefId,
    message: crate::i18n::i18n_ast::Message,
    message_placeholder: String,
    source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(IcuStartOp::new(
        xref,
        message,
        message_placeholder,
        source_span,
    ))
}

/// ICU End operation
#[derive(Debug, Clone)]
pub struct IcuEndOp {
    /// The ID of the corresponding IcuStartOp
    pub xref: XrefId,
}

impl IcuEndOp {
    pub fn new(xref: XrefId) -> Self {
        IcuEndOp { xref }
    }
}

impl Op for IcuEndOp {
    fn kind(&self) -> OpKind {
        OpKind::IcuEnd
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        None
    }
}

impl CreateOp for IcuEndOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

/// Create an ICU end op
pub fn create_icu_end_op(xref: XrefId) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(IcuEndOp::new(xref))
}

/// Declare Let operation
#[derive(Debug, Clone)]
pub struct DeclareLetOp {
    /// XrefId allocated for this let declaration
    pub xref: XrefId,
    /// Slot handle
    pub handle: SlotHandle,
    /// The declared name
    pub declared_name: Arc<str>,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl DeclareLetOp {
    pub fn new(xref: XrefId, declared_name: Arc<str>, source_span: ParseSourceSpan) -> Self {
        DeclareLetOp {
            xref,
            handle: SlotHandle::default(),
            declared_name,
            source_span,
        }
    }
}

impl Op for DeclareLetOp {
    fn kind(&self) -> OpKind {
        OpKind::DeclareLet
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

impl CreateOp for DeclareLetOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

impl ConsumesSlotOpTrait for DeclareLetOp {
    fn handle(&self) -> &SlotHandle {
        &self.handle
    }

    fn num_slots_used(&self) -> usize {
        1
    }

    fn xref(&self) -> XrefId {
        self.xref
    }
}

/// Create a DeclareLetOp
pub fn create_declare_let_op(
    xref: XrefId,
    declared_name: Arc<str>,
    source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(DeclareLetOp::new(xref, declared_name, source_span))
}

// Additional operations that were missing

/// TwoWayListenerOp - Logical operation representing the event side of a two-way binding
#[derive(Debug)]
pub struct TwoWayListenerOp {
    /// Target XrefId
    pub target: XrefId,
    /// Element XrefId
    pub element: XrefId,
    /// Target slot handle
    pub target_slot: SlotHandle,
    /// Name of the event which is being listened to
    pub name: Arc<str>,
    /// Tag name of the element on which this listener is placed
    pub tag: Option<String>,
    /// A list of UpdateOps representing the body of the event listener
    pub handler_ops: IrOpList<Box<dyn UpdateOp + Send + Sync>>,
    /// Name of the function
    pub handler_fn_name: Option<String>,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl TwoWayListenerOp {
    pub fn new(
        target: XrefId,
        element: XrefId,
        target_slot: SlotHandle,
        name: Arc<str>,
        tag: Option<String>,
        handler_ops: IrOpList<Box<dyn UpdateOp + Send + Sync>>,
        source_span: ParseSourceSpan,
    ) -> Self {
        TwoWayListenerOp {
            target,
            element,
            target_slot,
            name,
            tag,
            handler_ops,
            handler_fn_name: None,
            source_span,
        }
    }
}
// ...
impl Op for TwoWayListenerOp {
    fn kind(&self) -> OpKind {
        OpKind::TwoWayListener
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

impl CreateOp for TwoWayListenerOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

unsafe impl Send for TwoWayListenerOp {}
unsafe impl Sync for TwoWayListenerOp {}

/// Create a TwoWayListenerOp
pub fn create_two_way_listener_op(
    target: XrefId,
    element: XrefId,
    target_slot: SlotHandle,
    name: Arc<str>,
    tag: Option<String>,
    handler_ops: IrOpList<Box<dyn UpdateOp + Send + Sync>>,
    source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(TwoWayListenerOp::new(
        target,
        element,
        target_slot,
        name,
        tag,
        handler_ops,
        source_span,
    ))
}

/// PipeOp - An operation to instantiate a pipe
#[derive(Debug, Clone)]
pub struct PipeOp {
    /// XrefId allocated for this pipe
    pub xref: XrefId,
    /// Slot handle
    pub handle: SlotHandle,
    /// Name of the pipe
    pub name: Arc<str>,
}

impl PipeOp {
    pub fn new(xref: XrefId, slot: SlotHandle, name: Arc<str>) -> Self {
        PipeOp {
            xref,
            handle: slot,
            name,
        }
    }
}

impl Op for PipeOp {
    fn kind(&self) -> OpKind {
        OpKind::Pipe
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        None
    }
}

impl CreateOp for PipeOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

impl ConsumesSlotOpTrait for PipeOp {
    fn handle(&self) -> &SlotHandle {
        &self.handle
    }

    fn num_slots_used(&self) -> usize {
        1
    }

    fn xref(&self) -> XrefId {
        self.xref
    }
}

/// NamespaceOp - An op corresponding to a namespace instruction, for switching between HTML, SVG, and MathML
#[derive(Debug, Clone)]
pub struct NamespaceOp {
    /// The active namespace
    pub active: Namespace,
}

impl NamespaceOp {
    pub fn new(namespace: Namespace) -> Self {
        NamespaceOp { active: namespace }
    }
}

impl Op for NamespaceOp {
    fn kind(&self) -> OpKind {
        OpKind::Namespace
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        None
    }
}

impl CreateOp for NamespaceOp {
    fn xref(&self) -> XrefId {
        // NamespaceOp doesn't have an xref - return dummy value
        XrefId::new(0)
    }
}

unsafe impl Send for NamespaceOp {}
unsafe impl Sync for NamespaceOp {}

/// Create a NamespaceOp
pub fn create_namespace_op(namespace: Namespace) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(NamespaceOp::new(namespace))
}

/// ProjectionDefOp - An op that creates a content projection definition for the view
#[derive(Debug, Clone)]
pub struct ProjectionDefOp {
    /// The parsed selector information for this projection def
    pub def: Option<OutputExpression>,
}

impl ProjectionDefOp {
    pub fn new(def: Option<OutputExpression>) -> Self {
        ProjectionDefOp { def }
    }
}

impl Op for ProjectionDefOp {
    fn kind(&self) -> OpKind {
        OpKind::ProjectionDef
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        None
    }
}

impl CreateOp for ProjectionDefOp {
    fn xref(&self) -> XrefId {
        // ProjectionDefOp doesn't have an xref - return dummy value
        XrefId::new(0)
    }
}

unsafe impl Send for ProjectionDefOp {}
unsafe impl Sync for ProjectionDefOp {}

/// Create a ProjectionDefOp
pub fn create_projection_def_op(def: Option<OutputExpression>) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(ProjectionDefOp::new(def))
}

/// DeferTrigger - Union type for defer trigger configurations
#[derive(Debug, Clone)]
pub enum DeferTrigger {
    Idle,
    Immediate,
    Never,
    Timer {
        delay: f64,
    },
    Hover {
        target_name: Option<String>,
        target_xref: Option<XrefId>,
        target_slot: Option<SlotHandle>,
        target_view: Option<XrefId>,
        target_slot_view_steps: Option<usize>,
    },
    Interaction {
        target_name: Option<String>,
        target_xref: Option<XrefId>,
        target_slot: Option<SlotHandle>,
        target_view: Option<XrefId>,
        target_slot_view_steps: Option<usize>,
    },
    Viewport {
        target_name: Option<String>,
        target_xref: Option<XrefId>,
        target_slot: Option<SlotHandle>,
        target_view: Option<XrefId>,
        target_slot_view_steps: Option<usize>,
        options: Option<OutputExpression>,
    },
}

/// ExtractedAttributeOp - Represents an attribute that has been extracted for inclusion in the consts array
#[derive(Debug, Clone)]
pub struct ExtractedAttributeOp {
    /// The `XrefId` of the template-like element the extracted attribute will belong to
    pub target: XrefId,
    /// The kind of binding represented by this extracted attribute
    pub binding_kind: BindingKind,
    /// The namespace of the attribute (or None if none)
    pub namespace: Option<String>,
    /// The name of the extracted attribute
    pub name: Arc<str>,
    /// The value expression of the extracted attribute
    pub expression: Option<OutputExpression>,
    /// If this attribute has a corresponding i18n attribute, then this is the i18n context for it
    pub i18n_context: Option<XrefId>,
    /// The security context of the binding
    pub security_context: Vec<SecurityContext>,
    /// The trusted value function for this property
    pub trusted_value_fn: Option<OutputExpression>,
    /// i18n message
    pub i18n_message: Option<I18nMessage>,
    /// Source span
    pub source_span: Option<ParseSourceSpan>,
}

impl ExtractedAttributeOp {
    pub fn new(
        target: XrefId,
        binding_kind: BindingKind,
        namespace: Option<String>,
        name: Arc<str>,
        expression: Option<OutputExpression>,
        i18n_context: Option<XrefId>,
        i18n_message: Option<I18nMessage>,
        security_context: Vec<SecurityContext>,
        source_span: Option<ParseSourceSpan>,
    ) -> Self {
        ExtractedAttributeOp {
            target,
            binding_kind,
            namespace,
            name,
            expression,
            i18n_context,
            security_context,
            trusted_value_fn: None,
            i18n_message,
            source_span,
        }
    }
}

impl Op for ExtractedAttributeOp {
    fn kind(&self) -> OpKind {
        OpKind::ExtractedAttribute
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl CreateOp for ExtractedAttributeOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

unsafe impl Send for ExtractedAttributeOp {}
unsafe impl Sync for ExtractedAttributeOp {}

/// Create an ExtractedAttributeOp
pub fn create_extracted_attribute_op(
    target: XrefId,
    binding_kind: BindingKind,
    namespace: Option<String>,
    name: Arc<str>,
    expression: Option<OutputExpression>,
    i18n_context: Option<XrefId>,
    i18n_message: Option<I18nMessage>,
    security_context: Vec<SecurityContext>,
    source_span: Option<ParseSourceSpan>,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(ExtractedAttributeOp::new(
        target,
        binding_kind,
        namespace,
        name,
        expression,
        i18n_context,
        i18n_message,
        security_context,
        source_span,
    ))
}

/// DeferOnOp - An operation that controls when a `@defer` loads
#[derive(Debug, Clone)]
pub struct DeferOnOp {
    /// The defer create op associated with this trigger
    pub defer: XrefId,
    /// The trigger for this defer op (e.g. idle, hover, etc)
    pub trigger: DeferTrigger,
    /// Modifier set on the trigger by the user (e.g. `hydrate`, `prefetch` etc)
    pub modifier: DeferOpModifierKind,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl DeferOnOp {
    pub fn new(
        defer: XrefId,
        trigger: DeferTrigger,
        modifier: DeferOpModifierKind,
        source_span: ParseSourceSpan,
    ) -> Self {
        DeferOnOp {
            defer,
            trigger,
            modifier,
            source_span,
        }
    }
}

impl Op for DeferOnOp {
    fn kind(&self) -> OpKind {
        OpKind::DeferOn
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

impl CreateOp for DeferOnOp {
    fn xref(&self) -> XrefId {
        // DeferOnOp doesn't have a meaningful xref - return defer instead
        self.defer
    }
}

unsafe impl Send for DeferOnOp {}
unsafe impl Sync for DeferOnOp {}

/// Create a DeferOnOp
pub fn create_defer_on_op(
    defer: XrefId,
    trigger: DeferTrigger,
    modifier: DeferOpModifierKind,
    source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(DeferOnOp::new(defer, trigger, modifier, source_span))
}

/// I18nParamValue - Represents a single value in an i18n param map
#[derive(Debug, Clone)]
pub enum I18nParamValueValue {
    String(String),
    Number(usize),
    Compound { element: usize, template: usize },
}

/// I18nParamValue - Represents a single value in an i18n param map
#[derive(Debug, Clone)]
pub struct I18nParamValue {
    /// The value - can be slot number, special string, or compound value
    pub value: I18nParamValueValue,
    /// The sub-template index associated with the value
    pub sub_template_index: Option<usize>,
    /// Flags associated with the value
    pub flags: I18nParamValueFlags,
}

/// I18nMessageOp - Represents an i18n message that has been extracted for inclusion in the consts array
#[derive(Debug, Clone)]
pub struct I18nMessageOp {
    /// An id used to reference this message
    pub xref: XrefId,
    /// The context from which this message was extracted
    pub i18n_context: XrefId,
    /// A reference to the i18n op this message was extracted from
    pub i18n_block: Option<XrefId>,
    /// The i18n message represented by this op
    pub message: I18nMessage,
    /// The placeholder used for this message when it is referenced in another message
    pub message_placeholder: Option<String>,
    /// Whether this message needs post-processing
    pub needs_postprocessing: bool,
    /// The param map, with placeholders represented as an Expression
    pub params: HashMap<String, OutputExpression>,
    /// The post-processing param map, with placeholders represented as an Expression
    pub postprocessing_params: HashMap<String, OutputExpression>,
    /// A list of sub-messages that are referenced by this message
    pub sub_messages: Vec<XrefId>,
}

impl I18nMessageOp {
    pub fn new(
        xref: XrefId,
        i18n_context: XrefId,
        i18n_block: Option<XrefId>,
        message: I18nMessage,
        message_placeholder: Option<String>,
        params: HashMap<String, OutputExpression>,
        postprocessing_params: HashMap<String, OutputExpression>,
        needs_postprocessing: bool,
    ) -> Self {
        I18nMessageOp {
            xref,
            i18n_context,
            i18n_block,
            message,
            message_placeholder,
            needs_postprocessing,
            params,
            postprocessing_params,
            sub_messages: Vec::new(),
        }
    }
}

impl Op for I18nMessageOp {
    fn kind(&self) -> OpKind {
        OpKind::I18nMessage
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        None
    }
}

impl CreateOp for I18nMessageOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

unsafe impl Send for I18nMessageOp {}
unsafe impl Sync for I18nMessageOp {}

/// Create an I18nMessageOp
pub fn create_i18n_message_op(
    xref: XrefId,
    i18n_context: XrefId,
    i18n_block: Option<XrefId>,
    message: I18nMessage,
    message_placeholder: Option<String>,
    params: HashMap<String, OutputExpression>,
    postprocessing_params: HashMap<String, OutputExpression>,
    needs_postprocessing: bool,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(I18nMessageOp::new(
        xref,
        i18n_context,
        i18n_block,
        message,
        message_placeholder,
        params,
        postprocessing_params,
        needs_postprocessing,
    ))
}

/// I18nOp - Represents an empty i18n block (non-start variant)
#[derive(Debug, Clone)]
pub struct I18nOp {
    /// Base i18n fields
    pub base: I18nOpBase,
    /// Source span
    pub source_span: Option<ParseSourceSpan>,
}

impl I18nOp {
    pub fn new(
        xref: XrefId,
        message: I18nMessage,
        root: Option<XrefId>,
        source_span: Option<ParseSourceSpan>,
    ) -> Self {
        I18nOp {
            base: I18nOpBase {
                xref,
                handle: SlotHandle::default(),
                root: root.unwrap_or(xref),
                message,
                message_index: None,
                sub_template_index: None,
                context: None,
            },
            source_span,
        }
    }
}

impl Op for I18nOp {
    fn kind(&self) -> OpKind {
        OpKind::I18n
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl CreateOp for I18nOp {
    fn xref(&self) -> XrefId {
        self.base.xref
    }
}

impl ConsumesSlotOpTrait for I18nOp {
    fn handle(&self) -> &SlotHandle {
        &self.base.handle
    }

    fn num_slots_used(&self) -> usize {
        1
    }

    fn xref(&self) -> XrefId {
        self.base.xref
    }
}

unsafe impl Send for I18nOp {}
unsafe impl Sync for I18nOp {}

/// Create an I18nOp
pub fn create_i18n_op(
    xref: XrefId,
    message: I18nMessage,
    root: Option<XrefId>,
    source_span: Option<ParseSourceSpan>,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(I18nOp::new(xref, message, root, source_span))
}

/// IcuPlaceholderOp - An op that represents a placeholder in an ICU expression
#[derive(Debug, Clone)]
pub struct IcuPlaceholderOp {
    /// The ID of the ICU placeholder
    pub xref: XrefId,
    /// The name of the placeholder in the ICU expression
    pub name: Arc<str>,
    /// The static strings to be combined with dynamic expression values
    pub strings: Vec<String>,
    /// Placeholder values for the i18n expressions
    pub expression_placeholders: Vec<I18nParamValue>,
}

impl IcuPlaceholderOp {
    pub fn new(xref: XrefId, name: Arc<str>, strings: Vec<String>) -> Self {
        IcuPlaceholderOp {
            xref,
            name,
            strings,
            expression_placeholders: Vec::new(),
        }
    }
}

impl Op for IcuPlaceholderOp {
    fn kind(&self) -> OpKind {
        OpKind::IcuPlaceholder
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        None
    }
}

impl CreateOp for IcuPlaceholderOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

unsafe impl Send for IcuPlaceholderOp {}
unsafe impl Sync for IcuPlaceholderOp {}

/// Create an IcuPlaceholderOp
pub fn create_icu_placeholder_op(
    xref: XrefId,
    name: Arc<str>,
    strings: Vec<String>,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(IcuPlaceholderOp::new(xref, name, strings))
}

/// I18nContextOp - An i18n context that is used to generate a translated i18n message
#[derive(Debug, Clone)]
pub struct I18nContextOp {
    /// The kind of context
    pub context_kind: I18nContextKind,
    /// The id of this context
    pub xref: XrefId,
    /// A reference to the I18nStartOp or I18nOp this context belongs to
    pub i18n_block: Option<XrefId>,
    /// The i18n message associated with this context
    pub message: I18nMessage,
    /// The param map for this context
    pub params: HashMap<String, Vec<I18nParamValue>>,
    /// The post-processing param map for this context
    pub postprocessing_params: HashMap<String, Vec<I18nParamValue>>,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl I18nContextOp {
    pub fn new(
        context_kind: I18nContextKind,
        xref: XrefId,
        i18n_block: Option<XrefId>,
        message: I18nMessage,
        source_span: ParseSourceSpan,
    ) -> Self {
        if i18n_block.is_none() && context_kind != I18nContextKind::Attr {
            panic!("AssertionError: i18nBlock must be provided for non-attribute contexts.");
        }

        I18nContextOp {
            context_kind,
            xref,
            i18n_block,
            message,
            params: HashMap::new(),
            postprocessing_params: HashMap::new(),
            source_span,
        }
    }
}

impl Op for I18nContextOp {
    fn kind(&self) -> OpKind {
        OpKind::I18nContext
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

impl CreateOp for I18nContextOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

unsafe impl Send for I18nContextOp {}
unsafe impl Sync for I18nContextOp {}

/// Create an I18nContextOp
pub fn create_i18n_context_op(
    context_kind: I18nContextKind,
    xref: XrefId,
    i18n_block: Option<XrefId>,
    message: I18nMessage,
    source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(I18nContextOp::new(
        context_kind,
        xref,
        i18n_block,
        message,
        source_span,
    ))
}

/// I18nAttributesOp - An op that creates i18n attributes configuration
#[derive(Debug, Clone)]
pub struct I18nAttributesOp {
    /// XrefId allocated for this i18n attributes op
    pub xref: XrefId,
    /// Slot handle
    pub handle: SlotHandle,
    /// The element targeted by these attributes
    pub target: XrefId,
    /// I18nAttributes instructions correspond to a const array with configuration information
    pub i18n_attributes_config: Option<ConstIndex>,
}

impl I18nAttributesOp {
    pub fn new(xref: XrefId, handle: SlotHandle, target: XrefId) -> Self {
        I18nAttributesOp {
            xref,
            handle,
            target,
            i18n_attributes_config: None,
        }
    }
}

impl Op for I18nAttributesOp {
    fn kind(&self) -> OpKind {
        OpKind::I18nAttributes
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        None
    }
}

impl CreateOp for I18nAttributesOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

impl ConsumesSlotOpTrait for I18nAttributesOp {
    fn handle(&self) -> &SlotHandle {
        &self.handle
    }

    fn num_slots_used(&self) -> usize {
        1
    }

    fn xref(&self) -> XrefId {
        self.xref
    }
}

unsafe impl Send for I18nAttributesOp {}
unsafe impl Sync for I18nAttributesOp {}

/// Create an I18nAttributesOp
pub fn create_i18n_attributes_op(
    xref: XrefId,
    handle: SlotHandle,
    target: XrefId,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(I18nAttributesOp::new(xref, handle, target))
}

/// AnimationListenerOp - A logical operation representing binding to an animation listener in the create IR
#[derive(Debug)]
pub struct AnimationListenerOp {
    /// Target XrefId (unique ID of this listener op)
    pub target: XrefId,
    /// Element XrefId (ID of the element/template this listener belongs to)
    pub element: XrefId,
    /// Target slot handle
    pub target_slot: SlotHandle,
    /// Whether this listener is from a host binding
    pub host_listener: bool,
    /// Name of the event which is being listened to
    pub name: Arc<str>,
    /// Whether the event is on enter or leave
    pub animation_kind: AnimationKind,
    /// Tag name of the element on which this listener is placed
    pub tag: Option<String>,
    /// A list of UpdateOps representing the body of the event listener
    pub handler_ops: IrOpList<Box<dyn UpdateOp + Send + Sync>>,
    /// Name of the function
    pub handler_fn_name: Option<String>,
    /// Whether this listener is known to consume `$event` in its body
    pub consumes_dollar_event: bool,
    /// Some event listeners can have a target, e.g. in `document:dragover`
    pub event_target: Option<String>,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl AnimationListenerOp {
    pub fn new(
        target: XrefId,
        element: XrefId,
        target_slot: SlotHandle,
        name: Arc<str>,
        tag: Option<String>,
        handler_ops: IrOpList<Box<dyn UpdateOp + Send + Sync>>,
        animation_kind: AnimationKind,
        event_target: Option<String>,
        host_listener: bool,
        source_span: ParseSourceSpan,
    ) -> Self {
        AnimationListenerOp {
            target,
            element,
            target_slot,
            host_listener,
            name,
            animation_kind,
            tag,
            handler_ops,
            handler_fn_name: None,
            consumes_dollar_event: false,
            event_target,
            source_span,
        }
    }
}

impl Op for AnimationListenerOp {
    fn kind(&self) -> OpKind {
        OpKind::AnimationListener
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

impl CreateOp for AnimationListenerOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

unsafe impl Send for AnimationListenerOp {}
unsafe impl Sync for AnimationListenerOp {}

/// Create an AnimationListenerOp
pub fn create_animation_listener_op(
    target: XrefId,
    element: XrefId,
    target_slot: SlotHandle,
    name: Arc<str>,
    tag: Option<String>,
    handler_ops: IrOpList<Box<dyn UpdateOp + Send + Sync>>,
    animation_kind: AnimationKind,
    event_target: Option<String>,
    host_listener: bool,
    source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(AnimationListenerOp::new(
        target,
        element,
        target_slot,
        name,
        tag,
        handler_ops,
        animation_kind,
        event_target,
        host_listener,
        source_span,
    ))
}

/// AnimationStringOp - A logical operation representing binding to an animation in the create IR
#[derive(Debug, Clone)]
pub struct AnimationStringOp {
    /// Target XrefId
    pub target: XrefId,
    /// The name of the extracted attribute
    pub name: Arc<str>,
    /// Kind of animation (enter or leave)
    pub animation_kind: AnimationKind,
    /// Expression which is bound to the property
    pub expression: BindingExpression,
    /// i18n message XrefId
    pub i18n_message: Option<XrefId>,
    /// The security context of the binding
    pub security_context: Vec<SecurityContext>,
    /// The sanitizer for this property
    pub sanitizer: Option<OutputExpression>,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl AnimationStringOp {
    pub fn new(
        name: Arc<str>,
        target: XrefId,
        animation_kind: AnimationKind,
        expression: BindingExpression,
        security_context: Vec<SecurityContext>,
        source_span: ParseSourceSpan,
    ) -> Self {
        AnimationStringOp {
            target,
            name,
            animation_kind,
            expression,
            i18n_message: None,
            security_context,
            sanitizer: None,
            source_span,
        }
    }
}

impl Op for AnimationStringOp {
    fn kind(&self) -> OpKind {
        OpKind::AnimationString
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

impl CreateOp for AnimationStringOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

unsafe impl Send for AnimationStringOp {}
unsafe impl Sync for AnimationStringOp {}

/// Create an AnimationStringOp
pub fn create_animation_string_op(
    name: Arc<str>,
    target: XrefId,
    animation_kind: AnimationKind,
    expression: BindingExpression,
    security_context: Vec<SecurityContext>,
    source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(AnimationStringOp::new(
        name,
        target,
        animation_kind,
        expression,
        security_context,
        source_span,
    ))
}

/// AnimationOp - A logical operation representing binding to an animation in the create IR
#[derive(Debug)]
pub struct AnimationOp {
    /// Target XrefId
    pub target: XrefId,
    /// The name of the extracted attribute
    pub name: Arc<str>,
    /// Kind of animation (enter or leave)
    pub animation_kind: AnimationKind,
    /// A list of UpdateOps representing the body of the callback function
    pub handler_ops: IrOpList<Box<dyn UpdateOp + Send + Sync>>,
    /// Name of the function
    pub handler_fn_name: Option<String>,
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl AnimationOp {
    pub fn new(
        name: Arc<str>,
        target: XrefId,
        animation_kind: AnimationKind,
        handler_ops: IrOpList<Box<dyn UpdateOp + Send + Sync>>,
        source_span: ParseSourceSpan,
    ) -> Self {
        AnimationOp {
            target,
            name,
            animation_kind,
            handler_ops,
            handler_fn_name: None,
            source_span,
        }
    }
}

impl Op for AnimationOp {
    fn kind(&self) -> OpKind {
        OpKind::Animation
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

impl CreateOp for AnimationOp {
    fn xref(&self) -> XrefId {
        self.target
    }
}

unsafe impl Send for AnimationOp {}
unsafe impl Sync for AnimationOp {}

/// Create an AnimationOp
pub fn create_animation_op(
    name: Arc<str>,
    target: XrefId,
    animation_kind: AnimationKind,
    handler_ops: IrOpList<Box<dyn UpdateOp + Send + Sync>>,
    source_span: ParseSourceSpan,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(AnimationOp::new(
        name,
        target,
        animation_kind,
        handler_ops,
        source_span,
    ))
}

/// ElementSourceLocation - Describes a location at which an element is defined within a template
#[derive(Debug, Clone)]
pub struct ElementSourceLocation {
    /// Target slot handle
    pub target_slot: SlotHandle,
    /// Offset
    pub offset: usize,
    /// Line number
    pub line: usize,
    /// Column number
    pub column: usize,
}

/// SourceLocationOp - Op that attaches the location at which each element is defined within the source template
#[derive(Debug, Clone)]
pub struct SourceLocationOp {
    /// Template path
    pub template_path: String,
    /// Locations of elements
    pub locations: Vec<ElementSourceLocation>,
}

impl SourceLocationOp {
    pub fn new(template_path: String, locations: Vec<ElementSourceLocation>) -> Self {
        SourceLocationOp {
            template_path,
            locations,
        }
    }
}

impl Op for SourceLocationOp {
    fn kind(&self) -> OpKind {
        OpKind::SourceLocation
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        None
    }
}

impl CreateOp for SourceLocationOp {
    fn xref(&self) -> XrefId {
        // SourceLocationOp doesn't have an xref - return dummy value
        XrefId::new(0)
    }
}

unsafe impl Send for SourceLocationOp {}
unsafe impl Sync for SourceLocationOp {}

/// Create a SourceLocationOp
pub fn create_source_location_op(
    template_path: String,
    locations: Vec<ElementSourceLocation>,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(SourceLocationOp::new(template_path, locations))
}

/// ControlCreateOp - An operation that determines whether a `[control]` binding targets a specialized control directive
#[derive(Debug, Clone)]
pub struct ControlCreateOp {
    /// Source span
    pub source_span: ParseSourceSpan,
}

impl ControlCreateOp {
    pub fn new(source_span: ParseSourceSpan) -> Self {
        ControlCreateOp { source_span }
    }
}

impl Op for ControlCreateOp {
    fn kind(&self) -> OpKind {
        OpKind::ControlCreate
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

impl CreateOp for ControlCreateOp {
    fn xref(&self) -> XrefId {
        // ControlCreateOp doesn't have an xref - return dummy value
        XrefId::new(0)
    }
}

unsafe impl Send for ControlCreateOp {}
unsafe impl Sync for ControlCreateOp {}

/// Create a ControlCreateOp
pub fn create_control_create_op(source_span: ParseSourceSpan) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(ControlCreateOp::new(source_span))
}

/// Logical operation representing a pipe creation in the creation IR.
#[derive(Debug, Clone)]
pub struct CreatePipeOp {
    /// XrefId allocated for this pipe
    pub xref: XrefId,
    /// Slot handle
    pub handle: SlotHandle,
    /// Name of the pipe
    pub name: Arc<str>,
}

impl CreatePipeOp {
    pub fn new(xref: XrefId, handle: SlotHandle, name: Arc<str>) -> Self {
        CreatePipeOp { xref, handle, name }
    }
}

impl Op for CreatePipeOp {
    fn kind(&self) -> OpKind {
        OpKind::Pipe
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        None
    }
}

impl CreateOp for CreatePipeOp {
    fn xref(&self) -> XrefId {
        self.xref
    }
}

impl ConsumesSlotOpTrait for CreatePipeOp {
    fn handle(&self) -> &SlotHandle {
        &self.handle
    }

    fn num_slots_used(&self) -> usize {
        1
    }

    fn xref(&self) -> XrefId {
        self.xref
    }
}

unsafe impl Send for CreatePipeOp {}
unsafe impl Sync for CreatePipeOp {}

pub fn create_pipe_op(
    xref: XrefId,
    handle: SlotHandle,
    name: Arc<str>,
) -> Box<dyn CreateOp + Send + Sync> {
    Box::new(CreatePipeOp::new(xref, handle, name))
}
