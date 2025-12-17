//! Expression Humanization Utilities for Tests
//!
//! Mirrors angular/packages/compiler/test/render3/util/expression.ts

use angular_compiler::expression_parser::ast::{AST, ASTWithSource};
use angular_compiler::expression_parser::ast::AbsoluteSourceSpan;
use angular_compiler::render3::r3_ast as t;
use angular_compiler::render3::r3_ast::Visitor;

// Include unparser from test utilities  
#[path = "../../expression_parser/utils/unparser.rs"]
mod unparser_mod;
pub use unparser_mod::unparse;

/// Humanized expression source - tuple of (unparsed_string, absolute_source_span)
pub type HumanizedExpressionSource = (String, AbsoluteSourceSpan);

/// Visitor that collects all expressions with their source spans
struct ExpressionSourceHumanizer {
    result: Vec<HumanizedExpressionSource>,
}

impl ExpressionSourceHumanizer {
    fn new() -> Self {
        ExpressionSourceHumanizer {
            result: vec![],
        }
    }
    
    fn record_ast(&mut self, ast: &AST) {
        let unparsed = unparse(ast);
        let span = ast.source_span();
        // AbsoluteSourceSpan is Copy, so we can use it directly
        self.result.push((unparsed, span));
    }
    
    fn visit_ast(&mut self, ast: &AST) {
        self.record_ast(ast);
        // Recursively visit child nodes
        match ast {
            AST::Binary(b) => {
                self.visit_ast(&*b.left);
                self.visit_ast(&*b.right);
            }
            AST::Conditional(c) => {
                self.visit_ast(&*c.condition);
                self.visit_ast(&*c.true_exp);
                // Conditional.false_exp is Box<AST>, not Option
                self.visit_ast(&*c.false_exp);
            }
            AST::Chain(ch) => {
                for expr in &ch.expressions {
                    self.visit_ast(expr);
                }
            }
            AST::Call(c) => {
                self.visit_ast(&*c.receiver);
                for arg in &c.args {
                    self.visit_ast(arg);
                }
            }
            AST::SafeCall(c) => {
                self.visit_ast(&*c.receiver);
                for arg in &c.args {
                    self.visit_ast(arg);
                }
            }
            AST::PropertyRead(p) => {
                self.visit_ast(&*p.receiver);
            }
            AST::SafePropertyRead(p) => {
                self.visit_ast(&*p.receiver);
            }
            AST::KeyedRead(k) => {
                self.visit_ast(&*k.receiver);
                self.visit_ast(&*k.key);
            }
            AST::SafeKeyedRead(k) => {
                self.visit_ast(&*k.receiver);
                self.visit_ast(&*k.key);
            }
            AST::KeyedWrite(k) => {
                self.visit_ast(&*k.receiver);
                self.visit_ast(&*k.key);
                self.visit_ast(&*k.value);
            }
            AST::PropertyWrite(p) => {
                self.visit_ast(&*p.receiver);
                self.visit_ast(&*p.value);
            }
            AST::Interpolation(i) => {
                for expr in &i.expressions {
                    self.visit_ast(expr);
                }
            }
            AST::LiteralArray(a) => {
                for expr in &a.expressions {
                    self.visit_ast(expr);
                }
            }
            AST::LiteralMap(m) => {
                for value in &m.values {
                    self.visit_ast(value);
                }
            }
            AST::BindingPipe(p) => {
                self.visit_ast(&*p.exp);
                for arg in &p.args {
                    self.visit_ast(arg);
                }
            }
            AST::PrefixNot(p) => {
                self.visit_ast(&*p.expression);
            }
            AST::NonNullAssert(n) => {
                self.visit_ast(&*n.expression);
            }
            AST::Unary(u) => {
                self.visit_ast(&*u.expr);
            }
            AST::TemplateLiteral(tl) => {
                for expr in &tl.expressions {
                    self.visit_ast(expr);
                }
            }
            AST::TaggedTemplateLiteral(ttl) => {
                self.visit_ast(&*ttl.tag);
                // TaggedTemplateLiteral has template: TemplateLiteral, not expressions
                // TemplateLiteral has parts which are strings, not expressions
                // For now, just visit the tag
            }
            AST::ParenthesizedExpression(p) => {
                self.visit_ast(&*p.expression);
            }
            AST::TypeofExpression(t) => {
                self.visit_ast(&*t.expression);
            }
            AST::VoidExpression(v) => {
                self.visit_ast(&*v.expression);
            }
            AST::ImplicitReceiver(_) | AST::ThisReceiver(_) | AST::LiteralPrimitive(_) | AST::EmptyExpr(_) => {
                // Leaf nodes - already recorded
            }
            _ => {
                // Handle other cases as needed
            }
        }
    }
}

/// R3 AST Visitor implementation for ExpressionSourceHumanizer
impl Visitor for ExpressionSourceHumanizer {
    type Result = ();
    
