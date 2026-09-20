# Data Flow Architecture

This document describes the flow of control and data during typical LocardX operations.

---

## 1. Drive Sanitization Data Flow

```text
[User (UI)]
    │ 1. Selects target disk + algorithm (e.g., NIST 800-88 Clear)
    │ 2. Types confirmation phrase
    ▼
[Tauri Command Layer (src-tauri/src/commands/drive_eraser_commands.rs)]
    │ 3. Validates command parameters
    ▼
[Security Interlock (crates/security)]
    │ 4. Verifies disk is not boot/system disk and not read-only
    ▼
[Operation Manager (crates/operation-manager)]
    │ 5. Creates OperationRecord (PENDING), registers cancellation token
    ▼
[Drive Erasure Engine (crates/drive-eraser)]
    │ 6. Executes multi-pass overwrite stream
    │ 7. Emits progress events -> Tauri Event Stream -> UI ProgressBar
    ▼
[Verification Engine (crates/verification)]
    │ 8. Verifies target sectors against expected pattern / entropy
    ▼
[Audit Subsystem (crates/audit)]
    │ 9. Computes hash-chain entry and commits to SQLite
    ▼
[Reporting Subsystem (crates/reporting)]
    │ 10. Emits Sanitization Certificate (JSON / PDF)
```

---

## 2. Forensic Carving Data Flow

```text
[User (UI)]
    │ 1. Selects source image file or physical disk
    ▼
[Recovery Engine (crates/recovery-engine)]
    │ 2. Opens handle in STRICT READ-ONLY mode
    │ 3. Records pre-operation SHA-256 hash of source image
    ▼
[TSK / Signature Scanner]
    │ 4. Scans sector clusters for file magic headers & footers
    │ 5. Reconstructs fragmented clusters
    ▼
[Confidence Scorer]
    │ 6. Computes score based on structure validity & entropy
    ▼
[Output Destination]
    │ 7. Writes recovered files to user-designated output folder
    │ 8. Verifies post-operation SHA-256 hash matches pre-operation hash
    ▼
[Audit & Forensic Report]
    │ 9. Logs extraction event and issues forensic recovery report
```
