# LocardX: Safety & Authorization Interlock Specification

## 1. Architectural Role & Purpose

The LocardX Safety & Authorization Interlock Layer establishes a mandatory, fail-closed gate between the high-level Operation Manager and underlying storage device execution engines.

Its core mandate is to ensure that no destructive operation (e.g. whole-drive erasure, partition zeroing, file sanitization) can ever execute without satisfying a multi-stage chain of cryptographic, authorization, geometric, and cooperative verification checks.

> [!IMPORTANT]
> **FOUNDATION STAGE INVARIANT:**
> All destructive erasure executors (`DriveErasure`, `FileErasure`, `FolderErasure`) and file carving engines (`Recovery`) remain permanently disabled. The safety layer evaluates, verifies, and records confirmation challenges, but will terminate execution with `ReasonCode::OperationDisabled`. No sector writes or file destructions are executable in this build.

---

## 2. The Multi-Stage Verification Pipeline

Before any future operation can be dispatched to an executor, it must traverse the strict sequential pipeline:

```
    AUTHENTICATION
          ↓
    AUTHORIZATION (RBAC)
          ↓
    OPERATION MANAGER
          ↓
    TARGET VALIDATION (Live Discovery)
          ↓
    SAFETY POLICY (Hard System/Boot Blocks)
          ↓
    TARGET REVALIDATION (TOCTOU Defense)
          ↓
    EXPLICIT CONFIRMATION (Time-Bound Challenge)
          ↓
    METHOD COMPATIBILITY
          ↓
    ERASURE EXECUTOR (Currently Disabled)
```

---

## 3. Core Architectural Principles

### A. Authorization $\neq$ Device Classification
Device classifications emitted by OS queries (`SystemDevice`, `BootDevice`, `RemovableDevice`, `ExternalDevice`, `FixedDataDevice`, `Unknown`) are **purely descriptive metadata**; they never independently confer authority.
- `RemovableDevice ≠ authorized`
- `ExternalDevice ≠ authorized`
- Authorization requires: authenticated user identity, role capabilities, operation category, risk level, target revalidation, and explicit confirmation.

### B. Fail-Closed Design
The interlock fails closed on every ambiguous or unexpected state:
- If target storage layout cannot be identified $\rightarrow$ **BLOCK** (`UNKNOWN_TARGET`)
- If target no longer exists $\rightarrow$ **BLOCK** (`INVALID_TARGET`)
- If user session has expired or is unauthenticated $\rightarrow$ **DENY** (`UNAUTHORIZED_ROLE`)
- If target layout or capacity changes between review and execution $\rightarrow$ **BLOCK** (`TARGET_CHANGED`)
- If confirmation token has expired or is missing $\rightarrow$ **BLOCK** (`MISSING_CONFIRMATION`)
- If an internal provider or dependency fails $\rightarrow$ **BLOCK** (`SAFETY_POLICY_VIOLATION`)

### C. System & Boot Drive Hard Protection
The active operating system root drive, Windows system volume, boot loader/EFI partition (`ESP`), and Unix root paths (`/`, `/boot`) are hard-blocked by server-side logic:
- `is_system` or `is_boot` targets return `Blocked` with `SYSTEM_DEVICE` or `BOOT_DEVICE`.
- **Absolute Rule**: User confirmation or UI overrides can **NEVER** bypass a system or boot device block.

### D. Time-Of-Check to Time-Of-Use (TOCTOU) Defense
A target snapshot (`TargetSnapshot`) captures immutable geometry at review time:
- Identifier (`\\.\PhysicalDrive1`)
- Byte capacity
- Hardware device ID
- Filesystem label
- System/boot flags
- Snapshot timestamp

Before an explicit confirmation can be accepted, the target is re-queried live from the host operating system. If removable media has been swapped, a disk partition altered, or a drive re-assigned, `TargetSnapshot::detect_changes` triggers an immediate revocation with `TARGET_CHANGED`.

### E. Two-Stage Structured Confirmation
1. **Stage 1 (Review & Challenge Generation)**: Operator reviews live target properties and requests a confirmation challenge. The server generates a UUID challenge with a 5-minute time-to-live (TTL) bound to the exact target snapshot and requesting actor.
2. **Stage 2 (Explicit Commitment)**: The operator must acknowledge high-risk consequences and type the exact target identifier. User B cannot submit a challenge issued to User A. Replay of consumed challenges is rejected.

---

## 4. Risk Classification

| Risk Level | Trigger Criteria | Action Required |
| :--- | :--- | :--- |
| **Low** | Read-only integrity operations (`IntegrityHash`, `IntegrityVerify`) on non-system targets. | Standard authorization check. |
| **Medium** | Read-only operations on raw physical drives or system-adjacent files. | Elevated logging; standard authorization. |
| **High** | Destructive operation requests on verified external or removable storage. | Two-stage explicit confirmation required. |
| **Critical** | Destructive attempts targeting System/Boot disks, unknown hardware, or changed targets. | **Hard Block**; non-overridable. |

---

## 5. Machine-Readable Reason Codes

The interlock communicates status via stable enum codes rather than arbitrary strings:

| Reason Code | Meaning |
| :--- | :--- |
| `VALID_TARGET` | Target is verified and safe for read-only operation. |
| `SYSTEM_DEVICE` | Target houses active operating system root (Hard Block). |
| `BOOT_DEVICE` | Target houses boot loader / EFI partition (Hard Block). |
| `UNKNOWN_TARGET` | Target hardware classification is indeterminate (Fail-Closed). |
| `INVALID_TARGET` | Target path or device identifier does not exist. |
| `UNAUTHORIZED_ROLE` | Authenticated role lacks capability for requested operation category. |
| `MISSING_CONFIRMATION` | High-risk target eligible for review; requires confirmation challenge. |
| `CONFIRMATION_EXPIRED` | Confirmation challenge exceeded 5-minute TTL. |
| `CONFIRMATION_INVALID` | Typed target string or warning acknowledgment mismatch. |
| `TARGET_CHANGED` | Target hardware/geometry changed since review (TOCTOU defense). |
| `UNSUPPORTED_OPERATION`| Operation category unrecognized. |
| `OPERATION_DISABLED` | Destructive executor is disabled in this foundation phase. |

---

## 6. Tamper-Evident Audit Trail Integration

All safety gate evaluations and lifecycle events are anchored into the append-only SHA-256 hash-chain via `AuditService`:
- `AUTHORIZATION_GRANTED` / `AUTHORIZATION_DENIED`
- `SAFETY_CHECK_STARTED` / `SAFETY_CHECK_BLOCKED`
- `TARGET_REVALIDATION_FAILED`
- `CONFIRMATION_REQUESTED`
- `CONFIRMATION_ACCEPTED` / `CONFIRMATION_REJECTED`