    fn visit_element(&mut self, element: &t::Element) {
        for input in &element.inputs {
            // BoundAttribute.value is ExprAST, not Option
            self.record_ast(&input.value);
            self.visit_ast(&input.value);
        }
        for output in &element.outputs {
            // BoundEvent.handler is ExprAST, not ASTWithSource
            self.record_ast(&output.handler);
            self.visit_ast(&output.handler);
        }
        use t::visit_all;
        let _ = visit_all(self, &element.children);
    }
    
    fn visit_template(&mut self, template: &t::Template) {
        for input in &template.inputs {
            // BoundAttribute.value is ExprAST, not Option
            self.record_ast(&input.value);
            self.visit_ast(&input.value);
        }
        for output in &template.outputs {
            // BoundEvent.handler is ExprAST, not ASTWithSource
            self.record_ast(&output.handler);
            self.visit_ast(&output.handler);
        }
        for attr in &template.template_attrs {
            match attr {
                t::TemplateAttr::Bound(b) => {
                    // BoundAttribute.value is ExprAST, not Option
                    self.record_ast(&b.value);
                    self.visit_ast(&b.value);
                }
                _ => {}
            }
        }
        use t::visit_all;
        let _ = visit_all(self, &template.children);
    }
    
    fn visit_bound_text(&mut self, text: &t::BoundText) {
        // BoundText.value is ExprAST, not ASTWithSource
        self.record_ast(&text.value);
        self.visit_ast(&text.value);
    }
    
    fn visit_bound_attribute(&mut self, attr: &t::BoundAttribute) {
        // BoundAttribute.value is ExprAST, not Option
        self.record_ast(&attr.value);
        self.visit_ast(&attr.value);
    }
    
    fn visit_bound_event(&mut self, event: &t::BoundEvent) {
        // BoundEvent.handler is ExprAST, not ASTWithSource
        self.record_ast(&event.handler);
        self.visit_ast(&event.handler);
    }
    
    fn visit_content(&mut self, content: &t::Content) {
        use t::visit_all;
        let _ = visit_all(self, &content.children);
    }
    
    fn visit_text(&mut self, _text: &t::Text) {}
    fn visit_variable(&mut self, _variable: &t::Variable) {}
    fn visit_reference(&mut self, _reference: &t::Reference) {}
    fn visit_text_attribute(&mut self, _attr: &t::TextAttribute) {}
    fn visit_comment(&mut self, _comment: &t::Comment) {}
    fn visit_icu(&mut self, icu: &t::Icu) {
        for var in icu.vars.values() {
            // BoundText.value is ExprAST
            self.record_ast(&var.value);
            self.visit_ast(&var.value);
        }
        for placeholder in icu.placeholders.values() {
            // IcuPlaceholder is an enum
            match placeholder {
                t::IcuPlaceholder::Text(_) => {
                    // Text placeholders don't have expressions
                }
                t::IcuPlaceholder::BoundText(bt) => {
                    self.record_ast(&bt.value);
                    self.visit_ast(&bt.value);
                }
            }
        }
    }
    fn visit_component(&mut self, component: &t::Component) {
        for input in &component.inputs {
            // BoundAttribute.value is ExprAST, not Option
            self.record_ast(&input.value);
            self.visit_ast(&input.value);
        }
        for output in &component.outputs {
            // BoundEvent.handler is ExprAST, not ASTWithSource
            self.record_ast(&output.handler);
            self.visit_ast(&output.handler);
        }
        use t::visit_all;
        let _ = visit_all(self, &component.children);
    }
    fn visit_directive(&mut self, directive: &t::Directive) {
        for input in &directive.inputs {
            // BoundAttribute.value is ExprAST, not Option
            self.record_ast(&input.value);
            self.visit_ast(&input.value);
        }
        for output in &directive.outputs {
            // BoundEvent.handler is ExprAST, not ASTWithSource
            self.record_ast(&output.handler);
            self.visit_ast(&output.handler);
        }
    }
    fn visit_unknown_block(&mut self, _block: &t::UnknownBlock) {}
    fn visit_deferred_block(&mut self, deferred: &t::DeferredBlock) {
        // Visit triggers
        // Visit triggers using visit_deferred_trigger method
        if let Some(ref trigger) = deferred.triggers.when {
            self.visit_deferred_trigger(&t::DeferredTrigger::Bound(trigger.clone()));
        }
        if let Some(ref trigger) = deferred.triggers.idle {
            self.visit_deferred_trigger(&t::DeferredTrigger::Idle(trigger.clone()));
        }
        if let Some(ref trigger) = deferred.triggers.immediate {
            self.visit_deferred_trigger(&t::DeferredTrigger::Immediate(trigger.clone()));
        }
        if let Some(ref trigger) = deferred.triggers.hover {
            self.visit_deferred_trigger(&t::DeferredTrigger::Hover(trigger.clone()));
        }
        if let Some(ref trigger) = deferred.triggers.timer {
            self.visit_deferred_trigger(&t::DeferredTrigger::Timer(trigger.clone()));
        }
        if let Some(ref trigger) = deferred.triggers.interaction {
            self.visit_deferred_trigger(&t::DeferredTrigger::Interaction(trigger.clone()));
        }
        if let Some(ref trigger) = deferred.triggers.viewport {
            self.visit_deferred_trigger(&t::DeferredTrigger::Viewport(trigger.clone()));
        }
        // Visit prefetch triggers
        if let Some(ref trigger) = deferred.prefetch_triggers.when {
            self.visit_deferred_trigger(&t::DeferredTrigger::Bound(trigger.clone()));
        }
        if let Some(ref trigger) = deferred.prefetch_triggers.immediate {
            self.visit_deferred_trigger(&t::DeferredTrigger::Immediate(trigger.clone()));
        }
        // Visit hydrate triggers
        if let Some(ref trigger) = deferred.hydrate_triggers.interaction {
            self.visit_deferred_trigger(&t::DeferredTrigger::Interaction(trigger.clone()));
        }
        if let Some(ref trigger) = deferred.hydrate_triggers.when {
            self.visit_deferred_trigger(&t::DeferredTrigger::Bound(trigger.clone()));
        }
        if let Some(ref trigger) = deferred.hydrate_triggers.timer {
            self.visit_deferred_trigger(&t::DeferredTrigger::Timer(trigger.clone()));
        }
        use t::visit_all;
        let _ = visit_all(self, &deferred.children);
    }
    
