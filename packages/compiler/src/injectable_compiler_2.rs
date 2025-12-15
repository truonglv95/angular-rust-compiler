//! Injectable Compiler
//!
//! Corresponds to packages/compiler/src/injectable_compiler_2.ts (191 lines)
//!
//! Compiles @Injectable decorators to generate factory functions
//! for dependency injection.

use serde::{Deserialize, Serialize};

/// Expression reference (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expression {
    pub value: serde_json::Value,
}

/// R3 Reference (to a type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3Reference {
    pub value: Expression,
    #[serde(rename = "type")]
    pub type_ref: Expression,
}

/// Maybe forward ref expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaybeForwardRefExpression {
    pub expression: Expression,
    pub forward_ref: bool,
}

/// Dependency metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3DependencyMetadata {
    pub token: Expression,
    pub attribute: Option<String>,
    pub host: bool,
    pub optional: bool,
    #[serde(rename = "self")]
    pub self_dep: bool,
    pub skip_self: bool,
}

/// Injectable metadata for compilation
///
/// TypeScript equivalent:
/// ```typescript
/// export interface R3InjectableMetadata {
///   name: string;
///   type: R3Reference;
///   typeArgumentCount: number;
///   providedIn: MaybeForwardRefExpression;
///   useClass?: MaybeForwardRefExpression;
///   useFactory?: o.Expression;
///   useExisting?: MaybeForwardRefExpression;
///   useValue?: MaybeForwardRefExpression;
///   deps?: R3DependencyMetadata[];
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3InjectableMetadata {
    /// Injectable class name
    pub name: String,

    /// Type reference
    #[serde(rename = "type")]
    pub type_ref: R3Reference,

    /// Number of type arguments (for generics)
    pub type_argument_count: u32,

    /// Where this injectable is provided (root, platform, any, or a specific module)
    pub provided_in: MaybeForwardRefExpression,

    /// Alternative class to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_class: Option<MaybeForwardRefExpression>,

    /// Factory function to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_factory: Option<Expression>,

    /// Existing injectable to reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_existing: Option<MaybeForwardRefExpression>,

    /// Direct value to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_value: Option<MaybeForwardRefExpression>,

    /// Dependencies for factory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deps: Option<Vec<R3DependencyMetadata>>,
}

/// Compiled expression result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R3CompiledExpression {
    /// Compiled expression
    pub expression: Expression,

    /// Generated type
    #[serde(rename = "type")]
    pub type_expr: Expression,

    /// Additional statements
    pub statements: Vec<Statement>,
}

/// Statement (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    pub kind: String,
    pub content: String,
}

/// Compile injectable to factory definition
///
/// TypeScript equivalent:
/// ```typescript
/// export function compileInjectable(
///   meta: R3InjectableMetadata,
///   resolveForwardRefs: boolean
/// ): R3CompiledExpression
/// ```
pub fn compile_injectable(
    meta: R3InjectableMetadata,
    resolve_forward_refs: bool,
) -> Result<R3CompiledExpression, String> {
    // Determine which factory strategy to use
    let result = if let Some(use_class) = meta.use_class.clone() {
        compile_injectable_use_class(meta, use_class, resolve_forward_refs)?
    } else if let Some(use_factory) = meta.use_factory.clone() {
        compile_injectable_use_factory(meta, use_factory)?
    } else if let Some(use_value) = meta.use_value.clone() {
        compile_injectable_use_value(meta, use_value)?
    } else if let Some(use_existing) = meta.use_existing.clone() {
        compile_injectable_use_existing(meta, use_existing)?
    } else {
        // Default: use the type's own factory
        compile_injectable_default(meta)?
    };

    Ok(result)
}

/// Compile injectable with useClass
fn compile_injectable_use_class(
    meta: R3InjectableMetadata,
    use_class: MaybeForwardRefExpression,
    _resolve_forward_refs: bool,
) -> Result<R3CompiledExpression, String> {
    // Check if useClass is the same as the type itself
    let use_class_on_self = is_expression_equivalent(&use_class.expression, &meta.type_ref.value);

    if use_class_on_self && meta.deps.is_none() {
        // useClass: Type where Type is self - just use default factory
        return compile_injectable_default(meta);
    }

    // Generate factory that instantiates useClass
    let expression = Expression {
        value: serde_json::json!({
            "kind": "factory",
            "target": "Injectable",
            "useClass": use_class.expression.value,
            "deps": meta.deps
        }),
    };

    Ok(R3CompiledExpression {
        expression,
        type_expr: create_injectable_type(&meta),
        statements: vec![],
    })
}

/// Compile injectable with useFactory
fn compile_injectable_use_factory(
    meta: R3InjectableMetadata,
    use_factory: Expression,
) -> Result<R3CompiledExpression, String> {
    let expression = Expression {
        value: serde_json::json!({
            "kind": "factory",
            "target": "Injectable",
            "useFactory": use_factory.value,
            "deps": meta.deps
        }),
    };

    Ok(R3CompiledExpression {
        expression,
        type_expr: create_injectable_type(&meta),
        statements: vec![],
    })
}

