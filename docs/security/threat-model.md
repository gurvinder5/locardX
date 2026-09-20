# Threat Model

This document identifies potential security threats and the defensive mitigations implemented in LocardX.

---

## 1. Accidental Destruction of Host Operating System
- **Threat**: User or automated job accidentally selects the primary system boot drive (`C:`, `/dev/nvme0n1`) for sanitization.
- **Mitigation**: `crates/security` queries mount points, boot flags, and system directories. If the target is the active system drive, the operation is blocked unconditionally.

## 2. Evidence Spoliation During Recovery
- **Threat**: Forensic recovery modifies source timestamps, sectors, or metadata during carving.
- **Mitigation**: Source file descriptors are opened strictly read-only. Cryptographic SHA-256 hashes are verified pre- and post-analysis.

## 3. Retroactive Audit Log Tampering
- **Threat**: An operator or unauthorized party modifies SQLite log entries to conceal unauthorized erasure or falsify certificates.
- **Mitigation**: Cryptographic hash chaining ($H_n = \text{SHA-256}(H_{n-1} \parallel \text{Payload}_n)$). Altering any historical row invalidates all subsequent rows.

## 4. UI-Level Injection / XSS
- **Threat**: Malicious filenames in target drives attempt to execute script code in the React webview.
- **Mitigation**: React automatic HTML escaping, strict Content Security Policy (CSP), and sanitized string representations.
