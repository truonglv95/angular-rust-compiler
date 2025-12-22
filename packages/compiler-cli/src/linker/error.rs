//! Linker Errors

use std::fmt;

/// An error that occurred during linking, recovering source location if possible.
#[derive(Debug, Clone)]
pub struct FatalLinkerError {
    pub message: String,
    pub node_debug_info: String,
}

impl FatalLinkerError {
    pub fn new(message: impl Into<String>, node_debug_info: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            node_debug_info: node_debug_info.into(),
        }
    }
}

impl fmt::Display for FatalLinkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Linker Error: {} (native node: {})", self.message, self.node_debug_info)
    }
}

impl std::error::Error for FatalLinkerError {}
