//! Render3 Partial Component Compilation
//!
//! Corresponds to packages/compiler/src/render3/partial/component.ts
//! Contains component declaration compilation for partial/linking mode

use crate::core::{ChangeDetectionStrategy, ViewEncapsulation};
use crate::output::output_ast::{
    Expression, ExternalExpr, InvokeFunctionExpr, LiteralArrayExpr, LiteralExpr, LiteralValue,
    ReadPropExpr,
};
use crate::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};
use crate::render3::r3_identifiers::Identifiers as R3;
use crate::render3::util::{generate_forward_ref, R3CompiledExpression};
use crate::render3::view::api::{
    DeclarationListEmitMode, R3ComponentMetadata, R3TemplateDependencyMetadata,
};
use crate::render3::view::util::DefinitionMap;

use super::directive::create_directive_definition_map;

/// Template info for declare component
#[derive(Debug, Clone)]
pub struct DeclareComponentTemplateInfo {
    /// The string contents of the template.
    pub content: String,
    /// A full path to the file which contains the template.
    pub source_url: String,
    /// Whether the template was inline or external.
    pub is_inline: bool,
    /// If the template was defined inline by a direct string literal.
    pub inline_template_literal_expression: Option<Expression>,
}

/// Parsed template structure
#[derive(Debug, Clone)]
pub struct ParsedTemplate {
    pub nodes: Vec<crate::render3::r3_ast::R3Node>,
    pub preserve_whitespaces: bool,
}

/// Helper to create literal expression
fn literal(value: LiteralValue) -> Expression {
    Expression::Literal(LiteralExpr {
        value,
        type_: None,
        source_span: None,
    })
}

/// Helper to create literal expression with source span
fn literal_with_span(value: LiteralValue, span: Option<ParseSourceSpan>) -> Expression {
    Expression::Literal(LiteralExpr {
        value,
        type_: None,
        source_span: span,
    })
}

/// Helper to create external expression from ExternalReference
fn external_expr(reference: crate::output::output_ast::ExternalReference) -> Expression {
    Expression::External(ExternalExpr {
        value: reference,
        type_: None,
        source_span: None,
    })
}

/// Compile a component declaration defined by the `R3ComponentMetadata`.
pub fn compile_declare_component_from_metadata(
    meta: &R3ComponentMetadata,
    template: &ParsedTemplate,
    additional_template_info: &DeclareComponentTemplateInfo,
) -> R3CompiledExpression {
    let definition_map = create_component_definition_map(meta, template, additional_template_info);

    let declare_component_ref = R3::declare_component();
    let declare_component_expr = external_expr(declare_component_ref);

    let expression = Expression::InvokeFn(InvokeFunctionExpr {
        fn_: Box::new(declare_component_expr),
        args: vec![Expression::LiteralMap(definition_map.to_literal_map())],
        type_: None,
        source_span: None,
        pure: false,
    });

    // TODO: implement create_component_type
    let type_ = crate::output::output_ast::dynamic_type();

    R3CompiledExpression::new(expression, type_, vec![])
}

