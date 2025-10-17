//! Compiler Configuration
//!
//! Corresponds to packages/compiler/src/config.ts (37 lines)

use crate::core::ViewEncapsulation;

#[derive(Debug, Clone)]
pub struct CompilerConfig {
    pub default_encapsulation: Option<ViewEncapsulation>,
    pub preserve_whitespaces: bool,
    pub strict_injection_parameters: bool,
}

impl CompilerConfig {
    pub fn new(
        default_encapsulation: Option<ViewEncapsulation>,
        preserve_whitespaces: Option<bool>,
        strict_injection_parameters: Option<bool>,
    ) -> Self {
        CompilerConfig {
            default_encapsulation: default_encapsulation.or(Some(ViewEncapsulation::Emulated)),
            preserve_whitespaces: preserve_whitespaces_default(preserve_whitespaces, false),
            strict_injection_parameters: strict_injection_parameters.unwrap_or(false),
        }
    }
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self::new(None, None, None)
    }
}

pub fn preserve_whitespaces_default(
    preserve_whitespaces_option: Option<bool>,
    default_setting: bool,
) -> bool {
    preserve_whitespaces_option.unwrap_or(default_setting)
}
