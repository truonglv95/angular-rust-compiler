//! Render3 AST
//!
//! Corresponds to packages/compiler/src/render3/r3_ast.ts
//! Contains AST node definitions for Render3 templates

use crate::core::SecurityContext;
use crate::expression_parser::ast::{
    AST as ExprAST, ASTWithSource, BindingType as ExprBindingType,
    ParsedEventType as ExprParsedEventType, LiteralMap,
};
use crate::i18n::i18n_ast::I18nMeta;
use crate::parse_util::ParseSourceSpan;
use std::collections::HashMap;

/// Base trait for all R3 AST nodes
pub trait Node {
    fn source_span(&self) -> &ParseSourceSpan;
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result;
}

/// Comment node - wrapper for raw html.Comment
#[derive(Debug, Clone)]
pub struct Comment {
    pub value: String,
    pub source_span: ParseSourceSpan,
}

impl Comment {
    pub fn new(value: String, source_span: ParseSourceSpan) -> Self {
        Comment { value, source_span }
    }
}

impl Node for Comment {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, _visitor: &mut V) -> V::Result {
        panic!("visit() not implemented for Comment")
    }
}

/// Text node
#[derive(Debug, Clone)]
pub struct Text {
    pub value: String,
    pub source_span: ParseSourceSpan,
}

impl Text {
    pub fn new(value: String, source_span: ParseSourceSpan) -> Self {
        Text { value, source_span }
    }
}

impl Node for Text {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_text(self)
    }
}

/// Bound text node (interpolation)
#[derive(Debug, Clone)]
pub struct BoundText {
    pub value: ExprAST,
    pub source_span: ParseSourceSpan,
    pub i18n: Option<I18nMeta>,
}

impl BoundText {
    pub fn new(value: ExprAST, source_span: ParseSourceSpan, i18n: Option<I18nMeta>) -> Self {
        BoundText { value, source_span, i18n }
    }
}

impl Node for BoundText {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_bound_text(self)
    }
}

/// Text attribute in the template
#[derive(Debug, Clone)]
pub struct TextAttribute {
    pub name: String,
    pub value: String,
    pub source_span: ParseSourceSpan,
    pub key_span: Option<ParseSourceSpan>,
    pub value_span: Option<ParseSourceSpan>,
    pub i18n: Option<I18nMeta>,
}

impl TextAttribute {
    pub fn new(
        name: String,
        value: String,
        source_span: ParseSourceSpan,
        key_span: Option<ParseSourceSpan>,
        value_span: Option<ParseSourceSpan>,
        i18n: Option<I18nMeta>,
    ) -> Self {
        TextAttribute {
            name,
            value,
            source_span,
            key_span,
            value_span,
            i18n,
        }
    }
}

impl Node for TextAttribute {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_text_attribute(self)
    }
}

/// Bound attribute node
#[derive(Debug, Clone)]
pub struct BoundAttribute {
    pub name: String,
    pub type_: ExprBindingType,
    pub security_context: SecurityContext,
    pub value: ExprAST,
    pub unit: Option<String>,
    pub source_span: ParseSourceSpan,
    pub key_span: ParseSourceSpan,
    pub value_span: Option<ParseSourceSpan>,
    pub i18n: Option<I18nMeta>,
}

impl BoundAttribute {
    pub fn new(
        name: String,
        type_: ExprBindingType,
        security_context: SecurityContext,
        value: ExprAST,
        unit: Option<String>,
        source_span: ParseSourceSpan,
        key_span: ParseSourceSpan,
        value_span: Option<ParseSourceSpan>,
        i18n: Option<I18nMeta>,
    ) -> Self {
        BoundAttribute {
            name,
            type_,
            security_context,
            value,
            unit,
            source_span,
            key_span,
            value_span,
            i18n,
        }
    }

    // TODO: Add from_bound_element_property when BoundElementProperty is available
}

impl Node for BoundAttribute {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_bound_attribute(self)
    }
}

