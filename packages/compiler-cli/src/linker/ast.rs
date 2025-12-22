//! Linker AST Abstraction
//!
//! Defines the interface for interacting with different AST implementations.

use std::fmt::Debug;

/// Location range in the source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// 0-based character position of the range start.
    pub start_pos: usize,
    /// 0-based line index of the range start.
    pub start_line: usize,
    /// 0-based column position of the range start.
    pub start_col: usize,
    /// 0-based character position of the range end.
    pub end_pos: usize,
}

/// Helper trait for AST nodes that can be used by the linker.
pub trait AstNode: Debug + Clone {}

/// An abstraction for getting information from an AST while being agnostic to the underlying AST implementation.
pub trait AstHost<TExpression: AstNode> {
    /// Get the name of the symbol represented by the given expression node, or `null` if it is not a symbol.
    fn get_symbol_name(&self, node: &TExpression) -> Option<String>;

    /// Return `true` if the given expression is a string literal.
    fn is_string_literal(&self, node: &TExpression) -> bool;
    
    /// Parse the string value from the given expression, or throw if it is not a string literal.
    fn parse_string_literal(&self, str: &TExpression) -> Result<String, String>;

    /// Return `true` if the given expression is a numeric literal.
    fn is_numeric_literal(&self, node: &TExpression) -> bool;
    
    /// Parse the numeric value from the given expression, or throw if it is not a numeric literal.
    fn parse_numeric_literal(&self, num: &TExpression) -> Result<f64, String>;

    /// Return `true` if the given expression is a boolean literal.
    fn is_boolean_literal(&self, node: &TExpression) -> bool;
    
    /// Parse the boolean value from the given expression.
    fn parse_boolean_literal(&self, bool: &TExpression) -> Result<bool, String>;

    /// Returns `true` if the value corresponds to `null`.
    fn is_null(&self, node: &TExpression) -> bool;

    /// Return `true` if the given expression is an array literal.
    fn is_array_literal(&self, node: &TExpression) -> bool;
    
    /// Parse an array of expressions from the given expression.
    fn parse_array_literal(&self, array: &TExpression) -> Result<Vec<TExpression>, String>;

    /// Return `true` if the given expression is an object literal.
    fn is_object_literal(&self, node: &TExpression) -> bool;
    
    /// Parse the given expression into a map of object property names to property expressions.
    fn parse_object_literal(&self, obj: &TExpression) -> Result<std::collections::HashMap<String, TExpression>, String>;

    /// Return `true` if the given expression is a function.
    fn is_function_expression(&self, node: &TExpression) -> bool;
    
    /// Compute the "value" of a function expression by parsing its body for a single `return` statement.
    fn parse_return_value(&self, fn_node: &TExpression) -> Result<TExpression, String>;

    /// Returns the parameter expressions for the function.
    fn parse_parameters(&self, fn_node: &TExpression) -> Result<Vec<TExpression>, String>;

    /// Return true if the given expression is a call expression.
    fn is_call_expression(&self, node: &TExpression) -> bool;
    
    /// Returns the expression that is called.
    fn parse_callee(&self, call: &TExpression) -> Result<TExpression, String>;
    
    /// Returns the argument expressions for the provided call expression.
    fn parse_arguments(&self, call: &TExpression) -> Result<Vec<TExpression>, String>;

    /// Compute the location range of the expression in the source file.
    fn get_range(&self, node: &TExpression) -> Result<Range, String>;

    /// Print the source code representation of the derived node.
    fn print_node(&self, node: &TExpression) -> String;
}
