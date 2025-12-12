use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Patch,
    Add,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub file: String,
    pub operation: Operation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub entries: Vec<ManifestEntry>,
}

impl Manifest {
    pub fn new(version: u32) -> Self {
        Manifest {
            version,
            entries: Vec::new(),
        }
    }

    pub fn load(path: &Path) -> io::Result<Manifest> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, content)
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn roundtrip_serialization() {
        let manifest = Manifest {
            version: 1,
            entries: vec![
                ManifestEntry {
                    file: "game.bin".to_string(),
                    operation: Operation::Patch,
                    original_hash: Some("abc123".to_string()),
                    diff_hash: Some("def456".to_string()),
                    final_hash: Some("ghi789".to_string()),
                },
                ManifestEntry {
                    file: "new_asset.bin".to_string(),
                    operation: Operation::Add,
                    original_hash: None,
                    diff_hash: None,
                    final_hash: Some("jkl012".to_string()),
                },
                ManifestEntry {
                    file: "old_asset.bin".to_string(),
                    operation: Operation::Delete,
                    original_hash: Some("mno345".to_string()),
                    diff_hash: None,
                    final_hash: None,
                },
            ],
        };

        let temp_file = NamedTempFile::new().unwrap();
        manifest.save(temp_file.path()).unwrap();

        let loaded = Manifest::load(temp_file.path()).unwrap();
        assert_eq!(manifest, loaded);
    }

    #[test]
    fn load_from_json_string() {
        let json = r#"{
            "version": 1,
            "entries": [
                {
                    "file": "test.bin",
                    "operation": "patch",
                    "original_hash": "aaa",
                    "diff_hash": "bbb",
                    "final_hash": "ccc"
                }
            ]
        }"#;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), json).unwrap();

        let manifest = Manifest::load(temp_file.path()).unwrap();
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0].file, "test.bin");
        assert_eq!(manifest.entries[0].operation, Operation::Patch);
    }

    #[test]
    fn load_missing_file_returns_error() {
        let result = Manifest::load(Path::new("/nonexistent/manifest.json"));
        assert!(result.is_err());
    }

    #[test]
    fn load_malformed_json_returns_error() {
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "not valid json").unwrap();

        let result = Manifest::load(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn save_produces_valid_json() {
        let manifest = Manifest {
            version: 1,
            entries: vec![ManifestEntry {
                file: "test.bin".to_string(),
                operation: Operation::Add,
                original_hash: None,
                diff_hash: None,
                final_hash: Some("hash123".to_string()),
            }],
        };

        let temp_file = NamedTempFile::new().unwrap();
        manifest.save(temp_file.path()).unwrap();

        let content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("\"operation\": \"add\""));
        assert!(content.contains("\"final_hash\": \"hash123\""));
        assert!(!content.contains("original_hash"));
        assert!(!content.contains("diff_hash"));
    }
}
