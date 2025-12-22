//! Render3 Control Flow
//!
//! Corresponds to packages/compiler/src/render3/r3_control_flow.ts
//! Contains control flow block parsing (if, for, switch)

use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;

use crate::expression_parser::ast::{ASTWithSource, AST};
use crate::i18n;
use crate::ml_parser::ast as html;
use crate::parse_util::{ParseError, ParseSourceSpan};
use crate::template_parser::binding_parser::BindingParser;

use super::r3_ast::{
    ForLoopBlock, ForLoopBlockEmpty, IfBlock, SwitchBlock,
    SwitchBlockCase, UnknownBlock, Variable, BlockNode,
};

lazy_static! {
    /// Pattern for the expression in a for loop block
    static ref FOR_LOOP_EXPRESSION_PATTERN: Regex =
        Regex::new(r"^\s*([0-9A-Za-z_$]*)\s+of\s+([\S\s]*)").unwrap();
    
    /// Pattern for the tracking expression in a for loop block
    static ref FOR_LOOP_TRACK_PATTERN: Regex = Regex::new(r"^track\s+([\S\s]*)").unwrap();
    
    /// Pattern for the `as` expression in a conditional block
    static ref CONDITIONAL_ALIAS_PATTERN: Regex = Regex::new(r"^(as\s+)(.*)").unwrap();
    
    /// Pattern used to identify an `else if` block
    static ref ELSE_IF_PATTERN: Regex = Regex::new(r"^else[^\S\r\n]+if").unwrap();
    
    /// Pattern used to identify a `let` parameter
    static ref FOR_LOOP_LET_PATTERN: Regex = Regex::new(r"^let\s+([\S\s]*)").unwrap();
    
    /// Pattern used to validate a JavaScript identifier
    static ref IDENTIFIER_PATTERN: Regex = Regex::new(r"^[$a-zA-Z_][0-9a-zA-Z_$]*$").unwrap();
    
    /// Pattern to group a string into leading whitespace, non whitespace, and trailing whitespace
    static ref CHARACTERS_IN_SURROUNDING_WHITESPACE_PATTERN: Regex =
        Regex::new(r"(\s*)(\S+)(\s*)").unwrap();
}

/// Helper function to get source_span from html::Node
fn get_node_source_span(node: &html::Node) -> ParseSourceSpan {
    match node {
        html::Node::Attribute(attr) => attr.source_span.clone(),
        html::Node::Comment(comment) => comment.source_span.clone(),
        html::Node::Element(el) => el.source_span.clone(),
        html::Node::Expansion(exp) => exp.source_span.clone(),
        html::Node::ExpansionCase(ec) => ec.source_span.clone(),
        html::Node::Text(text) => text.source_span.clone(),
        html::Node::Block(block) => block.source_span.clone(),
        html::Node::BlockParameter(bp) => bp.source_span.clone(),
        html::Node::Component(comp) => comp.source_span.clone(),
        html::Node::Directive(dir) => dir.source_span.clone(),
        html::Node::LetDeclaration(let_decl) => let_decl.source_span.clone(),
    }
}

/// Names of variables that are allowed to be used in the `let` expression of a `for` loop
/// Returns Vec to preserve deterministic ordering (important for code generation)
fn allowed_for_loop_let_variables() -> Vec<&'static str> {
    // Order matters: $index must come before $count for deterministic output
    vec!["$index", "$first", "$last", "$even", "$odd", "$count"]
}

/// Predicate function that determines if a block with
/// a specific name can be connected to a `for` block.
pub fn is_connected_for_loop_block(name: &str) -> bool {
    name == "empty"
}

/// Predicate function that determines if a block with
/// a specific name can be connected to an `if` block.
pub fn is_connected_if_loop_block(name: &str) -> bool {
    name == "else" || ELSE_IF_PATTERN.is_match(name)
}

/// Result of creating an if block
pub struct CreateIfBlockResult {
    pub node: Option<IfBlock>,
    pub errors: Vec<ParseError>,
}

/// Intermediate if block branch with html children for transformation
pub struct IfBlockBranchInput<'a> {
    pub expression: Option<AST>,
    pub html_children: &'a [html::Node],
    pub expression_alias: Option<Variable>,
    pub block: BlockNode,
    pub i18n: Option<i18n::I18nMeta>,
}

