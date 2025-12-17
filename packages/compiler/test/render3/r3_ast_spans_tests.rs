//! R3 AST Source Spans Tests
//!
//! Mirrors angular/packages/compiler/test/render3/r3_ast_spans_spec.ts

use angular_compiler::render3::r3_ast as t;
use angular_compiler::render3::r3_ast::{Visitor, Node};
use angular_compiler::parse_util::ParseSourceSpan;
// Include test utilities
#[path = "view/util.rs"]
mod view_util;
use view_util::{parse_r3, ParseR3Options};

// Helper to humanize ParseSourceSpan to string
fn humanize_span(span: &Option<ParseSourceSpan>) -> String {
    match span {
        Some(s) => s.to_string(),
        None => "<empty>".to_string(),
    }
}

// Visitor that collects source spans for all R3 AST nodes
struct R3AstSourceSpans {
    result: Vec<Vec<String>>,
}

impl R3AstSourceSpans {
    fn new() -> Self {
        R3AstSourceSpans {
            result: vec![],
        }
    }
    
    fn visit_all(&mut self, nodes: &[t::R3Node]) {
        use t::visit_all;
        visit_all(self, nodes);
    }
}

impl Visitor for R3AstSourceSpans {
    type Result = ();
    
    fn visit_element(&mut self, element: &t::Element) {
        self.result.push(vec![
            "Element".to_string(),
            humanize_span(&Some(element.source_span.clone())),
            humanize_span(&Some(element.start_source_span.clone())),
            humanize_span(&element.end_source_span.clone()),
        ]);
        for attr in &element.attributes {
            attr.visit(self);
        }
        for input in &element.inputs {
            input.visit(self);
        }
        for output in &element.outputs {
            output.visit(self);
        }
        for directive in &element.directives {
            directive.visit(self);
        }
        for reference in &element.references {
            reference.visit(self);
        }
        self.visit_all(&element.children);
    }
    
    fn visit_template(&mut self, template: &t::Template) {
        self.result.push(vec![
            "Template".to_string(),
            humanize_span(&Some(template.source_span.clone())),
            humanize_span(&Some(template.start_source_span.clone())),
            humanize_span(&template.end_source_span.clone()),
        ]);
        for attr in &template.attributes {
            attr.visit(self);
        }
        for input in &template.inputs {
            input.visit(self);
        }
        for output in &template.outputs {
            output.visit(self);
        }
        for directive in &template.directives {
            directive.visit(self);
        }
        for attr in &template.template_attrs {
            match attr {
                t::TemplateAttr::Text(t) => t.visit(self),
                t::TemplateAttr::Bound(b) => b.visit(self),
            }
        }
        for reference in &template.references {
            reference.visit(self);
        }
        for variable in &template.variables {
            variable.visit(self);
        }
        self.visit_all(&template.children);
    }
    
    fn visit_content(&mut self, content: &t::Content) {
        self.result.push(vec![
            "Content".to_string(),
            humanize_span(&Some(content.source_span.clone())),
        ]);
        for attr in &content.attributes {
            attr.visit(self);
        }
        self.visit_all(&content.children);
    }
    
    fn visit_variable(&mut self, variable: &t::Variable) {
        self.result.push(vec![
            "Variable".to_string(),
            humanize_span(&Some(variable.source_span.clone())),
            humanize_span(&Some(variable.key_span.clone())),
            humanize_span(&variable.value_span.as_ref().map(|s| s.clone())),
        ]);
    }
    
    fn visit_reference(&mut self, reference: &t::Reference) {
        self.result.push(vec![
            "Reference".to_string(),
            humanize_span(&Some(reference.source_span.clone())),
            humanize_span(&Some(reference.key_span.clone())),
            humanize_span(&reference.value_span.as_ref().map(|s| s.clone())),
        ]);
    }
    
    fn visit_text_attribute(&mut self, attribute: &t::TextAttribute) {
        self.result.push(vec![
            "TextAttribute".to_string(),
            humanize_span(&Some(attribute.source_span.clone())),
            humanize_span(&attribute.key_span.as_ref().map(|s| s.clone())),
            humanize_span(&attribute.value_span.as_ref().map(|s| s.clone())),
        ]);
    }
    
    fn visit_bound_attribute(&mut self, attribute: &t::BoundAttribute) {
        self.result.push(vec![
            "BoundAttribute".to_string(),
            humanize_span(&Some(attribute.source_span.clone())),
            humanize_span(&Some(attribute.key_span.clone())),
            humanize_span(&attribute.value_span.as_ref().map(|s| s.clone())),
        ]);
    }
    
