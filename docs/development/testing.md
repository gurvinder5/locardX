# Testing Strategy & Guidelines

LocardX maintains strict testing requirements to guarantee algorithm correctness, evidence preservation, and host safety.

---

## Safety Requirements

> [!CAUTION]
> Under NO circumstances should any automated test, benchmark, or manual debugging session target a real host storage device (`/dev/sd*`, `\\\\.\\PhysicalDrive*`, `C:`, `D:`).

### Approved Test Targets
1. **Mock Device Implementations**: Rust structs implementing device traits backed by memory buffers.
2. **Virtual Disk Files**: Loopback mounts, VHD/VHDX files, or raw sparse files.
3. **Synthetic Filesystem Images**: Pre-generated FAT32, NTFS, or ext4 raw disk images stored under `test-data/filesystem-images/`.

---

## Running Tests

### All Backend Crates
```bash
cargo test --workspace
```

### Specific Crate (e.g., Audit Hash Chain)
```bash
cargo test -p audit
```

### With Tracing Output
```bash
RUST_LOG=debug cargo test -p drive-eraser -- --nocapture
```

### Frontend Test Suite
```bash
cd frontend
npm test
```

---

## Evidence Integrity Invariant Tests

Whenever testing carving or recovery features, the test harness must include an invariant check:

```rust
// Compute hash before recovery
let initial_hash = sha256_digest(&image_path)?;

// Execute recovery routine
let results = recovery_engine.scan_and_carve(&image_path)?;

// Verify hash after recovery
let post_hash = sha256_digest(&image_path)?;
assert_eq!(initial_hash, post_hash, "Source evidence was modified during recovery!");
```
