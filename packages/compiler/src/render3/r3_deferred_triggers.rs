//! Render3 Deferred Triggers
//!
//! Corresponds to packages/compiler/src/render3/r3_deferred_triggers.ts
//! Contains deferred trigger parsing

use lazy_static::lazy_static;
use regex::Regex;

// Note: Unused imports removed - would be needed for full viewport trigger options parsing
// use crate::expression_parser::ast::{AST, ASTWithSource, LiteralMap, LiteralPrimitive, LiteralArray, PropertyRead, ImplicitReceiver, ThisReceiver};
use crate::chars;
use crate::expression_parser::lexer::{Lexer, Token, TokenType};
use crate::ml_parser::ast as html;
use crate::parse_util::{ParseError, ParseSourceSpan};
use crate::template_parser::binding_parser::BindingParser;

use super::r3_ast::{
    BoundDeferredTrigger, DeferredBlockPlaceholder, DeferredBlockTriggers, DeferredTrigger,
    HoverDeferredTrigger, IdleDeferredTrigger, ImmediateDeferredTrigger,
    InteractionDeferredTrigger, NeverDeferredTrigger, TimerDeferredTrigger,
    ViewportDeferredTrigger,
};

lazy_static! {
    /// Pattern for a timing value in a trigger
    static ref TIME_PATTERN: Regex = Regex::new(r"^\d+\.?\d*(ms|s)?$").unwrap();

    /// Pattern for a separator between keywords in a trigger expression
    static ref SEPARATOR_PATTERN: Regex = Regex::new(r"^\s$").unwrap();
}

/// Pairs of characters that form syntax that is comma-delimited
fn comma_delimited_syntax() -> std::collections::HashMap<char, char> {
    let mut map = std::collections::HashMap::new();
    map.insert(chars::LBRACE, chars::RBRACE); // Object literals
    map.insert(chars::LBRACKET, chars::RBRACKET); // Array literals
    map.insert(chars::LPAREN, chars::RPAREN); // Function calls
    map
}

/// Possible types of `on` triggers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnTriggerType {
    Idle,
    Timer,
    Interaction,
    Immediate,
    Hover,
    Viewport,
    Never,
}

impl OnTriggerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OnTriggerType::Idle => "idle",
            OnTriggerType::Timer => "timer",
            OnTriggerType::Interaction => "interaction",
            OnTriggerType::Immediate => "immediate",
            OnTriggerType::Hover => "hover",
            OnTriggerType::Viewport => "viewport",
            OnTriggerType::Never => "never",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(OnTriggerType::Idle),
            "timer" => Some(OnTriggerType::Timer),
            "interaction" => Some(OnTriggerType::Interaction),
            "immediate" => Some(OnTriggerType::Immediate),
            "hover" => Some(OnTriggerType::Hover),
            "viewport" => Some(OnTriggerType::Viewport),
            "never" => Some(OnTriggerType::Never),
            _ => None,
        }
    }
}

/// Parsed information about a defer trigger parameter
#[derive(Debug, Clone)]
pub struct ParsedParameter {
    /// Expression of the parameter
    pub expression: String,
    /// Index within the trigger at which the parameter starts
    pub start: usize,
}

/// Parses a `never` deferred trigger
pub fn parse_never_trigger(
    param: &html::BlockParameter,
    triggers: &mut DeferredBlockTriggers,
    errors: &mut Vec<ParseError>,
) {
    let expression = &param.expression;
    let source_span = &param.source_span;

    let never_index = expression.find("never");
    let prefetch_span = get_prefetch_span(expression, source_span);
    let hydrate_span = get_hydrate_span(expression, source_span);

    if let Some(idx) = never_index {
        let never_source_span = ParseSourceSpan::new(
            source_span.start.move_by(idx as i32),
            source_span.start.move_by((idx + "never".len()) as i32),
        );
        track_trigger(
            "never",
            triggers,
            errors,
            DeferredTrigger::Never(NeverDeferredTrigger {
                name_span: Some(never_source_span),
                source_span: source_span.clone(),
                prefetch_span,
                when_or_on_source_span: None,
                hydrate_span,
            }),
        );
    } else {
        errors.push(ParseError::new(
            source_span.clone(),
            "Could not find \"never\" keyword in expression".to_string(),
        ));
    }
}

