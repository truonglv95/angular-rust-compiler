// Xi18n Tests
//
// Tests for the i18n extraction module.

#[cfg(test)]
mod tests {
    use crate::ngtsc::xi18n::*;
    
    mod message_extractor_tests {
        use super::*;
        
        #[test]
        fn should_create_empty_extractor() {
            let extractor = MessageExtractor::new();
            assert!(extractor.messages().is_empty());
        }
        
        #[test]
        fn should_add_message() {
            let mut extractor = MessageExtractor::new();
            
            extractor.add_message(I18nMessage {
                id: "msg1".to_string(),
                content: "Hello".to_string(),
                description: Some("Greeting".to_string()),
                meaning: None,
                source_file: "app.component.html".to_string(),
                source_span: Some((10, 25)),
            });
            
            assert_eq!(extractor.messages().len(), 1);
        }
        
        #[test]
        fn should_get_message_by_id() {
            let mut extractor = MessageExtractor::new();
            
            extractor.add_message(I18nMessage {
                id: "greeting".to_string(),
                content: "Welcome!".to_string(),
                description: None,
                meaning: None,
                source_file: "test.html".to_string(),
                source_span: None,
            });
            
            let msg = extractor.get_message("greeting").unwrap();
            assert_eq!(msg.content, "Welcome!");
        }
        
        #[test]
        fn should_return_none_for_unknown_message() {
            let extractor = MessageExtractor::new();
            assert!(extractor.get_message("unknown").is_none());
        }
    }
    
    mod xliff_output_tests {
        use super::*;
        
        #[test]
        fn should_generate_xliff() {
            let mut extractor = MessageExtractor::new();
            
            extractor.add_message(I18nMessage {
                id: "hello".to_string(),
                content: "Hello World".to_string(),
                description: None,
                meaning: None,
                source_file: "app.html".to_string(),
                source_span: None,
            });
            
            let xliff = extractor.to_xliff();
            
            assert!(xliff.contains("xliff"));
            assert!(xliff.contains("version=\"2.0\""));
            assert!(xliff.contains("hello"));
            assert!(xliff.contains("Hello World"));
        }
    }
    
    mod xmb_output_tests {
        use super::*;
        
        #[test]
        fn should_generate_xmb() {
            let mut extractor = MessageExtractor::new();
            
            extractor.add_message(I18nMessage {
                id: "goodbye".to_string(),
                content: "Goodbye".to_string(),
                description: None,
                meaning: None,
                source_file: "test.html".to_string(),
                source_span: None,
            });
            
            let xmb = extractor.to_xmb();
            
            assert!(xmb.contains("messagebundle"));
            assert!(xmb.contains("msg id=\"goodbye\""));
            assert!(xmb.contains("Goodbye"));
        }
    }
}
