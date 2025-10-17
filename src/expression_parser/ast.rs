/**
 * Angular Expression AST
 *
 * Defines all AST node types for Angular template expressions
 * Mirrors packages/compiler/src/expression_parser/ast.ts (862 lines)
 */

use serde::{Deserialize, Serialize};
use crate::core::SecurityContext;
use crate::parse_util::{ParseError, ParseSourceSpan};

/// Source span for error reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseSpan {
    pub start: usize,
    pub end: usize,
}

impl ParseSpan {
    pub fn new(start: usize, end: usize) -> Self {
        ParseSpan { start, end }
    }

    pub fn to_absolute(&self, absolute_offset: usize) -> AbsoluteSourceSpan {
        AbsoluteSourceSpan::new(
            absolute_offset + self.start,
            absolute_offset + self.end,
        )
    }
}

/// Absolute source span for mapping back to source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsoluteSourceSpan {
    pub start: usize,
    pub end: usize,
}

impl AbsoluteSourceSpan {
    pub fn new(start: usize, end: usize) -> Self {
        AbsoluteSourceSpan { start, end }
    }
}

/// Base trait for all AST nodes
pub trait AstNode {
    fn span(&self) -> &ParseSpan;
    fn source_span(&self) -> &AbsoluteSourceSpan;
    fn visit<V: AstVisitor>(&self, visitor: &mut V) -> V::Result;
}

/// Visitor pattern for AST traversal
pub trait AstVisitor {
    type Result;

    fn visit_binary(&mut self, ast: &Binary) -> Self::Result;
    fn visit_chain(&mut self, ast: &Chain) -> Self::Result;
    fn visit_conditional(&mut self, ast: &Conditional) -> Self::Result;
    fn visit_implicit_receiver(&mut self, ast: &ImplicitReceiver) -> Self::Result;
    fn visit_this_receiver(&mut self, ast: &ThisReceiver) -> Self::Result;
    fn visit_property_read(&mut self, ast: &PropertyRead) -> Self::Result;
    fn visit_safe_property_read(&mut self, ast: &SafePropertyRead) -> Self::Result;
    fn visit_keyed_read(&mut self, ast: &KeyedRead) -> Self::Result;
    fn visit_safe_keyed_read(&mut self, ast: &SafeKeyedRead) -> Self::Result;
    fn visit_literal_primitive(&mut self, ast: &LiteralPrimitive) -> Self::Result;
    fn visit_literal_array(&mut self, ast: &LiteralArray) -> Self::Result;
    fn visit_literal_map(&mut self, ast: &LiteralMap) -> Self::Result;
    fn visit_call(&mut self, ast: &Call) -> Self::Result;
    fn visit_safe_call(&mut self, ast: &SafeCall) -> Self::Result;
    fn visit_pipe(&mut self, ast: &BindingPipe) -> Self::Result;
    fn visit_prefix_not(&mut self, ast: &PrefixNot) -> Self::Result;
    fn visit_unary(&mut self, ast: &Unary) -> Self::Result;
    fn visit_non_null_assert(&mut self, ast: &NonNullAssert) -> Self::Result;
}

/// Main AST enum containing all node types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AST {
    EmptyExpr(EmptyExpr),
    ImplicitReceiver(ImplicitReceiver),
    ThisReceiver(ThisReceiver),
    Chain(Chain),
    Conditional(Conditional),
    PropertyRead(PropertyRead),
    SafePropertyRead(SafePropertyRead),
    KeyedRead(KeyedRead),
    SafeKeyedRead(SafeKeyedRead),
    BindingPipe(BindingPipe),
    LiteralPrimitive(LiteralPrimitive),
    LiteralArray(LiteralArray),
    LiteralMap(LiteralMap),
    Interpolation(Interpolation),
    Binary(Binary),
    PrefixNot(PrefixNot),
    Unary(Unary),
    TypeofExpression(TypeofExpression),
    VoidExpression(VoidExpression),
    NonNullAssert(NonNullAssert),
    Call(Call),
    SafeCall(SafeCall),
    TemplateLiteral(TemplateLiteral),
    TaggedTemplateLiteral(TaggedTemplateLiteral),
    ParenthesizedExpression(ParenthesizedExpression),
    RegularExpressionLiteral(RegularExpressionLiteral),
}

/// Empty expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyExpr {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
}

/// Implicit receiver (the component instance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplicitReceiver {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
}

/// This receiver (explicit `this`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThisReceiver {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
}

