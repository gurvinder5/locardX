---
name: Bug Report
about: Create a report to help improve LocardX
title: '[BUG]: '
labels: ['bug', 'triage']
assignees: ''
---

## Description
A clear and concise description of the bug.

## Affected Module
Select the primary module affected:
- [ ] Secure Drive Eraser (`crates/drive-eraser`)
- [ ] Secure File & Folder Eraser (`crates/file-eraser`)
- [ ] Advanced File Carving & Recovery (`crates/recovery-engine`)
- [ ] Device Manager (`crates/device-manager`)
- [ ] Operation Manager (`crates/operation-manager`)
- [ ] Verification Engine (`crates/verification`)
- [ ] Audit Logging (`crates/audit`)
- [ ] Authentication / RBAC (`crates/auth`)
- [ ] Database / Persistence (`crates/database`)
- [ ] Reporting Subsystem (`crates/reporting`)
- [ ] Frontend UI / Desktop Shell (`frontend` / `src-tauri`)

## Test Media Confirmation
> [!IMPORTANT]
> To prevent data loss, bugs involving destructive erasure routines must only be reproduced on test media.

- [ ] **I confirm that this issue was observed on synthetic test media, virtual disks, or dedicated disposable lab hardware (NOT on a personal or production storage drive).**
- Test media type used: *(e.g., VHDX image, USB test drive, in-memory mock, loopback device)*

## Steps to Reproduce
1. Go to '...'
2. Click on '....'
3. Execute operation '....'
4. See error

## Expected Behavior
A clear and concise description of what you expected to happen.

## Actual Behavior
A clear and concise description of what actually happened.

## Environment Details
- **OS**: [e.g., Windows 11 23H2, Ubuntu 22.04 LTS]
- **LocardX Version / Commit**: [e.g., v0.1.0 or commit hash]
- **Target Storage Interface**: [e.g., NVMe, SATA, USB 3.0, Virtual Disk]
- **Filesystem**: [e.g., NTFS, FAT32, exFAT, ext4]

## Diagnostic Logs & Output
```text
Paste relevant tracing/console logs or error stack traces here if available.
(Ensure no passwords, tokens, or confidential forensic data are included)
```

## Additional Context
Add any other context about the problem here.
