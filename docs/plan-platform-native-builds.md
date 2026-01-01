# Plan: Platform-Native Patcher Builds

## Overview

Make graft produce proper platform-native applications with customizable metadata:
- Window title configurable via manifest
- Custom icons for Windows and macOS
- Windows: GUI apps with embedded icons
- macOS: .app bundles with icons
- Rename `patcher` subcommand to `build`
- Auto-demo mode for bare stubs

---

## A. Title in Manifest

**Goal:** Allow customizing the patcher window title.

### Changes to `graft patch` command

Add optional `--title` parameter:
```bash
graft patch --old v1/ --new v2/ --output patch/ --title "My Game Patcher"
```

### Manifest changes (`patch/manifest.json`)

```json
{
  "version": "1.0.0",
  "title": "My Game Patcher",
  "entries": [...]
}
```

- If `--title` not provided, default to "Graft Patcher"
- User can edit manifest.json directly to change title

### GUI changes (`graft-gui`)

- Read title from embedded patch manifest
- Pass to `eframe::run_native(title, ...)`

---

## B. Asset Folder with Default Icons

**Goal:** Provide default icons that users can replace.

### Folder structure

```
patch/
  manifest.json
  diffs/
  files/
  .graft_assets/
    icon.png          # High-res source (1024x1024 recommended)
```

### Default icons

Graft embeds a generic default icon. When `graft patch` creates a patch:
1. Creates `.graft_assets/` folder
2. Copies default `icon.png` into it

**User workflow:**
- Replace `.graft_assets/icon.png` with custom icon (1024x1024 PNG recommended)
- Run `graft build` - icons are embedded/bundled automatically

### Icon file to create (manual step)

Create a simple generic graft icon:
- `icon.png` - 1024x1024 PNG (source for both platforms)

Location in graft source:
```
crates/graft/assets/
  default_icon.png
```

---

## C. Windows Icon Embedding

**Goal:** Embed icons in Windows .exe using pure Rust (cross-platform).

### Crate: [editpe](https://crates.io/crates/editpe)

Cross-platform PE resource editor. Works on Linux/macOS to modify Windows executables.

```rust
use editpe::Image;

let mut image = Image::parse(&exe_bytes)?;
let mut resources = image.resource_directory().cloned().unwrap_or_default();
resources.set_main_icon_file(&icon_path)?;  // Accepts PNG, auto-converts
image.set_resource_directory(resources)?;
let modified = image.write()?;
```

### Add to `crates/graft/Cargo.toml`

```toml
editpe = { version = "0.2", default-features = false, features = ["std", "images"] }
```

---

## D. macOS .app Bundle with Icon

**Goal:** Create .app bundles with custom icons.

### Bundle structure

```
MyPatcher.app/
  Contents/
    MacOS/
      MyPatcher
    Info.plist
    Resources/
      AppIcon.icns
```

### Icon handling

