# game-localizer

Tools for creating and applying binary patches to game files.

## Installation

```
cargo install --path .
```

## Commands

### Diff

Create a diff:
```
game-localizer diff create <original> <modified> <diff-output>
```

Apply a diff:
```
game-localizer diff apply <original> <diff-file> <output>
```

### Hash

Calculate SHA-256 hash of a file:
```
game-localizer hash calculate <file>
```

Compare two files by hash:
```
game-localizer hash compare <file1> <file2>
```

Check if a file matches an expected hash:
```
game-localizer hash check <hash> <file>
```

### Patch

Create a patch from two directories:
```
game-localizer patch create <original-dir> <modified-dir> <patch-output-dir>
```

This compares the directories and generates:
- `manifest.json` - lists all operations with SHA-256 hashes
- `diffs/` - binary diffs for modified files
- `files/` - copies of newly added files

Apply a patch to a target directory:
```
game-localizer patch apply <target-dir> <patch-dir>
```

This will:
1. Validate all files exist and match expected hashes
2. Backup modified/deleted files to `.patch-backup/`
3. Apply all changes (patch, add, delete)
4. Verify results match expected hashes
5. Rollback automatically on any failure
