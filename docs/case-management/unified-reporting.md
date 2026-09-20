# Unified Forensic & Sanitization Reporting Architecture (Step 13C)

## 1. Executive Summary

LocardX Step 13C provides an automated, tamper-evident reporting engine that aggregates data from all workstation modules into court-admissible forensic certificates and technical case summaries.

The reporting engine is implemented in `crates/reporting` and integrated with `crates/case-management`, `crates/audit`, and the Tauri IPC layer.

---

## 2. Canonical Digest & Integrity Verification

To prevent post-generation tampering or alteration of evidence certificates, every generated `CaseForensicReport` is cryptographically sealed with a **Canonical SHA-256 Digest**:

$$\text{Report Digest} = \text{SHA-256}(\text{CaseID} \parallel \text{Title} \parallel \text{Acquisitions} \parallel \text{Recoveries} \parallel \text{Erasures} \parallel \text{Custody} \parallel \text{AuditRoot})$$

### 2.1 Canonical Serialization Rules
1. **Case Info**: Case ID, Reference, Title, Lead Investigator, Status.
2. **Acquisitions**: For each acquisition sorted by acquisition ID:
   - `ID:OperationID:DeviceID:Format:SizeBytes:ImageSHA256`
3. **Recoveries**: For each recovery sorted by job ID:
   - `JobID:OperationID:AcquisitionID:SourceSHA256:FilesRecovered`
4. **Erasures**: For each erasure sorted by operation ID:
   - `OperationID:DeviceID:Method:Status:VerificationOutcome:EvidenceDigest`
5. **Custody**: For each custody item in chronological order:
   - `CustodyID:EventType:ActorID:Timestamp:Action:AuditHash`
6. **Audit Anchor**: `AuditValid:TotalEvents:AuditRootHash`

### 2.2 Verification API
The `verify_case_report(report)` method executes:
```rust
let is_intact = forensic_generator.verify_report_integrity(&report);
```
If any field in the report (such as file count, image hash, or custody entry) is modified after generation, the recomputed digest diverges from `report.integrity.report_digest`, and verification fails closed.

---

## 3. Supported Report Formats

### 3.1 Structured JSON (`CaseForensicReport`)
Exposed over Tauri IPC for desktop UI rendering, verification, and automated forensic lab pipeline ingestion.

### 3.2 Markdown Forensic Summary
Formatted for readability in reports, documentation binders, and incident response tracking:
- Header with Case Reference, Investigator, and Status.
- Executive Investigation Summary.
- Hardware & Physical Evidence Inventory.
- Bitstream Forensic Acquisition Ledger with SHA-256 Hashes.
- File Recovery Findings & Confidence Breakdown.
- Sanitization & NIST SP 800-88 Disposal Records.
- Chain of Custody Table with Audit Sequence Numbers.
- Cryptographic Verification Block with Canonical Report Digest.

### 3.3 Monospaced Plain Text Certificate
A standardized, 80-column fixed-width forensic certificate suitable for printing, cryptographically signing, or archiving in cold storage alongside disk images:
- ASCII border decoration and alignment.
- Explicit disclaimers regarding read-only handling and fail-closed safety interlocks.
- Digital signature blocks for the lead investigator and technical peer reviewer.

---

## 4. Multi-Module Aggregation Model

When `generate_case_report(case_id)` is invoked:
1. Queries `cases` for case reference and investigator details.
2. Queries `case_operations` to identify all linked operations.
3. Automatically fetches:
   - Records from `acquisitions` for all linked `ForensicAcquisition` operations.
   - Records from `recovery_jobs` and `recovered_files` for all linked `Recovery` operations.
   - Records from `drive_erasure_records` for all linked `DriveErasure` operations.
4. Gathers all `case_custody` entries for the case.
5. Performs a live verification of the system audit chain via `audit_service.verify_chain()`.
6. Computes the canonical report digest.
7. Persists the report summary in `case_reports` with its canonical digest and timestamp.
8. Logs a `CASE_REPORT_GENERATED` event into the system audit trail, sealing the generation event into the hash chain.
