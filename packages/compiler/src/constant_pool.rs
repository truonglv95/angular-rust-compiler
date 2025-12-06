//! Constant Pool
//!
//! Corresponds to packages/compiler/src/constant_pool.ts (357 lines)
//!
//! ConstantPool tries to reuse literal factories when two or more literals are identical.
//! This optimizes the generated code by avoiding duplicate constant definitions.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

const CONSTANT_PREFIX: &str = "_c";
const POOL_INCLUSION_LENGTH_THRESHOLD_FOR_STRINGS: usize = 50;

/// Expression type (simplified for now)
/// Will integrate with output/output_ast.rs later
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExpressionKind {
    Literal,
    Variable,
    FunctionCall,
    Array,
    Map,
    Other,
}

/// Statement type (simplified)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Statement {
    pub kind: StatementKind,
    pub name: Option<String>,
    pub value: Option<Expression>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StatementKind {
    DeclareVar,
    Expression,
    Return,
}

/// Fixup expression - placeholder that can be replaced later
///
/// This allows the constant pool to change an expression from a direct reference
/// to a constant to a shared constant.
#[derive(Debug, Clone)]
struct FixupExpression {
    original: Expression,
    resolved: Expression,
    shared: bool,
}

impl FixupExpression {
    fn new(expr: Expression) -> Self {
        FixupExpression {
            original: expr.clone(),
            resolved: expr,
            shared: false,
        }
    }

    fn fixup(&mut self, expression: Expression) {
        self.resolved = expression;
        self.shared = true;
    }
}

/// Constant Pool - manages constant values during compilation
///
/// TypeScript equivalent:
/// ```typescript
/// export class ConstantPool {
///   statements: o.Statement[] = [];
///   private literals = new Map<string, FixupExpression>();
///   private literalFactories = new Map<string, o.Expression>();
///   private sharedConstants = new Map<string, o.Expression>();
/// }
/// ```
pub struct ConstantPool {
    /// Generated statements (const declarations)
    pub statements: Vec<Statement>,

    /// Map of literal values to their fixup expressions
    literals: HashMap<String, FixupExpression>,

    /// Map of literal factories (for arrays/objects)
    literal_factories: HashMap<String, Expression>,

    /// Map of shared constants
    shared_constants: HashMap<String, Expression>,

    /// Claimed names to avoid collisions
    claimed_names: HashMap<String, u32>,

    /// Next name index
    next_name_index: u32,

    /// Whether Closure Compiler is enabled (affects optimization strategy)
    is_closure_compiler_enabled: bool,
}

impl ConstantPool {
    /// Create new constant pool
    pub fn new(is_closure_compiler_enabled: bool) -> Self {
        ConstantPool {
            statements: Vec::new(),
            literals: HashMap::new(),
            literal_factories: HashMap::new(),
            shared_constants: HashMap::new(),
            claimed_names: HashMap::new(),
            next_name_index: 0,
            is_closure_compiler_enabled,
        }
    }

    /// Get or create constant literal
    ///
    /// If the literal already exists, returns reference to existing constant.
    /// Otherwise, creates new constant and adds to pool.
    pub fn get_const_literal(&mut self, literal: Expression, force_shared: bool) -> Expression {
        // Don't pool simple/short literals
        if self.is_simple_literal(&literal) {
            return literal;
        }

        // Generate key for this literal
        let key = self.key_of_expression(&literal);

        // Check if already exists and needs sharing
        let needs_sharing = if let Some(fixup) = self.literals.get(&key) {
            !fixup.shared || force_shared
        } else {
            false
        };

        if needs_sharing {
            // Need to convert to shared constant
            let name = self.fresh_name();
            let var_expr = self.create_variable(name.clone());

            let stmt = Statement {
                kind: StatementKind::DeclareVar,
                name: Some(name),
                value: Some(literal.clone()),
            };
            self.statements.push(stmt);

            // Update fixup
            if let Some(fixup) = self.literals.get_mut(&key) {
                fixup.fixup(var_expr.clone());
            }
            return var_expr;
        }

        // Return existing if found
        if let Some(fixup) = self.literals.get(&key) {
            return fixup.resolved.clone();
        }

        // Create new fixup
        let mut fixup = FixupExpression::new(literal.clone());

        if force_shared {
            let name = self.fresh_name();
            let var_expr = self.create_variable(name.clone());

            let stmt = Statement {
                kind: StatementKind::DeclareVar,
                name: Some(name),
                value: Some(literal),
            };
            self.statements.push(stmt);

            fixup.fixup(var_expr.clone());
            self.literals.insert(key, fixup);
            return var_expr;
        }

        let result = fixup.resolved.clone();
        self.literals.insert(key, fixup);
        result
    }

