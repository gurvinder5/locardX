# Agent & Contributor Workflow

This document defines standard development workflows for human contributors and AI coding assistants working on LocardX.

## Core Directives

1. **Architecture Integrity**: Never modify or reorganize the crate or directory layout without prior planning and explicit agreement.
2. **Safety First**: LocardX implements data destruction logic. Never run live sanitization or raw disk access on non-test hardware.
3. **Evidence Protection**: All forensic acquisition and recovery tasks must operate strictly read-only on source media.
4. **No Premature Core Implementation**: Implement only the assigned module scope. Do not stub or implement raw disk I/O, ATA commands, or NVMe sanitization routines unless specifically tasked.

## Development Lifecycle

### 1. Inspection & Research
- Review the crate structure and workspace dependencies.
- Inspect relevant documentation under `docs/architecture/` and `docs/security/`.
- Identify affected shared interfaces (`crates/common`, `crates/database`, `crates/audit`).

### 2. Implementation Boundaries
- **Rust Backend (`crates/*`, `src-tauri/`)**: Contains all core business logic, filesystem parsers, verification algorithms, and OS abstractions.
- **Frontend (`frontend/`)**: React/TypeScript UI layer handling user input, visualization, progress tracking, and report presentation.
- Keep IPC commands typed and minimal via Tauri command handlers.

### 3. Verification & Testing
- Run test suites using mock devices or loopback images:
  ```bash
  cargo test
  ```
- Run frontend linting and type checks:
  ```bash
  cd frontend
  npm run lint
  npm run build
  ```

### 4. Git Cleanliness
- Check git status and diff before staging changes:
  ```bash
  git status
  git diff
  ```
- Ensure no secrets, tokens, `.env` files, or binary disk images are staged.
- Commit using Conventional Commits syntax (e.g., `feat:`, `fix:`, `docs:`, `test:`, `chore:`).