/// Result of pre-processing an if block (before children transformation)
pub struct PreProcessIfBlockResult<'a> {
    pub branches: Vec<IfBlockBranchInput<'a>>,
    pub errors: Vec<ParseError>,
    pub whole_source_span: ParseSourceSpan,
    pub start_source_span: ParseSourceSpan,
    pub end_source_span: Option<ParseSourceSpan>,
    pub name_span: ParseSourceSpan,
}

/// Pre-processes an `if` loop block, returning branches with html children for external transformation
pub fn preprocess_if_block<'a>(
    ast: &'a html::Block,
    connected_blocks: &'a [html::Block],
    binding_parser: &mut BindingParser,
) -> PreProcessIfBlockResult<'a> {
    let mut errors = validate_if_connected_blocks(connected_blocks);
    let mut branches: Vec<IfBlockBranchInput<'a>> = Vec::new();

    if let Some(main_block_params) =
        parse_conditional_block_parameters(ast, &mut errors, binding_parser)
    {
        branches.push(IfBlockBranchInput {
            expression: Some(main_block_params.expression),
            html_children: &ast.children,
            expression_alias: main_block_params.expression_alias,
            block: BlockNode::new(
                ast.name_span.clone(),
                ast.source_span.clone(),
                ast.start_source_span.clone(),
                ast.end_source_span.clone(),
            ),
            i18n: ast.i18n.clone(),
        });
    }

    for block in connected_blocks {
        if ELSE_IF_PATTERN.is_match(&block.name) {
            if let Some(params) =
                parse_conditional_block_parameters(block, &mut errors, binding_parser)
            {
                branches.push(IfBlockBranchInput {
                    expression: Some(params.expression),
                    html_children: &block.children,
                    expression_alias: params.expression_alias,
                    block: BlockNode::new(
                        block.name_span.clone(),
                        block.source_span.clone(),
                        block.start_source_span.clone(),
                        block.end_source_span.clone(),
                    ),
                    i18n: block.i18n.clone(),
                });
            }
        } else if block.name == "else" {
            branches.push(IfBlockBranchInput {
                expression: None,
                html_children: &block.children,
                expression_alias: None,
                block: BlockNode::new(
                    block.name_span.clone(),
                    block.source_span.clone(),
                    block.start_source_span.clone(),
                    block.end_source_span.clone(),
                ),
                i18n: block.i18n.clone(),
            });
        }
    }

    // The outer IfBlock should have a span that encapsulates all branches
    let if_block_start_source_span = if !branches.is_empty() {
        branches[0].block.start_source_span.clone()
    } else {
        ast.start_source_span.clone()
    };

    let if_block_end_source_span = if !branches.is_empty() {
        branches.last().and_then(|b| b.block.end_source_span.clone())
    } else {
        ast.end_source_span.clone()
    };

    let mut whole_source_span = ast.source_span.clone();
    if let Some(last_branch) = branches.last() {
        whole_source_span = ParseSourceSpan::new(
            if_block_start_source_span.start.clone(),
            last_branch.block.source_span.end.clone(),
        );
    }

    PreProcessIfBlockResult {
        branches,
        errors,
        whole_source_span,
        start_source_span: ast.start_source_span.clone(),
        end_source_span: if_block_end_source_span,
        name_span: ast.name_span.clone(),
    }
}

/// Result of creating a for loop
pub struct CreateForLoopResult {
    pub node: Option<ForLoopBlock>,
    pub errors: Vec<ParseError>,
}