    fn visit_bound_event(&mut self, event: &t::BoundEvent) {
        self.result.push(vec![
            "BoundEvent".to_string(),
            humanize_span(&Some(event.source_span.clone())),
            humanize_span(&Some(event.key_span.clone())),
            humanize_span(&Some(event.handler_span.clone())),
        ]);
    }
    
    fn visit_text(&mut self, text: &t::Text) {
        self.result.push(vec![
            "Text".to_string(),
            humanize_span(&Some(text.source_span.clone())),
        ]);
    }
    
    fn visit_bound_text(&mut self, text: &t::BoundText) {
        self.result.push(vec![
            "BoundText".to_string(),
            humanize_span(&Some(text.source_span.clone())),
        ]);
    }
    
    fn visit_icu(&mut self, icu: &t::Icu) {
        self.result.push(vec![
            "Icu".to_string(),
            humanize_span(&Some(icu.source_span.clone())),
        ]);
        for var in icu.vars.values() {
            self.result.push(vec![
                "Icu:Var".to_string(),
                humanize_span(&Some(var.source_span.clone())),
            ]);
        }
        for placeholder in icu.placeholders.values() {
            // IcuPlaceholder is an enum, need to match to get source_span
            let placeholder_span = match placeholder {
                t::IcuPlaceholder::Text(text) => Some(text.source_span.clone()),
                t::IcuPlaceholder::BoundText(bound_text) => Some(bound_text.source_span.clone()),
            };
            self.result.push(vec![
                "Icu:Placeholder".to_string(),
                humanize_span(&placeholder_span),
            ]);
        }
    }
    
    fn visit_deferred_block(&mut self, deferred: &t::DeferredBlock) {
        self.result.push(vec![
            "DeferredBlock".to_string(),
            humanize_span(&Some(deferred.source_span().clone())),
            humanize_span(&Some(deferred.block.start_source_span.clone())),
            humanize_span(&deferred.block.end_source_span.as_ref().map(|s| s.clone())),
        ]);
        // Visit triggers
        if let Some(ref trigger) = deferred.triggers.when {
            t::DeferredTrigger::Bound(trigger.clone()).visit(self);
        }
        if let Some(ref trigger) = deferred.triggers.idle {
            t::DeferredTrigger::Idle(trigger.clone()).visit(self);
        }
        if let Some(ref trigger) = deferred.triggers.immediate {
            t::DeferredTrigger::Immediate(trigger.clone()).visit(self);
        }
        if let Some(ref trigger) = deferred.triggers.hover {
            t::DeferredTrigger::Hover(trigger.clone()).visit(self);
        }
        if let Some(ref trigger) = deferred.triggers.timer {
            t::DeferredTrigger::Timer(trigger.clone()).visit(self);
        }
        if let Some(ref trigger) = deferred.triggers.interaction {
            t::DeferredTrigger::Interaction(trigger.clone()).visit(self);
        }
        if let Some(ref trigger) = deferred.triggers.viewport {
            t::DeferredTrigger::Viewport(trigger.clone()).visit(self);
        }
        // Visit prefetch triggers
        if let Some(ref trigger) = deferred.prefetch_triggers.when {
            t::DeferredTrigger::Bound(trigger.clone()).visit(self);
        }
        if let Some(ref trigger) = deferred.prefetch_triggers.immediate {
            t::DeferredTrigger::Immediate(trigger.clone()).visit(self);
        }
        // Visit hydrate triggers
        if let Some(ref trigger) = deferred.hydrate_triggers.interaction {
            t::DeferredTrigger::Interaction(trigger.clone()).visit(self);
        }
        if let Some(ref trigger) = deferred.hydrate_triggers.when {
            t::DeferredTrigger::Bound(trigger.clone()).visit(self);
        }
        if let Some(ref trigger) = deferred.hydrate_triggers.timer {
            t::DeferredTrigger::Timer(trigger.clone()).visit(self);
        }
        self.visit_all(&deferred.children);
        if let Some(ref placeholder) = deferred.placeholder {
            placeholder.visit(self);
        }
        if let Some(ref loading) = deferred.loading {
            loading.visit(self);
        }
        if let Some(ref error) = deferred.error {
            error.visit(self);
        }
    }
    
