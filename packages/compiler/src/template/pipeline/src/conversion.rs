//! Conversion Module
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/conversion.ts
//! Contains utilities for converting AST to IR

use crate::expression_parser::ast::ParseSpan;
use crate::output::output_ast::{BinaryOperator, Expression};
use crate::parse_util::ParseSourceSpan;
use crate::template::pipeline::ir::expression::{
    ContextExpr, EmptyExpr, LexicalReadExpr, PipeBindingExpr, SafeInvokeFunctionExpr,
    SafeKeyedReadExpr, SafePropertyReadExpr,
};
// use crate::template::pipeline::ir::handle::SlotHandle;
use crate::template::pipeline::ir::Namespace;
use crate::template::pipeline::src::compilation::CompilationJob;

/// Binary operator mappings from string to BinaryOperator
pub static BINARY_OPERATORS: &[(&str, BinaryOperator)] = &[
    ("&&", BinaryOperator::And),
    (">", BinaryOperator::Bigger),
    (">=", BinaryOperator::BiggerEquals),
    ("|", BinaryOperator::BitwiseOr),
    ("&", BinaryOperator::BitwiseAnd),
    ("/", BinaryOperator::Divide),
    ("=", BinaryOperator::Assign),
    ("==", BinaryOperator::Equals),
    ("===", BinaryOperator::Identical),
    ("<", BinaryOperator::Lower),
    ("<=", BinaryOperator::LowerEquals),
    ("-", BinaryOperator::Minus),
    ("%", BinaryOperator::Modulo),
    ("**", BinaryOperator::Exponentiation),
    ("*", BinaryOperator::Multiply),
    ("!=", BinaryOperator::NotEquals),
    ("!==", BinaryOperator::NotIdentical),
    ("??", BinaryOperator::NullishCoalesce),
    ("||", BinaryOperator::Or),
    ("+", BinaryOperator::Plus),
    ("in", BinaryOperator::In),
    ("+=", BinaryOperator::AdditionAssignment),
    ("-=", BinaryOperator::SubtractionAssignment),
    ("*=", BinaryOperator::MultiplicationAssignment),
    ("/=", BinaryOperator::DivisionAssignment),
    ("%=", BinaryOperator::RemainderAssignment),
    ("**=", BinaryOperator::ExponentiationAssignment),
    ("&&=", BinaryOperator::AndAssignment),
    ("||=", BinaryOperator::OrAssignment),
    ("??=", BinaryOperator::NullishCoalesceAssignment),
];

/// Get BinaryOperator from string
pub fn binary_operator_from_str(op: &str) -> Option<BinaryOperator> {
    BINARY_OPERATORS
        .iter()
        .find(|(k, _)| *k == op)
        .map(|(_, v)| *v)
}

/// Convert namespace prefix key to Namespace enum
pub fn namespace_for_key(namespace_prefix_key: Option<&str>) -> Namespace {
    match namespace_prefix_key {
        Some("svg") => Namespace::SVG,
        Some("math") => Namespace::Math,
        _ => Namespace::HTML,
    }
}

/// Convert Namespace enum to key string
pub fn key_for_namespace(namespace: Namespace) -> Option<&'static str> {
    match namespace {
        Namespace::SVG => Some("svg"),
        Namespace::Math => Some("math"),
        Namespace::HTML => None,
    }
}

/// Prefix tag name with namespace
pub fn prefix_with_namespace(stripped_tag: &str, namespace: Namespace) -> String {
    if namespace == Namespace::HTML {
        return stripped_tag.to_string();
    }
    if let Some(key) = key_for_namespace(namespace) {
        format!(":{}:{}", key, stripped_tag)
    } else {
        stripped_tag.to_string()
    }
}

/// Literal type for template values
#[derive(Debug, Clone)]
pub enum LiteralType {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
    Array(Vec<LiteralType>),
}

impl LiteralType {
    /// Convert to Expression
    pub fn to_expression(&self) -> Expression {
        use crate::output::output_ast::{LiteralExpr, LiteralValue};
        match self {
            LiteralType::String(s) => Expression::Literal(LiteralExpr {
                value: LiteralValue::String(s.clone()),
                type_: None,
                source_span: None,
            }),
            LiteralType::Number(n) => Expression::Literal(LiteralExpr {
                value: LiteralValue::Number(*n),
                type_: None,
                source_span: None,
            }),
            LiteralType::Boolean(b) => Expression::Literal(LiteralExpr {
                value: LiteralValue::Bool(*b),
                type_: None,
                source_span: None,
            }),
            LiteralType::Null => Expression::Literal(LiteralExpr {
                value: LiteralValue::Null,
                type_: None,
                source_span: None,
            }),
            LiteralType::Array(arr) => {
                use crate::output::output_ast::LiteralArrayExpr;
                let entries: Vec<Expression> = arr.iter().map(|v| v.to_expression()).collect();
                Expression::LiteralArray(LiteralArrayExpr {
                    entries,
                    type_: None,
                    source_span: None,
                })
            }
        }
    }
}

