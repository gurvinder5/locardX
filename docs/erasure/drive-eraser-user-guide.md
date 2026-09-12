# LocardX Secure Drive Eraser — Operator & Lab User Guide

## 1. Overview & Forensic Objectives

The **LocardX Secure Drive Eraser** is an enterprise forensic sanitization engine built to sanitize physical secondary storage devices in full compliance with NIST SP 800-88 Rev. 1, DoD 5220.22-M, and modern ATA/NVMe controller standards.

### Core Capabilities:
- **Media-Aware Sanitization:** Differentiates rotational magnetic media (HDD), SATA solid-state drives (SSD), NVMe PCIe drives, and removable flash (USB/SD).
- **Absolute Host Protection:** Unconditionally blocks sanitization of host operating system drives, boot/EFI partitions, and logical filesystem volumes.
- **Fail-Closed Verification:** 100% full sequential or sampled sector readback verification; indeterminate results fail closed as `UnableToVerify`.
- **Tamper-Evident Reporting:** Generates cryptographic SHA-256 canonical report digests tied directly to the immutable LocardX audit chain.
- **Dual Execution Modes:** Deterministic, non-destructive **Simulation Mode** for training/validation, and **Real Hardware Mode** for physical drive repurposing/decommissioning.

---

## 2. Fundamental Safety Invariants

Before operating the Drive Eraser, operators must understand the non-negotiable safety boundaries enforced at the kernel and engine levels:

