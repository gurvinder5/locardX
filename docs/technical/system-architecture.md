# LocardX System Architecture Guide

## 1. Overview & Architectural Principles

LocardX is a multi-tier, modular forensic workstation designed with strong separation of concerns, memory safety, and fail-closed security invariants. It links a high-performance Rust core (14 workspace crates) with a modern TypeScript/React user interface via Tauri IPC.

### Key Architectural Invariants:
1. **Safety Interlock Separation**: Destructive operations cannot bypass the centralized `SafetyEngine` and `RealHardwareExecutionGate`.
2. **Evidence Immutability**: All evidence acquisition and recovery modules enforce read-only streaming access; source images are mathematically invariant before and after analysis.
3. **Tamper-Evident Operational Audit**: Operations, state changes, and custody transitions produce cryptographic SHA-256 hash chains that cannot be modified or reordered undetected.
4. **Closed Identity & Access Control**: Closed user provisioning with Argon2id password key derivation and strict RBAC authorization boundaries.
5. **Fail-Closed Semantics**: Any validation failure, hash discrepancy, or boundary exception aborts processing immediately without degrading to an insecure fallback.

---

## 2. Workspace Crate Architecture

The Rust workspace comprises 14 dedicated crates:

```text
                               ┌──────────────────────┐
                               │   locardx-desktop    │ (Tauri App & IPC Commands)
                               └──────────┬───────────┘
                                          │
    ┌──────────────────┬──────────────────┼──────────────────┬──────────────────┐
    │                  │                  │                  │                  │
┌───▼──────────┐ ┌─────▼──────────┐ ┌─────▼──────────┐ ┌─────▼──────────┐ ┌─────▼──────────┐
│locardx-case- │ │locardx-recovery│ │locardx-drive-  │ │locardx-file-   │ │locardx-acquisit- │
│management    │ │-engine         │ │eraser          │ │eraser          │ │ion               │
└───┬──────────┘ └─────┬──────────┘ └─────┬──────────┘ └─────┬──────────┘ └─────┬──────────┘
    │                  │                  │                  │                  │
    └──────────────────┼──────────────────┴──────────────────┼──────────────────┘
                       │                                     │
           ┌───────────▼──────────┐              ┌───────────▼──────────┐
           │  locardx-reporting   │              │locardx-operation-mgr │
           └───────────┬──────────┘              └───────────┬──────────┘
                       │                                     │
    ┌──────────────────┴──────────────────┬──────────────────┴──────────────────┐
    │                                     │                                     │
┌───▼──────────┐                ┌─────────▼────────┐                  ┌─────────▼────────┐
│locardx-      │                │ locardx-security │                  │locardx-device-   │
│verification  │                └─────────┬────────┘                  │manager           │
└───┬──────────┘                          │                           └─────────┬────────┘
    │                                     │                                     │
    └──────────────────┬──────────────────┴──────────────────┬──────────────────┘
                       │                                     │
           ┌───────────▼──────────┐              ┌───────────▼──────────┐
           │     locardx-auth     │              │     locardx-audit    │
           └───────────┬──────────┘              └───────────┬──────────┘
                       │                                     │
                       └──────────────────┬──────────────────┘
                                          │
                               ┌──────────▼──────────┐
                               │  locardx-database   │ (SQLite + Migrations)
                               └──────────┬───────────┘
                                          │
                               ┌──────────▼──────────┐
                               │   locardx-common    │ (Errors & Shared Types)
                               └─────────────────────┘
```

