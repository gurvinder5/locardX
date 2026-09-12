# LocardX Forensic Acquisition — Operator User Guide
**Module:** Forensic Disk Imaging & Bitstream Acquisition (Step 11)  
**Audience:** Forensic Examiners, Incident Responders, Security Engineers  
**Target Environment:** LocardX Desktop Application  

---

## 1. Introduction

The **Forensic Acquisition** module in LocardX provides non-destructive, bit-for-bit physical disk acquisition into standard raw DD image format (`.raw` / `.dd`). It is designed for forensic preservation, digital investigations, and evidential ingestion into the LocardX Recovery Module.

Key Operational Guarantees:
- **Zero modification of source media**: Physical disk handles are opened strictly read-only (`GENERIC_READ` on Windows, `O_RDONLY` on Linux).
- **Simultaneous cryptographic integrity**: Computes SHA-256 in a single streaming pass as sectors are acquired.
- **Fail-closed evidence protection**: Any aborted or interrupted acquisition is purged from disk, preventing incomplete files from entering evidence repositories.

---

## 2. Navigating to Forensic Acquisition

1. Launch the LocardX desktop application.
2. In the top navigation bar, click on **Forensic Acquisition** (marked with the blue **Download** icon).
3. The interface displays:
   - **Read-Only Source Guarantee Badge**: Confirms no write operations can reach the source disk.
   - **Source Selection Table**: Lists detected physical storage drives.
   - **Destination & Configuration Panel**: Specifies target `.raw` output path and buffer size.
   - **Telemetry Dashboard**: Displays real-time throughput, percentage, and elapsed/remaining time.

---

## 3. Step-by-Step Acquisition Workflow

### Step 1: Selecting the Source Physical Disk
1. Review the detected devices in the **1. Select Physical Source Device** list.
2. Select the target physical device (e.g. `\\.\PhysicalDrive1` or Kingston USB drive).
3. Confirm drive properties: Model, Serial Number, Media Type, and Capacity.
> **Note**: Logical drive letters (such as `C:\` or `D:\`) are strictly disallowed as acquisition sources to ensure low-level sector-level completeness, including partition tables, unallocated space, and volume slack.

### Step 2: Configuring Destination & Parameters
1. Enter the full path for the destination image file in **Destination Image File** (e.g. `D:\Forensics\case_001_disk.raw`).
2. The pre-flight validator automatically checks:
   - Destination file path syntax and directory writability.
   - Available free space on the destination volume (must equal source drive capacity + 100 MiB margin).
   - Collision check: Ensures the destination path does **not** reside on the same physical drive being acquired.
3. Select the **Chunk Read Buffer** (default: `1 MiB (Recommended)`).
4. If the destination file already exists, check **Allow Overwriting Existing File** only if intentional.

### Step 3: Generating and Reviewing the Acquisition Plan
1. Click **Generate Forensic Acquisition Plan**.
2. Review the plan summary:
   - Source snapshot identity and exact byte capacity.
   - Destination file path and image format (`raw`).
   - Single-pass streaming SHA-256 confirmation.

### Step 4: Executing the Acquisition
1. Click **Execute Read-Only Forensic Acquisition**.
2. The **Acquisition in Progress** panel activates:
   - **Progress Bar**: Live acquisition percentage.
   - **Throughput**: Current transfer speed in MB/s.
   - **Elapsed & Remaining Time**: Real-time counter and estimated time to completion.
3. If necessary, click **Cancel Acquisition** to cooperatively abort. Aborted operations automatically delete partial files to preserve evidence purity.

### Step 5: Verification & Evidential Handoff
1. Upon successful completion:
   - **SHA-256 Hash**: The calculated SHA-256 checksum is displayed in monospace. Click **Copy Hash** to copy it to the clipboard for evidence documentation.
   - **Total Image Size**: Exact byte count verified against source disk capacity.
   - **Audit Reference**: Cryptographic hash recorded in the immutable audit chain.
2. Click **Verify Image Disk Integrity On-Demand** at any time to re-verify the on-disk image file against the recorded SHA-256 digest.
3. The generated `AcquisitionArtifact` is now registered in SQLite and ready for handoff to the **Forensic Recovery Module**.

---

## 4. Crash Recovery & Power Disruption Behavior

In the event of an unexpected system reboot, power outage, or OS crash during an active acquisition:
- LocardX's startup sweep automatically detects the incomplete acquisition.
- The record is marked as `Failed` with reason `Interrupted`.
- It can **never** be mistakenly verified or handed off as a valid evidential artifact.