    /// Get shared constant with custom key function
    pub fn get_shared_constant(&mut self, key: String, expr: Expression) -> Expression {
        if let Some(existing) = self.shared_constants.get(&key) {
            return existing.clone();
        }

        let id = self.fresh_name();
        let var_expr = self.create_variable(id.clone());

        let stmt = Statement {
            kind: StatementKind::DeclareVar,
            name: Some(id),
            value: Some(expr),
        };
        self.statements.push(stmt);

        self.shared_constants.insert(key.clone(), var_expr.clone());
        var_expr
    }

    /// Get literal factory for arrays or objects
    ///
    /// Creates a pure function that builds the literal with variable parts
    pub fn get_literal_factory(&mut self, literal: Expression) -> LiteralFactory {
        let key = self.key_of_expression(&literal);

        if let Some(existing) = self.literal_factories.get(&key) {
            return LiteralFactory {
                literal_factory: existing.clone(),
                literal_factory_arguments: vec![], // TODO: Extract arguments
            };
        }

        // Create factory function
        let factory_name = self.fresh_name();
        let factory_expr = self.create_variable(factory_name.clone());

        // TODO: Create actual factory function
        // For now, just store the expression
        self.literal_factories.insert(key, factory_expr.clone());

        LiteralFactory {
            literal_factory: factory_expr,
            literal_factory_arguments: vec![],
        }
    }

    /// Generate unique name
    ///
    /// If preferredName is given and available, use it.
    /// Otherwise, generate _c0, _c1, etc.
    pub fn unique_name(&mut self, preferred_name: Option<&str>) -> String {
        if let Some(name) = preferred_name {
            if !self.claimed_names.contains_key(name) {
                self.claimed_names.insert(name.to_string(), 0);
                return name.to_string();
            }

            // Name already claimed, try with suffix
            let count = self.claimed_names.get_mut(name).unwrap();
            *count += 1;
            let unique = format!("{}_{}", name, count);
            self.claimed_names.insert(unique.clone(), 0);
            return unique;
        }

        self.fresh_name()
    }

    /// Generate fresh constant name (_c0, _c1, etc.)
    fn fresh_name(&mut self) -> String {
        let name = format!("{}{}", CONSTANT_PREFIX, self.next_name_index);
        self.next_name_index += 1;
        name
    }

    /// Check if literal is simple (don't need to pool)
    fn is_simple_literal(&self, expr: &Expression) -> bool {
        match &expr.kind {
            ExpressionKind::Literal => {
                // Check if it's a long string
                if let Some(s) = expr.value.as_str() {
                    s.len() < POOL_INCLUSION_LENGTH_THRESHOLD_FOR_STRINGS
                } else {
                    true // Numbers, booleans are simple
                }
            }
            _ => false,
        }
    }

    /// Generate key for expression (for deduplication)
    fn key_of_expression(&self, expr: &Expression) -> String {
        // Simple key generation
        // TODO: Implement full KeyVisitor like TypeScript
        format!("{:?}_{:?}", expr.kind, expr.value)
    }

    /// Create variable expression
    fn create_variable(&self, name: String) -> Expression {
        Expression {
            kind: ExpressionKind::Variable,
            value: serde_json::json!(name),
        }
    }
}

impl Default for ConstantPool {
    fn default() -> Self {
        Self::new(false)
    }
}

/// Result of getLiteralFactory()
pub struct LiteralFactory {
    pub literal_factory: Expression,
    pub literal_factory_arguments: Vec<Expression>,
}

/// Shared constant definition trait
pub trait SharedConstantDefinition {
    fn key_of(&self, expr: &Expression) -> String;
    fn to_shared_constant_declaration(&self, name: String, expr: Expression) -> Statement;
}

/// Generic key function (for expression deduplication)
pub struct GenericKeyFn;

impl GenericKeyFn {
    pub const INSTANCE: GenericKeyFn = GenericKeyFn;

