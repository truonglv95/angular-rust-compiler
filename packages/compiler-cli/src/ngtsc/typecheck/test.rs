// TypeCheck Tests
//
// Tests for the typecheck module.

#[cfg(test)]
mod tests {
    use super::api::*;
    use super::src::*;
    
    mod type_checking_config_tests {
        use super::*;
        
        #[test]
        fn should_create_default_config() {
            let config = TypeCheckingConfig::default();
            assert!(!config.check_type_of_input_bindings);
            assert!(!config.strict_null_input_bindings);
        }
        
        #[test]
        fn should_create_strict_config() {
            let config = TypeCheckingConfig::strict();
            assert!(config.check_type_of_input_bindings);
            assert!(config.strict_null_input_bindings);
            assert!(config.check_type_of_output_events);
            assert!(config.check_type_of_animations);
        }
    }
    
    mod type_check_context_tests {
        use super::*;
        
        #[test]
        fn should_create_context() {
            let config = TypeCheckingConfig::default();
            let ctx = TypeCheckingContext::new("test.ts".to_string(), config);
            
            assert!(ctx.get_diagnostics().is_empty());
        }
        
        #[test]
        fn should_record_diagnostics() {
            let config = TypeCheckingConfig::default();
            let mut ctx = TypeCheckingContext::new("test.ts".to_string(), config);
            
            ctx.add_diagnostic(TypeCheckDiagnostic {
                message: "Test error".to_string(),
                code: 1000,
                span: None,
            });
            
            assert_eq!(ctx.get_diagnostics().len(), 1);
        }
    }
    
    mod template_type_checker_tests {
        use super::*;
        
        #[test]
        fn should_create_checker() {
            let config = TypeCheckingConfig::default();
            let checker = TemplateTypeCheckerImpl::new(config);
            
            // Initially no diagnostics
            assert!(checker.get_all_diagnostics().is_empty());
        }
        
        #[test]
        fn should_check_simple_template() {
            let config = TypeCheckingConfig::default();
            let mut checker = TemplateTypeCheckerImpl::new(config);
            
            // Add a component for checking
            checker.add_component(ComponentToCheck {
                name: "TestComponent".to_string(),
                template: "<div>Hello</div>".to_string(),
                file: "test.ts".to_string(),
            });
            
            let result = checker.check_all();
            // Simple template should have no errors
            assert!(result.is_ok());
        }
    }
    
    mod template_symbol_tests {
        use super::*;
        
        #[test]
        fn should_identify_element_symbol() {
            let symbol = TemplateSymbol::Element(ElementSymbol {
                name: "div".to_string(),
                attributes: vec![],
            });
            
            assert!(matches!(symbol, TemplateSymbol::Element(_)));
        }
        
        #[test]
        fn should_identify_directive_symbol() {
            let symbol = TemplateSymbol::Directive(DirectiveSymbol {
                name: "NgIf".to_string(),
                selector: "[ngIf]".to_string(),
                is_component: false,
            });
            
            assert!(matches!(symbol, TemplateSymbol::Directive(_)));
        }
        
        #[test]
        fn should_identify_reference_symbol() {
            let symbol = TemplateSymbol::Reference(ReferenceSymbol {
                name: "myRef".to_string(),
                target: None,
            });
            
            assert!(matches!(symbol, TemplateSymbol::Reference(_)));
        }
        
        #[test]
        fn should_identify_variable_symbol() {
            let symbol = TemplateSymbol::Variable(VariableSymbol {
                name: "item".to_string(),
                kind: VariableKind::LetDeclaration,
            });
            
            assert!(matches!(symbol, TemplateSymbol::Variable(_)));
        }
    }
    
    mod diagnostics_tests {
        use super::*;
        
        #[test]
        fn should_create_error_diagnostic() {
            let diag = create_template_diagnostic(
                "Property 'foo' does not exist on type 'Bar'".to_string(),
                8002,
                None,
            );
            
            assert_eq!(diag.code, 8002);
            assert!(diag.message.contains("foo"));
        }
        
        #[test]
        fn should_have_correct_error_codes() {
            assert_eq!(TemplateDiagnosticCode::InvalidBananaInBox as i32, 8101);
            assert_eq!(TemplateDiagnosticCode::NullishCoalescingNotNullable as i32, 8102);
            assert_eq!(TemplateDiagnosticCode::MissingControlFlowDirective as i32, 8103);
        }
    }
}
