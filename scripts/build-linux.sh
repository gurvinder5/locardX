#!/usr/bin/env bash
# ============================================================================
# LocardX Linux Build Script (Native / WSL2)
# Compiles frontend and generates native Linux .deb and .AppImage bundles.
# ============================================================================

set -euo pipefail

echo "======================================================="
echo " [LocardX] Building Linux Desktop Packages (.deb / .AppImage)"
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

echo "[1/4] Checking Linux system libraries..."
REQUIRED_PKGS=(
    "libwebkit2gtk-4.1-dev"
    "build-essential"
    "curl"
    "wget"
    "file"
    "libssl-dev"
    "libgtk-3-dev"
    "libayatana-appindicator3-dev"
    "librsvg2-dev"
)

if command -v dpkg-query &> /dev/null; then
    MISSING_PKGS=()
    for pkg in "${REQUIRED_PKGS[@]}"; do
        if ! dpkg-query -W -f='${Status}' "$pkg" 2>/dev/null | grep -q "ok installed"; then
            MISSING_PKGS+=("$pkg")
        fi
    done

    if [ ${#MISSING_PKGS[@]} -ne 0 ]; then
        echo "[WARNING] The following system packages appear to be missing:"
        echo "  ${MISSING_PKGS[*]}"
        echo "You can install them via:"
        echo "  sudo apt-get update && sudo apt-get install -y ${MISSING_PKGS[*]}"
        echo ""
        read -p "Continue anyway? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
fi

echo "[2/4] Installing dependencies..."
npm install
(cd frontend && npm install)

echo "[3/4] Building frontend assets..."
npm run build --prefix frontend

echo "[4/4] Building Tauri Linux application bundles..."
npx tauri build

echo "======================================================="
echo " [SUCCESS] Linux bundles created successfully!"
echo " Debian package (.deb):  src-tauri/target/release/bundle/deb/"
echo " AppImage package:       src-tauri/target/release/bundle/appimage/"
echo " Binary executable:      src-tauri/target/release/locardx-desktop"
echo "======================================================="
