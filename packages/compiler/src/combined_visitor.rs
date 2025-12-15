//! Combined Visitor Module
//!
//! Corresponds to packages/compiler/src/combined_visitor.ts
//! Visitor that traverses all template and expression AST nodes in a template.
//! Useful for cases where every single node needs to be visited.

use crate::expression_parser::ast::{AST, ASTWithSource, RecursiveAstVisitor};
use crate::render3::r3_ast as t;
use crate::render3::r3_ast::Visitor;

/// Visitor that traverses all template and expression AST nodes in a template.
/// Useful for cases where every single node needs to be visited.
pub struct CombinedRecursiveAstVisitor {
    expr_visitor: RecursiveAstVisitor,
}

impl CombinedRecursiveAstVisitor {
    pub fn new() -> Self {
        CombinedRecursiveAstVisitor {
            expr_visitor: RecursiveAstVisitor::new(),
        }
    }

    /// Visit an AST expression node
    pub fn visit_ast(&mut self, ast: &AST) {
        self.expr_visitor.visit(ast);
    }

    /// Visit an ASTWithSource node (extracts and visits the inner AST)
    pub fn visit_ast_with_source(&mut self, ast_with_source: &ASTWithSource) {
        self.visit_ast(&ast_with_source.ast);
    }

    /// Visit all template nodes
    fn visit_all_template_nodes(&mut self, nodes: &[t::R3Node]) {
        use t::visit_all;
        visit_all(self, nodes);
    }

    /// Helper to visit all triggers in a DeferredBlockTriggers collection
    fn visit_deferred_block_triggers(&mut self, triggers: &t::DeferredBlockTriggers) {
        // Visit when trigger
        if let Some(ref when) = triggers.when {
            self.visit_deferred_trigger(&t::DeferredTrigger::Bound(when.clone()));
        }
        // Visit idle trigger
        if let Some(ref idle) = triggers.idle {
            self.visit_deferred_trigger(&t::DeferredTrigger::Idle(idle.clone()));
        }
        // Visit immediate trigger
        if let Some(ref immediate) = triggers.immediate {
            self.visit_deferred_trigger(&t::DeferredTrigger::Immediate(immediate.clone()));
        }
        // Visit hover trigger
        if let Some(ref hover) = triggers.hover {
            self.visit_deferred_trigger(&t::DeferredTrigger::Hover(hover.clone()));
        }
        // Visit timer trigger
        if let Some(ref timer) = triggers.timer {
            self.visit_deferred_trigger(&t::DeferredTrigger::Timer(timer.clone()));
        }
        // Visit interaction trigger
        if let Some(ref interaction) = triggers.interaction {
            self.visit_deferred_trigger(&t::DeferredTrigger::Interaction(interaction.clone()));
        }
        // Visit viewport trigger
        if let Some(ref viewport) = triggers.viewport {
            self.visit_deferred_trigger(&t::DeferredTrigger::Viewport(viewport.clone()));
        }
        // Visit never trigger
        if let Some(ref never) = triggers.never {
            self.visit_deferred_trigger(&t::DeferredTrigger::Never(never.clone()));
        }
    }
}

impl Default for CombinedRecursiveAstVisitor {
    fn default() -> Self {
        Self::new()
    }
}


// Implement Visitor trait for R3 AST nodes
impl Visitor for CombinedRecursiveAstVisitor {
    type Result = ();

    fn visit_element(&mut self, element: &t::Element) {
        // Visit attributes (convert TextAttribute to R3Node)
        for attr in &element.attributes {
            self.visit_all_template_nodes(&[t::R3Node::TextAttribute(attr.clone())]);
        }
        // Visit inputs (convert BoundAttribute to R3Node)
        for input in &element.inputs {
            self.visit_all_template_nodes(&[t::R3Node::BoundAttribute(input.clone())]);
        }
        // Visit outputs (convert BoundEvent to R3Node)
        for output in &element.outputs {
            self.visit_all_template_nodes(&[t::R3Node::BoundEvent(output.clone())]);
        }
        // Visit directives (convert Directive to R3Node)
        for dir in &element.directives {
            self.visit_all_template_nodes(&[t::R3Node::Directive(dir.clone())]);
        }
        // Visit references (convert Reference to R3Node)
        for ref_node in &element.references {
            self.visit_all_template_nodes(&[t::R3Node::Reference(ref_node.clone())]);
        }
        // Visit children
        self.visit_all_template_nodes(&element.children);
    }