/// Convert literal or array literal to Expression
pub fn literal_or_array_literal(value: LiteralType) -> Expression {
    value.to_expression()
}

/// Convert a ParseSpan to ParseSourceSpan using a base source span.
/// This corresponds to the TypeScript `convertSourceSpan` function.
pub fn convert_source_span(
    span: &ParseSpan,
    base_source_span: Option<&ParseSourceSpan>,
) -> Option<ParseSourceSpan> {
    let base = base_source_span?;

    let start = base.start.move_by(span.start as i32);
    let end = base.start.move_by(span.end as i32);

    // Note: TypeScript also handles fullStart, but ParseSourceSpan in Rust
    // doesn't have a fullStart field, so we just use start/end
    Some(ParseSourceSpan::new(start, end))
}

/// Convert AST expression to Output Expression.
///
/// This function converts template AST expressions into output AST expressions.
/// IR expressions are returned directly when appropriate (e.g., LexicalReadExpr,
/// ContextExpr, SafePropertyReadExpr, etc.).
pub fn convert_ast(
    ast: &crate::expression_parser::ast::AST,
    job: &mut dyn CompilationJob,
    view_xref: crate::template::pipeline::ir::XrefId,
    base_source_span: Option<&crate::parse_util::ParseSourceSpan>,
) -> Expression {
    convert_ast_internal(ast, job, view_xref, base_source_span)
}

