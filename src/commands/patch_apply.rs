use std::fs;
use std::path::Path;

use crate::patch::apply::apply_entry;
use crate::patch::verify::verify_entry;
use crate::patch::{PatchError, BACKUP_DIR, MANIFEST_FILENAME};
use crate::utils::file_ops::{backup_file, restore_file};
use crate::utils::hash::hash_bytes;
use crate::utils::manifest::{Manifest, ManifestEntry};

/// Apply a patch to a target directory.
///
/// Workflow:
/// 1. Load and parse manifest
/// 2. Validate all entries (files exist, hashes match)
/// 3. Backup all files that will be modified/deleted
/// 4. Apply each entry, verifying immediately after
/// 5. On any failure, rollback to original state
pub fn run(target_dir: &Path, patch_dir: &Path) -> Result<(), PatchError> {
    // Load manifest
    let manifest_path = patch_dir.join(MANIFEST_FILENAME);
    let manifest = Manifest::load(&manifest_path).map_err(|e| PatchError::ManifestError {
        reason: e.to_string(),
    })?;

    // Validation phase
    validate_entries(&manifest.entries, target_dir)?;

    // Backup phase
    let backup_dir = target_dir.join(BACKUP_DIR);
    backup_entries(&manifest.entries, target_dir, &backup_dir)?;

    // Apply+verify phase (with rollback on failure)
    let mut applied: Vec<&ManifestEntry> = Vec::new();

    for entry in &manifest.entries {
        if let Err(e) = apply_entry(entry, target_dir, patch_dir) {
            rollback(&applied, target_dir, &backup_dir)?;
            return Err(e);
        }

        if let Err(e) = verify_entry(entry, target_dir) {
            rollback(&applied, target_dir, &backup_dir)?;
            return Err(e);
        }

        applied.push(entry);
    }

    Ok(())
}