    fn visit_template(&mut self, template: &t::Template) {
        // Visit attributes (convert TextAttribute to R3Node)
        for attr in &template.attributes {
            self.visit_all_template_nodes(&[t::R3Node::TextAttribute(attr.clone())]);
        }
        // Visit inputs (convert BoundAttribute to R3Node)
        for input in &template.inputs {
            self.visit_all_template_nodes(&[t::R3Node::BoundAttribute(input.clone())]);
        }
        // Visit outputs (convert BoundEvent to R3Node)
        for output in &template.outputs {
            self.visit_all_template_nodes(&[t::R3Node::BoundEvent(output.clone())]);
        }
        // Visit directives (convert Directive to R3Node)
        for dir in &template.directives {
            self.visit_all_template_nodes(&[t::R3Node::Directive(dir.clone())]);
        }
        // Visit template_attrs
        for attr in &template.template_attrs {
            match attr {
                t::TemplateAttr::Text(ta) => {
                    self.visit_all_template_nodes(&[t::R3Node::TextAttribute(ta.clone())]);
                }
                t::TemplateAttr::Bound(ba) => {
                    self.visit_all_template_nodes(&[t::R3Node::BoundAttribute(ba.clone())]);
                }
            }
        }
        // Visit variables (convert Variable to R3Node)
        for var in &template.variables {
            self.visit_all_template_nodes(&[t::R3Node::Variable(var.clone())]);
        }
        // Visit references (convert Reference to R3Node)
        for ref_node in &template.references {
            self.visit_all_template_nodes(&[t::R3Node::Reference(ref_node.clone())]);
        }
        // Visit children
        self.visit_all_template_nodes(&template.children);
    }

    fn visit_content(&mut self, content: &t::Content) {
        // Visit attributes (convert TextAttribute to R3Node)
        for attr in &content.attributes {
            self.visit_all_template_nodes(&[t::R3Node::TextAttribute(attr.clone())]);
        }
        self.visit_all_template_nodes(&content.children);
    }

    fn visit_bound_attribute(&mut self, attribute: &t::BoundAttribute) {
        // BoundAttribute has value as ExprAST directly
        self.visit_ast(&attribute.value);
    }

    fn visit_bound_event(&mut self, event: &t::BoundEvent) {
        // BoundEvent has handler as ExprAST directly
        self.visit_ast(&event.handler);
    }

    fn visit_bound_text(&mut self, text: &t::BoundText) {
        // BoundText has value as ExprAST directly
        self.visit_ast(&text.value);
    }

    fn visit_icu(&mut self, icu: &t::Icu) {
        // Visit all vars (which are BoundText containing AST expressions)
        for (_key, bound_text) in &icu.vars {
            self.visit_ast(&bound_text.value);
        }
        // Visit all placeholders
        for (_key, placeholder) in &icu.placeholders {
            match placeholder {
                t::IcuPlaceholder::Text(_) => {
                    // Text placeholders don't contain AST
                }
                t::IcuPlaceholder::BoundText(bt) => {
                    self.visit_ast(&bt.value);
                }
            }
        }
    }

    fn visit_deferred_block(&mut self, deferred: &t::DeferredBlock) {
        // Visit hydrate triggers first (to match insertion order)
        self.visit_deferred_block_triggers(&deferred.hydrate_triggers);
        // Visit regular triggers
        self.visit_deferred_block_triggers(&deferred.triggers);
        // Visit prefetch triggers
        self.visit_deferred_block_triggers(&deferred.prefetch_triggers);
        // Visit children
        self.visit_all_template_nodes(&deferred.children);
        // Visit connected blocks (placeholder, loading, error)
        if let Some(ref placeholder) = deferred.placeholder {
            self.visit_all_template_nodes(&[t::R3Node::DeferredBlockPlaceholder((**placeholder).clone())]);
        }
        if let Some(ref loading) = deferred.loading {
            self.visit_all_template_nodes(&[t::R3Node::DeferredBlockLoading((**loading).clone())]);
        }
        if let Some(ref error) = deferred.error {
            self.visit_all_template_nodes(&[t::R3Node::DeferredBlockError((**error).clone())]);
        }
    }

    fn visit_deferred_trigger(&mut self, trigger: &t::DeferredTrigger) {
        match trigger {
            t::DeferredTrigger::Bound(b) => {
                // BoundDeferredTrigger has value as ExprAST directly
                self.visit_ast(&b.value);
            }
            t::DeferredTrigger::Viewport(v) => {
                // ViewportDeferredTrigger has options as Option<LiteralMap>
                if let Some(ref options) = v.options {
                    // LiteralMap contains AST values
                    for value in &options.values {
                        self.visit_ast(value);
                    }
                }
            }
            _ => {
                // Other trigger types don't contain AST
            }
        }
    }

    fn visit_deferred_block_placeholder(&mut self, block: &t::DeferredBlockPlaceholder) {
        self.visit_all_template_nodes(&block.children);
    }

    fn visit_deferred_block_error(&mut self, block: &t::DeferredBlockError) {
        self.visit_all_template_nodes(&block.children);
    }

    fn visit_deferred_block_loading(&mut self, block: &t::DeferredBlockLoading) {
        self.visit_all_template_nodes(&block.children);
    }

