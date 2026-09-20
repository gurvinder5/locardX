# LocardX Tauri IPC Command Reference

This document catalogs the desktop IPC endpoints exposed between the React frontend and the Rust application shell.

## 1. System Commands

### `get_app_info`
- **Handler**: `commands::get_app_info_handler`
- **Description**: Returns application metadata, version, build status, and environment.
- **Parameters**: None.
- **Returns**: `AppInfo { name, version, build_status, environment }`
- **Privilege**: Public / Unauthenticated.

---

## 2. Authentication & User Management Commands

### `is_first_run`
- **Handler**: `commands::is_first_run_handler`
- **Description**: Checks whether the system has an established Administrator or requires first-run setup.
- **Parameters**: None.
- **Returns**: `bool`
- **Privilege**: Public / Unauthenticated.

### `initialize_admin`
- **Handler**: `commands::initialize_admin_handler`
- **Description**: Creates the root Administrator account during initial deployment. Permanently fails once an admin exists.
- **Parameters**: `payload: InitAdminRequest { username, password }`
- **Returns**: `PublicUser`
- **Privilege**: Unauthenticated (only valid when `is_first_run` is true).

### `login`
- **Handler**: `commands::login_handler`
- **Description**: Authenticates user credentials with Argon2id and generates a secure session.
- **Parameters**: `payload: LoginRequest { username, password }`
- **Returns**: `AuthSessionResponse { token, user, expires_at }`
- **Privilege**: Public / Unauthenticated.

### `logout`
- **Handler**: `commands::logout_handler`
- **Description**: Immediately invalidates and revokes the active session token.
- **Parameters**: `token: String`
- **Returns**: `()`
- **Privilege**: Authenticated caller.

### `get_current_user`
- **Handler**: `commands::get_current_user_handler`
- **Description**: Resolves session token to current user profile without password hash.
- **Parameters**: `token: String`
- **Returns**: `PublicUser`
- **Privilege**: Authenticated caller.

### `create_user`
- **Handler**: `commands::create_user_handler`
- **Description**: Registers a new user account with assigned role.
- **Parameters**: `token: String`, `payload: CreateUserRequest { username, password, role }`
- **Returns**: `PublicUser`
- **Privilege**: Administrator only.

### `set_user_enabled`
- **Handler**: `commands::set_user_enabled_handler`
- **Description**: Enables or disables a user account. Disabling immediately terminates all active sessions. Protects against disabling the sole administrator.
- **Parameters**: `token: String`, `payload: SetUserEnabledRequest { user_id, enabled }`
- **Returns**: `()`
- **Privilege**: Administrator only.

### `list_users`
- **Handler**: `commands::list_users_handler`
- **Description**: Returns all registered user accounts with sanitized profiles.
- **Parameters**: `token: String`
- **Returns**: `Vec<PublicUser>`
- **Privilege**: Administrator only.

### `change_password`
- **Handler**: `commands::change_password_handler`
- **Description**: Updates the password for the current authenticated user session.
- **Parameters**: `token: String`, `payload: ChangePasswordRequest { current_password, new_password }`
- **Returns**: `()`
- **Privilege**: Authenticated caller.

---

## 3. Storage Device & Filesystem Discovery Commands

### `list_storage_devices`
- **Handler**: `commands::list_storage_devices_handler`
- **Description**: Discovers attached physical storage devices and logical volumes using read-only OS queries. Does not open handles for writing, modify partition tables, or issue destructive commands. Returns sanitized DTOs without raw OS handles or unmasked serial numbers.
- **Parameters**: None.
- **Returns**: `Vec<StorageDeviceDto>`
- **Safety Invariant**: *Device classification is not authorization.*
- **Privilege**: Authenticated caller.

---

## 4. Cryptographic Integrity & Evidence Verification Commands

