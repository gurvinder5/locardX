# LocardX Documentation Index

Welcome to the **LocardX** technical and architectural documentation.

## Documentation Structure

### 1. Architecture (`docs/architecture/`)
- [Architecture Overview](architecture/README.md): High-level system architecture, separation of core engines, and shared services.
- [System Architecture](architecture/system-architecture.md): Layered architecture between React/Tauri frontend and Rust backend.
- [Module Architecture](architecture/module-architecture.md): Detailed breakdown of individual crates and their responsibilities.
- [Data Flow](architecture/data-flow.md): Event flow from user interaction to disk I/O and audit logging.
- [Security Architecture](architecture/security-architecture.md): Security boundaries, privilege levels, and defense in depth.

### 2. Development (`docs/development/`)
- [Agent & Contributor Rules](../AGENT_RULES.md): Fundamental development rules, safety guidelines, and evidence integrity rules.
- [Development Setup](development/development-setup.md): Environment preparation, prerequisites, and build commands.
- [Agent & Contributor Workflow](development/agent-workflow.md): AI-agent and developer workflow expectations.
- [Git Workflow](development/git-workflow.md): Branching, commits, and pull requests.
- [Testing Guide](development/testing.md): Unit tests, mock media, and test execution.

### 3. Security (`docs/security/`)
- [Security Policy](../SECURITY.md): Vulnerability reporting, test media requirements, and disk safety.
- [Security Controls](security/security-controls.md): Safeguards, write blockers, confirmation dialogs, and hash chains.
- [Threat Model](security/threat-model.md): Assets, threats, trust boundaries, and mitigations.
- [Authentication](security/authentication.md): RBAC, local session storage, and authorization.

### 4. Testing & Test Data (`docs/testing/` & `tests/`)
- [Testing Overview](testing/README.md): Test philosophy, synthetic media generators, and integration test layout.

### 5. Domain Engines (`docs/erasure/` & `docs/forensic/`)
- [Drive Erasure](erasure/drive-erasure.md): Whole-disk sanitization standards (NIST SP 800-88, DoD 5220.22-M).
- [File & Folder Erasure](erasure/file-folder-erasure.md): Targeted file destruction, metadata zeroing, and ADS removal.
- [Verification](erasure/verification.md): Multi-pass pattern verification and cryptographic proof.
- [Carving & Recovery](forensic/carving.md): Signature carving, TSK filesystem traversal, and fragmentation reassembly.
- [Evidence Integrity](forensic/evidence-integrity.md): Read-only guarantees and hash preservation.

### 6. User Guides (`docs/user/`)
- [Installation Guide](user/installation.md): End-user installation requirements.
- [User Manual](user/user-manual.md): Operating instructions for sanitization and recovery tasks.
