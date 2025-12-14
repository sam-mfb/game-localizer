pub mod apply;
pub mod verify;

use std::fmt;

/// Error type for patch operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchError {
    ValidationFailed { file: String, reason: String },
    BackupFailed { file: String, reason: String },
    ApplyFailed { file: String, reason: String },
    VerificationFailed { file: String, expected: String, actual: String },
    RollbackFailed { reason: String },
    ManifestError { reason: String },
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatchError::ValidationFailed { file, reason } => {
                write!(f, "validation failed for '{}': {}", file, reason)
            }
            PatchError::BackupFailed { file, reason } => {
                write!(f, "backup failed for '{}': {}", file, reason)
            }
            PatchError::ApplyFailed { file, reason } => {
                write!(f, "apply failed for '{}': {}", file, reason)
            }
            PatchError::VerificationFailed { file, expected, actual } => {
                write!(
                    f,
                    "verification failed for '{}': expected hash {}, got {}",
                    file, expected, actual
                )
            }
            PatchError::RollbackFailed { reason } => {
                write!(f, "rollback failed: {}", reason)
            }
            PatchError::ManifestError { reason } => {
                write!(f, "manifest error: {}", reason)
            }
        }
    }
}

impl std::error::Error for PatchError {}