/// Bound event node
#[derive(Debug, Clone)]
pub struct BoundEvent {
    pub name: String,
    pub type_: ExprParsedEventType,
    pub handler: ExprAST,
    pub target: Option<String>,
    pub phase: Option<String>,
    pub source_span: ParseSourceSpan,
    pub handler_span: ParseSourceSpan,
    pub key_span: ParseSourceSpan,
}

impl BoundEvent {
    pub fn new(
        name: String,
        type_: ExprParsedEventType,
        handler: ExprAST,
        target: Option<String>,
        phase: Option<String>,
        source_span: ParseSourceSpan,
        handler_span: ParseSourceSpan,
        key_span: ParseSourceSpan,
    ) -> Self {
        BoundEvent {
            name,
            type_,
            handler,
            target,
            phase,
            source_span,
            handler_span,
            key_span,
        }
    }
}

impl Node for BoundEvent {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_bound_event(self)
    }
}

/// Element node
#[derive(Debug, Clone)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<TextAttribute>,
    pub inputs: Vec<BoundAttribute>,
    pub outputs: Vec<BoundEvent>,
    pub directives: Vec<Directive>,
    pub children: Vec<R3Node>,
    pub references: Vec<Reference>,
    pub is_self_closing: bool,
    pub source_span: ParseSourceSpan,
    pub start_source_span: ParseSourceSpan,
    pub end_source_span: Option<ParseSourceSpan>,
    pub is_void: bool,
    pub i18n: Option<I18nMeta>,
}

impl Element {
    pub fn new(
        name: String,
        attributes: Vec<TextAttribute>,
        inputs: Vec<BoundAttribute>,
        outputs: Vec<BoundEvent>,
        directives: Vec<Directive>,
        children: Vec<R3Node>,
        references: Vec<Reference>,
        is_self_closing: bool,
        source_span: ParseSourceSpan,
        start_source_span: ParseSourceSpan,
        end_source_span: Option<ParseSourceSpan>,
        is_void: bool,
        i18n: Option<I18nMeta>,
    ) -> Self {
        Element {
            name,
            attributes,
            inputs,
            outputs,
            directives,
            children,
            references,
            is_self_closing,
            source_span,
            start_source_span,
            end_source_span,
            is_void,
            i18n,
        }
    }
}

impl Node for Element {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_element(self)
    }
}

/// Block node base
#[derive(Debug, Clone)]
pub struct BlockNode {
    pub name_span: ParseSourceSpan,
    pub source_span: ParseSourceSpan,
    pub start_source_span: ParseSourceSpan,
    pub end_source_span: Option<ParseSourceSpan>,
}

impl BlockNode {
    pub fn new(
        name_span: ParseSourceSpan,
        source_span: ParseSourceSpan,
        start_source_span: ParseSourceSpan,
        end_source_span: Option<ParseSourceSpan>,
    ) -> Self {
        BlockNode {
            name_span,
            source_span,
            start_source_span,
            end_source_span,
        }
    }
}

/// Deferred trigger types
#[derive(Debug, Clone)]
pub enum DeferredTrigger {
    Bound(BoundDeferredTrigger),
    Never(NeverDeferredTrigger),
    Idle(IdleDeferredTrigger),
    Immediate(ImmediateDeferredTrigger),
    Hover(HoverDeferredTrigger),
    Timer(TimerDeferredTrigger),
    Interaction(InteractionDeferredTrigger),
    Viewport(ViewportDeferredTrigger),
}

impl Node for DeferredTrigger {
    fn source_span(&self) -> &ParseSourceSpan {
        match self {
            DeferredTrigger::Bound(t) => &t.source_span,
            DeferredTrigger::Never(t) => &t.source_span,
            DeferredTrigger::Idle(t) => &t.source_span,
            DeferredTrigger::Immediate(t) => &t.source_span,
            DeferredTrigger::Hover(t) => &t.source_span,
            DeferredTrigger::Timer(t) => &t.source_span,
            DeferredTrigger::Interaction(t) => &t.source_span,
            DeferredTrigger::Viewport(t) => &t.source_span,
        }
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_deferred_trigger(self)
    }
}