    fn visit_deferred_trigger(&mut self, trigger: &t::DeferredTrigger) {
        match trigger {
            t::DeferredTrigger::Bound(bound) => {
                // BoundDeferredTrigger has value as ExprAST
                self.record_ast(&bound.value);
            }
            t::DeferredTrigger::Viewport(viewport) => {
                // ViewportDeferredTrigger has options as Option<LiteralMap>
                if let Some(ref _options) = viewport.options {
                    // LiteralMap is an AST variant, so we can record it
                    // But we need to convert it to AST first
                    // For now, skip if options is not directly an AST
                }
            }
            _ => {
                // Other triggers don't have expressions to record
            }
        }
    }
    fn visit_deferred_block_placeholder(&mut self, placeholder: &t::DeferredBlockPlaceholder) {
        use t::visit_all;
        let _ = visit_all(self, &placeholder.children);
    }
    fn visit_deferred_block_error(&mut self, error: &t::DeferredBlockError) {
        use t::visit_all;
        let _ = visit_all(self, &error.children);
    }
    fn visit_deferred_block_loading(&mut self, loading: &t::DeferredBlockLoading) {
        use t::visit_all;
        let _ = visit_all(self, &loading.children);
    }
    fn visit_switch_block(&mut self, switch: &t::SwitchBlock) {
        // Visit switch expression and cases
        // SwitchBlock.expression is ExprAST
        self.record_ast(&switch.expression);
        self.visit_ast(&switch.expression);
        for case in &switch.cases {
            use t::Node;
            case.visit(self);
        }
    }
    fn visit_switch_block_case(&mut self, case: &t::SwitchBlockCase) {
        if let Some(ref expr) = case.expression {
            // SwitchBlockCase.expression is Option<ExprAST>, not ASTWithSource
            self.record_ast(expr);
            self.visit_ast(expr);
        }
        use t::visit_all;
        let _ = visit_all(self, &case.children);
    }
    fn visit_for_loop_block(&mut self, for_loop: &t::ForLoopBlock) {
        // Visit loop expression and trackBy
        // ForLoopBlock.expression and track_by are ASTWithSource
        self.visit_ast_with_source(&for_loop.expression);
        self.visit_ast_with_source(&for_loop.track_by);
        use t::visit_all;
        let _ = visit_all(self, &for_loop.children);
    }
    fn visit_for_loop_block_empty(&mut self, empty: &t::ForLoopBlockEmpty) {
        use t::visit_all;
        let _ = visit_all(self, &empty.children);
    }
    fn visit_if_block(&mut self, if_block: &t::IfBlock) {
        for branch in &if_block.branches {
            use t::Node;
            branch.visit(self);
        }
    }
    fn visit_if_block_branch(&mut self, branch: &t::IfBlockBranch) {
        if let Some(ref expr) = branch.expression {
            // IfBlockBranch.expression is Option<ExprAST>, not ASTWithSource
            self.record_ast(expr);
            self.visit_ast(expr);
        }
        if let Some(ref _expr_alias) = branch.expression_alias {
            // expression_alias is Variable, doesn't have expressions
        }
        use t::visit_all;
        let _ = visit_all(self, &branch.children);
    }
    fn visit_let_declaration(&mut self, decl: &t::LetDeclaration) {
        // LetDeclaration.value is ExprAST, not ASTWithSource
        self.record_ast(&decl.value);
        self.visit_ast(&decl.value);
    }
}

impl ExpressionSourceHumanizer {
    fn visit_ast_with_source(&mut self, ast_with_source: &ASTWithSource) {
        self.record_ast(&*ast_with_source.ast);
        self.visit_ast(&*ast_with_source.ast);
    }
}

/// Humanizes expression AST source spans in a template
pub fn humanize_expression_source(template_asts: &[t::R3Node]) -> Vec<HumanizedExpressionSource> {
    let mut humanizer = ExpressionSourceHumanizer::new();
    use t::visit_all;
    let _ = visit_all(&mut humanizer, template_asts);
    humanizer.result
}

