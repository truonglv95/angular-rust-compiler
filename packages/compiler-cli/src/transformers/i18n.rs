// i18n Transformers
//
// Internationalization support for transformers.

use super::api::I18nFormat;

/// i18n message bundle loader.
pub struct MessageLoader {
    /// Loaded messages by ID.
    messages: std::collections::HashMap<String, String>,
    /// Locale.
    locale: String,
}

impl MessageLoader {
    pub fn new(locale: impl Into<String>) -> Self {
        Self {
            messages: std::collections::HashMap::new(),
            locale: locale.into(),
        }
    }
    
    /// Load messages from a file.
    pub fn load(&mut self, content: &str, format: I18nFormat) -> Result<(), String> {
        match format {
            I18nFormat::Xlf | I18nFormat::Xlf2 => self.load_xliff(content),
            I18nFormat::Xmb => self.load_xmb(content),
            I18nFormat::Json => self.load_json(content),
        }
    }
    
    /// Get a translated message.
    pub fn get(&self, id: &str) -> Option<&str> {
        self.messages.get(id).map(|s| s.as_str())
    }
    
    /// Get locale.
    pub fn locale(&self) -> &str {
        &self.locale
    }
    
    fn load_xliff(&mut self, _content: &str) -> Result<(), String> {
        // Simplified XLIFF parsing
        Ok(())
    }
    
    fn load_xmb(&mut self, _content: &str) -> Result<(), String> {
        // Simplified XMB parsing
        Ok(())
    }
    
    fn load_json(&mut self, content: &str) -> Result<(), String> {
        // Simple JSON parsing
        for line in content.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().trim_matches('"');
                let value = value.trim().trim_matches(',').trim_matches('"');
                self.messages.insert(key.to_string(), value.to_string());
            }
        }
        Ok(())
    }
}

/// i18n inlining transformer.
pub struct I18nInlineTransformer {
    /// Message loader.
    loader: MessageLoader,
    /// Missing translation strategy.
    missing_strategy: MissingTranslationStrategy,
}

/// Strategy for missing translations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MissingTranslationStrategy {
    /// Throw an error.
    Error,
    /// Log a warning.
    #[default]
    Warning,
    /// Ignore missing translations.
    Ignore,
}

impl I18nInlineTransformer {
    pub fn new(loader: MessageLoader, missing_strategy: MissingTranslationStrategy) -> Self {
        Self {
            loader,
            missing_strategy,
        }
    }
    
    /// Translate a message.
    pub fn translate(&self, id: &str, fallback: &str) -> String {
        if let Some(msg) = self.loader.get(id) {
            msg.to_string()
        } else {
            match self.missing_strategy {
                MissingTranslationStrategy::Error => {
                    panic!("Missing translation for: {}", id);
                }
                MissingTranslationStrategy::Warning => {
                    eprintln!("Warning: Missing translation for: {}", id);
                    fallback.to_string()
                }
                MissingTranslationStrategy::Ignore => {
                    fallback.to_string()
                }
            }
        }
    }
}
