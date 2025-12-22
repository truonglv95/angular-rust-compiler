// Metadata Extraction
//
// Functions for extracting Angular metadata from class declarations.

use super::util::Decorator;

/// Metadata for a class, used for setClassMetadata.
#[derive(Debug, Clone)]
pub struct R3ClassMetadata {
    /// The class type expression.
    pub type_: String,
    /// The decorators metadata.
    pub decorators: Vec<DecoratorMetadata>,
    /// Constructor parameters metadata.
    pub ctor_parameters: Option<Vec<CtorParameterMetadata>>,
    /// Property decorators.
    pub prop_decorators: Vec<PropDecoratorMetadata>,
}

/// Decorator metadata entry.
#[derive(Debug, Clone)]
pub struct DecoratorMetadata {
    /// Decorator type/name.
    pub type_: String,
    /// Decorator arguments.
    pub args: Option<Vec<String>>,
}

/// Constructor parameter metadata.
#[derive(Debug, Clone)]
pub struct CtorParameterMetadata {
    /// Type expression for the parameter.
    pub type_: Option<String>,
    /// Decorators on the parameter.
    pub decorators: Vec<DecoratorMetadata>,
}

/// Property decorator metadata.
#[derive(Debug, Clone)]
pub struct PropDecoratorMetadata {
    /// Property name.
    pub name: String,
    /// Decorators on the property.
    pub decorators: Vec<DecoratorMetadata>,
}

impl R3ClassMetadata {
    pub fn new(type_: impl Into<String>) -> Self {
        Self {
            type_: type_.into(),
            decorators: Vec::new(),
            ctor_parameters: None,
            prop_decorators: Vec::new(),
        }
    }
    
    pub fn with_decorators(mut self, decorators: Vec<DecoratorMetadata>) -> Self {
        self.decorators = decorators;
        self
    }
    
    pub fn with_ctor_parameters(mut self, params: Vec<CtorParameterMetadata>) -> Self {
        self.ctor_parameters = Some(params);
        self
    }
    
    pub fn with_prop_decorators(mut self, props: Vec<PropDecoratorMetadata>) -> Self {
        self.prop_decorators = props;
        self
    }
}

/// Extract class metadata for setClassMetadata call.
pub fn extract_class_metadata(
    class_name: &str,
    decorators: &[Decorator],
    ctor_params: Option<&[super::di::CtorParameter]>,
    prop_decorators: &[(String, Vec<Decorator>)],
    is_core: bool,
    annotate_for_closure: bool,
) -> Option<R3ClassMetadata> {
    // Filter to Angular decorators
    let angular_decorators: Vec<DecoratorMetadata> = decorators
        .iter()
        .filter(|d| super::util::is_angular_decorator(d, &d.name, is_core))
        .map(|d| decorator_to_metadata(d))
        .collect();
    
    if angular_decorators.is_empty() {
        return None;
    }
    
    // Convert constructor parameters
    let ctor_meta = ctor_params.map(|params| {
        params.iter().map(|p| ctor_parameter_to_metadata(p, is_core)).collect()
    });
    
    // Convert property decorators
    let prop_meta: Vec<PropDecoratorMetadata> = prop_decorators
        .iter()
        .map(|(name, decs)| PropDecoratorMetadata {
            name: name.clone(),
            decorators: decs.iter()
                .filter(|d| super::util::is_angular_decorator(d, &d.name, is_core))
                .map(decorator_to_metadata)
                .collect(),
        })
        .filter(|p| !p.decorators.is_empty())
        .collect();
    
    Some(R3ClassMetadata {
        type_: class_name.to_string(),
        decorators: angular_decorators,
        ctor_parameters: ctor_meta,
        prop_decorators: prop_meta,
    })
}

/// Convert a decorator to metadata.
pub fn decorator_to_metadata(decorator: &Decorator) -> DecoratorMetadata {
    DecoratorMetadata {
        type_: decorator.name.clone(),
        args: decorator.args.clone(),
    }
}

/// Convert a constructor parameter to metadata.
pub fn ctor_parameter_to_metadata(
    param: &super::di::CtorParameter,
    is_core: bool,
) -> CtorParameterMetadata {
    let decorators: Vec<DecoratorMetadata> = param.decorators
        .iter()
        .filter(|d| is_core || d.is_angular_core())
        .map(|d| DecoratorMetadata {
            type_: d.name.clone(),
            args: if d.args.is_empty() { None } else { Some(d.args.clone()) },
        })
        .collect();
    
    CtorParameterMetadata {
        type_: param.type_token.clone(),
        decorators,
    }
}

/// Convert a class member to metadata.
pub fn decorated_class_member_to_metadata(
    member_name: &str,
    decorators: &[Decorator],
    is_core: bool,
) -> Option<PropDecoratorMetadata> {
    let angular_decorators: Vec<DecoratorMetadata> = decorators
        .iter()
        .filter(|d| super::util::is_angular_decorator(d, &d.name, is_core))
        .map(decorator_to_metadata)
        .collect();
    
    if angular_decorators.is_empty() {
        None
    } else {
        Some(PropDecoratorMetadata {
            name: member_name.to_string(),
            decorators: angular_decorators,
        })
    }
}

/// Check if a decorator should be treated as Angular decorator.
pub fn is_angular_decorator_for_metadata(decorator: &Decorator, is_core: bool) -> bool {
    super::util::is_angular_decorator(decorator, &decorator.name, is_core)
}
