# LocardX Forensic Workstation — User Manual

## 1. Introduction

**LocardX** is a unified digital forensic workstation application engineered for forensic practitioners, digital investigators, and incident responders. It provides mathematically verifiable chain of custody, hardware-interlocked sanitization planning, bitstream raw disk acquisition, deep signature-based carving, and tamper-evident unified reporting.

### Core Capabilities:
- **Unified Case Management**: Track evidential assets, cross-module operations, and chronological chains of custody.
- **Forensic Disk Acquisition**: Bitstream raw DD disk imaging with streaming SHA-256 verification.
- **Advanced File Recovery**: Deep signature carving with multi-factor confidence scoring (JPEG, PNG, PDF, ZIP, DOCX, SQLite, Text).
- **Secure Drive Sanitization**: Standards-compliant media-aware erasure planning (NIST 800-88, DoD 5220.22-M) with simulation mode and fail-closed safety interlocks.
- **Closed Identity & Access Control**: Role-based access control (Administrator, Investigator, Operator, Viewer) with Argon2id password protection.
- **Tamper-Evident Audit Ledger**: SHA-256 cryptographic hash chain verifying all actions taken across the system.

---

## 2. Initial Setup & First-Time Bootstrap

### 2.1 Starting the Application
When LocardX is launched for the first time, no user accounts exist. The application enters **Bootstrap Mode**.

1. The login screen detects an uninitialized workstation and displays the **Create Initial Administrator** form.
2. Enter the desired administrator credentials:
   - **Username**: Must be alphanumeric (`admin`, `lead_examiner`).
   - **Password**: Minimum 12 characters, requiring uppercase, lowercase, numeric, and special characters.
   - **Confirm Password**: Must match exactly.
   - **Display Name**: Optional formal name or badge identifier.
3. Click **Initialize Workstation**.
4. Once completed, the registration system automatically closes permanently. No public self-registration is permitted.

---

## 3. User Management & Roles

Administrators can provision user accounts under the **Settings > User Management** panel.

### User Roles & Permissions:
- **Administrator**: Complete system control; manages users, views audit ledgers, configures global settings.
- **Investigator**: Creates and manages cases, executes acquisitions, performs file recovery, generates forensic reports.
- **Operator**: Plans and executes sanitization workflows (simulation), reviews drive health, exports sanitization certificates.
- **Viewer**: Read-only auditor access; inspects cases, reads reports, verifies audit chains. Cannot initiate destructive actions or create cases.

---

## 4. Investigation Case Workflow

### 4.1 Creating an Investigation Case
1. Navigate to **Cases** from the sidebar.
2. Click **New Case**.
3. Fill out the mandatory case attributes:
   - **Case Reference Number** (e.g., `CASE-2026-0042`)
   - **Case Title** (e.g., `External Storage Triage — Device Alpha`)
   - **Description**
   - **Agency / Department Metadata**
4. Click **Create Case**. The case opens in `Open` status with the current user assigned as the lead investigator.

### 4.2 Associating Evidence & Operations
Within the case detail view:
- **Add Physical Storage**: Register seized hard drives, SSDs, or thumb drives with physical serial numbers and capacity.
- **Associate Acquisition**: Link an existing bitstream acquisition operation to the case.
- **Associate Recovery**: Link a completed carving job to the case.
- **Record Custody Event**: Record transfers of evidence, physical movements, or technical integrity checks.

---

## 5. Forensic Disk Acquisition

1. Connect the suspect storage media via an approved hardware write-blocker.
2. Navigate to **Acquisition**.
3. Select the source physical storage device from the discovered devices list.
4. Set the destination path on a verified evidence volume (e.g., `D:\Evidence\Image.raw`).
5. Choose the bitstream output format:
   - **Raw DD (`.raw`, `.dd`)**: Uncompressed forensic clone.
6. Click **Start Acquisition**.
7. LocardX reads the physical drive block-by-block, displaying live throughput, elapsed time, and streaming SHA-256 computation.
8. Upon completion, the system automatically compares the live readback hash with the destination file hash to ensure 100% bitstream fidelity.

---

## 6. File Recovery & Carving

1. Navigate to **File Recovery**.
2. Select an acquired raw evidence image (`.raw` / `.dd`).
3. Configure the recovery parameters:
   - **Recovery Mode**:
     - *All (Filesystem Metadata + Raw Carving)*
     - *Raw Carving Only*
     - *Filesystem Only*
   - **Target File Formats**: Choose specific types (Images, Documents, Databases, Archives) or scan all supported signatures.
   - **Minimum Confidence Score**: Filter candidates by score threshold (default: 50).
   - **Output Directory**: Target directory where recovered files will be organized into subfolders (`images/`, `documents/`, etc.).
4. Click **Execute Recovery**.
5. The recovery engine performs sliding window carving, validates internal header/footer structures, computes multi-factor confidence grades (High, Medium, Low, Uncertain), and saves extracted files.
6. Review the recovered file catalog with inline metadata, SHA-256 hashes, and confidence factors.

---

## 7. Secure Drive Sanitization (Simulation)

> [!IMPORTANT]
> To prevent catastrophic data loss during operational triage, LocardX executes drive sanitization in **Simulation Mode** by default. System and boot volumes are permanently blocked by kernel-level safety interlocks.

1. Navigate to **Drive Sanitization**.
2. Select the target physical drive. Notice that any disk containing active OS volumes (e.g., `C:\`) is flagged as **BLOCKED** and cannot be selected.
3. Select a recognized sanitization standard:
   - **NIST SP 800-88 Rev. 1 Clear (Single-Pass Zero)**
   - **NIST SP 800-88 Rev. 1 Clear (Random Overwrite)**
   - **DoD 5220.22-M (3-Pass Multi-Stage)**
4. Click **Generate Sanitization Plan**.
5. **Two-Stage Destructive Confirmation**:
   - The workstation displays a high-contrast red confirmation challenge.
   - You must check the warning acknowledgement box.
   - You must manually type the exact target drive path (e.g., `\\.\PhysicalDrive1`).
6. Click **Confirm & Simulate**.
7. The simulated execution runs the selected algorithm passes, performs post-erasure sample readback verification, and generates a formal **Sanitization Certificate**.

---

## 8. Unified Forensic Reports & Audit Trail

### 8.1 Generating Case Forensic Reports
1. Open the target Case.
2. Click **Generate Final Report**.
3. LocardX compiles all associated acquisitions, carved file statistics, confidence distributions, chain of custody timelines, and the current audit hash into a canonical JSON report.
4. The report computes an immutable cryptographic SHA-256 digest (`report_digest`).
5. Click **Verify Integrity** at any time to recompute the canonical digest and prove the report has not been modified.

### 8.2 Inspecting Audit Ledgers
1. Navigate to **Audit Ledger**.
2. Every action taken across all modules is listed with sequence numbers, UTC timestamps, actor usernames, and event types.
3. Click **Verify Audit Hash Chain**: LocardX scans every sequential record from `seq=1` to the current link, verifying that `current_hash = SHA256(previous_hash || payload)`. A green badge confirms that the audit chain is 100% valid and unbroken.
