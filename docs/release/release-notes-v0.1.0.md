# LocardX Release Notes — Version 0.1.0

**Release Tag**: `v0.1.0-release`  
**Release Date**: September 12, 2026  
**Status**: Production / Release Candidate Final Acceptance  

---

## 1. Release Highlights

LocardX v0.1.0 marks the final major milestone and first official production release of the LocardX Unified Forensic Workstation. This release consolidates all forensic, recovery, sanitization, case management, and identity capabilities into a unified, mathematically verifiable desktop application.

### Key Features Delivered:
- **Unified Case & Custody Management (Step 13)**: End-to-end investigation case lifecycles, cross-module operation association, and technical chain-of-custody tracking.
- **Closed Identity & Access Control (Step 14)**: OWASP-compliant Argon2id password hashing, closed user bootstrap (zero public self-registration), and strict 4-tier RBAC boundaries.
- **Forensic Disk Acquisition (Step 11)**: Bitstream raw DD imaging with real-time streaming SHA-256 verification and immutable artifact registration.
- **Advanced File Recovery & Carving (Step 12)**: Deep carving supporting JPEG, PNG, PDF, ZIP, DOCX, SQLite, and text file signatures with multi-factor confidence scoring.
- **Secure Drive Sanitization (Step 10)**: Media-aware sanitization planning for HDD, SSD, and USB media (NIST 800-88 Clear, DoD 5220.22-M) with simulation mode, system boot disk hard-blocks, and two-stage confirmation challenges.
- **Tamper-Evident Audit Ledger**: Continuous SHA-256 hash chains cryptographically validating the integrity of every action taken across the workstation.
- **Unified Forensic Reporting**: Single-click multi-module case investigation reports and sanitization certificates with canonical SHA-256 verification.

---

## 2. System Requirements & Build Instructions

### Prerequisites
- **Operating System**: Windows 10/11 (x86_64), Linux (Ubuntu 22.04+, Debian 12+), macOS 13+
- **Rust Toolchain**: `rustc` and `cargo` 1.83.0 or higher
- **Node.js**: Node 18+ and `npm` 9+
- **Tauri Prerequisites**: Windows Webview2 (built into Windows 10/11), C++ build tools (MSVC on Windows)

### Compilation & Build Commands

1. **Verify Backend Build & Unit Tests**:
   ```bash
   cargo check --workspace
   cargo test --workspace
   ```

2. **Verify System Integration & Benchmarks**:
   ```bash
   cargo test -p locardx-desktop --test final_system_integration_test -- --nocapture
   cargo test -p locardx-desktop --test performance_benchmarks -- --nocapture
   ```

3. **Verify Frontend Assets**:
   ```bash
   cd frontend
   npm run typecheck
   npm run build
   cd ..
   ```

4. **Build Production Release Binary**:
   ```bash
   cargo build --release -p locardx-desktop
   ```
   The compiled executable will be located at:
   `target/release/locardx-desktop.exe` (Windows) or `target/release/locardx-desktop` (Linux/macOS).

---

## 3. Security Invariants & Operational Guarantees

- **Zero Write Invariant**: Evidence files are accessed strictly read-only; pre- and post-recovery SHA-256 checks prove zero bytes altered.
- **Hard-Blocked OS Media**: Boot volumes (`C:\`, `/`) cannot be targeted for sanitization under any circumstance.
- **Audit Tamper Detection**: Any database tampering is immediately surfaced during startup chain verification.
- **Fail-Closed Validation**: Any mismatched hashes or unauthorized role access reject operations immediately.