    fn visit_switch_block(&mut self, block: &t::SwitchBlock) {
        // SwitchBlock has expression as ExprAST directly
        self.visit_ast(&block.expression);
        // Visit all cases
        for case in &block.cases {
            self.visit_all_template_nodes(&[t::R3Node::SwitchBlockCase(case.clone())]);
        }
    }

    fn visit_switch_block_case(&mut self, block: &t::SwitchBlockCase) {
        // SwitchBlockCase has expression as Option<ExprAST>
        if let Some(ref expr) = block.expression {
            self.visit_ast(expr);
        }
        self.visit_all_template_nodes(&block.children);
    }

    fn visit_for_loop_block(&mut self, block: &t::ForLoopBlock) {
        // Visit the item variable
        self.visit_all_template_nodes(&[t::R3Node::Variable(block.item.clone())]);
        // Visit context variables
        for var in &block.context_variables {
            self.visit_all_template_nodes(&[t::R3Node::Variable(var.clone())]);
        }
        // ForLoopBlock has expression and track_by as ASTWithSource
        self.visit_ast(&block.expression.ast);
        self.visit_ast(&block.track_by.ast);
        self.visit_all_template_nodes(&block.children);
        if let Some(ref empty) = block.empty {
            self.visit_all_template_nodes(&[t::R3Node::ForLoopBlockEmpty((**empty).clone())]);
        }
    }

    fn visit_for_loop_block_empty(&mut self, block: &t::ForLoopBlockEmpty) {
        self.visit_all_template_nodes(&block.children);
    }

    fn visit_if_block(&mut self, block: &t::IfBlock) {
        // Visit all branches
        for branch in &block.branches {
            self.visit_all_template_nodes(&[t::R3Node::IfBlockBranch(branch.clone())]);
        }
    }

    fn visit_if_block_branch(&mut self, block: &t::IfBlockBranch) {
        // IfBlockBranch has expression as Option<ExprAST>
        if let Some(ref expr) = block.expression {
            self.visit_ast(expr);
        }
        // Visit expression alias if present
        if let Some(ref expr_alias) = block.expression_alias {
            self.visit_all_template_nodes(&[t::R3Node::Variable(expr_alias.clone())]);
        }
        self.visit_all_template_nodes(&block.children);
    }

    fn visit_let_declaration(&mut self, decl: &t::LetDeclaration) {
        // LetDeclaration has value as ExprAST directly
        self.visit_ast(&decl.value);
    }

    fn visit_component(&mut self, component: &t::Component) {
        // Visit attributes (convert TextAttribute to R3Node)
        for attr in &component.attributes {
            self.visit_all_template_nodes(&[t::R3Node::TextAttribute(attr.clone())]);
        }
        // Visit inputs (convert BoundAttribute to R3Node)
        for input in &component.inputs {
            self.visit_all_template_nodes(&[t::R3Node::BoundAttribute(input.clone())]);
        }
        // Visit outputs (convert BoundEvent to R3Node)
        for output in &component.outputs {
            self.visit_all_template_nodes(&[t::R3Node::BoundEvent(output.clone())]);
        }
        // Visit directives (convert Directive to R3Node)
        for dir in &component.directives {
            self.visit_all_template_nodes(&[t::R3Node::Directive(dir.clone())]);
        }
        // Visit references (convert Reference to R3Node)
        for ref_node in &component.references {
            self.visit_all_template_nodes(&[t::R3Node::Reference(ref_node.clone())]);
        }
        // Visit children
        self.visit_all_template_nodes(&component.children);
    }

    fn visit_directive(&mut self, directive: &t::Directive) {
        // Visit attributes (convert TextAttribute to R3Node)
        for attr in &directive.attributes {
            self.visit_all_template_nodes(&[t::R3Node::TextAttribute(attr.clone())]);
        }
        // Visit inputs (convert BoundAttribute to R3Node)
        for input in &directive.inputs {
            self.visit_all_template_nodes(&[t::R3Node::BoundAttribute(input.clone())]);
        }
        // Visit outputs (convert BoundEvent to R3Node)
        for output in &directive.outputs {
            self.visit_all_template_nodes(&[t::R3Node::BoundEvent(output.clone())]);
        }
        // Visit references (convert Reference to R3Node)
        for ref_node in &directive.references {
            self.visit_all_template_nodes(&[t::R3Node::Reference(ref_node.clone())]);
        }
    }

    fn visit_variable(&mut self, _variable: &t::Variable) {
        // Variables don't contain AST expressions to visit
    }

    fn visit_reference(&mut self, _reference: &t::Reference) {
        // References don't contain AST expressions to visit
    }

    fn visit_text_attribute(&mut self, _attribute: &t::TextAttribute) {
        // Text attributes don't contain AST expressions to visit
    }

    fn visit_text(&mut self, _text: &t::Text) {
        // Text nodes don't contain AST expressions to visit
    }

    fn visit_unknown_block(&mut self, _block: &t::UnknownBlock) {
        // Unknown blocks don't contain AST expressions to visit
    }
}
