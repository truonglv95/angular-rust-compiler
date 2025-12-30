//! Render3 Deferred Blocks
//!
//! Corresponds to packages/compiler/src/render3/r3_deferred_blocks.ts
//! Contains deferred block creation and parsing

use lazy_static::lazy_static;
use regex::Regex;

use crate::ml_parser::ast as html;
use crate::parse_util::{ParseError, ParseSourceSpan};
use crate::template_parser::binding_parser::BindingParser;

use super::r3_ast::{
    BlockNode, DeferredBlock, DeferredBlockError, DeferredBlockLoading, DeferredBlockPlaceholder,
    DeferredBlockTriggers,
};
use super::r3_deferred_triggers::{
    get_trigger_parameters_start, parse_deferred_time, parse_never_trigger, parse_on_trigger,
    parse_when_trigger,
};

lazy_static! {
    /// Pattern to identify a `prefetch when` trigger
    static ref PREFETCH_WHEN_PATTERN: Regex = Regex::new(r"^prefetch\s+when\s").unwrap();

    /// Pattern to identify a `prefetch on` trigger
    static ref PREFETCH_ON_PATTERN: Regex = Regex::new(r"^prefetch\s+on\s").unwrap();

    /// Pattern to identify a `hydrate when` trigger
    static ref HYDRATE_WHEN_PATTERN: Regex = Regex::new(r"^hydrate\s+when\s").unwrap();

    /// Pattern to identify a `hydrate on` trigger
    static ref HYDRATE_ON_PATTERN: Regex = Regex::new(r"^hydrate\s+on\s").unwrap();

    /// Pattern to identify a `hydrate never` trigger
    static ref HYDRATE_NEVER_PATTERN: Regex = Regex::new(r"^hydrate\s+never(\s*)$").unwrap();

    /// Pattern to identify a `minimum` parameter in a block
    static ref MINIMUM_PARAMETER_PATTERN: Regex = Regex::new(r"^minimum\s").unwrap();

    /// Pattern to identify a `after` parameter in a block
    static ref AFTER_PARAMETER_PATTERN: Regex = Regex::new(r"^after\s").unwrap();

    /// Pattern to identify a `when` parameter in a block
    static ref WHEN_PARAMETER_PATTERN: Regex = Regex::new(r"^when\s").unwrap();

    /// Pattern to identify a `on` parameter in a block
    static ref ON_PARAMETER_PATTERN: Regex = Regex::new(r"^on\s").unwrap();
}

/// Predicate function that determines if a block with a specific name can be connected to a `defer` block
pub fn is_connected_defer_loop_block(name: &str) -> bool {
    name == "placeholder" || name == "loading" || name == "error"
}

/// Result of creating a deferred block
pub struct CreateDeferredBlockResult {
    pub node: DeferredBlock,
    pub errors: Vec<ParseError>,
}

/// Creates a deferred block from an HTML AST node
pub fn create_deferred_block(
    ast: &html::Block,
    connected_blocks: &[html::Block],
    binding_parser: &mut BindingParser,
) -> CreateDeferredBlockResult {
    let mut errors: Vec<ParseError> = Vec::new();

    let (placeholder, loading, error) = parse_connected_blocks(connected_blocks, &mut errors);
    let (triggers, prefetch_triggers, hydrate_triggers) =
        parse_primary_triggers(ast, binding_parser, &mut errors, placeholder.as_ref());

    // The `defer` block has a main span encompassing all of the connected branches as well
    let mut last_end_source_span = ast.end_source_span.clone();
    let mut end_of_last_source_span = ast.source_span.end.clone();

    if !connected_blocks.is_empty() {
        let last_connected_block = &connected_blocks[connected_blocks.len() - 1];
        last_end_source_span = last_connected_block.end_source_span.clone();
        end_of_last_source_span = last_connected_block.source_span.end.clone();
    }

    let source_span_with_connected_blocks =
        ParseSourceSpan::new(ast.source_span.start.clone(), end_of_last_source_span);

    let node = DeferredBlock {
        children: vec![], // Would be populated by visitor
        triggers,
        prefetch_triggers,
        hydrate_triggers,
        placeholder: placeholder.map(Box::new),
        loading: loading.map(Box::new),
        error: error.map(Box::new),
        block: BlockNode::new(
            ast.name_span.clone(),
            source_span_with_connected_blocks,
            ast.start_source_span.clone(),
            last_end_source_span,
        ),
        main_block_span: ast.source_span.clone(),
        i18n: ast.i18n.clone(),
    };

    CreateDeferredBlockResult { node, errors }
}

