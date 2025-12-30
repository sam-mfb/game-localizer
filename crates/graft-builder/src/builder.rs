use crate::archive;
use crate::error::BuildError;
use crate::targets::{self, Target};
use graft_core::patch;
use graft_core::utils::manifest::PatchInfo;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Build a GUI patcher executable from a patch directory.
///
/// # Arguments
/// * `patch_dir` - Path to the patch directory (containing manifest.json)
/// * `output_dir` - Directory where the built executable will be placed
/// * `name` - Optional name for the executable (defaults to "patcher")
///
/// # Returns
/// Path to the built executable on success.
pub fn build(patch_dir: &Path, output_dir: &Path, name: Option<&str>) -> Result<PathBuf, BuildError> {
    let results = build_impl(patch_dir, output_dir, name, None)?;
    // For single-target build, return the single path
    Ok(results.into_iter().next().unwrap())
}

/// Build GUI patcher executables for multiple targets using cross-compilation.
///
/// # Arguments
/// * `patch_dir` - Path to the patch directory (containing manifest.json)
/// * `output_dir` - Directory where the built executables will be placed
/// * `name` - Optional base name for the executables (defaults to "patcher")
/// * `targets` - List of targets to build for
///
/// # Returns
/// List of paths to the built executables on success.
pub fn build_cross(
    patch_dir: &Path,
    output_dir: &Path,
    name: Option<&str>,
    targets: &[Target],
) -> Result<Vec<PathBuf>, BuildError> {
    // Check that cross is available
    check_cross_available()?;

    build_impl(patch_dir, output_dir, name, Some(targets))
}

/// Internal implementation shared by build and build_cross
fn build_impl(
    patch_dir: &Path,
    output_dir: &Path,
    name: Option<&str>,
    targets: Option<&[Target]>,
) -> Result<Vec<PathBuf>, BuildError> {
    // Step 1: Validate patch directory
    let manifest = patch::validate_patch_dir(patch_dir)?;
    let patch_info = PatchInfo::from_manifest(&manifest);
    let patcher_name = name.unwrap_or("patcher");

    println!(
        "Building patcher for patch v{} ({} entries: {} patches, {} additions, {} deletions)...",
        patch_info.version,
        patch_info.entry_count,
        patch_info.patches,
        patch_info.additions,
        patch_info.deletions
    );

    // Step 2: Find workspace root
    let workspace_root = find_workspace_root()?;

    // Step 3: Create the archive in temp location (cleaned up when archive is dropped)
    println!("Creating patch archive...");
    let archive = archive::ArchiveFile::create(patch_dir)
        .map_err(BuildError::ArchiveCreationFailed)?;

    // Step 4: Create output directory
    fs::create_dir_all(output_dir).map_err(|e| BuildError::OutputDirCreationFailed {
        path: output_dir.to_path_buf(),
        source: e,
    })?;

    // Step 5: Build for each target
    let mut output_paths = Vec::new();

    match targets {
        None => {
            // Native build (existing behavior)
            println!("Building graft-gui with embedded patch...");
            run_cargo_build(&workspace_root, archive.path())?;

            let binary_name = get_binary_name(patcher_name);
            let source_binary = get_release_binary_path(&workspace_root, None);
            let dest_binary = output_dir.join(&binary_name);

            copy_binary(&source_binary, &dest_binary)?;
            output_paths.push(dest_binary);
        }
        Some(target_list) => {
            // Cross-compilation
            for target in target_list {
                println!("Building for {}...", target.name);
                run_cross_build(&workspace_root, archive.path(), target)?;

                let output_name = targets::get_output_name(patcher_name, target);
                let source_binary = get_release_binary_path(&workspace_root, Some(target));
                let dest_binary = output_dir.join(&output_name);

                copy_binary(&source_binary, &dest_binary)?;
                output_paths.push(dest_binary);
                println!("  -> {}", output_name);
            }
        }
    }

    println!("Build complete!");
    Ok(output_paths)
}