/// Parses a `when` deferred trigger
pub fn parse_when_trigger(
    param: &html::BlockParameter,
    binding_parser: &mut BindingParser,
    triggers: &mut DeferredBlockTriggers,
    errors: &mut Vec<ParseError>,
) {
    let expression = &param.expression;
    let source_span = &param.source_span;

    let when_index = expression.find("when");
    let prefetch_span = get_prefetch_span(expression, source_span);
    let hydrate_span = get_hydrate_span(expression, source_span);

    if let Some(idx) = when_index {
        let when_source_span = ParseSourceSpan::new(
            source_span.start.move_by(idx as i32),
            source_span.start.move_by((idx + "when".len()) as i32),
        );

        let start = get_trigger_parameters_start(expression, idx + 1);
        if start > 0 {
            let parsed = binding_parser.parse_binding(
                &expression[start..],
                false,
                source_span.clone(),
                source_span.start.offset + start,
            );

            track_trigger(
                "when",
                triggers,
                errors,
                DeferredTrigger::Bound(BoundDeferredTrigger {
                    value: *parsed.ast,
                    source_span: source_span.clone(),
                    prefetch_span,
                    when_source_span,
                    hydrate_span,
                }),
            );
        }
    } else {
        errors.push(ParseError::new(
            source_span.clone(),
            "Could not find \"when\" keyword in expression".to_string(),
        ));
    }
}

/// Parses an `on` trigger
pub fn parse_on_trigger(
    param: &html::BlockParameter,
    binding_parser: &mut BindingParser,
    triggers: &mut DeferredBlockTriggers,
    errors: &mut Vec<ParseError>,
    _placeholder: Option<&DeferredBlockPlaceholder>,
) {
    let expression = &param.expression;
    let source_span = &param.source_span;

    let on_index = expression.find("on");
    let prefetch_span = get_prefetch_span(expression, source_span);
    let hydrate_span = get_hydrate_span(expression, source_span);

    if let Some(idx) = on_index {
        let on_source_span = ParseSourceSpan::new(
            source_span.start.move_by(idx as i32),
            source_span.start.move_by((idx + "on".len()) as i32),
        );

        let start = get_trigger_parameters_start(expression, idx + 1);
        let is_hydration_trigger = expression.starts_with("hydrate");

        let mut parser = OnTriggerParser::new(
            expression.to_string(),
            binding_parser,
            start,
            source_span.clone(),
            triggers,
            errors,
            is_hydration_trigger,
            prefetch_span,
            on_source_span,
            hydrate_span,
        );
        parser.parse();
    } else {
        errors.push(ParseError::new(
            source_span.clone(),
            "Could not find \"on\" keyword in expression".to_string(),
        ));
    }
}

fn get_prefetch_span(expression: &str, source_span: &ParseSourceSpan) -> Option<ParseSourceSpan> {
    if !expression.starts_with("prefetch") {
        return None;
    }
    Some(ParseSourceSpan::new(
        source_span.start.clone(),
        source_span.start.move_by("prefetch".len() as i32),
    ))
}

fn get_hydrate_span(expression: &str, source_span: &ParseSourceSpan) -> Option<ParseSourceSpan> {
    if !expression.starts_with("hydrate") {
        return None;
    }
    Some(ParseSourceSpan::new(
        source_span.start.clone(),
        source_span.start.move_by("hydrate".len() as i32),
    ))
}

/// On trigger parser
struct OnTriggerParser<'a> {
    expression: String,
    binding_parser: &'a BindingParser<'a>,
    start: usize,
    span: ParseSourceSpan,
    triggers: &'a mut DeferredBlockTriggers,
    errors: &'a mut Vec<ParseError>,
    is_hydration_trigger: bool,
    prefetch_span: Option<ParseSourceSpan>,
    on_source_span: ParseSourceSpan,
    hydrate_span: Option<ParseSourceSpan>,
    tokens: Vec<Token>,
    index: usize,
}

