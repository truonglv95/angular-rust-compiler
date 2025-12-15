//! Constant Pool
//!
//! Corresponds to packages/compiler/src/constant_pool.ts
//!
//! ConstantPool tries to reuse literal factories when two or more literals are identical.
//! This optimizes the generated code by avoiding duplicate constant definitions.

use std::collections::HashMap;
use crate::output::output_ast as o;

const CONSTANT_PREFIX: &str = "_c";
const POOL_INCLUSION_LENGTH_THRESHOLD_FOR_STRINGS: usize = 50;

/// Fixup expression - placeholder that can be replaced later
#[derive(Debug, Clone)]
struct FixupExpression {
    original: o::Expression,
    resolved: o::Expression,
    shared: bool,
}

impl FixupExpression {
    fn new(expr: o::Expression) -> Self {
        FixupExpression {
            original: expr.clone(),
            resolved: expr,
            shared: false,
        }
    }

    fn fixup(&mut self, expression: o::Expression) {
        self.resolved = expression;
        self.shared = true;
    }
}

pub trait SharedConstantDefinition {
    fn key_of(&self, expr: &o::Expression) -> String;
    fn to_shared_constant_declaration(&self, name: String, expr: o::Expression) -> o::Statement;
}

/// Generic key function (for expression deduplication)
pub struct GenericKeyFn;

impl GenericKeyFn {
    pub const INSTANCE: GenericKeyFn = GenericKeyFn;

    /// Generate key for an expression
    pub fn key_of(&self, expr: &o::Expression) -> String {
        // Simplified key generation using debug formatting for now
        // TODO: Implement proper key visitor
        format!("{:?}", expr) 
    }
}

pub struct ConstantPool {
    pub statements: Vec<o::Statement>,
    literals: HashMap<String, FixupExpression>,
    literal_factories: HashMap<String, o::Expression>,
    shared_constants: HashMap<String, o::Expression>,
    claimed_names: HashMap<String, u32>,
    next_name_index: u32,
    pub is_closure_compiler_enabled: bool,
}

impl ConstantPool {
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

    pub fn get_const_literal(&mut self, literal: o::Expression, force_shared: bool) -> o::Expression {
        if self.is_simple_literal(&literal) {
            return literal;
        }

        let key = self.key_of_expression(&literal);

        let needs_sharing = if let Some(fixup) = self.literals.get(&key) {
            !fixup.shared || force_shared
        } else {
            false
        };

        if needs_sharing {
            let name = self.fresh_name();
            let var_expr = o::variable(name.clone());

            let stmt = o::Statement::DeclareVar(o::DeclareVarStmt {
                name,
                value: Some(Box::new(literal.clone())),
                type_: None,
                modifiers: o::StmtModifier::Final,
                source_span: None,
            });
            self.statements.push(stmt);

             if let Some(fixup) = self.literals.get_mut(&key) {
                fixup.fixup(*var_expr.clone()); // Dereference the Box
            }
            return *var_expr; // Dereference to return Expression
        }

        if let Some(fixup) = self.literals.get(&key) {
            return fixup.resolved.clone();
        }

        let mut fixup = FixupExpression::new(literal.clone());

        if force_shared {
            let name = self.fresh_name();
             let var_expr = o::variable(name.clone());

            let stmt = o::Statement::DeclareVar(o::DeclareVarStmt {
                name,
                value: Some(Box::new(literal)),
                type_: None, 
                modifiers: o::StmtModifier::Final,
                source_span: None,
            });
            self.statements.push(stmt);

            fixup.fixup(*var_expr.clone());
            self.literals.insert(key, fixup);
            return *var_expr;
        }

        let result = fixup.resolved.clone();
        self.literals.insert(key, fixup);
        result
    }

    pub fn get_shared_constant(&mut self, definition: Box<dyn SharedConstantDefinition>, initial_value: o::Expression) -> o::Expression {
        let key = definition.key_of(&initial_value);
        if let Some(existing) = self.shared_constants.get(&key) {
            return existing.clone();
        }
        
        let id = self.fresh_name();
        let stmt = definition.to_shared_constant_declaration(id.clone(), initial_value);
        self.statements.push(stmt);
        
        let var_expr = o::variable(id);
        self.shared_constants.insert(key, *var_expr.clone());
        *var_expr
    }