    /// Generate key for an expression
    pub fn key_of(&self, expr: &Expression) -> String {
        // Simplified key generation
        // TODO: Implement full visitor pattern like TypeScript
        match &expr.kind {
            ExpressionKind::Literal => {
                format!("literal:{}", expr.value)
            }
            ExpressionKind::Variable => {
                format!("var:{}", expr.value)
            }
            ExpressionKind::Array => {
                format!("array:{}", expr.value)
            }
            ExpressionKind::Map => {
                format!("map:{}", expr.value)
            }
            _ => format!("{:?}", expr.value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_pool_creation() {
        let pool = ConstantPool::new(false);
        assert_eq!(pool.statements.len(), 0);
        assert!(!pool.is_closure_compiler_enabled);
    }

    #[test]
    fn test_fresh_name_generation() {
        let mut pool = ConstantPool::new(false);

        let name1 = pool.fresh_name();
        let name2 = pool.fresh_name();
        let name3 = pool.fresh_name();

        assert_eq!(name1, "_c0");
        assert_eq!(name2, "_c1");
        assert_eq!(name3, "_c2");
    }

    #[test]
    fn test_unique_name_with_preference() {
        let mut pool = ConstantPool::new(false);

        // First use of preferred name
        let name1 = pool.unique_name(Some("myConst"));
        assert_eq!(name1, "myConst");

        // Second use of same name - should get suffix
        let name2 = pool.unique_name(Some("myConst"));
        assert_eq!(name2, "myConst_1");

        // Third use
        let name3 = pool.unique_name(Some("myConst"));
        assert_eq!(name3, "myConst_2");
    }

    #[test]
    fn test_simple_literal_detection() {
        let pool = ConstantPool::new(false);

        // Short string - is simple
        let short_str = Expression {
            kind: ExpressionKind::Literal,
            value: serde_json::json!("hello"),
        };
        assert!(pool.is_simple_literal(&short_str));

        // Long string - not simple (should be pooled)
        let long_str = Expression {
            kind: ExpressionKind::Literal,
            value: serde_json::json!("a".repeat(100)),
        };
        assert!(!pool.is_simple_literal(&long_str));

        // Number - is simple
        let num = Expression {
            kind: ExpressionKind::Literal,
            value: serde_json::json!(42),
        };
        assert!(pool.is_simple_literal(&num));
    }

    #[test]
    fn test_get_const_literal_simple() {
        let mut pool = ConstantPool::new(false);

        // Simple literal should be returned as-is (not pooled)
        let expr = Expression {
            kind: ExpressionKind::Literal,
            value: serde_json::json!("hi"),
        };

        let result = pool.get_const_literal(expr.clone(), false);

        // Should return same expression
        assert_eq!(result.value, expr.value);

        // Should not add to statements
        assert_eq!(pool.statements.len(), 0);
    }

    #[test]
    fn test_get_const_literal_with_sharing() {
        let mut pool = ConstantPool::new(false);

        // Long string should be pooled
        let expr = Expression {
            kind: ExpressionKind::Literal,
            value: serde_json::json!("a".repeat(100)),
        };

        let result = pool.get_const_literal(expr.clone(), true);

        // Should create a variable reference
        assert!(matches!(result.kind, ExpressionKind::Variable));

        // Should add declaration statement
        assert_eq!(pool.statements.len(), 1);
        assert_eq!(pool.statements[0].kind, StatementKind::DeclareVar);
    }

    #[test]
    fn test_shared_constant_reuse() {
        let mut pool = ConstantPool::new(false);

        let expr = Expression {
            kind: ExpressionKind::Literal,
            value: serde_json::json!(42),
        };

        // First call creates constant
        let result1 = pool.get_shared_constant("myKey".to_string(), expr.clone());
        assert_eq!(pool.statements.len(), 1);

        // Second call with same key should reuse
        let result2 = pool.get_shared_constant("myKey".to_string(), expr.clone());
        assert_eq!(pool.statements.len(), 1); // No new statement

        // Results should point to same variable
        assert_eq!(result1.value, result2.value);
    }

    #[test]
    fn test_key_generation() {
        let key_fn = GenericKeyFn::INSTANCE;

        let expr1 = Expression {
            kind: ExpressionKind::Literal,
            value: serde_json::json!("test"),
        };

        let expr2 = Expression {
            kind: ExpressionKind::Literal,
            value: serde_json::json!("test"),
        };

        let expr3 = Expression {
            kind: ExpressionKind::Literal,
            value: serde_json::json!("different"),
        };

        // Same literals should have same key
        assert_eq!(key_fn.key_of(&expr1), key_fn.key_of(&expr2));

        // Different literals should have different keys
        assert_ne!(key_fn.key_of(&expr1), key_fn.key_of(&expr3));
    }
}

/// Helper to check if string literal is long
fn is_long_string_literal(expr: &Expression) -> bool {
    if let ExpressionKind::Literal = expr.kind {
        if let Some(s) = expr.value.as_str() {
            return s.len() >= POOL_INCLUSION_LENGTH_THRESHOLD_FOR_STRINGS;
        }
    }
    false
}