/// Creates a `for` loop block from an HTML AST node
pub fn create_for_loop(
    ast: &html::Block,
    connected_blocks: &[html::Block],
    binding_parser: &mut BindingParser,
) -> CreateForLoopResult {
    let mut errors: Vec<ParseError> = Vec::new();
    let params = parse_for_loop_parameters(ast, &mut errors, binding_parser);
    let mut node: Option<ForLoopBlock> = None;
    let mut empty: Option<ForLoopBlockEmpty> = None;

    for block in connected_blocks {
        if block.name == "empty" {
            if empty.is_some() {
                errors.push(ParseError::new(
                    block.source_span.clone(),
                    "@for loop can only have one @empty block".to_string(),
                ));
            } else if !block.parameters.is_empty() {
                errors.push(ParseError::new(
                    block.source_span.clone(),
                    "@empty block cannot have parameters".to_string(),
                ));
            } else {
                empty = Some(ForLoopBlockEmpty {
                    children: vec![], // Would be populated by visitor
                    block: BlockNode::new(
                        block.name_span.clone(),
                        block.source_span.clone(),
                        block.start_source_span.clone(),
                        block.end_source_span.clone(),
                    ),
                    i18n: block.i18n.clone(),
                });
            }
        } else {
            errors.push(ParseError::new(
                block.source_span.clone(),
                format!("Unrecognized @for loop block \"{}\"", block.name),
            ));
        }
    }

    if let Some(params) = params {
        if params.track_by.is_none() {
            errors.push(ParseError::new(
                ast.start_source_span.clone(),
                "@for loop must have a \"track\" expression".to_string(),
            ));
        } else {
            let track_by = params.track_by.unwrap();
            
            // Validate track by expression
            validate_track_by_expression(&track_by.expression, &track_by.keyword_span, &mut errors);

            let end_span = empty
                .as_ref()
                .and_then(|e| e.block.end_source_span.clone())
                .or_else(|| ast.end_source_span.clone());

            let source_span = ParseSourceSpan::new(
                ast.source_span.start.clone(),
                end_span
                    .as_ref()
                    .map(|s| s.end.clone())
                    .unwrap_or_else(|| ast.source_span.end.clone()),
            );

            node = Some(ForLoopBlock {
                item: params.item_name,
                expression: params.expression,
                track_by: track_by.expression,
                track_keyword_span: track_by.keyword_span,
                context_variables: params.context,
                children: vec![], // Would be populated by visitor
                empty: empty.map(Box::new),
                block: BlockNode::new(
                    ast.name_span.clone(),
                    source_span,
                    ast.start_source_span.clone(),
                    end_span,
                ),
                main_block_span: ast.source_span.clone(),
                i18n: ast.i18n.clone(),
            });
        }
    }

    CreateForLoopResult { node, errors }
}

/// Result of creating a switch block
pub struct CreateSwitchBlockResult {
    pub node: Option<SwitchBlock>,
    pub errors: Vec<ParseError>,
}

/// Creates a switch block from an HTML AST node
pub fn create_switch_block(
    ast: &html::Block,
    binding_parser: &mut BindingParser,
) -> CreateSwitchBlockResult {
    let errors = validate_switch_block(ast);
    
    let primary_expression = if !ast.parameters.is_empty() {
        parse_block_parameter_to_binding(&ast.parameters[0], binding_parser, None)
    } else {
        binding_parser.parse_binding("", false, ast.source_span.clone(), 0)
    };

    let mut cases: Vec<SwitchBlockCase> = Vec::new();
    let mut unknown_blocks: Vec<UnknownBlock> = Vec::new();
    let mut default_case: Option<SwitchBlockCase> = None;

    for child in &ast.children {
        if let html::Node::Block(block) = child {
            if (block.name != "case" || block.parameters.is_empty()) && block.name != "default" {
                unknown_blocks.push(UnknownBlock {
                    name: block.name.clone(),
                    source_span: block.source_span.clone(),
                    name_span: block.name_span.clone(),
                });
                continue;
            }

            let is_default = block.name != "case";
            let expression = if !is_default {
                Some(*parse_block_parameter_to_binding(
                    &block.parameters[0],
                    binding_parser,
                    None,
                ).ast)
            } else {
                None
            };

            let case = SwitchBlockCase {
                expression,
                children: vec![], // Would be populated by visitor
                block: BlockNode::new(
                    block.name_span.clone(),
                    block.source_span.clone(),
                    block.start_source_span.clone(),
                    block.end_source_span.clone(),
                ),
                i18n: block.i18n.clone(),
            };

            if is_default {
                default_case = Some(case);
            } else {
                cases.push(case);
            }
        }
    }

    // Ensure default case is last
    if let Some(dc) = default_case {
        cases.push(dc);
    }

    CreateSwitchBlockResult {
        node: Some(SwitchBlock {
            expression: *primary_expression.ast,
            cases,
            unknown_blocks,
            block: BlockNode::new(
                ast.name_span.clone(),
                ast.source_span.clone(),
                ast.start_source_span.clone(),
                ast.end_source_span.clone(),
            ),
        }),
        errors,
    }
}