    /// Get shared function reference from pool.
    /// This method checks if the function expression is already declared, and if so returns a reference to it.
    /// Otherwise, it declares the function and returns a reference.
    pub fn get_shared_function_reference(
        &mut self,
        fn_expr: o::Expression,
        prefix: String,
        use_unique_name: bool,
    ) -> o::Expression {
        use crate::output::output_ast::ExpressionTrait;
        
        // Check if function is already declared
        let is_arrow = matches!(fn_expr, o::Expression::ArrowFn(_));
        
        for stmt in &self.statements {
            match stmt {
                o::Statement::DeclareVar(var_stmt) => {
                    if is_arrow {
                        if let Some(ref value) = var_stmt.value {
                            if value.as_ref().is_equivalent(&fn_expr) {
                                return *o::variable(var_stmt.name.clone());
                            }
                        }
                    }
                }
                o::Statement::DeclareFn(func_stmt) => {
                    if !is_arrow {
                        if let o::Expression::Fn(func_expr) = &fn_expr {
                            // Compare function expressions - simplified check
                            // In full implementation, we'd need to compare function bodies, params, etc.
                            if func_expr.name == Some(func_stmt.name.clone()) {
                                return *o::variable(func_stmt.name.clone());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Function not found, declare it
        let name = if use_unique_name {
            self.unique_name(prefix, false)
        } else {
            // If name already exists and use_unique_name is false, still make it unique
            if self.claimed_names.contains_key(&prefix) {
                self.unique_name(prefix, false)
            } else {
                self.claimed_names.insert(prefix.clone(), 0);
                prefix
            }
        };
        
        let stmt = match fn_expr {
            o::Expression::Fn(func_expr) => {
                // Convert FunctionExpr to DeclareFunctionStmt
                let mut func_expr_clone = func_expr.clone();
                func_expr_clone.name = Some(name.clone());
                o::Statement::DeclareFn(o::DeclareFunctionStmt {
                    name: name.clone(),
                    params: func_expr_clone.params,
                    statements: func_expr_clone.statements,
                    type_: func_expr_clone.type_,
                    modifiers: o::StmtModifier::Final,
                    source_span: func_expr_clone.source_span,
                })
            }
            _ => {
                o::Statement::DeclareVar(o::DeclareVarStmt {
                    name: name.clone(),
                    value: Some(Box::new(fn_expr)),
                    type_: None,
                    modifiers: o::StmtModifier::Final,
                    source_span: None,
                })
            }
        };
        
        self.statements.push(stmt);
        *o::variable(name)
    }

    pub fn get_literal_factory(&mut self, literal: o::Expression) -> LiteralFactory {
        let key = self.key_of_expression(&literal);

        if let Some(existing) = self.literal_factories.get(&key) {
            return LiteralFactory {
                literal_factory: existing.clone(),
                literal_factory_arguments: vec![],
            };
        }

        let factory_name = self.fresh_name();
        let factory_expr = o::variable(factory_name.clone());

        // TODO: Create actual factory function logic if needed, for now similar to placeholder
        self.literal_factories.insert(key, *factory_expr.clone());

        LiteralFactory {
            literal_factory: *factory_expr,
            literal_factory_arguments: vec![],
        }
    }

    pub fn unique_name(&mut self, preferred_name: String, _always_include_suffix: bool) -> String {
        // Rust specific: TS has `alwaysIncludeSuffix` (boolean).
        // I'll ignore it for now or assume false implies check existing.
        
        if !self.claimed_names.contains_key(&preferred_name) {
            self.claimed_names.insert(preferred_name.clone(), 0);
            return preferred_name;
        }

        let count = self.claimed_names.get_mut(&preferred_name).unwrap();
        *count += 1;
        let unique = format!("{}_{}", preferred_name, count);
        self.claimed_names.insert(unique.clone(), 0);
        return unique;
    }

    fn fresh_name(&mut self) -> String {
        let name = format!("{}{}", CONSTANT_PREFIX, self.next_name_index);
        self.next_name_index += 1;
        name
    }

    fn is_simple_literal(&self, expr: &o::Expression) -> bool {
        match expr {
            o::Expression::Literal(lit_expr) => {
                 match &lit_expr.value {
                     o::LiteralValue::String(s) => s.len() < POOL_INCLUSION_LENGTH_THRESHOLD_FOR_STRINGS,
                     _ => true
                 }
            },
            _ => false,
        }
    }

    fn key_of_expression(&self, expr: &o::Expression) -> String {
        format!("{:?}", expr) // Placeholder
    }
}

pub struct LiteralFactory {
    pub literal_factory: o::Expression,
    pub literal_factory_arguments: Vec<o::Expression>,
}

impl Default for ConstantPool {
    fn default() -> Self {
        Self::new(false)
    }
}
