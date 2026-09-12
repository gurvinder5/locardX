# LocardX Forensic Case Management Architecture (Step 13A)

## 1. Executive Overview

LocardX Step 13 establishes the Unified Case, Audit, and Reporting layer for the digital forensics and sanitization workstation. It unifies:
- **Physical Drive Sanitization (Step 10)**: ATA/NVMe sanitize operations, NIST SP 800-88 compliance records, post-wipe verification.
- **Forensic Disk Acquisition (Step 11)**: Bitstream Raw DD imaging, streaming SHA-256 calculation, source drive geometry.
- **Advanced File Recovery (Step 12)**: Read-only image validation, The Sleuth Kit filesystem traversal, signature file carving, confidence scoring.
- **Tamper-Evident Ledger (Step 13B & 13C)**: Unified cross-module audit trail, cryptographic chain-of-custody tracking, canonical report digest generation.

---

## 2. Core Architectural Principles & Invariants

The case management subsystem enforces strict digital forensic principles:

### 2.1 Non-Destructive Invariant
- **Zero Modification**: The case subsystem NEVER modifies, overwrites, truncates, or deletes evidential disk images, raw bitstreams, or target drives.
- **Read-Only Preservation**: Evidence records reference immutable source locations and physical disk identifiers.
- **Permanent Retention on Case Close**: Closing or archiving an investigation case (`status = 'completed'` or `'archived'`) marks the metadata record timestamp (`closed_at`), but **NEVER deletes or purges** associated evidence, operations, audit trails, or reports.

### 2.2 Relational Integrity & Schema Design
The subsystem is backed by SQLite with full foreign key constraints and transactional integrity:

```
                      +-------------------+
                      |      cases        |
                      +-------------------+
                         |    |     |   |
          +--------------+    |     |   +---------------+
          |                   |     |                   |
          v                   v     v                   v
+------------------+ +-------------+ +---------------+ +---------------+
| case_operations  | |case_evidence| | case_custody  | | case_reports  |
+------------------+ +-------------+ +---------------+ +---------------+
        |                                   |                   |
        v                                   v                   v
[operations table]                  [audit_events]      [canonical sha256]
```

#### Entities:
1. **`cases`**:
   - `case_id` (TEXT PRIMARY KEY): Unique UUID identifier.
   - `case_reference` (TEXT UNIQUE): Human-readable reference (e.g. `CASE-2026-001`).
   - `title` (TEXT NOT NULL): Short descriptive title.
   - `description` (TEXT): Case scope and investigation background.
   - `status` (TEXT NOT NULL): Lifecycle state (`open`, `in_progress`, `completed`, `archived`).
   - `lead_investigator` (TEXT NOT NULL): Username or operator ID of the primary examiner.
   - `created_at`, `updated_at`, `closed_at` (TEXT NOT NULL/NULL): ISO 8601 timestamps.
   - `metadata_json` (TEXT): Flexible structured metadata.

2. **`case_operations`**:
   - Links forensic actions across modules to a specific case.
   - `operation_type`: `ForensicAcquisition`, `Recovery`, `DriveErasure`, `FileErasure`, `IntegrityCheck`.
   - Automatic cascade: When an operation completes, the module associates the operation to the active case if configured.

3. **`case_evidence`**:
   - Identifies evidential items introduced into the investigation.
   - `evidence_type`: `physical_storage`, `acquisition_image`, `recovered_dataset`, `logical_file`.
   - `identifier`: Path (`/evidence/disk1.dd`) or Win32 device ID (`\\.\PhysicalDrive1`).
   - `sha256`: Cryptographic bitstream digest (optional for live physical hardware before acquisition).

4. **`case_custody`**:
   - Append-only chain-of-custody ledger recording every evidential touch.
   - Tied directly to the system-wide cryptographic `audit_events` hash chain.

5. **`case_reports`**:
   - Persisted forensic certificates and executive summaries with verified canonical SHA-256 digests.

---

## 3. Workstation Integration Workflow

```
1. Case Registration (Investigator logs reference e.g. "CASE-2026-ALPHA")
   │
2. Physical Evidence Logging (Win32 PhysicalDrive1 logged with serial/capacity)
   │
3. Read-Only Forensic Acquisition (Step 11 writes Raw DD image + streaming SHA-256)
   │──> Auto-associates Acquisition Operation & DD Image Evidence to Case
   │──> Appends Custody Event ("evidence_acquired")
   │
4. Carving & File Recovery (Step 12 validates DD image SHA-256 and extracts files)
   │──> Auto-associates Recovery Job to Case
   │──> Appends Custody Event ("evidence_analyzed")
   │
5. Target Sanitization (Step 10 executes NIST SP 800-88 Purge/Clear on media)
   │──> Auto-associates Erasure Operation to Case
   │──> Records post-wipe verification proof
   │
6. Unified Forensic Certificate Generation (Step 13C computes canonical digest)
   │──> Validates entire audit hash chain
   │──> Emits tamper-evident Markdown & Plain Text certificates
```

---

## 4. Security & Role-Based Access Controls

All operations require an active session token authenticated via `locardx-auth`:
- **Administrators**: Can create, modify, assign, close, or archive any case; full access to user management and audit trails.
- **Investigators**: Can create cases, associate operations, introduce evidence, record chain-of-custody actions, and generate signed reports.
- **Operators**: Can view case summaries, link authorized operational executions, and view evidence items.
- **Session Expiry & Fail-Closed**: Unauthenticated requests or expired sessions are rejected immediately with standard `LocardError::Authentication` responses.
