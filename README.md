# LocardX

> A high-assurance desktop application for forensic data recovery, file carving, and cryptographically verified data sanitization.

---

## Overview

**LocardX** is an open, modern desktop forensic and data sanitization platform built with Rust, Tauri, and React. Named in homage to Edmond Locard's exchange principle—*"every contact leaves a trace"*—LocardX addresses both sides of digital storage lifecycle management:

1. **Forensic Recovery**: Identifying, carving, reconstructing, and extracting forensically valuable artifacts and lost files from damaged, formatted, or unallocated storage spaces without altering underlying evidence.
2. **Secure Sanitization**: Permanently and irreversibly purging sensitive data from individual files, directories, free space, or entire physical storage devices with verifiable cryptographic proof of erasure.

LocardX combines low-level system performance, strict memory safety, tamper-evident audit logging, and an intuitive modern interface.

---

## Problem Statement

Modern forensic investigators and security administrators face significant challenges when managing sensitive storage devices:

- **Evidence Contamination**: Standard operating system file access frequently writes access timestamps, updates metadata, or triggers background indexing, contaminating forensic evidence.
- **Incomplete Sanitization**: Standard OS deletions only unbind directory pointers; modern solid-state drives (SSDs, NVMe) and flash media utilize wear leveling and over-provisioning that render naive single-pass file wiping ineffective.
- **Lack of Verifiability**: Existing tools rarely provide tamper-evident cryptographic chains connecting the sanitization routine to an immutable audit record and verifiable certificate.
- **Fragmented Tooling**: Analysts must navigate disjointed CLI utilities for acquisition, separate tools for carving, and distinct third-party utilities for drive wiping.

LocardX integrates forensic extraction and secure sanitization under a unified, safety-first desktop architecture with end-to-end cryptographic verification.

---

## Core Modules

LocardX is organized into three primary operational modules, backed by shared platform services:

### 1. Secure Drive Eraser
- **Scope**: Whole-disk sanitization across physical drives, external storage, and logical block devices.
- **Capabilities (Planned / In Development)**:
  - NIST SP 800-88 Rev. 1 compliant sanitization methods (Clear, Purge).
  - Multi-pass pseudorandom pattern overwriting (DoD 5220.22-M, Gutmann algorithms).
  - Hardware-level sanitization command dispatch (ATA Secure Erase, NVMe Sanitize / Format).
  - Post-erasure verification scanning and cryptographic hash sampling.
  - Multi-tiered safety interlocks preventing accidental destruction of OS/system drives.

### 2. Secure File & Folder Eraser
- **Scope**: Targeted, surgical file and directory destruction without affecting surrounding data.
- **Capabilities (Planned / In Development)**:
  - Multi-pass data block overwriting with cryptographically secure pseudorandom numbers (CSPRNG).
  - Filesystem metadata sanitization (renaming, inode/MFT attribute zeroing, timestamp manipulation).
  - Alternate Data Streams (ADS) and extended attributes (xattr) clearing.
  - Slack space and free space sanitization.
  - Batch target selection and directory tree recursion.

### 3. Advanced File Carving & Recovery
- **Scope**: Forensically sound, read-only file extraction and artifact reconstruction.
- **Capabilities (Planned / In Development)**:
  - Signature-based file carving (headers, footers, magic bytes for JPEG, PNG, PDF, ZIP, etc.).
  - The Sleuth Kit (TSK) integration for structured filesystem parsing (NTFS, FAT32, exFAT, ext4).
  - Smart fragmentation reconstruction and bifragment gap analysis.
  - Recovery confidence scoring based on entropy, structure validation, and metadata consistency.
  - Read-only enforcement ensuring source evidence integrity is preserved.

---

## Architecture

LocardX enforces strict separation of concerns between presentation, desktop orchestration, core engines, and shared services:

```text
┌─────────────────────────────────────────────────────────────┐
│                 React + TypeScript Frontend                 │
│         (Tailwind CSS, Zustand, UI Components)              │
└──────────────────────────────┬──────────────────────────────┘
                               │ Tauri IPC / Commands
┌──────────────────────────────▼──────────────────────────────┐
│                    Tauri Desktop Shell                      │
│            (Window management, system events)               │
└──────────────────────────────┬──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│                   Domain Engine Layer                       │
│  ┌──────────────────┐ ┌───────────────────┐ ┌─────────────┐ │
│  │   Drive Eraser   │ │    File Eraser    │ │  Recovery   │ │
│  │      Engine      │ │      Engine       │ │   Engine    │ │
│  └────────┬─────────┘ └─────────┬─────────┘ └──────┬──────┘ │
└───────────┼─────────────────────┼──────────────────┼────────┘
            │                     │                  │
┌───────────▼─────────────────────▼──────────────────▼────────┐
│                   Shared Platform Services                  │
│  ┌───────────────────────┐   ┌───────────────────────────┐  │
│  │ Authentication & RBAC │   │ Device & FS Detection     │  │
│  ├───────────────────────┤   ├───────────────────────────┤  │
│  │ Operation Manager     │   │ Verification Engine       │  │
│  ├───────────────────────┤   ├───────────────────────────┤  │
│  │ Tamper-Evident Audit  │   │ SQLite Persistence        │  │
│  ├───────────────────────┤   ├───────────────────────────┤  │
│  │ Reporting Subsystem   │   │ Security & Integrity Core │  │
│  └───────────────────────┘   └───────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Technology Stack

| Layer | Technologies |
| :--- | :--- |
| **Desktop Shell** | [Tauri](https://tauri.app/) (Lightweight, secure system webview integration) |
| **Frontend UI** | [React 18](https://react.dev/), [TypeScript](https://www.typescriptlang.org/), [Vite](https://vitejs.dev/), [Tailwind CSS](https://tailwindcss.com/) |
| **State & Icons** | [Zustand](https://github.com/pmndrs/zustand), [Lucide React](https://lucide.dev/) |
| **Backend Core** | [Rust](https://www.rust-lang.org/) (2021 Edition, memory-safe system programming) |
| **Async Runtime** | [Tokio](https://tokio.rs/) |
| **Data Persistence** | [SQLite](https://www.sqlite.org/) via [rusqlite](https://github.com/rusqlite/rusqlite) |
| **Serialization** | [Serde](https://serde.rs/) (JSON, binary serialization) |
| **Observability** | [tracing](https://github.com/tokio-rs/tracing) and structured diagnostic logging |
| **Forensic Parser** | [The Sleuth Kit (TSK)](https://www.sleuthkit.org/) + custom Rust carving engine |
| **Cryptographic Integrity** | SHA-256 hash chains, CSPRNG pattern generation |

---

## Project Structure

```text
LocardX/
├── .github/                   # GitHub Actions CI/Security workflows and issue templates
│   ├── ISSUE_TEMPLATE/        # Standardized bug, feature, and security issue forms
│   ├── workflows/             # ci.yml and security.yml pipelines
│   └── pull_request_template.md
├── crates/                    # Modular Rust backend crates
│   ├── audit/                 # Tamper-evident hash-chain audit logging
│   ├── auth/                  # Local user authentication, roles, and sessions
│   ├── common/                # Shared domain types, error models, and utilities
│   ├── database/              # SQLite connection pool and schema migrations
│   ├── device-manager/        # OS block device enumeration and geometry detection
│   ├── drive-eraser/          # Whole-disk sanitization and overwriting engine
│   ├── file-eraser/           # Targeted file/directory overwriting and metadata wiping
│   ├── operation-manager/     # Job scheduling, progress tracking, and cancellation
│   ├── recovery-engine/       # TSK integration, signature carving, and reconstruction
│   ├── reporting/             # Audit, forensic, and sanitization report generation
│   ├── security/              # Cryptographic primitives, integrity checks, and safety guards
│   └── verification/          # Pattern verification and hash-based integrity checkers
├── docs/                      # Architectural, development, security, and user documentation
│   ├── api/                   # IPC and crate interface specifications
│   ├── architecture/          # High-level architecture, data flows, and module designs
│   ├── development/           # Setup, contributor guidelines, and testing guides
│   ├── erasure/               # Sanitization standards and verification methodologies
│   ├── forensic/              # Carving algorithms, TSK integration, and evidence integrity
│   ├── reporting/             # Report templates and certificate schemas
│   ├── security/              # Threat models and security controls
│   └── user/                  # User manual and installation instructions
├── frontend/                  # React + TypeScript + Vite frontend application
│   ├── src/
│   │   ├── app/               # Root routing, providers, and layout
│   │   ├── components/        # UI components (common, drive-eraser, file-eraser, recovery)
│   │   ├── hooks/             # Custom React hooks
│   │   ├── pages/             # Page views
│   │   ├── services/          # Tauri IPC client services
│   │   ├── stores/            # Zustand global stores
│   │   └── types/             # Frontend TypeScript interfaces
├── scripts/                   # Local automation and helper scripts
├── src-tauri/                 # Tauri desktop shell, capabilities, and command bindings
├── test-data/                 # Synthetic filesystem images and test assets
└── tests/                     # Integration and end-to-end test suites
```

---

## Development Setup

### Prerequisites

1. **Rust Toolchain**: Stable toolchain (Rust 1.75+ recommended)
   ```bash
   # Install via rustup: https://rustup.rs
   rustup default stable
   rustup component add clippy rustfmt
   ```
2. **Node.js**: Node.js 18 LTS or 20+ with `npm`
   ```bash
   node --version
   npm --version
   ```
3. **Platform Build Tools**:
   - **Windows**: [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC) and [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
   - **Linux**: Standard build tools and WebKitGTK libraries (`libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libssl-dev`, `libgtk-3-dev`)
   - **macOS**: Xcode Command Line Tools

### Setup Instructions

1. Clone the repository:
   ```bash
   git clone https://github.com/gurvinder5/locardX.git
   cd locardX
   ```

2. Copy the example environment configuration:
   ```bash
   cp .env.example .env
   ```

3. Install frontend dependencies:
   ```bash
   cd frontend
   npm install
   cd ..
   ```

---

## Building the Application

### Running in Development Mode
```bash
# From repository root (runs frontend dev server and launches Tauri window)
npm run tauri dev
```

### Building Release Packages
```bash
npm run tauri build
```
Compiled standalone binaries and platform installers are placed in `src-tauri/target/release/bundle/`.

---

## Testing

> [!WARNING]
> Testing destructive operations against physical non-test drives is strictly prohibited. Always use synthetic virtual disks or mock interfaces.

### Rust Unit & Integration Tests
```bash
cargo test --workspace
```

### Code Formatting & Linting
```bash
# Rust checks
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# Frontend checks
cd frontend
npm run lint
npm run typecheck
```

---

## Security Considerations

- **Destructive Operation Safety**: All sanitization routines implement multi-step confirmation gates and prevent execution against running operating system volumes.
- **Evidence Immutability**: Forensic recovery tools operate in strictly read-only mode with write-blocker detection where supported.
- **Audit Tamper-Evidence**: Operation history is recorded using an append-only SHA-256 hash chain stored in SQLite, detecting any retroactive log tampering.
- **Zero Hardcoded Secrets**: Secrets, keys, and tokens are never embedded in code or configuration.

For vulnerability disclosure protocols and lab safety procedures, refer to [SECURITY.md](SECURITY.md).

---

## Documentation

Full architectural, technical, and operational documentation is available in the [`docs/`](docs/) directory:

- [System Architecture](docs/architecture/system-architecture.md)
- [Module Architecture](docs/architecture/module-architecture.md)
- [Security Controls & Threat Model](docs/security/security-controls.md)
- [Development Setup](docs/development/development-setup.md)
- [Contributor & Agent Rules](AGENT_RULES.md)
- [Testing Strategy](docs/development/testing.md)

---

## Current Development Status

The repository is currently in the **foundation and scaffolding phase**.

| Component / Module | Status | Notes |
| :--- | :--- | :--- |
| **Project Structure & Scaffolding** | **Active / Complete** | Complete workspace crate structure, frontend scaffold, and documentation layout. |
| **GitHub Workflows & CI** | **Active / Complete** | CI/Security pipelines, templates, and contributor rules configured. |
| **Authentication & RBAC** | *Under Development* | Crate structure and session interfaces defined; persistence in progress. |
| **Device Detection** | *Under Development* | Block device enumeration interfaces created. |
| **Operation Manager** | *Under Development* | Job queue and cancellation token scaffolding. |
| **Audit & Hash-Chain** | *Prototype* | SHA-256 hash chaining logic designed. |
| **Secure Drive Eraser** | *Planned / Skeleton* | Overwrite algorithms and disk I/O to be implemented in subsequent phases. |
| **Secure File Eraser** | *Planned / Skeleton* | File overwriting and metadata stripping to be implemented in subsequent phases. |
| **Forensic Recovery Engine** | *Planned / Skeleton* | TSK bindings and file carver to be implemented in subsequent phases. |
| **Frontend User Interface** | *Prototype* | React views, layout, and component frames created. |

---

## Future Work

- [ ] Implementation of NIST 800-88 Rev. 1 Clear/Purge sanitization engines
- [ ] Integration of ATA and NVMe hardware sanitization command sets
- [ ] The Sleuth Kit (TSK) C/Rust bridge for NTFS, FAT, and ext4 traversal
- [ ] Header/footer carver engine with fragmentation reassembly
- [ ] Cryptographic sanitization certificate generation (PDF/JSON)
- [ ] Cross-platform desktop release pipelines for Windows and Linux

---

## License

LocardX license determination is currently pending project owner finalization. See [LICENSE_SELECTION.md](LICENSE_SELECTION.md) and [LICENSE.template](LICENSE.template) for licensing options and guidelines.