impl<'a> OnTriggerParser<'a> {
    fn new(
        expression: String,
        binding_parser: &'a BindingParser<'a>,
        start: usize,
        span: ParseSourceSpan,
        triggers: &'a mut DeferredBlockTriggers,
        errors: &'a mut Vec<ParseError>,
        is_hydration_trigger: bool,
        prefetch_span: Option<ParseSourceSpan>,
        on_source_span: ParseSourceSpan,
        hydrate_span: Option<ParseSourceSpan>,
    ) -> Self {
        let lexer = Lexer::new();
        let tokens = lexer.tokenize(&expression[start..]);

        OnTriggerParser {
            expression,
            binding_parser,
            start,
            span,
            triggers,
            errors,
            is_hydration_trigger,
            prefetch_span,
            on_source_span,
            hydrate_span,
            tokens,
            index: 0,
        }
    }

    fn parse(&mut self) {
        while !self.tokens.is_empty() && self.index < self.tokens.len() {
            let token = self.token().clone();

            if !token.is_identifier() {
                self.unexpected_token(&token);
                break;
            }

            if self.is_followed_by_or_last(chars::COMMA as u8) {
                self.consume_trigger(&token, vec![]);
                self.advance();
            } else if self.is_followed_by_or_last(chars::LPAREN as u8) {
                self.advance(); // Advance to the opening paren
                let prev_errors = self.errors.len();
                let parameters = self.consume_parameters();
                if self.errors.len() != prev_errors {
                    break;
                }
                self.consume_trigger(&token, parameters);
                self.advance(); // Advance past the closing paren
            } else if self.index < self.tokens.len() - 1 {
                self.unexpected_token(&self.tokens[self.index + 1].clone());
            }

            self.advance();
        }
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn is_followed_by_or_last(&self, char: u8) -> bool {
        if self.index == self.tokens.len() - 1 {
            return true;
        }
        self.tokens[self.index + 1].is_character(char as char)
    }

    fn token(&self) -> &Token {
        &self.tokens[self.index.min(self.tokens.len() - 1)]
    }

    fn consume_trigger(&mut self, identifier: &Token, parameters: Vec<ParsedParameter>) {
        let trigger_name_start_span = self
            .span
            .start
            .move_by((self.start + identifier.index - self.tokens[0].index) as i32);
        let name_span = ParseSourceSpan::new(
            trigger_name_start_span.clone(),
            trigger_name_start_span.move_by(identifier.str_value.len() as i32),
        );
        let end_span =
            trigger_name_start_span.move_by((self.token().end - identifier.index) as i32);

        let is_first_trigger = identifier.index == 0;
        let on_source_span = if is_first_trigger {
            Some(self.on_source_span.clone())
        } else {
            None
        };
        let prefetch_source_span = if is_first_trigger {
            self.prefetch_span.clone()
        } else {
            None
        };
        let hydrate_source_span = if is_first_trigger {
            self.hydrate_span.clone()
        } else {
            None
        };
        let source_span = ParseSourceSpan::new(
            if is_first_trigger {
                self.span.start.clone()
            } else {
                trigger_name_start_span.clone()
            },
            end_span,
        );

        let trigger_type = OnTriggerType::from_str(&identifier.str_value);

        match trigger_type {
            Some(OnTriggerType::Idle) => {
                if let Err(e) = self.create_idle_trigger(
                    &parameters,
                    name_span,
                    source_span,
                    prefetch_source_span,
                    on_source_span,
                    hydrate_source_span,
                ) {
                    self.error(identifier, &e);
                }
            }
            Some(OnTriggerType::Timer) => {
                if let Err(e) = self.create_timer_trigger(
                    &parameters,
                    name_span,
                    source_span,
                    prefetch_source_span,
                    on_source_span,
                    hydrate_source_span,
                ) {
                    self.error(identifier, &e);
                }
            }
            Some(OnTriggerType::Immediate) => {
                if let Err(e) = self.create_immediate_trigger(
                    &parameters,
                    name_span,
                    source_span,
                    prefetch_source_span,
                    on_source_span,
                    hydrate_source_span,
                ) {
                    self.error(identifier, &e);
                }
            }
            Some(OnTriggerType::Hover) => {
                if let Err(e) = self.create_hover_trigger(
                    &parameters,
                    name_span,
                    source_span,
                    prefetch_source_span,
                    on_source_span,
                    hydrate_source_span,
                ) {
                    self.error(identifier, &e);
                }
            }
            Some(OnTriggerType::Interaction) => {
                if let Err(e) = self.create_interaction_trigger(
                    &parameters,
                    name_span,
                    source_span,
                    prefetch_source_span,
                    on_source_span,
                    hydrate_source_span,
                ) {
                    self.error(identifier, &e);
                }
            }
            Some(OnTriggerType::Viewport) => {
                if let Err(e) = self.create_viewport_trigger(
                    &parameters,
                    name_span,
                    source_span,
                    prefetch_source_span,
                    on_source_span,
                    hydrate_source_span,
                ) {
                    self.error(identifier, &e);
                }
            }
            _ => {
                self.error(
                    identifier,
                    &format!("Unrecognized trigger type \"{}\"", identifier.str_value),
                );
            }
        }
    }

    fn create_idle_trigger(
        &mut self,
        parameters: &[ParsedParameter],
        name_span: ParseSourceSpan,
        source_span: ParseSourceSpan,
        prefetch_span: Option<ParseSourceSpan>,
        on_source_span: Option<ParseSourceSpan>,
        hydrate_span: Option<ParseSourceSpan>,
    ) -> Result<(), String> {
        if !parameters.is_empty() {
            return Err(format!(
                "\"{}\" trigger cannot have parameters",
                OnTriggerType::Idle.as_str()
            ));
        }

        track_trigger(
            "idle",
            self.triggers,
            self.errors,
            DeferredTrigger::Idle(IdleDeferredTrigger {
                name_span: Some(name_span),
                source_span,
                prefetch_span,
                when_or_on_source_span: on_source_span,
                hydrate_span,
            }),
        );
        Ok(())
    }

    fn create_timer_trigger(
        &mut self,
        parameters: &[ParsedParameter],
        name_span: ParseSourceSpan,
        source_span: ParseSourceSpan,
        prefetch_span: Option<ParseSourceSpan>,
        on_source_span: Option<ParseSourceSpan>,
        hydrate_span: Option<ParseSourceSpan>,
    ) -> Result<(), String> {
        if parameters.len() != 1 {
            return Err(format!(
                "\"{}\" trigger must have exactly one parameter",
                OnTriggerType::Timer.as_str()
            ));
        }

        let delay = parse_deferred_time(&parameters[0].expression).ok_or_else(|| {
            format!(
                "Could not parse time value of trigger \"{}\"",
                OnTriggerType::Timer.as_str()
            )
        })?;

        track_trigger(
            "timer",
            self.triggers,
            self.errors,
            DeferredTrigger::Timer(TimerDeferredTrigger {
                delay,
                name_span,
                source_span,
                prefetch_span,
                on_source_span,
                hydrate_span,
            }),
        );
        Ok(())
    }

    fn create_immediate_trigger(
        &mut self,
        parameters: &[ParsedParameter],
        name_span: ParseSourceSpan,
        source_span: ParseSourceSpan,
        prefetch_span: Option<ParseSourceSpan>,
        on_source_span: Option<ParseSourceSpan>,
        hydrate_span: Option<ParseSourceSpan>,
    ) -> Result<(), String> {
        if !parameters.is_empty() {
            return Err(format!(
                "\"{}\" trigger cannot have parameters",
                OnTriggerType::Immediate.as_str()
            ));
        }

        track_trigger(
            "immediate",
            self.triggers,
            self.errors,
            DeferredTrigger::Immediate(ImmediateDeferredTrigger {
                name_span: Some(name_span),
                source_span,
                prefetch_span,
                when_or_on_source_span: on_source_span,
                hydrate_span,
            }),
        );
        Ok(())
    }

    fn create_hover_trigger(
        &mut self,
        parameters: &[ParsedParameter],
        name_span: ParseSourceSpan,
        source_span: ParseSourceSpan,
        prefetch_span: Option<ParseSourceSpan>,
        on_source_span: Option<ParseSourceSpan>,
        hydrate_span: Option<ParseSourceSpan>,
    ) -> Result<(), String> {
        self.validate_reference_trigger(OnTriggerType::Hover, parameters)?;

        let reference = parameters.first().map(|p| p.expression.clone());

        track_trigger(
            "hover",
            self.triggers,
            self.errors,
            DeferredTrigger::Hover(HoverDeferredTrigger {
                reference: reference.map(|r| r.into()),
                name_span,
                source_span,
                prefetch_span,
                on_source_span,
                hydrate_span,
            }),
        );
        Ok(())
    }

    fn create_interaction_trigger(
        &mut self,
        parameters: &[ParsedParameter],
        name_span: ParseSourceSpan,
        source_span: ParseSourceSpan,
        prefetch_span: Option<ParseSourceSpan>,
        on_source_span: Option<ParseSourceSpan>,
        hydrate_span: Option<ParseSourceSpan>,
    ) -> Result<(), String> {
        self.validate_reference_trigger(OnTriggerType::Interaction, parameters)?;

        let reference = parameters.first().map(|p| p.expression.clone());

        track_trigger(
            "interaction",
            self.triggers,
            self.errors,
            DeferredTrigger::Interaction(InteractionDeferredTrigger {
                reference: reference.map(|r| r.into()),
                name_span,
                source_span,
                prefetch_span,
                on_source_span,
                hydrate_span,
            }),
        );
        Ok(())
    }

    fn create_viewport_trigger(
        &mut self,
        parameters: &[ParsedParameter],
        name_span: ParseSourceSpan,
        source_span: ParseSourceSpan,
        prefetch_span: Option<ParseSourceSpan>,
        on_source_span: Option<ParseSourceSpan>,
        hydrate_span: Option<ParseSourceSpan>,
    ) -> Result<(), String> {
        self.validate_reference_trigger(OnTriggerType::Viewport, parameters)?;

        let reference = if parameters.is_empty() || parameters[0].expression.starts_with('{') {
            None
        } else {
            Some(parameters[0].expression.clone())
        };

        // Note: Full options parsing for viewport trigger is simplified here
        let options = None;

        track_trigger(
            "viewport",
            self.triggers,
            self.errors,
            DeferredTrigger::Viewport(ViewportDeferredTrigger {
                reference: reference.map(|r| r.into()),
                options,
                name_span,
                source_span,
                prefetch_span,
                on_source_span,
                hydrate_span,
            }),
        );
        Ok(())
    }

    fn validate_reference_trigger(
        &self,
        trigger_type: OnTriggerType,
        parameters: &[ParsedParameter],
    ) -> Result<(), String> {
        if self.is_hydration_trigger {
            if trigger_type == OnTriggerType::Viewport {
                if parameters.len() > 1 {
                    return Err(format!(
                        "Hydration trigger \"{}\" cannot have more than one parameter",
                        trigger_type.as_str()
                    ));
                }
            } else if !parameters.is_empty() {
                return Err(format!(
                    "Hydration trigger \"{}\" cannot have parameters",
                    trigger_type.as_str()
                ));
            }
        } else if parameters.len() > 1 {
            return Err(format!(
                "\"{}\" trigger can only have zero or one parameters",
                trigger_type.as_str()
            ));
        }
        Ok(())
    }

    fn consume_parameters(&mut self) -> Vec<ParsedParameter> {
        let mut parameters = Vec::new();

        let token = self.token().clone();
        if !token.is_character('(') {
            self.unexpected_token(&token);
            return parameters;
        }

        self.advance();

        let comma_delim = comma_delimited_syntax();
        let mut comma_delim_stack: Vec<char> = Vec::new();
        let mut tokens: Vec<Token> = Vec::new();

        while self.index < self.tokens.len() {
            let token = self.token().clone();

            if token.is_character(')') && comma_delim_stack.is_empty() {
                if !tokens.is_empty() {
                    parameters.push(ParsedParameter {
                        expression: self.token_range_text(&tokens),
                        start: tokens[0].index,
                    });
                }
                break;
            }

            if token.token_type == TokenType::Character {
                if let Some(ch) = token.str_value.chars().next() {
                    if let Some(&closing) = comma_delim.get(&ch) {
                        comma_delim_stack.push(closing);
                    }
                }
            }

            if !comma_delim_stack.is_empty()
                && token.is_character(comma_delim_stack[comma_delim_stack.len() - 1])
            {
                comma_delim_stack.pop();
            }

            if comma_delim_stack.is_empty() && token.is_character(',') && !tokens.is_empty() {
                parameters.push(ParsedParameter {
                    expression: self.token_range_text(&tokens),
                    start: tokens[0].index,
                });
                self.advance();
                tokens.clear();
                continue;
            }

            tokens.push(token);
            self.advance();
        }

        let token = self.token().clone();
        if !token.is_character(')') || !comma_delim_stack.is_empty() {
            self.error(&token, "Unexpected end of expression");
        }

        if self.index < self.tokens.len() - 1 && !self.tokens[self.index + 1].is_character(',') {
            self.unexpected_token(&self.tokens[self.index + 1].clone());
        }

        parameters
    }

    fn token_range_text(&self, tokens: &[Token]) -> String {
        if tokens.is_empty() {
            return String::new();
        }

        self.expression[self.start + tokens[0].index..self.start + tokens[tokens.len() - 1].end]
            .to_string()
    }

    fn error(&mut self, token: &Token, message: &str) {
        let new_start = self.span.start.move_by((self.start + token.index) as i32);
        let new_end = new_start.move_by((token.end - token.index) as i32);
        self.errors.push(ParseError::new(
            ParseSourceSpan::new(new_start, new_end),
            message.to_string(),
        ));
    }

    fn unexpected_token(&mut self, token: &Token) {
        self.error(token, &format!("Unexpected token \"{}\"", token.str_value));
    }
}

/// Adds a trigger to a map of triggers
fn track_trigger(
    name: &str,
    triggers: &mut DeferredBlockTriggers,
    errors: &mut Vec<ParseError>,
    trigger: DeferredTrigger,
) {
    let source_span = match &trigger {
        DeferredTrigger::Bound(t) => t.source_span.clone(),
        DeferredTrigger::Never(t) => t.source_span.clone(),
        DeferredTrigger::Idle(t) => t.source_span.clone(),
        DeferredTrigger::Immediate(t) => t.source_span.clone(),
        DeferredTrigger::Hover(t) => t.source_span.clone(),
        DeferredTrigger::Timer(t) => t.source_span.clone(),
        DeferredTrigger::Interaction(t) => t.source_span.clone(),
        DeferredTrigger::Viewport(t) => t.source_span.clone(),
    };

    let already_exists = match name {
        "when" => triggers.when.is_some(),
        "idle" => triggers.idle.is_some(),
        "immediate" => triggers.immediate.is_some(),
        "hover" => triggers.hover.is_some(),
        "timer" => triggers.timer.is_some(),
        "interaction" => triggers.interaction.is_some(),
        "viewport" => triggers.viewport.is_some(),
        "never" => triggers.never.is_some(),
        _ => false,
    };

    if already_exists {
        errors.push(ParseError::new(
            source_span,
            format!("Duplicate \"{}\" trigger is not allowed", name),
        ));
    } else {
        match (name, trigger) {
            ("when", DeferredTrigger::Bound(t)) => triggers.when = Some(t),
            ("idle", DeferredTrigger::Idle(t)) => triggers.idle = Some(t),
            ("immediate", DeferredTrigger::Immediate(t)) => triggers.immediate = Some(t),
            ("hover", DeferredTrigger::Hover(t)) => triggers.hover = Some(t),
            ("timer", DeferredTrigger::Timer(t)) => triggers.timer = Some(t),
            ("interaction", DeferredTrigger::Interaction(t)) => triggers.interaction = Some(t),
            ("viewport", DeferredTrigger::Viewport(t)) => triggers.viewport = Some(t),
            ("never", DeferredTrigger::Never(t)) => triggers.never = Some(t),
            _ => {}
        }
    }
}

/// Gets the index within an expression at which the trigger parameters start
pub fn get_trigger_parameters_start(value: &str, start_position: usize) -> usize {
    let mut has_found_separator = false;

    for (i, c) in value.chars().enumerate().skip(start_position) {
        if c.is_whitespace() {
            has_found_separator = true;
        } else if has_found_separator {
            return i;
        }
    }

    0
}

/// Parses a time expression from a deferred trigger to milliseconds.
/// Returns None if it cannot be parsed.
pub fn parse_deferred_time(value: &str) -> Option<i64> {
    let trimmed = value.trim();

    if !TIME_PATTERN.is_match(trimmed) {
        return None;
    }

    // Extract number and optional unit
    let mut num_str = String::new();
    let mut unit_str = String::new();

    for c in trimmed.chars() {
        if c.is_ascii_digit() || c == '.' {
            num_str.push(c);
        } else {
            unit_str.push(c);
        }
    }

    let num: f64 = num_str.parse().ok()?;
    let multiplier = if unit_str == "s" { 1000.0 } else { 1.0 };

    Some((num * multiplier) as i64)
}
