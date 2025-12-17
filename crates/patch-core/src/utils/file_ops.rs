use std::fs;
use std::io;
use std::path::Path;

/// Copy a file to a backup directory, preserving the filename.
/// Creates the backup directory if it doesn't exist.
pub fn backup_file(file: &Path, backup_dir: &Path) -> io::Result<()> {
    let filename = file
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no filename"))?;

    fs::create_dir_all(backup_dir)?;

    let backup_path = backup_dir.join(filename);
    fs::copy(file, &backup_path)?;

    Ok(())
}

/// Restore a file from a backup directory, overwriting the original.
pub fn restore_file(file: &Path, backup_dir: &Path) -> io::Result<()> {
    let filename = file
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no filename"))?;

    let backup_path = backup_dir.join(filename);
    fs::copy(&backup_path, file)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn backup_copies_file() {
        let source_dir = tempdir().unwrap();
        let backup_dir = tempdir().unwrap();

        let file_path = source_dir.path().join("test.bin");
        fs::write(&file_path, b"original content").unwrap();

        backup_file(&file_path, backup_dir.path()).unwrap();

        let backup_path = backup_dir.path().join("test.bin");
        assert!(backup_path.exists());
        assert_eq!(fs::read(&backup_path).unwrap(), b"original content");
    }

    #[test]
    fn backup_creates_directory() {
        let source_dir = tempdir().unwrap();
        let parent_dir = tempdir().unwrap();
        let backup_dir = parent_dir.path().join("nested").join("backup");

        let file_path = source_dir.path().join("test.bin");
        fs::write(&file_path, b"content").unwrap();

        assert!(!backup_dir.exists());
        backup_file(&file_path, &backup_dir).unwrap();
        assert!(backup_dir.exists());
        assert!(backup_dir.join("test.bin").exists());
    }

    #[test]
    fn backup_missing_file_errors() {
        let backup_dir = tempdir().unwrap();
        let missing = Path::new("/nonexistent/file.bin");

        let result = backup_file(missing, backup_dir.path());

        assert!(result.is_err());
    }

    #[test]
    fn restore_replaces_file() {
        let target_dir = tempdir().unwrap();
        let backup_dir = tempdir().unwrap();

        let file_path = target_dir.path().join("test.bin");
        fs::write(&file_path, b"modified content").unwrap();

        let backup_path = backup_dir.path().join("test.bin");
        fs::write(&backup_path, b"original content").unwrap();

        restore_file(&file_path, backup_dir.path()).unwrap();

        assert_eq!(fs::read(&file_path).unwrap(), b"original content");
    }

    #[test]
    fn restore_creates_file_if_missing() {
        let target_dir = tempdir().unwrap();
        let backup_dir = tempdir().unwrap();

        let file_path = target_dir.path().join("test.bin");
        let backup_path = backup_dir.path().join("test.bin");
        fs::write(&backup_path, b"backup content").unwrap();

        assert!(!file_path.exists());
        restore_file(&file_path, backup_dir.path()).unwrap();
        assert!(file_path.exists());
        assert_eq!(fs::read(&file_path).unwrap(), b"backup content");
    }

    #[test]
    fn restore_missing_backup_errors() {
        let target_dir = tempdir().unwrap();
        let backup_dir = tempdir().unwrap();

        let file_path = target_dir.path().join("test.bin");

        let result = restore_file(&file_path, backup_dir.path());

        assert!(result.is_err());
    }
}