/// Base for deferred triggers
#[derive(Debug, Clone)]
pub struct DeferredTriggerBase {
    pub name_span: Option<ParseSourceSpan>,
    pub source_span: ParseSourceSpan,
    pub prefetch_span: Option<ParseSourceSpan>,
    pub when_or_on_source_span: Option<ParseSourceSpan>,
    pub hydrate_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct BoundDeferredTrigger {
    pub value: ExprAST,
    pub source_span: ParseSourceSpan,
    pub prefetch_span: Option<ParseSourceSpan>,
    pub when_source_span: ParseSourceSpan,
    pub hydrate_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct NeverDeferredTrigger {
    pub name_span: Option<ParseSourceSpan>,
    pub source_span: ParseSourceSpan,
    pub prefetch_span: Option<ParseSourceSpan>,
    pub when_or_on_source_span: Option<ParseSourceSpan>,
    pub hydrate_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct IdleDeferredTrigger {
    pub name_span: Option<ParseSourceSpan>,
    pub source_span: ParseSourceSpan,
    pub prefetch_span: Option<ParseSourceSpan>,
    pub when_or_on_source_span: Option<ParseSourceSpan>,
    pub hydrate_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct ImmediateDeferredTrigger {
    pub name_span: Option<ParseSourceSpan>,
    pub source_span: ParseSourceSpan,
    pub prefetch_span: Option<ParseSourceSpan>,
    pub when_or_on_source_span: Option<ParseSourceSpan>,
    pub hydrate_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct HoverDeferredTrigger {
    pub reference: Option<String>,
    pub name_span: ParseSourceSpan,
    pub source_span: ParseSourceSpan,
    pub prefetch_span: Option<ParseSourceSpan>,
    pub on_source_span: Option<ParseSourceSpan>,
    pub hydrate_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct TimerDeferredTrigger {
    pub delay: i64,
    pub name_span: ParseSourceSpan,
    pub source_span: ParseSourceSpan,
    pub prefetch_span: Option<ParseSourceSpan>,
    pub on_source_span: Option<ParseSourceSpan>,
    pub hydrate_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct InteractionDeferredTrigger {
    pub reference: Option<String>,
    pub name_span: ParseSourceSpan,
    pub source_span: ParseSourceSpan,
    pub prefetch_span: Option<ParseSourceSpan>,
    pub on_source_span: Option<ParseSourceSpan>,
    pub hydrate_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct ViewportDeferredTrigger {
    pub reference: Option<String>,
    pub options: Option<LiteralMap>,
    pub name_span: ParseSourceSpan,
    pub source_span: ParseSourceSpan,
    pub prefetch_span: Option<ParseSourceSpan>,
    pub on_source_span: Option<ParseSourceSpan>,
    pub hydrate_span: Option<ParseSourceSpan>,
}

/// Deferred block triggers collection
#[derive(Debug, Clone, Default)]
pub struct DeferredBlockTriggers {
    pub when: Option<BoundDeferredTrigger>,
    pub idle: Option<IdleDeferredTrigger>,
    pub immediate: Option<ImmediateDeferredTrigger>,
    pub hover: Option<HoverDeferredTrigger>,
    pub timer: Option<TimerDeferredTrigger>,
    pub interaction: Option<InteractionDeferredTrigger>,
    pub viewport: Option<ViewportDeferredTrigger>,
    pub never: Option<NeverDeferredTrigger>,
}

/// Deferred block placeholder
#[derive(Debug, Clone)]
pub struct DeferredBlockPlaceholder {
    pub children: Vec<R3Node>,
    pub minimum_time: Option<i64>,
    pub block: BlockNode,
    pub i18n: Option<I18nMeta>,
}

impl Node for DeferredBlockPlaceholder {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.block.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_deferred_block_placeholder(self)
    }
}

/// Deferred block loading
#[derive(Debug, Clone)]
pub struct DeferredBlockLoading {
    pub children: Vec<R3Node>,
    pub after_time: Option<i64>,
    pub minimum_time: Option<i64>,
    pub block: BlockNode,
    pub i18n: Option<I18nMeta>,
}

impl Node for DeferredBlockLoading {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.block.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_deferred_block_loading(self)
    }
}

/// Deferred block error
#[derive(Debug, Clone)]
pub struct DeferredBlockError {
    pub children: Vec<R3Node>,
    pub block: BlockNode,
    pub i18n: Option<I18nMeta>,
}

impl Node for DeferredBlockError {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.block.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_deferred_block_error(self)
    }
}

/// Deferred block
#[derive(Debug, Clone)]
pub struct DeferredBlock {
    pub children: Vec<R3Node>,
    pub triggers: DeferredBlockTriggers,
    pub prefetch_triggers: DeferredBlockTriggers,
    pub hydrate_triggers: DeferredBlockTriggers,
    pub placeholder: Option<Box<DeferredBlockPlaceholder>>,
    pub loading: Option<Box<DeferredBlockLoading>>,
    pub error: Option<Box<DeferredBlockError>>,
    pub block: BlockNode,
    pub main_block_span: ParseSourceSpan,
    pub i18n: Option<I18nMeta>,
}

impl Node for DeferredBlock {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.block.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_deferred_block(self)
    }
}

/// Switch block
#[derive(Debug, Clone)]
pub struct SwitchBlock {
    pub expression: ExprAST,
    pub cases: Vec<SwitchBlockCase>,
    pub unknown_blocks: Vec<UnknownBlock>,
    pub block: BlockNode,
}

impl Node for SwitchBlock {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.block.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_switch_block(self)
    }
}

/// Switch block case
#[derive(Debug, Clone)]
pub struct SwitchBlockCase {
    pub expression: Option<ExprAST>,
    pub children: Vec<R3Node>,
    pub block: BlockNode,
    pub i18n: Option<I18nMeta>,
}

impl Node for SwitchBlockCase {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.block.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_switch_block_case(self)
    }
}

/// For loop block
#[derive(Debug, Clone)]
pub struct ForLoopBlock {
    pub item: Variable,
    pub expression: ASTWithSource,
    pub track_by: ASTWithSource,
    pub track_keyword_span: ParseSourceSpan,
    pub context_variables: Vec<Variable>,
    pub children: Vec<R3Node>,
    pub empty: Option<Box<ForLoopBlockEmpty>>,
    pub block: BlockNode,
    pub main_block_span: ParseSourceSpan,
    pub i18n: Option<I18nMeta>,
}

impl Node for ForLoopBlock {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.block.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_for_loop_block(self)
    }
}

/// For loop block empty
#[derive(Debug, Clone)]
pub struct ForLoopBlockEmpty {
    pub children: Vec<R3Node>,
    pub block: BlockNode,
    pub i18n: Option<I18nMeta>,
}

impl Node for ForLoopBlockEmpty {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.block.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_for_loop_block_empty(self)
    }
}