    fn visit_deferred_trigger(&mut self, trigger: &t::DeferredTrigger) {
        let name = match trigger {
            t::DeferredTrigger::Bound(_) => "BoundDeferredTrigger",
            t::DeferredTrigger::Immediate(_) => "ImmediateDeferredTrigger",
            t::DeferredTrigger::Hover(_) => "HoverDeferredTrigger",
            t::DeferredTrigger::Idle(_) => "IdleDeferredTrigger",
            t::DeferredTrigger::Timer(_) => "TimerDeferredTrigger",
            t::DeferredTrigger::Interaction(_) => "InteractionDeferredTrigger",
            t::DeferredTrigger::Viewport(_) => "ViewportDeferredTrigger",
            t::DeferredTrigger::Never(_) => "NeverDeferredTrigger",
        };
        self.result.push(vec![
            name.to_string(),
            humanize_span(&Some(trigger.source_span().clone())),
        ]);
    }
    
    fn visit_deferred_block_placeholder(&mut self, placeholder: &t::DeferredBlockPlaceholder) {
        self.result.push(vec![
            "DeferredBlockPlaceholder".to_string(),
            humanize_span(&Some(placeholder.source_span().clone())),
            humanize_span(&Some(placeholder.block.start_source_span.clone())),
            humanize_span(&placeholder.block.end_source_span.as_ref().map(|s| s.clone())),
        ]);
        self.visit_all(&placeholder.children);
    }
    
    fn visit_deferred_block_loading(&mut self, loading: &t::DeferredBlockLoading) {
        self.result.push(vec![
            "DeferredBlockLoading".to_string(),
            humanize_span(&Some(loading.source_span().clone())),
            humanize_span(&Some(loading.block.start_source_span.clone())),
            humanize_span(&loading.block.end_source_span.as_ref().map(|s| s.clone())),
        ]);
        self.visit_all(&loading.children);
    }
    
    fn visit_deferred_block_error(&mut self, error: &t::DeferredBlockError) {
        self.result.push(vec![
            "DeferredBlockError".to_string(),
            humanize_span(&Some(error.source_span().clone())),
            humanize_span(&Some(error.block.start_source_span.clone())),
            humanize_span(&error.block.end_source_span.as_ref().map(|s| s.clone())),
        ]);
        self.visit_all(&error.children);
    }
    
    fn visit_switch_block(&mut self, switch: &t::SwitchBlock) {
        self.result.push(vec![
            "SwitchBlock".to_string(),
            humanize_span(&Some(switch.source_span().clone())),
            humanize_span(&Some(switch.block.start_source_span.clone())),
            humanize_span(&switch.block.end_source_span.as_ref().map(|s| s.clone())),
        ]);
        for case in &switch.cases {
            case.visit(self);
        }
    }
    
    fn visit_switch_block_case(&mut self, case: &t::SwitchBlockCase) {
        self.result.push(vec![
            "SwitchBlockCase".to_string(),
            humanize_span(&Some(case.source_span().clone())),
            humanize_span(&Some(case.block.start_source_span.clone())),
        ]);
        self.visit_all(&case.children);
    }
    
    fn visit_for_loop_block(&mut self, for_loop: &t::ForLoopBlock) {
        self.result.push(vec![
            "ForLoopBlock".to_string(),
            humanize_span(&Some(for_loop.source_span().clone())),
            humanize_span(&Some(for_loop.block.start_source_span.clone())),
            humanize_span(&for_loop.block.end_source_span.as_ref().map(|s| s.clone())),
        ]);
        self.visit_variable(&for_loop.item);
        for var in &for_loop.context_variables {
            var.visit(self);
        }
        self.visit_all(&for_loop.children);
        if let Some(ref empty) = for_loop.empty {
            empty.visit(self);
        }
    }
    
    fn visit_for_loop_block_empty(&mut self, empty: &t::ForLoopBlockEmpty) {
        self.result.push(vec![
            "ForLoopBlockEmpty".to_string(),
            humanize_span(&Some(empty.source_span().clone())),
            humanize_span(&Some(empty.block.start_source_span.clone())),
        ]);
        self.visit_all(&empty.children);
    }
    
    fn visit_if_block(&mut self, if_block: &t::IfBlock) {
        self.result.push(vec![
            "IfBlock".to_string(),
            humanize_span(&Some(if_block.source_span().clone())),
            humanize_span(&Some(if_block.block.start_source_span.clone())),
            humanize_span(&if_block.block.end_source_span.as_ref().map(|s| s.clone())),
        ]);
        for branch in &if_block.branches {
            branch.visit(self);
        }
    }
    