fn convert_ast_internal(
    ast: &crate::expression_parser::ast::AST,
    job: &mut dyn CompilationJob,
    view_xref: crate::template::pipeline::ir::XrefId,
    base_source_span: Option<&crate::parse_util::ParseSourceSpan>,
) -> Expression {
    use crate::expression_parser::ast::AST;
    use crate::output::output_ast::{
        BinaryOperatorExpr, ConditionalExpr, InvokeFunctionExpr, LiteralArrayExpr, LiteralExpr,
        LiteralMapEntry, LiteralMapExpr, LiteralValue, NotExpr, ReadKeyExpr, ReadPropExpr,
        ReadVarExpr, TypeofExpr, UnaryOperatorExpr, VoidExpr, WriteKeyExpr, WritePropExpr,
    };

    match ast {
        AST::LiteralPrimitive(lit) => {
            use crate::expression_parser::ast::LiteralPrimitive;
            let (value, span) = match lit {
                LiteralPrimitive::String { value, span, .. } => {
                    (LiteralValue::String(value.clone()), span)
                }
                LiteralPrimitive::Number { value, span, .. } => {
                    (LiteralValue::Number(*value), span)
                }
                LiteralPrimitive::Boolean { value, span, .. } => (LiteralValue::Bool(*value), span),
                LiteralPrimitive::Null { span, .. } => (LiteralValue::Null, span),
                LiteralPrimitive::Undefined { span, .. } => {
                    (LiteralValue::Null, span) // Treat undefined as null
                }
            };
            Expression::Literal(LiteralExpr {
                value,
                type_: None,
                source_span: convert_source_span(&span, base_source_span),
            })
        }
        AST::Binary(bin) => {
            let op = binary_operator_from_str(&bin.operation)
                .expect(&format!("Unknown binary operator: {}", bin.operation));
            Expression::BinaryOp(BinaryOperatorExpr {
                operator: op,
                lhs: Box::new(convert_ast_internal(
                    &bin.left,
                    job,
                    view_xref,
                    base_source_span,
                )),
                rhs: Box::new(convert_ast_internal(
                    &bin.right,
                    job,
                    view_xref,
                    base_source_span,
                )),
                type_: None,
                source_span: convert_source_span(&bin.span, base_source_span),
            })
        }
        AST::Unary(un) => {
            let op = match un.operator.as_str() {
                "+" => crate::output::output_ast::UnaryOperator::Plus,
                "-" => crate::output::output_ast::UnaryOperator::Minus,
                _ => panic!("Unknown unary operator: {}", un.operator),
            };
            Expression::Unary(UnaryOperatorExpr {
                operator: op,
                expr: Box::new(convert_ast_internal(
                    &un.expr,
                    job,
                    view_xref,
                    base_source_span,
                )),
                type_: None,
                source_span: convert_source_span(&un.span, base_source_span),
            })
        }
        AST::Conditional(cond) => Expression::Conditional(ConditionalExpr {
            condition: Box::new(convert_ast_internal(
                &cond.condition,
                job,
                view_xref,
                base_source_span,
            )),
            true_case: Box::new(convert_ast_internal(
                &cond.true_exp,
                job,
                view_xref,
                base_source_span,
            )),
            false_case: Some(Box::new(convert_ast_internal(
                &cond.false_exp,
                job,
                view_xref,
                base_source_span,
            ))),
            type_: None,
            source_span: convert_source_span(&cond.span, base_source_span),
        }),
        AST::Call(call) => {
            // TypeScript checks: if (ast.receiver instanceof e.ImplicitReceiver) throw error
            if matches!(*call.receiver, AST::ImplicitReceiver(_)) {
                panic!("Unexpected ImplicitReceiver in Call expression");
            }
            Expression::InvokeFn(InvokeFunctionExpr {
                fn_: Box::new(convert_ast_internal(
                    &call.receiver,
                    job,
                    view_xref,
                    base_source_span,
                )),
                args: call
                    .args
                    .iter()
                    .map(|arg| convert_ast_internal(arg, job, view_xref, base_source_span))
                    .collect(),
                type_: None,
                source_span: convert_source_span(&call.span, base_source_span),
                pure: false,
            })
        }
        AST::PropertyRead(prop) => {
            // Whether this is an implicit receiver, *excluding* explicit reads of `this`.
            let is_implicit_receiver = matches!(*prop.receiver, AST::ImplicitReceiver(_))
                && !matches!(*prop.receiver, AST::ThisReceiver(_));

            if is_implicit_receiver {
                // Return IR LexicalReadExpr as in TypeScript
                let source_span = convert_source_span(&prop.span, base_source_span);
                Expression::LexicalRead(LexicalReadExpr {
                    name: prop.name.clone().into(),
                    source_span: source_span.clone(),
                })
            } else {
                Expression::ReadProp(ReadPropExpr {
                    receiver: Box::new(convert_ast_internal(
                        &prop.receiver,
                        job,
                        view_xref,
                        base_source_span,
                    )),
                    name: prop.name.clone().into(),
                    type_: None,
                    source_span: convert_source_span(&prop.span, base_source_span),
                })
            }
        }
        AST::KeyedRead(keyed) => Expression::ReadKey(ReadKeyExpr {
            receiver: Box::new(convert_ast_internal(
                &keyed.receiver,
                job,
                view_xref,
                base_source_span,
            )),
            index: Box::new(convert_ast_internal(
                &keyed.key,
                job,
                view_xref,
                base_source_span,
            )),
            type_: None,
            source_span: convert_source_span(&keyed.span, base_source_span),
        }),
        AST::LiteralArray(arr) => Expression::LiteralArray(LiteralArrayExpr {
            entries: arr
                .expressions
                .iter()
                .map(|expr| convert_ast_internal(expr, job, view_xref, base_source_span))
                .collect(),
            type_: None,
            source_span: None, // Literal arrays typically use surrounding expression span
        }),
        AST::LiteralMap(map) => {
            let entries: Vec<LiteralMapEntry> = map
                .keys
                .iter()
                .zip(map.values.iter())
                .map(|(key, value)| LiteralMapEntry {
                    key: key.key.clone(),
                    value: Box::new(convert_ast_internal(
                        value,
                        job,
                        view_xref,
                        base_source_span,
                    )),
                    quoted: key.quoted,
                })
                .collect();
            Expression::LiteralMap(LiteralMapExpr {
                entries,
                type_: None,
                source_span: convert_source_span(&map.span, base_source_span),
            })
        }
        AST::PrefixNot(not) => Expression::NotExpr(NotExpr {
            condition: Box::new(convert_ast_internal(
                &not.expression,
                job,
                view_xref,
                base_source_span,
            )),
            source_span: convert_source_span(&not.span, base_source_span),
        }),
        AST::TypeofExpression(ty) => Expression::TypeOf(TypeofExpr {
            expr: Box::new(convert_ast_internal(
                &ty.expression,
                job,
                view_xref,
                base_source_span,
            )),
            type_: None,
            source_span: None,
        }),
        AST::VoidExpression(void) => Expression::Void(VoidExpr {
            expr: Box::new(convert_ast_internal(
                &void.expression,
                job,
                view_xref,
                base_source_span,
            )),
            type_: None,
            source_span: convert_source_span(&void.span, base_source_span),
        }),
        AST::NonNullAssert(nn) => {
            // Non-null assertion shouldn't impact generated instructions, so we can just drop it
            convert_ast_internal(&nn.expression, job, view_xref, base_source_span)
        }
        AST::ImplicitReceiver(_) => {
            // In TypeScript, ImplicitReceiver is handled in PropertyRead
            // This case should not be reached in normal flow
            // Return placeholder - actual handling is in PropertyRead case
            Expression::ReadVar(ReadVarExpr {
                name: "$implicit".to_string(),
                type_: None,
                source_span: None,
            })
        }
        AST::ThisReceiver(_) => {
            // Return IR ContextExpr with root view's xref
            let root_xref = job.root().xref();
            Expression::Context(ContextExpr::new(root_xref))
        }
        AST::EmptyExpr(empty) => {
            // Return IR EmptyExpr as in TypeScript
            let source_span = convert_source_span(&empty.span, base_source_span);
            Expression::Empty(EmptyExpr {
                source_span: source_span.clone(),
            })
        }
        AST::SafePropertyRead(safe_prop) => {
            // Return IR SafePropertyReadExpr as in TypeScript
            // Note: Source span is not set here (same as TypeScript implementation)
            let receiver_expr =
                convert_ast_internal(&safe_prop.receiver, job, view_xref, base_source_span);
            Expression::SafePropertyRead(SafePropertyReadExpr::new(
                Box::new(receiver_expr),
                safe_prop.name.clone().into(),
            ))
        }
        AST::SafeKeyedRead(safe_keyed) => {
            // Return IR SafeKeyedReadExpr as in TypeScript
            let receiver_expr =
                convert_ast_internal(&safe_keyed.receiver, job, view_xref, base_source_span);
            let key_expr = convert_ast_internal(&safe_keyed.key, job, view_xref, base_source_span);
            Expression::SafeKeyedRead(SafeKeyedReadExpr::new(
                Box::new(receiver_expr),
                Box::new(key_expr),
                convert_source_span(&safe_keyed.span, base_source_span),
            ))
        }
        AST::SafeCall(safe_call) => {
            // Return IR SafeInvokeFunctionExpr as in TypeScript
            // Note: Source span is not set here (same as TypeScript implementation)
            let receiver_expr =
                convert_ast_internal(&safe_call.receiver, job, view_xref, base_source_span);
            let args: Vec<Expression> = safe_call
                .args
                .iter()
                .map(|arg| convert_ast_internal(arg, job, view_xref, base_source_span))
                .collect();
            Expression::SafeInvokeFunction(SafeInvokeFunctionExpr::new(
                Box::new(receiver_expr),
                args,
            ))
        }
        AST::Chain(_chain) => {
            // In TypeScript, this throws: throw new Error(`AssertionError: Chain in unknown context`);
            // Chain expressions should be handled in make_listener_handler_ops, not here
            panic!("Chain expression should not be converted directly - handle in make_listener_handler_ops");
        }
        AST::BindingPipe(pipe) => {
            // Return IR PipeBindingExpr as in TypeScript
            // Use add_pipe to get or create the slot for this pipe in the current view
            let (slot_handle, target_xref) = job.add_pipe(&pipe.name, view_xref);

            // Also mark as used
            job.mark_pipe_used(&pipe.name);

            let mut args = vec![convert_ast_internal(
                &pipe.exp,
                job,
                view_xref,
                base_source_span,
            )];
            args.extend(
                pipe.args
                    .iter()
                    .map(|arg| convert_ast_internal(arg, job, view_xref, base_source_span)),
            );
            Expression::PipeBinding(PipeBindingExpr::new(
                target_xref,
                slot_handle,
                pipe.name.clone(),
                args,
            ))
        }
        AST::TemplateLiteral(template) => {
            use crate::output::output_ast::{TemplateLiteralElement, TemplateLiteralExpr};
            let elements: Vec<TemplateLiteralElement> = template
                .elements
                .iter()
                .map(|el| TemplateLiteralElement {
                    text: el.text.clone(),
                    raw_text: el.text.clone(), // AST element doesn't have raw_text, use text as fallback
                    source_span: convert_source_span(&el.span, base_source_span),
                })
                .collect();
            let expressions: Vec<Expression> = template
                .expressions
                .iter()
                .map(|expr| convert_ast_internal(expr, job, view_xref, base_source_span))
                .collect();
            Expression::TemplateLiteral(TemplateLiteralExpr {
                elements,
                expressions,
            })
        }
        AST::TaggedTemplateLiteral(tagged) => {
            use crate::output::output_ast::{
                TaggedTemplateLiteralExpr, TemplateLiteral, TemplateLiteralElement,
            };
            let tag_expr = convert_ast_internal(&tagged.tag, job, view_xref, base_source_span);
            // Convert template literal elements and expressions
            let elements: Vec<TemplateLiteralElement> = tagged
                .template
                .elements
                .iter()
                .map(|el| TemplateLiteralElement {
                    text: el.text.clone(),
                    raw_text: el.text.clone(), // AST element doesn't have raw_text, use text as fallback
                    source_span: convert_source_span(&el.span, base_source_span),
                })
                .collect();
            let expressions: Vec<Expression> = tagged
                .template
                .expressions
                .iter()
                .map(|expr| convert_ast_internal(expr, job, view_xref, base_source_span))
                .collect();
            let template = TemplateLiteral {
                elements,
                expressions,
            };
            Expression::TaggedTemplate(TaggedTemplateLiteralExpr {
                tag: Box::new(tag_expr),
                template,
                type_: None,
                source_span: convert_source_span(&tagged.span, base_source_span),
            })
        }
        AST::ParenthesizedExpression(paren) => {
            // Parenthesized expressions don't affect semantics, so we can just unwrap
            // In TypeScript, this returns a ParenthesizedExpr, but in Rust output_ast doesn't have it
            // So we just return the inner expression
            convert_ast_internal(&paren.expression, job, view_xref, base_source_span)
        }
        AST::PropertyWrite(prop) => {
            // Whether this is an implicit receiver, *excluding* explicit reads of `this`.
            let is_implicit_receiver = matches!(*prop.receiver, AST::ImplicitReceiver(_))
                && !matches!(*prop.receiver, AST::ThisReceiver(_));

            if is_implicit_receiver {
                // TODO: Implement LexicalWriteExpr in IR and use it here.
                // For now, assume it's a property write on the component context.
                let root_xref = job.root().xref();
                Expression::WriteProp(WritePropExpr {
                    receiver: Box::new(Expression::Context(ContextExpr::new(root_xref))),
                    name: prop.name.clone().into(),
                    value: Box::new(convert_ast_internal(
                        &prop.value,
                        job,
                        view_xref,
                        base_source_span,
                    )),
                    type_: None,
                    source_span: convert_source_span(&prop.span, base_source_span),
                })
            } else {
                Expression::WriteProp(WritePropExpr {
                    receiver: Box::new(convert_ast_internal(
                        &prop.receiver,
                        job,
                        view_xref,
                        base_source_span,
                    )),
                    name: prop.name.clone().into(),
                    value: Box::new(convert_ast(&prop.value, job, view_xref, base_source_span)),
                    type_: None,
                    source_span: convert_source_span(&prop.span, base_source_span),
                })
            }
        }
        AST::KeyedWrite(keyed) => Expression::WriteKey(WriteKeyExpr {
            receiver: Box::new(convert_ast_internal(
                &keyed.receiver,
                job,
                view_xref,
                base_source_span,
            )),
            index: Box::new(convert_ast_internal(
                &keyed.key,
                job,
                view_xref,
                base_source_span,
            )),
            value: Box::new(convert_ast_internal(
                &keyed.value,
                job,
                view_xref,
                base_source_span,
            )),
            type_: None,
            source_span: convert_source_span(&keyed.span, base_source_span),
        }),
        AST::RegularExpressionLiteral(regex) => {
            // RegularExpressionLiteralExpr doesn't exist in output_ast
            // In TypeScript, this returns RegularExpressionLiteralExpr(ast.body, ast.flags, baseSourceSpan)
            // For now, we use ExternalExpr as a placeholder
            let flags_str = regex.flags.as_ref().map(|f| f.as_str()).unwrap_or("");
            Expression::External(crate::output::output_ast::ExternalExpr {
                value: crate::output::output_ast::ExternalReference {
                    name: Some(format!("/{}/{}", regex.body, flags_str)),
                    module_name: None,
                    runtime: None,
                },
                type_: None,
                source_span: base_source_span.cloned(),
            })
        }
        _ => {
            panic!(
                "Unhandled AST type in convert_ast: {:?}",
                std::mem::discriminant(ast)
            );
        }
    }
}
