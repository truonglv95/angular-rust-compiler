//! Version
//!
//! Corresponds to packages/compiler-cli/src/version.ts
//! Version information for the compiler.

/// Angular compiler version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Angular compiler version string.
pub fn version_string() -> String {
    format!("Angular Compiler CLI v{}", VERSION)
}

/// Version information.
#[derive(Debug, Clone)]
pub struct Version {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Patch version.
    pub patch: u32,
    /// Prerelease tag.
    pub prerelease: Option<String>,
}

impl Version {
    /// Parse version from string.
    pub fn parse(version: &str) -> Option<Self> {
        let parts: Vec<&str> = version.split('-').collect();
        let version_part = parts[0];
        let prerelease = parts.get(1).map(|s| s.to_string());
        
        let nums: Vec<&str> = version_part.split('.').collect();
        if nums.len() < 3 {
            return None;
        }
        
        Some(Self {
            major: nums[0].parse().ok()?,
            minor: nums[1].parse().ok()?,
            patch: nums[2].parse().ok()?,
            prerelease,
        })
    }
    
    /// Get current version.
    pub fn current() -> Self {
        Self::parse(VERSION).unwrap_or(Self {
            major: 0,
            minor: 0,
            patch: 1,
            prerelease: Some("dev".to_string()),
        })
    }
    
    /// Format version as string.
    pub fn to_string(&self) -> String {
        let base = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if let Some(ref pre) = self.prerelease {
            format!("{}-{}", base, pre)
        } else {
            base
        }
    }
}

/// Check if TypeScript version is supported.
pub fn check_typescript_version(_ts_version: &str) -> Result<(), String> {
    // Would check TypeScript version compatibility
    Ok(())
}
