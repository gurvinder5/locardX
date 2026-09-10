# LocardX Development & Agent Rules

## Project Overview

LocardX is a desktop forensic and secure data sanitization platform designed for high-integrity storage operations.

Key Technologies:
- **Desktop Shell**: Tauri
- **Frontend**: React, TypeScript, Tailwind CSS, Vite
- **Backend**: Rust, Tokio async runtime
- **Data Persistence**: SQLite with rusqlite
- **Serialization**: Serde
- **Logging & Tracing**: `tracing`
- **Integrity**: SHA-256 and tamper-evident hash-chain audit logging
- **Forensic Engine**: The Sleuth Kit (TSK) integration + custom Rust carving engine
- **Reporting**: Structured forensic, audit, and sanitization reports

## Architectural Principles

The application maintains strict separation of concerns across three core pillars:
1. **Secure Drive Eraser** $\rightarrow$ Drive Erasure Engine
2. **Secure File & Folder Eraser** $\rightarrow$ File/Folder Erasure Engine
3. **Advanced File Carving & Recovery** $\rightarrow$ Recovery Engine

Shared platform services remain separate from domain engines:
- Authentication & Access Control (RBAC)
- Device Detection & Filesystem Identification
- Operation & Job Management (scheduling, cooperative cancellation, progress)
- Verification (multi-pass pattern matching, hash verification)
- Audit Logging (hash-chain integrity)
- Reporting Subsystem
- Database Persistence

## Development Guidelines

- **Preserve Architecture**: Follow the established directory and module hierarchy. Do not refactor architecture without explicit agreement.
- **Minimal Abstractions**: Prefer small, focused, testable modules with explicit interfaces over speculative abstractions.
- **Scope Discipline**: Implement only the assigned task. Do not modify unrelated modules.
- **Language Boundaries**:
  - Keep business, forensic, and low-level disk logic in **Rust**.
  - Keep presentation, layout, and state orchestration in **React / TypeScript**.
  - **Never** place destructive operations or raw disk execution logic directly in UI code.

## Git & Collaboration Rules

- `main` is protected and must never receive direct feature commits.
- `develop` is the integration branch for tested changes.
- All new work must be conducted on dedicated branches (`feature/<name>`, `fix/<name>`, `chore/<name>`).
- Never force-push (`--force`) to shared branches.
- Never rewrite or discard another contributor's commits.
- Keep commits small, logical, and formatted using Conventional Commits.
- Inspect `git diff` before staging and committing.

## Safety & Destructive Operations

LocardX handles low-level storage operations that are inherently destructive when executing sanitization routines.

> [!CAUTION]
> **NEVER** execute destructive disk, partition, or filesystem wiping routines against an actual physical disk, personal drive, or production system during development or testing.

- Always test destructive routines against **mocks, virtual disks (VHD/VHDX), loopback devices, or dedicated disposable lab media**.
- Destructive operations must require explicit double-confirmation and validation guards before execution.
- Never write tests that target default or unvalidated system drive letters or device paths (`/dev/sda`, `\\\\.\\PhysicalDrive0`, `C:`).

## Evidence Integrity

- Forensic carving and recovery routines must treat source media as **strictly read-only**.
- Never modify or write to source media during recovery workflows.
- Calculate and record SHA-256 hashes at every milestone (source image, recovered artifact, audit log entry).

## Testing & Verification

- Every core module must include unit tests covering standard cases, boundary conditions, and failure paths.
- Destructive operations must include mock verification tests ensuring zero out-of-bounds writes.
- Tests must never be weakened or disabled simply to pass a build.
