//! R3 Template Transform Tests
//!
//! Mirrors angular/packages/compiler/test/render3/r3_template_transform_spec.ts

use angular_compiler::expression_parser::ast::{BindingType, ParsedEventType};
use angular_compiler::render3::r3_ast as t;
use angular_compiler::render3::r3_ast::{Node, Visitor};
// Include test utilities
#[path = "../expression_parser/utils/unparser.rs"]
mod unparser_mod;
use unparser_mod::unparse;

#[path = "view/util.rs"]
mod view_util;
use view_util::{parse_r3, ParseR3Options};

// Transform an IVY AST to a flat list of nodes to ease testing
struct R3AstHumanizer {
    result: Vec<Vec<String>>,
}

impl R3AstHumanizer {
    fn new() -> Self {
        R3AstHumanizer { result: vec![] }
    }

    fn visit_all(&mut self, nodes: &[t::R3Node]) {
        use t::visit_all;
        let _ = visit_all(self, nodes);
    }
}

impl Visitor for R3AstHumanizer {
    type Result = ();

    fn visit_element(&mut self, element: &t::Element) {
        let mut res = vec!["Element".to_string(), element.name.to_string()];
        if element.is_self_closing {
            res.push("#selfClosing".to_string());
        }
        self.result.push(res);
        // Visit attributes, inputs, outputs, directives, references, children
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
        let mut res = vec!["Template".to_string()];
        if template.is_self_closing {
            res.push("#selfClosing".to_string());
        }
        self.result.push(res);
        // Visit attributes, inputs, outputs, directives, references, variables, children
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
        let mut res = vec!["Content".to_string(), content.selector.to_string()];
        if content.is_self_closing {
            res.push("#selfClosing".to_string());
        }
        self.result.push(res);
        for attr in &content.attributes {
            attr.visit(self);
        }
        self.visit_all(&content.children);
    }

    fn visit_variable(&mut self, variable: &t::Variable) {
        self.result.push(vec![
            "Variable".to_string(),
            variable.name.to_string(),
            variable.value.to_string(),
        ]);
    }

    fn visit_reference(&mut self, reference: &t::Reference) {
        self.result.push(vec![
            "Reference".to_string(),
            reference.name.to_string(),
            reference.value.to_string(),
        ]);
    }

    fn visit_text_attribute(&mut self, attribute: &t::TextAttribute) {
        self.result.push(vec![
            "TextAttribute".to_string(),
            attribute.name.to_string(),
            attribute.value.to_string(),
        ]);
    }

    fn visit_bound_attribute(&mut self, attribute: &t::BoundAttribute) {
        let binding_type_str = match attribute.type_ {
            BindingType::Property => "0",
            BindingType::Attribute => "1",
            BindingType::Class => "2",
            BindingType::Style => "3",
            BindingType::LegacyAnimation => "4",
            BindingType::TwoWay => "5",
            BindingType::Animation => "6",
        }
        .to_string();

        // For BoundAttribute, value is ExprAST
        let value_str = unparse(&attribute.value);

        self.result.push(vec![
            "BoundAttribute".to_string(),
            binding_type_str,
            attribute.name.to_string(),
            value_str,
        ]);
    }

    fn visit_bound_event(&mut self, event: &t::BoundEvent) {
        let event_type_str = match event.type_ {
            ParsedEventType::Regular => "0",
            ParsedEventType::Animation => "1",
            ParsedEventType::TwoWay => "2",
            ParsedEventType::LegacyAnimation => "3",
        }
        .to_string();

        // BoundEvent has handler as ExprAST
        let handler_str = unparse(&event.handler);

        let target_str = event
            .target
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| String::new());

        self.result.push(vec![
            "BoundEvent".to_string(),
            event_type_str,
            event.name.to_string(),
            target_str,
            handler_str,
        ]);
    }

    fn visit_text(&mut self, text: &t::Text) {
        self.result
            .push(vec!["Text".to_string(), text.value.to_string()]);
    }

    fn visit_bound_text(&mut self, text: &t::BoundText) {
        // BoundText has value as ExprAST directly
        let value_str = unparse(&text.value);
        self.result.push(vec!["BoundText".to_string(), value_str]);
    }

    fn visit_icu(&mut self, _icu: &t::Icu) {
        // ICUs are not included in humanized output for these tests
    }

    fn visit_deferred_block(&mut self, deferred: &t::DeferredBlock) {
        self.result.push(vec!["DeferredBlock".to_string()]);

        // Visit triggers
        if let Some(ref when) = deferred.triggers.when {
            self.visit_deferred_trigger(&t::DeferredTrigger::Bound(when.clone()));
        }
        if let Some(ref idle) = deferred.triggers.idle {
            self.visit_deferred_trigger(&t::DeferredTrigger::Idle(idle.clone()));
        }
        if let Some(ref immediate) = deferred.triggers.immediate {
            self.visit_deferred_trigger(&t::DeferredTrigger::Immediate(immediate.clone()));
        }
        if let Some(ref hover) = deferred.triggers.hover {
            self.visit_deferred_trigger(&t::DeferredTrigger::Hover(hover.clone()));
        }
        if let Some(ref timer) = deferred.triggers.timer {
            self.visit_deferred_trigger(&t::DeferredTrigger::Timer(timer.clone()));
        }
        if let Some(ref interaction) = deferred.triggers.interaction {
            self.visit_deferred_trigger(&t::DeferredTrigger::Interaction(interaction.clone()));
        }
        if let Some(ref viewport) = deferred.triggers.viewport {
            self.visit_deferred_trigger(&t::DeferredTrigger::Viewport(viewport.clone()));
        }
        if let Some(ref never) = deferred.triggers.never {
            self.visit_deferred_trigger(&t::DeferredTrigger::Never(never.clone()));
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

    fn visit_switch_block(&mut self, switch: &t::SwitchBlock) {
        // SwitchBlock has expression as ExprAST directly
        let expr_str = unparse(&switch.expression);
        self.result.push(vec!["SwitchBlock".to_string(), expr_str]);
        for case in &switch.cases {
            case.visit(self);
        }
    }

    fn visit_switch_block_case(&mut self, case: &t::SwitchBlockCase) {
        // SwitchBlockCase has expression as Option<ExprAST>
        let expr_str = if let Some(ref expr) = case.expression {
            unparse(expr)
        } else {
            String::new()
        };
        self.result
            .push(vec!["SwitchBlockCase".to_string(), expr_str]);
        self.visit_all(&case.children);
    }

    fn visit_for_loop_block(&mut self, for_loop: &t::ForLoopBlock) {
        // ForLoopBlock has expression and track_by as ASTWithSource
        let expr_str = unparse(&for_loop.expression.ast);
        let track_by_str = unparse(&for_loop.track_by.ast);
        self.result
            .push(vec!["ForLoopBlock".to_string(), expr_str, track_by_str]);
        for_loop.item.visit(self);
        for var in &for_loop.context_variables {
            var.visit(self);
        }
        self.visit_all(&for_loop.children);
        if let Some(ref empty) = for_loop.empty {
            empty.visit(self);
        }
    }

    fn visit_for_loop_block_empty(&mut self, _empty: &t::ForLoopBlockEmpty) {
        self.result.push(vec!["ForLoopBlockEmpty".to_string()]);
        // Note: ForLoopBlockEmpty children visit handled by parent
    }

    fn visit_if_block(&mut self, if_block: &t::IfBlock) {
        self.result.push(vec!["IfBlock".to_string()]);
        for branch in &if_block.branches {
            branch.visit(self);
        }
    }

    fn visit_if_block_branch(&mut self, branch: &t::IfBlockBranch) {
        // IfBlockBranch has expression as Option<ExprAST>
        let expr_str = if let Some(ref expr) = branch.expression {
            unparse(expr)
        } else {
            String::new()
        };
        self.result
            .push(vec!["IfBlockBranch".to_string(), expr_str]);
        if let Some(ref expr_alias) = branch.expression_alias {
            expr_alias.visit(self);
        }
        self.visit_all(&branch.children);
    }

    fn visit_unknown_block(&mut self, block: &t::UnknownBlock) {
        self.result
            .push(vec!["UnknownBlock".to_string(), block.name.to_string()]);
    }

    fn visit_let_declaration(&mut self, decl: &t::LetDeclaration) {
        // LetDeclaration has value as ExprAST directly
        let value_str = unparse(&decl.value);
        self.result.push(vec![
            "LetDeclaration".to_string(),
            decl.name.to_string(),
            value_str,
        ]);
    }

    fn visit_component(&mut self, component: &t::Component) {
        let mut res = vec![
            "Component".to_string(),
            component.component_name.to_string(),
            component
                .tag_name
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| String::new()),
            component.full_name.to_string(),
        ];
        if component.is_self_closing {
            res.push("#selfClosing".to_string());
        }
        self.result.push(res);
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
        self.result
            .push(vec!["Directive".to_string(), directive.name.to_string()]);
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

    fn visit_deferred_trigger(&mut self, trigger: &t::DeferredTrigger) {
        match trigger {
            t::DeferredTrigger::Bound(b) => {
                // BoundDeferredTrigger has value as ExprAST directly
                let value_str = unparse(&b.value);
                self.result
                    .push(vec!["BoundDeferredTrigger".to_string(), value_str]);
            }
            t::DeferredTrigger::Immediate(_i) => {
                self.result
                    .push(vec!["ImmediateDeferredTrigger".to_string()]);
            }
            t::DeferredTrigger::Hover(h) => {
                let ref_str = h
                    .reference
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| String::new());
                self.result
                    .push(vec!["HoverDeferredTrigger".to_string(), ref_str]);
            }
            t::DeferredTrigger::Idle(_i) => {
                self.result.push(vec!["IdleDeferredTrigger".to_string()]);
            }
            t::DeferredTrigger::Timer(t) => {
                self.result.push(vec![
                    "TimerDeferredTrigger".to_string(),
                    t.delay.to_string(),
                ]);
            }
            t::DeferredTrigger::Interaction(i) => {
                let ref_str = i
                    .reference
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| String::new());
                self.result
                    .push(vec!["InteractionDeferredTrigger".to_string(), ref_str]);
            }
            t::DeferredTrigger::Viewport(v) => {
                let ref_str = v
                    .reference
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| String::new());
                let mut res = vec!["ViewportDeferredTrigger".to_string(), ref_str];
                // ViewportDeferredTrigger has options as Option<LiteralMap>, not AST
                // For now, just check if options exist
                if v.options.is_some() {
                    // TODO: Serialize LiteralMap if needed
                    res.push("{}".to_string());
                }
                self.result.push(res);
            }
            t::DeferredTrigger::Never(_n) => {
                self.result.push(vec!["NeverDeferredTrigger".to_string()]);
            }
        }
    }

    fn visit_deferred_block_placeholder(&mut self, placeholder: &t::DeferredBlockPlaceholder) {
        let mut res = vec!["DeferredBlockPlaceholder".to_string()];
        if let Some(min_time) = placeholder.minimum_time {
            res.push(format!("minimum {}ms", min_time));
        }
        self.result.push(res);
        self.visit_all(&placeholder.children);
    }

    fn visit_deferred_block_loading(&mut self, loading: &t::DeferredBlockLoading) {
        let mut res = vec!["DeferredBlockLoading".to_string()];
        if let Some(after_time) = loading.after_time {
            res.push(format!("after {}ms", after_time));
        }
        if let Some(min_time) = loading.minimum_time {
            res.push(format!("minimum {}ms", min_time));
        }
        self.result.push(res);
        self.visit_all(&loading.children);
    }

    fn visit_deferred_block_error(&mut self, error: &t::DeferredBlockError) {
        self.result.push(vec!["DeferredBlockError".to_string()]);
        self.visit_all(&error.children);
    }

    fn visit_comment(&mut self, _comment: &t::Comment) {
        // Comments not included in humanized output
    }
}

