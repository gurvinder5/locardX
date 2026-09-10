# Contributing to LocardX

Thank you for your interest in contributing to **LocardX**! As a forensic and data sanitization platform, correctness, memory safety, reproducible auditability, and physical device safety are paramount.

Please read this document carefully before submitting any contributions.

---

## 1. Code of Conduct & General Principles

- **Respect Architectural Boundaries**: Do not refactor existing crate hierarchies or interfaces without prior alignment.
- **Safety First**: Destructive data operations must never execute against unverified or physical host drives during testing or automated runs.
- **Evidence Immutability**: All forensic extraction and carving mechanisms must be strictly non-destructive and read-only.
- **No Premature Implementation**: Implement only what is in the approved task scope.

---

## 2. Branching Strategy

The repository follows a structured branching model:

- `main`: Protected. Contains stable, verified, and release-ready code. Direct pushes are disabled.
- `develop`: Protected. Integration branch for completed features and fixes.
- `feature/<name>`: Feature branches created from `develop` (e.g., `feature/drive-detection`, `feature/hash-chain`).
- `fix/<name>`: Bug fixes created from `develop` (e.g., `fix/mft-traversal-bounds`).
- `chore/<name>`: Infrastructure, CI, or documentation updates (e.g., `chore/github-actions-ci`).

### Branch Rules:
1. Always branch from the latest `develop` branch unless patching a critical production release.
2. Never force-push (`--force`) to `main`, `develop`, or another developer's active branch.
3. Never use `git reset --hard` to rewrite shared history.

---

## 3. Commit Guidelines

We enforce the **Conventional Commits** specification:

```text
<type>(<scope>): <short summary>

[optional body explaining motivation, technical approach, and trade-offs]

[optional footer(s), e.g., Closes #123]
```

### Allowed Types:
- `feat`: A new user-facing feature or backend capability
- `fix`: A bug fix or correction
- `docs`: Documentation changes only
- `style`: Formatting, whitespace, or style corrections (no logic changes)
- `refactor`: Code changes that neither fix a bug nor add a feature
- `perf`: Performance improvements
- `test`: Adding or correcting tests
- `chore`: Maintenance tasks, dependency bumps, or build configuration

### Examples:
- `feat(device-manager): add Windows volume mount point enumeration`
- `fix(audit): correct SHA-256 chain recalculation on empty initial state`
- `docs(architecture): update data-flow diagram for file carving`

---

## 4. Pull Request (PR) Process

1. **Keep PRs Focused**: A PR should address a single concern or feature. Avoid bundling unrelated modifications.
2. **Fill Out the PR Template**: Ensure every section of [pull_request_template.md](.github/pull_request_template.md) is filled in.
3. **Verify Locally**: Before opening a PR, ensure all checks pass:
   ```bash
   # Rust formatting and tests
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace

   # Frontend verification
   cd frontend
   npm run lint
   npm run typecheck
   npm run build
   ```
4. **Code Review**: Every PR requires at least one peer review. Address feedback constructively and avoid force-pushing when updating PRs.

---

## 5. Testing Requirements

Every module, crate, and feature must include adequate automated tests:

- **Unit Tests**: Place unit tests alongside the code or in crate `tests/` directories.
- **Mock Media Only**: Tests involving drive wiping, partition manipulation, or filesystem parsing must use:
  - In-memory data buffers
  - Loopback images
  - Synthetic disk image files (e.g., in `test-data/`)
  - Mock device manager implementations
- **Zero Real Disk Operations**: Any test targeting a real disk letter (e.g., `C:`, `/dev/sda`, `\\\\.\\PhysicalDrive0`) will be rejected immediately.
- **Negative & Boundary Tests**: Test malformed headers, truncated files, permission errors, and cancelled operations.

---

## 6. Security-Sensitive Changes

Contributions touching security, authentication, raw I/O, or audit logging require heightened scrutiny:

- **No Hardcoded Secrets**: Never commit passwords, private keys, authentication tokens, or test credentials.
- **Audit Logging**: Any new administrative, sanitization, or recovery operation must emit an immutable audit event via `crates/audit`.
- **Destructive Confirmation**: Any command capable of modifying or destroying data must mandate explicit double-confirmation before invocation.

---

## 7. Documentation Requirements

- If your change adds or alters a public API, IPC command, or shared type, update the corresponding documentation under `docs/api/`.
- If you introduce or modify architectural components, update `docs/architecture/`.
- Ensure all public Rust functions, structs, and traits have doc-comments (`///`).