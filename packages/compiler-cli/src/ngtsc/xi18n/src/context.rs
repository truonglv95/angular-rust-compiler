// Xi18n Context
//
// Message extraction context and utilities.

use std::collections::HashMap;

/// I18n message.
#[derive(Debug, Clone)]
pub struct I18nMessage {
    pub id: String,
    pub content: String,
    pub description: Option<String>,
    pub meaning: Option<String>,
    pub source_file: String,
    pub source_span: Option<(usize, usize)>,
}

/// Message extractor.
#[derive(Debug, Default)]
pub struct MessageExtractor {
    messages: HashMap<String, I18nMessage>,
}

impl MessageExtractor {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn add_message(&mut self, message: I18nMessage) {
        self.messages.insert(message.id.clone(), message);
    }
    
    pub fn get_message(&self, id: &str) -> Option<&I18nMessage> {
        self.messages.get(id)
    }
    
    pub fn messages(&self) -> Vec<&I18nMessage> {
        self.messages.values().collect()
    }
    
    pub fn to_xliff(&self) -> String {
        let mut output = String::new();
        output.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        output.push('\n');
        output.push_str(r#"<xliff version="2.0" xmlns="urn:oasis:names:tc:xliff:document:2.0">"#);
        output.push('\n');
        output.push_str("  <file>\n");
        
        for msg in self.messages.values() {
            output.push_str(&format!(
                "    <unit id=\"{}\">\n      <segment>\n        <source>{}</source>\n      </segment>\n    </unit>\n",
                msg.id, msg.content
            ));
        }
        
        output.push_str("  </file>\n");
        output.push_str("</xliff>\n");
        output
    }
    
    pub fn to_xmb(&self) -> String {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<messagebundle>\n");
        
        for msg in self.messages.values() {
            if let Some(ref desc) = msg.description {
                output.push_str(&format!("  <msg id=\"{}\" desc=\"{}\">{}</msg>\n", 
                    msg.id, desc, msg.content));
            } else {
                output.push_str(&format!("  <msg id=\"{}\">{}</msg>\n", 
                    msg.id, msg.content));
            }
        }
        
        output.push_str("</messagebundle>\n");
        output
    }
}
