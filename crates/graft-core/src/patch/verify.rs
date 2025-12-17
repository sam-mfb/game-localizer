use std::fs;
use std::path::Path;

use crate::patch::PatchError;
use crate::utils::hash::hash_bytes;
use crate::utils::manifest::ManifestEntry;

/// Verify a single manifest entry after it has been applied.
///
/// - Patch: verifies file matches final_hash
/// - Add: verifies file matches final_hash
/// - Delete: verifies file no longer exists
pub fn verify_entry(entry: &ManifestEntry, target_dir: &Path) -> Result<(), PatchError> {
    match entry {
        ManifestEntry::Patch {
            file, final_hash, ..
        }
        | ManifestEntry::Add { file, final_hash } => {
            let target_path = target_dir.join(file);

            let data = fs::read(&target_path).map_err(|e| PatchError::VerificationFailed {
                file: file.clone(),
                expected: final_hash.clone(),
                actual: format!("failed to read file: {}", e),
            })?;

            let actual_hash = hash_bytes(&data);

            if &actual_hash != final_hash {
                return Err(PatchError::VerificationFailed {
                    file: file.clone(),
                    expected: final_hash.clone(),
                    actual: actual_hash,
                });
            }
        }
        ManifestEntry::Delete { file, .. } => {
            let target_path = target_dir.join(file);

            if target_path.exists() {
                return Err(PatchError::VerificationFailed {
                    file: file.clone(),
                    expected: "file deleted".to_string(),
                    actual: "file still exists".to_string(),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn verify_patch_correct_hash() {
        let target_dir = tempdir().unwrap();

        let content = b"patched content";
        fs::write(target_dir.path().join("file.bin"), content).unwrap();

        let entry = ManifestEntry::Patch {
            file: "file.bin".to_string(),
            original_hash: "x".to_string(),
            diff_hash: "y".to_string(),
            final_hash: hash_bytes(content),
        };

        let result = verify_entry(&entry, target_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn verify_patch_wrong_hash() {
        let target_dir = tempdir().unwrap();

        fs::write(target_dir.path().join("file.bin"), b"wrong content").unwrap();

        let entry = ManifestEntry::Patch {
            file: "file.bin".to_string(),
            original_hash: "x".to_string(),
            diff_hash: "y".to_string(),
            final_hash: "expected_hash".to_string(),
        };

        let result = verify_entry(&entry, target_dir.path());
        assert!(matches!(
            result,
            Err(PatchError::VerificationFailed { .. })
        ));
    }

    #[test]
    fn verify_add_correct_hash() {
        let target_dir = tempdir().unwrap();

        let content = b"new file content";
        fs::write(target_dir.path().join("new.bin"), content).unwrap();

        let entry = ManifestEntry::Add {
            file: "new.bin".to_string(),
            final_hash: hash_bytes(content),
        };

        let result = verify_entry(&entry, target_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn verify_add_wrong_hash() {
        let target_dir = tempdir().unwrap();

        fs::write(target_dir.path().join("new.bin"), b"wrong content").unwrap();

        let entry = ManifestEntry::Add {
            file: "new.bin".to_string(),
            final_hash: "expected_hash".to_string(),
        };

        let result = verify_entry(&entry, target_dir.path());
        assert!(matches!(
            result,
            Err(PatchError::VerificationFailed { .. })
        ));
    }

    #[test]
    fn verify_delete_file_gone() {
        let target_dir = tempdir().unwrap();

        let entry = ManifestEntry::Delete {
            file: "deleted.bin".to_string(),
            original_hash: "x".to_string(),
        };

        // File doesn't exist - should pass
        let result = verify_entry(&entry, target_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn verify_delete_file_still_exists() {
        let target_dir = tempdir().unwrap();

        fs::write(target_dir.path().join("deleted.bin"), b"still here").unwrap();

        let entry = ManifestEntry::Delete {
            file: "deleted.bin".to_string(),
            original_hash: "x".to_string(),
        };

        let result = verify_entry(&entry, target_dir.path());
        assert!(matches!(
            result,
            Err(PatchError::VerificationFailed { .. })
        ));
    }

    #[test]
    fn verify_missing_file_errors() {
        let target_dir = tempdir().unwrap();

        let entry = ManifestEntry::Patch {
            file: "missing.bin".to_string(),
            original_hash: "x".to_string(),
            diff_hash: "y".to_string(),
            final_hash: "z".to_string(),
        };

        let result = verify_entry(&entry, target_dir.path());
        assert!(matches!(
            result,
            Err(PatchError::VerificationFailed { .. })
        ));
    }
}