/// Gathers the declaration fields for a component into a `DefinitionMap`.
pub fn create_component_definition_map(
    meta: &R3ComponentMetadata,
    template: &ParsedTemplate,
    template_info: &DeclareComponentTemplateInfo,
) -> DefinitionMap {
    let mut definition_map = create_directive_definition_map(&meta.directive);

    // Check if template has blocks
    let has_blocks = check_has_blocks(&template.nodes);

    definition_map.set(
        "template",
        Some(get_template_expression(template, template_info)),
    );

    if template_info.is_inline {
        definition_map.set("isInline", Some(literal(LiteralValue::Bool(true))));
    }

    // Set the minVersion to 17.0.0 if the component is using at least one block
    if has_blocks {
        definition_map.set(
            "minVersion",
            Some(literal(LiteralValue::String("17.0.0".to_string()))),
        );
    }

    // styles
    if !meta.styles.is_empty() {
        let styles_exprs: Vec<Expression> = meta
            .styles
            .iter()
            .map(|s| literal(LiteralValue::String(s.clone())))
            .collect();
        definition_map.set(
            "styles",
            Some(Expression::LiteralArray(LiteralArrayExpr {
                entries: styles_exprs,
                type_: None,
                source_span: None,
            })),
        );
    }

    // dependencies
    if let Some(deps_expr) = compile_used_dependencies_metadata(meta) {
        definition_map.set("dependencies", Some(deps_expr));
    }

    // viewProviders
    if let Some(ref view_providers) = meta.view_providers {
        definition_map.set("viewProviders", Some(view_providers.clone()));
    }

    // animations
    if let Some(ref animations) = meta.animations {
        definition_map.set("animations", Some(animations.clone()));
    }

    // changeDetection
    if let Some(ref change_detection) = meta.change_detection {
        match change_detection {
            crate::render3::view::api::ChangeDetectionOrExpression::Strategy(strategy) => {
                let strategy_name = match strategy {
                    ChangeDetectionStrategy::OnPush => "OnPush",
                    ChangeDetectionStrategy::Default => "Default",
                };
                // Import ChangeDetectionStrategy from core
                let core_ref = R3::core();
                let cd_strategy_expr = Expression::ReadProp(ReadPropExpr {
                    receiver: Box::new(external_expr(core_ref)),
                    name: format!("ChangeDetectionStrategy.{}", strategy_name),
                    type_: None,
                    source_span: None,
                });
                definition_map.set("changeDetection", Some(cd_strategy_expr));
            }
            crate::render3::view::api::ChangeDetectionOrExpression::Expression(_) => {
                panic!("Impossible state! Change detection flag is not resolved!");
            }
        }
    }

    // encapsulation
    if meta.encapsulation != ViewEncapsulation::Emulated {
        let encap_name = match meta.encapsulation {
            ViewEncapsulation::None => "None",
            ViewEncapsulation::ShadowDom => "ShadowDom",
            _ => "Emulated",
        };
        // Import ViewEncapsulation from core
        let core_ref = R3::core();
        let encap_expr = Expression::ReadProp(ReadPropExpr {
            receiver: Box::new(external_expr(core_ref)),
            name: format!("ViewEncapsulation.{}", encap_name),
            type_: None,
            source_span: None,
        });
        definition_map.set("encapsulation", Some(encap_expr));
    }

    // preserveWhitespaces
    if template.preserve_whitespaces {
        definition_map.set(
            "preserveWhitespaces",
            Some(literal(LiteralValue::Bool(true))),
        );
    }

    // deferBlockDependencies
    match &meta.defer {
        crate::render3::view::api::R3ComponentDeferMetadata::PerBlock { blocks } => {
            let mut resolvers: Vec<Expression> = vec![];
            let mut has_resolvers = false;

            for deps in blocks.values() {
                if let Some(d) = deps {
                    resolvers.push(d.clone());
                    has_resolvers = true;
                } else {
                    resolvers.push(literal(LiteralValue::Null));
                }
            }

            if has_resolvers {
                definition_map.set(
                    "deferBlockDependencies",
                    Some(Expression::LiteralArray(LiteralArrayExpr {
                        entries: resolvers,
                        type_: None,
                        source_span: None,
                    })),
                );
            }
        }
        crate::render3::view::api::R3ComponentDeferMetadata::PerComponent { .. } => {
            panic!("Unsupported defer function emit mode in partial compilation");
        }
    }

    definition_map
}

fn get_template_expression(
    _template: &ParsedTemplate,
    template_info: &DeclareComponentTemplateInfo,
) -> Expression {
    // If the template has been defined using a direct literal, use that expression directly
    if let Some(ref inline_expr) = template_info.inline_template_literal_expression {
        return inline_expr.clone();
    }

    // If the template is defined inline but not through a literal
    if template_info.is_inline {
        return literal(LiteralValue::String(template_info.content.clone()));
    }

    // The template is external so we must synthesize an expression node with source-span
    let contents = &template_info.content;
    let start =
        ParseLocation::from_source(contents.clone(), template_info.source_url.clone(), 0, 0, 0);
    let end = compute_end_location(&contents, &template_info.source_url);
    let span = ParseSourceSpan::new(start, end);

    literal_with_span(LiteralValue::String(contents.clone()), Some(span))
}

fn compute_end_location(contents: &str, source_url: &str) -> ParseLocation {
    let length = contents.len();
    let mut _line_start: i32 = 0;
    let mut last_line_start = 0usize;
    let mut line = 0usize;

    loop {
        if let Some(idx) = contents[last_line_start..].find('\n') {
            _line_start = (last_line_start + idx) as i32;
            last_line_start = last_line_start + idx + 1;
            line += 1;
        } else {
            break;
        }
    }

    ParseLocation::from_source(
        contents.to_string(),
        source_url.to_string(),
        length,
        line,
        length - last_line_start,
    )
}

