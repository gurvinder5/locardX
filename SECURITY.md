# Security Policy

LocardX is designed for forensic recovery and secure data sanitization environments where data integrity, confidentiality, and physical safety are critical. This policy outlines security standards, vulnerability reporting procedures, and development safety rules.

---

## 1. Reporting Security Vulnerabilities

We take the security of LocardX seriously. If you discover a vulnerability or potential security weakness in LocardX, please follow responsible disclosure practices.

### How to Report

- **Preferred Method**: Submit a private report via **[GitHub Private Vulnerability Reporting](https://github.com/gurvinder5/locardX/security/advisories/new)** directly on the repository.
- **Alternative Method**: Contact the repository owner directly through their verified GitHub profile page or repository maintainer contact options.
- **Do Not Disclose Publicly**: Please do **not** open public GitHub issues or publicly post details about suspected vulnerabilities until a fix has been coordinated and released.

### What to Include in Your Report

1. Description of the vulnerability and its potential impact.
2. Step-by-step reproduction instructions or a minimal proof of concept (PoC).
3. Affected components (e.g., specific Rust crate, Tauri command, or frontend component).
4. Any relevant environment details (operating system, privilege level, hardware type).

---

## 2. Lab Safety: Strict Prohibition Against Real Disk Testing

> [!CAUTION]
> **LocardX contains code designed to permanently destroy data.**
> Testing destructive sanitization functionality against physical production disks, personal workstations, or real forensic evidence is strictly prohibited.

- **Mandatory Use of Test Media**: Developers and testers must conduct all disk-level sanitization experiments exclusively using:
  - Synthetic disk images (e.g., created via `qemu-img`, `dd`, or Windows `diskpart` VHD/VHDX)
  - Loopback virtual block devices
  - In-memory mock device drivers (`crates/device-manager/src/mock.rs`)
  - Dedicated, physically isolated lab media clearly marked for disposal
- **System Drive Protection**: LocardX maintains internal safeguards preventing operations against the host operating system drive or currently mounted boot volumes. Bypassing these safeguards during testing is prohibited unless running inside an isolated, non-production virtual machine.

---

## 3. Handling of Forensic Evidence

When developing or executing forensic recovery routines:

- **Strictly Read-Only Access**: Recovery operations must never open source block devices or forensic disk images with write permissions (`O_RDWR` / `GENERIC_WRITE`).
- **Hardware Write Blockers**: Physical media under forensic examination should always be connected via validated hardware write blockers.
- **Cryptographic Hashing**: Source evidence files and target disk images must have their SHA-256 hashes computed and verified prior to analysis and upon completion.

---

## 4. Protection of Secrets and Credentials

- Never commit private keys, passwords, authentication tokens, API credentials, or certificates to the repository.
- Use `.env.example` as a template; never commit `.env` or local configuration files containing sensitive values.
- All secrets detected in pull requests will result in an immediate build failure and required credential revocation.

---

## 5. Audit Trail & Hash-Chain Integrity

- All operations (erasure passes, carver executions, access control changes) must be recorded in the append-only SQLite audit log.
- Each audit log entry includes an incremental SHA-256 hash linking back to the previous entry, establishing a tamper-evident audit chain.
- Tampered or broken audit chains must immediately trigger high-priority alerts in the application and be flagged in forensic reports.
