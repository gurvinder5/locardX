# Deterministic Evidential Confidence Scoring Model
**LocardX Forensic Recovery -- Evidence Factor Specification**

---

## 1. Principles of Forensic Confidence

Unlike heuristic or probabilistic carvers that assign subjective recovery ratings, LocardX employs a strictly deterministic, evidence-factor model. Every point in the 0--100 confidence score corresponds directly to an observable, reproducible cryptographic or structural property of the evidence.

$$Score = 	ext{clamp}\left(\sum_{k=1}^N w_k, 0, 100ight)$$

---

## 2. Factor Weights & Penalty Hierarchy

| Evidence Factor | Factor Code | Weight ($w_k$) | Condition / Forensic Justification |
| :--- | :--- | :--- | :--- |
| **Magic Header Match** | `HEADER_MAGIC_MATCH` | **+20** | Magic bytes match expected format specification at candidate origin |
| **Header Corrupted/Missing** | `HEADER_MAGIC_MATCH` | **-50** | Expected magic bytes absent or mutilated |
| **Valid Footer / Trailer** | `FOOTER_MATCH` | **+20** | Format EOF delimiter (e.g. `FF D9`, `IEND`, `%%EOF`, `PK 05 06`) verified |
| **Footer Anomalous/Missing**| `FOOTER_MATCH` | **-20** | Trailer marker absent or out of sequence |
| **Complete Structure Valid** | `STRUCTURE_VALIDATION` | **+25** | All internal chunks, markers, page headers, or tables parse without defect |
| **Partially Valid Structure**| `STRUCTURE_VALIDATION` | **+10** | Core markers valid but non-fatal warnings present |
| **Truncated / Incomplete** | `STRUCTURE_VALIDATION` | **-20** | Premature EOF before structural termination reached |
| **Corrupted Structure** | `STRUCTURE_VALIDATION` | **-35** | Parser rejected payload due to malformed metadata structures |
| **Filesystem Correlation** | `FILESYSTEM_METADATA` | **+15** | Candidate offset and size correlate with an NTFS MFT or FAT32 directory entry |
| **Cluster-Contiguous Run** | `CLUSTER_CONTIGUOUS` | **+10** | Continuous sequential sector run on cluster boundaries without fragmentation |
| **Discontinuous Clusters** | `CLUSTER_CONTIGUOUS` | **-10** | Discontinuous physical sectors requiring fragment assembly |

---

## 3. Qualitative Confidence Grades

Based on the calculated numerical score, files are classified into four qualitative forensic grades:

| Grade | Score Range | Evidential Admissibility & Forensic Interpretation |
| :--- | :--- | :--- |
| **High** | $85 \le 	ext{Score} \le 100$ | Completely intact file with verified headers, internal chunks, footer, and/or filesystem cross-reference. Suitable for primary evidential presentation. |
| **Medium** | $65 \le 	ext{Score} \le 84$ | Structurally sound file carved from unallocated space without active directory correlation, or with minor non-fatal warnings. |
| **Low** | $40 \le 	ext{Score} \le 64$ | Partially reconstructed file with missing non-essential metadata or minor trailing truncation. |
| **Uncertain** | $0 \le 	ext{Score} \le 39$ | Severely truncated, damaged, or unverified candidate. Filtered by default unless permissive carving is configured. |

---

## 4. Evidence Factor Transparency

Every `RecoveredFile` persists its complete array of `EvidenceFactor` items:

```json
[
  {
    "factor_type": "HEADER_MAGIC_MATCH",
    "weight": 20,
    "description": "Validated magic signature header for JPEG",
    "passed": true
  },
  {
    "factor_type": "FOOTER_MATCH",
    "weight": 20,
    "description": "Located valid EOF/trailer signature",
    "passed": true
  },
  {
    "factor_type": "STRUCTURE_VALIDATION",
    "weight": 25,
    "description": "Internal structure fully validated: SOI marker verified; EOI marker located; SOF/DQT internal markers validated",
    "passed": true
  },
  {
    "factor_type": "CLUSTER_CONTIGUOUS",
    "weight": 10,
    "description": "Contiguous physical cluster run confirmed",
    "passed": true
  }
]
```
