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
    o::Statement::Expression(o::ExpressionStatement {
        expr,
        source_span,
    })
}

// Update helper as well, same return type as it produces a Statement
pub fn call_update(
    fn_: ExternalReference,
    args: Vec<o::Expression>,
    source_span: Option<ParseSourceSpan>,
) -> o::Statement {
    call(fn_, args, source_span)
}

fn element_or_container_base(
    instruction: o::ExternalReference,
    slot: i32,
    tag: Option<String>,
    const_index: Option<i32>,
    local_ref_index: Option<i32>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    let mut args = vec![*o::literal(slot as f64)]; // unbox literal

    if let Some(t) = tag {
        args.push(*o::literal(t));
    }

    if let Some(local_ref) = local_ref_index {
        args.push(*o::literal(
             match const_index { Some(i) => o::LiteralValue::from(i as f64), None => o::LiteralValue::Null }, 
        ));
        args.push(*o::literal(local_ref as f64));
    } else if let Some(const_i) = const_index {
        args.push(*o::literal(const_i as f64));
    }
    
    call(instruction, args, Some(source_span))
}

pub fn element_start(
    slot: i32,
    tag: String,
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

pub fn element(
    slot: i32,
    tag: String,
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

pub fn text(
    slot: i32,
    initial_value: String,
    source_span: Option<ParseSourceSpan>,
) -> o::Statement {
    let mut args = vec![*o::literal(slot as f64)];
    if !initial_value.is_empty() {
         args.push(*o::literal(initial_value));
    }
    call(Identifiers::text(), args, source_span)
}

pub fn pipe(slot: i32, name: String) -> o::Statement {
    call(
        Identifiers::pipe(),
        vec![
            *o::literal(slot as f64),
            *o::literal(name),
        ],
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

pub fn property(
    name: String,
    expression: o::Expression, 
    sanitizer: Option<o::Expression>,
    source_span: ParseSourceSpan,
) -> o::Statement {
    let mut args = vec![*o::literal(name)];
    args.push(expression);
    if let Some(san) = sanitizer {
        args.push(san);
    }
    call(Identifiers::property(), args, Some(source_span))
}

pub fn disable_bindings() -> o::Statement {
    call(Identifiers::disable_bindings(), vec![], None)
}

pub fn enable_bindings() -> o::Statement {
    call(Identifiers::enable_bindings(), vec![], None)
}

/// Creates a two-way binding set instruction expression.
/// Corresponds to `ng.twoWayBindingSet(target, value)` in TypeScript.
pub fn two_way_binding_set(target: Box<o::Expression>, value: Box<o::Expression>) -> Box<o::Expression> {
    o::import_ref(Identifiers::two_way_binding_set()).call_fn(
        vec![*target, *value],
        None,
        None,
    )
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
    source_span: ParseSourceSpan,
) -> o::Statement {
    let mut args = vec![
        *o::literal(slot as f64),
        template_fn,
        *o::literal(decls as f64),
        *o::literal(vars as f64),
    ];
    if let Some(t) = tag {
        args.push(*o::literal(t));
    } else if const_index.is_some() {
        args.push(*o::literal(o::LiteralValue::Null));
    }

    if let Some(c) = const_index {
        args.push(*o::literal(c as f64));
    }

    call(Identifiers::template_create(), args, Some(source_span))
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

