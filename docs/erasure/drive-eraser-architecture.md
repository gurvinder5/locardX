# LocardX Secure Drive Eraser Architecture (Step 10A & Step 10B)

## Executive Summary

LocardX Step 10A and Step 10B establish the production domain architecture, hardware capability detection engine, media-aware sanitization planning, two-stage authorization interlocks, time-of-check to time-of-use (TOCTOU) physical snapshot defenses, deterministic simulation execution harness, and native platform read-only physical hardware discovery for physical storage device sanitization.

In strict compliance with the **Step 10A and Step 10B.2 Critical Safety Boundary**, real hardware destructive commands (including raw sector writes, ATA Secure Erase, NVMe Format/Sanitize, destructive IOCTL commands, diskpart, and format utilities) are permanently prohibited and disabled. Real hardware sanitization remains a non-destructive stub returning `RealHardwareExecutionNotEnabled`. All destructive drive execution remains strictly locked.

---

## 1. Core Architectural Separation: Physical Devices vs. Logical Volumes

A fundamental invariant of LocardX is the rigid architectural distinction between **Physical Storage Devices** (Pillar 1) and **Logical Filesystem Targets** (Pillar 2 / Step 9):

| Characteristic | Physical Storage Device (Step 10A/10B) | Logical Volume / File (Step 9) |
|---|---|---|
| **Target Representation** | Raw physical device node (`\\.\PhysicalDriveX`, `/dev/sdX`, `/dev/nvmeXn1`) | Drive letter, volume path, mount point (`C:\`, `D:\Data`) |
| **Sanitization Boundary** | Entire physical media spanning all LBAs, partitions, and tables | Individual files, directories, or filesystem allocated clusters |
| **Protected Invariants** | Hard block on active OS System Disk (`PhysicalDrive0`) & Boot media | Hard block on System Root (`C:\Windows`, `C:\Program Files`) |
| **Forensic Implications** | Eradicates MBR, GPT, backup partition tables, and all filesystems | Unlinks directory entry and shreds file stream; leaves FS intact |
| **Input Interception** | Target paths containing `:\`, mount points, or logical volumes are rejected with `LogicalVolumeTargetRejected` before planning | Operates exclusively on canonical filesystem paths |

Any attempt to submit a logical volume (e.g. `C:\`, `D:\`) to the Drive Eraser fails immediately with an explicit error explaining that physical device identifiers must be selected.

---

## 2. Media-Aware Sanitization & Method Selection

Different physical storage media have fundamentally different physical properties, controller abstractions, and wear-leveling behaviors. LocardX rejects the dangerous fallacy that a "one size fits all" multi-pass overwrite is appropriate for all storage:

### 2.1 Rotational Magnetic Disks (HDD)
- **Primary Method:** `Nist80088ClearZero` (Single-pass pseudorandom or 0x00 overwrite).
- **Secondary Option:** `Dod522022M` (3-pass overwrite: 0x00, 0xFF, random, with verify).
- **Physical Limitations:** Reallocated defect sectors (P-List / G-List) and Host Protected Area (HPA) / Device Configuration Overlay (DCO) require explicit ATA unlock commands. Addressable LBA range is fully overwritten.
- **Verification:** `FullDeviceReadVerify` (100% sequential LBA readback verifying 0x00 null bytes).

### 2.2 SATA Solid State Drives (SATA SSD)
- **Primary Method:** `AtaSecureErase` / `AtaSanitize`.
- **Rationale:** Traditional overwriting causes severe Flash Translation Layer (FTL) wear amplification and cannot reliably address over-provisioned spare blocks or wear-leveled NAND blocks. `AtaSecureErase` commands the internal drive controller to issue a block-level flash purge.
- **Verification:** `FirmwareStatusVerify` (Query ATA IDENTIFY DEVICE security status word 128 and completion registers).

### 2.3 NVMe Solid State Drives (NVMe SSD)
- **Primary Method:** `NvmeCryptoErase` (Cryptographic Media Key Destruction) or `NvmeFormatSanitize`.
- **Rationale:** NVMe drives employ hardware encryption engines. Purging the internal media encryption key renders all underlying NAND flash physically irrecoverable in milliseconds without wear. Block erase commands sanitize all namespaces and over-provisioned flash areas.
- **Verification:** `CryptoKeyDestructionCheck` (Inspect NVMe Sanitize Progress log page `0x15` and verify key generation sequence).

### 2.4 Removable USB Flash & Memory Cards (SD/microSD)
- **Primary Method:** `Nist80088ClearZero` with explicit limitation warnings.
- **Limitations:** Low-cost USB and SD bridge controllers lack standardized sanitize firmware logs and perform low-cost FTL wear leveling. Addressable logical sectors are zeroed, but physical cells outside logical mapping may retain residual charge patterns.
- **Verification:** `SampledSectorVerification` (Pseudorandom sampling across beginning, middle, and end LBA blocks).

### 2.5 Unknown Media
- **Safety Policy:** Unidentified devices or devices with undetectable hardware capabilities fail closed with `UnsupportedMedia`. Destructive planning is rejected.

---

## 3. Hardware Capability Detection Engine & Fail-Closed States

The capability assessment engine evaluates hardware features into explicit capability states:

```rust
pub enum CapabilityState {
    Supported,
    Unsupported,
    Unknown,
    DetectionFailed,
}
```

- **Fail-Closed Invariant:** If a capability evaluates to `CapabilityState::Unknown` or `CapabilityState::DetectionFailed`, `should_fail_closed()` evaluates to `true`. Planning and gate authorization reject operations that require unverified capabilities.
- Probed capabilities include:
  - `SequentialWrite`, `FullDeviceRead`, `SectorAccess`
  - `AtaSecureErase`, `AtaEnhancedSecureErase`, `AtaSanitizeCrypto`, `AtaSanitizeBlock`, `AtaSanitizeOverwrite`
  - `NvmeCryptoErase`, `NvmeFormatSanitize`, `NvmeSanitizeBlock`
  - `TcgOpalCryptoErase`, `TrimDeallocate`

---

## 4. Two-Stage Safety & Authorization Pipeline

Before any physical drive sanitization can be planned or executed, it must pass multiple fail-closed safety interlocks:

1. **Active OS & Boot Protection:**
   - Physical drives flagged with `is_system_device: true`, `Classification::SystemDevice`, or `Classification::BootDevice` are unconditionally hard-blocked.
   - Any physical drive hosting an active system volume (`C:\`) or active boot volume (ESP / UEFI) is rejected with `SystemOrBootDeviceProtected`.
   - No UI override, administrative role, or confirmation challenge can bypass this block.

2. **Operator Authorization Challenge:**
   - An authenticated session token is required.
   - The operator must explicitly check the destructive consequences acknowledgment.
   - The operator must manually type the exact physical device identifier (e.g. `\\.\PhysicalDrive1` or `PhysicalDrive1`). Typing mismatch aborts the operation.

3. **Single-Use Hardware Execution Permit:**
   - Real hardware execution requests pass through `RealHardwareExecutionGate::request_permit`.
   - The gate verifies execution mode restrictions, target format, system/boot status, two-stage confirmations, capability sufficiency, and executes a live TOCTOU re-probe against the hardware provider.
   - A single-use `HardwareExecutionPermit` is issued upon success, cryptographically binding the operation ID, plan ID, physical device ID, snapshot, method, and mode.

---

## 5. TOCTOU (Time-of-Check to Time-of-Use) Hardware Snapshot Defense

A critical forensic risk in whole-disk erasure is drive substitution or hardware mutation between the time of planning and the moment of execution.

LocardX implements live hardware re-probing (`detect_mutation` on `PhysicalDeviceSnapshot`):
- When a plan is created, a `PhysicalDeviceSnapshot` records:
  - `device_id`, `vendor`, `model`, `serial_number`, `media_type`, `capacity_bytes`, `sector_size`, `physical_sector_size`, `bus_type`, `is_system`, `is_boot`, `is_read_only`, `exists`, `partition_count`, and `snapshot_timestamp`.
- Immediately before execution begins, the live hardware bus is re-queried.
- **Fail-Closed Mutation Triggers:**
  - Device disconnected or disappeared (`!live.exists`)
  - Target device ID or path substitution
  - Serial number mismatch
  - Media capacity mutation
  - Logical or physical sector size mutation
  - Bus interface type mutation
  - Media type mutation (e.g. SSD replaced by HDD)
  - Hardware vendor or model mutation
  - Acquisition of active system or boot status
  - Hardware write-protection state transition (`is_read_only`)
- If any mutation is detected, execution aborts with `DriveEraseFailureReason::DeviceMutated` and an audit record is logged.

---

## 6. Deterministic Simulation Engine Architecture

Step 10A implements the `DriveSanitizer` trait via mock and simulated backends:

```rust
pub trait DriveSanitizer: Send + Sync {
    fn execution_mode(&self) -> ExecutionMode;
    fn execute(
        &self,
        permit: &HardwareExecutionPermit,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<DriveEraseResult, DriveEraseFailureReason>;
}
```

Implementations:
- `SimulatedHddOverwrite`: Calculates transfer timings based on drive RPM, simulates multi-pass pattern overwrites, emits progress telemetry, and supports cooperative operator cancellation.
- `SimulatedAtaSecureErase`: Simulates ATA firmware controller execution times and register status queries.
- `SimulatedNvmeSanitize`: Simulates NVMe block-level sanitize and log page updates.
- `SimulatedNvmeCryptoErase`: Simulates instant cryptographic media key destruction.

---

## 7. Native Platform Hardware Discovery & Read-Only Probing (Step 10B.2)

Step 10B.2 implements real physical storage device enumeration and hardware capability probing behind the `DriveHardwareProvider` trait:

### 7.1 Windows Implementation (`WindowsHardwareProvider`)
- **Query-Only Access Handles:** Opens device handles with `dwDesiredAccess = 0` (equivalent to `FILE_READ_ATTRIBUTES`), `FILE_SHARE_READ | FILE_SHARE_WRITE`, and `OPEN_EXISTING`. No write or full read access is ever requested.
- **Geometry & Alignment:** Issues `IOCTL_DISK_GET_DRIVE_GEOMETRY_EX` to determine total capacity, media type, and sector size. Queries `StorageAccessAlignmentProperty` via `IOCTL_STORAGE_QUERY_PROPERTY` to obtain physical sector sizing.
- **Bus & Controller Identification:** Queries `StorageDeviceProperty` to read vendor ID, product ID, product revision, serial number, and bus interface type (`NVMe`, `SATA`, `USB`, `SCSI`, `RAID`).
- **Rotational vs Solid-State Detection:** Queries `StorageDeviceSeekPenaltyProperty`. Devices with zero seek penalty are classified as SSDs; non-zero seek penalty indicates rotational magnetic HDDs.
- **TRIM / Deallocate Detection:** Queries `StorageDeviceTrimProperty` to probe discard/TRIM support.
- **Write-Protection Status:** Queries `IOCTL_DISK_IS_WRITABLE` (read-only query) to detect hardware write-blockers or read-only switches.
- **System and Boot Drive Mapping:** Queries `IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS` across all mounted volumes (including system directory root `C:\` and EFI system partitions) to map volumes back to physical disk extent numbers. Matches are flagged as `is_system = true`, `is_boot = true`, and `classification = SystemDevice`, ensuring hard-blocked protection.

### 7.2 Linux Implementation (`LinuxHardwareProvider`)
- **Safe Sysfs Parsing:** Iterates `/sys/block/*` entries without issuing raw SCSI/ATA/NVMe commands.
- **Attribute Queries:** Reads `/sys/block/<dev>/size`, `queue/hw_sector_size`, `queue/physical_block_size`, `queue/rotational`, `removable`, `ro`, `device/model`, `device/vendor`, and `device/serial`.
- **System & Boot Mount Cross-Referencing:** Parses `/proc/mounts` to identify mount points `/`, `/boot`, and `/boot/efi`, resolving backing block devices to physical storage disks.

---

## 8. Forensic Verification & Tamper-Evident Audit Logging

All sanitization lifecycle events are committed to SQLite (`drive_erasure_records`) and the cryptographic SHA-256 hash-chained audit service:

| Audit Event | Trigger |
|---|---|
| `DRIVE_ERASURE_REJECTED` | System disk, boot disk, or logical volume target rejected |
| `DRIVE_ERASURE_TARGET_CHANGED` | TOCTOU live hardware re-probe detects drive substitution |
| `DRIVE_ERASURE_SIMULATION_STARTED` | Two-stage confirmation accepted; simulation begins |
| `DRIVE_ERASURE_SIMULATION_COMPLETED` | Sanitization routine finishes processing simulated LBAs |
| `DRIVE_ERASURE_CANCELLED` | Operator cancels running simulation via cancellation token |
| `DRIVE_ERASURE_VERIFICATION_STARTED` | Post-sanitization verification check initiated |
| `DRIVE_ERASURE_VERIFICATION_COMPLETED` | Forensic verification passed and recorded |

---

## 9. Real Hardware Execution Boundary & Gate Invariants

The strict boundary between execution modes ensures:
1. **Execution Gate Interlock:** `RealHardwareExecutionGate` controls hardware execution authorization. By default, it initializes closed (`hardware_execution_enabled: false`).
2. **Deterministic Non-Destructive Stub Option:** `RealHardwareSanitizer::stub()` preserves backwards-compatibility and testing boundaries by rejecting execution unconditionally with `RealHardwareExecutionNotEnabled`.
3. **Fail-Closed Stance:** Missing or unprobed serial numbers and capabilities are never fabricated; unknown or failed detections fail closed.
4. **Permit TTL and Binding:** Every execution attempt requires an unconsumed, unexpired (TTL <= 300s) `HardwareExecutionPermit` bound to the operation ID, plan ID, target device, method, and snapshot.

---

## 10. Real Hardware Sanitization Backend Architecture (Step 10B.3)

Step 10B.3 transitions `RealHardwareSanitizer` into a production-grade execution engine backed by platform-isolated hardware executors (`DriveHardwareExecutor`):

### 10.1 Platform Hardware Executor Abstraction
- **`DriveHardwareExecutor` Trait:**
  ```rust
  pub trait DriveHardwareExecutor: Send + Sync {
      fn acquire_exclusive_access(&self, device_id: &str) -> Result<Box<dyn ExclusiveDriveHandle>, DriveEraseFailureReason>;
      fn execute_sanitization(&self, handle: &mut Box<dyn ExclusiveDriveHandle>, permit: &HardwareExecutionPermit, is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>, on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>) -> Result<(u64, f64), DriveEraseFailureReason>;
      fn verify_sanitization(&self, handle: &mut Box<dyn ExclusiveDriveHandle>, permit: &HardwareExecutionPermit) -> Result<DriveVerificationResult, DriveEraseFailureReason>;
  }
  ```
- **`WindowsDriveHardwareExecutor`:**
  - Volume Extent Locking: Maps all volumes across the physical drive and dispatches `FSCTL_LOCK_VOLUME` and `FSCTL_DISMOUNT_VOLUME` to ensure zero concurrent filesystem file locks or unflushed cached dirty buffers.
  - Exclusive Win32 Handle: Opens `\\.\PhysicalDriveX` with `GENERIC_READ | GENERIC_WRITE`, `FILE_SHARE_READ | FILE_SHARE_WRITE`, `OPEN_EXISTING`, and `FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH`.
  - Overwrite Execution: Writes sector-aligned 64 KB pattern blocks directly to physical disk LBAs with Windows IO completion monitoring.
  - Firmware Sanitize Pass-Through: Issues `IOCTL_STORAGE_PROTOCOL_COMMAND` for NVMe Block Erase / Crypto Erase or `SMART_RCV_DRIVE_DATA` for ATA security erase.
  - Readback Verification: Performs sequential or sampled LBA verification passes reading 64 KB chunks, verifying zeroed (0x00) sectors.
- **`LinuxDriveHardwareExecutor`:**
  - Unmounts backing filesystem mount points from `/proc/mounts`.
  - Opens block devices (`/dev/sdX`, `/dev/nvmeXn1`) with `O_RDWR | O_EXCL | O_DIRECT | O_SYNC`.
  - Uses direct raw block I/O and `ioctl(BLKDISCARD)` / `ioctl(NVME_IOCTL_ADMIN_CMD)`.

### 10.2 The 15-Point Pre-Execution Checkpoint
Before a single destructive byte or firmware command is issued, `RealHardwareSanitizer` executes a mandatory 15-point check:
1. Target is a valid physical storage device format (`\\.\PhysicalDriveX`, `/dev/sdX`).
2. Target is NOT a logical volume, drive letter, or filesystem mount path.
3. Target is NOT the active OS system device (`PhysicalDrive0`, `/`).
4. Target is NOT the active boot / EFI system device.
5. Device classification is neither `SystemDevice` nor `BootDevice`.
6. Target exists and is physically online.
7. Operator identity and session token are valid.
8. Two-stage confirmation challenge is valid.
9. Operator typed target path matches exactly.
10. Required media sanitization capability is explicitly `Supported`.
11. Device is NOT write-protected or read-only.
12. Single-use `HardwareExecutionPermit` is valid and not expired (TTL check).
13. Permit has NOT been previously consumed.
14. Live physical topology re-probe passes with ZERO mutation (TOCTOU check).
15. Device concurrency lock (`DeviceLockRegistry`) and native exclusive handle acquired.

### 10.3 Device Concurrency Registry & Locking
- `DeviceLockRegistry`: An atomic, thread-safe memory registry of active device sanitization locks.
- `DeviceLockGuard`: RAII lock guard that automatically releases the lock upon completion, failure, or panic.
- Concurrent operations against the same physical device are rejected immediately with `ExclusiveAccessFailed`.

### 10.4 Post-Sanitization Forensic Verification Handoff
- Verification runs immediately upon sanitization completion using the still-held exclusive device handle.
- Outcomes map strictly to fail-closed statuses:
  - `Verified` -> `Completed`
  - `VerificationFailed` -> `VerificationFailed`
  - `UnableToVerify` -> `UnableToVerify` (Fail-Closed)
- Interrupted or inconclusive operations NEVER become `Completed`.

---

## 11. Dedicated Non-Destructive Test Harness (`FaultInjectableExecutor`)

To guarantee test coverage without endangering physical storage or requiring real hardware drives in continuous integration, LocardX provides the `FaultInjectableExecutor`:

- **Component:** `crates/drive-eraser/src/platform/test_executor.rs`
- **Purpose:** Implements `DriveHardwareExecutor` with programmatic fault-injection modes:
  - `Normal`: Simulates successful exclusive lock, block overwrite or firmware sanitize, and full readback verification.
  - `FailExclusiveLock`: Injects exclusive device locking failures (simulating concurrent file locks, open handles, or `ERROR_ACCESS_DENIED`).
  - `FailIoError`: Injects I/O failures during write or firmware execution (simulating physical bad sectors or bus reset).
  - `FailDisconnect`: Simulates sudden physical media disconnection mid-execution.
  - `FailVerificationMismatch`: Injects readback verification mismatch (non-zero byte discovered during forensic verification pass).
  - `FailUnableToVerify`: Injects indeterminate verification failures (controller query timeout or bus timeout).
- **Execution Tracking:** Records call metrics (`exclusive_lock_calls`, `sanitization_calls`, `verification_calls`, `bytes_written`, `last_progress`) for granular assertions.
- **Safety Invariant:** All simulated I/O is in-memory; no physical storage handles are opened.

---

## 12. Media & Method Sanitization Support Matrix

LocardX applies a media-tailored sanitization policy. The table below specifies the complete matrix across supported and unsupported storage types:

| Media Type | Bus Interface | Detection Heuristics | Primary Method | Fallback Method(s) | Verification Strategy | Known Physical Limitations | Fail-Closed Policy |
|---|---|---|---|---|---|---|---|
| **Rotational HDD** | SATA, PATA, SAS, USB Bridge | Non-zero seek penalty (`StorageDeviceSeekPenaltyProperty`), `rotational == 1` | `Nist80088ClearZero` (Single-pass 0x00) | `Dod522022M` (3-pass 0x00/0xFF/Rand) | `FullDeviceReadVerify` (100% LBA readback) | Reallocated defect sectors (G-List/P-List) inaccessible via standard LBA. Addressable sectors fully cleared. | Fails closed if sector size or capacity cannot be determined. |
| **SATA SSD** | SATA | Zero seek penalty, ATA command set supported, non-NVMe bus | `AtaSecureErase` or `AtaSanitize` | `Nist80088ClearZero` (Only if ATA sanitize is unsupported and overwrite is permitted) | `FirmwareStatusVerify` (IDENTIFY DEVICE register / log queries) | Controller wear leveling and over-provisioned spare blocks may retain data if overwrite fallback is used. | If ATA sanitize status cannot be verified, fails closed with `UnableToVerify`. |
| **NVMe SSD** | NVMe (PCIe, M.2, U.2) | Bus type NVMe, PCIe controller vendor/device ID | `NvmeCryptoErase` (Crypto Key Destruction) | `NvmeFormatSanitize` (Block Erase) | `CryptoKeyDestructionCheck` (Sanitize status log page 0x15) | Crypto erase requires SED / hardware-encrypted controller. Block sanitize clears all namespaces. | If NVMe sanitize log reports error or incomplete state, fails closed with `VerificationFailed`. |
| **Removable USB Flash** | USB (MSC / UASP) | `is_removable == true`, Bus type USB | `Nist80088ClearZero` (Block overwrite) | None (Multi-pass adds wear without security gain) | `SampledSectorVerification` (Pseudorandom sampling across LBA range) | Low-cost FTL controllers lack firmware sanitize logs and wear-level across unmapped flash blocks. | Fails closed if device is hardware write-protected or read-only. |
| **Memory Cards** | SD, microSD, CF, MMC | Removable media, card reader bus type | `Nist80088ClearZero` (Block overwrite) | None | `SampledSectorVerification` (Boundary & random chunk verification) | Physical flash cells outside active LBA mapping retain charge. Wear leveling cannot be audited. | Fails closed if card lock switch engaged (`is_read_only == true`). |
| **Unknown / Unrecognized** | Any / Unknown | Undetected bus, unreadable geometry, or conflicting properties | None | None | None | Controller and media characteristics cannot be verified. | Unconditionally fails closed with `UnsupportedMedia`. Planning and execution are strictly blocked. |

---

## 13. Reporting, Tamper-Evident Forensics & Certificates

Every completed or attempted sanitization operation generates an immutable, tamper-evident sanitization record:

### 13.1 Report Architecture (`DriveSanitizationReport`)
- **Location:** `crates/reporting/src/models.rs`, `sanitization_report.rs`, `service.rs`
- **Storage:** Persisted in SQLite `sanitization_reports` table and referenced by `drive_erasure_records`.
- **Integrity Digest:** Canonical JSON representation hashed with SHA-256 (`report_digest`).
- **Evidence Digest:** Cryptographic digest computed during readback verification (`evidence_digest`), verifying zero-fill or pattern uniformity.
- **Audit Chain Reference:** Cryptographically linked to the SHA-256 hash-chained audit service (`audit_chain_reference`).
- **Tamper Verification:** Any modification of report fields invalidates `verify_report_integrity()`.

### 13.2 Simulation Warning Framing
To prevent fraudulent certification of simulated operations:
- When `is_simulation == true`, all report outputs, JSON records, markdown exports, and ASCII certificates embed mandatory, prominent warnings:
  ```text
  ********************************************************************************
  *                     SIMULATION MODE SANITIZATION RECORD                      *
  *                 NO PHYSICAL STORAGE MEDIA WAS ERASED OR MODIFIED             *
  *       THIS CERTIFICATE REPRESENTS A TEST SIMULATION ONLY - NOT A REAL PURGE  *
  ********************************************************************************
  ```
- Real hardware certificates omit simulation banners and carry the `TAMPER-EVIDENT FORENSIC SANITIZATION CERTIFICATE` designation.

---

## 14. Crash Recovery & Resumption Policy (Fail-Closed On Interruption)

Power failure, system crash, OS reboot, or process termination during physical sanitization presents a severe forensic hazard:

### 14.1 The Fail-Closed Resumption Policy
- **Absolute Prohibition on Silent Resumption:** LocardX will NEVER silently resume or mark completed an interrupted physical sanitization operation.
- **Physical Topology Invalidation:** Following a power cycle or reboot, drive enumeration order (`PhysicalDriveX` / `/dev/sdX`), controller states, and partition tables may have mutated. Attempting automated resumption could destroy the wrong device.
- **Startup Recovery Sweep:**
  - Upon LocardX startup (`init()` in `AppState`), the system invokes:
    1. `OperationManager::recover_interrupted_operations()`: Finds any operations in `Running`, `Queued`, or `Cancelling` states in SQLite and transitions them to `Failed` with error message `"Operation was interrupted by system shutdown, crash, or reboot."` Emits `OPERATION_INTERRUPTED_RECOVERED` audit event.
    2. `DriveEraserService::recover_interrupted_erasures()`: Identifies incomplete drive erasure records and transitions their status to `Unknown` with verification outcome `UnableToVerify`.
- **Operator Remediation:** The operator is alerted of the interrupted operation, the drive must be physically inspected, and sanitization must be re-planned and re-authorized from Step 1.

---

## 15. Platform & Administrative Privilege Requirements

Direct physical storage device access requires native operating system kernel privileges:

### 15.1 Windows Platform Requirements
- **Privilege Verification:** `TokenElevation` via Win32 `GetTokenInformation`. Requires elevated administrator context (UAC elevated).
- **Access Rights:** Direct access to `\\.\PhysicalDriveX` requires `GENERIC_READ | GENERIC_WRITE` with unbuffered I/O (`FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH`).
- **Volume Extent Locking:** Uses `FSCTL_LOCK_VOLUME` and `FSCTL_DISMOUNT_VOLUME` across all volume extents mapped to the target disk to prevent cached filesystem writes.
- **Access Denied Handling:** If non-elevated, Win32 returns `ERROR_ACCESS_DENIED` (code 5). The backend catches this and returns an explicit `PrivilegeCheckFailed` error directing the user to restart LocardX as Administrator.

### 15.2 Linux Platform Requirements
- **Privilege Verification:** Checks `geteuid() == 0` (root context).
- **Access Rights:** Direct access to `/dev/sdX` or `/dev/nvmeXn1` requires `O_RDWR | O_EXCL | O_DIRECT | O_SYNC`.
- **Filesystem Unmounting:** All mounted partitions on the target drive must be unmounted before exclusive access can be acquired.
- **Access Denied Handling:** Returns `EACCES` or `EPERM` if non-root. The backend wraps this in `PrivilegeCheckFailed`.

### 15.3 API & UI Privilege Status
- Backend endpoint: `check_drive_eraser_privileges` returns `DrivePrivilegeStatus { is_elevated, platform, message }`.
- Frontend UI: Queries privileges on page load and displays an alert banner if Real Hardware mode is selected in a non-elevated session.



