# Git Workflow & Collaboration Guide

LocardX maintains a structured Git workflow to ensure code quality, change traceability, and stability across releases.

---

## 1. Branch Strategy

- `main`: Production-ready releases. Protected.
- `develop`: Ongoing integration of features and fixes. Protected.
- `feature/<feature-name>`: Scoped feature development branches.
- `fix/<bug-name>`: Scoped bug fix branches.
- `chore/<chore-name>`: Build, CI, dependencies, or documentation maintenance.

---

## 2. Commit Message Guidelines

Use the [Conventional Commits](https://www.conventionalcommits.org/) convention:

```text
<type>(<scope>): <description>

[optional body]

[optional footer]
```

---

## 3. Pull Request (PR) Checklist

Before submitting a pull request:
1. Ensure all new code has unit tests.
2. Verify all tests pass locally: `cargo test --workspace`.
3. Check code formatting: `cargo fmt --all -- --check`.
4. Ensure no real disk operations or unverified device targets are included in tests.
5. Fill out the [Pull Request Template](../../.github/pull_request_template.md).
