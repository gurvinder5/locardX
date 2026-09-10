# Development Environment Setup

This guide details the prerequisites and step-by-step instructions for setting up a local development environment for LocardX.

---

## 1. Prerequisites

### Rust
LocardX requires the stable Rust toolchain (version 1.75 or newer).
- Install via [rustup](https://rustup.rs):
  ```bash
  # Windows (PowerShell) or Linux/macOS
  rustup default stable
  rustup component add clippy rustfmt
  ```

### Node.js & npm
- Install Node.js 18 LTS or 20+ from [nodejs.org](https://nodejs.org/).
- Confirm installation:
  ```bash
  node --version
  npm --version
  ```

### Platform Prerequisites

#### Windows
- **C++ Build Tools**: Install the *Desktop development with C++* workload via the [Visual Studio Build Tools Installer](https://visualstudio.microsoft.com/visual-cpp-build-tools/).
- **WebView2**: Built into Windows 10/11; if missing, install the [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

#### Linux (Debian / Ubuntu)
```bash
sudo apt update
sudo apt install -y build-essential curl wget file libssl-dev libgtk-3-dev \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
```

---

## 2. Setting Up the Repository

```bash
# Clone the repository
git clone https://github.com/gurvinder5/locardX.git
cd locardX

# Copy environment template
cp .env.example .env

# Install frontend packages
cd frontend
npm install
cd ..
```

---

## 3. Running LocardX in Development

To start the Vite development server and the Tauri desktop window simultaneously:

```bash
npm run tauri dev
```
*(or from root, once configured)*

---

## 4. Helpful Local Scripts

Windows batch scripts are provided in `scripts/`:
- `scripts/setup-dev.bat`: Validates toolchain presence and sets up dependencies.
- `scripts/build.bat`: Builds both backend and frontend.
- `scripts/test.bat`: Executes all workspace test suites.
- `scripts/lint.bat`: Runs formatting and clippy checks.