/// Chain of expressions (e.g., `a; b; c`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub expressions: Vec<Box<AST>>,
}

/// Ternary conditional (e.g., `condition ? true : false`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conditional {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub condition: Box<AST>,
    pub true_exp: Box<AST>,
    pub false_exp: Box<AST>,
}

/// Property read (e.g., `obj.property`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyRead {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub name_span: AbsoluteSourceSpan,
    pub receiver: Box<AST>,
    pub name: String,
}

/// Safe property read (e.g., `obj?.property`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafePropertyRead {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub name_span: AbsoluteSourceSpan,
    pub receiver: Box<AST>,
    pub name: String,
}

/// Keyed read (e.g., `obj[key]`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyedRead {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub receiver: Box<AST>,
    pub key: Box<AST>,
}

/// Safe keyed read (e.g., `obj?.[key]`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeKeyedRead {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub receiver: Box<AST>,
    pub key: Box<AST>,
}

/// Pipe types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingPipeType {
    /// Referenced by name: `{{value | pipeName}}`
    ReferencedByName,
    /// Referenced directly: `{{value | PipeClass}}`
    ReferencedDirectly,
}

/// Pipe binding (e.g., `value | pipeName:arg1:arg2`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingPipe {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub name_span: AbsoluteSourceSpan,
    pub exp: Box<AST>,
    pub name: String,
    pub args: Vec<Box<AST>>,
    pub pipe_type: BindingPipeType,
}

/// Literal primitive (string, number, boolean, null)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "literalType")]
pub enum LiteralPrimitive {
    String {
        span: ParseSpan,
        source_span: AbsoluteSourceSpan,
        value: String,
    },
    Number {
        span: ParseSpan,
        source_span: AbsoluteSourceSpan,
        value: f64,
    },
    Boolean {
        span: ParseSpan,
        source_span: AbsoluteSourceSpan,
        value: bool,
    },
    Null {
        span: ParseSpan,
        source_span: AbsoluteSourceSpan,
    },
    Undefined {
        span: ParseSpan,
        source_span: AbsoluteSourceSpan,
    },
}

/// Array literal (e.g., `[1, 2, 3]`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteralArray {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub expressions: Vec<Box<AST>>,
}

/// Map literal key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteralMapKey {
    pub key: String,
    pub quoted: bool,
}

/// Object literal (e.g., `{a: 1, b: 2}`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteralMap {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub keys: Vec<LiteralMapKey>,
    pub values: Vec<Box<AST>>,
}

/// Binary operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binary {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub operation: String,
    pub left: Box<AST>,
    pub right: Box<AST>,
}

/// Prefix not operator (e.g., `!expr`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixNot {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub expression: Box<AST>,
}

/// Unary operator (e.g., `+expr`, `-expr`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unary {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub operator: String,
    pub expr: Box<AST>,
}

/// Function call (e.g., `fn(a, b)`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub receiver: Box<AST>,
    pub args: Vec<Box<AST>>,
    pub argument_span: AbsoluteSourceSpan,
}

/// Safe function call (e.g., `fn?.(a, b)`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeCall {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub receiver: Box<AST>,
    pub args: Vec<Box<AST>>,
    pub argument_span: AbsoluteSourceSpan,
}

/// Non-null assertion (e.g., `expr!`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonNullAssert {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub expression: Box<AST>,
}

// Helper constructors
impl EmptyExpr {
    pub fn new(span: ParseSpan, source_span: AbsoluteSourceSpan) -> Self {
        EmptyExpr { span, source_span }
    }
}

impl ImplicitReceiver {
    pub fn new(span: ParseSpan, source_span: AbsoluteSourceSpan) -> Self {
        ImplicitReceiver { span, source_span }
    }
}

impl ThisReceiver {
    pub fn new(span: ParseSpan, source_span: AbsoluteSourceSpan) -> Self {
        ThisReceiver { span, source_span }
    }
}

impl PropertyRead {
    pub fn new(
        span: ParseSpan,
        source_span: AbsoluteSourceSpan,
        name_span: AbsoluteSourceSpan,
        receiver: Box<AST>,
        name: String,
    ) -> Self {
        PropertyRead {
            span,
            source_span,
            name_span,
            receiver,
            name,
        }
    }
}

impl Binary {
    pub fn new(
        span: ParseSpan,
        source_span: AbsoluteSourceSpan,
        operation: String,
        left: Box<AST>,
        right: Box<AST>,
    ) -> Self {
        Binary {
            span,
            source_span,
            operation,
            left,
            right,
        }
    }
}