/// If block
#[derive(Debug, Clone)]
pub struct IfBlock {
    pub branches: Vec<IfBlockBranch>,
    pub block: BlockNode,
}

impl Node for IfBlock {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.block.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_if_block(self)
    }
}

/// If block branch
#[derive(Debug, Clone)]
pub struct IfBlockBranch {
    pub expression: Option<ExprAST>,
    pub children: Vec<R3Node>,
    pub expression_alias: Option<Variable>,
    pub block: BlockNode,
    pub i18n: Option<I18nMeta>,
}

impl Node for IfBlockBranch {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.block.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_if_block_branch(self)
    }
}

/// Unknown block (for autocompletion)
#[derive(Debug, Clone)]
pub struct UnknownBlock {
    pub name: String,
    pub source_span: ParseSourceSpan,
    pub name_span: ParseSourceSpan,
}

impl Node for UnknownBlock {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_unknown_block(self)
    }
}

/// Let declaration
#[derive(Debug, Clone)]
pub struct LetDeclaration {
    pub name: String,
    pub value: ExprAST,
    pub source_span: ParseSourceSpan,
    pub name_span: ParseSourceSpan,
    pub value_span: ParseSourceSpan,
}

impl Node for LetDeclaration {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_let_declaration(self)
    }
}

/// Component node
#[derive(Debug, Clone)]
pub struct Component {
    pub component_name: String,
    pub tag_name: Option<String>,
    pub full_name: String,
    pub attributes: Vec<TextAttribute>,
    pub inputs: Vec<BoundAttribute>,
    pub outputs: Vec<BoundEvent>,
    pub directives: Vec<Directive>,
    pub children: Vec<R3Node>,
    pub references: Vec<Reference>,
    pub is_self_closing: bool,
    pub source_span: ParseSourceSpan,
    pub start_source_span: ParseSourceSpan,
    pub end_source_span: Option<ParseSourceSpan>,
    pub i18n: Option<I18nMeta>,
}

