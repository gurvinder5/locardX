# Technical Chain of Custody & Audit Architecture (Step 13B)

## 1. Principle of Digital Chain of Custody

In digital forensics, the chain of custody establishes the chronological history and evidential custody of digital media. It proves:
1. **Who** handled the evidence (Actor / Operator ID).
2. **When** the handling took place (Cryptographically recorded ISO 8601 timestamp).
3. **What** action was performed (Acquisition, Verification, Carving, Sanitization).
4. **Integrity**: That the digital evidence remained unmodified and bit-identical from the point of seizure through analysis to final reporting.

---

## 2. Cryptographic Hash-Chain Mechanism

The LocardX audit subsystem (`crates/audit`) provides an append-only, tamper-evident hash-chained ledger backed by SQLite.

### 2.1 Hash Chain Formula
Each event in `audit_events` is cryptographically bound to its predecessor:

$$H_0 = \text{SHA-256}(\text{"GENESIS"} \parallel \text{Initial Seed})$$

$$H_i = \text{SHA-256}(H_{i-1} \parallel \text{seq}_i \parallel \text{event\_id}_i \parallel \text{type}_i \parallel \text{timestamp}_i \parallel \text{actor}_i \parallel \text{target}_i \parallel \text{details}_i)$$

Where:
- $H_i$ is `current_hash`.
- $H_{i-1}$ is `prev_hash`.
- $\text{seq}_i$ is the monotonically increasing 1-indexed `sequence_number`.
- $\parallel$ represents byte-level canonical concatenation with length delimiters.

### 2.2 Tamper Evident Verification
The verification algorithm traverses all records in sequence from sequence 1 to $N$:
1. Checks that sequence numbers are strictly contiguous ($1, 2, 3, \dots, N$).
2. Validates that record $i$'s `prev_hash` equals record $i-1$'s `current_hash`.
3. Recomputes $H_i$ from the row's raw values and asserts equality with `current_hash`.
4. If any bit in the database is modified, inserted, or deleted out-of-band:
   - The chain immediately fails verification at the exact sequence number (`broken_sequence`).
   - The UI and reporting engine flag the ledger as `Audit Integrity Compromised`.

---

## 3. Case Custody Events & Lifecycle

The `case_custody` table logs granular custody transactions specifically tied to cases and evidence items:

| Event Type | Description | Associated Modules |
| :--- | :--- | :--- |
| `evidence_introduced` | Initial registration of physical disk or external image into case | Workstation UI / Seizure Log |
| `evidence_acquired` | Bitstream raw DD image generated and verified | Step 11 Forensic Acquisition |
| `evidence_verified` | Independent SHA-256 hash re-verification performed | Step 10 & 11 Integrity Service |
| `evidence_analyzed` | Filesystem traversal and signature carving conducted | Step 12 File Recovery |
| `evidence_exported` | Extracted file or forensic artifact exported to external media | Step 12 Recovery Exporter |
| `report_generated` | Final case certificate or forensic summary compiled | Step 13C Forensic Reporting |
| `case_status_changed` | Case transitioned between Open, In Progress, and Completed | Case Management Lifecycle |

### 3.1 Dual-Anchoring
Every custody record in `case_custody` references:
1. `audit_event_id`: The primary key of the corresponding system audit log event.
2. `audit_hash`: The SHA-256 cryptographic hash of that event at the time of creation.

This links case-specific notes directly to the immutable system audit chain.

---

## 4. Chronological Case Timeline

The Case Management Service (`locardx-case-management`) provides `get_case_timeline()` which synthesizes:
- All `case_custody` records for the case.
- All system `audit_events` whose `target_ref` matches the case reference (e.g. `CASE-2026-001`) or any associated operation ID or evidence ID.

The result is merged and sorted in strict chronological order ($t_1 \le t_2 \le \dots \le t_m$), providing examiners and court evaluators with a continuous audit stream of every action executed under the authority of the investigation.
