//! Output AST Module
//!
//! Corresponds to packages/compiler/src/output/output_ast.ts
//! Defines the AST for output code generation

use crate::i18n::digest::compute_msg_id;
use crate::i18n::i18n_ast::Message;
use crate::parse_util::ParseSourceSpan;
use std::collections::HashMap;

//// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeModifier {
    None = 0,
    Const = 1,
}

pub trait TypeTrait {
    fn modifiers(&self) -> TypeModifier;
    fn visit_type(&self, visitor: &mut dyn TypeVisitor, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
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

#[derive(Debug, Clone)]
pub struct TransplantedType<T: Clone> {
    pub type_: T,
    pub modifiers: TypeModifier,
}

#[derive(Debug, Clone)]
pub enum Type {
    Builtin(BuiltinType),
    Expression(ExpressionType),
    Array(ArrayType),
    Map(MapType),
    Transplanted(TransplantedType<Box<dyn std::any::Any>>),
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
    fn visit_builtin_type(&mut self, type_: &BuiltinType, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_expression_type(&mut self, type_: &ExpressionType, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_array_type(&mut self, type_: &ArrayType, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_map_type(&mut self, type_: &MapType, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_transplanted_type(&mut self, type_: &TransplantedType<Box<dyn std::any::Any>>, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
}

///// Expressions

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Minus,
    Plus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    fn visit_expression(&self, visitor: &mut dyn ExpressionVisitor, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
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

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Null,
    String(String),
    Number(f64),
    Bool(bool),
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

#[derive(Debug, Clone)]
pub struct ExternalReference {
    pub module_name: Option<String>,
    pub name: Option<String>,
    pub runtime: Option<Box<dyn std::any::Any>>,
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

#[derive(Debug, Clone)]
pub struct LiteralMapEntry {
    pub key: String,
    pub value: Box<Expression>,
    pub quoted: bool,
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

#[derive(Debug, Clone)]
pub struct WrappedNodeExpr {
    pub node: Box<dyn std::any::Any>,
    pub type_: Option<Type>,
    pub source_span: Option<ParseSourceSpan>,
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

pub trait ExpressionVisitor {
    fn visit_read_var_expr(&mut self, expr: &ReadVarExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_write_var_expr(&mut self, expr: &WriteVarExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_write_key_expr(&mut self, expr: &WriteKeyExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_write_prop_expr(&mut self, expr: &WritePropExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_invoke_function_expr(&mut self, expr: &InvokeFunctionExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_tagged_template_expr(&mut self, expr: &TaggedTemplateLiteralExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_instantiate_expr(&mut self, expr: &InstantiateExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_literal_expr(&mut self, expr: &LiteralExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_localized_string(&mut self, expr: &LocalizedString, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_external_expr(&mut self, expr: &ExternalExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    // Add more visitor methods as needed...
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
    // Add more statement types as needed...
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
    fn visit_declare_var_stmt(&mut self, stmt: &DeclareVarStmt, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_declare_function_stmt(&mut self, stmt: &DeclareFunctionStmt, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_expression_stmt(&mut self, stmt: &ExpressionStatement, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_return_stmt(&mut self, stmt: &ReturnStatement, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    fn visit_if_stmt(&mut self, stmt: &IfStmt, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any>;
    // Add more visitor methods as needed...
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

// Helper methods for Expression enum
impl Expression {
    pub fn prop(&self, name: impl Into<String>, source_span: Option<ParseSourceSpan>) -> Box<Expression> {
        Box::new(Expression::ReadProp(ReadPropExpr {
            receiver: Box::new(self.clone()),
            name: name.into(),
            type_: None,
            source_span,
        }))
    }

    pub fn key(&self, index: Box<Expression>, type_: Option<Type>, source_span: Option<ParseSourceSpan>) -> Box<Expression> {
        Box::new(Expression::ReadKey(ReadKeyExpr {
            receiver: Box::new(self.clone()),
            index,
            type_,
            source_span,
        }))
    }

    pub fn call_fn(&self, params: Vec<Expression>, source_span: Option<ParseSourceSpan>, pure: Option<bool>) -> Box<Expression> {
        Box::new(Expression::InvokeFn(InvokeFunctionExpr {
            fn_: Box::new(self.clone()),
            args: params,
            type_: None,
            source_span,
            pure: pure.unwrap_or(false),
        }))
    }

    pub fn instantiate(&self, params: Vec<Expression>, type_: Option<Type>, source_span: Option<ParseSourceSpan>) -> Box<Expression> {
        Box::new(Expression::Instantiate(InstantiateExpr {
            class_expr: Box::new(self.clone()),
            args: params,
            type_,
            source_span,
        }))
    }

    pub fn conditional(&self, true_case: Box<Expression>, false_case: Option<Box<Expression>>, source_span: Option<ParseSourceSpan>) -> Box<Expression> {
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

pub fn fn_expr(params: Vec<FnParam>, statements: Vec<Statement>, type_: Option<Type>, name: Option<String>) -> Box<Expression> {
    Box::new(Expression::Fn(FunctionExpr {
        params,
        statements,
        type_,
        source_span: None,
        name,
    }))
}

pub fn arrow_fn(params: Vec<FnParam>, body: ArrowFunctionBody, type_: Option<Type>) -> Box<Expression> {
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
        }
    }
}

