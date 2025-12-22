// Partial Evaluator Tests
//
// Tests for the partial evaluator module.

#[cfg(test)]
mod tests {
    use crate::ngtsc::partial_evaluator::*;
    
    mod resolved_value_tests {
        use super::*;
        
        #[test]
        fn should_create_string_value() {
            let value = ResolvedValue::String("hello".to_string());
            assert_eq!(value.as_string(), Some("hello"));
        }
        
        #[test]
        fn should_create_number_value() {
            let value = ResolvedValue::Number(42.0);
            assert_eq!(value.as_number(), Some(42.0));
        }
        
        #[test]
        fn should_create_boolean_value() {
            let value = ResolvedValue::Boolean(true);
            assert_eq!(value.as_bool(), Some(true));
        }
        
        #[test]
        fn should_create_array_value() {
            let value = ResolvedValue::Array(vec![
                ResolvedValue::Number(1.0),
                ResolvedValue::Number(2.0),
            ]);
            
            let arr = value.as_array().unwrap();
            assert_eq!(arr.len(), 2);
        }
        
        #[test]
        fn should_identify_known_values() {
            assert!(ResolvedValue::String("test".to_string()).is_known());
            assert!(ResolvedValue::Number(123.0).is_known());
            assert!(ResolvedValue::Boolean(false).is_known());
            assert!(ResolvedValue::Null.is_known());
            assert!(!ResolvedValue::Unknown.is_known());
            assert!(!ResolvedValue::Error("err".to_string()).is_known());
        }
    }
    
    mod partial_evaluator_tests {
        use super::*;
        
        #[test]
        fn should_create_evaluator() {
            let evaluator = PartialEvaluator::new();
            assert!(evaluator.get_known("anything").is_none());
        }
        
        #[test]
        fn should_set_and_get_known_value() {
            let mut evaluator = PartialEvaluator::new();
            evaluator.set_known("MY_CONST", ResolvedValue::String("constant".to_string()));
            
            let value = evaluator.get_known("MY_CONST").unwrap();
            assert_eq!(value.as_string(), Some("constant"));
        }
    }
    
    mod function_ref_tests {
        use super::*;
        
        #[test]
        fn should_create_function_ref() {
            let func_ref = FunctionRef {
                name: "myFunction".to_string(),
                module: Some("./utils".to_string()),
            };
            
            assert_eq!(func_ref.name, "myFunction");
            assert_eq!(func_ref.module, Some("./utils".to_string()));
        }
    }
    
    mod class_ref_tests {
        use super::*;
        
        #[test]
        fn should_create_class_ref() {
            let class_ref = ClassRef {
                name: "MyClass".to_string(),
                module: None,
            };
            
            assert_eq!(class_ref.name, "MyClass");
            assert!(class_ref.module.is_none());
        }
    }
}