impl Node for Component {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_component(self)
    }
}

/// Directive node
#[derive(Debug, Clone)]
pub struct Directive {
    pub name: String,
    pub attributes: Vec<TextAttribute>,
    pub inputs: Vec<BoundAttribute>,
    pub outputs: Vec<BoundEvent>,
    pub references: Vec<Reference>,
    pub source_span: ParseSourceSpan,
    pub start_source_span: ParseSourceSpan,
    pub end_source_span: Option<ParseSourceSpan>,
    pub i18n: Option<I18nMeta>,
}

impl Node for Directive {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_directive(self)
    }
}

/// Template node
#[derive(Debug, Clone)]
pub struct Template {
    pub tag_name: Option<String>,
    pub attributes: Vec<TextAttribute>,
    pub inputs: Vec<BoundAttribute>,
    pub outputs: Vec<BoundEvent>,
    pub directives: Vec<Directive>,
    pub template_attrs: Vec<TemplateAttr>,
    pub children: Vec<R3Node>,
    pub references: Vec<Reference>,
    pub variables: Vec<Variable>,
    pub is_self_closing: bool,
    pub source_span: ParseSourceSpan,
    pub start_source_span: ParseSourceSpan,
    pub end_source_span: Option<ParseSourceSpan>,
    pub i18n: Option<I18nMeta>,
}

/// Template attribute (either bound or text)
#[derive(Debug, Clone)]
pub enum TemplateAttr {
    Bound(BoundAttribute),
    Text(TextAttribute),
}

impl Node for Template {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_template(self)
    }
}

/// Content node (ng-content)
#[derive(Debug, Clone)]
pub struct Content {
    pub selector: String,
    pub attributes: Vec<TextAttribute>,
    pub children: Vec<R3Node>,
    pub is_self_closing: bool,
    pub source_span: ParseSourceSpan,
    pub start_source_span: ParseSourceSpan,
    pub end_source_span: Option<ParseSourceSpan>,
    pub i18n: Option<I18nMeta>,
}

impl Content {
    pub fn name(&self) -> &str {
        "ng-content"
    }
}

impl Node for Content {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_content(self)
    }
}

/// Variable node
#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub source_span: ParseSourceSpan,
    pub key_span: ParseSourceSpan,
    pub value_span: Option<ParseSourceSpan>,
}

impl Node for Variable {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_variable(self)
    }
}

/// Reference node
#[derive(Debug, Clone)]
pub struct Reference {
    pub name: String,
    pub value: String,
    pub source_span: ParseSourceSpan,
    pub key_span: ParseSourceSpan,
    pub value_span: Option<ParseSourceSpan>,
}