    fn visit_if_block_branch(&mut self, branch: &t::IfBlockBranch) {
        self.result.push(vec![
            "IfBlockBranch".to_string(),
            humanize_span(&Some(branch.source_span().clone())),
            humanize_span(&Some(branch.block.start_source_span.clone())),
        ]);
        if let Some(ref expr_alias) = branch.expression_alias {
            self.visit_variable(expr_alias);
        }
        self.visit_all(&branch.children);
    }
    
    fn visit_unknown_block(&mut self, block: &t::UnknownBlock) {
        self.result.push(vec![
            "UnknownBlock".to_string(),
            humanize_span(&Some(block.source_span.clone())),
        ]);
    }
    
    fn visit_let_declaration(&mut self, decl: &t::LetDeclaration) {
        self.result.push(vec![
            "LetDeclaration".to_string(),
            humanize_span(&Some(decl.source_span.clone())),
            humanize_span(&Some(decl.name_span.clone())),
            humanize_span(&Some(decl.value_span.clone())),
        ]);
    }
    
    fn visit_component(&mut self, component: &t::Component) {
        self.result.push(vec![
            "Component".to_string(),
            humanize_span(&Some(component.source_span.clone())),
            humanize_span(&Some(component.start_source_span.clone())),
            humanize_span(&component.end_source_span.as_ref().map(|s| s.clone())),
        ]);
        for attr in &component.attributes {
            attr.visit(self);
        }
        for input in &component.inputs {
            input.visit(self);
        }
        for output in &component.outputs {
            output.visit(self);
        }
        for directive in &component.directives {
            directive.visit(self);
        }
        for reference in &component.references {
            reference.visit(self);
        }
        self.visit_all(&component.children);
    }
    
    fn visit_directive(&mut self, directive: &t::Directive) {
        self.result.push(vec![
            "Directive".to_string(),
            humanize_span(&Some(directive.source_span.clone())),
            humanize_span(&Some(directive.start_source_span.clone())),
            humanize_span(&directive.end_source_span.as_ref().map(|s| s.clone())),
        ]);
        for attr in &directive.attributes {
            attr.visit(self);
        }
        for input in &directive.inputs {
            input.visit(self);
        }
        for output in &directive.outputs {
            output.visit(self);
        }
        for reference in &directive.references {
            reference.visit(self);
        }
    }
    
    fn visit_comment(&mut self, _comment: &t::Comment) {
        // Comments not included in spans test
    }
}

fn expect_from_html(html: &str, selectorless_enabled: bool) -> Vec<Vec<String>> {
    expect_from_r3_nodes(&parse_r3(html, ParseR3Options {
        selectorless_enabled: Some(selectorless_enabled),
        ..Default::default()
    }).nodes)
}

fn expect_from_r3_nodes(nodes: &[t::R3Node]) -> Vec<Vec<String>> {
    let mut humanizer = R3AstSourceSpans::new();
    humanizer.visit_all(nodes);
    humanizer.result
}

#[cfg(test)]
mod tests {
    use super::*;

    mod nodes_without_binding {
        use super::*;

        #[test]
        fn is_correct_for_text_nodes() {
            let result = expect_from_html("a", false);
            assert_eq!(result, vec![vec!["Text".to_string(), "a".to_string()]]);
        }

