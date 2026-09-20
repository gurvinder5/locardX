# LocardX Authentication Architecture Specification

## 1. Executive Summary

LocardX is a mission-critical digital forensics and data sanitization workstation. The authentication and access control architecture (Step 14) is architected to guarantee operational security, auditability, and role separation without degrading or modifying physical drive protection invariants.

---

## 2. Core Architectural Principles

### 2.1 Closed Registration Model
1. **No Public Registration**: LocardX provides zero endpoints or UI mechanisms for unauthenticated or self-service user registration.
2. **First-Run Bootstrap**:
   - On initial launch, the database contains zero active Administrator records (`is_first_run() == true`).
   - The workstation permits exactly one bootstrap call to `initialize_admin` / `initialize_administrator`.
   - Once successfully created, first-run bootstrap is permanently locked at the database and business logic layer.
   - Any subsequent call to `initialize_admin` returns `LocardError::SecurityViolation` and logs a security warning.
3. **Controlled User Provisioning**:
   - All subsequent users can only be provisioned by an active, authenticated Administrator possessing the `UserCreate` permission.
   - User creation requires explicitly defining `username`, strong initial `password`, assigned `UserRole`, and optional `display_name` and `metadata_json`.

### 2.2 Modern Password Cryptography
- **Algorithm**: Argon2id (hybrid data-dependent and data-independent memory-hard password hashing function).
- **Salt Generation**: 16 cryptographically secure random bytes generated per hash via `rand::rngs::OsRng`.
- **Serialization**: Standard PHC (Password Hashing Competition) string format: `$argon2id$v=19$m=65536,t=3,p=1$...`
- **Zero-Plaintext Invariant**:
  - Plaintext passwords are never persisted to disk, database, caches, or files.
  - Plaintext passwords and password hashes are strictly redacted and forbidden from log messages, tracing outputs, audit trails, and forensic reports.
  - `PublicUser` DTOs omit `password_hash` at compile time.
- **Timing Attack Mitigation**:
  - Verification of non-existent users executes a dummy Argon2id hash check against a pre-computed hash to equalize response times and prevent username enumeration.

### 2.3 Desktop Session Security
- **Token Generation**: High-entropy 256-bit cryptographically secure hexadecimal strings (64 characters).
- **Session Lifespan**: Default duration of 28,800 seconds (8 hours) with sliding expiration upon valid activity.
- **Session Invalidation**:
  - Explicit logout immediately purges the session token from active state and database.
  - Account disabling or locking immediately terminates and revokes all active tokens associated with that `user_id`.
  - Expired tokens return `LocardError::Unauthorized` or `is_valid: false` and reject all protected operations.
- **Lockout Safeguards**:
  - 5 consecutive failed login attempts result in a 300-second (5-minute) account lockout.
  - Administrators can explicitly execute `unlock_user` to clear failed attempt counters.

---

## 3. Physical Drive Safety Interlocks Independence

> [!IMPORTANT]
> **Safety Invariant**: Backend authorization checks must **NEVER** replace, bypass, or weaken the physical-drive safety interlocks established in Step 10 (Secure Drive Eraser).

The execution pipeline enforces layered defense-in-depth:
1. **Layer 1 - RBAC Authorization**: Validates caller session has `ErasureExecute` permission.
2. **Layer 2 - Security Engine Interlocks**:
   - System/boot drive blocking (`validate_target_safety`).
   - System volume partition blocking (`C:\`, EFI, Recovery).
   - Write-blocker invariant enforcement.
3. **Layer 3 - Real Hardware Safety Gate**:
   - Real hardware execution strictly disabled in software until explicit safety gate activation.
   - Two-stage cryptographic confirmation token verification.
   - Typed confirmation string validation (`ERASE-TARGET-DEVICE`).
   - TOCTOU snapshot verification (disk size, serial number, partition layout checked immediately before issuance of ATA/SCSI commands).

---

## 4. Tamper-Evident Audit Integration

All authentication lifecycle events and authorization decisions are cryptographically recorded into the immutable hash-chain audit log (`crates/audit`):
- `FIRST_ADMIN_CREATED`: System bootstrap completion.
- `LOGIN_SUCCESS`: Authenticated operator session establishment.
- `LOGIN_FAILURE`: Invalid credentials or locked account attempt.
- `LOGOUT`: Clean user session termination.
- `SESSION_EXPIRED`: Invalidation due to timeout.
- `USER_CREATED`: New account provisioning by an Administrator.
- `USER_ENABLED` / `USER_DISABLED`: Status change of user accounts.
- `USER_ROLE_CHANGED`: Privilege escalation or modification.
- `USER_UPDATED`: Modification of display name or metadata.
- `PASSWORD_CHANGED`: Password rotation.
- `AUTHORIZATION_DENIED`: Violation attempt by a user lacking required permission.
- `ACCOUNT_UNLOCKED`: Manual reset of account lockout.

Each audit record contains sequential monotonic numbering, UTC timestamp, sha256 hash of event data, and cryptographic linkage to the preceding event hash.

---

## 5. Tauri IPC Command Architecture

The desktop application layer (`src-tauri`) mediates between the React UI and core Rust engines:

```mermaid
graph TD
    UI[Frontend React UI] -->|Tauri Invoke| IPC[Tauri IPC Layer (auth_commands)]
    IPC -->|Token + Permission Check| AuthSvc[AuthService (crates/auth)]
    AuthSvc -->|Verify Session| DB[(SQLite Database)]
    AuthSvc -->|Evaluate RBAC| Engine[AuthorizationEngine]
    AuthSvc -->|Record Event| Audit[AuditService (crates/audit)]
    IPC -->|Authorized Invocation| Core[Forensic / Sanitization Engines]
```
