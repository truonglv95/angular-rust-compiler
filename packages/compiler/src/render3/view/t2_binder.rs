//! Render3 T2 Binder
//!
//! Corresponds to packages/compiler/src/render3/view/t2_binder.ts
//! Contains template binding and directive matching logic

use std::collections::{HashMap, HashSet};

use crate::directive_matching::SelectorMatcher;
use crate::expression_parser::ast::{PropertyRead, SafePropertyRead, AST};
use crate::render3::r3_ast::{self as t, DeferredBlock, Element, Reference, Template};

use super::t2_api::{
    BoundTarget, DirectiveMeta, DirectiveOwner, InputOutputPropertySet,
    LegacyAnimationTriggerNames, ReferenceTarget, ScopedNode, Target, TargetBinder, TemplateEntity,
};
use super::util::create_css_selector_from_node;

/// Computes a difference between two lists.
fn diff(full_list: &[String], items_to_exclude: &[String]) -> Vec<String> {
    let exclude: HashSet<&String> = items_to_exclude.iter().collect();
    full_list
        .iter()
        .filter(|item| !exclude.contains(item))
        .cloned()
        .collect()
}

/// Fake DirectiveMeta implementation for find_matching_directives_and_pipes
#[derive(Debug, Clone)]
struct FakeDirectiveMeta {
    selector_str: String,
}

impl DirectiveMeta for FakeDirectiveMeta {
    fn name(&self) -> &str {
        &self.selector_str
    }

    fn selector(&self) -> Option<&str> {
        Some(&self.selector_str)
    }

    fn is_component(&self) -> bool {
        false
    }

    fn inputs(&self) -> &dyn InputOutputPropertySet {
        &EmptyPropertySet
    }

    fn outputs(&self) -> &dyn InputOutputPropertySet {
        &EmptyPropertySet
    }

    fn export_as(&self) -> Option<&[String]> {
        None
    }

    fn is_structural(&self) -> bool {
        false
    }

    fn ng_content_selectors(&self) -> Option<&[String]> {
        None
    }

    fn preserve_whitespaces(&self) -> bool {
        false
    }

    fn animation_trigger_names(&self) -> Option<&LegacyAnimationTriggerNames> {
        None
    }
}

/// Empty property set implementation
struct EmptyPropertySet;

impl InputOutputPropertySet for EmptyPropertySet {
    fn has_binding_property_name(&self, _property_name: &str) -> bool {
        false
    }
}

/// Object used to match template nodes to directives.
pub enum DirectiveMatcher<DirectiveT: DirectiveMeta> {
    Selector(SelectorMatcher<Vec<DirectiveT>>),
    Selectorless(crate::directive_matching::SelectorlessMatcher<DirectiveT>),
}

/// Result of finding matching directives and pipes.
#[derive(Debug, Clone)]
pub struct MatchingDirectivesAndPipes {
    pub directives: MatchingResult,
    pub pipes: MatchingResult,
}

#[derive(Debug, Clone)]
pub struct MatchingResult {
    pub regular: Vec<String>,
    pub defer_candidates: Vec<String>,
}

/// Find matching directives and pipes in a template.
pub fn find_matching_directives_and_pipes(
    template: &str,
    directive_selectors: &[String],
) -> MatchingDirectivesAndPipes {
    use super::template::parse_template;
    use crate::directive_matching::CssSelector;

    // Create a SelectorMatcher and add fake directives for each selector
    let mut matcher = SelectorMatcher::<Vec<FakeDirectiveMeta>>::new();
    for selector in directive_selectors {
        // Create a fake directive for matching
        let fake_directive = FakeDirectiveMeta {
            selector_str: selector.clone(),
        };

        // Parse the selector and add to matcher
        if let Ok(css_selectors) = CssSelector::parse(selector) {
            for css_selector in css_selectors {
                matcher.add_selectable(css_selector, vec![fake_directive.clone()]);
            }
        }
    }

    // Parse the template
    let parsed_template = parse_template(template, "", Default::default());

    // Create binder with matcher
    let binder = R3TargetBinder::new(Some(DirectiveMatcher::Selector(matcher)));
    let target = Target {
        template: Some(parsed_template.nodes),
        host: None,
    };

    // Bind the template
    let bound = binder.bind(target);

    // Extract directives and pipes
    let eager_directives = bound.get_eagerly_used_directives();
    let all_directives = bound.get_used_directives();
    let eager_pipes = bound.get_eagerly_used_pipes();
    let all_pipes = bound.get_used_pipes();

    // Extract selector strings
    let eager_directive_selectors: Vec<String> = eager_directives
        .iter()
        .filter_map(|d| d.selector().map(String::from))
        .collect();
    let all_directive_selectors: Vec<String> = all_directives
        .iter()
        .filter_map(|d| d.selector().map(String::from))
        .collect();

    MatchingDirectivesAndPipes {
        directives: MatchingResult {
            regular: eager_directive_selectors.clone(),
            defer_candidates: diff(&all_directive_selectors, &eager_directive_selectors),
        },
        pipes: MatchingResult {
            regular: eager_pipes.clone(),
            defer_candidates: diff(&all_pipes, &eager_pipes),
        },
    }
}

/// Processes `Target`s with a given set of directives and performs binding.
pub struct R3TargetBinder<DirectiveT: DirectiveMeta + Clone> {
    directive_matcher: Option<DirectiveMatcher<DirectiveT>>,
}

impl<DirectiveT: DirectiveMeta + Clone + 'static> R3TargetBinder<DirectiveT> {
    pub fn new(directive_matcher: Option<DirectiveMatcher<DirectiveT>>) -> Self {
        R3TargetBinder { directive_matcher }
    }
}

// Helper to extract pipes from AST
fn extract_pipes_from_ast(ast: &AST, pipes: &mut HashSet<String>) {
    match ast {
        AST::BindingPipe(pipe) => {
            pipes.insert(pipe.name.clone());
            extract_pipes_from_ast(&pipe.exp, pipes);
            for arg in &pipe.args {
                extract_pipes_from_ast(arg, pipes);
            }
        }
        AST::Binary(b) => {
            extract_pipes_from_ast(&b.left, pipes);
            extract_pipes_from_ast(&b.right, pipes);
        }
        AST::Chain(c) => {
            for expr in &c.expressions {
                extract_pipes_from_ast(expr, pipes);
            }
        }
        AST::Conditional(c) => {
            extract_pipes_from_ast(&c.condition, pipes);
            extract_pipes_from_ast(&c.true_exp, pipes);
            extract_pipes_from_ast(&c.false_exp, pipes);
        }
        AST::PropertyRead(p) => {
            extract_pipes_from_ast(&p.receiver, pipes);
        }
        AST::SafePropertyRead(p) => {
            extract_pipes_from_ast(&p.receiver, pipes);
        }
        AST::KeyedRead(k) => {
            extract_pipes_from_ast(&k.receiver, pipes);
            extract_pipes_from_ast(&k.key, pipes);
        }
        AST::SafeKeyedRead(k) => {
            extract_pipes_from_ast(&k.receiver, pipes);
            extract_pipes_from_ast(&k.key, pipes);
        }
        AST::LiteralArray(a) => {
            for expr in &a.expressions {
                extract_pipes_from_ast(expr, pipes);
            }
        }
        AST::LiteralMap(m) => {
            for value in &m.values {
                extract_pipes_from_ast(value, pipes);
            }
        }
        AST::Interpolation(i) => {
            for expr in &i.expressions {
                extract_pipes_from_ast(expr, pipes);
            }
        }
        AST::Call(c) => {
            extract_pipes_from_ast(&c.receiver, pipes);
            for arg in &c.args {
                extract_pipes_from_ast(arg, pipes);
            }
        }
        AST::SafeCall(c) => {
            extract_pipes_from_ast(&c.receiver, pipes);
            for arg in &c.args {
                extract_pipes_from_ast(arg, pipes);
            }
        }
        AST::PrefixNot(p) => {
            extract_pipes_from_ast(&p.expression, pipes);
        }
        AST::Unary(u) => {
            extract_pipes_from_ast(&u.expr, pipes);
        }
        AST::TypeofExpression(t) => {
            extract_pipes_from_ast(&t.expression, pipes);
        }
        AST::VoidExpression(v) => {
            extract_pipes_from_ast(&v.expression, pipes);
        }
        AST::NonNullAssert(n) => {
            extract_pipes_from_ast(&n.expression, pipes);
        }
        AST::ParenthesizedExpression(p) => {
            extract_pipes_from_ast(&p.expression, pipes);
        }
        AST::PropertyWrite(p) => {
            extract_pipes_from_ast(&p.receiver, pipes);
            extract_pipes_from_ast(&p.value, pipes);
        }
        AST::KeyedWrite(k) => {
            extract_pipes_from_ast(&k.receiver, pipes);
            extract_pipes_from_ast(&k.key, pipes);
            extract_pipes_from_ast(&k.value, pipes);
        }
        AST::TemplateLiteral(t) => {
            for expr in &t.expressions {
                extract_pipes_from_ast(expr, pipes);
            }
        }
        AST::TaggedTemplateLiteral(t) => {
            extract_pipes_from_ast(&t.tag, pipes);
            for expr in &t.template.expressions {
                extract_pipes_from_ast(expr, pipes);
            }
        }
        _ => {}
    }
}

