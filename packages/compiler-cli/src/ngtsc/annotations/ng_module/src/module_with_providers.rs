// Module With Providers
//
// Handles detection and resolution of ModuleWithProviders<T> types.

/// Result of resolving a ModuleWithProviders type.
#[derive(Debug, Clone)]
pub struct ResolvedModuleWithProviders {
    /// Reference to the NgModule class.
    pub ng_module: String,
    /// Whether this was resolved from a method call.
    pub is_method_call: bool,
}

impl ResolvedModuleWithProviders {
    pub fn new(ng_module: impl Into<String>) -> Self {
        Self {
            ng_module: ng_module.into(),
            is_method_call: false,
        }
    }
    
    pub fn from_method(ng_module: impl Into<String>) -> Self {
        Self {
            ng_module: ng_module.into(),
            is_method_call: true,
        }
    }
}

/// Error when analyzing ModuleWithProviders.
#[derive(Debug, Clone)]
pub struct ModuleWithProvidersError {
    pub message: String,
    pub symbol_name: String,
}

impl ModuleWithProvidersError {
    pub fn missing_generic(symbol_name: impl Into<String>) -> Self {
        let name = symbol_name.into();
        Self {
            message: format!(
                "{} returns a ModuleWithProviders type without a generic type argument. \
                Please add a generic type argument to the ModuleWithProviders type.",
                name
            ),
            symbol_name: name,
        }
    }
}

/// Configuration for creating a ModuleWithProviders resolver.
#[derive(Debug, Clone)]
pub struct MwpResolverConfig {
    /// Whether compiling @angular/core.
    pub is_core: bool,
}

impl MwpResolverConfig {
    pub fn new(is_core: bool) -> Self {
        Self { is_core }
    }
}

/// Check if a type name represents ModuleWithProviders.
pub fn is_module_with_providers_type(
    type_name: &str,
    import_from: Option<&str>,
    is_core: bool,
) -> bool {
    if type_name != "ModuleWithProviders" {
        return false;
    }
    
    // If compiling core, no import check needed
    if is_core {
        return true;
    }
    
    // Must be from @angular/core
    import_from == Some("@angular/core")
}

/// Try to resolve a ModuleWithProviders type from a type expression.
pub fn try_resolve_module_with_providers(
    type_name: &str,
    type_arg: Option<&str>,
    import_from: Option<&str>,
    is_core: bool,
    symbol_name: &str,
) -> Result<Option<ResolvedModuleWithProviders>, ModuleWithProvidersError> {
    if !is_module_with_providers_type(type_name, import_from, is_core) {
        return Ok(None);
    }
    
    // Type argument is required
    let ng_module = type_arg.ok_or_else(|| {
        ModuleWithProvidersError::missing_generic(symbol_name)
    })?;
    
    Ok(Some(ResolvedModuleWithProviders::new(ng_module)))
}

/// Check if a value might be a resolved ModuleWithProviders.
pub fn is_resolved_module_with_providers(value: &dyn std::any::Any) -> bool {
    value.is::<ResolvedModuleWithProviders>()
}