impl Node for Reference {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_reference(self)
    }
}

/// ICU node
#[derive(Debug, Clone)]
pub struct Icu {
    pub vars: HashMap<String, BoundText>,
    pub placeholders: HashMap<String, IcuPlaceholder>,
    pub source_span: ParseSourceSpan,
    pub i18n: Option<I18nMeta>,
}

#[derive(Debug, Clone)]
pub enum IcuPlaceholder {
    Text(Text),
    BoundText(BoundText),
}

impl Node for Icu {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Result {
        visitor.visit_icu(self)
    }
}

/// Host element (for type checking only)
#[derive(Debug, Clone)]
pub struct HostElement {
    pub tag_names: Vec<String>,
    pub bindings: Vec<BoundAttribute>,
    pub listeners: Vec<BoundEvent>,
    pub source_span: ParseSourceSpan,
}

impl HostElement {
    pub fn new(
        tag_names: Vec<String>,
        bindings: Vec<BoundAttribute>,
        listeners: Vec<BoundEvent>,
        source_span: ParseSourceSpan,
    ) -> Self {
        if tag_names.is_empty() {
            panic!("HostElement must have at least one tag name.");
        }
        HostElement {
            tag_names,
            bindings,
            listeners,
            source_span,
        }
    }
}

impl Node for HostElement {
    fn source_span(&self) -> &ParseSourceSpan {
        &self.source_span
    }
    
    fn visit<V: Visitor>(&self, _visitor: &mut V) -> V::Result {
        panic!("HostElement cannot be visited")
    }
}

/// Enum for all R3 node types
#[derive(Debug, Clone)]
pub enum R3Node {
    Comment(Comment),
    Text(Text),
    BoundText(BoundText),
    TextAttribute(TextAttribute),
    BoundAttribute(BoundAttribute),
    BoundEvent(BoundEvent),
    Element(Element),
    DeferredTrigger(DeferredTrigger),
    DeferredBlockPlaceholder(DeferredBlockPlaceholder),
    DeferredBlockLoading(DeferredBlockLoading),
    DeferredBlockError(DeferredBlockError),
    DeferredBlock(DeferredBlock),
    SwitchBlock(SwitchBlock),
    SwitchBlockCase(SwitchBlockCase),
    ForLoopBlock(ForLoopBlock),
    ForLoopBlockEmpty(ForLoopBlockEmpty),
    IfBlock(IfBlock),
    IfBlockBranch(IfBlockBranch),
    UnknownBlock(UnknownBlock),
    LetDeclaration(LetDeclaration),
    Component(Component),
    Directive(Directive),
    Template(Template),
    Content(Content),
    Variable(Variable),
    Reference(Reference),
    Icu(Icu),
    HostElement(HostElement),
}

/// Visitor trait for R3 AST
pub trait Visitor {
    type Result;
    
    fn visit(&mut self, _node: &R3Node) -> Option<Self::Result> {
        None
    }
    