/// Copy binary from source to destination
fn copy_binary(source: &Path, dest: &Path) -> Result<(), BuildError> {
    if !source.exists() {
        return Err(BuildError::BinaryNotFound(source.to_path_buf()));
    }

    fs::copy(source, dest).map_err(|e| BuildError::CopyFailed {
        from: source.to_path_buf(),
        to: dest.to_path_buf(),
        source: e,
    })?;

    Ok(())
}

/// Check if cross is available
fn check_cross_available() -> Result<(), BuildError> {
    Command::new("cross")
        .arg("--version")
        .output()
        .map_err(|_| BuildError::CrossNotFound)?;
    Ok(())
}

/// Find the workspace root by looking for Cargo.toml with [workspace]
fn find_workspace_root() -> Result<PathBuf, BuildError> {
    // Try using CARGO_MANIFEST_DIR if available (set during cargo run)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(manifest_dir);
        // graft-builder is in crates/graft-builder, so workspace is ../..
        if let Some(workspace) = manifest_path.parent().and_then(|p| p.parent()) {
            if workspace.join("Cargo.toml").exists() {
                return Ok(workspace.to_path_buf());
            }
        }
    }

    // Fallback: use cargo locate-project
    let output = Command::new("cargo")
        .args(["locate-project", "--workspace", "--message-format=plain"])
        .output()
        .map_err(|_| BuildError::WorkspaceNotFound)?;

    if !output.status.success() {
        return Err(BuildError::WorkspaceNotFound);
    }

    let path_str = String::from_utf8_lossy(&output.stdout);
    let cargo_toml = PathBuf::from(path_str.trim());

    cargo_toml
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or(BuildError::WorkspaceNotFound)
}

/// Run cargo build for graft-gui with embedded_patch feature (native build)
fn run_cargo_build(workspace_root: &Path, archive_path: &Path) -> Result<(), BuildError> {
    let output = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--package",
            "graft-gui",
            "--features",
            "embedded_patch",
        ])
        .env("GRAFT_PATCH_ARCHIVE", archive_path)
        .current_dir(workspace_root)
        .output()
        .map_err(|e| BuildError::CargoBuildFailed {
            exit_code: None,
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(BuildError::CargoBuildFailed {
            exit_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

/// Run cross build for graft-gui with embedded_patch feature (cross-compilation)
fn run_cross_build(
    workspace_root: &Path,
    archive_path: &Path,
    target: &Target,
) -> Result<(), BuildError> {
    let output = Command::new("cross")
        .args([
            "build",
            "--release",
            "--package",
            "graft-gui",
            "--features",
            "embedded_patch",
            "--target",
            target.triple,
        ])
        .env("GRAFT_PATCH_ARCHIVE", archive_path)
        .current_dir(workspace_root)
        .output()
        .map_err(|e| BuildError::CargoBuildFailed {
            exit_code: None,
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(BuildError::CargoBuildFailed {
            exit_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

/// Get the platform-appropriate binary name (for native builds)
fn get_binary_name(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    }
}

/// Get the path to the release binary
fn get_release_binary_path(workspace_root: &Path, target: Option<&Target>) -> PathBuf {
    match target {
        Some(t) => {
            let binary_name = format!("graft-gui{}", t.binary_suffix);
            workspace_root
                .join("target")
                .join(t.triple)
                .join("release")
                .join(binary_name)
        }
        None => {
            let binary_name = if cfg!(target_os = "windows") {
                "graft-gui.exe"
            } else {
                "graft-gui"
            };
            workspace_root.join("target/release").join(binary_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_binary_name_adds_exe_on_windows() {
        let name = get_binary_name("patcher");
        if cfg!(target_os = "windows") {
            assert_eq!(name, "patcher.exe");
        } else {
            assert_eq!(name, "patcher");
        }
    }

    #[test]
    fn find_workspace_root_works() {
        // This test only works when running via cargo test
        let result = find_workspace_root();
        assert!(result.is_ok());
        let root = result.unwrap();
        assert!(root.join("Cargo.toml").exists());
        assert!(root.join("crates/graft-builder").exists());
    }
}
