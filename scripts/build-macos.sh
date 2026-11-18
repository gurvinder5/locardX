#!/usr/bin/env bash
# ============================================================================
# LocardX macOS Build Script
# Compiles frontend and produces native macOS .dmg and .app application bundles.
# Supports both Apple Silicon (aarch64) and Intel (x86_64), or Universal.
# ============================================================================

set -euo pipefail

echo "======================================================="
echo " [LocardX] Building macOS Application (.dmg / .app)"
echo "======================================================="

# Verify essential build dependencies
MISSING_DEPS=()
for cmd in cargo rustc node npm; do
    if ! command -v "$cmd" &> /dev/null; then
        MISSING_DEPS+=("$cmd")
    fi
done

if [ ${#MISSING_DEPS[@]} -ne 0 ]; then
    echo "[ERROR] Missing required tools: ${MISSING_DEPS[*]}"
    echo "Please install Rust (https://rustup.rs) and Node.js (https://nodejs.org)."
    exit 1
fi

# Check Xcode Command Line Tools
if ! xcode-select -p &> /dev/null; then
    echo "[ERROR] Xcode Command Line Tools are not installed."
    echo "Run: xcode-select --install"
    exit 1
fi

TARGET="${1:-}"

echo "[1/4] Installing dependencies..."
npm install
(cd frontend && npm install)

echo "[2/4] Building frontend distribution..."
npm run build --prefix frontend

echo "[3/4] Building Tauri macOS bundles..."
if [ -n "$TARGET" ]; then
    echo "Target architecture requested: $TARGET"
    rustup target add "$TARGET" || true
    npx tauri build --target "$TARGET"
else
    # Build for native host architecture (default)
    npx tauri build
fi

echo "======================================================="
echo " [SUCCESS] macOS bundle created successfully!"
echo " Disk Image (.dmg):   src-tauri/target/release/bundle/dmg/"
echo " Application (.app):  src-tauri/target/release/bundle/macos/"
echo "======================================================="
