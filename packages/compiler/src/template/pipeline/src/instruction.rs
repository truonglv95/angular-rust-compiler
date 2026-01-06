//! Helpers for generating calls to Ivy instructions.
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/instruction.ts

use crate::output::output_ast as o;
use crate::parse_util::ParseSourceSpan;
use crate::render3::r3_identifiers::Identifiers;
use o::ExternalReference;

pub fn call(
    fn_: ExternalReference,
    args: Vec<o::Expression>,
    source_span: Option<ParseSourceSpan>,
) -> o::Statement {
    let expr = o::import_ref(fn_).call_fn(args, source_span.clone(), None);
    o::Statement::Expression(o::ExpressionStatement { expr, source_span })
}

// Update helper as well, same return type as it produces a Statement
pub fn call_update(
    fn_: ExternalReference,
    args: Vec<o::Expression>,
    source_span: Option<ParseSourceSpan>,
) -> o::Statement {
    call(fn_, args, source_span)
}

fn element_or_container_base<S: AsRef<str>>(
    instruction: o::ExternalReference,
    slot: i32,
    tag: Option<S>,
    const_index: Option<i32>,
    local_ref_index: Option<i32>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    let mut args = vec![*o::literal(slot as f64)]; // unbox literal

    if let Some(t) = tag {
        args.push(*o::literal(t.as_ref().to_string()));
    }

    if let Some(local_ref) = local_ref_index {
        args.push(*o::literal(match const_index {
            Some(i) => o::LiteralValue::from(i as f64),
            None => o::LiteralValue::Null,
        }));
        args.push(*o::literal(local_ref as f64));
    } else if let Some(const_i) = const_index {
        args.push(*o::literal(const_i as f64));
    }

    call(instruction, args, Some(source_span))
}

pub fn element_start<S: AsRef<str>>(
    slot: i32,
    tag: S,
    const_index: Option<i32>,
    local_ref_index: Option<i32>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    element_or_container_base(
        Identifiers::element_start(),
        slot,
        Some(tag),
        const_index,
        local_ref_index,
        source_span,
    )
}

pub fn dom_element_start<S: AsRef<str>>(
    slot: i32,
    tag: S,
    const_index: Option<i32>,
    local_ref_index: Option<i32>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    element_or_container_base(
        Identifiers::dom_element_start(),
        slot,
        Some(tag),
        const_index,
        local_ref_index,
        source_span,
    )
}

pub fn element<S: AsRef<str>>(
    slot: i32,
    tag: S,
    const_index: Option<i32>,
    local_ref_index: Option<i32>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    element_or_container_base(
        Identifiers::element(),
        slot,
        Some(tag),
        const_index,
        local_ref_index,
        source_span,
    )
}

pub fn element_end(source_span: Option<ParseSourceSpan>) -> o::Statement {
    call(Identifiers::element_end(), vec![], source_span)
}

pub fn dom_element_end(source_span: Option<ParseSourceSpan>) -> o::Statement {
    call(Identifiers::dom_element_end(), vec![], source_span)
}

pub fn text<S: AsRef<str>>(
    slot: i32,
    initial_value: S,
    source_span: Option<ParseSourceSpan>,
) -> o::Statement {
    let mut args = vec![*o::literal(slot as f64)];
    let val_ref = initial_value.as_ref();
    if !val_ref.is_empty() {
        args.push(*o::literal(val_ref));
    }
    call(Identifiers::text(), args, source_span)
}

pub fn pipe<S: AsRef<str>>(slot: i32, name: S) -> o::Statement {
    call(
        Identifiers::pipe(),
        vec![*o::literal(slot as f64), *o::literal(name.as_ref())],
        None,
    )
}

pub fn advance(delta: i32, source_span: ParseSourceSpan) -> o::Statement {
    let args = if delta > 1 {
        vec![*o::literal(delta as f64)]
    } else {
        vec![]
    };
    call(Identifiers::advance(), args, Some(source_span))
}

pub fn property<S: AsRef<str>>(
    name: S,
    expression: o::Expression,
    sanitizer: Option<o::Expression>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    let mut args = vec![*o::literal(name.as_ref())];
    args.push(expression);
    if let Some(san) = sanitizer {
        args.push(san);
    }
    call(Identifiers::property(), args, Some(source_span))
}