/// Parsed for loop parameters
struct ForLoopParams {
    item_name: Variable,
    track_by: Option<TrackBy>,
    expression: ASTWithSource,
    context: Vec<Variable>,
}

struct TrackBy {
    expression: ASTWithSource,
    keyword_span: ParseSourceSpan,
}

/// Parses the parameters of a `for` loop block
fn parse_for_loop_parameters(
    block: &html::Block,
    errors: &mut Vec<ParseError>,
    binding_parser: &mut BindingParser,
) -> Option<ForLoopParams> {
    if block.parameters.is_empty() {
        errors.push(ParseError::new(
            block.start_source_span.clone(),
            "@for loop does not have an expression".to_string(),
        ));
        return None;
    }

    let expression_param = &block.parameters[0];
    let secondary_params = &block.parameters[1..];

    let stripped = strip_optional_parentheses(expression_param, errors)?;
    let captures = FOR_LOOP_EXPRESSION_PATTERN.captures(&stripped)?;

    let item_name_str = captures.get(1)?.as_str();
    let raw_expression = captures.get(2)?.as_str();

    if raw_expression.trim().is_empty() {
        errors.push(ParseError::new(
            expression_param.source_span.clone(),
            "Cannot parse expression. @for loop expression must match the pattern \"<identifier> of <expression>\"".to_string(),
        ));
        return None;
    }

    let allowed_vars = allowed_for_loop_let_variables();
    if allowed_vars.contains(&item_name_str) {
        errors.push(ParseError::new(
            expression_param.source_span.clone(),
            format!(
                "@for loop item name cannot be one of {}.",
                allowed_vars.into_iter().collect::<Vec<_>>().join(", ")
            ),
        ));
    }

    let variable_name = expression_param.expression.split(' ').next().unwrap_or("");
    let variable_span = ParseSourceSpan::new(
        expression_param.source_span.start.clone(),
        expression_param
            .source_span
            .start
            .move_by(variable_name.len() as i32),
    );

    let mut result = ForLoopParams {
        item_name: Variable {
            name: item_name_str.to_string(),
            value: "$implicit".to_string(),
            source_span: variable_span.clone(),
            key_span: variable_span,
            value_span: None,
        },
        track_by: None,
        expression: parse_block_parameter_to_binding(
            expression_param,
            binding_parser,
            Some(raw_expression),
        ),
        context: allowed_for_loop_let_variables()
            .into_iter()
            .map(|var_name| {
                let empty_span = ParseSourceSpan::new(
                    block.start_source_span.end.clone(),
                    block.start_source_span.end.clone(),
                );
                Variable {
                    name: var_name.to_string(),
                    value: var_name.to_string(),
                    source_span: empty_span.clone(),
                    key_span: empty_span,
                    value_span: None,
                }
            })
            .collect(),
    };

    for param in secondary_params {
        if let Some(let_captures) = FOR_LOOP_LET_PATTERN.captures(&param.expression) {
            let let_match = let_captures.get(1).map(|m| m.as_str()).unwrap_or("");
            let variables_span = ParseSourceSpan::new(
                param
                    .source_span
                    .start
                    .move_by((param.expression.len() - let_match.len()) as i32),
                param.source_span.end.clone(),
            );
            parse_let_parameter(
                &param.source_span,
                let_match,
                &variables_span,
                item_name_str,
                &mut result.context,
                errors,
            );
            continue;
        }

        if let Some(track_captures) = FOR_LOOP_TRACK_PATTERN.captures(&param.expression) {
            if result.track_by.is_some() {
                errors.push(ParseError::new(
                    param.source_span.clone(),
                    "@for loop can only have one \"track\" expression".to_string(),
                ));
            } else {
                let track_match = track_captures.get(1).map(|m| m.as_str()).unwrap_or("");
                let expression =
                    parse_block_parameter_to_binding(param, binding_parser, Some(track_match));
                
                if matches!(*expression.ast, AST::EmptyExpr(_)) {
                    errors.push(ParseError::new(
                        block.start_source_span.clone(),
                        "@for loop must have a \"track\" expression".to_string(),
                    ));
                }

                let keyword_span = ParseSourceSpan::new(
                    param.source_span.start.clone(),
                    param.source_span.start.move_by("track".len() as i32),
                );
                result.track_by = Some(TrackBy {
                    expression,
                    keyword_span,
                });
            }
            continue;
        }

        errors.push(ParseError::new(
            param.source_span.clone(),
            format!("Unrecognized @for loop parameter \"{}\"", param.expression),
        ));
    }

    Some(result)
}

