//! Default Configuration
//!
//! Corresponds to packages/compiler/src/ml_parser/defaults.ts (35 lines)

use crate::assertions::assert_interpolation_symbols;
use std::collections::HashSet;
use once_cell::sync::Lazy;

/// Interpolation configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpolationConfig {
    pub start: String,
    pub end: String,
}

impl InterpolationConfig {
    pub fn new(start: String, end: String) -> Self {
        InterpolationConfig { start, end }
    }

    pub fn from_array(markers: Option<&[String]>) -> Result<Self, String> {
        match markers {
            None => Ok(default_interpolation_config()),
            Some(m) => {
                assert_interpolation_symbols("interpolation", Some(m))?;
                Ok(InterpolationConfig::new(m[0].clone(), m[1].clone()))
            }
        }
    }
}

/// Default interpolation config {{ }}
pub fn default_interpolation_config() -> InterpolationConfig {
    InterpolationConfig::new("{{".to_string(), "}}".to_string())
}

/// Default container blocks
pub static DEFAULT_CONTAINER_BLOCKS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();
    set.insert("switch");
    set
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_interpolation_config() {
        let config = default_interpolation_config();
        assert_eq!(config.start, "{{");
        assert_eq!(config.end, "}}");
    }

    #[test]
    fn test_interpolation_config_from_array() {
        let markers = vec!["[[".to_string(), "]]".to_string()];
        let config = InterpolationConfig::from_array(Some(&markers)).unwrap();
        assert_eq!(config.start, "[[");
        assert_eq!(config.end, "]]");
    }

    #[test]
    fn test_interpolation_config_from_none() {
        let config = InterpolationConfig::from_array(None).unwrap();
        assert_eq!(config.start, "{{");
        assert_eq!(config.end, "}}");
    }

    #[test]
    fn test_default_container_blocks() {
        assert!(DEFAULT_CONTAINER_BLOCKS.contains("switch"));
    }
}