pub fn dom_property<S: AsRef<str>>(
    name: S,
    expression: o::Expression,
    sanitizer: Option<o::Expression>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    let mut args = vec![*o::literal(name.as_ref())];
    args.push(expression);
    if let Some(san) = sanitizer {
        args.push(san);
    }
    call(Identifiers::dom_property(), args, Some(source_span))
}

pub fn attribute<S: AsRef<str>>(
    name: S,
    expression: o::Expression,
    sanitizer: Option<o::Expression>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    let mut args = vec![*o::literal(name.as_ref())];
    args.push(expression);
    if let Some(san) = sanitizer {
        args.push(san);
    }
    call(Identifiers::attribute(), args, Some(source_span))
}

pub fn disable_bindings() -> o::Statement {
    call(Identifiers::disable_bindings(), vec![], None)
}

pub fn enable_bindings() -> o::Statement {
    call(Identifiers::enable_bindings(), vec![], None)
}

pub fn listener(
    name: String,
    handler_fn: o::Expression,
    event_target: Option<String>,
    source_span: Option<ParseSourceSpan>,
) -> o::Statement {
    let mut args = vec![*o::literal(name), handler_fn];
    if let Some(target) = event_target {
        args.push(*o::literal(target));
    }
    call(Identifiers::listener(), args, source_span)
}

pub fn two_way_listener<S: AsRef<str>>(
    name: S,
    handler: o::Expression,
    prevent_default: bool,
    source_span: Option<ParseSourceSpan>,
) -> o::Statement {
    let mut args = vec![*o::literal(name.as_ref()), handler];
    if prevent_default {
        args.push(*o::literal(false));
    }
    call(Identifiers::two_way_listener(), args, source_span)
}

pub fn two_way_property<S: AsRef<str>>(
    name: S,
    expression: o::Expression,
    sanitizer: Option<o::Expression>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    let mut args = vec![*o::literal(name.as_ref())];
    args.push(expression);
    if let Some(san) = sanitizer {
        args.push(san);
    }
    call(Identifiers::two_way_property(), args, Some(source_span))
}

/// Creates a two-way binding set instruction expression.
/// Corresponds to `ng.twoWayBindingSet(target, value)` in TypeScript.
pub fn two_way_binding_set(
    target: Box<o::Expression>,
    value: Box<o::Expression>,
) -> Box<o::Expression> {
    o::import_ref(Identifiers::two_way_binding_set()).call_fn(vec![*target, *value], None, None)
}

pub fn pure_function(slot: i32, func: o::Expression, args: Vec<o::Expression>) -> o::Expression {
    let num_args = args.len();
    let id = match num_args {
        0 => Identifiers::pure_function0(),
        1 => Identifiers::pure_function1(),
        2 => Identifiers::pure_function2(),
        3 => Identifiers::pure_function3(),
        4 => Identifiers::pure_function4(),
        5 => Identifiers::pure_function5(),
        6 => Identifiers::pure_function6(),
        7 => Identifiers::pure_function7(),
        8 => Identifiers::pure_function8(),
        _ => Identifiers::pure_function_v(),
    };

    let mut call_args = vec![*o::literal(slot as f64), func];
    if num_args > 8 {
        // Box args into array for pureFunctionV
        call_args.push(o::Expression::LiteralArray(o::LiteralArrayExpr {
            entries: args,
            type_: None,
            source_span: None,
        }));
    } else {
        call_args.extend(args);
    }

    *o::import_ref(id).call_fn(call_args, None, None)
}

pub fn template(
    slot: i32,
    template_fn: o::Expression,
    decls: usize,
    vars: usize,
    tag: Option<String>,
    const_index: Option<i32>,
    local_ref_index: Option<i32>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    let mut args = vec![
        *o::literal(slot as f64),
        template_fn,
        *o::literal(decls as f64),
        *o::literal(vars as f64),
    ];

    // Tag argument (or null if we need to emit later args)
    if let Some(t) = tag {
        args.push(*o::literal(t));
    } else if const_index.is_some() || local_ref_index.is_some() {
        args.push(*o::literal(o::LiteralValue::Null));
    }

    // Const index argument (or null if we need to emit local_ref_index)
    if let Some(c) = const_index {
        args.push(*o::literal(c as f64));
    } else if local_ref_index.is_some() {
        args.push(*o::literal(o::LiteralValue::Null));
    }

    // Local ref index and templateRefExtractor for named ng-templates
    if let Some(lri) = local_ref_index {
        args.push(*o::literal(lri as f64));
        args.push(*o::import_ref(Identifiers::template_ref_extractor()));
    }

    call(Identifiers::template_create(), args, Some(source_span))
}