### `calculate_file_hash`
- **Handler**: `commands::calculate_file_hash_handler`
- **Description**: Computes the cryptographic hash of a file on the host system using bounded 64 KiB streaming passes. Never loads the entire file into memory, never alters target metadata or content. Anchors the calculation into the tamper-evident SHA-256 audit hash-chain and persists the result to `integrity_records`.
- **Parameters**:
  - `path: String`: File system path to the target evidence file.
  - `session_token: Option<String>`: Optional active user session token to associate the action with an operator account.
- **Returns**: `HashResult { target: TargetIdentity, algorithm: HashAlgorithm, digest: String, bytes_processed: u64, started_at: String, completed_at: String, duration_ms: u64, status: HashStatus }`
- **Privilege**: Authenticated / authorized operator.

### `verify_file_hash`
- **Handler**: `commands::verify_file_hash_handler`
- **Description**: Verifies the cryptographic integrity of a target file against an expected reference hash. Employs constant-time hex comparison to prevent side-channel timing analysis. Emits explicit outcomes: `Verified`, `Mismatch { expected, calculated }`, or `UnableToVerify { reason }`. All attempts are recorded in the tamper-evident audit chain (`INTEGRITY_VERIFICATION_PASS` / `FAIL` / `ERROR`).
- **Parameters**:
  - `path: String`: File system path to the target evidence file.
  - `expected_digest: String`: 64-character hexadecimal reference hash (case-insensitive).
  - `session_token: Option<String>`: Optional active user session token.
- **Returns**: `VerificationResult { target: TargetIdentity, algorithm: HashAlgorithm, expected_digest: String, calculated_digest: Option<String>, bytes_processed: u64, status: VerificationStatus, timestamp: String, duration_ms: u64 }`
- **Privilege**: Authenticated / authorized operator.

### `list_integrity_records`
- **Handler**: `commands::list_integrity_records_handler`
- **Description**: Returns historic cryptographic calculation and verification records from the SQLite database.
- **Parameters**:
  - `limit: Option<u32>`: Maximum number of records to return (defaults to 50).
- **Returns**: `Vec<IntegrityRecord>`
- **Privilege**: Authenticated caller.

---

## 5. Operation Manager & Lifecycle Commands

### `submit_integrity_hash_operation`
- **Handler**: `commands::submit_integrity_hash_operation`
- **Description**: Creates and schedules an asynchronous integrity hash calculation operation under the cooperative Operation Manager lifecycle. Returns the initial `OperationDto` immediately while calculation proceeds in a background task. Emits lifecycle state transitions (`Created` -> `Queued` -> `Running` -> `Completed` / `Failed` / `Cancelled`) with progress updates and audit trail correlation.
- **Parameters**:
  - `path: String`: File system path to the target evidence file.
  - `session_token: Option<String>`: Optional active user session token for actor attribution.
- **Returns**: `OperationDto`
- **Safety Invariant**: *Read-only evidence verification; non-destructive.*
- **Privilege**: Authenticated / authorized operator.

### `submit_integrity_verify_operation`
- **Handler**: `commands::submit_integrity_verify_operation`
- **Description**: Creates and schedules an asynchronous cryptographic integrity verification operation against an expected reference SHA-256 digest under the Operation Manager lifecycle.
- **Parameters**:
  - `path: String`: File system path to the target evidence file.
  - `expected_digest: String`: 64-character hexadecimal reference hash.
  - `session_token: Option<String>`: Optional active user session token for actor attribution.
- **Returns**: `OperationDto`
- **Safety Invariant**: *Read-only evidence verification; non-destructive.*
- **Privilege**: Authenticated / authorized operator.

### `get_operation`
- **Handler**: `commands::get_operation`
- **Description**: Fetches the current state, progress telemetry, target details, actor attribution, and result summary of an operation by its UUID.
- **Parameters**:
  - `operation_id: String`: UUID of the operation.
- **Returns**: `OperationDto`
- **Privilege**: Authenticated caller.

