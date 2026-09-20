# Module Architecture

LocardX organizes backend capabilities into isolated, purpose-built Rust crates located under `crates/`.

---

## Crate Directory & Responsibilities

| Crate | Responsibility | Key Dependencies |
| :--- | :--- | :--- |
| `crates/common` | Shared types, error definitions (`LocardError`), ID types, timestamps | `serde`, `thiserror` |
| `crates/auth` | User authentication, RBAC, password hashing, session store | `locardx-common` |
| `crates/device-manager` | OS block device enumeration, physical geometry, write-blocker detection | `locardx-common` |
| `crates/operation-manager` | Job scheduling, task queuing, progress broadcasting, cooperative cancellation | `locardx-common`, `tokio` |
| `crates/drive-eraser` | Whole-disk sanitization, multi-pass overwrites, ATA/NVMe sanitize command dispatch | `locardx-common`, `locardx-device-manager` |
| `crates/file-eraser` | Targeted file and directory wiping, metadata destruction, ADS clearing | `locardx-common` |
| `crates/recovery-engine` | Signature-based carving, TSK filesystem traversal, fragment reconstruction | `locardx-common` |
| `crates/verification` | Multi-pass pattern verification and cryptographic hash validation | `locardx-common` |
| `crates/audit` | Append-only SHA-256 hash-chained audit logging service | `locardx-common`, `locardx-database` |
| `crates/database` | SQLite connection pool, schema management, and migrations | `rusqlite`, `locardx-common` |
| `crates/reporting` | Generation of forensic reports, sanitization certificates, and audit exports | `locardx-common` |
| `crates/security` | Safety checks, system drive protection, privilege management | `locardx-common` |
