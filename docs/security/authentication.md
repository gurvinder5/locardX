# LocardX Authentication & Access Control Specification

## 1. Architectural Overview

LocardX is designed for high-integrity forensic and data sanitization environments. It enforces a strict **Closed Registration Model**:
- No public self-registration or signup endpoints exist.
- A fresh installation enters a one-time setup mode allowing the creation of the initial root Administrator.
- Once the initial Administrator is established, first-run initialization is permanently locked at the backend layer.
- All subsequent user accounts must be created and provisioned by an authenticated Administrator.

---

## 2. Role-Based Access Control (RBAC)

LocardX implements four tiered operational roles:

| Role | Description | Allowed Operations |
| :--- | :--- | :--- |
| **Administrator** | Root operational authority | User management (create, enable, disable), system configuration, full access to all pillars and audit trails. |
| **Investigator** | Digital forensic examiner | Forensic recovery, file carving, report generation, audit trail review. |
| **Operator** | Sanitization technician | Targeted file erasure, drive sanitization, execution of destruction routines. |
| **Viewer** | Auditing observer | Read-only inspection of reports and verification hashes. |

> **Security Rule**: Authorization is strictly enforced in the Rust application layer. Role claims from the frontend webview are discarded; the session token determines caller identity and privileges.

---

## 3. Password Security & Cryptographic Invariants

- **Hashing Algorithm**: Argon2id via the standard `argon2` crate.
- **Salt Generation**: Cryptographically secure pseudo-random 16-byte salts (`rand::thread_rng`).
- **Complexity Requirements**:
  - Minimum 10 characters.
  - At least 1 uppercase letter (`[A-Z]`).
  - At least 1 lowercase letter (`[a-z]`).
  - At least 1 numerical digit (`[0-9]`).
  - At least 1 symbol/special character (`[^A-Za-z0-9]`).
- **Zero Leakage**:
  - Passwords and password hashes are never written to log files, traces, or terminal outputs.
  - `PublicUser` DTOs omit `password_hash` at compile time.
  - Constant-time dummy verification is performed on non-existent accounts to eliminate timing-based user enumeration.

---

## 4. Session Life Cycle & Lockout Controls

- **Token Format**: 256-bit cryptographically secure hex tokens (64 characters).
- **Session Duration**: 8 hours from creation.
- **Immediate Invalidation**:
  - Calling `logout` deletes the active token immediately.
  - Disabling a user account terminates and purges all active sessions for that user instantly.
- **Brute-Force Protection**:
  - Failed login attempts are logged and bounded (lockout after 5 consecutive failures within a 15-minute window).
  - Generic error messages ("Invalid username or password.") are returned uniformly.

---

## 5. Audit Logging

Every security-relevant event is written to the cryptographic audit trail via `AuditService` with SHA-256 hash chaining:
- `FIRST_ADMIN_CREATED`
- `LOGIN_SUCCESS`
- `LOGIN_FAILURE`
- `LOGOUT`
- `USER_CREATED`
- `USER_ENABLED`
- `USER_DISABLED`
- `PASSWORD_CHANGED`
