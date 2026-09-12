# Forensic Evidence Preservation & Integrity Controls
**LocardX Module Specification -- Step 12 Evidential Standards**

---

## 1. Principles of Evidential Preservation

The core doctrine of LocardX is that forensic analysis must never alter the original physical evidence or evidential disk images. The Recovery Module adheres strictly to ISO/IEC 27037 and NIST SP 800-86 standards:

1. **Non-Destructive Read-Only Access**: All physical disk access is read-only. Forensic raw DD images are opened with read-only file descriptors.
2. **Fail-Closed Tamper Detection**: Before analysis begins, streaming SHA-256 over the entire image must match the acquisition digest recorded in `AcquisitionArtifact`. Any difference halts the workflow immediately.
3. **No In-Place Modifications**: Recovered files and reports are written to an isolated external export directory.
4. **No Synthetic Data Fabrication**: Gaps or missing sectors in fragmented files are flagged as incomplete or partially valid; the engine never fabricates or hallucinates content.

---

## 2. Database Schema (Migration 011)

The recovery workflow state, validated evidence sources, recovered files, and forensic reports are persisted across five relational tables in SQLite:

- `recovery_sources`: Stores verified acquisition sources, image paths, capacities, SHA-256 hashes, and hardware identities.
- `recovery_jobs`: Tracks every recovery operation lifecycle (`Pending` -> `Scanning` -> `Completed` / `Cancelled` / `Failed`), bytes scanned, candidates evaluated, and failure reasons.
- `recovered_files`: Stores all extracted files, source byte offsets, sizes, MIME types, validation statuses, confidence scores, and individual evidence factors.
- `recovery_fragments`: Tracks discrete cluster segments for fragmented file reconstructions.
- `recovery_reports`: Stores immutable summary reports and cryptographic report digests.

---

## 3. Tamper-Evident Audit Logging

All recovery operations append structured, cryptographically linked events to the SHA-256 audit chain:

| Audit Event Type | Trigger | Logged Details |
| :--- | :--- | :--- |
| `RECOVERY_SOURCE_VALIDATED` | Source validation completes | Acquisition ID, image path, size, SHA-256 hash |
| `RECOVERY_PLANNED` | Pre-flight plan created | Mode, target types, minimum confidence threshold |
| `RECOVERY_STARTED` | Engine begins scanning | Job ID, operation ID, source path |
| `RECOVERY_CANCEL_REQUESTED` | Operator triggers cancel | Operation ID, requesting operator |
| `RECOVERY_COMPLETED` | Execution finishes | Files recovered, bytes scanned, elapsed duration |
| `RECOVERY_CANCELLED` | Execution aborted | Partial bytes scanned, elapsed duration |
| `RECOVERY_FAILED` | Fail-closed error encountered| Structured failure reason code |
| `RECOVERY_REPORT_GENERATED`| Final report persisted | Report ID, job ID, cryptographic report digest |

---

## 4. Cryptographic Report Digest

Upon recovery completion, the engine generates an immutable forensic summary report. The report calculates a SHA-256 report digest over the canonical string:

$$\text{ReportDigest} = \text{SHA256}\left(\text{report\_id} \mathbin{\Vert} \text{job\_id} \mathbin{\Vert} \text{acquisition\_id} \mathbin{\Vert} \text{files\_recovered} \mathbin{\Vert} \text{average\_confidence}\right)$$

This digest is committed to the database and recorded in the hash-chained audit log, providing undeniable cryptographic proof of investigation findings.
