use std::path::Path;

use graft_core::patch::{
    rollback, validate_backup, validate_patched_entries, PatchError, Progress, ProgressAction,
    BACKUP_DIR,
};
use graft_core::utils::manifest::Manifest;

fn format_action(action: ProgressAction) -> &'static str {
    match action {
        ProgressAction::Validating => "Validating",
        ProgressAction::CheckingNotExists => "Checking",
        ProgressAction::BackingUp => "Backing up",
        ProgressAction::Skipping => "Skipping",
        ProgressAction::Patching => "Patching",
        ProgressAction::Adding => "Adding",
        ProgressAction::Deleting => "Deleting",
        ProgressAction::Restoring => "Restoring",
        ProgressAction::Removing => "Removing",
    }
}

/// Rollback a previously applied patch using the backup directory.
///
/// This restores files from `.patch-backup` to their original state.
///
/// If `force` is false, validates that patched files are in expected state first.
/// If `force` is true, skips patched files validation (but still validates backups).
pub fn run(target_dir: &Path, manifest_path: &Path, force: bool) -> Result<(), PatchError> {
    // Load manifest
    let manifest = Manifest::load(manifest_path).map_err(|e| PatchError::ManifestError {
        reason: e.to_string(),
    })?;

    // Get backup directory
    let backup_dir = target_dir.join(BACKUP_DIR);
    if !backup_dir.exists() {
        return Err(PatchError::RollbackFailed {
            reason: format!("backup directory not found: {}", backup_dir.display()),
        });
    }

    // Validate patched files are in expected state (skip if --force)
    if !force {
        validate_patched_entries(&manifest.entries, target_dir, Some(|p: Progress| {
            println!("{} [{}/{}]: {}", format_action(p.action), p.index + 1, p.total, p.file);
        }))?;
    }

    // Validate backup integrity before rolling back (always required)
    validate_backup(&manifest.entries, &backup_dir, Some(|p: Progress| {
        println!("{} [{}/{}]: {}", format_action(p.action), p.index + 1, p.total, p.file);
    }))?;

    // Rollback all entries (treat all as "applied")
    let entries: Vec<_> = manifest.entries.iter().collect();
    rollback(&entries, target_dir, &backup_dir, Some(|p: Progress| {
        println!("{} [{}/{}]: {}", format_action(p.action), p.index + 1, p.total, p.file);
    }))?;

    Ok(())
}
