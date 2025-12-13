use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;

use crate::utils::hash::hash_bytes;

/// Represents a detected change between two directories.
/// This is an intermediate type - does not include diff_hash since
/// the diff hasn't been created yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChange {
    Patch {
        file: String,
        original_hash: String,
        final_hash: String,
    },
    Add {
        file: String,
        final_hash: String,
    },
    Delete {
        file: String,
        original_hash: String,
    },
}

impl FileChange {
    pub fn file(&self) -> &str {
        match self {
            FileChange::Patch { file, .. } => file,
            FileChange::Add { file, .. } => file,
            FileChange::Delete { file, .. } => file,
        }
    }
}

/// List all file names (not paths) in a directory.
/// Only returns regular files, not subdirectories.
pub fn list_files(dir: &Path) -> io::Result<Vec<String>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;

        if file_type.is_file() {
            if let Some(name) = entry.file_name().to_str() {
                files.push(name.to_string());
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Compare two directories and categorize files into changes.
/// Returns entries for: patch (modified), add (new), delete (removed).
/// Unchanged files (same hash) are skipped.
pub fn categorize_files(orig_dir: &Path, new_dir: &Path) -> io::Result<Vec<FileChange>> {
    let orig_files: HashSet<String> = list_files(orig_dir)?.into_iter().collect();
    let new_files: HashSet<String> = list_files(new_dir)?.into_iter().collect();

    let mut changes = Vec::new();

    // Files in both directories - check if modified
    for file in orig_files.intersection(&new_files) {
        let orig_path = orig_dir.join(file);
        let new_path = new_dir.join(file);

        let orig_data = fs::read(&orig_path)?;
        let new_data = fs::read(&new_path)?;

        let orig_hash = hash_bytes(&orig_data);
        let new_hash = hash_bytes(&new_data);

        if orig_hash != new_hash {
            changes.push(FileChange::Patch {
                file: file.clone(),
                original_hash: orig_hash,
                final_hash: new_hash,
            });
        }
        // Unchanged files are skipped
    }

    // Files only in new directory - add
    for file in new_files.difference(&orig_files) {
        let new_path = new_dir.join(file);
        let new_data = fs::read(&new_path)?;
        let new_hash = hash_bytes(&new_data);

        changes.push(FileChange::Add {
            file: file.clone(),
            final_hash: new_hash,
        });
    }

    // Files only in original directory - delete
    for file in orig_files.difference(&new_files) {
        let orig_path = orig_dir.join(file);
        let orig_data = fs::read(&orig_path)?;
        let orig_hash = hash_bytes(&orig_data);

        changes.push(FileChange::Delete {
            file: file.clone(),
            original_hash: orig_hash,
        });
    }

    // Sort by filename for consistent ordering
    changes.sort_by(|a, b| a.file().cmp(b.file()));

    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn list_files_returns_only_files() {
        let dir = tempdir().unwrap();

        // Create a file
        File::create(dir.path().join("file.txt")).unwrap();

        // Create a subdirectory
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let files = list_files(dir.path()).unwrap();

        assert_eq!(files, vec!["file.txt"]);
    }

    #[test]
    fn list_files_returns_sorted() {
        let dir = tempdir().unwrap();

        File::create(dir.path().join("zebra.bin")).unwrap();
        File::create(dir.path().join("alpha.bin")).unwrap();
        File::create(dir.path().join("middle.bin")).unwrap();

        let files = list_files(dir.path()).unwrap();

        assert_eq!(files, vec!["alpha.bin", "middle.bin", "zebra.bin"]);
    }

    #[test]
    fn list_files_empty_directory() {
        let dir = tempdir().unwrap();

        let files = list_files(dir.path()).unwrap();

        assert!(files.is_empty());
    }

    #[test]
    fn list_files_nonexistent_directory() {
        let result = list_files(Path::new("/nonexistent/directory"));

        assert!(result.is_err());
    }

    #[test]
    fn categorize_identifies_patch() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();

        fs::write(orig_dir.path().join("file.bin"), b"original").unwrap();
        fs::write(new_dir.path().join("file.bin"), b"modified").unwrap();

        let changes = categorize_files(orig_dir.path(), new_dir.path()).unwrap();

        assert_eq!(changes.len(), 1);
        assert!(matches!(
            &changes[0],
            FileChange::Patch { file, original_hash, final_hash }
            if file == "file.bin" && original_hash != final_hash
        ));
    }

    #[test]
    fn categorize_identifies_add() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();

        fs::write(new_dir.path().join("new_file.bin"), b"new content").unwrap();

        let changes = categorize_files(orig_dir.path(), new_dir.path()).unwrap();

        assert_eq!(changes.len(), 1);
        assert!(matches!(
            &changes[0],
            FileChange::Add { file, .. } if file == "new_file.bin"
        ));
    }

    #[test]
    fn categorize_identifies_delete() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();

        fs::write(orig_dir.path().join("old_file.bin"), b"old content").unwrap();

        let changes = categorize_files(orig_dir.path(), new_dir.path()).unwrap();

        assert_eq!(changes.len(), 1);
        assert!(matches!(
            &changes[0],
            FileChange::Delete { file, .. } if file == "old_file.bin"
        ));
    }

    #[test]
    fn categorize_skips_unchanged() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();

        fs::write(orig_dir.path().join("same.bin"), b"same content").unwrap();
        fs::write(new_dir.path().join("same.bin"), b"same content").unwrap();

        let changes = categorize_files(orig_dir.path(), new_dir.path()).unwrap();

        assert!(changes.is_empty());
    }

    #[test]
    fn categorize_mixed_operations() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();

        // Unchanged
        fs::write(orig_dir.path().join("unchanged.bin"), b"same").unwrap();
        fs::write(new_dir.path().join("unchanged.bin"), b"same").unwrap();

        // Modified
        fs::write(orig_dir.path().join("modified.bin"), b"old").unwrap();
        fs::write(new_dir.path().join("modified.bin"), b"new").unwrap();

        // Added
        fs::write(new_dir.path().join("added.bin"), b"added").unwrap();

        // Deleted
        fs::write(orig_dir.path().join("deleted.bin"), b"deleted").unwrap();

        let changes = categorize_files(orig_dir.path(), new_dir.path()).unwrap();

        assert_eq!(changes.len(), 3);

        assert!(changes.iter().any(|c| matches!(c, FileChange::Add { file, .. } if file == "added.bin")));
        assert!(changes.iter().any(|c| matches!(c, FileChange::Delete { file, .. } if file == "deleted.bin")));
        assert!(changes.iter().any(|c| matches!(c, FileChange::Patch { file, .. } if file == "modified.bin")));
    }

    #[test]
    fn categorize_empty_directories() {
        let orig_dir = tempdir().unwrap();
        let new_dir = tempdir().unwrap();

        let changes = categorize_files(orig_dir.path(), new_dir.path()).unwrap();

        assert!(changes.is_empty());
    }

    #[test]
    fn categorize_nonexistent_directory_errors() {
        let new_dir = tempdir().unwrap();

        let result = categorize_files(Path::new("/nonexistent"), new_dir.path());

        assert!(result.is_err());
    }

    #[test]
    fn file_helper_returns_filename() {
        let patch = FileChange::Patch {
            file: "a.bin".to_string(),
            original_hash: "x".to_string(),
            final_hash: "z".to_string(),
        };
        let add = FileChange::Add {
            file: "b.bin".to_string(),
            final_hash: "x".to_string(),
        };
        let delete = FileChange::Delete {
            file: "c.bin".to_string(),
            original_hash: "x".to_string(),
        };

        assert_eq!(patch.file(), "a.bin");
        assert_eq!(add.file(), "b.bin");
        assert_eq!(delete.file(), "c.bin");
    }
}
