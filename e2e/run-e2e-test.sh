#!/bin/bash
set -e

# E2E test script for graft patcher workflow
# Currently supports Linux only

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WORK_DIR="$SCRIPT_DIR/.e2e-work"

# Cleanup on exit
cleanup() {
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

echo "=== Graft E2E Test ==="
echo ""

# Detect platform
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64)  TARGET="linux-x64" ;;
    aarch64) TARGET="linux-arm64" ;;
    *)       echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac
echo "Platform: $TARGET"
echo ""

# Step 1: Build graft-gui stub
echo "Step 1: Building graft-gui stub..."
cargo build --release -p graft-gui --manifest-path "$REPO_ROOT/Cargo.toml"

# Step 2: Build graft CLI
echo "Step 2: Building graft CLI..."
cargo build --release -p graft --manifest-path "$REPO_ROOT/Cargo.toml"

# Step 3: Setup work directory
echo "Step 3: Setting up test environment..."
rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR/stubs"
mkdir -p "$WORK_DIR/patch"
mkdir -p "$WORK_DIR/output"

# Copy stub with correct naming
cp "$REPO_ROOT/target/release/graft-gui" "$WORK_DIR/stubs/graft-gui-stub-$TARGET"

GRAFT="$REPO_ROOT/target/release/graft"
PATCHER="$WORK_DIR/output/patcher-$TARGET"

# Step 4: Create patch
echo "Step 4: Creating patch..."
"$GRAFT" patch create \
    "$SCRIPT_DIR/exampleOrig" \
    "$SCRIPT_DIR/exampleTarget" \
    "$WORK_DIR/patch" \
    -v 1 \
    --title "E2E Test Patch"

# Step 5: Build patcher
echo "Step 5: Building patcher..."
"$GRAFT" build "$WORK_DIR/patch" -o "$WORK_DIR/output" --stub-dir "$WORK_DIR/stubs"

# Step 6: Prepare test target
echo "Step 6: Preparing test target..."
cp -r "$SCRIPT_DIR/exampleOrig" "$WORK_DIR/test-target"

# Step 7: Apply patch
echo "Step 7: Applying patch..."
"$PATCHER" headless apply "$WORK_DIR/test-target" -y

# Step 8: Verify patch applied
echo "Step 8: Verifying patch..."
# Compare test-target with exampleTarget (excluding .patch-backup)
DIFF_OUTPUT=$(diff -rq "$WORK_DIR/test-target" "$SCRIPT_DIR/exampleTarget" 2>&1 | grep -v ".patch-backup" || true)
if [ -z "$DIFF_OUTPUT" ]; then
    echo "  Patch applied correctly!"
else
    echo "  ERROR: Patch verification failed!"
    echo "$DIFF_OUTPUT"
    exit 1
fi

# Step 9: Rollback (answer 'y' to delete backup)
echo "Step 9: Rolling back..."
echo "y" | "$PATCHER" headless rollback "$WORK_DIR/test-target"

# Step 10: Verify rollback
echo "Step 10: Verifying rollback..."
DIFF_OUTPUT=$(diff -rq "$WORK_DIR/test-target" "$SCRIPT_DIR/exampleOrig" 2>&1 | grep -v ".patch-backup" || true)
if [ -z "$DIFF_OUTPUT" ]; then
    echo "  Rollback successful!"
else
    echo "  ERROR: Rollback verification failed!"
    echo "$DIFF_OUTPUT"
    exit 1
fi

# Step 11: Verify backup deleted
echo "Step 11: Verifying backup deletion..."
if [ ! -d "$WORK_DIR/test-target/.patch-backup" ]; then
    echo "  Backup deleted successfully!"
else
    echo "  ERROR: Backup directory still exists!"
    exit 1
fi

echo ""
echo "=== E2E Test PASSED ==="