1. **Physical Devices Only (Pillar 1 vs. Pillar 2):**
   - Whole-disk sanitization operates exclusively on raw physical device identifiers (e.g. `\\.\PhysicalDrive1` on Windows, `/dev/sdb` on Linux).
   - Logical volume targets (e.g., `C:\`, `D:\Data`, `/home`) are **strictly rejected**. Single-file or directory wiping must be conducted through the File Eraser (Pillar 2).
2. **System & Boot Hard Blocking:**
   - Active OS disks (`PhysicalDrive0`, `/dev/nvme0n1` hosting `/`), boot partitions, and EFI system partitions are identified via volume extent mapping and blocked unconditionally.
   - No administrative role, user override, or configuration flag can bypass this block.
3. **Two-Stage Authorization:**
   - Destructive real hardware sanitization requires explicit operator acknowledgment of permanent data loss, accompanied by exact manual re-typing of the physical device identifier.
4. **Fail-Closed on Disruption:**
   - Interrupted, aborted, or power-cycled operations NEVER resume automatically. They transition to `Unknown` / `UnableToVerify` on startup.

---

## 3. Operating Modes

### 3.1 Simulation Mode (Default / Safe Mode)
- **Visual Indicator:** Amber banner at top of UI.
- **Behavior:** Deterministic execution simulation. Calculates transfer timings, models sector overwrite passes, emits real-time progress events, and generates simulation-marked forensic certificates.
- **Safety Guarantee:** Zero physical storage handles or destructive IOCTLs are opened. Safe to run on any workstation.

### 3.2 Real Hardware Mode (Production Purge)
- **Visual Indicator:** Rose / Red warning banner with "PRODUCTION HARDWARE BACKEND ACTIVE".
- **Behavior:** Acquires exclusive device access, dismounts volume extents, writes unbuffered direct sectors or issues native controller sanitize commands, and performs readback verification.
- **Privilege Requirement:** Requires elevated Administrator privileges on Windows (UAC elevated) or root privileges on Linux (`geteuid() == 0`).

---

## 4. Step-by-Step Operator Workflow

### Step 1: Device Discovery & Selection
1. Navigate to **Drive Eraser** from the main navigation sidebar.
2. Select the **Device Source**:
   - **Real Devices:** Enumerates physical drives physically connected to the workstation.
   - **Mock Test Devices:** Populates mock test drives for lab validation and UI testing.
3. Choose the target physical storage device from the dropdown list.
   - System and boot drives are indicated with a red shield and disabled from selection.
   - Only secondary, non-system physical drives are selectable.

### Step 2: Capability Inspection
1. The hardware capability engine immediately probes the selected physical drive in read-only mode.
2. Review the detected parameters:
   - **Media Classification:** Rotational HDD, SATA SSD, NVMe SSD, or Removable Flash.
   - **Bus Interface:** SATA, NVMe, USB, SCSI.
   - **Sector Geometry:** Logical and physical sector sizing (e.g., 512B / 4096B).
   - **Hardware Capabilities:** ATA Secure Erase, NVMe Crypto Erase, Sequential Overwrite.

### Step 3: Plan Generation
1. Click **Plan Drive Erasure**.
2. The planning engine selects the optimal, standards-compliant sanitization method for the media type:
   - Rotational HDD: `Nist80088ClearZero` (Single-pass 0x00 overwrite).
   - SATA SSD: `AtaSecureErase` or `AtaSanitize` (Firmware controller purge).
   - NVMe SSD: `NvmeCryptoErase` (Cryptographic key destruction) or `NvmeFormatSanitize`.
   - Removable Flash: `Nist80088ClearZero` with sampled sector verification.
3. Review the generated plan: pass count, estimated duration, risk rating, and forensic verification plan.

### Step 4: Two-Stage Safety Authorization
1. Click **Authorize Erasure**.
2. A modal dialog opens presenting full device details, capacity, and warning text.
3. Check the acknowledgment checkbox:
   - *"I confirm that I want to physically erase [device_id]... I understand all data cannot be recovered."*
4. Type the exact device path into the confirmation input (e.g., `\\.\PhysicalDrive1`).
5. Click **Execute Sanitization**.

### Step 5: Execution & Live Telemetry
1. The engine acquires an exclusive device lock (`DeviceLockRegistry`).
2. The UI displays real-time progress telemetry:
   - Percent complete (0% to 100%)
   - Processed bytes / total bytes
   - Current execution stage (Exclusive locking, Sector write, Readback verification)
   - Elapsed time and ETA

### Step 6: Verification & Tamper-Evident Certificate
1. Upon sanitization completion, the post-sanitization forensic verification routine runs automatically.
2. Verify the outcome badge: **Verified** (Green).
3. Click **View Sanitization Certificate**:
   - Inspect the **Report ID**, **Canonical Report SHA-256 Digest**, and **Audit Chain Reference**.
   - Review the verified device serial number and byte count.
   - Click **Download Certificate (.md)** to export the certificate to disk.
   - Click **Copy Forensic Record** to copy the full JSON audit record to the clipboard.

---

## 5. Administrative Privilege & Environment Setup

### Windows
- To execute real hardware drive erasure on Windows, LocardX must run elevated:
  - Right-click `LocardX.exe` -> **Run as administrator**.
  - If started without elevation, a warning banner appears when toggling to Real Hardware mode, and hardware lock acquisition will return an explicit Access Denied error.

### Linux
- Direct block device access requires superuser privileges:
  ```bash
  sudo ./locardx-desktop
  ```
- If run without root, the privilege check returns non-elevated and execution fails closed.

---

## 6. Crash Recovery & Resumption Handling

If the workstation loses power, crashes, or reboots mid-sanitization:

1. **Automatic Detection:**
   - When LocardX launches, the startup sweep scans for any operations left in `Running` or `Queued` states.
2. **Fail-Closed Invariant:**
   - The operation is marked as `Failed` in the operation manager.
   - The drive erasure record is transitioned to `Unknown` with verification outcome `UnableToVerify`.
   - An audit event `OPERATION_INTERRUPTED_RECOVERED` is emitted.
3. **Operator Remediation:**
   - The drive is in an indeterminate state.
   - Do NOT attempt to read filesystems from the drive.
   - Re-open LocardX, select the drive, re-generate a sanitization plan, and perform a complete sanitization run from start to finish.

---

## 7. Laboratory Validation & Test Automation

For automated continuous integration or lab verification without physical hardware destruction:

1. **Non-Destructive Simulation:**
   - Execute all test suites using `ExecutionMode::Simulation`.
2. **Fault Injection Testing (`FaultInjectableExecutor`):**
   - Inject simulated device locking failures (`FailExclusiveLock`).
   - Inject readback verification mismatches (`FailVerificationMismatch`).
   - Inject hardware disconnection mid-pass (`FailDisconnect`).
3. **Automated Test Command:**
   ```bash
   cargo test -p locardx-drive-eraser --test e2e_acceptance_tests
   ```
   This executes the 13-stage end-to-end acceptance suite validating all safety boundaries, mutation detection, simulated sanitization, verification handoff, report generation, and crash recovery.
