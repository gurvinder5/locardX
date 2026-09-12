# Forensic Evidence Integrity & Cryptographic Hashing

## 1. Overview & Forensic Principles

In forensic analysis and data sanitization verification, the fundamental principle is that **evidence must never be altered during inspection** (Locard's Exchange Principle inverted: the investigator's tools must leave no contact trace on source evidence).

LocardX provides cryptographic hash calculation and integrity verification designed to meet digital forensics standards:
- **Strict Read-Only Access**: Hashing and verification open files strictly with read-only flags (`std::fs::File::open`), never issuing write, append, or truncate operations.
- **Bounded Memory Consumption**: Computations stream through a bounded 64 KiB buffer (`STREAM_CHUNK_SIZE`), ensuring constant \(O(1)\) memory usage regardless of whether the file is 10 KB or 100 GB.
- **Side-Channel Mitigation**: Hash comparison utilizes constant-time byte comparisons to prevent timing side channels.
- **Chain of Custody Anchoring**: Every calculation and verification operation is registered as an immutable event in the tamper-evident SHA-256 audit hash-chain.

---

## 2. Cryptographic Specification

### Algorithm
- **Algorithm**: SHA-256 (Secure Hash Algorithm 256-bit, FIPS 180-4).
- **Implementation**: RustCrypto `sha2` crate (pure Rust, audited, constant memory).
- **Digest Output**: 64 lowercase hexadecimal characters.

### Test Vectors
LocardX validates hash correctness against standard NIST test vectors:
- **Empty String**:
  `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- **"abc"**:
  `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`
- **Standard 56-byte sequence ("abcdbcdecdef...nopq")**:
  `248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1`

---

## 3. Architecture & Data Structures

### Target Identity vs. Cryptographic Digest
LocardX explicitly separates *what* is being hashed (`TargetIdentity`) from the *digest* produced:

```rust
pub struct TargetIdentity {
    pub target_type: TargetType,
    pub identifier: String,    // Normalized canonical path
    pub display_name: String,  // Filename for user presentation
    pub size_bytes: Option<u64>,
}
```

This prevents ambiguous provenance records where a digest exists without clear metadata regarding the exact evidence file, size, and source path.

### Verification States
Verification results distinguish three explicit outcomes:
1. **`Verified`**: The calculated digest exactly matches the reference digest.
2. **`Mismatch`**: Both expected and calculated digests are syntactically valid, but their cryptographic contents differ. Both values are retained in the result and audit log.
3. **`UnableToVerify`**: The verification could not complete (e.g., target file does not exist, permission denied, or reference hash is syntactically invalid).

---

## 4. Tamper-Evident Audit Integration

Every calculation and verification is committed to SQLite `audit_events` and `integrity_records` tables:
- **Hash calculation event**: `EVIDENCE_HASH_CALCULATED`
- **Successful verification event**: `INTEGRITY_VERIFICATION_PASS`
- **Failed verification event**: `INTEGRITY_VERIFICATION_FAIL`
- **Verification error event**: `INTEGRITY_VERIFICATION_ERROR`

Each event incorporates the previous event's SHA-256 digest, creating an unbroken cryptographic hash-chain that detects any retroactive tampering or record deletion.