// Helper to extract pipes from R3Node
fn extract_pipes_from_node(
    node: &t::R3Node,
    pipes: &mut HashSet<String>,
    is_deferred: bool,
    eager_pipes: &mut HashSet<String>,
) {
    match node {
        t::R3Node::BoundAttribute(attr) => {
            extract_pipes_from_ast(&attr.value, pipes);
            if !is_deferred {
                extract_pipes_from_ast(&attr.value, eager_pipes);
            }
        }
        t::R3Node::BoundEvent(event) => {
            extract_pipes_from_ast(&event.handler, pipes);
            if !is_deferred {
                extract_pipes_from_ast(&event.handler, eager_pipes);
            }
        }
        t::R3Node::BoundText(text) => {
            extract_pipes_from_ast(&text.value, pipes);
            if !is_deferred {
                extract_pipes_from_ast(&text.value, eager_pipes);
            }
        }
        t::R3Node::Element(el) => {
            for input in &el.inputs {
                extract_pipes_from_ast(&input.value, pipes);
                if !is_deferred {
                    extract_pipes_from_ast(&input.value, eager_pipes);
                }
            }
            for output in &el.outputs {
                extract_pipes_from_ast(&output.handler, pipes);
                if !is_deferred {
                    extract_pipes_from_ast(&output.handler, eager_pipes);
                }
            }
            for child in &el.children {
                extract_pipes_from_node(child, pipes, is_deferred, eager_pipes);
            }
        }
        t::R3Node::Template(tmpl) => {
            for input in &tmpl.inputs {
                extract_pipes_from_ast(&input.value, pipes);
                if !is_deferred {
                    extract_pipes_from_ast(&input.value, eager_pipes);
                }
            }
            for output in &tmpl.outputs {
                extract_pipes_from_ast(&output.handler, pipes);
                if !is_deferred {
                    extract_pipes_from_ast(&output.handler, eager_pipes);
                }
            }
            for child in &tmpl.children {
                extract_pipes_from_node(child, pipes, is_deferred, eager_pipes);
            }
        }
        t::R3Node::Component(comp) => {
            for input in &comp.inputs {
                extract_pipes_from_ast(&input.value, pipes);
                if !is_deferred {
                    extract_pipes_from_ast(&input.value, eager_pipes);
                }
            }
            for output in &comp.outputs {
                extract_pipes_from_ast(&output.handler, pipes);
                if !is_deferred {
                    extract_pipes_from_ast(&output.handler, eager_pipes);
                }
            }
            for child in &comp.children {
                extract_pipes_from_node(child, pipes, is_deferred, eager_pipes);
            }
        }
        t::R3Node::DeferredBlock(deferred) => {
            // Process main content (deferred)
            for child in &deferred.children {
                extract_pipes_from_node(child, pipes, true, eager_pipes);
            }
            // Process placeholder, loading, error (eager)
            if let Some(ref placeholder) = deferred.placeholder {
                for child in &placeholder.children {
                    extract_pipes_from_node(child, pipes, false, eager_pipes);
                }
            }
            if let Some(ref loading) = deferred.loading {
                for child in &loading.children {
                    extract_pipes_from_node(child, pipes, false, eager_pipes);
                }
            }
            if let Some(ref error) = deferred.error {
                for child in &error.children {
                    extract_pipes_from_node(child, pipes, false, eager_pipes);
                }
            }
        }

        t::R3Node::IfBlock(if_block) => {
            for branch in &if_block.branches {
                for child in &branch.children {
                    extract_pipes_from_node(child, pipes, is_deferred, eager_pipes);
                }
            }
        }
        t::R3Node::ForLoopBlock(for_loop) => {
            for child in &for_loop.children {
                extract_pipes_from_node(child, pipes, is_deferred, eager_pipes);
            }
            if let Some(ref empty) = for_loop.empty {
                for child in &empty.children {
                    extract_pipes_from_node(child, pipes, is_deferred, eager_pipes);
                }
            }
        }
        t::R3Node::SwitchBlock(switch) => {
            for case in &switch.cases {
                for child in &case.children {
                    extract_pipes_from_node(child, pipes, is_deferred, eager_pipes);
                }
            }
        }
        _ => {
            // Skip other node types that don't contain pipes
        }
    }
}

fn get_node_children(node: &t::R3Node) -> Option<Vec<t::R3Node>> {
    match node {
        t::R3Node::Content(content) => Some(content.children.clone()),
        t::R3Node::IfBlock(if_block) => {
            let mut children = vec![];
            for branch in &if_block.branches {
                children.extend(branch.children.clone());
            }
            Some(children)
        }
        t::R3Node::IfBlockBranch(branch) => Some(branch.children.clone()),
        t::R3Node::SwitchBlock(switch) => {
            let mut children = vec![];
            for case in &switch.cases {
                children.extend(case.children.clone());
            }
            Some(children)
        }
        t::R3Node::SwitchBlockCase(case) => Some(case.children.clone()),
        t::R3Node::ForLoopBlock(for_loop) => {
            let mut children = for_loop.children.clone();
            if let Some(ref empty) = for_loop.empty {
                children.extend(empty.children.clone());
            }
            Some(children)
        }
        t::R3Node::ForLoopBlockEmpty(empty) => Some(empty.children.clone()),
        t::R3Node::DeferredBlock(deferred) => {
            let mut children = deferred.children.clone();
            // Extend with sub-block children directly, don't wrap as R3Node variants
            if let Some(ref placeholder) = deferred.placeholder {
                children.extend(placeholder.children.clone());
            }
            if let Some(ref loading) = deferred.loading {
                children.extend(loading.children.clone());
            }
            if let Some(ref error) = deferred.error {
                children.extend(error.children.clone());
            }
            Some(children)
        }
        t::R3Node::DeferredBlockPlaceholder(placeholder) => Some(placeholder.children.clone()),
        t::R3Node::DeferredBlockLoading(loading) => Some(loading.children.clone()),
        t::R3Node::DeferredBlockError(error) => Some(error.children.clone()),
        _ => None,
    }
}

// Helper to extract defer blocks
fn extract_defer_blocks(nodes: &[t::R3Node]) -> Vec<DeferredBlock> {
    let mut blocks = vec![];
    for node in nodes {
        match node {
            t::R3Node::DeferredBlock(block) => {
                blocks.push((**block).clone());
                // Also check nested blocks
                blocks.extend(extract_defer_blocks(&block.children));
                if let Some(ref placeholder) = block.placeholder {
                    blocks.extend(extract_defer_blocks(&placeholder.children));
                }
                if let Some(ref loading) = block.loading {
                    blocks.extend(extract_defer_blocks(&loading.children));
                }
                if let Some(ref error) = block.error {
                    blocks.extend(extract_defer_blocks(&error.children));
                }
            }
            t::R3Node::Element(el) => {
                blocks.extend(extract_defer_blocks(&el.children));
            }
            t::R3Node::Template(tmpl) => {
                blocks.extend(extract_defer_blocks(&tmpl.children));
            }
            t::R3Node::IfBlock(if_block) => {
                for branch in &if_block.branches {
                    blocks.extend(extract_defer_blocks(&branch.children));
                }
            }
            t::R3Node::ForLoopBlock(for_loop) => {
                blocks.extend(extract_defer_blocks(&for_loop.children));
                if let Some(ref empty) = for_loop.empty {
                    blocks.extend(extract_defer_blocks(&empty.children));
                }
            }
            t::R3Node::SwitchBlock(switch) => {
                for case in &switch.cases {
                    blocks.extend(extract_defer_blocks(&case.children));
                }
            }
            _ => {
                // Skip other node types
            }
        }
    }
    blocks
}

impl<DirectiveT: DirectiveMeta + Clone + 'static> TargetBinder<DirectiveT>
    for R3TargetBinder<DirectiveT>
{
    fn bind(&self, target: Target<DirectiveT>) -> Box<dyn BoundTarget<DirectiveT>> {
        if target.template.is_none() && target.host.is_none() {
            panic!("Empty bound targets are not supported");
        }

        let mut directives_map: HashMap<DirectiveOwnerWrapper, Vec<DirectiveT>> = HashMap::new();
        let mut eager_directives: Vec<DirectiveT> = vec![];
        let missing_directives: HashSet<String> = HashSet::new();
        let mut bindings: HashMap<BindingKey, BindingTarget<DirectiveT>> = HashMap::new();
        let mut references_map: HashMap<ReferenceKey, ReferenceTargetInternal<DirectiveT>> =
            HashMap::new();
        let mut scoped_node_entities: HashMap<ScopedNodeWrapper, Vec<TemplateEntity>> =
            HashMap::new();
        let mut expressions_map: HashMap<ExprKey, TemplateEntity> = HashMap::new();
        let mut symbols_map: HashMap<EntityKey, ScopedNodeWrapper> = HashMap::new();
        let mut nesting_level: HashMap<ScopedNodeWrapper, usize> = HashMap::new();
        let mut scoped_nodes_by_span: HashMap<String, ScopedNode> = HashMap::new();
        let mut used_pipes: HashSet<String> = HashSet::new();
        let mut eager_pipes: HashSet<String> = HashSet::new();
        let mut defer_blocks: Vec<DeferredBlock> = vec![];

        // Extract defer blocks
        if let Some(ref template_nodes) = target.template {
            defer_blocks = extract_defer_blocks(template_nodes);

            // Extract pipes
            for node in template_nodes {
                extract_pipes_from_node(node, &mut used_pipes, false, &mut eager_pipes);
            }

            // Build scope and match directives
            if let Some(ref matcher) = self.directive_matcher {
                match_directives_in_template(
                    template_nodes,
                    matcher,
                    &mut directives_map,
                    &mut eager_directives,
                    &mut bindings,
                    &mut references_map,
                    false,
                );
            }

            // Extract entities from template
            extract_entities_from_template(
                template_nodes,
                &mut scoped_node_entities,
                &mut expressions_map,
                &mut symbols_map,
                &mut nesting_level,
                &mut scoped_nodes_by_span,
                0,
            );
        }

        Box::new(R3BoundTarget {
            target,
            directives_map,
            eager_directives,
            missing_directives,
            bindings,
            references_map,
            expressions_map,
            symbols_map,
            scoped_nodes_by_span,
            nesting_level,
            scoped_node_entities,
            used_pipes,
            eager_pipes,
            defer_blocks,
        })
    }
}

// Helper types for HashMap keys
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct DirectiveOwnerWrapper {
    key: String,
}