fn validate_track_by_expression(
    expression: &ASTWithSource,
    parse_source_span: &ParseSourceSpan,
    errors: &mut Vec<ParseError>,
) {
    if contains_pipe(&expression.ast) {
        errors.push(ParseError::new(
            parse_source_span.clone(),
            "Cannot use pipes in track expressions".to_string(),
        ));
    }
}

/// Parses the `let` parameter of a `for` loop block
fn parse_let_parameter(
    source_span: &ParseSourceSpan,
    expression: &str,
    span: &ParseSourceSpan,
    loop_item_name: &str,
    context: &mut Vec<Variable>,
    errors: &mut Vec<ParseError>,
) {
    let allowed_vars = allowed_for_loop_let_variables();
    let parts: Vec<&str> = expression.split(',').collect();
    let mut start_span = span.start.clone();

    for part in parts {
        let expression_parts: Vec<&str> = part.split('=').collect();
        let name = if expression_parts.len() == 2 {
            expression_parts[0].trim()
        } else {
            ""
        };
        let variable_name = if expression_parts.len() == 2 {
            expression_parts[1].trim()
        } else {
            ""
        };

        if name.is_empty() || variable_name.is_empty() {
            errors.push(ParseError::new(
                source_span.clone(),
                "Invalid @for loop \"let\" parameter. Parameter should match the pattern \"<name> = <variable name>\"".to_string(),
            ));
        } else if !allowed_vars.contains(&variable_name) {
            errors.push(ParseError::new(
                source_span.clone(),
                format!(
                    "Unknown \"let\" parameter variable \"{}\". The allowed variables are: {}",
                    variable_name,
                    allowed_vars.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
            ));
        } else if name == loop_item_name {
            errors.push(ParseError::new(
                source_span.clone(),
                format!(
                    "Invalid @for loop \"let\" parameter. Variable cannot be called \"{}\"",
                    loop_item_name
                ),
            ));
        } else if context.iter().any(|v| v.name == name) {
            errors.push(ParseError::new(
                source_span.clone(),
                format!("Duplicate \"let\" parameter variable \"{}\"", variable_name),
            ));
        } else {
            let key_span = span.clone(); // Simplified span calculation
            let value_span = None; // Simplified

            let var_source_span = ParseSourceSpan::new(
                key_span.start.clone(),
                value_span
                    .as_ref()
                    .map(|s: &ParseSourceSpan| s.end.clone())
                    .unwrap_or_else(|| key_span.end.clone()),
            );

            context.push(Variable {
                name: name.to_string(),
                value: variable_name.to_string(),
                source_span: var_source_span,
                key_span,
                value_span,
            });
        }

        start_span = start_span.move_by((part.len() + 1) as i32);
    }
}

/// Checks that the shape of the blocks connected to an `@if` block is correct
fn validate_if_connected_blocks(connected_blocks: &[html::Block]) -> Vec<ParseError> {
    let mut errors: Vec<ParseError> = Vec::new();
    let mut has_else = false;

    for (i, block) in connected_blocks.iter().enumerate() {
        if block.name == "else" {
            if has_else {
                errors.push(ParseError::new(
                    block.start_source_span.clone(),
                    "Conditional can only have one @else block".to_string(),
                ));
            } else if connected_blocks.len() > 1 && i < connected_blocks.len() - 1 {
                errors.push(ParseError::new(
                    block.start_source_span.clone(),
                    "@else block must be last inside the conditional".to_string(),
                ));
            } else if !block.parameters.is_empty() {
                errors.push(ParseError::new(
                    block.start_source_span.clone(),
                    "@else block cannot have parameters".to_string(),
                ));
            }
            has_else = true;
        } else if !ELSE_IF_PATTERN.is_match(&block.name) {
            errors.push(ParseError::new(
                block.start_source_span.clone(),
                format!("Unrecognized conditional block @{}", block.name),
            ));
        }
    }

    errors
}

/// Checks that the shape of a `switch` block is valid
fn validate_switch_block(ast: &html::Block) -> Vec<ParseError> {
    let mut errors: Vec<ParseError> = Vec::new();
    let mut has_default = false;

    if ast.parameters.len() != 1 {
        errors.push(ParseError::new(
            ast.start_source_span.clone(),
            "@switch block must have exactly one parameter".to_string(),
        ));
        return errors;
    }

    for child in &ast.children {
        match child {
            html::Node::Comment(_) => continue,
            html::Node::Text(text) if text.value.trim().is_empty() => continue,
            html::Node::Block(block) => {
                if block.name != "case" && block.name != "default" {
                    errors.push(ParseError::new(
                        block.source_span.clone(),
                        "@switch block can only contain @case and @default blocks".to_string(),
                    ));
                    continue;
                }

                if block.name == "default" {
                    if has_default {
                        errors.push(ParseError::new(
                            block.start_source_span.clone(),
                            "@switch block can only have one @default block".to_string(),
                        ));
                    } else if !block.parameters.is_empty() {
                        errors.push(ParseError::new(
                            block.start_source_span.clone(),
                            "@default block cannot have parameters".to_string(),
                        ));
                    }
                    has_default = true;
                } else if block.name == "case" && block.parameters.len() != 1 {
                    errors.push(ParseError::new(
                        block.start_source_span.clone(),
                        "@case block must have exactly one parameter".to_string(),
                    ));
                }
            }
            _ => {
                errors.push(ParseError::new(
                    get_node_source_span(child),
                    "@switch block can only contain @case and @default blocks".to_string(),
                ));
            }
        }
    }

    errors
}

/// Parses a block parameter into a binding AST
fn parse_block_parameter_to_binding(
    ast: &html::BlockParameter,
    binding_parser: &mut BindingParser,
    part: Option<&str>,
) -> ASTWithSource {
    let (start, end) = if let Some(part_str) = part {
        let start = ast.expression.rfind(part_str).unwrap_or(0);
        (start, start + part_str.len())
    } else {
        (0, ast.expression.len())
    };

    binding_parser.parse_binding(
        &ast.expression[start..end],
        false,
        ast.source_span.clone(),
        ast.source_span.start.offset + start,
    )
}

/// Conditional block parameters result
struct ConditionalBlockParams {
    expression: AST,
    expression_alias: Option<Variable>,
}

/// Parses the parameter of a conditional block (`if` or `else if`)
fn parse_conditional_block_parameters(
    block: &html::Block,
    errors: &mut Vec<ParseError>,
    binding_parser: &mut BindingParser,
) -> Option<ConditionalBlockParams> {
    if block.parameters.is_empty() {
        errors.push(ParseError::new(
            block.start_source_span.clone(),
            "Conditional block does not have an expression".to_string(),
        ));
        return None;
    }

    let expression = parse_block_parameter_to_binding(&block.parameters[0], binding_parser, None);
    let mut expression_alias: Option<Variable> = None;

    // Start from 1 since we processed the first parameter already
    for param in &block.parameters[1..] {
        if let Some(alias_captures) = CONDITIONAL_ALIAS_PATTERN.captures(&param.expression) {
            if block.name != "if" && !ELSE_IF_PATTERN.is_match(&block.name) {
                errors.push(ParseError::new(
                    param.source_span.clone(),
                    "\"as\" expression is only allowed on `@if` and `@else if` blocks".to_string(),
                ));
            } else if expression_alias.is_some() {
                errors.push(ParseError::new(
                    param.source_span.clone(),
                    "Conditional can only have one \"as\" expression".to_string(),
                ));
            } else {
                let as_prefix = alias_captures.get(1).map(|m| m.as_str()).unwrap_or("");
                let name = alias_captures
                    .get(2)
                    .map(|m| m.as_str().trim())
                    .unwrap_or("");

                if IDENTIFIER_PATTERN.is_match(name) {
                    let variable_start = param.source_span.start.move_by(as_prefix.len() as i32);
                    let variable_span = ParseSourceSpan::new(
                        variable_start.clone(),
                        variable_start.move_by(name.len() as i32),
                    );
                    expression_alias = Some(Variable {
                        name: name.to_string(),
                        value: name.to_string(),
                        source_span: variable_span.clone(),
                        key_span: variable_span,
                        value_span: None,
                    });
                } else {
                    errors.push(ParseError::new(
                        param.source_span.clone(),
                        "\"as\" expression must be a valid JavaScript identifier".to_string(),
                    ));
                }
            }
        } else {
            errors.push(ParseError::new(
                param.source_span.clone(),
                format!("Unrecognized conditional parameter \"{}\"", param.expression),
            ));
        }
    }

    Some(ConditionalBlockParams {
        expression: *expression.ast,
        expression_alias,
    })
}

/// Strips optional parentheses around from a control from expression parameter
fn strip_optional_parentheses(
    param: &html::BlockParameter,
    errors: &mut Vec<ParseError>,
) -> Option<String> {
    let expression = &param.expression;
    let mut open_parens = 0;
    let mut start = 0;
    let mut end = expression.len().saturating_sub(1);

    let chars: Vec<char> = expression.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if ch == '(' {
            start = i + 1;
            open_parens += 1;
        } else if ch.is_whitespace() {
            continue;
        } else {
            break;
        }
    }

    if open_parens == 0 {
        return Some(expression.clone());
    }

    for i in (0..expression.len()).rev() {
        let ch = chars[i];

        if ch == ')' {
            end = i;
            open_parens -= 1;
            if open_parens == 0 {
                break;
            }
        } else if ch.is_whitespace() {
            continue;
        } else {
            break;
        }
    }

    if open_parens != 0 {
        errors.push(ParseError::new(
            param.source_span.clone(),
            "Unclosed parentheses in expression".to_string(),
        ));
        return None;
    }

    Some(expression[start..end].to_string())
}

