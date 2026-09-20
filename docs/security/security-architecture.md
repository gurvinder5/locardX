# LocardX Security Architecture & Controls

## 1. Threat Model & Trust Boundaries

LocardX operates in forensic environments where evidential integrity and protection against catastrophic accidental data loss are paramount. The architecture establishes four distinct security boundaries:

```text
┌────────────────────────────────────────────────────────┐
│               Untrusted UI Layer (WebView)             │
│  - Presentation logic, React state, user inputs        │
└───────────────────────────┬────────────────────────────┘
                            │ Tauri IPC (Strict Typed Schemas)
┌───────────────────────────▼────────────────────────────┐
│               Security Gatekeeper (Tauri/IPC)          │
│  - Session token validation                            │
│  - RBAC permission evaluation                          │
└───────────────────────────┬────────────────────────────┘
                            │ Rust Domain Calls
┌───────────────────────────▼────────────────────────────┐
│               Core Safety & Domain Engines             │
│  - Target Identity Classification                      │
│  - System Drive Hard-Blocks                            │
│  - Two-Stage Destructive Confirmation Engine           │
│  - RealHardwareExecutionGate Interlock                 │
└───────────────────────────┬────────────────────────────┘
                            │ Cryptographic & Storage Layer
┌───────────────────────────▼────────────────────────────┐
│               Audit & Persistence Invariants           │
│  - Append-only SHA-256 Audit Chain                     │
│  - Read-Only Evidence Source Handles                   │
│  - Fail-Closed Hash Validation                         │
└────────────────────────────────────────────────────────┘
```

---

## 2. Authentication & Closed Identity Management

### 2.1 Closed Registration Model
- **Zero Self-Registration**: LocardX structurally disallows public user registration. The application boots into an uninitialized state where only a single initial Administrator can be provisioned.
- **Closed State Enforcement**: Once the initial Administrator account is created, setup permanently closes. All subsequent user creation must be explicitly performed by an authenticated Administrator.

### 2.2 Password Security (Argon2id)
- Passwords are never stored in plaintext or with legacy hashing algorithms (MD5, SHA-1, or plain SHA-256).
- **Parameters**:
  - Algorithm: **Argon2id** (v19)
  - Memory Cost: **65,536 KiB (64 MiB)**
  - Iterations (Time Cost): **3 passes**
  - Parallelism: **4 lanes / threads**
- Verification latency is ~600 ms, rendering offline brute-force attacks computationally infeasible.

### 2.3 Session Tokens & Time-to-Live
- Authenticated sessions issue cryptographically secure random 256-bit hexadecimal tokens (`Uuid::new_v4()` + CSPRNG entropy).
- Tokens are held in-memory and validated on every IPC command invocation.

---

## 3. Role-Based Access Control (RBAC)

The system enforces 4 hierarchical, non-escalatable roles:

| Module / Operation | Administrator | Investigator | Operator | Viewer |
| :--- | :---: | :---: | :---: | :---: |
| **System Bootstrap & User Provisioning** | **ALLOWED** | DENIED | DENIED | DENIED |
| **User Role Modification / Revocation** | **ALLOWED** | DENIED | DENIED | DENIED |
| **Case Creation & Status Modification** | **ALLOWED** | **ALLOWED** | DENIED | DENIED |
| **Case Evidence Association** | **ALLOWED** | **ALLOWED** | DENIED | DENIED |
| **Forensic Disk Acquisition** | **ALLOWED** | **ALLOWED** | DENIED | DENIED |
| **Deep File Carving & Recovery** | **ALLOWED** | **ALLOWED** | DENIED | DENIED |
| **Drive Sanitization Planning** | **ALLOWED** | DENIED | **ALLOWED** | DENIED |
| **Destructive Erasure Confirmation** | **ALLOWED** | DENIED | **ALLOWED** | DENIED |
| **Generate Forensic Reports** | **ALLOWED** | **ALLOWED** | DENIED | DENIED |
| **Inspect Cases & Read Reports** | **ALLOWED** | **ALLOWED** | **ALLOWED** | **ALLOWED** |
| **Verify Audit Hash Chains** | **ALLOWED** | **ALLOWED** | **ALLOWED** | **ALLOWED** |

Any unauthorized attempt returns an immediate error and registers an `AUTHORIZATION_DENIED` record in the tamper-evident audit ledger.

---

## 4. Physical Drive & System Boot Safety Interlocks

### 4.1 System & Boot Drive Hard-Block
- The `SafetyEngine` queries the OS device manager to determine the drive classification of every attached physical disk:
  - Disks containing the active operating system partition (e.g., `C:\` on Windows, `/` on Linux) are flagged as `SystemDevice`.
  - Disks containing boot partitions (EFI System Partition, Windows Recovery Environment, Active Boot Flag) are flagged as `BootDevice`.
- **Inviolable Invariant**: Any sanitization or destructive operation targeting a `SystemDevice` or `BootDevice` is hard-blocked immediately with reason code `SYSTEM_DEVICE` or `BOOT_DEVICE`. No administrator override exists in software.

### 4.2 Two-Stage Destructive Confirmation
1. **Stage 1 (Challenge Generation)**:
   - The user requests planning for an approved non-system drive.
   - The engine generates a time-limited `ConfirmationChallenge` (TTL: 300 seconds).
   - A high-visibility modal requires explicit checkbox acknowledgement of data destruction.
2. **Stage 2 (Identity Verification)**:
   - The user must manually type the exact physical device identifier (e.g., `\\.\PhysicalDrive1`).
   - The engine verifies the typed string against the challenge target before proceeding.

### 4.3 Time-of-Check to Time-of-Use (TOCTOU) Validation
- Before any sanitization pass begins, the engine re-queries the operating system storage bus.
- If the disk geometry, serial number, vendor, or capacity has changed since planning, the operation fails closed with `TARGET_CHANGED`.

### 4.4 RealHardwareExecutionGate
- In release v0.1.0, physical drive erasure executes in **Simulation Mode** by default.
- Destructive raw hardware sector overwriting is structurally blocked behind the `RealHardwareExecutionGate`, preventing accidental media destruction in lab triage workstations.

---

## 5. Evidence Immutability & Read-Only Invariants

- **Write Prohibition**: All forensic acquisition and carving operations open raw disk handles and evidence container files with strict read-only permissions (`O_RDONLY` / `FILE_READ_DATA`).
- **Fail-Closed Verification**: Before any recovery pass, the evidence image's streaming SHA-256 checksum is calculated. If it differs from the acquisition artifact reference by even a single bit, the job is immediately aborted with `HashMismatch`.
- **Zero-Write Mathematical Proof**: Automated regression tests prove that evidence container file hashes are identical before and after recovery operations.

---

## 6. Cryptographic Audit Trail & Chain Verification

- Every operational event is appended to the `audit_events` database table.
- Each event calculates:
  $$\text{current\_hash} = \text{SHA-256}(\text{previous\_hash} \parallel \text{timestamp} \parallel \text{event\_type} \parallel \text{actor\_id} \parallel \text{target\_id} \parallel \text{details})$$
- The genesis link (`sequence_number = 1`) uses a known root seed.
- If any database record is edited, deleted, or reordered by an attacker or database administrator, `AuditService::verify_chain()` detects the broken link and pinpoints the exact sequence number where tampering occurred.