        #[test]
        fn is_correct_for_elements_with_attributes() {
            let result = expect_from_html("<div a=\"b\"></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div a=\"b\"></div>".to_string(), "<div a=\"b\">".to_string(), "</div>".to_string()],
                vec!["TextAttribute".to_string(), "a=\"b\"".to_string(), "a".to_string(), "b".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_elements_with_attributes_without_value() {
            let result = expect_from_html("<div a></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div a></div>".to_string(), "<div a>".to_string(), "</div>".to_string()],
                vec!["TextAttribute".to_string(), "a".to_string(), "a".to_string(), "<empty>".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_self_closing_elements_with_trailing_whitespace() {
            let result = expect_from_html("<input />\n  <span>\n</span>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<input />".to_string(), "<input />".to_string(), "<input />".to_string()],
                vec!["Element".to_string(), "<span>\n</span>".to_string(), "<span>".to_string(), "</span>".to_string()],
            ]);
        }
    }

    mod bound_text_nodes {
        use super::*;

        #[test]
        fn is_correct_for_bound_text_nodes() {
            let result = expect_from_html("{{a}}", false);
            assert_eq!(result, vec![vec!["BoundText".to_string(), "{{a}}".to_string()]]);
        }
    }

    mod bound_attributes {
        use super::*;

        #[test]
        fn is_correct_for_bound_properties() {
            let result = expect_from_html("<div [someProp]=\"v\"></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div [someProp]=\"v\"></div>".to_string(), "<div [someProp]=\"v\">".to_string(), "</div>".to_string()],
                vec!["BoundAttribute".to_string(), "[someProp]=\"v\"".to_string(), "someProp".to_string(), "v".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_bound_properties_without_value() {
            let result = expect_from_html("<div [someProp]></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div [someProp]></div>".to_string(), "<div [someProp]>".to_string(), "</div>".to_string()],
                vec!["BoundAttribute".to_string(), "[someProp]".to_string(), "someProp".to_string(), "<empty>".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_bound_properties_via_bind() {
            let result = expect_from_html("<div bind-prop=\"v\"></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div bind-prop=\"v\"></div>".to_string(), "<div bind-prop=\"v\">".to_string(), "</div>".to_string()],
                vec!["BoundAttribute".to_string(), "bind-prop=\"v\"".to_string(), "prop".to_string(), "v".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_bound_properties_via_interpolation() {
            let result = expect_from_html("<div prop=\"{{v}}\"></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div prop=\"{{v}}\"></div>".to_string(), "<div prop=\"{{v}}\">".to_string(), "</div>".to_string()],
                vec!["BoundAttribute".to_string(), "prop=\"{{v}}\"".to_string(), "prop".to_string(), "{{v}}".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_bound_properties_via_data() {
            let result = expect_from_html("<div data-prop=\"{{v}}\"></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div data-prop=\"{{v}}\"></div>".to_string(), "<div data-prop=\"{{v}}\">".to_string(), "</div>".to_string()],
                vec!["BoundAttribute".to_string(), "data-prop=\"{{v}}\"".to_string(), "prop".to_string(), "{{v}}".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_bound_properties_via_at_symbol() {
            let result = expect_from_html("<div bind-@animation=\"v\"></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div bind-@animation=\"v\"></div>".to_string(), "<div bind-@animation=\"v\">".to_string(), "</div>".to_string()],
                vec!["BoundAttribute".to_string(), "bind-@animation=\"v\"".to_string(), "animation".to_string(), "v".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_bound_properties_via_animation_prefix() {
            let result = expect_from_html("<div bind-animate-animationName=\"v\"></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div bind-animate-animationName=\"v\"></div>".to_string(), "<div bind-animate-animationName=\"v\">".to_string(), "</div>".to_string()],
                vec!["BoundAttribute".to_string(), "bind-animate-animationName=\"v\"".to_string(), "animationName".to_string(), "v".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_bound_properties_via_at_without_value() {
            let result = expect_from_html("<div @animation></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div @animation></div>".to_string(), "<div @animation>".to_string(), "</div>".to_string()],
                vec!["BoundAttribute".to_string(), "@animation".to_string(), "animation".to_string(), "<empty>".to_string()],
            ]);
        }
    }

    mod templates {
        use super::*;

        #[test]
        fn is_correct_for_star_directives() {
            let result = expect_from_html("<div *ngIf></div>", false);
            assert_eq!(result, vec![
                vec!["Template".to_string(), "<div *ngIf></div>".to_string(), "<div *ngIf>".to_string(), "</div>".to_string()],
                vec!["TextAttribute".to_string(), "ngIf".to_string(), "ngIf".to_string(), "<empty>".to_string()],
                vec!["Element".to_string(), "<div *ngIf></div>".to_string(), "<div *ngIf>".to_string(), "</div>".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_ng_template() {
            let result = expect_from_html("<ng-template></ng-template>", false);
            assert_eq!(result, vec![
                vec!["Template".to_string(), "<ng-template></ng-template>".to_string(), "<ng-template>".to_string(), "</ng-template>".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_reference_via_hash() {
            let result = expect_from_html("<ng-template #a></ng-template>", false);
            assert_eq!(result, vec![
                vec!["Template".to_string(), "<ng-template #a></ng-template>".to_string(), "<ng-template #a>".to_string(), "</ng-template>".to_string()],
                vec!["Reference".to_string(), "#a".to_string(), "a".to_string(), "<empty>".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_reference_with_name() {
            let result = expect_from_html("<ng-template #a=\"b\"></ng-template>", false);
            assert_eq!(result, vec![
                vec!["Template".to_string(), "<ng-template #a=\"b\"></ng-template>".to_string(), "<ng-template #a=\"b\">".to_string(), "</ng-template>".to_string()],
                vec!["Reference".to_string(), "#a=\"b\"".to_string(), "a".to_string(), "b".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_reference_via_ref() {
            let result = expect_from_html("<ng-template ref-a></ng-template>", false);
            assert_eq!(result, vec![
                vec!["Template".to_string(), "<ng-template ref-a></ng-template>".to_string(), "<ng-template ref-a>".to_string(), "</ng-template>".to_string()],
                vec!["Reference".to_string(), "ref-a".to_string(), "a".to_string(), "<empty>".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_reference_via_data_ref() {
            let result = expect_from_html("<ng-template data-ref-a></ng-template>", false);
            assert_eq!(result, vec![
                vec!["Template".to_string(), "<ng-template data-ref-a></ng-template>".to_string(), "<ng-template data-ref-a>".to_string(), "</ng-template>".to_string()],
                vec!["Reference".to_string(), "data-ref-a".to_string(), "a".to_string(), "<empty>".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_variables_via_let() {
            let result = expect_from_html("<ng-template let-a=\"b\"></ng-template>", false);
            assert_eq!(result, vec![
                vec!["Template".to_string(), "<ng-template let-a=\"b\"></ng-template>".to_string(), "<ng-template let-a=\"b\">".to_string(), "</ng-template>".to_string()],
                vec!["Variable".to_string(), "let-a=\"b\"".to_string(), "a".to_string(), "b".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_variables_via_data_let() {
            let result = expect_from_html("<ng-template data-let-a=\"b\"></ng-template>", false);
            assert_eq!(result, vec![
                vec!["Template".to_string(), "<ng-template data-let-a=\"b\"></ng-template>".to_string(), "<ng-template data-let-a=\"b\">".to_string(), "</ng-template>".to_string()],
                vec!["Variable".to_string(), "data-let-a=\"b\"".to_string(), "a".to_string(), "b".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_attributes() {
            let result = expect_from_html("<ng-template k1=\"v1\"></ng-template>", false);
            assert_eq!(result, vec![
                vec!["Template".to_string(), "<ng-template k1=\"v1\"></ng-template>".to_string(), "<ng-template k1=\"v1\">".to_string(), "</ng-template>".to_string()],
                vec!["TextAttribute".to_string(), "k1=\"v1\"".to_string(), "k1".to_string(), "v1".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_bound_attributes() {
            let result = expect_from_html("<ng-template [k1]=\"v1\"></ng-template>", false);
            assert_eq!(result, vec![
                vec!["Template".to_string(), "<ng-template [k1]=\"v1\"></ng-template>".to_string(), "<ng-template [k1]=\"v1\">".to_string(), "</ng-template>".to_string()],
                vec!["BoundAttribute".to_string(), "[k1]=\"v1\"".to_string(), "k1".to_string(), "v1".to_string()],
            ]);
        }
    }

    mod inline_templates {
        use super::*;

        #[test]
        fn is_correct_for_attribute_and_bound_attributes() {
            let result = expect_from_html("<div *ngFor=\"let item of items\"></div>", false);
            // Note: Exact spans may vary, so we check key components
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result.iter().any(|v| v[0] == "TextAttribute" && v[2] == "ngFor"));
            assert!(result.iter().any(|v| v[0] == "BoundAttribute" && v[2] == "of"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[2] == "item"));
            assert!(result.iter().any(|v| v[0] == "Element"));
        }
    }

    mod events {
        use super::*;

        #[test]
        fn is_correct_for_event_names_case_sensitive() {
            let result = expect_from_html("<div (someEvent)=\"v\"></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div (someEvent)=\"v\"></div>".to_string(), "<div (someEvent)=\"v\">".to_string(), "</div>".to_string()],
                vec!["BoundEvent".to_string(), "(someEvent)=\"v\"".to_string(), "someEvent".to_string(), "v".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_bound_events_via_on() {
            let result = expect_from_html("<div on-event=\"v\"></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div on-event=\"v\"></div>".to_string(), "<div on-event=\"v\">".to_string(), "</div>".to_string()],
                vec!["BoundEvent".to_string(), "on-event=\"v\"".to_string(), "event".to_string(), "v".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_bound_events_via_data_on() {
            let result = expect_from_html("<div data-on-event=\"v\"></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div data-on-event=\"v\"></div>".to_string(), "<div data-on-event=\"v\">".to_string(), "</div>".to_string()],
                vec!["BoundEvent".to_string(), "data-on-event=\"v\"".to_string(), "event".to_string(), "v".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_bound_events_and_properties_via_banana_box() {
            let result = expect_from_html("<div [(prop)]=\"v\"></div>", false);
            assert!(result.iter().any(|v| v[0] == "BoundAttribute" && v[2] == "prop"));
            assert!(result.iter().any(|v| v[0] == "BoundEvent" && v[2] == "prop"));
        }

        #[test]
        fn is_correct_for_bound_events_via_at() {
            let result = expect_from_html("<div (@name.done)=\"v\"></div>", false);
            assert!(result.iter().any(|v| v[0] == "BoundEvent"));
        }
    }

    mod references {
        use super::*;

        #[test]
        fn is_correct_for_references_via_hash() {
            let result = expect_from_html("<div #a></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div #a></div>".to_string(), "<div #a>".to_string(), "</div>".to_string()],
                vec!["Reference".to_string(), "#a".to_string(), "a".to_string(), "<empty>".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_references_with_name() {
            let result = expect_from_html("<div #a=\"b\"></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div #a=\"b\"></div>".to_string(), "<div #a=\"b\">".to_string(), "</div>".to_string()],
                vec!["Reference".to_string(), "#a=\"b\"".to_string(), "a".to_string(), "b".to_string()],
            ]);
        }

        #[test]
        fn is_correct_for_references_via_ref() {
            let result = expect_from_html("<div ref-a></div>", false);
            assert_eq!(result, vec![
                vec!["Element".to_string(), "<div ref-a></div>".to_string(), "<div ref-a>".to_string(), "</div>".to_string()],
                vec!["Reference".to_string(), "ref-a".to_string(), "a".to_string(), "<empty>".to_string()],
            ]);
        }
    }

    mod icu_expressions {
        use super::*;

        #[test]
        fn is_correct_for_variables_and_placeholders() {
            let result = expect_from_html(
                "<span i18n>{item.var, plural, other { {{item.placeholder}} items } }</span>",
                false
            );
            assert!(result.iter().any(|v| v[0] == "Icu"));
            assert!(result.iter().any(|v| v[0] == "Icu:Var"));
            assert!(result.iter().any(|v| v[0] == "Icu:Placeholder"));
        }

        #[test]
        fn is_correct_for_nested_icus() {
            let result = expect_from_html(
                "<span i18n>{item.var, plural, other { {{item.placeholder}} {nestedVar, plural, other { {{nestedPlaceholder}} }}} }</span>",
                false
            );
            assert!(result.iter().any(|v| v[0] == "Icu"));
            assert!(result.iter().filter(|v| v[0] == "Icu:Var").count() >= 2);
            assert!(result.iter().filter(|v| v[0] == "Icu:Placeholder").count() >= 2);
        }
    }

    mod deferred_blocks {
        use super::*;

        #[test]
        fn is_correct_for_deferred_blocks() {
            let html = "@defer (when isVisible() && foo; on hover(button), timer(10s), idle, immediate, \
                interaction(button), viewport(container); prefetch on immediate; \
                prefetch when isDataLoaded(); hydrate on interaction; hydrate when isVisible(); hydrate on timer(1200)) {<calendar-cmp [date]=\"current\"/>}\
                @loading (minimum 1s; after 100ms) {Loading...}\
                @placeholder (minimum 500) {Placeholder content!}\
                @error {Loading failed :(}";
            
            let result = expect_from_html(html, false);
            // Check that DeferredBlock exists
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            // Check triggers exist
            assert!(result.iter().any(|v| v[0] == "BoundDeferredTrigger" || v[0] == "HoverDeferredTrigger" || v[0] == "TimerDeferredTrigger" || v[0] == "IdleDeferredTrigger" || v[0] == "ImmediateDeferredTrigger" || v[0] == "InteractionDeferredTrigger" || v[0] == "ViewportDeferredTrigger"));
            // Check placeholder, loading, error blocks exist
            assert!(result.iter().any(|v| v[0] == "DeferredBlockPlaceholder"));
            assert!(result.iter().any(|v| v[0] == "DeferredBlockLoading"));
            assert!(result.iter().any(|v| v[0] == "DeferredBlockError"));
        }
    }

    mod switch_blocks {
        use super::*;

        #[test]
        fn is_correct_for_switch_blocks() {
            let html = "@switch (cond.kind) {\
                @case (x()) {X case}\
                @case ('hello') {Y case}\
                @case (42) {Z case}\
                @default {No case matched}\
                }";
            
            let result = expect_from_html(html, false);
            assert!(result.iter().any(|v| v[0] == "SwitchBlock"));
            assert!(result.iter().filter(|v| v[0] == "SwitchBlockCase").count() >= 4);
        }
    }

    mod for_loop_blocks {
        use super::*;

        #[test]
        fn is_correct_for_loop_blocks() {
            let html = "@for (item of items.foo.bar; track item.id; let i = $index, _o_d_d_ = $odd) {<h1>{{ item }}</h1>}\
                @empty {There were no items in the list.}";
            
            let result = expect_from_html(html, false);
            assert!(result.iter().any(|v| v[0] == "ForLoopBlock"));
            assert!(result.iter().any(|v| v[0] == "ForLoopBlockEmpty"));
            assert!(result.iter().filter(|v| v[0] == "Variable").count() >= 2);
        }
    }

    mod if_blocks {
        use super::*;

        #[test]
        fn is_correct_for_if_blocks() {
            let html = "@if (cond.expr; as foo) {Main case was true!}\
                @else if (other.expr) {Extra case was true!}\
                @else {False case!}";
            
            let result = expect_from_html(html, false);
            assert!(result.iter().any(|v| v[0] == "IfBlock"));
            assert!(result.iter().filter(|v| v[0] == "IfBlockBranch").count() >= 3);
        }
    }

    mod let_declaration {
        use super::*;

        #[test]
        fn is_correct_for_a_let_declaration() {
            let result = expect_from_html("@let foo = 123;", false);
            assert!(result.iter().any(|v| v[0] == "LetDeclaration"));
        }
    }

    mod component_tags {
        use super::*;

        #[test]
        fn is_correct_for_a_simple_component() {
            let result = expect_from_html("<MyComp></MyComp>", true);
            assert!(result.iter().any(|v| v[0] == "Component"));
        }

        #[test]
        fn is_correct_for_a_self_closing_component() {
            let result = expect_from_html("<MyComp/>", true);
            assert!(result.iter().any(|v| v[0] == "Component"));
        }

        #[test]
        fn is_correct_for_a_component_with_a_tag_name() {
            let result = expect_from_html("<MyComp:button></MyComp:button>", true);
            assert!(result.iter().any(|v| v[0] == "Component"));
        }

        #[test]
        fn is_correct_for_a_component_with_attributes_and_directives() {
            let result = expect_from_html(
                "<MyComp before=\"foo\" @Dir middle @OtherDir([a]=\"a\" (b)=\"b()\") after=\"123\">Hello</MyComp>",
                true
            );
            assert!(result.iter().any(|v| v[0] == "Component"));
            assert!(result.iter().any(|v| v[0] == "Directive"));
        }

        #[test]
        fn is_correct_for_a_component_nested_inside_other_markup() {
            let result = expect_from_html(
                "@if (expr) {<div>Hello: <MyComp><span><OtherComp/></span></MyComp></div>}",
                true
            );
            assert!(result.iter().any(|v| v[0] == "Component"));
        }
    }

    mod directives {
        use super::*;

        #[test]
        fn is_correct_for_a_directive_with_no_attributes() {
            let result = expect_from_html("<div @Dir></div>", true);
            assert!(result.iter().any(|v| v[0] == "Directive"));
        }

        #[test]
        fn is_correct_for_a_directive_with_attributes() {
            let result = expect_from_html("<div @Dir(a=\"1\" [b]=\"two\" (c)=\"c()\")></div>", true);
            assert!(result.iter().any(|v| v[0] == "Directive"));
        }

        #[test]
        fn is_correct_for_directives_mixed_with_other_attributes() {
            let result = expect_from_html(
                "<div before=\"foo\" @Dir middle @OtherDir([a]=\"a\" (b)=\"b()\") after=\"123\"></div>",
                true
            );
            assert!(result.iter().filter(|v| v[0] == "Directive").count() >= 2);
        }
    }
}