### `list_operations`
- **Handler**: `commands::list_operations`
- **Description**: Returns historic and active operations from the SQLite database with optional filtering by lifecycle state and operation type.
- **Parameters**:
  - `limit: Option<u32>`: Maximum number of operations to return (defaults to 50).
  - `state_filter: Option<String>`: Optional filter by `OperationState` (e.g., `Running`, `Completed`, `Cancelled`).
  - `type_filter: Option<String>`: Optional filter by `OperationType` (e.g., `IntegrityHash`, `IntegrityVerify`).
- **Returns**: `Vec<OperationDto>`
- **Privilege**: Authenticated caller.

### `cancel_operation`
- **Handler**: `commands::cancel_operation`
- **Description**: Cooperatively signals cancellation of an active or queued operation. Transitions the operation to `Cancelling`, sets the atomic cancellation token, logs `OPERATION_CANCELLATION_REQUESTED` in the audit hash-chain, and transitions to `Cancelled` once the cooperative executor halts.
- **Parameters**:
  - `operation_id: String`: UUID of the operation to cancel.
  - `session_token: Option<String>`: Optional active user session token for actor attribution.
- **Returns**: `()`
- **Privilege**: Authenticated / authorized operator.

---

## 6. Safety & Authorization Interlock Commands

### `evaluate_operation_safety`
- **Handler**: `commands::evaluate_operation_safety`
- **Description**: Evaluates target safety, RBAC permissions, system/boot protection, and method compatibility in real time. Re-queries host storage to prevent stale client state. Emits `SAFETY_CHECK_STARTED` and `SAFETY_CHECK_BLOCKED` / `AUTHORIZATION_DENIED` into the SHA-256 audit hash-chain.
- **Parameters**:
  - `request: EvaluateSafetyRequest { target_type, target_identifier, target_display_name, target_size_bytes, operation_type, session_token }`
- **Returns**: `SafetyDecision { evaluation_id, target, operation_type, actor_id, decision, reason_code, risk_level, message, evaluated_at, target_snapshot, requires_confirmation }`
- **Privilege**: Authenticated caller.

### `request_destructive_confirmation`
- **Handler**: `commands::request_destructive_confirmation`
- **Description**: Stage 1 of the two-stage confirmation protocol. Validates operator role, captures an immutable target snapshot, enforces hard system/boot blocks, and generates a 5-minute time-bound confirmation challenge persisted in SQLite `operation_confirmations`.
- **Parameters**:
  - `request: RequestConfirmationRequest { operation_id, target_type, target_identifier, target_display_name, target_size_bytes, operation_type, session_token }`
- **Returns**: `ConfirmationChallenge { confirmation_id, operation_id, actor_id, target, operation_type, risk_level, target_snapshot, created_at, expires_at }`
- **Privilege**: Authorized operator / investigator / administrator.

### `confirm_destructive_operation`
- **Handler**: `commands::confirm_destructive_operation`
- **Description**: Stage 2 of the two-stage confirmation protocol. Verifies actor identity, warning acknowledgment, typed target string, and TTL. Re-queries host storage devices live to detect any TOCTOU target substitutions. Upon valid commitment, returns `Blocked` with `ReasonCode::OperationDisabled` because destructive executors remain permanently disabled in this foundation phase.
- **Parameters**:
  - `request: ConfirmOperationRequest { confirmation_id, operation_id, warning_acknowledged, typed_target_confirmation, session_token }`
- **Returns**: `SafetyDecision`
- **Privilege**: Matching requesting operator.

### `list_safety_evaluations`
- **Handler**: `commands::list_safety_evaluations`
- **Description**: Queries recent safety evaluation records from SQLite table `safety_evaluations` for compliance and audit visibility.
- **Parameters**:
  - `limit: Option<u32>`: Maximum records to return (defaults to 50).
- **Returns**: `Vec<SafetyDecision>`
- **Privilege**: Authenticated caller.