    fn visit_element(&mut self, element: &Element) -> Self::Result;
    fn visit_template(&mut self, template: &Template) -> Self::Result;
    fn visit_content(&mut self, content: &Content) -> Self::Result;
    fn visit_variable(&mut self, variable: &Variable) -> Self::Result;
    fn visit_reference(&mut self, reference: &Reference) -> Self::Result;
    fn visit_text_attribute(&mut self, attribute: &TextAttribute) -> Self::Result;
    fn visit_bound_attribute(&mut self, attribute: &BoundAttribute) -> Self::Result;
    fn visit_bound_event(&mut self, attribute: &BoundEvent) -> Self::Result;
    fn visit_text(&mut self, text: &Text) -> Self::Result;
    fn visit_bound_text(&mut self, text: &BoundText) -> Self::Result;
    fn visit_icu(&mut self, icu: &Icu) -> Self::Result;
    fn visit_deferred_block(&mut self, deferred: &DeferredBlock) -> Self::Result;
    fn visit_deferred_block_placeholder(&mut self, block: &DeferredBlockPlaceholder) -> Self::Result;
    fn visit_deferred_block_error(&mut self, block: &DeferredBlockError) -> Self::Result;
    fn visit_deferred_block_loading(&mut self, block: &DeferredBlockLoading) -> Self::Result;
    fn visit_deferred_trigger(&mut self, trigger: &DeferredTrigger) -> Self::Result;
    fn visit_switch_block(&mut self, block: &SwitchBlock) -> Self::Result;
    fn visit_switch_block_case(&mut self, block: &SwitchBlockCase) -> Self::Result;
    fn visit_for_loop_block(&mut self, block: &ForLoopBlock) -> Self::Result;
    fn visit_for_loop_block_empty(&mut self, block: &ForLoopBlockEmpty) -> Self::Result;
    fn visit_if_block(&mut self, block: &IfBlock) -> Self::Result;
    fn visit_if_block_branch(&mut self, block: &IfBlockBranch) -> Self::Result;
    fn visit_unknown_block(&mut self, block: &UnknownBlock) -> Self::Result;
    fn visit_let_declaration(&mut self, decl: &LetDeclaration) -> Self::Result;
    fn visit_component(&mut self, component: &Component) -> Self::Result;
    fn visit_directive(&mut self, directive: &Directive) -> Self::Result;
    
    // Optional method for visiting comments (not in TypeScript Visitor interface but used in tests)
    // Note: This method must be implemented by each visitor implementation
    // It's not in the TypeScript Visitor interface but is used in test code
    fn visit_comment(&mut self, _comment: &Comment) -> Self::Result;
}

/// Visit all nodes in a list
pub fn visit_all<V: Visitor>(visitor: &mut V, nodes: &[R3Node]) -> Vec<V::Result> {
    let mut result = Vec::new();
    for node in nodes {
        if let Some(r) = visitor.visit(node) {
            result.push(r);
        } else {
            let r = match node {
                R3Node::Text(n) => n.visit(visitor),
                R3Node::BoundText(n) => n.visit(visitor),
                R3Node::TextAttribute(n) => n.visit(visitor),
                R3Node::BoundAttribute(n) => n.visit(visitor),
                R3Node::BoundEvent(n) => n.visit(visitor),
                R3Node::Element(n) => n.visit(visitor),
                R3Node::DeferredTrigger(n) => n.visit(visitor),
                R3Node::DeferredBlockPlaceholder(n) => n.visit(visitor),
                R3Node::DeferredBlockLoading(n) => n.visit(visitor),
                R3Node::DeferredBlockError(n) => n.visit(visitor),
                R3Node::DeferredBlock(n) => n.visit(visitor),
                R3Node::SwitchBlock(n) => n.visit(visitor),
                R3Node::SwitchBlockCase(n) => n.visit(visitor),
                R3Node::ForLoopBlock(n) => n.visit(visitor),
                R3Node::ForLoopBlockEmpty(n) => n.visit(visitor),
                R3Node::IfBlock(n) => n.visit(visitor),
                R3Node::IfBlockBranch(n) => n.visit(visitor),
                R3Node::UnknownBlock(n) => n.visit(visitor),
                R3Node::LetDeclaration(n) => n.visit(visitor),
                R3Node::Component(n) => n.visit(visitor),
                R3Node::Directive(n) => n.visit(visitor),
                R3Node::Template(n) => n.visit(visitor),
                R3Node::Content(n) => n.visit(visitor),
                R3Node::Variable(n) => n.visit(visitor),
                R3Node::Reference(n) => n.visit(visitor),
                R3Node::Icu(n) => n.visit(visitor),
                R3Node::Comment(_) | R3Node::HostElement(_) => continue,
            };
            result.push(r);
        }
    }
    result
}

