
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableDeclarationType {
    Const,
    Let,
    Var,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOperator {
    Plus,
    Minus,
    Not,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOperator {
    And,
    Greater,
    GreaterEquals,
    BitAnd,
    BitOr,
    Divide,
    Equals,
    IdentityEquals,
    Less,
    LessEquals,
    Minus,
    Modulo,
    Multiply,
    Power,
    NotEquals,
    IdentityNotEquals,
    Or,
    Plus,
    NullishCoalesce,
    In,
    Assign,
    PlusAssign,
    MinusAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    PowerAssign,
    AndAssign,
    OrAssign,
    NullishCoalesceAssign,
}

pub struct SourceMapLocation {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

pub struct SourceMapRange {
    pub url: String,
    pub content: String,
    pub start: SourceMapLocation,
    pub end: SourceMapLocation,
}

pub struct ObjectLiteralProperty<TExpression> {
    pub property_name: String,
    pub value: TExpression,
    pub quoted: bool,
}

pub struct TemplateLiteral<TExpression> {
    pub elements: Vec<TemplateElement>,
    pub expressions: Vec<TExpression>,
}

pub struct TemplateElement {
    pub raw: String,
    pub cooked: String,
    pub range: Option<SourceMapRange>,
}

pub trait LeadingComment {
    fn to_string(&self) -> String;
    fn multiline(&self) -> bool;
    fn trailing_newline(&self) -> bool;
}

pub trait AstFactory {
    type Statement;
    type Expression;

    fn attach_comments(&self, statement: &mut Self::Statement, leading_comments: &[Box<dyn LeadingComment>]);
    
    // For expression comments, we might need a separate method or ensure Expression can handle it.
    // The TS interface allows TStatement | TExpression.
    // In Rust we might differentiate or require a common trait.
    // Let's rely on the implementation to handle "statement" as a generic term or handle both.
    // For now, I'll add attach_comments_to_expression
    fn attach_comments_to_expression(&self, expression: &mut Self::Expression, leading_comments: &[Box<dyn LeadingComment>]);

    fn create_array_literal(&self, elements: Vec<Self::Expression>) -> Self::Expression;

    fn create_assignment(&self, target: Self::Expression, operator: BinaryOperator, value: Self::Expression) -> Self::Expression;

    fn create_binary_expression(&self, left_operand: Self::Expression, operator: BinaryOperator, right_operand: Self::Expression) -> Self::Expression;

    fn create_block(&self, body: Vec<Self::Statement>) -> Self::Statement;

    fn create_call_expression(&self, callee: Self::Expression, args: Vec<Self::Expression>, pure: bool) -> Self::Expression;

    fn create_conditional(&self, condition: Self::Expression, then_expression: Self::Expression, else_expression: Self::Expression) -> Self::Expression;

    fn create_element_access(&self, expression: Self::Expression, element: Self::Expression) -> Self::Expression;

    fn create_expression_statement(&self, expression: Self::Expression) -> Self::Statement;

    fn create_function_declaration(&self, function_name: &str, parameters: Vec<String>, body: Self::Statement) -> Self::Statement;

    fn create_function_expression(&self, function_name: Option<&str>, parameters: Vec<String>, body: Self::Statement) -> Self::Expression;

    // body can be expression or statement
    fn create_arrow_function_expression(&self, parameters: Vec<String>, body: ArrowFunctionBody<Self::Statement, Self::Expression>) -> Self::Expression;

    fn create_dynamic_import(&self, url: Result<&str, Self::Expression>) -> Self::Expression;

    fn create_identifier(&self, name: &str) -> Self::Expression;

    fn create_if_statement(&self, condition: Self::Expression, then_statement: Self::Statement, else_statement: Option<Self::Statement>) -> Self::Statement;

    fn create_literal(&self, value: LiteralValue) -> Self::Expression;

    fn create_new_expression(&self, expression: Self::Expression, args: Vec<Self::Expression>) -> Self::Expression;

    fn create_object_literal(&self, properties: Vec<ObjectLiteralProperty<Self::Expression>>) -> Self::Expression;

    fn create_parenthesized_expression(&self, expression: Self::Expression) -> Self::Expression;

    fn create_property_access(&self, expression: Self::Expression, property_name: &str) -> Self::Expression;

    fn create_return_statement(&self, expression: Option<Self::Expression>) -> Self::Statement;

    fn create_tagged_template(&self, tag: Self::Expression, template: TemplateLiteral<Self::Expression>) -> Self::Expression;

    fn create_template_literal(&self, template: TemplateLiteral<Self::Expression>) -> Self::Expression;

    fn create_throw_statement(&self, expression: Self::Expression) -> Self::Statement;

    fn create_type_of_expression(&self, expression: Self::Expression) -> Self::Expression;

    fn create_void_expression(&self, expression: Self::Expression) -> Self::Expression;

    fn create_unary_expression(&self, operator: UnaryOperator, operand: Self::Expression) -> Self::Expression;

    fn create_variable_declaration(&self, variable_name: &str, initializer: Option<Self::Expression>, type_: VariableDeclarationType) -> Self::Statement;

    fn create_regular_expression_literal(&self, body: &str, flags: Option<&str>) -> Self::Expression;

    fn set_source_map_range_for_stmt(&self, node: Self::Statement, source_map_range: Option<SourceMapRange>) -> Self::Statement;
    fn set_source_map_range_for_expr(&self, node: Self::Expression, source_map_range: Option<SourceMapRange>) -> Self::Expression;
}

pub enum ArrowFunctionBody<S, E> {
    Stmt(S),
    Expr(E),
}

pub enum LiteralValue<'a> {
    String(&'a str),
    Number(f64),
    Boolean(bool),
    Null,
    Undefined,
}