fn parse_connected_blocks(
    connected_blocks: &[html::Block],
    errors: &mut Vec<ParseError>,
) -> (
    Option<DeferredBlockPlaceholder>,
    Option<DeferredBlockLoading>,
    Option<DeferredBlockError>,
) {
    let mut placeholder: Option<DeferredBlockPlaceholder> = None;
    let mut loading: Option<DeferredBlockLoading> = None;
    let mut error: Option<DeferredBlockError> = None;

    for block in connected_blocks {
        if !is_connected_defer_loop_block(&block.name) {
            errors.push(ParseError::new(
                block.start_source_span.clone(),
                format!("Unrecognized block \"@{}\"", block.name),
            ));
            break;
        }

        match block.name.as_ref() {
            "placeholder" => {
                if placeholder.is_some() {
                    errors.push(ParseError::new(
                        block.start_source_span.clone(),
                        "@defer block can only have one @placeholder block".to_string(),
                    ));
                } else {
                    match parse_placeholder_block(block) {
                        Ok(p) => placeholder = Some(p),
                        Err(e) => errors.push(ParseError::new(block.start_source_span.clone(), e)),
                    }
                }
            }
            "loading" => {
                if loading.is_some() {
                    errors.push(ParseError::new(
                        block.start_source_span.clone(),
                        "@defer block can only have one @loading block".to_string(),
                    ));
                } else {
                    match parse_loading_block(block) {
                        Ok(l) => loading = Some(l),
                        Err(e) => errors.push(ParseError::new(block.start_source_span.clone(), e)),
                    }
                }
            }
            "error" => {
                if error.is_some() {
                    errors.push(ParseError::new(
                        block.start_source_span.clone(),
                        "@defer block can only have one @error block".to_string(),
                    ));
                } else {
                    match parse_error_block(block) {
                        Ok(e) => error = Some(e),
                        Err(e) => errors.push(ParseError::new(block.start_source_span.clone(), e)),
                    }
                }
            }
            _ => {}
        }
    }

    (placeholder, loading, error)
}

fn parse_placeholder_block(ast: &html::Block) -> Result<DeferredBlockPlaceholder, String> {
    let mut minimum_time: Option<i64> = None;

    for param in &ast.parameters {
        if MINIMUM_PARAMETER_PATTERN.is_match(&param.expression) {
            if minimum_time.is_some() {
                return Err(
                    "@placeholder block can only have one \"minimum\" parameter".to_string()
                );
            }

            let start = get_trigger_parameters_start(&param.expression, 0);
            let parsed_time = parse_deferred_time(&param.expression[start..]);

            if parsed_time.is_none() {
                return Err("Could not parse time value of parameter \"minimum\"".to_string());
            }

            minimum_time = parsed_time;
        } else {
            return Err(format!(
                "Unrecognized parameter in @placeholder block: \"{}\"",
                param.expression
            ));
        }
    }

    Ok(DeferredBlockPlaceholder {
        children: vec![], // Would be populated by visitor
        minimum_time,
        block: BlockNode::new(
            ast.name_span.clone(),
            ast.source_span.clone(),
            ast.start_source_span.clone(),
            ast.end_source_span.clone(),
        ),
        i18n: ast.i18n.clone(),
    })
}

