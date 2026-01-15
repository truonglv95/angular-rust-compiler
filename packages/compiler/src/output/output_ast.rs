//! Output AST Module
//!
//! Corresponds to packages/compiler/src/output/output_ast.ts
//! Defines the AST for output code generation

use crate::output::abstract_emitter::HasSourceSpan;
use crate::parse_util::ParseSourceSpan;
use std::any::Any;

//// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeModifier {
    None = 0,
    Const = 1,
}

pub trait TypeTrait {
    fn modifiers(&self) -> TypeModifier;
    fn visit_type(
        &self,
        visitor: &mut dyn TypeVisitor,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn has_modifier(&self, modifier: TypeModifier) -> bool {
        self.modifiers() as u8 & modifier as u8 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinTypeName {
    Dynamic,
    Bool,
    String,
    Int,
    Number,
    Function,
    Inferred,
    None,
}

#[derive(Debug, Clone)]
pub struct BuiltinType {
    pub name: BuiltinTypeName,
    pub modifiers: TypeModifier,
}

impl BuiltinType {
    pub fn new(name: BuiltinTypeName, modifiers: Option<TypeModifier>) -> Self {
        BuiltinType {
            name,
            modifiers: modifiers.unwrap_or(TypeModifier::None),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExpressionType {
    pub value: Box<Expression>,
    pub modifiers: TypeModifier,
    pub type_params: Option<Vec<Type>>,
}

#[derive(Debug, Clone)]
pub struct ArrayType {
    pub of: Box<Type>,
    pub modifiers: TypeModifier,
}

#[derive(Debug, Clone)]
pub struct MapType {
    pub value_type: Option<Box<Type>>,
    pub modifiers: TypeModifier,
}

#[derive(Debug)]
pub struct TransplantedType<T> {
    pub type_: T,
    pub modifiers: TypeModifier,
}

impl<T: Clone> Clone for TransplantedType<T> {
    fn clone(&self) -> Self {
        TransplantedType {
            type_: self.type_.clone(),
            modifiers: self.modifiers,
        }
    }
}

#[derive(Debug)]
pub enum Type {
    Builtin(BuiltinType),
    Expression(ExpressionType),
    Array(ArrayType),
    Map(MapType),
    Transplanted(TransplantedType<Box<dyn std::any::Any>>),
}

impl Clone for Type {
    fn clone(&self) -> Self {
        match self {
            Type::Builtin(t) => Type::Builtin(t.clone()),
            Type::Expression(t) => Type::Expression(t.clone()),
            Type::Array(t) => Type::Array(t.clone()),
            Type::Map(t) => Type::Map(t.clone()),
            Type::Transplanted(t) => {
                // For TransplantedType with Box<dyn Any>, we can't clone the inner type
                // So we create a new TransplantedType with the same modifiers but a new empty Box
                // This is a limitation - the actual type information is lost on clone
                Type::Transplanted(TransplantedType {
                    type_: Box::new(()), // Placeholder - actual type info cannot be cloned
                    modifiers: t.modifiers,
                })
            }
        }
    }
}

// Predefined types
pub fn dynamic_type() -> Type {
    Type::Builtin(BuiltinType::new(BuiltinTypeName::Dynamic, None))
}

pub fn inferred_type() -> Type {
    Type::Builtin(BuiltinType::new(BuiltinTypeName::Inferred, None))
}

pub fn bool_type() -> Type {
    Type::Builtin(BuiltinType::new(BuiltinTypeName::Bool, None))
}

pub fn int_type() -> Type {
    Type::Builtin(BuiltinType::new(BuiltinTypeName::Int, None))
}

pub fn number_type() -> Type {
    Type::Builtin(BuiltinType::new(BuiltinTypeName::Number, None))
}

pub fn string_type() -> Type {
    Type::Builtin(BuiltinType::new(BuiltinTypeName::String, None))
}

pub fn function_type() -> Type {
    Type::Builtin(BuiltinType::new(BuiltinTypeName::Function, None))
}

pub fn none_type() -> Type {
    Type::Builtin(BuiltinType::new(BuiltinTypeName::None, None))
}

pub trait TypeVisitor {
    fn visit_builtin_type(
        &mut self,
        type_: &BuiltinType,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_expression_type(
        &mut self,
        type_: &ExpressionType,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_array_type(
        &mut self,
        type_: &ArrayType,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_map_type(
        &mut self,
        type_: &MapType,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_transplanted_type(
        &mut self,
        type_: &TransplantedType<Box<dyn std::any::Any>>,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
}

///// Expressions

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Minus,
    Plus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    Equals,
    NotEquals,
    Assign,
    Identical,
    NotIdentical,
    Minus,
    Plus,
    Divide,
    Multiply,
    Modulo,
    And,
    Or,
    BitwiseOr,
    BitwiseAnd,
    Lower,
    LowerEquals,
    Bigger,
    BiggerEquals,
    NullishCoalesce,
    Exponentiation,
    In,
    AdditionAssignment,
    SubtractionAssignment,
    MultiplicationAssignment,
    DivisionAssignment,
    RemainderAssignment,
    ExponentiationAssignment,
    AndAssignment,
    OrAssignment,
    NullishCoalesceAssignment,
}

/// Base trait for all expressions
pub trait ExpressionTrait {
    fn type_(&self) -> Option<&Type>;
    fn source_span(&self) -> Option<&ParseSourceSpan>;
    fn visit_expression(
        &self,
        visitor: &mut dyn ExpressionVisitor,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn is_equivalent(&self, e: &Expression) -> bool;
    fn is_constant(&self) -> bool;
    fn clone_expr(&self) -> Expression;
}

#[derive(Debug, Clone)]
pub enum Expression {
    ReadVar(ReadVarExpr),
    WriteVar(WriteVarExpr),
    WriteKey(WriteKeyExpr),
    WriteProp(WritePropExpr),
    InvokeFn(InvokeFunctionExpr),
    TaggedTemplate(TaggedTemplateLiteralExpr),
    Instantiate(InstantiateExpr),
    Literal(LiteralExpr),
    TemplateLiteral(TemplateLiteralExpr),
    Localized(LocalizedString),
    External(ExternalExpr),
    ExternalRef(ExternalReference),
    Conditional(ConditionalExpr),
    DynamicImport(DynamicImportExpr),
    NotExpr(NotExpr),
    FnParam(FnParam),
    IfNull(IfNullExpr),
    AssertNotNull(AssertNotNullExpr),
    Cast(CastExpr),
    Fn(FunctionExpr),
    ArrowFn(ArrowFunctionExpr),
    BinaryOp(BinaryOperatorExpr),
    ReadProp(ReadPropExpr),
    ReadKey(ReadKeyExpr),
    LiteralArray(LiteralArrayExpr),
    LiteralMap(LiteralMapExpr),
    CommaExpr(CommaExpr),
    WrappedNode(WrappedNodeExpr),
    TypeOf(TypeofExpr),
    Void(VoidExpr),
    Unary(UnaryOperatorExpr),
    Parens(ParenthesizedExpr),
    RegularExpressionLiteral(RegularExpressionLiteralExpr),
    RawCode(RawCodeExpr),

    // IR Expression variants
    LexicalRead(crate::template::pipeline::ir::expression::LexicalReadExpr),
    Reference(crate::template::pipeline::ir::expression::ReferenceExpr),
    Context(crate::template::pipeline::ir::expression::ContextExpr),
    NextContext(crate::template::pipeline::ir::expression::NextContextExpr),
    GetCurrentView(crate::template::pipeline::ir::expression::GetCurrentViewExpr),
    RestoreView(crate::template::pipeline::ir::expression::RestoreViewExpr),
    ResetView(crate::template::pipeline::ir::expression::ResetViewExpr),
    ReadVariable(crate::template::pipeline::ir::expression::ReadVariableExpr),
    PureFunction(crate::template::pipeline::ir::expression::PureFunctionExpr),
    PureFunctionParameter(crate::template::pipeline::ir::expression::PureFunctionParameterExpr),
    PipeBinding(crate::template::pipeline::ir::expression::PipeBindingExpr),
    PipeBindingVariadic(crate::template::pipeline::ir::expression::PipeBindingVariadicExpr),
    SafePropertyRead(crate::template::pipeline::ir::expression::SafePropertyReadExpr),
    SafeKeyedRead(crate::template::pipeline::ir::expression::SafeKeyedReadExpr),
    SafeInvokeFunction(crate::template::pipeline::ir::expression::SafeInvokeFunctionExpr),
    SafeTernary(crate::template::pipeline::ir::expression::SafeTernaryExpr),
    Empty(crate::template::pipeline::ir::expression::EmptyExpr),
    AssignTemporary(crate::template::pipeline::ir::expression::AssignTemporaryExpr),
    ReadTemporary(crate::template::pipeline::ir::expression::ReadTemporaryExpr),
    SlotLiteral(crate::template::pipeline::ir::expression::SlotLiteralExpr),
    ConditionalCase(crate::template::pipeline::ir::expression::ConditionalCaseExpr),
    ConstCollected(crate::template::pipeline::ir::expression::ConstCollectedExpr),
    TwoWayBindingSet(crate::template::pipeline::ir::expression::TwoWayBindingSetExpr),
    ContextLetReference(crate::template::pipeline::ir::expression::ContextLetReferenceExpr),
    StoreLet(crate::template::pipeline::ir::expression::StoreLetExpr),
    TrackContext(crate::template::pipeline::ir::expression::TrackContextExpr),
}

#[derive(Debug, Clone)]
pub struct ReadVarExpr {
    pub name: String,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct WriteVarExpr {
    pub name: String,
    pub value: Box<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct WriteKeyExpr {
    pub receiver: Box<Expression>,
    pub index: Box<Expression>,
    pub value: Box<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct WritePropExpr {
    pub receiver: Box<Expression>,
    pub name: String,
    pub value: Box<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct InvokeFunctionExpr {
    pub fn_: Box<Expression>,
    pub args: Vec<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
    pub pure: bool,
}

#[derive(Debug, Clone)]
pub struct TaggedTemplateLiteralExpr {
    pub tag: Box<Expression>,
    pub template: TemplateLiteral,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct InstantiateExpr {
    pub class_expr: Box<Expression>,
    pub args: Vec<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct LiteralExpr {
    pub value: LiteralValue,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

impl LiteralExpr {
    pub fn is_equivalent(&self, other: &Self) -> bool {
        self.value.is_equivalent(&other.value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Null,
    Undefined,
    String(String),
    Number(f64),
    Bool(bool),
}

impl LiteralValue {
    pub fn is_equivalent(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Debug, Clone)]
pub struct TemplateLiteralExpr {
    pub elements: Vec<TemplateLiteralElement>,
    pub expressions: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct TemplateLiteral {
    pub elements: Vec<TemplateLiteralElement>,
    pub expressions: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct TemplateLiteralElement {
    pub text: String,
    pub raw_text: String,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct LocalizedString {
    pub meta_block: String,
    pub message_parts: Vec<LiteralPiece>,
    pub placeholder_names: Vec<PlaceholderPiece>,
    pub expressions: Vec<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct LiteralPiece {
    pub text: String,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone)]
pub struct PlaceholderPiece {
    pub text: String,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone)]
pub struct ExternalExpr {
    pub value: ExternalReference,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

impl ExternalExpr {
    pub fn is_equivalent(&self, other: &Self) -> bool {
        self.value.module_name == other.value.module_name && self.value.name == other.value.name
    }
}

#[derive(Debug)]
pub struct ExternalReference {
    pub module_name: Option<String>,
    pub name: Option<String>,
    pub runtime: Option<Box<dyn std::any::Any>>,
}

impl Clone for ExternalReference {
    fn clone(&self) -> Self {
        ExternalReference {
            module_name: self.module_name.clone(),
            name: self.name.clone(),
            runtime: None, // Cannot clone Box<dyn Any>, so set to None
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConditionalExpr {
    pub condition: Box<Expression>,
    pub true_case: Box<Expression>,
    pub false_case: Option<Box<Expression>>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct DynamicImportExpr {
    pub url: String,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct NotExpr {
    pub condition: Box<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct FnParam {
    pub name: String,
    pub type_: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct IfNullExpr {
    pub condition: Box<Expression>,
    pub null_case: Box<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct AssertNotNullExpr {
    pub condition: Box<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct CastExpr {
    pub value: Box<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct FunctionExpr {
    pub params: Vec<FnParam>,
    pub statements: Vec<Statement>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ArrowFunctionExpr {
    pub params: Vec<FnParam>,
    pub body: ArrowFunctionBody,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub enum ArrowFunctionBody {
    Expression(Box<Expression>),
    Statements(Vec<Statement>),
}

#[derive(Debug, Clone)]
pub struct BinaryOperatorExpr {
    pub operator: BinaryOperator,
    pub lhs: Box<Expression>,
    pub rhs: Box<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct ReadPropExpr {
    pub receiver: Box<Expression>,
    pub name: String,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct ReadKeyExpr {
    pub receiver: Box<Expression>,
    pub index: Box<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct LiteralArrayExpr {
    pub entries: Vec<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

impl LiteralArrayExpr {
    pub fn is_equivalent(&self, other: &Self) -> bool {
        if self.entries.len() != other.entries.len() {
            return false;
        }
        for (i, entry) in self.entries.iter().enumerate() {
            if !entry.is_equivalent(&other.entries[i]) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone)]
pub struct LiteralMapEntry {
    pub key: String,
    pub value: Box<Expression>,
    pub quoted: bool,
}

#[derive(Debug, Clone)]
pub struct RawCodeExpr {
    pub code: String,
    pub source_span: Option<ParseSourceSpan>,
}

impl HasSourceSpan for RawCodeExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct LiteralMapExpr {
    pub entries: Vec<LiteralMapEntry>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct CommaExpr {
    pub parts: Vec<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug)]
pub struct WrappedNodeExpr {
    pub node: Box<dyn std::any::Any>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

impl Clone for WrappedNodeExpr {
    fn clone(&self) -> Self {
        WrappedNodeExpr {
            node: Box::new(()), // Cannot clone Box<dyn Any>, so use placeholder
            type_: self.type_.clone(),
            source_span: self.source_span.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeofExpr {
    pub expr: Box<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct VoidExpr {
    pub expr: Box<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct UnaryOperatorExpr {
    pub operator: UnaryOperator,
    pub expr: Box<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct ParenthesizedExpr {
    pub expr: Box<Expression>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct RegularExpressionLiteralExpr {
    pub pattern: String,
    pub flags: String,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
}

pub trait ExpressionVisitor {
    fn visit_raw_code_expr(
        &mut self,
        expr: &RawCodeExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_read_var_expr(
        &mut self,
        expr: &ReadVarExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_write_var_expr(
        &mut self,
        expr: &WriteVarExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_write_key_expr(
        &mut self,
        expr: &WriteKeyExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_write_prop_expr(
        &mut self,
        expr: &WritePropExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_invoke_function_expr(
        &mut self,
        expr: &InvokeFunctionExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_tagged_template_expr(
        &mut self,
        expr: &TaggedTemplateLiteralExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_instantiate_expr(
        &mut self,
        expr: &InstantiateExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_literal_expr(
        &mut self,
        expr: &LiteralExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_localized_string(
        &mut self,
        expr: &LocalizedString,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_external_expr(
        &mut self,
        expr: &ExternalExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_binary_operator_expr(
        &mut self,
        expr: &BinaryOperatorExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_read_prop_expr(
        &mut self,
        expr: &ReadPropExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_read_key_expr(
        &mut self,
        expr: &ReadKeyExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_conditional_expr(
        &mut self,
        expr: &ConditionalExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_unary_operator_expr(
        &mut self,
        expr: &UnaryOperatorExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_parenthesized_expr(
        &mut self,
        expr: &ParenthesizedExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_function_expr(
        &mut self,
        expr: &FunctionExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_arrow_function_expr(
        &mut self,
        expr: &ArrowFunctionExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_literal_array_expr(
        &mut self,
        expr: &LiteralArrayExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_literal_map_expr(
        &mut self,
        expr: &LiteralMapExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_comma_expr(
        &mut self,
        expr: &CommaExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_typeof_expr(
        &mut self,
        expr: &TypeofExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_void_expr(
        &mut self,
        expr: &VoidExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_not_expr(
        &mut self,
        expr: &NotExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_if_null_expr(
        &mut self,
        expr: &IfNullExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_assert_not_null_expr(
        &mut self,
        expr: &AssertNotNullExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_cast_expr(
        &mut self,
        expr: &CastExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_dynamic_import_expr(
        &mut self,
        expr: &DynamicImportExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_template_literal_expr(
        &mut self,
        expr: &TemplateLiteralExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_regular_expression_literal(
        &mut self,
        expr: &RegularExpressionLiteralExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_wrapped_node_expr(
        &mut self,
        expr: &WrappedNodeExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;

    // IR Expression visitor methods
    fn visit_lexical_read_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::LexicalReadExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_reference_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ReferenceExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_context_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ContextExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_next_context_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::NextContextExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_get_current_view_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::GetCurrentViewExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_restore_view_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::RestoreViewExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_reset_view_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ResetViewExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_read_variable_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ReadVariableExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_pure_function_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::PureFunctionExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_pure_function_parameter_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::PureFunctionParameterExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_pipe_binding_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::PipeBindingExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_pipe_binding_variadic_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::PipeBindingVariadicExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_safe_property_read_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SafePropertyReadExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_safe_keyed_read_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SafeKeyedReadExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_safe_invoke_function_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SafeInvokeFunctionExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_safe_ternary_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SafeTernaryExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_empty_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::EmptyExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_assign_temporary_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::AssignTemporaryExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_read_temporary_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ReadTemporaryExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_slot_literal_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::SlotLiteralExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_conditional_case_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ConditionalCaseExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_const_collected_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ConstCollectedExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_two_way_binding_set_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::TwoWayBindingSetExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_context_let_reference_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::ContextLetReferenceExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_store_let_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::StoreLetExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_track_context_expr(
        &mut self,
        expr: &crate::template::pipeline::ir::expression::TrackContextExpr,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
}

///// Statements

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StmtModifier {
    None = 0,
    Final = 1,
    Private = 2,
    Exported = 4,
    Static = 8,
}

#[derive(Debug, Clone)]
pub enum Statement {
    DeclareVar(DeclareVarStmt),
    DeclareFn(DeclareFunctionStmt),
    Expression(ExpressionStatement),
    Return(ReturnStatement),
    IfStmt(IfStmt),
    Block(BlockStmt),
}

#[derive(Debug, Clone)]
pub struct BlockStmt {
    pub statements: Vec<Statement>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct DeclareVarStmt {
    pub name: String,
    pub value: Option<Box<Expression>>,
    pub type_: Option<Type>,
    pub modifiers: StmtModifier,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct DeclareFunctionStmt {
    pub name: String,
    pub params: Vec<FnParam>,
    pub statements: Vec<Statement>,
    pub type_: Option<Type>,
    pub modifiers: StmtModifier,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct ExpressionStatement {
    pub expr: Box<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub value: Box<Expression>,
    pub source_span: Option<ParseSourceSpan>,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Box<Expression>,
    pub true_case: Vec<Statement>,
    pub false_case: Vec<Statement>,
    pub source_span: Option<ParseSourceSpan>,
}

pub trait StatementVisitor {
    fn visit_declare_var_stmt(
        &mut self,
        stmt: &DeclareVarStmt,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_declare_function_stmt(
        &mut self,
        stmt: &DeclareFunctionStmt,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_expression_stmt(
        &mut self,
        stmt: &ExpressionStatement,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_return_stmt(
        &mut self,
        stmt: &ReturnStatement,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_if_stmt(
        &mut self,
        stmt: &IfStmt,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
    fn visit_block_stmt(
        &mut self,
        stmt: &BlockStmt,
        context: &mut dyn std::any::Any,
    ) -> Box<dyn std::any::Any>;
}

// Helper functions for creating common expressions
pub fn variable(name: impl Into<String>) -> Box<Expression> {
    Box::new(Expression::ReadVar(ReadVarExpr {
        name: name.into(),
        type_: None,
        source_span: None,
    }))
}

pub fn literal(value: impl Into<LiteralValue>) -> Box<Expression> {
    Box::new(Expression::Literal(LiteralExpr {
        value: value.into(),
        type_: None,
        source_span: None,
    }))
}

pub fn literal_arr(values: Vec<Expression>) -> Box<Expression> {
    Box::new(Expression::LiteralArray(LiteralArrayExpr {
        entries: values,
        type_: None,
        source_span: None,
    }))
}

pub fn literal_map(entries: Vec<LiteralMapEntry>) -> Box<Expression> {
    Box::new(Expression::LiteralMap(LiteralMapExpr {
        entries,
        type_: None,
        source_span: None,
    }))
}

pub fn import_ref(id: ExternalReference) -> Box<Expression> {
    Box::new(Expression::External(ExternalExpr {
        value: id,
        type_: None,
        source_span: None,
    }))
}

// Implement conversions
impl From<String> for LiteralValue {
    fn from(s: String) -> Self {
        LiteralValue::String(s)
    }
}

impl From<&str> for LiteralValue {
    fn from(s: &str) -> Self {
        LiteralValue::String(s.to_string())
    }
}

impl From<f64> for LiteralValue {
    fn from(n: f64) -> Self {
        LiteralValue::Number(n)
    }
}

impl From<bool> for LiteralValue {
    fn from(b: bool) -> Self {
        LiteralValue::Bool(b)
    }
}

impl Expression {
    pub fn is_equivalent(&self, other: &Self) -> bool {
        match (self, other) {
            (Expression::Literal(a), Expression::Literal(b)) => a.is_equivalent(b),
            (Expression::LiteralArray(a), Expression::LiteralArray(b)) => a.is_equivalent(b),
            (Expression::External(a), Expression::External(b)) => a.is_equivalent(b),
            (Expression::ReadVar(a), Expression::ReadVar(b)) => a.name == b.name,
            // Add more cases as needed, or fall back to false
            _ => false,
        }
    }

    pub fn prop(
        &self,
        name: impl Into<String>,
        source_span: Option<ParseSourceSpan>,
    ) -> Box<Expression> {
        Box::new(Expression::ReadProp(ReadPropExpr {
            receiver: Box::new(self.clone()),
            name: name.into(),
            type_: None,
            source_span,
        }))
    }

    pub fn key(
        &self,
        index: Box<Expression>,
        type_: Option<Type>,
        source_span: Option<ParseSourceSpan>,
    ) -> Box<Expression> {
        Box::new(Expression::ReadKey(ReadKeyExpr {
            receiver: Box::new(self.clone()),
            index,
            type_,
            source_span,
        }))
    }

    pub fn call_fn(
        &self,
        params: Vec<Expression>,
        source_span: Option<ParseSourceSpan>,
        pure: Option<bool>,
    ) -> Box<Expression> {
        Box::new(Expression::InvokeFn(InvokeFunctionExpr {
            fn_: Box::new(self.clone()),
            args: params,
            type_: None,
            source_span,
            pure: pure.unwrap_or(false),
        }))
    }

    pub fn instantiate(
        &self,
        params: Vec<Expression>,
        type_: Option<Type>,
        source_span: Option<ParseSourceSpan>,
    ) -> Box<Expression> {
        Box::new(Expression::Instantiate(InstantiateExpr {
            class_expr: Box::new(self.clone()),
            args: params,
            type_,
            source_span,
        }))
    }

    pub fn conditional(
        &self,
        true_case: Box<Expression>,
        false_case: Option<Box<Expression>>,
        source_span: Option<ParseSourceSpan>,
    ) -> Box<Expression> {
        Box::new(Expression::Conditional(ConditionalExpr {
            condition: Box::new(self.clone()),
            true_case,
            false_case,
            type_: None,
            source_span,
        }))
    }

    pub fn to_stmt(&self) -> Statement {
        Statement::Expression(ExpressionStatement {
            expr: Box::new(self.clone()),
            source_span: None,
        })
    }

    /// Creates a binary OR expression (||)
    pub fn or(
        &self,
        rhs: Box<Expression>,
        source_span: Option<ParseSourceSpan>,
    ) -> Box<Expression> {
        Box::new(Expression::BinaryOp(BinaryOperatorExpr {
            operator: BinaryOperator::Or,
            lhs: Box::new(self.clone()),
            rhs,
            type_: None,
            source_span,
        }))
    }

    /// Creates a write property expression (property assignment)
    pub fn set(
        &self,
        value: Box<Expression>,
        source_span: Option<ParseSourceSpan>,
    ) -> Box<Expression> {
        // Check if self is ReadPropExpr or ReadKeyExpr
        match self {
            Expression::ReadProp(read_prop) => Box::new(Expression::WriteProp(WritePropExpr {
                receiver: read_prop.receiver.clone(),
                name: read_prop.name.clone(),
                value,
                type_: None,
                source_span,
            })),
            Expression::ReadKey(read_key) => Box::new(Expression::WriteKey(WriteKeyExpr {
                receiver: read_key.receiver.clone(),
                index: read_key.index.clone(),
                value,
                type_: None,
                source_span,
            })),
            _ => {
                panic!("set() can only be called on ReadPropExpr or ReadKeyExpr");
            }
        }
    }
}

// Additional helper functions
pub fn import_expr(module_name: impl Into<String>, name: impl Into<String>) -> Box<Expression> {
    Box::new(Expression::External(ExternalExpr {
        value: ExternalReference {
            module_name: Some(module_name.into()),
            name: Some(name.into()),
            runtime: None,
        },
        type_: None,
        source_span: None,
    }))
}

pub fn not(expr: Box<Expression>) -> Box<Expression> {
    Box::new(Expression::NotExpr(NotExpr {
        condition: expr,
        source_span: None,
    }))
}

pub fn fn_expr(
    params: Vec<FnParam>,
    statements: Vec<Statement>,
    type_: Option<Type>,
    name: Option<String>,
) -> Box<Expression> {
    Box::new(Expression::Fn(FunctionExpr {
        params,
        statements,
        type_,
        source_span: None,
        name,
    }))
}

pub fn arrow_fn(
    params: Vec<FnParam>,
    body: ArrowFunctionBody,
    type_: Option<Type>,
) -> Box<Expression> {
    Box::new(Expression::ArrowFn(ArrowFunctionExpr {
        params,
        body,
        type_,
        source_span: None,
    }))
}

pub fn null_expr() -> Box<Expression> {
    literal(LiteralValue::Null)
}

pub fn typed_null_expr() -> Box<Expression> {
    Box::new(Expression::Literal(LiteralExpr {
        value: LiteralValue::Null,
        type_: Some(none_type()),
        source_span: None,
    }))
}

// Clone implementation for Expression
impl Expression {
    pub fn clone(&self) -> Self {
        match self {
            Expression::ReadVar(e) => Expression::ReadVar(e.clone()),
            Expression::WriteVar(e) => Expression::WriteVar(e.clone()),
            Expression::WriteKey(e) => Expression::WriteKey(e.clone()),
            Expression::WriteProp(e) => Expression::WriteProp(e.clone()),
            Expression::InvokeFn(e) => Expression::InvokeFn(e.clone()),
            Expression::TaggedTemplate(e) => Expression::TaggedTemplate(e.clone()),
            Expression::Instantiate(e) => Expression::Instantiate(e.clone()),
            Expression::Literal(e) => Expression::Literal(e.clone()),
            Expression::TemplateLiteral(e) => Expression::TemplateLiteral(e.clone()),
            Expression::Localized(e) => Expression::Localized(e.clone()),
            Expression::External(e) => Expression::External(e.clone()),
            Expression::ExternalRef(e) => Expression::ExternalRef(e.clone()),
            Expression::RawCode(e) => Expression::RawCode(e.clone()),
            Expression::Conditional(e) => Expression::Conditional(e.clone()),
            Expression::DynamicImport(e) => Expression::DynamicImport(e.clone()),
            Expression::NotExpr(e) => Expression::NotExpr(e.clone()),
            Expression::FnParam(e) => Expression::FnParam(e.clone()),
            Expression::IfNull(e) => Expression::IfNull(e.clone()),
            Expression::AssertNotNull(e) => Expression::AssertNotNull(e.clone()),
            Expression::Cast(e) => Expression::Cast(e.clone()),
            Expression::Fn(e) => Expression::Fn(e.clone()),
            Expression::ArrowFn(e) => Expression::ArrowFn(e.clone()),
            Expression::BinaryOp(e) => Expression::BinaryOp(e.clone()),
            Expression::ReadProp(e) => Expression::ReadProp(e.clone()),
            Expression::ReadKey(e) => Expression::ReadKey(e.clone()),
            Expression::LiteralArray(e) => Expression::LiteralArray(e.clone()),
            Expression::LiteralMap(e) => Expression::LiteralMap(e.clone()),
            Expression::CommaExpr(e) => Expression::CommaExpr(e.clone()),
            Expression::WrappedNode(e) => Expression::WrappedNode(e.clone()),
            Expression::TypeOf(e) => Expression::TypeOf(e.clone()),
            Expression::Void(e) => Expression::Void(e.clone()),
            Expression::Unary(e) => Expression::Unary(e.clone()),
            Expression::Parens(e) => Expression::Parens(e.clone()),
            Expression::RegularExpressionLiteral(e) => {
                Expression::RegularExpressionLiteral(e.clone())
            }
            // IR Expression variants
            Expression::LexicalRead(e) => Expression::LexicalRead(e.clone()),
            Expression::Reference(e) => Expression::Reference(e.clone()),
            Expression::Context(e) => Expression::Context(e.clone()),
            Expression::NextContext(e) => Expression::NextContext(e.clone()),
            Expression::GetCurrentView(e) => Expression::GetCurrentView(e.clone()),
            Expression::RestoreView(e) => Expression::RestoreView(e.clone()),
            Expression::ResetView(e) => Expression::ResetView(e.clone()),
            Expression::ReadVariable(e) => Expression::ReadVariable(e.clone()),
            Expression::PureFunction(e) => Expression::PureFunction(e.clone()),
            Expression::PureFunctionParameter(e) => Expression::PureFunctionParameter(e.clone()),
            Expression::PipeBinding(e) => Expression::PipeBinding(e.clone()),
            Expression::PipeBindingVariadic(e) => Expression::PipeBindingVariadic(e.clone()),
            Expression::SafePropertyRead(e) => Expression::SafePropertyRead(e.clone()),
            Expression::SafeKeyedRead(e) => Expression::SafeKeyedRead(e.clone()),
            Expression::SafeInvokeFunction(e) => Expression::SafeInvokeFunction(e.clone()),
            Expression::SafeTernary(e) => Expression::SafeTernary(e.clone()),
            Expression::Empty(e) => Expression::Empty(e.clone()),
            Expression::AssignTemporary(e) => Expression::AssignTemporary(e.clone()),
            Expression::ReadTemporary(e) => Expression::ReadTemporary(e.clone()),
            Expression::SlotLiteral(e) => Expression::SlotLiteral(e.clone()),
            Expression::ConditionalCase(e) => Expression::ConditionalCase(e.clone()),
            Expression::ConstCollected(e) => Expression::ConstCollected(e.clone()),
            Expression::TwoWayBindingSet(e) => Expression::TwoWayBindingSet(e.clone()),
            Expression::ContextLetReference(e) => Expression::ContextLetReference(e.clone()),
            Expression::StoreLet(e) => Expression::StoreLet(e.clone()),
            Expression::TrackContext(e) => Expression::TrackContext(e.clone()),
        }
    }
}

// Implement HasSourceSpan for all expression and statement types
impl HasSourceSpan for BlockStmt {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for ReadVarExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for WriteVarExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for WriteKeyExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for WritePropExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for InvokeFunctionExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for InstantiateExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for LiteralExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for ExternalExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for DeclareVarStmt {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for DeclareFunctionStmt {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for ExpressionStatement {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for ReturnStatement {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for IfStmt {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for TaggedTemplateLiteralExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for BinaryOperatorExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for ReadPropExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for ReadKeyExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for ConditionalExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for UnaryOperatorExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for ParenthesizedExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for FunctionExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for ArrowFunctionExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for LiteralArrayExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for LiteralMapExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for CommaExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for TypeofExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for VoidExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for NotExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for IfNullExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for AssertNotNullExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for CastExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for DynamicImportExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

impl HasSourceSpan for TemplateLiteralExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        // TemplateLiteralExpr doesn't have source_span, return None
        None
    }
}

impl HasSourceSpan for RegularExpressionLiteralExpr {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        self.source_span.as_ref()
    }
}

// Implement ExpressionTrait for Expression enum
impl ExpressionTrait for Expression {
    fn type_(&self) -> Option<&Type> {
        match self {
            Expression::ReadVar(e) => e.type_.as_ref(),
            Expression::WriteVar(e) => e.type_.as_ref(),
            Expression::WriteKey(e) => e.type_.as_ref(),
            Expression::WriteProp(e) => e.type_.as_ref(),
            Expression::InvokeFn(e) => e.type_.as_ref(),
            Expression::TaggedTemplate(e) => e.type_.as_ref(),
            Expression::Instantiate(e) => e.type_.as_ref(),
            Expression::Literal(e) => e.type_.as_ref(),
            Expression::TemplateLiteral(_) => None,
            Expression::Localized(_) => None,
            Expression::External(e) => e.type_.as_ref(),
            Expression::ExternalRef(_) => None,
            Expression::Conditional(e) => e.type_.as_ref(),
            Expression::DynamicImport(_) => None,
            Expression::NotExpr(_) => None,
            Expression::FnParam(_) => None,
            Expression::IfNull(_) => None,
            Expression::AssertNotNull(_) => None,
            Expression::Cast(e) => e.type_.as_ref(),
            Expression::Fn(e) => e.type_.as_ref(),
            Expression::ArrowFn(e) => e.type_.as_ref(),
            Expression::BinaryOp(e) => e.type_.as_ref(),
            Expression::ReadProp(e) => e.type_.as_ref(),
            Expression::ReadKey(e) => e.type_.as_ref(),
            Expression::LiteralArray(e) => e.type_.as_ref(),
            Expression::LiteralMap(e) => e.type_.as_ref(),
            Expression::CommaExpr(_) => None,
            Expression::WrappedNode(e) => e.type_.as_ref(),
            Expression::TypeOf(e) => e.type_.as_ref(),
            Expression::Void(e) => e.type_.as_ref(),
            Expression::Unary(e) => e.type_.as_ref(),
            Expression::Parens(e) => e.type_.as_ref(),
            Expression::RegularExpressionLiteral(e) => e.type_.as_ref(),
            Expression::RawCode(_) => None,
            // IR Expression variants - typically don't have types
            Expression::LexicalRead(_)
            | Expression::Reference(_)
            | Expression::Context(_)
            | Expression::NextContext(_)
            | Expression::GetCurrentView(_)
            | Expression::RestoreView(_)
            | Expression::ResetView(_)
            | Expression::ReadVariable(_)
            | Expression::PureFunction(_)
            | Expression::PureFunctionParameter(_)
            | Expression::PipeBinding(_)
            | Expression::PipeBindingVariadic(_)
            | Expression::SafePropertyRead(_)
            | Expression::SafeKeyedRead(_)
            | Expression::SafeInvokeFunction(_)
            | Expression::SafeTernary(_)
            | Expression::Empty(_)
            | Expression::AssignTemporary(_)
            | Expression::ReadTemporary(_)
            | Expression::SlotLiteral(_)
            | Expression::ConditionalCase(_)
            | Expression::ConstCollected(_)
            | Expression::TwoWayBindingSet(_)
            | Expression::ContextLetReference(_)
            | Expression::StoreLet(_)
            | Expression::TrackContext(_) => None,
        }
    }

    fn source_span(&self) -> Option<&ParseSourceSpan> {
        match self {
            Expression::ReadVar(e) => e.source_span.as_ref(),
            Expression::WriteVar(e) => e.source_span.as_ref(),
            Expression::WriteKey(e) => e.source_span.as_ref(),
            Expression::WriteProp(e) => e.source_span.as_ref(),
            Expression::InvokeFn(e) => e.source_span.as_ref(),
            Expression::TaggedTemplate(e) => e.source_span.as_ref(),
            Expression::Instantiate(e) => e.source_span.as_ref(),
            Expression::Literal(e) => e.source_span.as_ref(),
            Expression::TemplateLiteral(_) => None,
            Expression::Localized(e) => e.source_span.as_ref(),
            Expression::External(e) => e.source_span.as_ref(),
            Expression::ExternalRef(_) => None,
            Expression::Conditional(e) => e.source_span.as_ref(),
            Expression::DynamicImport(e) => e.source_span.as_ref(),
            Expression::NotExpr(e) => e.source_span.as_ref(),
            Expression::FnParam(_) => None,
            Expression::IfNull(e) => e.source_span.as_ref(),
            Expression::AssertNotNull(e) => e.source_span.as_ref(),
            Expression::Cast(e) => e.source_span.as_ref(),
            Expression::Fn(e) => e.source_span.as_ref(),
            Expression::ArrowFn(e) => e.source_span.as_ref(),
            Expression::BinaryOp(e) => e.source_span.as_ref(),
            Expression::ReadProp(e) => e.source_span.as_ref(),
            Expression::ReadKey(e) => e.source_span.as_ref(),
            Expression::LiteralArray(e) => e.source_span.as_ref(),
            Expression::LiteralMap(e) => e.source_span.as_ref(),
            Expression::CommaExpr(e) => e.source_span.as_ref(),
            Expression::WrappedNode(e) => e.source_span.as_ref(),
            Expression::TypeOf(e) => e.source_span.as_ref(),
            Expression::Void(e) => e.source_span.as_ref(),
            Expression::Unary(e) => e.source_span.as_ref(),
            Expression::Parens(e) => e.source_span.as_ref(),
            Expression::RegularExpressionLiteral(e) => e.source_span.as_ref(),
            Expression::RawCode(e) => e.source_span.as_ref(),
            // IR Expression variants - delegate to their source_span
            Expression::LexicalRead(e) => e.source_span.as_ref(),
            Expression::Reference(e) => e.source_span.as_ref(),
            Expression::Context(e) => e.source_span.as_ref(),
            Expression::NextContext(e) => e.source_span.as_ref(),
            Expression::GetCurrentView(e) => e.source_span.as_ref(),
            Expression::RestoreView(e) => e.source_span.as_ref(),
            Expression::ResetView(e) => e.source_span.as_ref(),
            Expression::ReadVariable(e) => e.source_span.as_ref(),
            Expression::PureFunction(e) => e.source_span.as_ref(),
            Expression::PureFunctionParameter(e) => e.source_span.as_ref(),
            Expression::PipeBinding(e) => e.source_span.as_ref(),
            Expression::PipeBindingVariadic(e) => e.source_span.as_ref(),
            Expression::SafePropertyRead(e) => e.source_span.as_ref(),
            Expression::SafeKeyedRead(e) => e.source_span.as_ref(),
            Expression::SafeInvokeFunction(e) => e.source_span.as_ref(),
            Expression::SafeTernary(e) => e.source_span.as_ref(),
            Expression::Empty(e) => e.source_span.as_ref(),
            Expression::AssignTemporary(e) => e.source_span.as_ref(),
            Expression::ReadTemporary(e) => e.source_span.as_ref(),
            Expression::SlotLiteral(e) => e.source_span.as_ref(),
            Expression::ConditionalCase(e) => e.source_span.as_ref(),
            Expression::ConstCollected(e) => e.source_span.as_ref(),
            Expression::TwoWayBindingSet(e) => e.source_span.as_ref(),
            Expression::ContextLetReference(e) => e.source_span.as_ref(),
            Expression::StoreLet(e) => Some(&e.source_span), // StoreLetExpr has non-optional source_span
            Expression::TrackContext(e) => e.source_span.as_ref(),
        }
    }

    fn visit_expression(
        &self,
        visitor: &mut dyn ExpressionVisitor,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        match self {
            Expression::ReadVar(e) => visitor.visit_read_var_expr(e, context),
            Expression::WriteVar(e) => visitor.visit_write_var_expr(e, context),
            Expression::WriteKey(e) => visitor.visit_write_key_expr(e, context),
            Expression::WriteProp(e) => visitor.visit_write_prop_expr(e, context),
            Expression::InvokeFn(e) => visitor.visit_invoke_function_expr(e, context),
            Expression::TaggedTemplate(e) => visitor.visit_tagged_template_expr(e, context),
            Expression::Instantiate(e) => visitor.visit_instantiate_expr(e, context),
            Expression::Literal(e) => visitor.visit_literal_expr(e, context),
            Expression::TemplateLiteral(e) => visitor.visit_template_literal_expr(e, context),
            Expression::Localized(e) => visitor.visit_localized_string(e, context),
            Expression::External(e) => visitor.visit_external_expr(e, context),
            Expression::ExternalRef(_) => Box::new(()),
            Expression::Conditional(e) => visitor.visit_conditional_expr(e, context),
            Expression::DynamicImport(e) => visitor.visit_dynamic_import_expr(e, context),
            Expression::NotExpr(e) => visitor.visit_not_expr(e, context),
            Expression::FnParam(_) => Box::new(()),
            Expression::IfNull(e) => visitor.visit_if_null_expr(e, context),
            Expression::AssertNotNull(e) => visitor.visit_assert_not_null_expr(e, context),
            Expression::Cast(e) => visitor.visit_cast_expr(e, context),
            Expression::Fn(e) => visitor.visit_function_expr(e, context),
            Expression::ArrowFn(e) => visitor.visit_arrow_function_expr(e, context),
            Expression::BinaryOp(e) => visitor.visit_binary_operator_expr(e, context),
            Expression::ReadProp(e) => visitor.visit_read_prop_expr(e, context),
            Expression::ReadKey(e) => visitor.visit_read_key_expr(e, context),
            Expression::LiteralArray(e) => visitor.visit_literal_array_expr(e, context),
            Expression::LiteralMap(e) => visitor.visit_literal_map_expr(e, context),
            Expression::CommaExpr(e) => visitor.visit_comma_expr(e, context),
            Expression::WrappedNode(e) => visitor.visit_wrapped_node_expr(e, context),
            Expression::TypeOf(e) => visitor.visit_typeof_expr(e, context),
            Expression::Void(e) => visitor.visit_void_expr(e, context),
            Expression::Unary(e) => visitor.visit_unary_operator_expr(e, context),
            Expression::Parens(e) => visitor.visit_parenthesized_expr(e, context),
            Expression::RegularExpressionLiteral(e) => {
                visitor.visit_regular_expression_literal(e, context)
            }
            Expression::RawCode(e) => visitor.visit_raw_code_expr(e, context),
            // IR Expression variants
            Expression::LexicalRead(e) => visitor.visit_lexical_read_expr(e, context),
            Expression::Reference(e) => visitor.visit_reference_expr(e, context),
            Expression::Context(e) => visitor.visit_context_expr(e, context),
            Expression::NextContext(e) => visitor.visit_next_context_expr(e, context),
            Expression::GetCurrentView(e) => visitor.visit_get_current_view_expr(e, context),
            Expression::RestoreView(e) => visitor.visit_restore_view_expr(e, context),
            Expression::ResetView(e) => visitor.visit_reset_view_expr(e, context),
            Expression::ReadVariable(e) => visitor.visit_read_variable_expr(e, context),
            Expression::PureFunction(e) => visitor.visit_pure_function_expr(e, context),
            Expression::PureFunctionParameter(e) => {
                visitor.visit_pure_function_parameter_expr(e, context)
            }
            Expression::PipeBinding(e) => visitor.visit_pipe_binding_expr(e, context),
            Expression::PipeBindingVariadic(e) => {
                visitor.visit_pipe_binding_variadic_expr(e, context)
            }
            Expression::SafePropertyRead(e) => visitor.visit_safe_property_read_expr(e, context),
            Expression::SafeKeyedRead(e) => visitor.visit_safe_keyed_read_expr(e, context),
            Expression::SafeInvokeFunction(e) => {
                visitor.visit_safe_invoke_function_expr(e, context)
            }
            Expression::SafeTernary(e) => visitor.visit_safe_ternary_expr(e, context),
            Expression::Empty(e) => visitor.visit_empty_expr(e, context),
            Expression::AssignTemporary(e) => visitor.visit_assign_temporary_expr(e, context),
            Expression::ReadTemporary(e) => visitor.visit_read_temporary_expr(e, context),
            Expression::SlotLiteral(e) => visitor.visit_slot_literal_expr(e, context),
            Expression::ConditionalCase(e) => visitor.visit_conditional_case_expr(e, context),
            Expression::ConstCollected(e) => visitor.visit_const_collected_expr(e, context),
            Expression::TwoWayBindingSet(e) => visitor.visit_two_way_binding_set_expr(e, context),
            Expression::ContextLetReference(e) => {
                visitor.visit_context_let_reference_expr(e, context)
            }
            Expression::StoreLet(e) => visitor.visit_store_let_expr(e, context),
            Expression::TrackContext(e) => visitor.visit_track_context_expr(e, context),
        }
    }

    fn is_equivalent(&self, _e: &Expression) -> bool {
        false // TODO: Implement proper equivalence checking
    }

    fn is_constant(&self) -> bool {
        match self {
            Expression::Literal(_) => true,
            Expression::LiteralArray(arr) => arr.entries.iter().all(|e| e.is_constant()),
            Expression::LiteralMap(map) => map.entries.iter().all(|e| e.value.is_constant()),
            _ => false,
        }
    }

    fn clone_expr(&self) -> Expression {
        self.clone()
    }
}

// Add visit_statement method to Statement enum
impl Statement {
    pub fn visit_statement(
        &self,
        visitor: &mut dyn StatementVisitor,
        context: &mut dyn Any,
    ) -> Box<dyn Any> {
        match self {
            Statement::DeclareVar(s) => visitor.visit_declare_var_stmt(s, context),
            Statement::DeclareFn(s) => visitor.visit_declare_function_stmt(s, context),
            Statement::Expression(s) => visitor.visit_expression_stmt(s, context),
            Statement::Return(s) => visitor.visit_return_stmt(s, context),
            Statement::IfStmt(s) => visitor.visit_if_stmt(s, context),
            Statement::Block(s) => visitor.visit_block_stmt(s, context),
        }
    }
}
