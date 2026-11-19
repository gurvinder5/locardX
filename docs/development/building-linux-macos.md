# Building LocardX for Linux and macOS

LocardX is built on **Tauri 2** (Rust backend + React/Vite frontend). Because desktop GUI applications bind directly to host system rendering and display frameworks (WebKitGTK on Linux, Apple WebKit/Cocoa on macOS), they cannot be cross-compiled directly from a standard Windows host using only `cargo` without the respective OS system libraries and toolchains.

This guide provides the three official ways to generate native **Linux** (`.deb`, `.AppImage`) and **macOS** (`.dmg`, `.app`) binaries.

---

## Method 1: Automated GitHub Actions (Recommended & Easiest)

We have configured an automated Build, Test & Deploy CI/CD workflow in [`.github/workflows/release.yml`](../../.github/workflows/release.yml) that automatically runs backend/frontend test suites, builds native packages on GitHub's cloud runners, and deploys releases for:
- **Linux x86_64**: `.deb` and `.AppImage` (on Ubuntu 22.04)
- **Windows x86_64**: `.msi` and `.exe` (on Windows runner)

*(Note: macOS runners have been removed from GitHub Actions to reduce build minutes and runner constraints; macOS bundles can still be built locally using `scripts/build-macos.sh` as documented below in Method 4).*

### Option A: Trigger On-Demand (No Tag Needed)
1. Push your code to your GitHub repository:
   ```bash
   git push origin <your-branch>
   ```
2. Navigate to your repository on GitHub.
3. Click on the **Actions** tab.
4. Select the **Build, Test & Deploy** workflow in the left sidebar.
5. Click **Run workflow** -> Select your branch -> Click **Run workflow**.
6. When the run finishes, scroll down to the **Artifacts** section at the bottom of the run page:
   - `locardx-linux-x86_64.zip` (contains `.deb` and `.AppImage`)
   - `locardx-windows-x86_64.zip` (contains `.msi` and `.exe`)
   - Click any artifact to download it immediately.

### Option B: Create an Official Release Tag
Pushing a version tag automatically compiles all platforms and publishes a GitHub Release with the installer assets attached:
```bash
git tag v0.1.0
git push origin v0.1.0
```

---

## Method 2: Building Linux from Windows using Docker

If you have Docker Desktop running on Windows, you can compile native Linux packages directly without a Linux VM or secondary machine.

1. Ensure Docker Desktop is running.
2. From the repository root, run:
   ```bat
   scripts\build-linux-docker.bat
   ```
3. The build script builds the container using [`Dockerfile.build-linux`](../../Dockerfile.build-linux) and outputs the packages to:
   ```
   dist\linux\
   ├── deb\
   │   └── locardx_0.1.0_amd64.deb
   └── appimage\
       └── locardx_0.1.0_amd64.AppImage
   ```

---

## Method 3: Building Natively on Linux (Ubuntu / Debian / WSL2)

To compile LocardX on a native Linux installation or within WSL2:

### 1. Install System Dependencies
```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  squashfs-tools
```

### 2. Install Rust and Node.js
```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Node.js 20+
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs
```

### 3. Run the Build Script
```bash
chmod +x scripts/build-linux.sh
./scripts/build-linux.sh
```
Bundles will be generated under `src-tauri/target/release/bundle/`.

---

## Method 4: Building Natively on macOS (Apple Silicon / Intel)

Due to Apple's licensing and ecosystem constraints, macOS `.dmg` and `.app` bundles must be built on macOS (either locally or through GitHub Actions).

### 1. Prerequisites on macOS
1. Install **Xcode Command Line Tools**:
   ```bash
   xcode-select --install
   ```
2. Install **Node.js 20+** and **Rust**:
   ```bash
   # Homebrew
   brew install node

   # Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```

### 2. Run the Build Script
```bash
chmod +x scripts/build-macos.sh
./scripts/build-macos.sh
```

To specifically target Apple Silicon or Intel:
```bash
# Apple Silicon (M1/M2/M3/M4)
./scripts/build-macos.sh aarch64-apple-darwin

# Intel Macs
./scripts/build-macos.sh x86_64-apple-darwin
```

Bundles will be generated under:
- `src-tauri/target/release/bundle/dmg/*.dmg`
- `src-tauri/target/release/bundle/macos/*.app`
