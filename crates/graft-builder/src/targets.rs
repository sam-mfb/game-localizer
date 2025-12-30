//! Cross-compilation target definitions

use crate::error::BuildError;

/// A cross-compilation target
#[derive(Debug, Clone)]
pub struct Target {
    /// The Rust target triple (e.g., "x86_64-unknown-linux-gnu")
    pub triple: &'static str,
    /// Short name for the target (e.g., "linux-x64")
    pub name: &'static str,
    /// Binary suffix (e.g., ".exe" for Windows, "" for others)
    pub binary_suffix: &'static str,
}

/// Linux x86_64
pub const LINUX_X64: Target = Target {
    triple: "x86_64-unknown-linux-gnu",
    name: "linux-x64",
    binary_suffix: "",
};

/// Linux ARM64
pub const LINUX_ARM64: Target = Target {
    triple: "aarch64-unknown-linux-gnu",
    name: "linux-arm64",
    binary_suffix: "",
};

/// Windows x86_64
pub const WINDOWS_X64: Target = Target {
    triple: "x86_64-pc-windows-gnu",
    name: "windows",
    binary_suffix: ".exe",
};

/// All available targets
pub const ALL_TARGETS: &[Target] = &[LINUX_X64, LINUX_ARM64, WINDOWS_X64];

/// Parse target names into Target structs
///
/// Accepts short names like "linux-x64", "linux-arm64", "windows"
pub fn parse_targets(names: &[String]) -> Result<Vec<Target>, BuildError> {
    names.iter().map(|name| parse_target(name)).collect()
}

/// Parse a single target name
fn parse_target(name: &str) -> Result<Target, BuildError> {
    match name.to_lowercase().as_str() {
        "linux-x64" | "linux-x86_64" | "x86_64-unknown-linux-gnu" => Ok(LINUX_X64),
        "linux-arm64" | "linux-aarch64" | "aarch64-unknown-linux-gnu" => Ok(LINUX_ARM64),
        "windows" | "windows-x64" | "x86_64-pc-windows-gnu" => Ok(WINDOWS_X64),
        _ => Err(BuildError::InvalidTarget(name.to_string())),
    }
}

/// Get the output binary name for a target
pub fn get_output_name(base_name: &str, target: &Target) -> String {
    format!("{}-{}{}", base_name, target.name, target.binary_suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_linux_x64() {
        let target = parse_target("linux-x64").unwrap();
        assert_eq!(target.triple, "x86_64-unknown-linux-gnu");
        assert_eq!(target.name, "linux-x64");
        assert_eq!(target.binary_suffix, "");
    }

    #[test]
    fn parse_windows() {
        let target = parse_target("windows").unwrap();
        assert_eq!(target.triple, "x86_64-pc-windows-gnu");
        assert_eq!(target.binary_suffix, ".exe");
    }

    #[test]
    fn parse_invalid_target() {
        let result = parse_target("invalid-target");
        assert!(result.is_err());
    }

    #[test]
    fn output_name_linux() {
        let name = get_output_name("patcher", &LINUX_X64);
        assert_eq!(name, "patcher-linux-x64");
    }

    #[test]
    fn output_name_windows() {
        let name = get_output_name("patcher", &WINDOWS_X64);
        assert_eq!(name, "patcher-windows.exe");
    }
}