/// Validate all entries before applying any changes.
fn validate_entries(entries: &[ManifestEntry], target_dir: &Path) -> Result<(), PatchError> {
    for entry in entries {
        match entry {
            ManifestEntry::Patch {
                file,
                original_hash,
                ..
            } => {
                let target_path = target_dir.join(file);

                if !target_path.exists() {
                    return Err(PatchError::ValidationFailed {
                        file: file.clone(),
                        reason: "file not found in target".to_string(),
                    });
                }

                let data = fs::read(&target_path).map_err(|e| PatchError::ValidationFailed {
                    file: file.clone(),
                    reason: format!("failed to read file: {}", e),
                })?;

                let actual_hash = hash_bytes(&data);
                if &actual_hash != original_hash {
                    return Err(PatchError::ValidationFailed {
                        file: file.clone(),
                        reason: format!(
                            "hash mismatch: expected {}, got {}",
                            original_hash, actual_hash
                        ),
                    });
                }
            }
            ManifestEntry::Add { file, .. } => {
                let target_path = target_dir.join(file);

                if target_path.exists() {
                    return Err(PatchError::ValidationFailed {
                        file: file.clone(),
                        reason: "file already exists in target".to_string(),
                    });
                }
            }
            ManifestEntry::Delete { file, original_hash } => {
                let target_path = target_dir.join(file);

                // Only validate hash if file exists - already gone is fine
                if target_path.exists() {
                    let data = fs::read(&target_path).map_err(|e| PatchError::ValidationFailed {
                        file: file.clone(),
                        reason: format!("failed to read file: {}", e),
                    })?;

                    let actual_hash = hash_bytes(&data);
                    if &actual_hash != original_hash {
                        return Err(PatchError::ValidationFailed {
                            file: file.clone(),
                            reason: format!(
                                "hash mismatch: expected {}, got {}",
                                original_hash, actual_hash
                            ),
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

/// Backup all files that will be modified or deleted.
fn backup_entries(
    entries: &[ManifestEntry],
    target_dir: &Path,
    backup_dir: &Path,
) -> Result<(), PatchError> {
    for entry in entries {
        match entry {
            ManifestEntry::Patch { file, .. } | ManifestEntry::Delete { file, .. } => {
                let target_path = target_dir.join(file);

                // Only backup if file exists (delete entries may already be gone)
                if target_path.exists() {
                    backup_file(&target_path, backup_dir).map_err(|e| PatchError::BackupFailed {
                        file: file.clone(),
                        reason: e.to_string(),
                    })?;
                }
            }
            ManifestEntry::Add { .. } => {
                // Nothing to backup for new files
            }
        }
    }

    Ok(())
}

/// Rollback applied changes by restoring from backup and removing added files.
fn rollback(
    applied: &[&ManifestEntry],
    target_dir: &Path,
    backup_dir: &Path,
) -> Result<(), PatchError> {
    for entry in applied {
        match entry {
            ManifestEntry::Patch { file, .. } => {
                // Patch entries always have backups (validated to exist)
                let target_path = target_dir.join(file);
                restore_file(&target_path, backup_dir).map_err(|e| PatchError::RollbackFailed {
                    reason: format!("failed to restore '{}': {}", file, e),
                })?;
            }
            ManifestEntry::Delete { file, .. } => {
                // Only restore if we have a backup (file existed before patch)
                let backup_path = backup_dir.join(file);
                if backup_path.exists() {
                    let target_path = target_dir.join(file);
                    restore_file(&target_path, backup_dir).map_err(|e| {
                        PatchError::RollbackFailed {
                            reason: format!("failed to restore '{}': {}", file, e),
                        }
                    })?;
                }
            }
            ManifestEntry::Add { file, .. } => {
                // Remove the newly added file
                let target_path = target_dir.join(file);
                if target_path.exists() {
                    fs::remove_file(&target_path).map_err(|e| PatchError::RollbackFailed {
                        reason: format!("failed to remove added file '{}': {}", file, e),
                    })?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::patch_create;
    use tempfile::tempdir;

    #[test]
    fn successful_apply_modifies_target() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();
        let patch_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Set up original and new directories
        fs::write(orig_dir.path().join("modified.bin"), b"original").unwrap();
        fs::write(new_dir.path().join("modified.bin"), b"modified").unwrap();
        fs::write(new_dir.path().join("added.bin"), b"new file").unwrap();
        fs::write(orig_dir.path().join("deleted.bin"), b"to delete").unwrap();

        // Create patch
        patch_create::run(orig_dir.path(), new_dir.path(), patch_dir.path(), 1).unwrap();

        // Set up target (copy of original)
        fs::write(target_dir.path().join("modified.bin"), b"original").unwrap();
        fs::write(target_dir.path().join("deleted.bin"), b"to delete").unwrap();

        // Apply patch
        run(target_dir.path(), patch_dir.path()).unwrap();

        // Verify results
        assert_eq!(
            fs::read(target_dir.path().join("modified.bin")).unwrap(),
            b"modified"
        );
        assert_eq!(
            fs::read(target_dir.path().join("added.bin")).unwrap(),
            b"new file"
        );
        assert!(!target_dir.path().join("deleted.bin").exists());
    }

    #[test]
    fn validation_rejects_missing_file() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();
        let patch_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a patch that modifies a file
        fs::write(orig_dir.path().join("file.bin"), b"original").unwrap();
        fs::write(new_dir.path().join("file.bin"), b"modified").unwrap();
        patch_create::run(orig_dir.path(), new_dir.path(), patch_dir.path(), 1).unwrap();

        // Target is missing the file
        let result = run(target_dir.path(), patch_dir.path());

        assert!(matches!(result, Err(PatchError::ValidationFailed { .. })));
    }

    #[test]
    fn validation_rejects_hash_mismatch() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();
        let patch_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a patch
        fs::write(orig_dir.path().join("file.bin"), b"original").unwrap();
        fs::write(new_dir.path().join("file.bin"), b"modified").unwrap();
        patch_create::run(orig_dir.path(), new_dir.path(), patch_dir.path(), 1).unwrap();

        // Target has different content
        fs::write(target_dir.path().join("file.bin"), b"different").unwrap();

        let result = run(target_dir.path(), patch_dir.path());

        assert!(matches!(result, Err(PatchError::ValidationFailed { .. })));
    }

    #[test]
    fn validation_rejects_existing_add_target() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();
        let patch_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a patch that adds a file
        fs::write(new_dir.path().join("new.bin"), b"new content").unwrap();
        patch_create::run(orig_dir.path(), new_dir.path(), patch_dir.path(), 1).unwrap();

        // Target already has that file
        fs::write(target_dir.path().join("new.bin"), b"existing").unwrap();

        let result = run(target_dir.path(), patch_dir.path());

        assert!(matches!(result, Err(PatchError::ValidationFailed { .. })));
    }

    #[test]
    fn already_deleted_file_succeeds() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();
        let patch_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a patch that deletes a file
        fs::write(orig_dir.path().join("deleted.bin"), b"content").unwrap();
        patch_create::run(orig_dir.path(), new_dir.path(), patch_dir.path(), 1).unwrap();

        // Target doesn't have the file (already deleted)
        let result = run(target_dir.path(), patch_dir.path());

        assert!(result.is_ok());
    }

    #[test]
    fn rollback_restores_on_failure() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();
        let patch_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create patch with two modifications
        fs::write(orig_dir.path().join("a.bin"), b"original a").unwrap();
        fs::write(new_dir.path().join("a.bin"), b"modified a").unwrap();
        fs::write(orig_dir.path().join("b.bin"), b"original b").unwrap();
        fs::write(new_dir.path().join("b.bin"), b"modified b").unwrap();
        patch_create::run(orig_dir.path(), new_dir.path(), patch_dir.path(), 1).unwrap();

        // Set up target correctly for first file, but corrupt the diff for second
        fs::write(target_dir.path().join("a.bin"), b"original a").unwrap();
        fs::write(target_dir.path().join("b.bin"), b"original b").unwrap();

        // Corrupt the second diff file to cause apply failure
        let diffs_dir = patch_dir.path().join("diffs");
        fs::write(diffs_dir.join("b.bin.diff"), b"corrupted diff data").unwrap();

        let result = run(target_dir.path(), patch_dir.path());

        // Should fail
        assert!(result.is_err());

        // First file should be rolled back to original
        assert_eq!(
            fs::read(target_dir.path().join("a.bin")).unwrap(),
            b"original a"
        );
    }

    #[test]
    fn backup_preserved_on_success() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();
        let patch_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(orig_dir.path().join("file.bin"), b"original").unwrap();
        fs::write(new_dir.path().join("file.bin"), b"modified").unwrap();
        patch_create::run(orig_dir.path(), new_dir.path(), patch_dir.path(), 1).unwrap();

        fs::write(target_dir.path().join("file.bin"), b"original").unwrap();

        run(target_dir.path(), patch_dir.path()).unwrap();

        // Backup directory should exist with original file
        let backup_dir = target_dir.path().join(BACKUP_DIR);
        assert!(backup_dir.exists());
        assert_eq!(fs::read(backup_dir.join("file.bin")).unwrap(), b"original");
    }

    #[test]
    fn missing_manifest_returns_error() {
        let target_dir = tempdir().unwrap();
        let patch_dir = tempdir().unwrap();

        let result = run(target_dir.path(), patch_dir.path());

        assert!(matches!(result, Err(PatchError::ManifestError { .. })));
    }
}