impl LiteralPrimitive {
    pub fn string(span: ParseSpan, source_span: AbsoluteSourceSpan, value: String) -> Self {
        LiteralPrimitive::String {
            span,
            source_span,
            value,
        }
    }

    pub fn number(span: ParseSpan, source_span: AbsoluteSourceSpan, value: f64) -> Self {
        LiteralPrimitive::Number {
            span,
            source_span,
            value,
        }
    }

    pub fn boolean(span: ParseSpan, source_span: AbsoluteSourceSpan, value: bool) -> Self {
        LiteralPrimitive::Boolean {
            span,
            source_span,
            value,
        }
    }

    pub fn null(span: ParseSpan, source_span: AbsoluteSourceSpan) -> Self {
        LiteralPrimitive::Null { span, source_span }
    }

    pub fn undefined(span: ParseSpan, source_span: AbsoluteSourceSpan) -> Self {
        LiteralPrimitive::Undefined { span, source_span }
    }
}

/// Interpolation ({{expr}})
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interpolation {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub strings: Vec<String>,
    pub expressions: Vec<Box<AST>>,
}

/// Typeof expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeofExpression {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub expression: Box<AST>,
}

/// Void expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidExpression {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub expression: Box<AST>,
}

/// Template literal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateLiteral {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub elements: Vec<TemplateLiteralElement>,
    pub expressions: Vec<Box<AST>>,
}

/// Template literal element (string part)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateLiteralElement {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub text: String,
}

/// Tagged template literal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaggedTemplateLiteral {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub tag: Box<AST>,
    pub template: TemplateLiteral,
}

/// Parenthesized expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParenthesizedExpression {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub expression: Box<AST>,
}

/// Regular expression literal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegularExpressionLiteral {
    pub span: ParseSpan,
    pub source_span: AbsoluteSourceSpan,
    pub body: String,
    pub flags: Option<String>,
}

/// AST with source location info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASTWithSource {
    pub ast: Box<AST>,
    pub source: Option<String>,
    pub location: String,
    pub absolute_offset: usize,
    pub errors: Vec<ParseError>,
}

impl ASTWithSource {
    pub fn new(
        ast: Box<AST>,
        source: Option<String>,
        location: String,
        absolute_offset: usize,
        errors: Vec<ParseError>,
    ) -> Self {
        ASTWithSource {
            ast,
            source,
            location,
            absolute_offset,
            errors,
        }
    }
}

/// Variable binding (let x = expr)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableBinding {
    pub span: AbsoluteSourceSpan,
    pub key: TemplateBindingIdentifier,
    pub value: Option<Box<AST>>,
}

/// Expression binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpressionBinding {
    pub span: AbsoluteSourceSpan,
    pub key: TemplateBindingIdentifier,
    pub value: Option<Box<AST>>,
}

/// Template binding identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateBindingIdentifier {
    pub source: String,
    pub span: AbsoluteSourceSpan,
}

/// Template binding (combination of variable and expression bindings)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TemplateBinding {
    Variable(VariableBinding),
    Expression(ExpressionBinding),
}

/// Parsed property types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParsedPropertyType {
    Default,
    Literal,
    Animation,
    TwoWay,
}

/// Parsed event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParsedEventType {
    Regular,
    Animation,
}

/// Binding types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingType {
    Property,
    Attribute,
    Class,
    Style,
    LegacyAnimation,
    TwoWay,
    Animation,
}

/// Parsed property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedProperty {
    pub name: String,
    pub expression: Box<AST>,
    pub property_type: ParsedPropertyType,
    pub source_span: ParseSourceSpan,
    pub key_span: ParseSourceSpan,
    pub value_span: ParseSourceSpan,
}

/// Parsed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedEvent {
    pub name: String,
    pub target_or_phase: Option<String>,
    pub event_type: ParsedEventType,
    pub handler: Box<AST>,
    pub source_span: ParseSourceSpan,
    pub handler_span: ParseSourceSpan,
    pub key_span: ParseSourceSpan,
}

/// Parsed variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedVariable {
    pub name: String,
    pub value: String,
    pub source_span: ParseSourceSpan,
    pub key_span: ParseSourceSpan,
    pub value_span: Option<ParseSourceSpan>,
}

