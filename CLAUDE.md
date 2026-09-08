# LocardX Development Rules

## Project

LocardX is a desktop forensic and secure data sanitization platform.

Technology:

- Tauri
- React
- TypeScript
- Tailwind CSS
- Rust
- Tokio
- Serde
- SQLite
- TSK
- SHA-256
- Hash-chain audit logging

## Architecture

The application contains:

1. Secure Drive Eraser
2. Secure File & Folder Eraser
3. Advanced File Carving & Recovery

Shared services include:

- Authentication
- Device detection
- File-system detection
- Operation/job management
- Verification
- Audit logging
- Reporting
- SQLite persistence

## Development Principles

- Follow the existing architecture.
- Do not redesign the architecture without explicit approval.
- Do not introduce unnecessary abstractions.
- Do not implement functionality outside the assigned task.
- Do not modify unrelated files.
- Prefer small, testable modules.
- Prefer explicit interfaces between modules.
- Keep business logic in Rust.
- Keep presentation logic in React/TypeScript.
- Do not place destructive operations directly in UI code.

## Git Rules

- Never commit directly to main.
- Work only on the assigned feature branch.
- Never force push.
- Never reset or delete another developer's work.
- Never modify files outside your assigned area unless required by an agreed interface.
- Keep commits focused.
- Before committing, inspect git diff.
- Before finishing, run relevant tests and formatting.

## Safety

LocardX performs potentially destructive storage operations.

NEVER execute destructive disk/file operations against an actual
user disk during development or testing.

Use mocks, fixtures, loopback devices, virtual disks, or dedicated test media.

Never:

- wipe a real disk
- overwrite arbitrary block devices
- delete user evidence
- modify forensic evidence
- disable OS security controls
- bypass encryption/security mechanisms

## Evidence Integrity

Recovery operations must preserve source evidence.

Do not modify the source evidence unless explicitly required by the
approved acquisition workflow.

Calculate and preserve hashes where required.

## Testing

Every core module must have:

- unit tests
- integration tests where applicable
- failure-path tests
- boundary-condition tests

Tests must not be weakened or removed merely to make implementation pass.

## Before Coding

1. Inspect repository structure.
2. Read relevant architecture documentation.
3. Identify interfaces and dependencies.
4. Explain the implementation plan.
5. Implement only the requested scope.
6. Run tests.
7. Review git diff.
8. Report changed files and test results.

## When Uncertain

Do not guess about:

- disk erasure semantics
- forensic evidence handling
- filesystem structures
- OS-level disk APIs
- security guarantees
- compliance claims

Stop and identify the uncertainty.