/// Check if an AST contains a pipe
fn contains_pipe(ast: &AST) -> bool {
    match ast {
        AST::BindingPipe(_) => true,
        AST::Binary(b) => contains_pipe(&*b.left) || contains_pipe(&*b.right),
        AST::Chain(c) => c.expressions.iter().any(|e| contains_pipe(&**e)),
        AST::Conditional(c) => {
            contains_pipe(&*c.condition) || contains_pipe(&*c.true_exp) || contains_pipe(&*c.false_exp)
        }
        AST::PropertyRead(p) => contains_pipe(&*p.receiver),
        AST::SafePropertyRead(p) => contains_pipe(&*p.receiver),
        AST::KeyedRead(k) => contains_pipe(&*k.receiver) || contains_pipe(&*k.key),
        AST::SafeKeyedRead(k) => contains_pipe(&*k.receiver) || contains_pipe(&*k.key),
        AST::LiteralArray(a) => a.expressions.iter().any(|e| contains_pipe(&**e)),
        AST::LiteralMap(m) => m.values.iter().any(|e| contains_pipe(&**e)),
        AST::Interpolation(i) => i.expressions.iter().any(|e| contains_pipe(&**e)),
        AST::Call(c) => contains_pipe(&*c.receiver) || c.args.iter().any(|e| contains_pipe(&**e)),
        AST::SafeCall(c) => contains_pipe(&*c.receiver) || c.args.iter().any(|e| contains_pipe(&**e)),
        AST::PrefixNot(p) => contains_pipe(&*p.expression),
        AST::Unary(u) => contains_pipe(&*u.expr),
        AST::TypeofExpression(t) => contains_pipe(&*t.expression),
        AST::VoidExpression(v) => contains_pipe(&*v.expression),
        AST::NonNullAssert(n) => contains_pipe(&*n.expression),
        AST::ParenthesizedExpression(p) => contains_pipe(&*p.expression),
        AST::PropertyWrite(p) => contains_pipe(&*p.receiver) || contains_pipe(&*p.value),
        AST::KeyedWrite(k) => contains_pipe(&*k.receiver) || contains_pipe(&*k.key) || contains_pipe(&*k.value),
        AST::TemplateLiteral(t) => t.expressions.iter().any(|e| contains_pipe(&**e)),
        AST::TaggedTemplateLiteral(t) => contains_pipe(&*t.tag) || t.template.expressions.iter().any(|e| contains_pipe(&**e)),
        _ => false, // Leaf nodes
    }
}