/// Bound element property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundElementProperty {
    pub name: String,
    pub binding_type: BindingType,
    pub security_context: SecurityContext,
    pub value: Box<AST>,
    pub unit: Option<String>,
    pub source_span: ParseSourceSpan,
    pub key_span: ParseSourceSpan,
    pub value_span: ParseSourceSpan,
}

/// Recursive AST visitor implementation
pub struct RecursiveAstVisitor;

impl RecursiveAstVisitor {
    pub fn new() -> Self {
        RecursiveAstVisitor
    }

    pub fn visit(&self, ast: &AST) {
        // Default implementation visits all child nodes
        match ast {
            AST::Binary(b) => {
                self.visit(&b.left);
                self.visit(&b.right);
            }
            AST::Chain(c) => {
                for expr in &c.expressions {
                    self.visit(expr);
                }
            }
            AST::Conditional(c) => {
                self.visit(&c.condition);
                self.visit(&c.true_exp);
                self.visit(&c.false_exp);
            }
            AST::PropertyRead(p) => {
                self.visit(&p.receiver);
            }
            AST::SafePropertyRead(p) => {
                self.visit(&p.receiver);
            }
            AST::KeyedRead(k) => {
                self.visit(&k.receiver);
                self.visit(&k.key);
            }
            AST::SafeKeyedRead(k) => {
                self.visit(&k.receiver);
                self.visit(&k.key);
            }
            AST::BindingPipe(p) => {
                self.visit(&p.exp);
                for arg in &p.args {
                    self.visit(arg);
                }
            }
            AST::LiteralArray(a) => {
                for expr in &a.expressions {
                    self.visit(expr);
                }
            }
            AST::LiteralMap(m) => {
                for value in &m.values {
                    self.visit(value);
                }
            }
            AST::Interpolation(i) => {
                for expr in &i.expressions {
                    self.visit(expr);
                }
            }
            AST::Call(c) => {
                self.visit(&c.receiver);
                for arg in &c.args {
                    self.visit(arg);
                }
            }
            AST::SafeCall(c) => {
                self.visit(&c.receiver);
                for arg in &c.args {
                    self.visit(arg);
                }
            }
            AST::PrefixNot(p) => {
                self.visit(&p.expression);
            }
            AST::Unary(u) => {
                self.visit(&u.expr);
            }
            AST::TypeofExpression(t) => {
                self.visit(&t.expression);
            }
            AST::VoidExpression(v) => {
                self.visit(&v.expression);
            }
            AST::NonNullAssert(n) => {
                self.visit(&n.expression);
            }
            AST::TemplateLiteral(t) => {
                for expr in &t.expressions {
                    self.visit(expr);
                }
            }
            AST::TaggedTemplateLiteral(t) => {
                self.visit(&t.tag);
                for expr in &t.template.expressions {
                    self.visit(expr);
                }
            }
            AST::ParenthesizedExpression(p) => {
                self.visit(&p.expression);
            }
            AST::RegularExpressionLiteral(_) |
            AST::EmptyExpr(_) |
            AST::ImplicitReceiver(_) |
            AST::ThisReceiver(_) |
            AST::LiteralPrimitive(_) => {
                // Leaf nodes
            }
        }
    }
}

impl Default for RecursiveAstVisitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_span() {
        let span = ParseSpan::new(0, 10);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 10);

        let abs_span = span.to_absolute(5);
        assert_eq!(abs_span.start, 5);
        assert_eq!(abs_span.end, 15);
    }

    #[test]
    fn test_literal_primitive() {
        let span = ParseSpan::new(0, 5);
        let source_span = AbsoluteSourceSpan::new(0, 5);

        let num = LiteralPrimitive::number(span.clone(), source_span.clone(), 42.0);
        match num {
            LiteralPrimitive::Number { value, .. } => assert_eq!(value, 42.0),
            _ => panic!("Expected number"),
        }

        let str_val = LiteralPrimitive::string(span.clone(), source_span.clone(), "hello".to_string());
        match str_val {
            LiteralPrimitive::String { value, .. } => assert_eq!(value, "hello"),
            _ => panic!("Expected string"),
        }
    }

    #[test]
    fn test_recursive_visitor() {
        let visitor = RecursiveAstVisitor::new();
        let ast = AST::LiteralPrimitive(LiteralPrimitive::number(
            ParseSpan::new(0, 1),
            AbsoluteSourceSpan::new(0, 1),
            42.0,
        ));
        visitor.visit(&ast); // Should not panic
    }
}
