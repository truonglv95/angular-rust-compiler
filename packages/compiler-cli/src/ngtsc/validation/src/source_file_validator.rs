// Source File Validator
//
// Validates source files.

/// Validation result.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    pub fn error(message: String, code: i32) -> Self {
        Self {
            is_valid: false,
            errors: vec![ValidationError { message, code }],
            warnings: Vec::new(),
        }
    }
    
    pub fn add_warning(&mut self, message: &str, code: i32) {
        self.warnings.push(ValidationWarning {
            message: message.to_string(),
            code,
        });
    }
}

/// Validation error.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    pub code: i32,
}

/// Validation warning.
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub message: String,
    pub code: i32,
}

/// Declaration validator.
pub struct DeclarationValidator;

impl DeclarationValidator {
    pub fn new() -> Self {
        Self
    }
    
    pub fn validate_component(
        &self,
        _name: &str,
        selector: Option<&str>,
        _template: Option<&str>,
    ) -> ValidationResult {
        if selector.is_none() {
            return ValidationResult::error(
                "Component must have a selector".to_string(),
                1001,
            );
        }
        ValidationResult::success()
    }
    
    pub fn validate_directive(&self, _name: &str, _selector: &str) -> ValidationResult {
        ValidationResult::success()
    }
    
    pub fn validate_pipe(&self, _name: &str, _pipe_name: &str) -> ValidationResult {
        ValidationResult::success()
    }
}

impl Default for DeclarationValidator {
    fn default() -> Self {
        Self::new()
    }
}