pub fn dom_template(
    slot: i32,
    template_fn: o::Expression,
    decls: usize,
    vars: usize,
    tag: String,
    const_index: Option<i32>,
    local_ref_index: Option<i32>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    let mut args = vec![
        *o::literal(slot as f64),
        template_fn,
        *o::literal(decls as f64),
        *o::literal(vars as f64),
        *o::literal(tag),
    ];

    // Const index argument
    if let Some(c) = const_index {
        args.push(*o::literal(c as f64));
    } else if local_ref_index.is_some() {
        args.push(*o::literal(o::LiteralValue::Null));
    }

    // Local ref index
    if let Some(lri) = local_ref_index {
        args.push(*o::literal(lri as f64));
    }

    call(Identifiers::dom_template(), args, Some(source_span))
}

pub fn conditional(
    slot: i32,
    condition: o::Expression,
    template_fn: Option<o::Expression>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    let mut args = vec![*o::literal(slot as f64), condition];
    if let Some(tmpl) = template_fn {
        args.push(tmpl);
    }
    call(Identifiers::conditional(), args, Some(source_span))
}

/// Creates a conditional create instruction.
/// Generates ɵɵconditionalCreate(slot, templateFn, decls, vars, tag, constsIndex) statement.
/// This is used for @if/@else if/@else chains instead of ɵɵtemplate.
pub fn conditional_create(
    slot: i32,
    template_fn: o::Expression,
    decls: usize,
    vars: usize,
    tag: Option<String>,
    const_index: Option<i32>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    let mut args = vec![
        *o::literal(slot as f64),
        template_fn,
        *o::literal(decls as f64),
        *o::literal(vars as f64),
    ];

    // Tag argument (required for conditional create)
    if let Some(t) = tag {
        args.push(*o::literal(t));
    } else {
        args.push(*o::literal(o::LiteralValue::Null));
    }

    // Const index argument
    if let Some(c) = const_index {
        args.push(*o::literal(c as f64));
    }

    call(Identifiers::conditional_create(), args, Some(source_span))
}

/// Creates a pipe binding expression.
/// Generates ɵɵpipeBind1/2/3/4/V based on number of arguments.
/// The signature is: ɵɵpipeBind(pipeSlot, varOffset, ...args)
pub fn pipe_bind(pipe_slot: i32, var_offset: i32, args: Vec<o::Expression>) -> o::Expression {
    let num_args = args.len();
    let id = match num_args {
        1 => Identifiers::pipe_bind1(),
        2 => Identifiers::pipe_bind2(),
        3 => Identifiers::pipe_bind3(),
        4 => Identifiers::pipe_bind4(),
        _ => Identifiers::pipe_bind_v(),
    };

    let mut call_args = vec![
        *o::literal(pipe_slot as f64),
        *o::literal(var_offset as f64),
    ];
    if num_args > 4 {
        // Box args into array for pipeBindV
        call_args.push(o::Expression::LiteralArray(o::LiteralArrayExpr {
            entries: args,
            type_: None,
            source_span: None,
        }));
    } else {
        call_args.extend(args);
    }

    *o::import_ref(id).call_fn(call_args, None, None)
}

/// Creates a reference expression for local template refs.
/// Generates ɵɵreference(slot) expression.
pub fn reference(slot: i32) -> o::Expression {
    *o::import_ref(Identifiers::reference()).call_fn(vec![*o::literal(slot as f64)], None, None)
}

/// Creates a classProp instruction.
/// Generates ɵɵclassProp(className, expression) statement.
pub fn class_prop<S: AsRef<str>>(
    name: S,
    expression: o::Expression,
    source_span: Option<ParseSourceSpan>,
) -> o::Statement {
    call(
        Identifiers::class_prop(),
        vec![*o::literal(name.as_ref()), expression],
        source_span,
    )
}

/// Creates a styleProp instruction.
/// Generates ɵɵstyleProp(styleName, expression, unit?) statement.
pub fn style_prop<S: AsRef<str>, U: AsRef<str>>(
    name: S,
    expression: o::Expression,
    unit: Option<U>,
    source_span: Option<ParseSourceSpan>,
) -> o::Statement {
    let mut args = vec![*o::literal(name.as_ref()), expression];
    if let Some(u) = unit {
        args.push(*o::literal(u.as_ref()));
    }
    call(Identifiers::style_prop(), args, source_span)
}