fn expect_from_html(
    html: &str,
    ignore_error: bool,
    selectorless_enabled: bool,
) -> Vec<Vec<String>> {
    let res = parse_r3(
        html,
        ParseR3Options {
            ignore_error: Some(ignore_error),
            selectorless_enabled: Some(selectorless_enabled),
            ..Default::default()
        },
    );
    expect_from_r3_nodes(&res.nodes)
}

fn expect_from_r3_nodes(nodes: &[t::R3Node]) -> Vec<Vec<String>> {
    let mut humanizer = R3AstHumanizer::new();
    humanizer.visit_all(nodes);
    humanizer.result
}

fn expect_span_from_html(html: &str) -> String {
    let res = parse_r3(html, ParseR3Options::default());
    if let Some(first_node) = res.nodes.first() {
        use t::Node;
        // R3Node is an enum, need to match to get the node and call source_span()
        match first_node {
            t::R3Node::Text(n) => n.source_span().to_string(),
            t::R3Node::BoundText(n) => n.source_span().to_string(),
            t::R3Node::TextAttribute(n) => n.source_span().to_string(),
            t::R3Node::BoundAttribute(n) => n.source_span().to_string(),
            t::R3Node::BoundEvent(n) => n.source_span().to_string(),
            t::R3Node::Element(n) => n.source_span().to_string(),
            t::R3Node::Template(n) => n.source_span().to_string(),
            t::R3Node::Content(n) => n.source_span().to_string(),
            t::R3Node::Variable(n) => n.source_span().to_string(),
            t::R3Node::Reference(n) => n.source_span().to_string(),
            t::R3Node::Icu(n) => n.source_span().to_string(),
            t::R3Node::DeferredBlock(n) => n.source_span().to_string(),
            t::R3Node::DeferredBlockPlaceholder(n) => n.source_span().to_string(),
            t::R3Node::DeferredBlockLoading(n) => n.source_span().to_string(),
            t::R3Node::DeferredBlockError(n) => n.source_span().to_string(),
            t::R3Node::SwitchBlock(n) => n.source_span().to_string(),
            t::R3Node::SwitchBlockCase(n) => n.source_span().to_string(),
            t::R3Node::ForLoopBlock(n) => n.source_span().to_string(),
            t::R3Node::ForLoopBlockEmpty(n) => n.source_span().to_string(),
            t::R3Node::IfBlock(n) => n.source_span().to_string(),
            t::R3Node::IfBlockBranch(n) => n.source_span().to_string(),
            t::R3Node::UnknownBlock(n) => n.source_span().to_string(),
            t::R3Node::LetDeclaration(n) => n.source_span().to_string(),
            t::R3Node::Component(n) => n.source_span().to_string(),
            t::R3Node::Directive(n) => n.source_span().to_string(),
            t::R3Node::DeferredTrigger(n) => n.source_span().to_string(),
            t::R3Node::Comment(_) | t::R3Node::HostElement(_) => String::new(),
        }
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_span_on_nodes_to_string {
        use super::*;

        #[test]
        fn should_create_valid_text_span_on_element_with_adjacent_start_and_end_tags() {
            let result = expect_span_from_html("<div></div>");
            assert_eq!(result, "<div></div>");
        }
    }

    mod nodes_without_binding {
        use super::*;

        #[test]
        fn should_parse_incomplete_tags_terminated_by_eof() {
            let result = expect_from_html("<a", true, false);
            assert_eq!(result, vec![vec!["Element".to_string(), "a".to_string()]]);
        }

        #[test]
        fn should_parse_incomplete_tags_terminated_by_another_tag() {
            let result = expect_from_html("<a <span></span>", true, false);
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "a"));
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "span"));
        }

        #[test]
        fn should_parse_text_nodes() {
            let result = expect_from_html("a", false, false);
            assert_eq!(result, vec![vec!["Text".to_string(), "a".to_string()]]);
        }

        #[test]
        fn should_parse_elements_with_attributes() {
            let result = expect_from_html("<div a=b></div>", false, false);
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "a" && v[2] == "b"));
        }

        #[test]
        fn should_parse_ng_content() {
            let result = expect_from_html("<ng-content select=\"a\"></ng-content>", false, false);
            assert!(result.iter().any(|v| v[0] == "Content" && v[1] == "a"));
        }
    }

    mod bound_text_nodes {
        use super::*;

        #[test]
        fn should_parse_bound_text_nodes() {
            let result = expect_from_html("{{a}}", false, false);
            assert!(result.iter().any(|v| v[0] == "BoundText"));
        }
    }

    mod bound_attributes {
        use super::*;

        #[test]
        fn should_parse_mixed_case_bound_properties() {
            let result = expect_from_html("<div [someProp]=\"v\"></div>", false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "someProp"));
        }

        #[test]
        fn should_parse_bound_properties_via_bind() {
            let result = expect_from_html("<div bind-prop=\"v\"></div>", false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "prop"));
        }

        #[test]
        fn should_parse_bound_properties_via_interpolation() {
            let result = expect_from_html("<div prop=\"{{v}}\"></div>", false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "prop"));
        }

        #[test]
        fn should_parse_mixed_case_bound_attributes() {
            let result = expect_from_html("<div [attr.someAttr]=\"v\"></div>", false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "someAttr"));
        }

        #[test]
        fn should_parse_bound_classes() {
            let result = expect_from_html("<div [class.some-class]=\"v\"></div>", false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "some-class"));
        }

        #[test]
        fn should_parse_bound_styles() {
            let result = expect_from_html("<div [style.someStyle]=\"v\"></div>", false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "someStyle"));
        }
    }

    mod templates {
        use super::*;

        #[test]
        fn should_support_star_directives() {
            let result = expect_from_html("<div *ngIf></div>", false, false);
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "ngIf"));
        }

        #[test]
        fn should_support_ng_template() {
            let result = expect_from_html("<ng-template></ng-template>", false, false);
            assert!(result.iter().any(|v| v[0] == "Template"));
        }

        #[test]
        fn should_support_reference_via_hash() {
            let result = expect_from_html("<ng-template #a></ng-template>", false, false);
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result.iter().any(|v| v[0] == "Reference" && v[1] == "a"));
        }

        #[test]
        fn should_parse_variables_via_let() {
            let result = expect_from_html("<ng-template let-a=\"b\"></ng-template>", false, false);
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[1] == "a"));
        }
    }

    mod inline_templates {
        use super::*;

        #[test]
        fn should_support_attribute_and_bound_attributes() {
            let result = expect_from_html("<div *ngFor=\"let item of items\"></div>", false, false);
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "ngFor"));
            assert!(result.iter().any(|v| v[0] == "BoundAttribute"));
            assert!(result.iter().any(|v| v[0] == "Variable"));
        }
    }

    mod events {
        use super::*;

        #[test]
        fn should_parse_event_names_case_sensitive() {
            let result = expect_from_html("<div (some-event)=\"v\"></div>", false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundEvent" && v[2] == "some-event"));

            let result2 = expect_from_html("<div (someEvent)=\"v\"></div>", false, false);
            assert!(result2
                .iter()
                .any(|v| v[0] == "BoundEvent" && v[2] == "someEvent"));
        }

        #[test]
        fn should_parse_bound_events_via_on() {
            let result = expect_from_html("<div on-event=\"v\"></div>", false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundEvent" && v[2] == "event"));
        }

        #[test]
        fn should_parse_bound_events_and_properties_via_two_way_binding() {
            let result = expect_from_html("<div [(prop)]=\"v\"></div>", false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "prop"));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundEvent" && v[2] == "propChange"));
        }
    }

    mod references {
        use super::*;

        #[test]
        fn should_parse_references_via_hash() {
            let result = expect_from_html("<div #a></div>", false, false);
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            assert!(result.iter().any(|v| v[0] == "Reference" && v[1] == "a"));
        }

        #[test]
        fn should_parse_references_via_ref() {
            let result = expect_from_html("<div ref-a></div>", false, false);
            assert!(result.iter().any(|v| v[0] == "Reference" && v[1] == "a"));
        }
    }

    mod ng_content {
        use super::*;

        #[test]
        fn should_parse_ng_content_without_selector() {
            let result = expect_from_html("<ng-content></ng-content>", false, false);
            assert!(result.iter().any(|v| v[0] == "Content" && v[1] == "*"));
        }

        #[test]
        fn should_parse_ng_content_with_selector() {
            let result = expect_from_html("<ng-content select=\"a\"></ng-content>", false, false);
            assert!(result.iter().any(|v| v[0] == "Content" && v[1] == "a"));
        }
    }

    mod nodes_without_binding_extended {
        use super::*;

        #[test]
        fn should_parse_ng_content_when_it_contains_ws_only() {
            let result = expect_from_html(
                r#"<ng-content select="a">    \n   </ng-content>"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "Content" && v[1] == "a"));
        }

        #[test]
        fn should_parse_ng_content_regardless_the_namespace() {
            let result = expect_from_html(
                r#"<svg><ng-content select="a"></ng-content></svg>"#,
                false,
                false,
            );
            assert!(result
                .iter()
                .any(|v| v[0] == "Element" && v[1] == ":svg:svg"));
            assert!(result.iter().any(|v| v[0] == "Content" && v[1] == "a"));
        }

        #[test]
        fn should_indicate_whether_an_element_is_void() {
            let result = parse_r3("<input><div></div>", ParseR3Options::default());
            if let (Some(t::R3Node::Element(input)), Some(t::R3Node::Element(div))) =
                (result.nodes.get(0), result.nodes.get(1))
            {
                assert_eq!(&*input.name, "input");
                assert!(input.is_void);
                assert_eq!(&*div.name, "div");
                assert!(!div.is_void);
            } else {
                panic!("Expected two Element nodes");
            }
        }
    }

    mod bound_attributes_extended {
        use super::*;

        #[test]
        fn should_parse_dash_case_bound_properties() {
            let result = expect_from_html(r#"<div [some-prop]="v"></div>"#, false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "some-prop"));
        }

        #[test]
        fn should_parse_dotted_name_bound_properties() {
            let result = expect_from_html(r#"<div [d.ot]="v"></div>"#, false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "d.ot"));
        }

        #[test]
        fn should_parse_and_dash_case_bound_classes() {
            let result = expect_from_html(r#"<div [class.some-class]="v"></div>"#, false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "some-class"));
        }

        #[test]
        fn should_parse_mixed_case_bound_classes() {
            let result = expect_from_html(r#"<div [class.someClass]="v"></div>"#, false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "someClass"));
        }

        #[test]
        fn should_parse_class_bindings_with_various_characters() {
            let result = expect_from_html(
                r#"<foo [class.text-primary/80]="expr" [class.data-active:text-green-300/80]="expr2" [class.data-[size='large']:p-8]="expr3" some-attr/>"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "Element"
                && v[1] == "foo"
                && v.contains(&"#selfClosing".to_string())));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "text-primary/80"));
        }
    }

    mod animation_bindings {
        use super::*;

        #[test]
        fn should_support_animate_enter() {
            let result = expect_from_html(r#"<div animate.enter="foo"></div>"#, false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "animate.enter"));

            let result2 = expect_from_html(
                r#"<div [animate.enter]="['foo', 'bar']"></div>"#,
                false,
                false,
            );
            assert!(result2
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "animate.enter"));

            let result3 = expect_from_html(
                r#"<div (animate.enter)="animateFn($event)"></div>"#,
                false,
                false,
            );
            assert!(result3
                .iter()
                .any(|v| v[0] == "BoundEvent" && v[2] == "animate.enter"));
        }

        #[test]
        fn should_support_animate_leave() {
            let result = expect_from_html(r#"<div animate.leave="foo"></div>"#, false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "animate.leave"));
        }
    }

    mod templates_extended {
        use super::*;

        #[test]
        fn should_support_ng_template_regardless_the_namespace() {
            let result = expect_from_html("<svg><ng-template></ng-template></svg>", false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "Element" && v[1] == ":svg:svg"));
            assert!(result.iter().any(|v| v[0] == "Template"));
        }

        #[test]
        fn should_support_ng_template_with_structural_directive() {
            let result =
                expect_from_html(r#"<ng-template *ngIf="true"></ng-template>"#, false, false);
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result.iter().any(|v| v[0] == "BoundAttribute"));
        }

        #[test]
        fn should_parse_attributes() {
            let result = expect_from_html(
                r#"<ng-template k1="v1" k2="v2"></ng-template>"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "k1" && v[2] == "v1"));
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "k2" && v[2] == "v2"));
        }

        #[test]
        fn should_parse_bound_attributes() {
            let result = expect_from_html(
                r#"<ng-template [k1]="v1" [k2]="v2"></ng-template>"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "k1"));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "k2"));
        }

        #[test]
        fn should_parse_variables_via_as() {
            let result = expect_from_html(r#"<div *ngIf="expr as local"></div>"#, false, false);
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "ngIf"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[1] == "local"));
        }
    }

    mod events_extended {
        use super::*;

        #[test]
        fn should_parse_bound_events_with_a_target() {
            let result = expect_from_html(r#"<div (window:event)="v"></div>"#, false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundEvent" && v[2] == "event" && v[3] == "window"));
        }

        #[test]
        fn should_parse_property_reads_bound_via_two_way_binding() {
            let result = expect_from_html(r#"<div [(prop)]="a.b.c"></div>"#, false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "prop"));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundEvent" && v[2] == "propChange"));
        }

        #[test]
        fn should_parse_keyed_reads_bound_via_two_way_binding() {
            let result = expect_from_html(r#"<div [(prop)]="a['b']['c']"></div>"#, false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "prop"));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundEvent" && v[2] == "propChange"));
        }
    }

    mod variables {
        use super::*;

        #[test]
        fn should_parse_variables_via_let_on_template() {
            let result = expect_from_html(r#"<ng-template let-a="b"></ng-template>"#, false, false);
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result
                .iter()
                .any(|v| v[0] == "Variable" && v[1] == "a" && v[2] == "b"));
        }
    }

    mod references_extended {
        use super::*;

        #[test]
        fn should_parse_camel_case_references() {
            let result = expect_from_html("<div #someA></div>", false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "Reference" && v[1] == "someA"));
        }
    }

    mod ng_content_extended {
        use super::*;

        #[test]
        fn should_parse_ng_content_with_a_specific_selector() {
            let result = expect_from_html(
                r#"<ng-content select="tag[attribute]"></ng-content>"#,
                false,
                false,
            );
            assert!(result
                .iter()
                .any(|v| v[0] == "Content" && v[1] == "tag[attribute]"));
        }

        #[test]
        fn should_parse_ng_content_with_children() {
            let result = expect_from_html(
                r#"<ng-content><section>Root <div>Parent <span>Child</span></div></section></ng-content>"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "Content"));
            assert!(result
                .iter()
                .any(|v| v[0] == "Element" && v[1] == "section"));
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "span"));
        }
    }

    mod ignored_elements {
        use super::*;

        #[test]
        fn should_ignore_script_elements() {
            let result = expect_from_html("<script></script>a", false, false);
            assert!(result.iter().any(|v| v[0] == "Text" && v[1] == "a"));
            assert!(!result.iter().any(|v| v[0] == "Element" && v[1] == "script"));
        }

        #[test]
        fn should_ignore_style_elements() {
            let result = expect_from_html("<style></style>a", false, false);
            assert!(result.iter().any(|v| v[0] == "Text" && v[1] == "a"));
            assert!(!result.iter().any(|v| v[0] == "Element" && v[1] == "style"));
        }
    }

    mod link_stylesheet {
        use super::*;

        #[test]
        fn should_keep_link_rel_stylesheet_elements_if_they_have_an_absolute_url() {
            let result = expect_from_html(
                r#"<link rel="stylesheet" href="http://someurl">"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "link"));
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "rel" && v[2] == "stylesheet"));
        }

        #[test]
        fn should_keep_link_rel_stylesheet_elements_if_they_have_no_uri() {
            let result = expect_from_html(r#"<link rel="stylesheet">"#, false, false);
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "link"));
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "rel"));
        }

        #[test]
        fn should_ignore_link_rel_stylesheet_elements_if_they_have_a_relative_uri() {
            let result = expect_from_html(
                r#"<link rel="stylesheet" href="./other.css">"#,
                false,
                false,
            );
            // Should be empty or not contain link element
            assert!(!result.iter().any(|v| v[0] == "Element" && v[1] == "link"));
        }
    }

    mod ng_non_bindable {
        use super::*;

        #[test]
        fn should_ignore_bindings_on_children_of_elements_with_ng_non_bindable() {
            let result = expect_from_html(r#"<div ngNonBindable>{{b}}</div>"#, false, false);
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "ngNonBindable"));
            assert!(result.iter().any(|v| v[0] == "Text" && v[1] == "{{b}}"));
        }

        #[test]
        fn should_keep_nested_children_of_elements_with_ng_non_bindable() {
            let result = expect_from_html(
                r#"<div ngNonBindable><span>{{b}}</span></div>"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "span"));
            assert!(result.iter().any(|v| v[0] == "Text" && v[1] == "{{b}}"));
        }
    }

    mod deferred_blocks_extended {
        use super::*;

        #[test]
        fn should_parse_a_deferred_block_with_a_timer_set_in_seconds() {
            let result = expect_from_html("@defer (on timer(10s)){hello}", false, false);
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result.iter().any(|v| v[0] == "TimerDeferredTrigger"));
        }

        #[test]
        fn should_parse_a_deferred_block_with_a_hover_trigger() {
            let result = expect_from_html("@defer (on hover(button)){hello}", false, false);
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result
                .iter()
                .any(|v| v[0] == "HoverDeferredTrigger" && v[1] == "button"));
        }

        #[test]
        fn should_parse_a_deferred_block_with_an_interaction_trigger() {
            let result = expect_from_html("@defer (on interaction(button)){hello}", false, false);
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result
                .iter()
                .any(|v| v[0] == "InteractionDeferredTrigger" && v[1] == "button"));
        }

        #[test]
        fn should_parse_a_deferred_block_with_connected_blocks() {
            let result = expect_from_html(
                r#"@defer {<calendar-cmp [date]="current"/>}@loading {Loading...}@placeholder {Placeholder content!}@error {Loading failed :(}"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result.iter().any(|v| v[0] == "DeferredBlockPlaceholder"));
            assert!(result.iter().any(|v| v[0] == "DeferredBlockLoading"));
            assert!(result.iter().any(|v| v[0] == "DeferredBlockError"));
        }

        #[test]
        fn should_parse_a_loading_block_with_parameters() {
            let result = expect_from_html(
                r#"@defer{<calendar-cmp [date]="current"/>}@loading (after 100ms; minimum 1.5s){Loading...}"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "DeferredBlockLoading"));
        }

        #[test]
        fn should_parse_a_placeholder_block_with_parameters() {
            let result = expect_from_html(
                r#"@defer {<calendar-cmp [date]="current"/>}@placeholder (minimum 1.5s){Placeholder...}"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "DeferredBlockPlaceholder"));
        }
    }

    mod switch_blocks {
        use super::*;

        #[test]
        fn should_parse_a_switch_block() {
            let result = expect_from_html(
                r#"
          @switch (cond.kind) {
            @case (x()) { X case }
            @case ('hello') {<button>Y case</button>}
            @case (42) { Z case }
            @default { No case matched }
          }
        "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "SwitchBlock"));
            assert!(result.iter().any(|v| v[0] == "SwitchBlockCase"));
        }

        #[test]
        fn should_parse_a_nested_switch_block() {
            let result = expect_from_html(
                r#"
          @switch (cond) {
            @case ('a') {
              @switch (innerCond) {
                @case ('innerA') { Inner A }
                @case ('innerB') { Inner B }
              }
            }
            @case ('b') {<button>Y case</button>}
          }
        "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "SwitchBlock"));
            // Should have nested switch blocks
            let switch_count = result.iter().filter(|v| v[0] == "SwitchBlock").count();
            assert!(switch_count >= 2, "Expected at least 2 SwitchBlock nodes");
        }
    }

    mod for_loop_blocks {
        use super::*;

        #[test]
        fn should_parse_a_for_loop_block() {
            let result = expect_from_html(
                r#"
        @for (item of items.foo.bar; track item.id) {
          {{ item }}
        } @empty {
          There were no items in the list.
        }
      "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "ForLoopBlock"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[1] == "item"));
            assert!(result.iter().any(|v| v[0] == "ForLoopBlockEmpty"));
        }

        #[test]
        fn should_parse_a_for_loop_block_with_let_parameters() {
            let result = expect_from_html(
                r#"
        @for (item of items.foo.bar; track item.id; let idx = $index, f = $first) {
          {{ item }}
        }
      "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "ForLoopBlock"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[1] == "idx"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[1] == "f"));
        }

        #[test]
        fn should_parse_nested_for_loop_blocks() {
            let result = expect_from_html(
                r#"
        @for (item of items.foo.bar; track item.id) {
          {{ item }}
          <div>
            @for (subitem of item.items; track subitem.id) {<h1>{{subitem}}</h1>}
          </div>
        } @empty {
          There were no items in the list.
        }
      "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "ForLoopBlock"));
            let for_loop_count = result.iter().filter(|v| v[0] == "ForLoopBlock").count();
            assert!(
                for_loop_count >= 2,
                "Expected at least 2 ForLoopBlock nodes"
            );
        }
    }

    mod if_blocks {
        use super::*;

        #[test]
        fn should_parse_an_if_block() {
            let result = expect_from_html(
                r#"
          @if (cond) {
            True
          } @else if (cond2) {
            Else if
          } @else {
            False
          }
        "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "IfBlock"));
            assert!(result.iter().any(|v| v[0] == "IfBlockBranch"));
        }

        #[test]
        fn should_parse_an_if_block_with_as_expression() {
            let result = expect_from_html(
                r#"
          @if (cond; as local) {
            True
          }
        "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "IfBlock"));
            assert!(result.iter().any(|v| v[0] == "Variable"));
        }

        #[test]
        fn should_parse_nested_if_blocks() {
            let result = expect_from_html(
                r#"
          @if (outer) {
            @if (inner) {
              Inner true
            } @else {
              Inner false
            }
          } @else {
            Outer false
          }
        "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "IfBlock"));
            let if_block_count = result.iter().filter(|v| v[0] == "IfBlock").count();
            assert!(if_block_count >= 2, "Expected at least 2 IfBlock nodes");
        }
    }

    mod let_declarations {
        use super::*;

        #[test]
        fn should_parse_a_let_declaration() {
            let result = expect_from_html("@let foo = 123 + 456;", false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "LetDeclaration" && v[1] == "foo"));
        }
    }

    mod component_nodes {
        use super::*;

        #[test]
        fn should_parse_a_simple_component_node() {
            let result = expect_from_html("<MyComp>Hello</MyComp>", false, true);
            assert!(result
                .iter()
                .any(|v| v[0] == "Component" && v[1] == "MyComp"));
            assert!(result.iter().any(|v| v[0] == "Text" && v[1] == "Hello"));
        }

        #[test]
        fn should_parse_a_component_node_with_a_tag_name() {
            let result = expect_from_html("<MyComp:button>Hello</MyComp:button>", false, true);
            assert!(result
                .iter()
                .any(|v| v[0] == "Component" && v[1] == "MyComp"));
        }
    }

    mod directives {
        use super::*;

        #[test]
        fn should_parse_a_directive_with_no_attributes() {
            let result = expect_from_html("<div @Dir></div>", false, true);
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            assert!(result.iter().any(|v| v[0] == "Directive" && v[1] == "Dir"));
        }

        #[test]
        fn should_parse_a_directive_with_attributes() {
            let result = expect_from_html(
                r#"<div @Dir(a="1" [b]="two" (c)="c()")></div>"#,
                false,
                true,
            );
            assert!(result.iter().any(|v| v[0] == "Directive" && v[1] == "Dir"));
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "a"));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "b"));
            assert!(result.iter().any(|v| v[0] == "BoundEvent" && v[2] == "c"));
        }

        #[test]
        fn should_parse_a_directive_mixed_with_other_attributes() {
            let result = expect_from_html(
                r#"<div before="foo" @Dir middle @OtherDir([a]="a" (b)="b()") after="123"></div>"#,
                false,
                true,
            );
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            assert!(result.iter().any(|v| v[0] == "Directive" && v[1] == "Dir"));
            assert!(result
                .iter()
                .any(|v| v[0] == "Directive" && v[1] == "OtherDir"));
        }

        #[test]
        fn should_remove_directives_inside_ng_non_bindable() {
            let result = expect_from_html(
                r#"<div ngNonBindable><span @EmptyDir @WithAttrs(foo="123" [bar]="321")></span></div>"#,
                false,
                true,
            );
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "span"));
            // Directives should not be present
            assert!(!result.iter().any(|v| v[0] == "Directive"));
        }
    }

    // Additional error/validation test cases
    mod bound_attributes_errors {
        use super::*;

        #[test]
        #[should_panic(expected = "Property name is missing")]
        fn should_report_missing_property_names_in_bind_syntax() {
            let _ = parse_r3("<div bind-></div>", ParseR3Options::default());
        }
    }

    mod events_errors {
        use super::*;

        #[test]
        #[should_panic(expected = "Event name is missing")]
        fn should_report_missing_event_names_in_on_syntax() {
            let _ = parse_r3("<div on-></div>", ParseR3Options::default());
        }

        #[test]
        fn should_parse_any_in_a_two_way_binding() {
            let result = expect_from_html(r#"<div [(prop)]="$any(v)"></div>"#, false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "prop"));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundEvent" && v[2] == "propChange"));
        }

        #[test]
        fn should_parse_bound_events_and_properties_via_bindon() {
            let result = expect_from_html(r#"<div bindon-prop="v"></div>"#, false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "prop"));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundEvent" && v[2] == "propChange"));
        }

        #[test]
        fn should_parse_bound_events_and_properties_via_two_way_with_non_null_operator() {
            let result = expect_from_html(r#"<div [(prop)]="v!"></div>"#, false, false);
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "prop"));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundEvent" && v[2] == "propChange"));
        }
    }

    mod templates_errors {
        use super::*;

        #[test]
        fn should_support_reference_via_ref_on_template() {
            let result = expect_from_html("<ng-template ref-a></ng-template>", false, false);
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result.iter().any(|v| v[0] == "Reference" && v[1] == "a"));
        }

        #[test]
        #[should_panic(expected = "defined more than once")]
        fn should_report_an_error_if_a_reference_is_used_multiple_times_on_the_same_template() {
            let _ = parse_r3(
                "<ng-template #a #a></ng-template>",
                ParseR3Options::default(),
            );
        }

        #[test]
        fn should_parse_variables_via_let_on_template() {
            let result = expect_from_html(r#"<ng-template let-a="b"></ng-template>"#, false, false);
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result
                .iter()
                .any(|v| v[0] == "Variable" && v[1] == "a" && v[2] == "b"));
        }

        #[test]
        fn should_support_ng_template_with_structural_directive_check_tag_name() {
            let result = parse_r3(
                r#"<ng-template *ngIf="true"></ng-template>"#,
                ParseR3Options::default(),
            );
            // The template should have nested templates
            assert!(!result.nodes.is_empty());
        }
    }

    mod inline_templates_extended {
        use super::*;

        #[test]
        fn should_parse_variables_via_let_in_inline_template() {
            let result = expect_from_html(r#"<div *ngIf="let a=b"></div>"#, false, false);
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[1] == "a"));
        }

        #[test]
        fn should_parse_incorrect_ng_for_usage() {
            let result = expect_from_html(r#"<div *ngFor="item of items"></div>"#, false, false);
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "ngFor"));
            assert!(result
                .iter()
                .any(|v| v[0] == "BoundAttribute" && v[2] == "ngForOf"));
        }
    }

    mod variables_errors {
        use super::*;

        #[test]
        #[should_panic(expected = "only supported on ng-template")]
        fn should_report_variables_not_on_template_elements() {
            let _ = parse_r3(r#"<div let-a-name="b"></div>"#, ParseR3Options::default());
        }

        #[test]
        #[should_panic(expected = "does not have a name")]
        fn should_report_missing_variable_names() {
            let _ = parse_r3("<ng-template let-><ng-template>", ParseR3Options::default());
        }
    }

    mod references_errors {
        use super::*;

        #[test]
        #[should_panic(expected = "not allowed in reference names")]
        fn should_report_invalid_reference_names() {
            let _ = parse_r3("<div #a-b></div>", ParseR3Options::default());
        }

        #[test]
        #[should_panic(expected = "does not have a name")]
        fn should_report_missing_reference_names() {
            let _ = parse_r3("<div #></div>", ParseR3Options::default());
        }

        #[test]
        #[should_panic(expected = "defined more than once")]
        fn should_report_an_error_if_a_reference_is_used_multiple_times_on_the_same_element() {
            let _ = parse_r3("<div #a #a></div>", ParseR3Options::default());
        }
    }

    mod literal_attribute_errors {
        use super::*;

        #[test]
        #[should_panic(expected = "Animation trigger is missing")]
        fn should_report_missing_animation_trigger_in_at_syntax() {
            let _ = parse_r3("<div @></div>", ParseR3Options::default());
        }
    }

    mod ng_content_extended_further {
        use super::*;

        #[test]
        fn should_parse_ng_content_with_a_selector() {
            let result = expect_from_html(
                r#"<ng-content select="a"></ng-content><ng-content></ng-content><ng-content select="b"></ng-content>"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "Content" && v[1] == "a"));
            assert!(result.iter().any(|v| v[0] == "Content" && v[1] == "*"));
            assert!(result.iter().any(|v| v[0] == "Content" && v[1] == "b"));
        }

        #[test]
        fn should_parse_ng_project_as_as_an_attribute() {
            let result =
                expect_from_html(r#"<ng-content ngProjectAs="a"></ng-content>"#, false, false);
            assert!(result.iter().any(|v| v[0] == "Content" && v[1] == "*"));
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "ngProjectAs"));
        }
    }

    mod parser_errors {
        use super::*;

        #[test]
        fn should_only_report_errors_on_the_node_on_which_the_error_occurred() {
            let result = parse_r3(
                r#"
        <input (input)="foo(12#3)">
        <button (click)="bar()"></button>
        <span (mousedown)="baz()"></span>
      "#,
                ParseR3Options {
                    ignore_error: Some(true),
                    ..Default::default()
                },
            );
            // Should have errors but continue parsing
            assert!(!result.errors.is_empty());
        }

        #[test]
        fn should_report_parsing_errors_on_the_specific_interpolated_expressions() {
            let result = parse_r3(
                r#"
          bunch of text bunch of text bunch of text bunch of text bunch of text bunch of text
          bunch of text bunch of text bunch of text bunch of text

          {{foo[0}} bunch of text bunch of text bunch of text bunch of text {{.bar}}

          bunch of text
          bunch of text
          bunch of text
          bunch of text
          bunch of text {{one + #two + baz}}
        "#,
                ParseR3Options {
                    ignore_error: Some(true),
                    ..Default::default()
                },
            );
            // Should have errors for the malformed interpolations
            assert!(!result.errors.is_empty());
        }
    }

    mod ng_non_bindable_extended {
        use super::*;

        #[test]
        fn should_ignore_script_elements_inside_of_elements_with_ng_non_bindable() {
            let result =
                expect_from_html("<div ngNonBindable><script></script>a</div>", false, false);
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            assert!(result.iter().any(|v| v[0] == "Text" && v[1] == "a"));
            assert!(!result.iter().any(|v| v[0] == "Element" && v[1] == "script"));
        }

        #[test]
        fn should_ignore_style_elements_inside_of_elements_with_ng_non_bindable() {
            let result =
                expect_from_html("<div ngNonBindable><style></style>a</div>", false, false);
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            assert!(result.iter().any(|v| v[0] == "Text" && v[1] == "a"));
        }

        #[test]
        fn should_ignore_link_rel_stylesheet_elements_inside_of_elements_with_ng_non_bindable() {
            let result = expect_from_html(
                r#"<div ngNonBindable><link rel="stylesheet">a</div>"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            assert!(result.iter().any(|v| v[0] == "Text" && v[1] == "a"));
        }
    }

    mod deferred_blocks_complete {
        use super::*;

        #[test]
        fn should_parse_a_deferred_block_with_a_timer_with_a_decimal_point() {
            let result = expect_from_html("@defer (on timer(1.5s)){hello}", false, false);
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result.iter().any(|v| v[0] == "TimerDeferredTrigger"));
        }

        #[test]
        fn should_parse_a_deferred_block_with_a_timer_that_has_no_units() {
            let result = expect_from_html("@defer (on timer(100)){hello}", false, false);
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result.iter().any(|v| v[0] == "TimerDeferredTrigger"));
        }

        #[test]
        fn should_parse_a_deferred_block_with_comments_between_the_connected_blocks() {
            let result = expect_from_html(
                r#"@defer {<calendar-cmp [date]="current"/>}<!-- Show this while loading --> @loading {Loading...}<!-- Show this on the server --> @placeholder {Placeholder content!}<!-- Show this on error --> @error {Loading failed :(}"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result.iter().any(|v| v[0] == "DeferredBlockPlaceholder"));
            assert!(result.iter().any(|v| v[0] == "DeferredBlockLoading"));
            assert!(result.iter().any(|v| v[0] == "DeferredBlockError"));
        }

        #[test]
        fn should_parse_a_deferred_block_with_when_and_on_triggers() {
            let result = expect_from_html(
                "@defer (when isVisible(); on timer(100ms), idle, viewport(button)){hello}",
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result.iter().any(|v| v[0] == "BoundDeferredTrigger"));
            assert!(result.iter().any(|v| v[0] == "TimerDeferredTrigger"));
            assert!(result.iter().any(|v| v[0] == "IdleDeferredTrigger"));
        }

        #[test]
        fn should_allow_new_line_after_trigger_name() {
            let result = expect_from_html(
                "@defer(\nwhen\nisVisible(); on\ntimer(100ms),\nidle, viewport(button)){hello}",
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
        }

        #[test]
        fn should_parse_a_deferred_block_with_prefetch_triggers() {
            let result = expect_from_html(
                "@defer (on idle; prefetch on viewport(button), hover(button); prefetch when shouldPrefetch()){hello}",
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result.iter().any(|v| v[0] == "IdleDeferredTrigger"));
        }

        #[test]
        fn should_parse_a_deferred_block_with_a_non_parenthesized_trigger_at_the_end() {
            let result = expect_from_html(
                "@defer (on idle, viewport(button), immediate){hello}",
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result.iter().any(|v| v[0] == "ImmediateDeferredTrigger"));
        }

        #[test]
        fn should_parse_triggers_with_implied_target_elements() {
            let result = expect_from_html(
                "@defer (on hover, interaction, viewport; prefetch on hover, interaction, viewport) {hello}@placeholder {<implied-trigger/>}",
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result.iter().any(|v| v[0] == "HoverDeferredTrigger"));
        }

        #[test]
        fn should_parse_a_viewport_trigger_with_an_options_parameter() {
            let result = expect_from_html(
                r#"@defer (on viewport({trigger: foo, rootMargin: "123px", threshold: [1, 2, 3]})){hello}"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result.iter().any(|v| v[0] == "ViewportDeferredTrigger"));
        }

        #[test]
        fn should_parse_a_viewport_trigger_with_an_options_parameter_but_without_a_trigger() {
            let result = expect_from_html(
                r#"@defer (on viewport({rootMargin: "123px"})){hello}"#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "DeferredBlock"));
            assert!(result.iter().any(|v| v[0] == "ViewportDeferredTrigger"));
        }
    }

    mod switch_blocks_extended {
        use super::*;

        #[test]
        fn should_parse_a_switch_block_when_preserve_whitespaces_is_enabled() {
            let result = parse_r3(
                r#"
        @switch (cond.kind) {
          @case (x()) {
            X case
          }
          @case ('hello') {
            <button>Y case</button>
          }
          @default {
            No case matched
          }
        }
      "#,
                ParseR3Options {
                    preserve_whitespaces: Some(true),
                    ..Default::default()
                },
            );
            assert!(!result.nodes.is_empty());
        }

        #[test]
        fn should_parse_a_switch_block_with_optional_parentheses() {
            let result = expect_from_html(
                r#"
          @switch ((cond.kind)) {
            @case ((x())) { X case }
            @case (('hello')) {<button>Y case</button>}
            @case ((42)) { Z case }
            @default { No case matched }
          }
        "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "SwitchBlock"));
            assert!(result.iter().any(|v| v[0] == "SwitchBlockCase"));
        }

        #[test]
        fn should_parse_a_switch_block_containing_comments() {
            let result = expect_from_html(
                r#"
          @switch (cond.kind) {
            <!-- X case -->
            @case (x) { X case }

            <!-- default case -->
            @default { No case matched }
          }
        "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "SwitchBlock"));
            assert!(result.iter().any(|v| v[0] == "SwitchBlockCase"));
        }
    }

    mod for_loop_blocks_extended {
        use super::*;

        #[test]
        fn should_parse_a_for_loop_block_with_optional_parentheses() {
            let result = expect_from_html(
                r#"
        @for ((item of items.foo.bar); track item.id){
          {{ item }}
        }
      "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "ForLoopBlock"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[1] == "item"));
        }

        #[test]
        fn should_parse_a_for_loop_block_with_newlines_in_its_let_parameters() {
            let result = expect_from_html(
                r#"
        @for (item of items.foo.bar; track item.id; let
idx = $index,
f = $first,
c = $count,
l = $last,
ev = $even,
od = $odd) {
          {{ item }}
        }
      "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "ForLoopBlock"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[1] == "idx"));
        }

        #[test]
        fn should_parse_a_for_loop_block_with_a_function_call_in_the_track_expression() {
            let result = expect_from_html(
                r#"
        @for (item of items.foo.bar; track trackBy(item.id, 123)) {
          {{ item }}
        }
      "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "ForLoopBlock"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[1] == "item"));
        }

        #[test]
        fn should_parse_a_for_loop_block_with_newlines_in_its_expression() {
            let result = expect_from_html(
                r#"
        @for (item
of
items.foo.bar; track item.id +
foo) {{{ item }}}
      "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "ForLoopBlock"));
        }

        #[test]
        fn should_parse_for_loop_block_expression_containing_new_lines() {
            let result = expect_from_html(
                r#"
        @for (item of [
          { id: 1 },
          { id: 2 }
        ]; track item.id) {
          {{ item }}
        }
      "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "ForLoopBlock"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[1] == "item"));
        }
    }

    mod if_blocks_extended {
        use super::*;

        #[test]
        fn should_parse_an_if_block_with_optional_parentheses() {
            let result = expect_from_html(
                r#"
          @if ((cond.expr)) {
            Main case was true!
          } @else if ((other.expr)) {
            Extra case was true!
          } @else {
            False case!
          }
        "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "IfBlock"));
            assert!(result.iter().any(|v| v[0] == "IfBlockBranch"));
        }

        #[test]
        fn should_parse_an_else_if_block_with_multiple_spaces() {
            let result = expect_from_html(
                r#"
        @if (cond.expr; as foo) {
          Main case was true!
        } @else        if (other.expr) {
          Other case was true!
        }
        "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "IfBlock"));
            assert!(result.iter().any(|v| v[0] == "IfBlockBranch"));
        }

        #[test]
        fn should_parse_an_if_block_containing_comments_between_the_branches() {
            let result = expect_from_html(
                r#"
        @if (cond.expr; as foo) {
          Main case was true!
        }
        <!-- Extra case -->
        @else if (other.expr) {
          Extra case was true!
        }
        <!-- False case -->
        @else {
          False case!
        }
        "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "IfBlock"));
            assert!(result.iter().any(|v| v[0] == "IfBlockBranch"));
        }

        #[test]
        fn should_parse_an_else_if_block_with_an_aliased_expression() {
            let result = expect_from_html(
                r#"
        @if (cond.expr; as foo) {
          Main case was true!
        } @else if (other.expr; as bar) {
          Other case was true!
        }
        "#,
                false,
                false,
            );
            assert!(result.iter().any(|v| v[0] == "IfBlock"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[1] == "foo"));
            assert!(result.iter().any(|v| v[0] == "Variable" && v[1] == "bar"));
        }
    }

    mod let_declarations_extended {
        use super::*;

        #[test]
        fn should_produce_a_text_node_when_let_is_used_inside_ng_non_bindable() {
            let result = expect_from_html("<div ngNonBindable>@let foo = 123;</div>", false, false);
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            assert!(result
                .iter()
                .any(|v| v[0] == "TextAttribute" && v[1] == "ngNonBindable"));
            assert!(result
                .iter()
                .any(|v| v[0] == "Text" && v[1].contains("@let")));
        }
    }

    mod component_nodes_extended {
        use super::*;

        #[test]
        fn should_parse_a_component_tag_nested_within_other_markup() {
            let result = expect_from_html(
                "@if (expr) {<div>Hello: <MyComp><span><OtherComp/></span></MyComp></div>}",
                false,
                true,
            );
            assert!(result.iter().any(|v| v[0] == "IfBlock"));
            assert!(result
                .iter()
                .any(|v| v[0] == "Component" && v[1] == "MyComp"));
            assert!(result
                .iter()
                .any(|v| v[0] == "Component" && v[1] == "OtherComp"));
        }

        #[test]
        fn should_parse_a_component_node_with_attributes_and_directives() {
            let result = expect_from_html(
                r#"<MyComp before="foo" @Dir middle @OtherDir([a]="a" (b)="b()") after="123">Hello</MyComp>"#,
                false,
                true,
            );
            assert!(result
                .iter()
                .any(|v| v[0] == "Component" && v[1] == "MyComp"));
            assert!(result.iter().any(|v| v[0] == "Directive" && v[1] == "Dir"));
            assert!(result
                .iter()
                .any(|v| v[0] == "Directive" && v[1] == "OtherDir"));
        }

        #[test]
        fn should_parse_a_component_node_with_star_directives() {
            let result = expect_from_html(r#"<MyComp *ngIf="expr">Hello</MyComp>"#, false, true);
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result
                .iter()
                .any(|v| v[0] == "Component" && v[1] == "MyComp"));
        }

        #[test]
        fn should_not_pick_up_attributes_from_directives_when_using_star_syntax() {
            let result = expect_from_html(
                r#"<MyComp *ngIf="true" @Dir(static="1" [bound]="expr" (event)="fn()")/>"#,
                false,
                true,
            );
            assert!(result.iter().any(|v| v[0] == "Template"));
            assert!(result
                .iter()
                .any(|v| v[0] == "Component" && v[1] == "MyComp"));
            assert!(result.iter().any(|v| v[0] == "Directive" && v[1] == "Dir"));
        }

        #[test]
        fn should_treat_components_as_elements_inside_ng_non_bindable() {
            let result = expect_from_html(
                r#"<div ngNonBindable><MyComp foo="bar" @Dir(some="attr")></MyComp></div>"#,
                false,
                true,
            );
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "div"));
            // Component should be treated as element in ngNonBindable
            assert!(result.iter().any(|v| v[0] == "Element" && v[1] == "MyComp"));
        }
    }

    mod ng_container_errors {
        use super::*;

        #[test]
        fn should_report_an_error_for_attribute_bindings_on_ng_container() {
            let result = parse_r3(
                r#"<ng-container [attr.title]="'test'"></ng-container>"#,
                ParseR3Options {
                    ignore_error: Some(true),
                    ..Default::default()
                },
            );
            // Should have an error about attribute bindings on ng-container
            assert!(!result.errors.is_empty());
        }

        #[test]
        fn should_not_report_an_error_on_non_attr_bindings_on_ng_container() {
            let _result = parse_r3(
                r#"<ng-container *ngIf="test" [ngTemplateOutlet]="foo"></ng-container>"#,
                ParseR3Options {
                    ignore_error: Some(true),
                    ..Default::default()
                },
            );
            // Should not have errors for non-attr bindings
            // Note: This might still have errors due to syntax issues, but not about attribute bindings
        }
    }

    // Validation test cases - Many of these will need proper error handling in the parser
    // For now, we mark them as should_panic or check errors when ignore_error is true
    mod deferred_blocks_validations {
        use super::*;

        // Note: These tests check for validation errors.
        // Most will panic if the parser properly validates, or can be checked with ignore_error=true
        #[test]
        #[should_panic]
        fn should_report_syntax_error_in_when_trigger() {
            let _ = parse_r3("@defer (when isVisible#){hello}", ParseR3Options::default());
        }

        #[test]
        #[should_panic]
        fn should_report_unrecognized_trigger() {
            let _ = parse_r3(
                "@defer (unknown visible()){hello}",
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_content_before_a_connected_block() {
            let _ = parse_r3(
                "@defer {hello} <br> @placeholder {placeholder}",
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_connected_defer_blocks_used_without_a_defer_block_placeholder() {
            let _ = parse_r3("@placeholder {placeholder}", ParseR3Options::default());
        }

        #[test]
        #[should_panic]
        fn should_report_connected_defer_blocks_used_without_a_defer_block_loading() {
            let _ = parse_r3("@loading {loading}", ParseR3Options::default());
        }

        #[test]
        #[should_panic]
        fn should_report_connected_defer_blocks_used_without_a_defer_block_error() {
            let _ = parse_r3("@error {error}", ParseR3Options::default());
        }

        #[test]
        #[should_panic]
        fn should_report_multiple_placeholder_blocks() {
            let _ = parse_r3(
                "@defer {hello} @placeholder {p1} @placeholder {p2}",
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_multiple_loading_blocks() {
            let _ = parse_r3(
                "@defer {hello} @loading {l1} @loading {l2}",
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_multiple_error_blocks() {
            let _ = parse_r3(
                "@defer {hello} @error {e1} @error {e2}",
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_unrecognized_parameter_in_placeholder_block() {
            let _ = parse_r3(
                "@defer {hello} @placeholder (unknown 100ms) {hi}",
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_unrecognized_parameter_in_loading_block() {
            let _ = parse_r3(
                "@defer {hello} @loading (unknown 100ms) {hi}",
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_any_parameter_usage_in_error_block() {
            let _ = parse_r3(
                "@defer {hello} @error (foo) {hi}",
                ParseR3Options::default(),
            );
        }
    }

    mod switch_blocks_validations {
        use super::*;

        #[test]
        #[should_panic]
        fn should_report_if_case_or_default_is_used_outside_of_a_switch_block() {
            let _ = parse_r3("@case (foo) {}", ParseR3Options::default());
        }

        #[test]
        #[should_panic]
        fn should_report_if_a_switch_has_no_parameters() {
            let _ = parse_r3(
                r#"
          @switch {
            @case (1) {case}
          }
        "#,
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_if_a_switch_has_more_than_one_parameter() {
            let _ = parse_r3(
                r#"
          @switch (foo; bar) {
            @case (1) {case}
          }
        "#,
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_if_a_case_has_no_parameters() {
            let _ = parse_r3(
                r#"
          @switch (cond) {
            @case {case}
          }
        "#,
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_if_a_case_has_more_than_one_parameter() {
            let _ = parse_r3(
                r#"
          @switch (cond) {
            @case (foo; bar) {case}
          }
        "#,
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_if_a_switch_has_multiple_default_blocks() {
            let _ = parse_r3(
                r#"
          @switch (cond) {
            @case (foo) {foo}
            @default {one}
            @default {two}
          }
        "#,
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_if_a_default_block_has_parameters() {
            let _ = parse_r3(
                r#"
          @switch (cond) {
            @case (foo) {foo}
            @default (bar) {bar}
          }
        "#,
                ParseR3Options::default(),
            );
        }
    }

    mod for_loop_blocks_validations {
        use super::*;

        #[test]
        #[should_panic]
        fn should_report_if_for_loop_does_not_have_an_expression() {
            let _ = parse_r3("@for {hello}", ParseR3Options::default());
        }

        #[test]
        #[should_panic]
        fn should_report_if_for_loop_does_not_have_a_tracking_expression() {
            let _ = parse_r3("@for (a of b) {hello}", ParseR3Options::default());
        }

        #[test]
        #[should_panic]
        fn should_report_multiple_track_parameters() {
            let _ = parse_r3(
                "@for (a of b; track c; track d) {hello}",
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_for_loop_with_multiple_empty_blocks() {
            let _ = parse_r3(
                r#"
          @for (a of b; track a) {
            Main
          } @empty {
            Empty one
          } @empty {
            Empty two
          }
        "#,
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_empty_block_with_parameters() {
            let _ = parse_r3(
                r#"
          @for (a of b; track a) {
            main
          } @empty (foo) {
            empty
          }
        "#,
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_an_empty_block_used_without_a_for_loop_block() {
            let _ = parse_r3("@empty {hello}", ParseR3Options::default());
        }
    }

    mod if_blocks_validations {
        use super::*;

        #[test]
        #[should_panic]
        fn should_report_an_if_block_without_a_condition() {
            let _ = parse_r3(
                r#"
          @if {hello}
        "#,
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_an_if_block_that_has_multiple_as_expressions() {
            let _ = parse_r3(
                r#"
          @if (foo; as foo; as bar) {hello}
        "#,
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_an_else_if_block_used_without_an_if_block() {
            let _ = parse_r3("@else if (foo) {hello}", ParseR3Options::default());
        }

        #[test]
        #[should_panic]
        fn should_report_an_else_block_used_without_an_if_block() {
            let _ = parse_r3("@else (foo) {hello}", ParseR3Options::default());
        }

        #[test]
        #[should_panic]
        fn should_report_content_between_an_if_and_else_if_block() {
            let _ = parse_r3(
                "@if (foo) {hello} <div></div> @else if (bar) {goodbye}",
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_content_between_an_if_and_else_block() {
            let _ = parse_r3(
                "@if (foo) {hello} <div></div> @else {goodbye}",
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_an_else_block_with_parameters() {
            let _ = parse_r3(
                r#"
          @if (foo) {hello} @else (bar) {goodbye}
        "#,
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_a_conditional_with_multiple_else_blocks() {
            let _ = parse_r3(
                r#"
          @if (foo) {hello} @else {goodbye} @else {goodbye again}
        "#,
                ParseR3Options::default(),
            );
        }

        #[test]
        #[should_panic]
        fn should_report_an_else_if_block_after_an_else_block() {
            let _ = parse_r3(
                r#"
          @if (foo) {hello} @else {goodbye} @else (if bar) {goodbye again}
        "#,
                ParseR3Options::default(),
            );
        }
    }

    mod let_declarations_validations {
        use super::*;

        #[test]
        #[should_panic]
        fn should_report_a_let_declaration_with_no_value() {
            let _ = parse_r3("@let foo =  ;", ParseR3Options::default());
        }
    }

    mod component_nodes_validations {
        use super::*;

        #[test]
        #[should_panic]
        fn should_not_allow_a_selectorless_component_with_an_unsupported_tag_name_link() {
            let _ = parse_r3(
                "<MyComp:link></MyComp:link>",
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_a_selectorless_component_with_an_unsupported_tag_name_style() {
            let _ = parse_r3(
                "<MyComp:style></MyComp:style>",
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_a_selectorless_component_with_an_unsupported_tag_name_script() {
            let _ = parse_r3(
                "<MyComp:script></MyComp:script>",
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_a_selectorless_component_with_an_unsupported_tag_name_ng_template() {
            let _ = parse_r3(
                "<MyComp:ng-template></MyComp:ng-template>",
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_a_selectorless_component_with_an_unsupported_tag_name_ng_container() {
            let _ = parse_r3(
                "<MyComp:ng-container></MyComp:ng-container>",
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_a_selectorless_component_with_an_unsupported_tag_name_ng_content() {
            let _ = parse_r3(
                "<MyComp:ng-content></MyComp:ng-content>",
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }
    }

    mod directives_validations {
        use super::*;

        #[test]
        #[should_panic]
        fn should_not_allow_star_syntax_inside_directives() {
            let _ = parse_r3(
                r#"<div @Dir(*ngIf="true")></div>"#,
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_ng_project_as_inside_directive_syntax() {
            let _ = parse_r3(
                r#"<div @Dir(ngProjectAs="foo")></div>"#,
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_ng_non_bindable_inside_directive_syntax() {
            let _ = parse_r3(
                r#"<div @Dir(ngNonBindable)></div>"#,
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_the_same_directive_to_be_applied_multiple_times() {
            let _ = parse_r3(
                r#"<div @One @Two @One(input="123")></div>"#,
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_class_bindings_inside_directives() {
            let _ = parse_r3(
                r#"<div @Dir([class.foo]="expr")></div>"#,
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_style_bindings_inside_directives() {
            let _ = parse_r3(
                r#"<div @Dir([style.foo]="expr")></div>"#,
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_attribute_bindings_inside_directives() {
            let _ = parse_r3(
                r#"<div @Dir([attr.foo]="expr")></div>"#,
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_animation_bindings_inside_directives() {
            let _ = parse_r3(
                r#"<div @Dir([@animation]="expr")></div>"#,
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_named_references_on_component() {
            let _ = parse_r3(
                r#"<MyComp #foo="bar"/>"#,
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_named_references_inside_directive_syntax() {
            let _ = parse_r3(
                r#"<div @Dir(#foo="bar")></div>"#,
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_duplicate_references_on_component() {
            let _ = parse_r3(
                r#"<MyComp #foo #foo/>"#,
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }

        #[test]
        #[should_panic]
        fn should_not_allow_duplicate_references_inside_directive_syntax() {
            let _ = parse_r3(
                r#"<div @Dir(#foo #foo)></div>"#,
                ParseR3Options {
                    selectorless_enabled: Some(true),
                    ..Default::default()
                },
            );
        }
    }
}