Convert `.graft_assets/icon.png` to .icns format using the [`icns`](https://crates.io/crates/icns) crate.

```rust
use icns::{IconFamily, Image};

let png_data = fs::read("icon.png")?;
let image = Image::read_png(&png_data)?;
let mut icon_family = IconFamily::new();
icon_family.add_icon(&image)?;
icon_family.write(File::create("AppIcon.icns")?)?;
```

### Add to `crates/graft/Cargo.toml`

```toml
icns = "0.3"
```

### Info.plist template

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "...">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>{app_name}</string>
    <key>CFBundleIdentifier</key>
    <string>com.graft.patcher.{app_name}</string>
    <key>CFBundleName</key>
    <string>{title}</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
```

---

## E. Auto-Demo Mode for Bare Stubs

**Goal:** Run demo mode automatically when no patch data embedded.

### Logic in `graft-gui/src/main.rs`

```rust
fn main() {
    match self_read::read_appended_data() {
        Ok(patch_data) => run_patcher(patch_data),
        Err(_) => run_demo(),
    }
}
```

- Remove `demo` subcommand from CLI
- Keep `headless` subcommand for scripted use

---

## F. Windows GUI Subsystem

**Goal:** No console window on Windows.

### Add to `crates/graft-gui/src/main.rs`

```rust
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
```

---

## G. Rename `patcher` to `build`

### Changes

- Rename `commands/patcher_create.rs` → `commands/build.rs`
- Update CLI enum: `Patcher` → `Build`
- Update usage: `graft build --patch patch/ --target windows-x64 --output MyPatcher.exe`

---

## H. Headless Mode Documentation

Add to README:

```markdown
### Headless Mode (CLI)

The `headless` subcommand applies patches without GUI:

    ./patcher headless --target /path/to/game

**Windows Note:** stdout/stderr are not connected when double-clicked.
For scripted use, run from terminal or use the main `graft` CLI.

**macOS Note:** For .app bundles, run the binary inside:

    ./MyPatcher.app/Contents/MacOS/MyPatcher headless --target /path/to/game
```

---

## Files to Modify/Create

| File | Action |
|------|--------|
| `crates/graft/Cargo.toml` | Add `editpe`, `icns` dependencies |
| `crates/graft/src/main.rs` | Rename patcher → build, add --title flag to patch |
| `crates/graft/src/commands/patch_create.rs` | Add title to manifest, create .graft_assets |
| `crates/graft/src/commands/patcher_create.rs` | Rename to build.rs, add icon/bundle logic |
| `crates/graft/src/commands/build/windows_icon.rs` | New: editpe icon embedding |
| `crates/graft/src/commands/build/macos_bundle.rs` | New: .app bundle + icns creation |
| `crates/graft/assets/default_icon.png` | New: default icon (manual creation) |
| `crates/graft-core/src/manifest.rs` | Add `title` field to Manifest struct |
| `crates/graft-gui/src/main.rs` | Add windows_subsystem, auto-demo logic |
| `crates/graft-gui/src/gui.rs` | Use title from manifest |
| `README.md` | Document headless caveats |

---

## Output Conventions

| Target | Output | Icon Source |
|--------|--------|-------------|
| `windows-x64` | `MyPatcher.exe` | `.graft_assets/icon.png` → embedded via editpe |
| `linux-x64` | `MyPatcher` | (no icon) |
| `macos-arm64` | `MyPatcher.app/` | `.graft_assets/icon.png` → .icns in bundle |
| `macos-x64` | `MyPatcher.app/` | `.graft_assets/icon.png` → .icns in bundle |

---

## Dependencies to Add

```toml
# crates/graft/Cargo.toml
editpe = { version = "0.2", default-features = false, features = ["std", "images"] }
icns = "0.3"
```

---

## User Workflow Summary

1. `graft patch --old v1/ --new v2/ --output patch/ --title "My Patcher"`
2. (Optional) Replace `patch/.graft_assets/icon.png` with custom 1024x1024 PNG
3. `graft build --patch patch/ --target windows-x64 --output MyPatcher.exe`
4. Distribute MyPatcher.exe (or .app for macOS)

---

## Implementation Phases

### Phase 1: Core Restructuring
- [ ] Rename `patcher` → `build` command (section G)
- [ ] Add `--title` to `graft patch`, update manifest (section A)
- [ ] Auto-demo mode for bare stubs (section E)
- [ ] Windows GUI subsystem attribute (section F)
- [ ] Headless mode documentation (section H)

### Phase 2: macOS Bundles
- [ ] Create `.graft_assets/` folder with default icon (section B)
- [ ] macOS .app bundle creation (section D)
- [ ] Icon embedding via `icns` crate

### Phase 3: Windows Icons
- [ ] Windows icon embedding via `editpe` crate (section C)
- [ ] Test cross-platform PE modification

---

## Future Considerations (Not in Scope)

- Code signing / notarization (separate CI workflow)
- Linux .desktop file generation
