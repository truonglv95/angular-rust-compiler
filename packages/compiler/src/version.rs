//! Version Information
//!
//! Corresponds to packages/compiler/src/version.ts (17 lines)

use crate::util::Version;
use once_cell::sync::Lazy;

/// Global VERSION instance
/// Matches Angular's: export const VERSION = new Version('0.0.0-PLACEHOLDER');
pub static VERSION: Lazy<Version> = Lazy::new(|| {
    Version::new("0.0.0-PLACEHOLDER")
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constant() {
        assert_eq!(VERSION.full, "0.0.0-PLACEHOLDER");
    }
}
