# LocardX Role-Based Access Control (RBAC) Matrix

## 1. Overview

LocardX implements a granular, least-privilege Role-Based Access Control (RBAC) model. The system defines four primary operational roles and 27 fine-grained permissions across all workstation functional domains:

1. **Administrator**: Root system authority, user provisioning, global auditing, and full operational capability.
2. **Investigator**: Digital forensics specialist, disk acquisition, file carving/recovery, case management, and report generation.
3. **Operator**: Sanitization and evidence handling technician, targeted file and physical drive sanitization.
4. **Viewer**: Independent auditing observer, read-only inspection of reports, cases, artifacts, and cryptographic audit hash-chains.

---

## 2. Granular Permissions Matrix

| Permission Key | Domain | Administrator | Investigator | Operator | Viewer |
| :--- | :--- | :---: | :---: | :---: | :---: |
| **Case Management (Step 13)** | | | | | |
| `CaseCreate` | Case Management | ✅ | ✅ | ❌ | ❌ |
| `CaseView` | Case Management | ✅ | ✅ | ✅ | ✅ |
| `CaseModify` | Case Management | ✅ | ✅ | ❌ | ❌ |
| `CaseClose` | Case Management | ✅ | ✅ | ❌ | ❌ |
| `CaseEvidenceAdd` | Evidence Tracking | ✅ | ✅ | ✅ | ❌ |
| `CaseCustodyRecord` | Chain of Custody | ✅ | ✅ | ✅ | ❌ |
| `CaseReportGenerate` | Case Reporting | ✅ | ✅ | ❌ | ❌ |
| **Forensic Acquisition (Step 11)** | | | | | |
| `AcquisitionViewSources` | Disk Acquisition | ✅ | ✅ | ✅ | ✅ |
| `AcquisitionStart` | Disk Acquisition | ✅ | ✅ | ❌ | ❌ |
| `AcquisitionCancel` | Disk Acquisition | ✅ | ✅ | ❌ | ❌ |
| `AcquisitionViewArtifacts` | Disk Acquisition | ✅ | ✅ | ✅ | ✅ |
| **Advanced Recovery (Step 12)** | | | | | |
| `RecoveryStart` | File Carving | ✅ | ✅ | ❌ | ❌ |
| `RecoveryViewResults` | File Carving | ✅ | ✅ | ✅ | ✅ |
| `RecoveryExportFiles` | Evidence Export | ✅ | ✅ | ❌ | ❌ |
| `RecoveryGenerateReport` | Recovery Reporting | ✅ | ✅ | ❌ | ❌ |
| **Drive Sanitization (Step 10)** | | | | | |
| `ErasureViewCapabilities` | Drive Sanitization | ✅ | ❌ | ✅ | ✅ |
| `ErasurePlan` | Drive Sanitization | ✅ | ❌ | ✅ | ❌ |
| `ErasureExecute` | Drive Sanitization | ✅ | ❌ | ✅ | ❌ |
| `ErasureGenerateReport` | Sanitization Reporting | ✅ | ❌ | ✅ | ❌ |
| **Cryptographic Audit Trail** | | | | | |
| `AuditView` | Audit Trail | ✅ | ✅ | ✅ | ✅ |
| `AuditVerify` | Hash-Chain Verification | ✅ | ✅ | ✅ | ✅ |
| **User Administration (Step 14)** | | | | | |
| `UserCreate` | Identity & Access | ✅ | ❌ | ❌ | ❌ |
| `UserList` | Identity & Access | ✅ | ❌ | ❌ | ❌ |
| `UserView` | Identity & Access | ✅ | ❌ | ❌ | ❌ |
| `UserUpdate` | Identity & Access | ✅ | ❌ | ❌ | ❌ |
| `UserDisable` | Identity & Access | ✅ | ❌ | ❌ | ❌ |
| `UserChangeRole` | Identity & Access | ✅ | ❌ | ❌ | ❌ |
| `UserUnlock` | Identity & Access | ✅ | ❌ | ❌ | ❌ |

---

## 3. Enforcement Points

1. **Rust Core Enforcement**:
   - `AuthorizationEngine::check_permission(role, permission)` evaluates permission sets statically and deterministically.
   - `AuthService::authorize_permission(token, permission)` validates session existence, non-expiration, account enablement, and role permissions.
   - Any unauthorized call emits a `LocardError::SecurityViolation` and logs an `AUTHORIZATION_DENIED` event into the tamper-evident audit chain.
2. **Desktop Tauri IPC Guard**:
   - Tauri command handlers in `src-tauri/src/commands/` call `state.auth.authorize_permission(token, Permission::...)` before delegating to subsystem handlers.
   - Unauthenticated or unauthorized callers receive safe, sanitized error responses with standard error codes (`UNAUTHORIZED`, `FORBIDDEN`).
3. **Frontend Defensive Adaptation**:
   - The UI adapts view components based on `useAuthStore().hasPermission(permission)`.
   - UI disabling is purely ergonomic; backend Rust guards guarantee frontend spoofing or client-side tampering is impossible.

---

## 4. Administrative Protection Invariants

- **Sole Administrator Protection**:
  - The system disallows disabling or demoting the last active Administrator account.
  - Attempting to disable or change the role of the sole active Administrator produces `LocardError::SecurityViolation("Cannot disable/demote the sole active administrator")`.
- **Root Elevation Separation**:
  - Administrator privileges do not bypass physical drive safety interlocks (Step 10) or immutable audit chain verification.
