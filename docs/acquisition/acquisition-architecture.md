# LocardX Forensic Acquisition Layer — Architectural Specification
**Document ID:** LX-ARCH-STEP11  
**Module:** `locardx-acquisition` (Step 11 — Forensic Disk Imaging & Bitstream Acquisition)  
**Status:** Approved & Implemented  
**Date:** September 2026  

---

## 1. Executive Summary & Purpose

The **Forensic Acquisition Layer** (`locardx-acquisition`) is a core foundational pillar of LocardX, providing **strictly read-only bitstream physical acquisition** of storage devices into raw DD image files (`.raw` / `.dd`). 

As the evidential input layer for the upcoming **Forensic Recovery Module (Pillar 3)**, this subsystem establishes cryptographic chain of custody, guarantees zero target mutation, calculates streaming SHA-256 digests in a single pass, and logs tamper-evident audit records linked to the immutable hash chain.

---

## 2. Core Architectural Separation

```
+-----------------------------------------------------------------------------------+
|                              OPERATOR / UI LAYER                                  |
|         ForensicAcquisitionPage.tsx   /   Tauri IPC Desktop Commands              |
+-----------------------------------------------------------------------------------+
                                         |
                   +---------------------+---------------------+
                   |                                           |
                   v                                           v
+------------------------------------+     +----------------------------------------+
|   FORENSIC ACQUISITION LAYER       |     |          DRIVE ERASER LAYER            |
|   (`locardx-acquisition`)          |     |          (`locardx-drive-eraser`)      |
|                                    |     |                                        |
| - Strictly Read-Only               |     | - Destructive sanitization             |
| - `GENERIC_READ` / `O_RDONLY` only |     | - Multi-pass sector overwriting        |
| - ZERO write permissions requested |     | - Firmware ATA/NVMe sanitize           |
| - Raw bitstream DD imaging         |     | - HardwareExecutionPermit required     |
| - Streaming SHA-256 verification   |     | - Strict authorization gates           |
+------------------------------------+     +----------------------------------------+
                   |
                   v
+-----------------------------------------------------------------------------------+
|                        EVIDENTIAL HANDOFF (`AcquisitionArtifact`)                 |
|                             -> Recovery Module (Pillar 3)                         |
+-----------------------------------------------------------------------------------+
```

### Absolute Isolation from Sanitization
- **Total Handle Independence**: The acquisition layer requests **only** read access (`GENERIC_READ` with `FILE_SHARE_READ | FILE_SHARE_WRITE` on Windows; `O_RDONLY` on POSIX). It never requests `GENERIC_WRITE`, never issues `IOCTL_DISK_FORMAT`, and never executes wipe commands.
- **Dedicated Service & State**: The acquisition engine runs independently from `DriveEraserService`, ensuring zero architectural cross-talk or accidental dispatch of sanitization methods on evidential source media.

---

## 3. Safety Invariants & Pre-Flight Validation

1. **Physical Device Exclusivity**:
   - Only raw physical device nodes (e.g., `\\.\PhysicalDrive0`, `/dev/sdb`) are accepted as acquisition sources.
   - Logical partition letters (`C:\`, `D:\`) and filesystem mount paths (`/mnt/data`) are strictly rejected via `validate_source_device_id`.
2. **TOCTOU Defense (Time-Of-Check to Time-Of-Use)**:
   - Before opening the device stream, the baseline hardware identity snapshot (`AcquisitionDeviceSnapshot`) is re-probed in real-time.
   - Any mutation in device identity, capacity, sector size, or bus type immediately triggers `AcquisitionFailureReason::SourceMutated`, terminating the operation before a single byte is read.
3. **Collision & Destination Isolation**:
   - **Source == Destination Rejection**: Destination path cannot match the source identifier.
   - **Device Node Rejection**: Image destination cannot be a raw disk device node (e.g. `\\.\PhysicalDrive2`).
   - **`DestinationOnSourceDisk` Prevention**: Using Win32 `IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS` (and Linux sysfs/mount mapping), the destination file's hosting physical drive is resolved. If the destination resides on any partition of the source physical disk, the operation fails with `DestinationOnSourceDisk` to prevent corruption or feedback loops.
4. **Free Space Verification & Headroom Margin**:
   - Pre-flight checks verify that destination volume free space exceeds the source device capacity plus a mandatory **100 MiB safety margin** (`DESTINATION_SAFETY_HEADROOM_BYTES`).
5. **Fail-Closed Cleanup**:
   - On error, user cancellation, or unexpected interruption, partial destination image files are immediately wiped and unlinked from the filesystem via `RawImageWriter::abort_and_cleanup()`.
   - Incomplete acquisitions are marked `Failed` in SQLite and can **never** produce an `AcquisitionArtifact`.

---

## 4. End-to-End Pipeline Workflow

```
[ Physical Storage Device ]
            |
            v
[ Device Detection & Probing ]
            |
            v
[ Device Identity Snapshot ] ----> (Immutable AcquisitionDeviceSnapshot)
            |
            v
[ Pre-Flight Validation ] -------> (Capacity + 100 MiB Headroom, Destination Separation)
            |
            v
[ TOCTOU Re-Validation ] --------> (Hardware re-probed immediately prior to stream open)
            |
            v
[ Read-Only Acquisition Loop ]
            |
            +---> 1 MiB Unbuffered Direct Chunks
            |
            +---> Streaming SHA-256 Hasher (Single Pass)
            |
            +---> RawImageWriter (.raw / .dd)
            |
            +---> Telemetry & Throttled Progress Updates
            |
            v
[ Final Flush & Sync ] ----------> (fsync/FlushFileBuffers)
            |
            v
[ Finalize SHA-256 Digest ] -----> (Cryptographic verification)
            |
            v
[ SQLite Persistence ] ----------> (Record in `acquisition_records`)
            |
            v
[ Audit Chain Logging ] ---------> (Tamper-evident `ACQUISITION_*` event chain)
            |
            v
[ AcquisitionArtifact Handoff ] -> (Pillar 3: Forensic Recovery Ingestion)
```

---

## 5. Evidential Handoff Contract

Upon successful completion and cryptographic verification, the acquisition layer generates an `AcquisitionArtifact`:

```rust
pub struct AcquisitionArtifact {
    pub acquisition_id: String,
    pub image_path: String,
    pub image_format: String,
    pub image_size_bytes: u64,
    pub image_sha256: String,
    pub source_device_snapshot: AcquisitionDeviceSnapshot,
    pub acquisition_timestamp: String,
    pub is_verified: bool,
    pub audit_reference: String,
}
```

The artifact provides `verify_integrity()`, which re-hashes the on-disk image file and checks file size against the baseline metadata, guaranteeing verifiable authenticity before the Forensic Recovery Module ingests the image.
