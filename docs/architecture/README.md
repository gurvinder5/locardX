# LocardX Architecture Overview

LocardX is designed with strict boundaries separating user interface presentation, desktop IPC dispatching, domain-specific execution engines, and shared infrastructure services.

---

## Core Domain Separation

The application centers around three independent operational pillars. Each user-facing feature routes to its dedicated execution engine:

```text
Secure Drive Eraser
        │
        ▼
Drive Erasure Engine (crates/drive-eraser)
        │
        ├─ NIST SP 800-88 Rev. 1 Clear & Purge
        ├─ Multi-pass pattern overwriting (DoD 5220.22-M, Gutmann)
        └─ Hardware sanitization dispatch (ATA Secure Erase, NVMe Sanitize)

Secure File & Folder Eraser
        │
        ▼
File/Folder Erasure Engine (crates/file-eraser)
        │
        ├─ Targeted file block overwriting
        ├─ Directory tree recursion
        ├─ Filesystem metadata clearing (timestamps, MFT/inode attributes)
        └─ Alternate Data Streams (ADS) and slack space zeroing

Advanced File Carving & Recovery
        │
        ▼
Recovery Engine (crates/recovery-engine)
        │
        ├─ Read-only source media enforcement
        ├─ Signature-based file carving (headers/footers)
        ├─ The Sleuth Kit (TSK) filesystem structure bridge
        └─ Fragment reconstruction & confidence scoring
```

---

## Shared Platform Services

Domain engines depend on shared, modular platform services. These services remain decoupled from the domain logic:

```text
┌─────────────────────────────────────────────────────────────┐
│                      Shared Services                        │
├──────────────────────────┬──────────────────────────────────┤
│ Authentication & RBAC    │ User roles, session lifecycle,   │
│ (crates/auth)            │ privilege checks                 │
├──────────────────────────┼──────────────────────────────────┤
│ Device & FS Detection    │ OS disk enumeration, geometry,   │
│ (crates/device-manager)  │ mount points, write-blocker info │
├──────────────────────────┼──────────────────────────────────┤
│ Operation Management     │ Job queues, scheduling, progress │
│ (crates/operation-manager)│ events, cooperative cancellation│
├──────────────────────────┼──────────────────────────────────┤
│ Verification Engine      │ Multi-pass pattern verification, │
│ (crates/verification)    │ hash comparison                  │
├──────────────────────────┼──────────────────────────────────┤
│ Audit Logging            │ Tamper-evident SHA-256 hash      │
│ (crates/audit)           │ chain log                        │
├──────────────────────────┼──────────────────────────────────┤
│ Database Persistence     │ SQLite connection pool and       │
│ (crates/database)        │ schema migrations                │
├──────────────────────────┼──────────────────────────────────┤
│ Reporting Subsystem      │ Forensic, audit, & sanitization  │
│ (crates/reporting)       │ report generation (PDF/JSON)     │
├──────────────────────────┼──────────────────────────────────┤
│ Security & Integrity     │ Safety interlocks, secret        │
│ (crates/security)        │ hygiene, crypto primitives       │
└──────────────────────────┴──────────────────────────────────┘
```

---

## High-Level Data & Control Flow

1. **User Interaction**: Analyst configures an operation in the React interface (e.g., selecting a test drive or targeting files for sanitization).
2. **IPC Dispatch**: Frontend dispatches a typed IPC invocation via Tauri commands (`src-tauri/src/commands/`).
3. **Safety & Policy Check**:
   - `crates/security` verifies the target is not a protected boot/OS volume.
   - `crates/auth` validates user permissions.
4. **Operation Registration**: `crates/operation-manager` allocates an operation ID, registers state, and returns a tracking handle.
5. **Execution**: The dedicated domain engine (`drive-eraser`, `file-eraser`, or `recovery-engine`) begins processing in background Tokio tasks.
6. **Progress Streaming**: Events and metrics stream via Tauri events to the frontend progress components.
7. **Verification**: If enabled, `crates/verification` samples or scans the results.
8. **Audit & Reporting**: `crates/audit` appends an immutable hash-chained event; `crates/reporting` emits a sanitization or forensic certificate.
