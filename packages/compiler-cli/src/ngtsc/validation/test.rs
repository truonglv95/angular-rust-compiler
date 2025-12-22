// Validation Tests
//
// Tests for the validation module.

#[cfg(test)]
mod tests {
    use crate::ngtsc::validation::*;
    
    mod validator_tests {
        use super::*;
        
        #[test]
        fn should_create_validator() {
            let validator = DeclarationValidator::new();
            let _ = validator;
        }
        
        #[test]
        fn should_validate_component() {
            let validator = DeclarationValidator::new();
            
            let result = validator.validate_component(
                "TestComponent",
                Some("test-selector"),
                Some("<div>Template</div>"),
            );
            
            assert!(result.is_valid);
        }
        
        #[test]
        fn should_fail_component_without_selector() {
            let validator = DeclarationValidator::new();
            
            let result = validator.validate_component(
                "TestComponent",
                None,
                Some("<div>Template</div>"),
            );
            
            // Components need selectors
            assert!(!result.is_valid || !result.errors.is_empty());
        }
        
        #[test]
        fn should_validate_directive() {
            let validator = DeclarationValidator::new();
            
            let result = validator.validate_directive(
                "TestDirective",
                "[appTest]",
            );
            
            assert!(result.is_valid);
        }
        
        #[test]
        fn should_validate_pipe() {
            let validator = DeclarationValidator::new();
            
            let result = validator.validate_pipe(
                "TestPipe",
                "testPipe",
            );
            
            assert!(result.is_valid);
            assert!(result.errors.is_empty());
        }
    }
    
    mod validation_result_tests {
        use super::*;
        
        #[test]
        fn should_create_success_result() {
            let result = ValidationResult::success();
            assert!(result.is_valid);
            assert!(result.errors.is_empty());
            assert!(result.warnings.is_empty());
        }
        
        #[test]
        fn should_create_error_result() {
            let result = ValidationResult::error("Something went wrong".to_string(), 1001);
            assert!(!result.is_valid);
            assert_eq!(result.errors.len(), 1);
            assert_eq!(result.errors[0].code, 1001);
        }
        
        #[test]
        fn should_add_warning() {
            let mut result = ValidationResult::success();
            result.add_warning("Consider using...", 2001);
            
            assert!(result.is_valid);
            assert_eq!(result.warnings.len(), 1);
        }
    }
}