/// Compile injectable with useValue
fn compile_injectable_use_value(
    meta: R3InjectableMetadata,
    use_value: MaybeForwardRefExpression,
) -> Result<R3CompiledExpression, String> {
    let expression = Expression {
        value: serde_json::json!({
            "kind": "factory",
            "target": "Injectable",
            "useValue": use_value.expression.value
        }),
    };

    Ok(R3CompiledExpression {
        expression,
        type_expr: create_injectable_type(&meta),
        statements: vec![],
    })
}

/// Compile injectable with useExisting
fn compile_injectable_use_existing(
    meta: R3InjectableMetadata,
    use_existing: MaybeForwardRefExpression,
) -> Result<R3CompiledExpression, String> {
    // useExisting is an inject() call on the existing token
    let expression = Expression {
        value: serde_json::json!({
            "kind": "inject",
            "token": use_existing.expression.value
        }),
    };

    Ok(R3CompiledExpression {
        expression,
        type_expr: create_injectable_type(&meta),
        statements: vec![],
    })
}

/// Compile injectable with default factory
fn compile_injectable_default(
    meta: R3InjectableMetadata,
) -> Result<R3CompiledExpression, String> {
    let expression = Expression {
        value: serde_json::json!({
            "kind": "factory",
            "target": "Injectable",
            "type": meta.type_ref.value.value
        }),
    };

    Ok(R3CompiledExpression {
        expression,
        type_expr: create_injectable_type(&meta),
        statements: vec![],
    })
}

/// Create injectable type expression
///
/// TypeScript equivalent:
/// ```typescript
/// export function createInjectableType(meta: R3InjectableMetadata) {
///   return new o.ExpressionType(
///     o.importExpr(Identifiers.InjectableDeclaration, [
///       typeWithParameters(meta.type.type, meta.typeArgumentCount),
///     ]),
///   );
/// }
/// ```
pub fn create_injectable_type(meta: &R3InjectableMetadata) -> Expression {
    Expression {
        value: serde_json::json!({
            "kind": "InjectableDeclaration",
            "type": meta.type_ref.type_ref.value,
            "typeArguments": meta.type_argument_count
        }),
    }
}

/// Check if two expressions are equivalent
fn is_expression_equivalent(expr1: &Expression, expr2: &Expression) -> bool {
    // Simplified comparison - compare JSON values
    expr1.value == expr2.value
}

/// Helper to create factory function expression
fn _create_factory_function(use_type: &Expression) -> Expression {
    Expression {
        value: serde_json::json!({
            "kind": "arrowFunction",
            "params": ["__ngFactoryType__"],
            "body": {
                "kind": "callExpression",
                "callee": {
                    "kind": "propertyAccess",
                    "object": use_type.value,
                    "property": "ɵfac"
                },
                "arguments": ["__ngFactoryType__"]
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_injectable_default() {
        let meta = R3InjectableMetadata {
            name: "MyService".to_string(),
            type_ref: R3Reference {
                value: Expression {
                    value: serde_json::json!("MyService"),
                },
                type_ref: Expression {
                    value: serde_json::json!("MyService"),
                },
            },
            type_argument_count: 0,
            provided_in: MaybeForwardRefExpression {
                expression: Expression {
                    value: serde_json::json!("root"),
                },
                forward_ref: false,
            },
            use_class: None,
            use_factory: None,
            use_existing: None,
            use_value: None,
            deps: None,
        };

        let result = compile_injectable(meta, false);
        assert!(result.is_ok());

        let compiled = result.unwrap();
        assert_eq!(compiled.statements.len(), 0);
    }

    #[test]
    fn test_compile_injectable_use_value() {
        let meta = R3InjectableMetadata {
            name: "MY_TOKEN".to_string(),
            type_ref: R3Reference {
                value: Expression { value: serde_json::json!("MY_TOKEN") },
                type_ref: Expression { value: serde_json::json!("any") },
            },
            type_argument_count: 0,
            provided_in: MaybeForwardRefExpression {
                expression: Expression { value: serde_json::json!("root") },
                forward_ref: false,
            },
            use_class: None,
            use_factory: None,
            use_existing: None,
            use_value: Some(MaybeForwardRefExpression {
                expression: Expression { value: serde_json::json!(42) },
                forward_ref: false,
            }),
            deps: None,
        };

        let result = compile_injectable(meta, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_injectable_type() {
        let meta = R3InjectableMetadata {
            name: "TestService".to_string(),
            type_ref: R3Reference {
                value: Expression { value: serde_json::json!("TestService") },
                type_ref: Expression { value: serde_json::json!("TestService") },
            },
            type_argument_count: 2,
            provided_in: MaybeForwardRefExpression {
                expression: Expression { value: serde_json::json!("root") },
                forward_ref: false,
            },
            use_class: None,
            use_factory: None,
            use_existing: None,
            use_value: None,
            deps: None,
        };

        let type_expr = create_injectable_type(&meta);

        // Check that type contains InjectableDeclaration
        let json = type_expr.value.to_string();
        assert!(json.contains("InjectableDeclaration"));
        assert!(json.contains("2")); // typeArguments
    }

    #[test]
    fn test_expression_equivalence() {
        let expr1 = Expression { value: serde_json::json!("MyClass") };
        let expr2 = Expression { value: serde_json::json!("MyClass") };
        let expr3 = Expression { value: serde_json::json!("OtherClass") };

        assert!(is_expression_equivalent(&expr1, &expr2));
        assert!(!is_expression_equivalent(&expr1, &expr3));
    }
}
