# Security Controls & Safeguards

This document describes the active and architectural security controls implemented in LocardX to ensure safe operations and defend against accidental or malicious data destruction.

---

## 1. Destructive Operation Safeguards

### Target Validation
Before any drive or file erasure operation begins:
- The device path is inspected by `crates/device-manager`.
- The volume is checked against known system boot partitions, current OS installation directories (`%SystemRoot%`, `/`), and mounted root drives.
- If a target is recognized as a system disk, the operation is blocked unconditionally.

### Explicit Double-Confirmation Interlocks
- Destructive operations require the user to explicitly type the target device identifier or volume label in the UI before confirmation can be submitted.
- The Tauri backend validates that the confirmation token exactly matches the target identifier.

---

## 2. Evidence Integrity Controls

### Read-Only File Handles
- Forensic recovery routines in `crates/recovery-engine` only request read access (`O_RDONLY` / `GENERIC_READ`).
- If write access is required by any downstream library, the operation is immediately aborted.

### Write-Blocker Status Detection
- When physical devices are enumerated, `crates/device-manager` interrogates hardware flags to report whether a hardware write blocker is present.

---

## 3. Cryptographic Audit Logging

- Every operation emits a structured audit record into SQLite (`crates/database`).
- Each record includes:
  - Timestamp (UTC ISO 8601)
  - Operator / User ID
  - Operation type and target identifier
  - Previous entry SHA-256 hash
  - Current entry SHA-256 hash ($H_n = \text{SHA-256}(H_{n-1} \parallel \text{Payload}_n)$)
- Any modification to historical records invalidates all subsequent hashes, providing immediate detection of tampering.