fn compile_used_dependencies_metadata(meta: &R3ComponentMetadata) -> Option<Expression> {
    let wrap_type: Box<dyn Fn(Expression) -> Expression> = match meta.declaration_list_emit_mode {
        DeclarationListEmitMode::Direct => Box::new(|expr| expr),
        _ => Box::new(generate_forward_ref),
    };

    if matches!(
        meta.declaration_list_emit_mode,
        DeclarationListEmitMode::RuntimeResolved
    ) {
        panic!("Unsupported emit mode");
    }

    if meta.declarations.is_empty() {
        return None;
    }

    let exprs: Vec<Expression> = meta
        .declarations
        .iter()
        .map(|decl| match decl {
            R3TemplateDependencyMetadata::Directive(dir) => {
                let mut dir_meta = DefinitionMap::new();
                let kind = if dir.is_component {
                    "component"
                } else {
                    "directive"
                };
                dir_meta.set(
                    "kind",
                    Some(literal(LiteralValue::String(kind.to_string()))),
                );
                dir_meta.set("type", Some(wrap_type(dir.type_.clone())));
                dir_meta.set(
                    "selector",
                    Some(literal(LiteralValue::String(dir.selector.clone()))),
                );

                if !dir.inputs.is_empty() {
                    let inputs_exprs: Vec<Expression> = dir
                        .inputs
                        .iter()
                        .map(|s| literal(LiteralValue::String(s.clone())))
                        .collect();
                    dir_meta.set(
                        "inputs",
                        Some(Expression::LiteralArray(LiteralArrayExpr {
                            entries: inputs_exprs,
                            type_: None,
                            source_span: None,
                        })),
                    );
                }

                if !dir.outputs.is_empty() {
                    let outputs_exprs: Vec<Expression> = dir
                        .outputs
                        .iter()
                        .map(|s| literal(LiteralValue::String(s.clone())))
                        .collect();
                    dir_meta.set(
                        "outputs",
                        Some(Expression::LiteralArray(LiteralArrayExpr {
                            entries: outputs_exprs,
                            type_: None,
                            source_span: None,
                        })),
                    );
                }

                if let Some(ref export_as) = dir.export_as {
                    let export_as_exprs: Vec<Expression> = export_as
                        .iter()
                        .map(|s| literal(LiteralValue::String(s.clone())))
                        .collect();
                    dir_meta.set(
                        "exportAs",
                        Some(Expression::LiteralArray(LiteralArrayExpr {
                            entries: export_as_exprs,
                            type_: None,
                            source_span: None,
                        })),
                    );
                }

                Expression::LiteralMap(dir_meta.to_literal_map())
            }
            R3TemplateDependencyMetadata::Pipe(pipe) => {
                let mut pipe_meta = DefinitionMap::new();
                pipe_meta.set(
                    "kind",
                    Some(literal(LiteralValue::String("pipe".to_string()))),
                );
                pipe_meta.set("type", Some(wrap_type(pipe.type_.clone())));
                pipe_meta.set(
                    "name",
                    Some(literal(LiteralValue::String(pipe.name.clone()))),
                );
                Expression::LiteralMap(pipe_meta.to_literal_map())
            }
            R3TemplateDependencyMetadata::NgModule(ng_module) => {
                let mut ng_meta = DefinitionMap::new();
                ng_meta.set(
                    "kind",
                    Some(literal(LiteralValue::String("ngmodule".to_string()))),
                );
                ng_meta.set("type", Some(wrap_type(ng_module.type_.clone())));
                Expression::LiteralMap(ng_meta.to_literal_map())
            }
        })
        .collect();

    Some(Expression::LiteralArray(LiteralArrayExpr {
        entries: exprs,
        type_: None,
        source_span: None,
    }))
}

/// Check if template has any blocks
fn check_has_blocks(nodes: &[crate::render3::r3_ast::R3Node]) -> bool {
    for node in nodes {
        match node {
            crate::render3::r3_ast::R3Node::DeferredBlock(_)
            | crate::render3::r3_ast::R3Node::IfBlock(_)
            | crate::render3::r3_ast::R3Node::ForLoopBlock(_)
            | crate::render3::r3_ast::R3Node::SwitchBlock(_) => return true,
            crate::render3::r3_ast::R3Node::Element(el) => {
                if check_has_blocks(&el.children) {
                    return true;
                }
            }
            crate::render3::r3_ast::R3Node::Template(tpl) => {
                if check_has_blocks(&tpl.children) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}
