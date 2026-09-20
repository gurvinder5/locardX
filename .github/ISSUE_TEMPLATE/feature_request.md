---
name: Feature Request
about: Suggest an idea or capability for LocardX
title: '[FEAT]: '
labels: ['enhancement']
assignees: ''
---

## Problem Statement
A clear and concise description of the problem or limitation you are addressing.
*Example: Currently, analysts cannot easily carve raw SQLite databases from unallocated clusters.*

## Proposed Solution
A clear and concise description of the feature, algorithm, or capability you propose to add.

## Affected Module(s)
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
- [ ] Frontend UI (`frontend`)

## Security & Forensic Implications
- Does this feature interact with physical storage devices or raw sectors?
- Does it guarantee read-only handling of source forensic evidence?
- Does it emit appropriate immutable audit events?
- Are there safety interlocks required to prevent accidental data destruction?

## Testing Requirements
Describe how this feature should be tested, including the synthetic test media or fixtures required.
*(e.g., requires FAT32 test image with fragmented JPEG files)*

## Alternatives Considered
A clear description of any alternative solutions, libraries, or workflows you considered.

## Additional Context
Add any other context, mockups, or specifications here.
