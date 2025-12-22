// Initializer Function Access Validation
//
// Validates that initializer API members are compatible with class member visibility.

/// Access levels for class members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLevel {
    Public,
    Protected,
    Private,
}

impl AccessLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessLevel::Public => "public",
            AccessLevel::Protected => "protected",
            AccessLevel::Private => "private",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "public" => Some(AccessLevel::Public),
            "protected" => Some(AccessLevel::Protected),
            "private" => Some(AccessLevel::Private),
            _ => None,
        }
    }
}

/// Metadata for an initializer function API.
#[derive(Debug, Clone)]
pub struct InitializerApiConfig {
    /// The function name (e.g., "input", "output").
    pub function_name: String,
    /// Allowed access levels for this API.
    pub allowed_access_levels: Vec<AccessLevel>,
}

impl InitializerApiConfig {
    pub fn new(function_name: impl Into<String>, allowed: Vec<AccessLevel>) -> Self {
        Self {
            function_name: function_name.into(),
            allowed_access_levels: allowed,
        }
    }
}

/// Error when access level is disallowed.
#[derive(Debug, Clone)]
pub struct AccessLevelError {
    pub function_name: String,
    pub actual_level: AccessLevel,
    pub allowed_levels: Vec<AccessLevel>,
}

impl AccessLevelError {
    pub fn message(&self) -> String {
        let allowed = self.allowed_levels.iter()
            .map(|l| l.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Cannot use \"{}\" on a class member that is declared as {}. Update the class field to be either: {}",
            self.function_name,
            self.actual_level.as_str(),
            allowed
        )
    }
}

/// Validates that the member's access level is compatible with the initializer API.
pub fn validate_access_of_initializer_api_member(
    api: &InitializerApiConfig,
    member_access: AccessLevel,
) -> Result<(), AccessLevelError> {
    if api.allowed_access_levels.contains(&member_access) {
        Ok(())
    } else {
        Err(AccessLevelError {
            function_name: api.function_name.clone(),
            actual_level: member_access,
            allowed_levels: api.allowed_access_levels.clone(),
        })
    }
}
