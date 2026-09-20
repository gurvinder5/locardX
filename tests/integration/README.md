# LocardX Integration Tests

This directory houses cross-crate end-to-end and subsystem integration test suites.

## Test Suites
- `authentication/`: Local user login, session timeout, role validation.
- `device_detection/`: Mock block device enumeration and geometry parsing.
- `drive_eraser/`: End-to-end sanitization on mock block devices and virtual disk files.
- `file_eraser/`: File overwriting and metadata stripping validation.
- `recovery/`: Carving extraction and Sleuth Kit bridge verification.
- `verification/`: Pattern matching and SHA-256 integrity verification.
- `audit/`: Operation logging and hash-chain verification.
- `reporting/`: PDF and JSON certificate emission.

## Execution
```bash
cargo test --test '*' -- --nocapture
```