impl DirectiveOwnerWrapper {
    fn from(owner: &DirectiveOwner) -> Self {
        let key = match owner {
            DirectiveOwner::Element(el) => {
                format!("Element:{}:{}", el.name, el.source_span.start.offset)
            }
            DirectiveOwner::Template(tmpl) => format!(
                "Template:{:?}:{}",
                tmpl.tag_name, tmpl.source_span.start.offset
            ),
            DirectiveOwner::Component(comp) => format!(
                "Component:{}:{}",
                comp.component_name, comp.source_span.start.offset
            ),
            DirectiveOwner::Directive(dir) => {
                format!("Directive:{}:{}", dir.name, dir.source_span.start.offset)
            }
            DirectiveOwner::HostElement(_host) => format!("HostElement"),
        };
        DirectiveOwnerWrapper { key }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct BindingKey {
    key: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ReferenceKey {
    key: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ExprKey {
    key: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct EntityKey {
    key: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ScopedNodeWrapper {
    key: String,
}

// Match directives in template
fn match_directives_in_template<DirectiveT: DirectiveMeta + Clone>(
    nodes: &[t::R3Node],
    matcher: &DirectiveMatcher<DirectiveT>,
    directives_map: &mut HashMap<DirectiveOwnerWrapper, Vec<DirectiveT>>,
    eager_directives: &mut Vec<DirectiveT>,
    bindings: &mut HashMap<BindingKey, BindingTarget<DirectiveT>>,
    references_map: &mut HashMap<ReferenceKey, ReferenceTargetInternal<DirectiveT>>,
    is_deferred: bool,
) {
    let _ = references_map; // TODO: Implement reference resolution
    for node in nodes {
        match node {
            t::R3Node::Element(el) => {
                match matcher {
                    DirectiveMatcher::Selector(selector_matcher) => {
                        if let Some(css_selector) =
                            create_css_selector_from_node(&t::R3Node::Element(el.clone()))
                        {
                            let mut matched_directives = vec![];
                            selector_matcher.match_selector(&css_selector, |_, results| {
                                matched_directives.extend(results.clone());
                            });
                            if !matched_directives.is_empty() {
                                let owner = DirectiveOwnerWrapper::from(&DirectiveOwner::Element(
                                    (**el).clone(),
                                ));
                                directives_map.insert(owner.clone(), matched_directives.clone());
                                if !is_deferred {
                                    eager_directives.extend(matched_directives.clone());
                                }

                                // Track bindings for inputs/outputs/attributes
                                track_element_bindings(
                                    &el.inputs,
                                    &el.outputs,
                                    &el.attributes,
                                    &matched_directives,
                                    bindings,
                                    &DirectiveOwner::Element((**el).clone()),
                                );
                            }
                        }
                    }
                    DirectiveMatcher::Selectorless(_) => {
                        // For selectorless, bindings are tracked via directives on the element
                    }
                }
                // Recursively process children
                for child in &el.children {
                    match_directives_in_template(
                        &[child.clone()],
                        matcher,
                        directives_map,
                        eager_directives,
                        bindings,
                        references_map,
                        is_deferred,
                    );
                }
            }
            t::R3Node::Template(tmpl) => {
                match matcher {
                    DirectiveMatcher::Selector(selector_matcher) => {
                        if let Some(css_selector) =
                            create_css_selector_from_node(&t::R3Node::Template(tmpl.clone()))
                        {
                            let mut matched_directives = vec![];
                            selector_matcher.match_selector(&css_selector, |_, results| {
                                matched_directives.extend(results.clone());
                            });
                            if !matched_directives.is_empty() {
                                let owner = DirectiveOwnerWrapper::from(&DirectiveOwner::Template(
                                    (**tmpl).clone(),
                                ));
                                directives_map.insert(owner.clone(), matched_directives.clone());
                                if !is_deferred {
                                    eager_directives.extend(matched_directives.clone());
                                }

                                // Track bindings
                                track_element_bindings(
                                    &tmpl.inputs,
                                    &tmpl.outputs,
                                    &tmpl.attributes,
                                    &matched_directives,
                                    bindings,
                                    &DirectiveOwner::Template((**tmpl).clone()),
                                );
                            }
                        }
                    }
                    DirectiveMatcher::Selectorless(_) => {
                        // For selectorless, references on templates are handled differently
                        for ref_node in &tmpl.references {
                            if ref_node.value.trim().is_empty() {
                                // Track empty references to template
                                let key = ReferenceKey {
                                    key: format!("Reference:{}", ref_node.name),
                                };
                                references_map.insert(key, ReferenceTargetInternal::Template(0));
                                // TODO: Store template reference properly
                            }
                        }
                    }
                }
                // Process directives on template (directives are handled in visitDirective)
                // Recursively process children
                for child in &tmpl.children {
                    match_directives_in_template(
                        &[child.clone()],
                        matcher,
                        directives_map,
                        eager_directives,
                        bindings,
                        references_map,
                        is_deferred,
                    );
                }
            }
            t::R3Node::Component(comp) => {
                match matcher {
                    DirectiveMatcher::Selectorless(selectorless_matcher) => {
                        let matched = selectorless_matcher.match_name(&comp.component_name);
                        if !matched.is_empty() {
                            let owner = DirectiveOwnerWrapper::from(&DirectiveOwner::Component(
                                (**comp).clone(),
                            ));
                            directives_map.insert(owner, matched.clone());
                            if !is_deferred {
                                eager_directives.extend(matched);
                            }
                        }
                    }
                    _ => {}
                }
                // Recursively process children
                for child in &comp.children {
                    match_directives_in_template(
                        &[child.clone()],
                        matcher,
                        directives_map,
                        eager_directives,
                        bindings,
                        references_map,
                        is_deferred,
                    );
                }
            }
            t::R3Node::Directive(dir) => {
                if let DirectiveMatcher::Selectorless(selectorless_matcher) = matcher {
                    let matched = selectorless_matcher.match_name(&dir.name);
                    if !matched.is_empty() {
                        let owner = DirectiveOwnerWrapper::from(&DirectiveOwner::Directive(
                            (**dir).clone(),
                        ));
                        directives_map.insert(owner, matched.clone());
                        if !is_deferred {
                            eager_directives.extend(matched);
                        }
                    }
                }
                // Note: Directives don't have children in the same way - they're applied to elements
            }
            t::R3Node::DeferredBlock(deferred) => {
                // Explicitly handle DeferredBlock to avoid get_node_children re-wrapping
                match_directives_in_template(
                    &deferred.children,
                    matcher,
                    directives_map,
                    eager_directives,
                    bindings,
                    references_map,
                    true,
                );
                if let Some(ref placeholder) = deferred.placeholder {
                    match_directives_in_template(
                        &placeholder.children,
                        matcher,
                        directives_map,
                        eager_directives,
                        bindings,
                        references_map,
                        false,
                    );
                }
                if let Some(ref loading) = deferred.loading {
                    match_directives_in_template(
                        &loading.children,
                        matcher,
                        directives_map,
                        eager_directives,
                        bindings,
                        references_map,
                        false,
                    );
                }
                if let Some(ref error) = deferred.error {
                    match_directives_in_template(
                        &error.children,
                        matcher,
                        directives_map,
                        eager_directives,
                        bindings,
                        references_map,
                        false,
                    );
                }
            }
            t::R3Node::DeferredBlockPlaceholder(placeholder) => {
                match_directives_in_template(
                    &placeholder.children,
                    matcher,
                    directives_map,
                    eager_directives,
                    bindings,
                    references_map,
                    false,
                );
            }
            t::R3Node::DeferredBlockLoading(loading) => {
                match_directives_in_template(
                    &loading.children,
                    matcher,
                    directives_map,
                    eager_directives,
                    bindings,
                    references_map,
                    false,
                );
            }
            t::R3Node::DeferredBlockError(error) => {
                match_directives_in_template(
                    &error.children,
                    matcher,
                    directives_map,
                    eager_directives,
                    bindings,
                    references_map,
                    false,
                );
            }
            t::R3Node::IfBlock(if_block) => {
                // Explicitly handle IfBlock to recurse into branches
                for branch in &if_block.branches {
                    match_directives_in_template(
                        &branch.children,
                        matcher,
                        directives_map,
                        eager_directives,
                        bindings,
                        references_map,
                        is_deferred,
                    );
                }
            }
            t::R3Node::ForLoopBlock(for_block) => {
                // Explicitly handle ForLoopBlock to recurse into main and empty blocks
                match_directives_in_template(
                    &for_block.children,
                    matcher,
                    directives_map,
                    eager_directives,
                    bindings,
                    references_map,
                    is_deferred,
                );
                if let Some(ref empty) = for_block.empty {
                    match_directives_in_template(
                        &empty.children,
                        matcher,
                        directives_map,
                        eager_directives,
                        bindings,
                        references_map,
                        is_deferred,
                    );
                }
            }
            t::R3Node::SwitchBlock(switch_block) => {
                // Explicitly handle SwitchBlock to recurse into cases
                for case in &switch_block.cases {
                    match_directives_in_template(
                        &case.children,
                        matcher,
                        directives_map,
                        eager_directives,
                        bindings,
                        references_map,
                        is_deferred,
                    );
                }
            }
            _ => {
                // Skip other nodes that don't have children or are not relevant
            }
        }
    }
}

// Helper to set a binding - if directive matches, use directive, otherwise use node
fn set_attribute_binding<DirectiveT: DirectiveMeta + Clone>(
    attr_name: &str,
    source_span: &crate::parse_util::ParseSourceSpan,
    key_type: &str,
    directives: &[DirectiveT],
    bindings: &mut HashMap<BindingKey, BindingTarget<DirectiveT>>,
    node: &DirectiveOwner,
) {
    let dir = if key_type == "BoundAttribute" {
        directives
            .iter()
            .find(|d| d.inputs().has_binding_property_name(attr_name))
    } else {
        directives
            .iter()
            .find(|d| d.outputs().has_binding_property_name(attr_name))
    };

    let key = BindingKey {
        key: format!("{}:{}:{:?}", key_type, attr_name, source_span),
    };

    if let Some(dir) = dir {
        bindings.insert(key, BindingTarget::Directive(dir.clone()));
    } else {
        // No directive matches - bind to the node itself
        match node {
            DirectiveOwner::Element(el) => {
                bindings.insert(key, BindingTarget::Element(el.clone()));
            }
            DirectiveOwner::Template(tmpl) => {
                bindings.insert(key, BindingTarget::Template(tmpl.clone()));
            }
            _ => {
                // Component/Directive/HostElement - skip for now
            }
        }
    }
}

// Track bindings for selector-based directive matching
fn track_element_bindings<DirectiveT: DirectiveMeta + Clone>(
    inputs: &[t::BoundAttribute],
    outputs: &[t::BoundEvent],
    attributes: &[t::TextAttribute],
    directives: &[DirectiveT],
    bindings: &mut HashMap<BindingKey, BindingTarget<DirectiveT>>,
    node: &DirectiveOwner,
) {
    // Track input bindings (bound attributes)
    for input in inputs {
        set_attribute_binding(
            &input.name,
            &input.source_span,
            "BoundAttribute",
            directives,
            bindings,
            node,
        );
    }

    // Track output bindings (bound events)
    for output in outputs {
        set_attribute_binding(
            &output.name,
            &output.source_span,
            "BoundEvent",
            directives,
            bindings,
            node,
        );
    }

    // Track text attribute bindings (can be inputs)
    for attr in attributes {
        // Check if any directive claims this attribute as input
        if let Some(dir) = directives
            .iter()
            .find(|d| d.inputs().has_binding_property_name(&attr.name))
        {
            let key = BindingKey {
                key: format!("TextAttribute:{}:{:?}", attr.name, attr.source_span),
            };
            bindings.insert(key, BindingTarget::Directive(dir.clone()));
        } else {
            // No directive matches - bind to the node itself
            let key = BindingKey {
                key: format!("TextAttribute:{}:{:?}", attr.name, attr.source_span),
            };
            match node {
                DirectiveOwner::Element(el) => {
                    bindings.insert(key, BindingTarget::Element(el.clone()));
                }
                DirectiveOwner::Template(tmpl) => {
                    bindings.insert(key, BindingTarget::Template(tmpl.clone()));
                }
                _ => {}
            }
        }
    }
}

// Extract entities (variables, references, let declarations) from template and map expressions
fn extract_entities_from_template(
    nodes: &[t::R3Node],
    scoped_node_entities: &mut HashMap<ScopedNodeWrapper, Vec<TemplateEntity>>,
    expressions_map: &mut HashMap<ExprKey, TemplateEntity>,
    symbols_map: &mut HashMap<EntityKey, ScopedNodeWrapper>,
    nesting_level: &mut HashMap<ScopedNodeWrapper, usize>,
    scoped_nodes_by_span: &mut HashMap<String, ScopedNode>,
    current_level: usize,
) {
    // Build a scope to lookup entities
    let scope = Scope::apply(nodes);
    let mut entities_vec = Vec::new();
    extract_entities_recursive(nodes, &mut entities_vec);
    // Store root scope entities (None represented as empty key)
    scoped_node_entities.insert(
        ScopedNodeWrapper {
            key: "root".to_string(),
        },
        entities_vec,
    );

    // Visit all nodes and extract expressions, also populate nesting level and scoped node entities
    let root_wrapper = ScopedNodeWrapper {
        key: "root".to_string(),
    };
    visit_expressions_in_template(
        nodes,
        &scope,
        expressions_map,
        symbols_map,
        nesting_level,
        scoped_nodes_by_span,
        scoped_node_entities,
        current_level,
        root_wrapper,
    );
}

// Visit expressions in template and map PropertyRead/SafePropertyRead to TemplateEntity
fn visit_expressions_in_template(
    nodes: &[t::R3Node],
    scope: &Scope,
    expressions_map: &mut HashMap<ExprKey, TemplateEntity>,
    symbols_map: &mut HashMap<EntityKey, ScopedNodeWrapper>,
    nesting_level: &mut HashMap<ScopedNodeWrapper, usize>,
    scoped_nodes_by_span: &mut HashMap<String, ScopedNode>,
    scoped_node_entities: &mut HashMap<ScopedNodeWrapper, Vec<TemplateEntity>>,
    current_level: usize,
    current_scope: ScopedNodeWrapper,
) {
    for node in nodes {
        match node {
            t::R3Node::BoundAttribute(attr) => {
                visit_ast_expressions(&attr.value, scope, expressions_map);
            }
            t::R3Node::BoundEvent(event) => {
                visit_ast_expressions(&event.handler, scope, expressions_map);
            }
            t::R3Node::BoundText(text) => {
                visit_ast_expressions(&text.value, scope, expressions_map);
            }
            t::R3Node::Element(el) => {
                for input in &el.inputs {
                    visit_ast_expressions(&input.value, scope, expressions_map);
                }
                for output in &el.outputs {
                    visit_ast_expressions(&output.handler, scope, expressions_map);
                }
                visit_expressions_in_template(
                    &el.children,
                    scope,
                    expressions_map,
                    symbols_map,
                    nesting_level,
                    scoped_nodes_by_span,
                    scoped_node_entities,
                    current_level,
                    current_scope.clone(),
                );
            }
            t::R3Node::Template(tmpl) => {
                // Store Template as ScopedNode for nesting level and symbol mapping
                let span_key = format!("{:?}", tmpl.source_span);
                let wrapper = ScopedNodeWrapper {
                    key: span_key.clone(),
                };
                scoped_nodes_by_span
                    .insert(span_key.clone(), ScopedNode::Template((**tmpl).clone()));
                nesting_level.insert(wrapper.clone(), current_level + 1);

                // Collect entities for this template scope
                let mut template_entities = Vec::new();
                for variable in &tmpl.variables {
                    template_entities.push(TemplateEntity::Variable(variable.clone()));
                    let key = EntityKey {
                        key: format!("Variable:{}", variable.name),
                    };
                    symbols_map.insert(key, wrapper.clone());
                }
                for reference in &tmpl.references {
                    template_entities.push(TemplateEntity::Reference(reference.clone()));
                    let key = EntityKey {
                        key: format!("Reference:{}", reference.name),
                    };
                    symbols_map.insert(key, wrapper.clone());
                }
                scoped_node_entities.insert(wrapper.clone(), template_entities);
                for input in &tmpl.inputs {
                    visit_ast_expressions(&input.value, scope, expressions_map);
                }
                for output in &tmpl.outputs {
                    visit_ast_expressions(&output.handler, scope, expressions_map);
                }
                visit_expressions_in_template(
                    &tmpl.children,
                    scope,
                    expressions_map,
                    symbols_map,
                    nesting_level,
                    scoped_nodes_by_span,
                    scoped_node_entities,
                    current_level + 1,
                    wrapper,
                );
            }
            t::R3Node::LetDeclaration(decl) => {
                // Track let declaration as symbol - scoped to current scope
                let key = EntityKey {
                    key: format!("LetDeclaration:{}", decl.name),
                };
                symbols_map.insert(key, current_scope.clone());

                // Add to entities of current scope
                if let Some(entities) = scoped_node_entities.get_mut(&current_scope) {
                    entities.push(TemplateEntity::LetDeclaration((*decl).as_ref().clone()));
                } else {
                    // This creates a new entry if somehow missing, though it should be initialized
                    // For root scope, it is initialized. For branches, it might not be if we didn't init it yet.
                    // But we init it when entering the node.
                    // EXCEPT: IfBlockBranch init logic was conditional on being non-empty.
                    // We must ensure it is initialized.
                    scoped_node_entities.insert(
                        current_scope.clone(),
                        vec![TemplateEntity::LetDeclaration((*decl).as_ref().clone())],
                    );
                }

                // Visit the value expression
                visit_ast_expressions(&decl.value, scope, expressions_map);
            }
            t::R3Node::ForLoopBlock(for_loop) => {
                // Store ForLoopBlock as ScopedNode
                let span_key = format!("{:?}", for_loop.block.source_span);
                let wrapper = ScopedNodeWrapper {
                    key: span_key.clone(),
                };
                scoped_nodes_by_span.insert(
                    span_key.clone(),
                    ScopedNode::ForLoopBlock((**for_loop).clone()),
                );
                nesting_level.insert(wrapper.clone(), current_level + 1);

                // Track item variable as symbol and entity
                let variable_entity = TemplateEntity::Variable(for_loop.item.clone());
                let key = EntityKey {
                    key: format!("Variable:{}", for_loop.item.name),
                };
                symbols_map.insert(key, wrapper.clone());

                // Collect entities for this for loop scope
                let for_loop_entities = vec![variable_entity];
                scoped_node_entities.insert(wrapper.clone(), for_loop_entities);

                // Visit expression (ASTWithSource has .ast field)
                visit_ast_expressions(&for_loop.expression.ast, scope, expressions_map);
                visit_expressions_in_template(
                    &for_loop.children,
                    scope,
                    expressions_map,
                    symbols_map,
                    nesting_level,
                    scoped_nodes_by_span,
                    scoped_node_entities,
                    current_level + 1,
                    wrapper,
                );
            }
            t::R3Node::IfBlockBranch(branch) => {
                let span_key = format!("{:?}", branch.block.source_span);
                let wrapper = ScopedNodeWrapper {
                    key: span_key.clone(),
                };
                scoped_nodes_by_span.insert(
                    span_key.clone(),
                    ScopedNode::IfBlockBranch((**branch).clone()),
                );
                nesting_level.insert(wrapper.clone(), current_level + 1);

                // Collect entities for this branch scope
                let mut branch_entities = Vec::new();

                // Track expression alias if present
                if let Some(ref alias) = branch.expression_alias {
                    branch_entities.push(TemplateEntity::Variable(alias.clone()));
                    let key = EntityKey {
                        key: format!("Variable:{}", alias.name),
                    };
                    symbols_map.insert(key, wrapper.clone());
                }

                // Always insert entities, even if empty initially, so we can append LetDeclaration later
                // Or insert if not present
                scoped_node_entities
                    .entry(wrapper.clone())
                    .or_insert(branch_entities);

                // Visit expression if present - Wait, expression is on branch?
                // t::IfBlockBranch has expression.
                if let Some(ref expr) = branch.expression {
                    visit_ast_expressions(expr, scope, expressions_map);
                }
                visit_expressions_in_template(
                    &branch.children,
                    scope,
                    expressions_map,
                    symbols_map,
                    nesting_level,
                    scoped_nodes_by_span,
                    scoped_node_entities,
                    current_level + 1,
                    wrapper,
                );
            }
            t::R3Node::SwitchBlockCase(case) => {
                // Store SwitchBlockCase as ScopedNode
                let span_key = format!("{:?}", case.block.source_span);
                let wrapper = ScopedNodeWrapper {
                    key: span_key.clone(),
                };
                scoped_nodes_by_span.insert(
                    span_key.clone(),
                    ScopedNode::SwitchBlockCase((**case).clone()),
                );
                nesting_level.insert(wrapper.clone(), current_level + 1);

                // Initialize entities
                scoped_node_entities
                    .entry(wrapper.clone())
                    .or_insert(Vec::new());

                // Visit expression if present
                if let Some(ref expr) = case.expression {
                    visit_ast_expressions(expr, scope, expressions_map);
                }
                visit_expressions_in_template(
                    &case.children,
                    scope,
                    expressions_map,
                    symbols_map,
                    nesting_level,
                    scoped_nodes_by_span,
                    scoped_node_entities,
                    current_level + 1,
                    wrapper,
                );
            }
            t::R3Node::SwitchBlock(switch) => {
                // Visit switch expression
                visit_ast_expressions(&switch.expression, scope, expressions_map);
                // Visit all cases - handled by SwitchBlockCase logic above?
                // No, cases are children but they are R3Nodes::SwitchBlockCase
                // And visit_expressions_in_template iterates them.
                // WE MUST manually iterate and call recursively because SwitchBlockCase is an R3Node that handles itself.
                // But current implementation loops.
                for case in &switch.cases {
                    // Wait, cases in switch.cases are structs, NOT R3Node enum?
                    // r3_ast::SwitchBlock { cases: Vec<SwitchBlockCase>, ... }
                    // SwitchBlockCase is a struct.
                    // So we must manually handle them here as if they were nodes, OR wrap logic.

                    // My previous view logic had:
                    // for case in &switch.cases { ... visit_expressions_in_template(&case.children, ...) }
                    // It did NOT recurse into 'case' node itself because 'case' is not a node variant in the loop?
                    // Ah, R3Node::SwitchBlockCase IS a variant.
                    // But R3Node::SwitchBlock has a Vec of SwitchBlockCase structs.
                    // Are children of SwitchBlock R3Nodes or SwitchBlockCase structs?
                    // They are structs.

                    // So I must keep the manual logic for SwitchBlock, but simulate the node behavior or just inline.
                    // I will INLINE logic for creating scope for case.

                    let span_key = format!("{:?}", case.block.source_span);
                    let wrapper = ScopedNodeWrapper {
                        key: span_key.clone(),
                    };
                    scoped_nodes_by_span
                        .insert(span_key.clone(), ScopedNode::SwitchBlockCase(case.clone()));
                    nesting_level.insert(wrapper.clone(), current_level + 1);

                    scoped_node_entities
                        .entry(wrapper.clone())
                        .or_insert(Vec::new());

                    if let Some(ref expr) = case.expression {
                        visit_ast_expressions(expr, scope, expressions_map);
                    }
                    visit_expressions_in_template(
                        &case.children,
                        scope,
                        expressions_map,
                        symbols_map,
                        nesting_level,
                        scoped_nodes_by_span,
                        scoped_node_entities,
                        current_level + 1,
                        wrapper,
                    );
                }
            }
            t::R3Node::IfBlock(if_block) => {
                // Visit all branches
                for branch in &if_block.branches {
                    // Logic similar to SwitchBlock - iterate explicitly
                    // Branch is struct.

                    let span_key = format!("{:?}", branch.block.source_span);
                    let wrapper = ScopedNodeWrapper {
                        key: span_key.clone(),
                    };
                    scoped_nodes_by_span
                        .insert(span_key.clone(), ScopedNode::IfBlockBranch(branch.clone()));
                    nesting_level.insert(wrapper.clone(), current_level + 1);

                    let mut branch_entities = Vec::new();
                    if let Some(ref alias) = branch.expression_alias {
                        branch_entities.push(TemplateEntity::Variable(alias.clone()));
                        let key = EntityKey {
                            key: format!("Variable:{}", alias.name),
                        };
                        symbols_map.insert(key, wrapper.clone());
                    }
                    scoped_node_entities
                        .entry(wrapper.clone())
                        .or_insert(branch_entities);

                    if let Some(ref expr) = branch.expression {
                        visit_ast_expressions(expr, scope, expressions_map);
                    }
                    visit_expressions_in_template(
                        &branch.children,
                        scope,
                        expressions_map,
                        symbols_map,
                        nesting_level,
                        scoped_nodes_by_span,
                        scoped_node_entities,
                        current_level + 1,
                        wrapper,
                    );
                }
            }
            t::R3Node::DeferredBlock(deferred) => {
                // Explicitly handle DeferredBlock to avoid get_node_children re-wrapping
                if !deferred.children.is_empty() {
                    visit_expressions_in_template(
                        &deferred.children,
                        scope,
                        expressions_map,
                        symbols_map,
                        nesting_level,
                        scoped_nodes_by_span,
                        scoped_node_entities,
                        current_level,
                        current_scope.clone(),
                    );
                }
                if let Some(ref placeholder) = deferred.placeholder {
                    if !placeholder.children.is_empty() {
                        visit_expressions_in_template(
                            &placeholder.children,
                            scope,
                            expressions_map,
                            symbols_map,
                            nesting_level,
                            scoped_nodes_by_span,
                            scoped_node_entities,
                            current_level,
                            current_scope.clone(),
                        );
                    }
                }
                if let Some(ref loading) = deferred.loading {
                    if !loading.children.is_empty() {
                        visit_expressions_in_template(
                            &loading.children,
                            scope,
                            expressions_map,
                            symbols_map,
                            nesting_level,
                            scoped_nodes_by_span,
                            scoped_node_entities,
                            current_level,
                            current_scope.clone(),
                        );
                    }
                }
                if let Some(ref error) = deferred.error {
                    if !error.children.is_empty() {
                        visit_expressions_in_template(
                            &error.children,
                            scope,
                            expressions_map,
                            symbols_map,
                            nesting_level,
                            scoped_nodes_by_span,
                            scoped_node_entities,
                            current_level,
                            current_scope.clone(),
                        );
                    }
                }
            }
            t::R3Node::DeferredBlockPlaceholder(placeholder) => {
                visit_expressions_in_template(
                    &placeholder.children,
                    scope,
                    expressions_map,
                    symbols_map,
                    nesting_level,
                    scoped_nodes_by_span,
                    scoped_node_entities,
                    current_level,
                    current_scope.clone(),
                );
            }
            t::R3Node::DeferredBlockLoading(loading) => {
                visit_expressions_in_template(
                    &loading.children,
                    scope,
                    expressions_map,
                    symbols_map,
                    nesting_level,
                    scoped_nodes_by_span,
                    scoped_node_entities,
                    current_level,
                    current_scope.clone(),
                );
            }
            t::R3Node::DeferredBlockError(error) => {
                visit_expressions_in_template(
                    &error.children,
                    scope,
                    expressions_map,
                    symbols_map,
                    nesting_level,
                    scoped_nodes_by_span,
                    scoped_node_entities,
                    current_level,
                    current_scope.clone(),
                );
            }
            _ => {
                // For remaining node types, skip recursion to avoid issues
            }
        }
    }
}

// Visit AST recursively and map PropertyRead/SafePropertyRead to TemplateEntity
fn visit_ast_expressions(
    ast: &AST,
    scope: &Scope,
    expressions_map: &mut HashMap<ExprKey, TemplateEntity>,
) {
    match ast {
        AST::PropertyRead(prop) => {
            maybe_map_property_read(prop, scope, expressions_map);
            visit_ast_expressions(&prop.receiver, scope, expressions_map);
        }
        AST::SafePropertyRead(prop) => {
            maybe_map_property_read_safe(prop, scope, expressions_map);
            visit_ast_expressions(&prop.receiver, scope, expressions_map);
        }
        AST::Binary(b) => {
            visit_ast_expressions(&b.left, scope, expressions_map);
            visit_ast_expressions(&b.right, scope, expressions_map);
        }
        AST::Conditional(c) => {
            visit_ast_expressions(&c.condition, scope, expressions_map);
            visit_ast_expressions(&c.true_exp, scope, expressions_map);
            visit_ast_expressions(&c.false_exp, scope, expressions_map);
        }
        AST::PropertyWrite(p) => {
            visit_ast_expressions(&p.receiver, scope, expressions_map);
            visit_ast_expressions(&p.value, scope, expressions_map);
        }
        AST::KeyedRead(k) => {
            visit_ast_expressions(&k.receiver, scope, expressions_map);
            visit_ast_expressions(&k.key, scope, expressions_map);
        }
        AST::KeyedWrite(k) => {
            visit_ast_expressions(&k.receiver, scope, expressions_map);
            visit_ast_expressions(&k.key, scope, expressions_map);
            visit_ast_expressions(&k.value, scope, expressions_map);
        }
        AST::Call(c) => {
            visit_ast_expressions(&c.receiver, scope, expressions_map);
            for arg in &c.args {
                visit_ast_expressions(arg, scope, expressions_map);
            }
        }
        AST::SafeCall(c) => {
            visit_ast_expressions(&c.receiver, scope, expressions_map);
            for arg in &c.args {
                visit_ast_expressions(arg, scope, expressions_map);
            }
        }
        AST::BindingPipe(p) => {
            visit_ast_expressions(&p.exp, scope, expressions_map);
            for arg in &p.args {
                visit_ast_expressions(arg, scope, expressions_map);
            }
        }
        AST::Interpolation(i) => {
            for expr in &i.expressions {
                visit_ast_expressions(expr, scope, expressions_map);
            }
        }
        AST::Chain(c) => {
            for expr in &c.expressions {
                visit_ast_expressions(expr, scope, expressions_map);
            }
        }
        AST::LiteralArray(a) => {
            for expr in &a.expressions {
                visit_ast_expressions(expr, scope, expressions_map);
            }
        }
        AST::LiteralMap(m) => {
            for value in &m.values {
                visit_ast_expressions(value, scope, expressions_map);
            }
        }
        AST::Unary(u) => {
            visit_ast_expressions(&u.expr, scope, expressions_map);
        }
        AST::PrefixNot(p) => {
            visit_ast_expressions(&p.expression, scope, expressions_map);
        }
        AST::ParenthesizedExpression(p) => {
            visit_ast_expressions(&p.expression, scope, expressions_map);
        }
        _ => {}
    }
}

// Map PropertyRead to TemplateEntity if receiver is ImplicitReceiver and name exists in scope
fn maybe_map_property_read(
    prop: &PropertyRead,
    scope: &Scope,
    expressions_map: &mut HashMap<ExprKey, TemplateEntity>,
) {
    // If receiver is ImplicitReceiver (not ThisReceiver), check if name exists in scope
    match prop.receiver.as_ref() {
        AST::ImplicitReceiver(_) => {
            // Not a ThisReceiver, so check scope
            if let Some(entity) = scope.lookup(&prop.name) {
                let key = ExprKey {
                    key: format!("PropertyRead:{:?}", prop.source_span),
                };
                expressions_map.insert(key, entity.clone());
            }
        }
        _ => {}
    }
}

fn maybe_map_property_read_safe(
    prop: &SafePropertyRead,
    scope: &Scope,
    expressions_map: &mut HashMap<ExprKey, TemplateEntity>,
) {
    // Similar to PropertyRead but for safe property access
    match prop.receiver.as_ref() {
        AST::ImplicitReceiver(_) => {
            if let Some(entity) = scope.lookup(&prop.name) {
                let key = ExprKey {
                    key: format!("SafePropertyRead:{:?}", prop.source_span),
                };
                expressions_map.insert(key, entity.clone());
            }
        }
        _ => {}
    }
}

fn extract_entities_recursive(nodes: &[t::R3Node], entities: &mut Vec<TemplateEntity>) {
    for node in nodes {
        match node {
            t::R3Node::Element(el) => {
                for reference in &el.references {
                    entities.push(TemplateEntity::Reference(reference.clone()));
                }
                extract_entities_recursive(&el.children, entities);
            }
            t::R3Node::Template(tmpl) => {
                // references are available in the parent scope
                for reference in &tmpl.references {
                    entities.push(TemplateEntity::Reference(reference.clone()));
                }
                // variables and children are in the inner scope - DO NOT recurse
            }
            t::R3Node::LetDeclaration(decl) => {
                entities.push(TemplateEntity::LetDeclaration((*decl).as_ref().clone()));
            }
            t::R3Node::IfBlock(_)
            | t::R3Node::ForLoopBlock(_)
            | t::R3Node::SwitchBlock(_)
            | t::R3Node::DeferredBlock(_) => {
                // These create new scopes/views - DO NOT recurse
            }
            _ => {
                // For other nodes (e.g. Content), if they have children, we might need to recurse if they don't create a new view.
                // But generally only Element has children in the same view.
                // Content (content projection) ? <ng-content> doesn't have children in AST.
                // So default to nothing.
            }
        }
    }
}

/// Internal representation of binding target.
#[derive(Debug, Clone)]
pub enum BindingTarget<DirectiveT> {
    Directive(DirectiveT),
    Element(Element),
    Template(Template),
}

/// Internal representation of reference target.
#[derive(Debug, Clone)]
pub enum ReferenceTargetInternal<DirectiveT> {
    Directive {
        directive: DirectiveT,
        node_id: usize,
    },
    Element(usize),
    Template(usize),
}

/// Metadata container for a `Target`.
pub struct R3BoundTarget<DirectiveT: DirectiveMeta + Clone> {
    target: Target<DirectiveT>,
    directives_map: HashMap<DirectiveOwnerWrapper, Vec<DirectiveT>>,
    eager_directives: Vec<DirectiveT>,
    missing_directives: HashSet<String>,
    bindings: HashMap<BindingKey, BindingTarget<DirectiveT>>,
    references_map: HashMap<ReferenceKey, ReferenceTargetInternal<DirectiveT>>,
    expressions_map: HashMap<ExprKey, TemplateEntity>,
    symbols_map: HashMap<EntityKey, ScopedNodeWrapper>,
    nesting_level: HashMap<ScopedNodeWrapper, usize>,
    // Store actual ScopedNodes for lookup by source span
    scoped_nodes_by_span: HashMap<String, ScopedNode>,
    scoped_node_entities: HashMap<ScopedNodeWrapper, Vec<TemplateEntity>>,
    used_pipes: HashSet<String>,
    eager_pipes: HashSet<String>,
    defer_blocks: Vec<DeferredBlock>,
}

impl<DirectiveT: DirectiveMeta + Clone + 'static> BoundTarget<DirectiveT>
    for R3BoundTarget<DirectiveT>
{
    fn target(&self) -> &Target<DirectiveT> {
        &self.target
    }

    fn get_directives_of_node(&self, node: &DirectiveOwner) -> Option<Vec<DirectiveT>> {
        let key = DirectiveOwnerWrapper::from(node);
        self.directives_map.get(&key).cloned()
    }

    fn get_reference_target(&self, reference: &Reference) -> Option<ReferenceTarget<DirectiveT>> {
        // TODO: Implement full reference resolution
        // For now, try to find element with matching name
        if let Some(ref template_nodes) = self.target.template {
            find_reference_target_in_nodes(template_nodes, reference)
        } else {
            None
        }
    }

    fn get_consumer_of_binding(
        &self,
        binding: &dyn std::any::Any,
    ) -> Option<super::t2_api::ConsumerOfBinding<DirectiveT>> {
        use crate::render3::r3_ast::{BoundAttribute, BoundEvent, TextAttribute};

        // Try to downcast to specific binding types
        if let Some(attr) = binding.downcast_ref::<BoundAttribute>() {
            let key = BindingKey {
                key: format!("BoundAttribute:{}:{:?}", attr.name, attr.source_span),
            };
            if let Some(target) = self.bindings.get(&key) {
                match target {
                    BindingTarget::Directive(d) => {
                        Some(super::t2_api::ConsumerOfBinding::Directive(d.clone()))
                    }
                    BindingTarget::Element(el) => {
                        Some(super::t2_api::ConsumerOfBinding::Element(el.clone()))
                    }
                    BindingTarget::Template(tmpl) => {
                        Some(super::t2_api::ConsumerOfBinding::Template(tmpl.clone()))
                    }
                }
            } else {
                None
            }
        } else if let Some(event) = binding.downcast_ref::<BoundEvent>() {
            let key = BindingKey {
                key: format!("BoundEvent:{}:{:?}", event.name, event.source_span),
            };
            if let Some(target) = self.bindings.get(&key) {
                match target {
                    BindingTarget::Directive(d) => {
                        Some(super::t2_api::ConsumerOfBinding::Directive(d.clone()))
                    }
                    BindingTarget::Element(el) => {
                        Some(super::t2_api::ConsumerOfBinding::Element(el.clone()))
                    }
                    BindingTarget::Template(tmpl) => {
                        Some(super::t2_api::ConsumerOfBinding::Template(tmpl.clone()))
                    }
                }
            } else {
                None
            }
        } else if let Some(attr) = binding.downcast_ref::<TextAttribute>() {
            let key = BindingKey {
                key: format!("TextAttribute:{}:{:?}", attr.name, attr.source_span),
            };
            if let Some(target) = self.bindings.get(&key) {
                match target {
                    BindingTarget::Directive(d) => {
                        Some(super::t2_api::ConsumerOfBinding::Directive(d.clone()))
                    }
                    BindingTarget::Element(el) => {
                        Some(super::t2_api::ConsumerOfBinding::Element(el.clone()))
                    }
                    BindingTarget::Template(tmpl) => {
                        Some(super::t2_api::ConsumerOfBinding::Template(tmpl.clone()))
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_expression_target(&self, expr: &AST) -> Option<TemplateEntity> {
        // Create a key from the AST expression's source span
        let key = match expr {
            AST::PropertyRead(prop) => ExprKey {
                key: format!("PropertyRead:{:?}", prop.source_span),
            },
            AST::SafePropertyRead(prop) => ExprKey {
                key: format!("SafePropertyRead:{:?}", prop.source_span),
            },
            _ => {
                // For other AST types, try to generate a key based on source span
                // This is a simplified approach - in full implementation, we'd need
                // to hash the AST structure
                return None;
            }
        };
        self.expressions_map.get(&key).cloned()
    }

    fn get_definition_node_of_symbol(&self, symbol: &TemplateEntity) -> Option<ScopedNode> {
        // Create a key for the symbol
        let key = match symbol {
            TemplateEntity::Reference(r) => EntityKey {
                key: format!("Reference:{}", r.name),
            },
            TemplateEntity::Variable(v) => EntityKey {
                key: format!("Variable:{}", v.name),
            },
            TemplateEntity::LetDeclaration(l) => EntityKey {
                key: format!("LetDeclaration:{}", l.name),
            },
        };
        // Lookup the ScopedNodeWrapper for this symbol
        if let Some(wrapper) = self.symbols_map.get(&key) {
            // Find the ScopedNode using the wrapper's key
            // We need to match by source span key
            for (span_key, scoped_node) in &self.scoped_nodes_by_span {
                if span_key == &wrapper.key {
                    return Some(scoped_node.clone());
                }
            }
            // Fallback: try to construct from wrapper key if it's a simple identifier
            // This is a workaround - ideally we'd store the mapping more directly
        }
        None
    }

    fn get_nesting_level(&self, node: &ScopedNode) -> usize {
        // Create a span key for this node
        let span_key = match node {
            ScopedNode::Template(tmpl) => format!("{:?}", tmpl.source_span),
            ScopedNode::ForLoopBlock(fl) => format!("{:?}", fl.block.source_span),
            ScopedNode::ForLoopBlockEmpty(fl_empty) => format!("{:?}", fl_empty.block.source_span),
            ScopedNode::IfBlockBranch(branch) => format!("{:?}", branch.block.source_span),
            ScopedNode::SwitchBlockCase(case) => format!("{:?}", case.block.source_span),
            ScopedNode::DeferredBlock(def) => format!("{:?}", def.block.source_span),
            ScopedNode::DeferredBlockLoading(loading) => format!("{:?}", loading.block.source_span),
            ScopedNode::DeferredBlockPlaceholder(placeholder) => {
                format!("{:?}", placeholder.block.source_span)
            }
            ScopedNode::DeferredBlockError(error) => format!("{:?}", error.block.source_span),
            ScopedNode::Content(content) => format!("{:?}", content.source_span),
            ScopedNode::HostElement(host) => format!("{:?}", host.source_span),
        };
        let wrapper = ScopedNodeWrapper { key: span_key };
        self.nesting_level.get(&wrapper).copied().unwrap_or(0)
    }

    fn get_entities_in_scope(&self, node: Option<&ScopedNode>) -> HashSet<TemplateEntity> {
        // Get entities for the given scope
        let wrapper = if let Some(scoped_node) = node {
            // Create wrapper from ScopedNode
            let span_key = match scoped_node {
                ScopedNode::Template(tmpl) => format!("{:?}", tmpl.source_span),
                ScopedNode::ForLoopBlock(fl) => format!("{:?}", fl.block.source_span),
                ScopedNode::ForLoopBlockEmpty(fl_empty) => {
                    format!("{:?}", fl_empty.block.source_span)
                }
                ScopedNode::IfBlockBranch(branch) => format!("{:?}", branch.block.source_span),
                ScopedNode::SwitchBlockCase(case) => format!("{:?}", case.block.source_span),
                ScopedNode::DeferredBlock(def) => format!("{:?}", def.block.source_span),
                ScopedNode::DeferredBlockLoading(loading) => {
                    format!("{:?}", loading.block.source_span)
                }
                ScopedNode::DeferredBlockPlaceholder(placeholder) => {
                    format!("{:?}", placeholder.block.source_span)
                }
                ScopedNode::DeferredBlockError(error) => format!("{:?}", error.block.source_span),
                ScopedNode::Content(content) => format!("{:?}", content.source_span),
                ScopedNode::HostElement(host) => format!("{:?}", host.source_span),
            };
            ScopedNodeWrapper { key: span_key }
        } else {
            // Root scope
            ScopedNodeWrapper {
                key: "root".to_string(),
            }
        };

        if let Some(entities_vec) = self.scoped_node_entities.get(&wrapper) {
            entities_vec.iter().cloned().collect()
        } else {
            HashSet::new()
        }
    }

    fn get_used_directives(&self) -> Vec<DirectiveT> {
        let mut set: HashSet<String> = HashSet::new();
        let mut result = vec![];
        for dirs in self.directives_map.values() {
            for dir in dirs {
                if set.insert(dir.name().to_string()) {
                    result.push(dir.clone());
                }
            }
        }
        result
    }

    fn get_eagerly_used_directives(&self) -> Vec<DirectiveT> {
        let mut set: HashSet<String> = HashSet::new();
        let mut result = vec![];
        for dir in &self.eager_directives {
            if set.insert(dir.name().to_string()) {
                result.push(dir.clone());
            }
        }
        result
    }

    fn get_used_pipes(&self) -> Vec<String> {
        self.used_pipes.iter().cloned().collect()
    }

    fn get_eagerly_used_pipes(&self) -> Vec<String> {
        self.eager_pipes.iter().cloned().collect()
    }

    fn get_defer_blocks(&self) -> Vec<DeferredBlock> {
        self.defer_blocks.clone()
    }

    fn get_deferred_trigger_target(
        &self,
        block: &DeferredBlock,
        trigger: &t::DeferredTrigger,
    ) -> Option<Element> {
        // Only triggers that refer to DOM nodes can be resolved
        let reference = match trigger {
            t::DeferredTrigger::Interaction(interaction) => &interaction.reference,
            t::DeferredTrigger::Viewport(viewport) => &viewport.reference,
            t::DeferredTrigger::Hover(hover) => &hover.reference,
            _ => return None,
        };

        // If reference is None, try to infer from placeholder
        if reference.is_none() {
            if let Some(ref placeholder) = block.placeholder {
                let mut target: Option<Element> = None;
                for child in &placeholder.children {
                    // Skip comments
                    if let t::R3Node::Comment(_) = child {
                        continue;
                    }
                    // We can only infer if there's one root element
                    if target.is_some() {
                        return None;
                    }
                    if let t::R3Node::Element(el) = child {
                        target = Some((*el).as_ref().clone());
                    }
                }
                return target;
            }
            return None;
        }

        // Try to find reference in scope
        let ref_name = reference.as_ref().unwrap();

        // Find element by reference in template
        if let Some(ref template_nodes) = self.target.template {
            if let Some(element) = find_element_by_reference(template_nodes, ref_name) {
                // Check if the element is inside the main content of the defer block
                // If it is, return None because triggers inside main content are invalid
                if is_element_in_nodes(&block.children, &element) {
                    return None;
                }
                return Some(element);
            }
        }
        None
    }

    fn is_deferred(&self, node: &Element) -> bool {
        // Check if element is within any defer block
        if let Some(ref template_nodes) = self.target.template {
            is_element_in_defer_blocks(template_nodes, node, &self.defer_blocks)
        } else {
            false
        }
    }

    fn referenced_directive_exists(&self, name: &str) -> bool {
        // Check if any matched directive has this name
        for dirs in self.directives_map.values() {
            for dir in dirs {
                if dir.name() == name {
                    return true;
                }
            }
        }
        false
    }
}

// Helper to find element by reference name
fn find_element_by_reference(nodes: &[t::R3Node], ref_name: &str) -> Option<Element> {
    for node in nodes {
        match node {
            t::R3Node::Element(el) => {
                // Check if this element has a matching reference
                for ref_node in &el.references {
                    if &*ref_node.name == ref_name {
                        return Some((*el).as_ref().clone());
                    }
                }
                // Recursively check children
                if let Some(result) = find_element_by_reference(&el.children, ref_name) {
                    return Some(result);
                }
            }
            t::R3Node::Template(tmpl) => {
                // Check references on template
                for ref_node in &tmpl.references {
                    if &*ref_node.name == ref_name {
                        // Template itself, not an element - return None for now
                        return None;
                    }
                }
                // Recursively check children
                if let Some(result) = find_element_by_reference(&tmpl.children, ref_name) {
                    return Some(result);
                }
            }
            t::R3Node::Component(comp) => {
                // Check references on component
                for ref_node in &comp.references {
                    if &*ref_node.name == ref_name {
                        // Convert component to element representation
                        return Some(Element {
                            name: comp.component_name.clone(),
                            attributes: vec![],
                            inputs: comp.inputs.clone(),
                            outputs: comp.outputs.clone(),
                            references: comp.references.clone(),
                            directives: vec![],
                            children: comp.children.clone(),
                            source_span: comp.source_span.clone(),
                            start_source_span: comp.start_source_span.clone(),
                            end_source_span: comp.end_source_span.clone(),
                            i18n: comp.i18n.clone(),
                            is_self_closing: false,
                            is_void: false,
                        });
                    }
                }
                // Recursively check children
                if let Some(result) = find_element_by_reference(&comp.children, ref_name) {
                    return Some(result);
                }
            }
            t::R3Node::DeferredBlock(deferred) => {
                if let Some(result) = find_element_by_reference(&deferred.children, ref_name) {
                    return Some(result);
                }
                if let Some(ref placeholder) = deferred.placeholder {
                    if let Some(result) = find_element_by_reference(&placeholder.children, ref_name)
                    {
                        return Some(result);
                    }
                }
                if let Some(ref loading) = deferred.loading {
                    if let Some(result) = find_element_by_reference(&loading.children, ref_name) {
                        return Some(result);
                    }
                }
                if let Some(ref error) = deferred.error {
                    if let Some(result) = find_element_by_reference(&error.children, ref_name) {
                        return Some(result);
                    }
                }
            }
            t::R3Node::IfBlock(if_block) => {
                for branch in &if_block.branches {
                    if let Some(result) = find_element_by_reference(&branch.children, ref_name) {
                        return Some(result);
                    }
                }
            }
            t::R3Node::ForLoopBlock(for_loop) => {
                if let Some(result) = find_element_by_reference(&for_loop.children, ref_name) {
                    return Some(result);
                }
                if let Some(ref empty) = for_loop.empty {
                    if let Some(result) = find_element_by_reference(&empty.children, ref_name) {
                        return Some(result);
                    }
                }
            }
            t::R3Node::SwitchBlock(switch) => {
                for case in &switch.cases {
                    if let Some(result) = find_element_by_reference(&case.children, ref_name) {
                        return Some(result);
                    }
                }
            }
            _ => {
                // Skip other node types that don't have children
            }
        }
    }
    None
}

// Helper to check if element is within defer blocks
fn is_element_in_defer_blocks(
    _nodes: &[t::R3Node],
    target_element: &Element,
    defer_blocks: &[DeferredBlock],
) -> bool {
    // Check each defer block's children
    for block in defer_blocks {
        if is_element_in_nodes(&block.children, target_element) {
            return true;
        }
        if let Some(ref placeholder) = block.placeholder {
            if is_element_in_nodes(&placeholder.children, target_element) {
                return true;
            }
        }
        if let Some(ref loading) = block.loading {
            if is_element_in_nodes(&loading.children, target_element) {
                return true;
            }
        }
        if let Some(ref error) = block.error {
            if is_element_in_nodes(&error.children, target_element) {
                return true;
            }
        }
    }
    false
}

// Helper to check if target element exists in nodes
fn is_element_in_nodes(nodes: &[t::R3Node], target_element: &Element) -> bool {
    for node in nodes {
        match node {
            t::R3Node::Element(el) => {
                // Compare by name and source span (simple comparison using start offset)
                if el.name == target_element.name
                    && el.source_span.start.offset == target_element.source_span.start.offset
                {
                    return true;
                }
                // Recursively check children
                if is_element_in_nodes(&el.children, target_element) {
                    return true;
                }
            }
            t::R3Node::Template(tmpl) => {
                if is_element_in_nodes(&tmpl.children, target_element) {
                    return true;
                }
            }
            t::R3Node::DeferredBlock(deferred) => {
                if is_element_in_nodes(&deferred.children, target_element) {
                    return true;
                }
                if let Some(ref placeholder) = deferred.placeholder {
                    if is_element_in_nodes(&placeholder.children, target_element) {
                        return true;
                    }
                }
                if let Some(ref loading) = deferred.loading {
                    if is_element_in_nodes(&loading.children, target_element) {
                        return true;
                    }
                }
                if let Some(ref error) = deferred.error {
                    if is_element_in_nodes(&error.children, target_element) {
                        return true;
                    }
                }
            }
            t::R3Node::IfBlock(if_block) => {
                for branch in &if_block.branches {
                    if is_element_in_nodes(&branch.children, target_element) {
                        return true;
                    }
                }
            }
            t::R3Node::ForLoopBlock(for_loop) => {
                if is_element_in_nodes(&for_loop.children, target_element) {
                    return true;
                }
                if let Some(ref empty) = for_loop.empty {
                    if is_element_in_nodes(&empty.children, target_element) {
                        return true;
                    }
                }
            }
            t::R3Node::SwitchBlock(switch) => {
                for case in &switch.cases {
                    if is_element_in_nodes(&case.children, target_element) {
                        return true;
                    }
                }
            }
            _ => {
                // Skip other node types
            }
        }
    }
    false
}

fn find_reference_target_in_nodes<DirectiveT: DirectiveMeta>(
    nodes: &[t::R3Node],
    reference: &Reference,
) -> Option<ReferenceTarget<DirectiveT>> {
    for node in nodes {
        match node {
            t::R3Node::Element(el) => {
                // Check if this element has a matching reference
                for ref_node in &el.references {
                    if ref_node.name == reference.name {
                        return Some(ReferenceTarget::Element((*el).as_ref().clone()));
                    }
                }
                // Recursively check children
                if let Some(result) = find_reference_target_in_nodes(&el.children, reference) {
                    return Some(result);
                }
            }
            t::R3Node::Template(tmpl) => {
                // Check references on template
                for ref_node in &tmpl.references {
                    if ref_node.name == reference.name {
                        return Some(ReferenceTarget::Template((*tmpl).as_ref().clone()));
                    }
                }
                // Recursively check children
                if let Some(result) = find_reference_target_in_nodes(&tmpl.children, reference) {
                    return Some(result);
                }
            }
            t::R3Node::Component(comp) => {
                // Check references on component
                for ref_node in &comp.references {
                    if ref_node.name == reference.name {
                        // TODO: Need to return component as element or directive
                        // For now, return element representation
                        return Some(ReferenceTarget::Element(Element {
                            name: comp.component_name.clone(),
                            attributes: vec![],
                            inputs: comp.inputs.clone(),
                            outputs: comp.outputs.clone(),
                            references: comp.references.clone(),
                            directives: vec![],
                            children: comp.children.clone(),
                            source_span: comp.source_span.clone(),
                            start_source_span: comp.start_source_span.clone(),
                            end_source_span: comp.end_source_span.clone(),
                            i18n: comp.i18n.clone(),
                            is_self_closing: false,
                            is_void: false,
                        }));
                    }
                }
                // Recursively check children
                if let Some(result) = find_reference_target_in_nodes(&comp.children, reference) {
                    return Some(result);
                }
            }
            t::R3Node::DeferredBlock(deferred) => {
                if let Some(result) = find_reference_target_in_nodes(&deferred.children, reference)
                {
                    return Some(result);
                }
                if let Some(ref placeholder) = deferred.placeholder {
                    if let Some(result) =
                        find_reference_target_in_nodes(&placeholder.children, reference)
                    {
                        return Some(result);
                    }
                }
                if let Some(ref loading) = deferred.loading {
                    if let Some(result) =
                        find_reference_target_in_nodes(&loading.children, reference)
                    {
                        return Some(result);
                    }
                }
                if let Some(ref error) = deferred.error {
                    if let Some(result) = find_reference_target_in_nodes(&error.children, reference)
                    {
                        return Some(result);
                    }
                }
            }
            t::R3Node::IfBlock(if_block) => {
                for branch in &if_block.branches {
                    if let Some(result) =
                        find_reference_target_in_nodes(&branch.children, reference)
                    {
                        return Some(result);
                    }
                }
            }
            t::R3Node::ForLoopBlock(for_loop) => {
                if let Some(result) = find_reference_target_in_nodes(&for_loop.children, reference)
                {
                    return Some(result);
                }
                if let Some(ref empty) = for_loop.empty {
                    if let Some(result) = find_reference_target_in_nodes(&empty.children, reference)
                    {
                        return Some(result);
                    }
                }
            }
            t::R3Node::SwitchBlock(switch) => {
                for case in &switch.cases {
                    if let Some(result) = find_reference_target_in_nodes(&case.children, reference)
                    {
                        return Some(result);
                    }
                }
            }
            _ => {
                // Skip other node types
            }
        }
    }
    None
}

/// Represents a binding scope within a template.
pub struct Scope {
    /// Named members of the `Scope`, such as `Reference`s or `Variable`s.
    pub named_entities: HashMap<std::sync::Arc<str>, TemplateEntity>,
    /// Set of element-like nodes that belong to this scope.
    pub element_like_in_scope: HashSet<usize>,
    /// Child `Scope`s for immediately nested `ScopedNode`s.
    pub child_scopes: HashMap<usize, Scope>,
    /// Whether this scope is deferred or if any of its ancestors are deferred.
    pub is_deferred: bool,
    /// Parent scope
    parent_scope: Option<Box<Scope>>,
    /// Root node id
    root_node: Option<usize>,
}

impl Scope {
    pub fn new_root_scope() -> Self {
        Scope {
            named_entities: HashMap::new(),
            element_like_in_scope: HashSet::new(),
            child_scopes: HashMap::new(),
            is_deferred: false,
            parent_scope: None,
            root_node: None,
        }
    }

    /// Process a template and construct its `Scope`.
    pub fn apply(nodes: &[t::R3Node]) -> Self {
        let mut scope = Self::new_root_scope();
        scope.ingest_nodes(nodes);
        scope
    }

    fn ingest_nodes(&mut self, nodes: &[t::R3Node]) {
        for node in nodes {
            match node {
                t::R3Node::Element(el) => {
                    // References on elements are in outer scope
                    for reference in &el.references {
                        self.maybe_declare(TemplateEntity::Reference(reference.clone()));
                    }
                    self.ingest_nodes(&el.children);
                }
                t::R3Node::Template(tmpl) => {
                    // References on template are in outer scope, variables are in inner scope
                    for reference in &tmpl.references {
                        self.maybe_declare(TemplateEntity::Reference(reference.clone()));
                    }
                    // Variables are in inner scope - create child scope for template
                    // For now, just process children
                    self.ingest_nodes(&tmpl.children);
                }
                t::R3Node::LetDeclaration(decl) => {
                    self.maybe_declare(TemplateEntity::LetDeclaration((*decl).as_ref().clone()));
                }
                t::R3Node::ForLoopBlock(for_loop) => {
                    // Item variable is in the loop's scope
                    self.maybe_declare(TemplateEntity::Variable(for_loop.item.clone()));
                    for context_var in &for_loop.context_variables {
                        self.maybe_declare(TemplateEntity::Variable(context_var.clone()));
                    }
                    self.ingest_nodes(&for_loop.children);
                }
                t::R3Node::IfBlock(if_block) => {
                    // Process all branches
                    for branch in &if_block.branches {
                        if let Some(ref alias) = branch.expression_alias {
                            self.maybe_declare(TemplateEntity::Variable(alias.clone()));
                        }
                        self.ingest_nodes(&branch.children);
                    }
                }
                t::R3Node::IfBlockBranch(branch) => {
                    // Expression alias is in branch scope
                    if let Some(ref alias) = branch.expression_alias {
                        self.maybe_declare(TemplateEntity::Variable(alias.clone()));
                    }
                    self.ingest_nodes(&branch.children);
                }
                t::R3Node::DeferredBlock(deferred) => {
                    self.ingest_nodes(&deferred.children);
                    if let Some(ref placeholder) = deferred.placeholder {
                        self.ingest_nodes(&placeholder.children);
                    }
                    if let Some(ref loading) = deferred.loading {
                        self.ingest_nodes(&loading.children);
                    }
                    if let Some(ref error) = deferred.error {
                        self.ingest_nodes(&error.children);
                    }
                }
                t::R3Node::SwitchBlock(switch) => {
                    for case in &switch.cases {
                        self.ingest_nodes(&case.children);
                    }
                }
                _ => {
                    // Skip other node types that don't have children
                }
            }
        }
    }

    fn maybe_declare(&mut self, entity: TemplateEntity) {
        let name = match &entity {
            TemplateEntity::Reference(r) => &r.name,
            TemplateEntity::Variable(v) => &v.name,
            TemplateEntity::LetDeclaration(l) => &l.name,
        };
        if !self.named_entities.contains_key(name.as_ref()) {
            self.named_entities.insert(name.clone(), entity);
        }
    }

    /// Look up a variable within this `Scope`.
    pub fn lookup(&self, name: &str) -> Option<TemplateEntity> {
        if let Some(entity) = self.named_entities.get(name) {
            return Some(entity.clone());
        }
        if let Some(ref parent) = self.parent_scope {
            return parent.lookup(name);
        }
        None
    }

    /// Get the child scope for a `ScopedNode`.
    pub fn get_child_scope(&self, node_id: usize) -> Option<&Scope> {
        self.child_scopes.get(&node_id)
    }
}
