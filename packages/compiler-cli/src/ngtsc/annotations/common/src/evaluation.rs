// Evaluation Utilities
//
// Utilities for resolving and validating evaluated expressions.

/// View encapsulation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewEncapsulation {
    /// No encapsulation.
    None = 0,
    /// Emulated encapsulation.
    Emulated = 1,
    /// Shadow DOM encapsulation.
    ShadowDom = 2,
}

impl ViewEncapsulation {
    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "None" | "ViewEncapsulation.None" => Some(ViewEncapsulation::None),
            "Emulated" | "ViewEncapsulation.Emulated" => Some(ViewEncapsulation::Emulated),
            "ShadowDom" | "ViewEncapsulation.ShadowDom" => Some(ViewEncapsulation::ShadowDom),
            _ => None,
        }
    }
    
    /// Parse from number.
    pub fn from_number(n: i32) -> Option<Self> {
        match n {
            0 => Some(ViewEncapsulation::None),
            1 => Some(ViewEncapsulation::Emulated),
            2 => Some(ViewEncapsulation::ShadowDom),
            _ => None,
        }
    }
}

/// A resolved enum value.
#[derive(Debug, Clone)]
pub struct EnumValue {
    /// The enum name.
    pub enum_name: String,
    /// The member name.
    pub member_name: String,
    /// The resolved numeric value.
    pub resolved: i32,
}

impl EnumValue {
    pub fn new(enum_name: impl Into<String>, member_name: impl Into<String>, resolved: i32) -> Self {
        Self {
            enum_name: enum_name.into(),
            member_name: member_name.into(),
            resolved,
        }
    }
}

/// Resolve an enum value from expression text.
pub fn resolve_enum_value(
    expr_text: &str,
    field: &str,
    enum_symbol_name: &str,
    is_core: bool,
) -> Result<Option<i32>, String> {
    // Try to parse ViewEncapsulation locally
    if enum_symbol_name == "ViewEncapsulation" {
        if let Some(encap) = resolve_encapsulation_enum_value_locally(expr_text) {
            return Ok(Some(encap as i32));
        }
    }
    
    // Check for valid enum reference pattern
    let expected_prefix = format!("{}.", enum_symbol_name);
    if expr_text.starts_with(&expected_prefix) || expr_text.contains(&format!(".{}", expected_prefix)) {
        // Would need actual evaluation here
        Ok(None)
    } else {
        Err(format!(
            "{} must be a member of {} enum from @angular/core",
            field, enum_symbol_name
        ))
    }
}

/// Resolve ViewEncapsulation enum locally from expression text.
pub fn resolve_encapsulation_enum_value_locally(expr_text: &str) -> Option<ViewEncapsulation> {
    let trimmed = expr_text.trim();
    
    // Check for ViewEncapsulation.X or something.ViewEncapsulation.X
    for (name, value) in [
        ("None", ViewEncapsulation::None),
        ("Emulated", ViewEncapsulation::Emulated),
        ("ShadowDom", ViewEncapsulation::ShadowDom),
    ] {
        let suffix = format!("ViewEncapsulation.{}", name);
        if trimmed == suffix || trimmed.ends_with(&format!(".{}", suffix)) {
            return Some(value);
        }
    }
    
    None
}

/// Check if a resolved value is a string array.
pub fn is_string_array(values: &[String]) -> bool {
    // All items are strings by definition in this representation
    true
}

/// Resolved value types.
#[derive(Debug, Clone)]
pub enum ResolvedValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ResolvedValue>),
    Object(std::collections::HashMap<String, ResolvedValue>),
    Reference(String),
    Enum(EnumValue),
    Null,
    Undefined,
    Unknown,
}

impl ResolvedValue {
    pub fn is_string(&self) -> bool {
        matches!(self, ResolvedValue::String(_))
    }
    
    pub fn is_array(&self) -> bool {
        matches!(self, ResolvedValue::Array(_))
    }
    
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ResolvedValue::String(s) => Some(s),
            _ => None,
        }
    }
    
    pub fn as_array(&self) -> Option<&Vec<ResolvedValue>> {
        match self {
            ResolvedValue::Array(arr) => Some(arr),
            _ => None,
        }
    }
    
    pub fn into_string_array(self) -> Option<Vec<String>> {
        match self {
            ResolvedValue::Array(arr) => {
                let mut result = Vec::new();
                for item in arr {
                    if let ResolvedValue::String(s) = item {
                        result.push(s);
                    } else {
                        return None;
                    }
                }
                Some(result)
            }
            _ => None,
        }
    }
}
