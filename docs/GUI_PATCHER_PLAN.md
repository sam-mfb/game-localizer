# Plan: GUI Patcher Builder

## Overview

Create a tool that takes a patch directory and produces self-contained GUI executables for macOS, Windows, and Linux.

**Workflow:**
1. `game-localizer patch create ...` - Create patch (existing)
2. `game-localizer patch apply ...` - Test patch (existing)
3. `patch-gui-builder build <patch-dir> -o dist/` - Build GUI executables (new)

**User choices:**
- GUI framework: egui/eframe (pure Rust, ~5MB binaries)
- Build approach: Separate `patch-gui-builder` tool
- Cross-platform: Local cross-compile using `cross` (Docker)

## Project Structure

Convert to Cargo workspace:

```
game-localizer/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── patch-core/               # Shared library (extracted)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── patch/            # apply.rs, verify.rs, mod.rs
│   │       └── utils/            # manifest.rs, diff.rs, hash.rs, file_ops.rs
│   │
│   ├── game-localizer/           # Existing CLI (moved)
│   │   └── src/
│   │       ├── main.rs
│   │       └── commands/
│   │
│   ├── patch-gui-builder/        # Builder tool (new)
│   │   └── src/
│   │       ├── main.rs           # CLI: build subcommand
│   │       ├── builder.rs        # Orchestrates build process
│   │       ├── archive.rs        # Creates tar.gz of patch
│   │       └── template.rs       # Generates Rust project
│   │
│   └── patcher-gui/              # GUI app template (new)
│       └── src/
│           ├── main.rs           # Entry point with embedded data
│           └── app.rs            # egui application
```

## Embedding Strategy

Embed patch as compressed tar archive:

```rust
// Generated in patcher-gui/src/main.rs
const PATCH_DATA: &[u8] = include_bytes!("../patch_data.tar.gz");

fn main() -> eframe::Result<()> {
    patcher_gui::run(PATCH_DATA)
}
```

At runtime: extract to temp dir, load manifest, apply patch.

## GUI App Design

**States:**
1. **Welcome** - Show patch info, "Select Folder" button
2. **FolderSelected** - Show path, "Apply Patch" button
3. **Applying** - Progress bar, current file
4. **Success** - Green checkmark, done message
5. **Error** - Red X, error details, "Show Details" expander

**Key dependencies:**
- `eframe` / `egui` - GUI framework
- `rfd` - Native file dialogs
- `tar` / `flate2` - Archive extraction
- `patch-core` - Patching logic

## CLI Interface

```
patch-gui-builder build <PATCH_DIR> [OPTIONS]

OPTIONS:
    -o, --output <DIR>       Output directory [default: ./dist]
    -n, --name <NAME>        Patcher name [default: from manifest]
    --targets <TARGETS>      linux,windows,macos [default: linux,windows]
```

## Implementation Phases

### Phase 1: Workspace Restructure
- Convert to Cargo workspace
- Create `crates/patch-core/` - extract `src/patch/` and `src/utils/`
- Move CLI to `crates/game-localizer/`
- Update imports, verify tests pass

### Phase 2: GUI Runtime (`patcher-gui`)
- Create egui app with state machine
- Implement patch extraction from embedded tar.gz
- Folder selection with `rfd`
- Progress display during apply
- Success/error views

### Phase 3: Builder Tool (`patch-gui-builder`)
- CLI with clap
- Archive creation (tar.gz patch data)
- Template project generation
- Local `cargo build --release` integration

### Phase 4: Cross-Compilation
- Add Cross.toml configuration
- Build orchestration for multiple targets
- Copy artifacts to output directory
- Document macOS limitations (requires osxcross)

### Phase 5: Polish
- Customization options (name, window title)
- Better error messages
- Documentation

## Key Files to Modify/Create

**Extract to patch-core:**
- `src/patch/mod.rs` → `crates/patch-core/src/patch/mod.rs`
- `src/patch/apply.rs` → `crates/patch-core/src/patch/apply.rs`
- `src/patch/verify.rs` → `crates/patch-core/src/patch/verify.rs`
- `src/utils/*` → `crates/patch-core/src/utils/*`

**New files:**
- `Cargo.toml` - workspace root
- `crates/patch-core/Cargo.toml`
- `crates/game-localizer/Cargo.toml`
- `crates/patch-gui-builder/src/main.rs`
- `crates/patch-gui-builder/src/builder.rs`
- `crates/patch-gui-builder/src/archive.rs`
- `crates/patcher-gui/src/main.rs`
- `crates/patcher-gui/src/app.rs`

## Dependencies

**patch-core:**
```toml
bsdiff = "0.2.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
```

**patcher-gui:**
```toml
eframe = "0.29"
rfd = "0.15"
tar = "0.4"
flate2 = "1.0"
tempfile = "3"
patch-core = { path = "../patch-core" }
```

**patch-gui-builder:**
```toml
clap = { version = "4", features = ["derive"] }
tar = "0.4"
flate2 = "1.0"
tempfile = "3"
patch-core = { path = "../patch-core" }
```

## Cross-Compilation Targets

| Target | Output Name | Notes |
|--------|-------------|-------|
| x86_64-unknown-linux-gnu | patcher-linux | Default |
| x86_64-pc-windows-gnu | patcher-windows.exe | Default |
| x86_64-apple-darwin | patcher-macos | Requires osxcross setup |

## Notes

- macOS cross-compilation from Linux is complex; may need to build on actual Mac or skip initially
- Windows builds from Linux work well with cross
- Consider ARM targets (aarch64) as future enhancement