### Crate Responsibilities:
1. `locardx-common`: Foundational types (`TargetIdentity`, `OperationType`), shared models, centralized `LocardError` and `SafeErrorResponse`.
2. `locardx-database`: SQLite wrapper with connection management, foreign key enforcement, WAL journaling, and 13 sequential SQL migrations.
3. `locardx-audit`: Tamper-evident operational audit ledger using sequential SHA-256 hash chains.
4. `locardx-auth`: User authentication, session management, closed provisioning, Argon2id hashing, and RBAC matrix.
5. `locardx-device-manager`: OS-agnostic physical block device and volume discovery, system-drive classification, and mock registries.
6. `locardx-security`: Centralized safety evaluation engine, target classification, 2-stage confirmation challenges, and destructive interlocks.
7. `locardx-verification`: Cryptographic streaming hashing (`StreamHasher`), post-sanitization readback verification, and evidence digest computation.
8. `locardx-acquisition`: Bitstream raw DD forensic imaging from block devices with real-time telemetry and artifact metadata registration.
9. `locardx-recovery-engine`: Deep file carver supporting JPEG, PNG, PDF, ZIP, DOCX, SQLite, and text signatures with multi-factor confidence scoring and TSK filesystem bridge.
10. `locardx-drive-eraser`: Media-aware whole-disk sanitization planner, NIST/DoD simulation backend, and real hardware execution gate.
11. `locardx-file-eraser`: Secure targeted file and directory erasure engine with path traversal protection.
12. `locardx-operation-manager`: Asynchronous task execution manager, progress broadcasting, and cooperative cancellation tokens.
13. `locardx-case-management`: Investigation case lifecycle, evidential asset tracking, and chronological chain-of-custody recording.
14. `locardx-reporting`: Tamper-evident report generator producing formal case investigation reports, sanitization certificates, and audit exports.
15. `locardx-desktop`: Tauri application harness, IPC command routing, and application state container (`AppState`).

---

## 3. Database Schema & Migrations

LocardX maintains persistent operational state in SQLite. Database integrity is enforced through strict PRAGMAs:
- `PRAGMA foreign_keys = ON;`
- `PRAGMA journal_mode = WAL;`
- `PRAGMA synchronous = NORMAL;`

### Sequential Migrations (001–013):
- **001_initial_schema**: Base operations, plans, and task statuses.
- **002_audit_log**: Append-only `audit_events` table with `previous_hash` and `current_hash`.
- **003_verification**: Integrity records and snapshot verification outcomes.
- **004_device_cache**: Discovered physical drives, serials, and device snapshot cache.
- **005_security_evaluations**: Recorded safety evaluations and reason codes.
- **006_safety_confirmations**: Destructive confirmation challenges, tokens, and expirations.
- **007_sanitization_certificates**: Drive and file sanitization certificates.
- **008_recovery_jobs**: File recovery sessions, target formats, and candidate metrics.
- **009_recovered_files**: Carved file catalog, offsets, sizes, confidence scores, and SHA-256 digests.
- **010_cases**: Investigation cases, titles, lead investigators, and status lifecycles.
- **011_case_evidence_operations**: Case associations linking operations and evidential assets.
- **012_chain_of_custody**: Chain of custody timeline entries bound to audit hash links.
- **013_closed_registration**: User accounts, Argon2id password hashes, roles, and session tokens.

---

## 4. Tauri Inter-Process Communication (IPC)

The desktop frontend communicates with the Rust backend via Tauri's IPC bridge:
- **Tauri Commands**: Request-response handlers invoked with `invoke('command_name', { payload })`. Handlers validate permissions, invoke core domain services, and return typed JSON or `SafeErrorResponse`.
- **Tauri Events**: One-way asynchronous event streams emitted from Rust to the frontend window for live progress telemetry:
  - `acquisition-progress`: Percent complete, bytes read, current MB/s.
  - `recovery-progress`: Blocks scanned, candidates evaluated, files carved.
  - `drive-erase-progress`: Sanitization pass, percentage, ETA.

---

## 5. Security & Safety Flow

```text
User Request (UI)
       │
       ▼
Tauri Command Handler
       │
       ▼
[1. RBAC Check] ─────────── Denied? ──► Log AUTHORIZATION_DENIED to Audit ──► Return SafeError
       │
       ▼
[2. Target Safety Engine] ─ Blocked? ─► Log SAFETY_CHECK_BLOCKED to Audit ──► Return SafeError
       │
       ▼
[3. Confirmation Gate] ──── Invalid? ─► Reject Operation ─────────────────► Return SafeError
       │
       ▼
[4. Domain Engine Execution] (Acquisition / Recovery / Sanitization Simulation)
       │
       ▼
[5. Cryptographic Post-Verification]
       │
       ▼
[6. Append Audit Hash Chain]
       │
       ▼
Return Success Response
```
