// Dependency Injection Utilities
//
// Functions for analyzing constructor dependencies and injection tokens.


/// Represents a constructor dependency metadata.
#[derive(Debug, Clone)]
pub struct R3DependencyMetadata {
    /// The token expression for the dependency.
    pub token: String,
    /// Whether the dependency is optional (@Optional).
    pub optional: bool,
    /// Whether to resolve from host (@Host).
    pub host: bool,
    /// Whether to use self (@Self).
    pub self_: bool,
    /// Whether to skip self (@SkipSelf).
    pub skip_self: bool,
    /// How to resolve this dependency.
    pub resolved: R3ResolvedDependencyType,
}

impl R3DependencyMetadata {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            optional: false,
            host: false,
            self_: false,
            skip_self: false,
            resolved: R3ResolvedDependencyType::Token,
        }
    }
    
    pub fn with_optional(mut self) -> Self {
        self.optional = true;
        self
    }
    
    pub fn with_host(mut self) -> Self {
        self.host = true;
        self
    }
    
    pub fn with_self(mut self) -> Self {
        self.self_ = true;
        self
    }
    
    pub fn with_skip_self(mut self) -> Self {
        self.skip_self = true;
        self
    }
}

/// How a dependency is resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum R3ResolvedDependencyType {
    /// A normal token.
    Token,
    /// The dependency is resolved to an attribute.
    Attribute,
    /// Injecting ChangeDetectorRef.
    ChangeDetectorRef,
    /// Invalid dependency that couldn't be resolved.
    Invalid,
}

/// Result of analyzing constructor dependencies.
#[derive(Debug, Clone)]
pub enum ConstructorDeps {
    /// Valid dependencies.
    Valid(Vec<R3DependencyMetadata>),
    /// Invalid - has errors.
    Invalid(Vec<ConstructorDepError>),
}

impl ConstructorDeps {
    pub fn is_valid(&self) -> bool {
        matches!(self, ConstructorDeps::Valid(_))
    }
    
    pub fn into_deps(self) -> Option<Vec<R3DependencyMetadata>> {
        match self {
            ConstructorDeps::Valid(deps) => Some(deps),
            ConstructorDeps::Invalid(_) => None,
        }
    }
}

/// Error when analyzing a constructor parameter.
#[derive(Debug, Clone)]
pub struct ConstructorDepError {
    /// Parameter index.
    pub index: usize,
    /// Parameter name, if known.
    pub name: Option<String>,
    /// The reason for the error.
    pub reason: UnavailableValueKind,
}

/// Reason a value is unavailable.
#[derive(Debug, Clone)]
pub enum UnavailableValueKind {
    /// Unknown reference.
    UnknownReference,
    /// Missing type.
    MissingType,
    /// Type is not a reference.
    TypeOnlyImport,
    /// Namespace import.
    NamespaceImport,
    /// Requires type only emit but not supported.
    RequiresTypeOnlyEmit,
    /// Unsupported reference.
    Unsupported,
}

impl UnavailableValueKind {
    pub fn message(&self) -> &'static str {
        match self {
            UnavailableValueKind::UnknownReference => "Unknown reference",
            UnavailableValueKind::MissingType => "Missing type annotation",
            UnavailableValueKind::TypeOnlyImport => "Type-only import cannot be used as a value",
            UnavailableValueKind::NamespaceImport => "Namespace imports cannot be used directly",
            UnavailableValueKind::RequiresTypeOnlyEmit => "Requires type-only emit",
            UnavailableValueKind::Unsupported => "Unsupported value reference",
        }
    }
}

/// Decorator names that affect dependency resolution.
const OPTIONAL_DECORATOR: &str = "Optional";
const SELF_DECORATOR: &str = "Self";
const SKIP_SELF_DECORATOR: &str = "SkipSelf";
const HOST_DECORATOR: &str = "Host";
const INJECT_DECORATOR: &str = "Inject";
const ATTRIBUTE_DECORATOR: &str = "Attribute";

/// Get constructor dependencies for a class.
pub fn get_constructor_dependencies(
    constructor_params: &[CtorParameter],
    is_core: bool,
) -> Option<ConstructorDeps> {
    if constructor_params.is_empty() {
        return Some(ConstructorDeps::Valid(Vec::new()));
    }
    
    let mut deps = Vec::new();
    let mut errors = Vec::new();
    
    for (index, param) in constructor_params.iter().enumerate() {
        match analyze_ctor_parameter(param, is_core) {
            Ok(dep) => deps.push(dep),
            Err(reason) => {
                errors.push(ConstructorDepError {
                    index,
                    name: param.name.clone(),
                    reason,
                });
            }
        }
    }
    
    if errors.is_empty() {
        Some(ConstructorDeps::Valid(deps))
    } else {
        Some(ConstructorDeps::Invalid(errors))
    }
}

/// Constructor parameter for analysis.
#[derive(Debug, Clone)]
pub struct CtorParameter {
    /// Parameter name.
    pub name: Option<String>,
    /// Type token, if available.
    pub type_token: Option<String>,
    /// Decorators on the parameter.
    pub decorators: Vec<ParameterDecorator>,
}

/// Decorator on a parameter.
#[derive(Debug, Clone)]
pub struct ParameterDecorator {
    /// Decorator name.
    pub name: String,
    /// Arguments, if any.
    pub args: Vec<String>,
    /// Module where the decorator comes from.
    pub from_module: Option<String>,
}

impl ParameterDecorator {
    /// Check if this decorator is from Angular core.
    pub fn is_angular_core(&self) -> bool {
        self.from_module.as_deref() == Some("@angular/core")
    }
}

fn analyze_ctor_parameter(
    param: &CtorParameter,
    is_core: bool,
) -> Result<R3DependencyMetadata, UnavailableValueKind> {
    let mut token = None;
    let mut optional = false;
    let mut host = false;
    let mut self_ = false;
    let mut skip_self = false;
    let mut resolved = R3ResolvedDependencyType::Token;
    
    // Process decorators
    for dec in &param.decorators {
        let is_angular = is_core || dec.is_angular_core();
        if !is_angular {
            continue;
        }
        
        match dec.name.as_str() {
            OPTIONAL_DECORATOR => optional = true,
            HOST_DECORATOR => host = true,
            SELF_DECORATOR => self_ = true,
            SKIP_SELF_DECORATOR => skip_self = true,
            INJECT_DECORATOR => {
                if let Some(arg) = dec.args.first() {
                    token = Some(arg.clone());
                }
            }
            ATTRIBUTE_DECORATOR => {
                if let Some(arg) = dec.args.first() {
                    token = Some(arg.clone());
                    resolved = R3ResolvedDependencyType::Attribute;
                }
            }
            _ => {}
        }
    }
    
    // Use type token if no explicit @Inject
    let final_token = token.or(param.type_token.clone())
        .ok_or(UnavailableValueKind::MissingType)?;
    
    Ok(R3DependencyMetadata {
        token: final_token,
        optional,
        host,
        self_,
        skip_self,
        resolved,
    })
}

/// Unwrap constructor dependencies into a simple result.
pub fn unwrap_constructor_dependencies(
    deps: Option<ConstructorDeps>,
) -> Option<Vec<R3DependencyMetadata>> {
    match deps {
        Some(ConstructorDeps::Valid(d)) => Some(d),
        _ => None,
    }
}

/// Get only valid constructor dependencies.
pub fn get_valid_constructor_dependencies(
    constructor_params: &[CtorParameter],
    is_core: bool,
) -> Option<Vec<R3DependencyMetadata>> {
    unwrap_constructor_dependencies(get_constructor_dependencies(constructor_params, is_core))
}