fn parse_loading_block(ast: &html::Block) -> Result<DeferredBlockLoading, String> {
    let mut after_time: Option<i64> = None;
    let mut minimum_time: Option<i64> = None;

    for param in &ast.parameters {
        if AFTER_PARAMETER_PATTERN.is_match(&param.expression) {
            if after_time.is_some() {
                return Err("@loading block can only have one \"after\" parameter".to_string());
            }

            let start = get_trigger_parameters_start(&param.expression, 0);
            let parsed_time = parse_deferred_time(&param.expression[start..]);

            if parsed_time.is_none() {
                return Err("Could not parse time value of parameter \"after\"".to_string());
            }

            after_time = parsed_time;
        } else if MINIMUM_PARAMETER_PATTERN.is_match(&param.expression) {
            if minimum_time.is_some() {
                return Err("@loading block can only have one \"minimum\" parameter".to_string());
            }

            let start = get_trigger_parameters_start(&param.expression, 0);
            let parsed_time = parse_deferred_time(&param.expression[start..]);

            if parsed_time.is_none() {
                return Err("Could not parse time value of parameter \"minimum\"".to_string());
            }

            minimum_time = parsed_time;
        } else {
            return Err(format!(
                "Unrecognized parameter in @loading block: \"{}\"",
                param.expression
            ));
        }
    }

    Ok(DeferredBlockLoading {
        children: vec![], // Would be populated by visitor
        after_time,
        minimum_time,
        block: BlockNode::new(
            ast.name_span.clone(),
            ast.source_span.clone(),
            ast.start_source_span.clone(),
            ast.end_source_span.clone(),
        ),
        i18n: ast.i18n.clone(),
    })
}

fn parse_error_block(ast: &html::Block) -> Result<DeferredBlockError, String> {
    if !ast.parameters.is_empty() {
        return Err("@error block cannot have parameters".to_string());
    }

    Ok(DeferredBlockError {
        children: vec![], // Would be populated by visitor
        block: BlockNode::new(
            ast.name_span.clone(),
            ast.source_span.clone(),
            ast.start_source_span.clone(),
            ast.end_source_span.clone(),
        ),
        i18n: ast.i18n.clone(),
    })
}

fn parse_primary_triggers(
    ast: &html::Block,
    binding_parser: &mut BindingParser,
    errors: &mut Vec<ParseError>,
    placeholder: Option<&DeferredBlockPlaceholder>,
) -> (
    DeferredBlockTriggers,
    DeferredBlockTriggers,
    DeferredBlockTriggers,
) {
    let mut triggers = DeferredBlockTriggers::default();
    let mut prefetch_triggers = DeferredBlockTriggers::default();
    let mut hydrate_triggers = DeferredBlockTriggers::default();

    for param in &ast.parameters {
        if WHEN_PARAMETER_PATTERN.is_match(&param.expression) {
            parse_when_trigger(param, binding_parser, &mut triggers, errors);
        } else if ON_PARAMETER_PATTERN.is_match(&param.expression) {
            parse_on_trigger(param, binding_parser, &mut triggers, errors, placeholder);
        } else if PREFETCH_WHEN_PATTERN.is_match(&param.expression) {
            parse_when_trigger(param, binding_parser, &mut prefetch_triggers, errors);
        } else if PREFETCH_ON_PATTERN.is_match(&param.expression) {
            parse_on_trigger(
                param,
                binding_parser,
                &mut prefetch_triggers,
                errors,
                placeholder,
            );
        } else if HYDRATE_WHEN_PATTERN.is_match(&param.expression) {
            parse_when_trigger(param, binding_parser, &mut hydrate_triggers, errors);
        } else if HYDRATE_ON_PATTERN.is_match(&param.expression) {
            parse_on_trigger(
                param,
                binding_parser,
                &mut hydrate_triggers,
                errors,
                placeholder,
            );
        } else if HYDRATE_NEVER_PATTERN.is_match(&param.expression) {
            parse_never_trigger(param, &mut hydrate_triggers, errors);
        } else {
            errors.push(ParseError::new(
                param.source_span.clone(),
                "Unrecognized trigger".to_string(),
            ));
        }
    }

    // Check for conflicting hydrate triggers
    if hydrate_triggers.never.is_some() {
        let has_other_hydrate_triggers = hydrate_triggers.when.is_some()
            || hydrate_triggers.idle.is_some()
            || hydrate_triggers.immediate.is_some()
            || hydrate_triggers.hover.is_some()
            || hydrate_triggers.timer.is_some()
            || hydrate_triggers.interaction.is_some()
            || hydrate_triggers.viewport.is_some();

        if has_other_hydrate_triggers {
            errors.push(ParseError::new(
                ast.start_source_span.clone(),
                "Cannot specify additional `hydrate` triggers if `hydrate never` is present"
                    .to_string(),
            ));
        }
    }

    (triggers, prefetch_triggers, hydrate_triggers)
}
