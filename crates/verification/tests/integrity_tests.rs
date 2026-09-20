use locardx_audit::AuditService;
use locardx_database::Database;
use locardx_verification::{
    HashAlgorithm, HashVerifier, IntegrityService, StreamHasher, TargetType, VerificationStatus,
    STREAM_CHUNK_SIZE,
};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

struct TempTestFile {
    path: PathBuf,
}

impl TempTestFile {
    fn new(content: &[u8]) -> Self {
        let filename = format!("locardx_test_{}.tmp", Uuid::new_v4());
        let path = std::env::temp_dir().join(filename);
        let mut file = File::create(&path).expect("Failed to create temporary test file");
        file.write_all(content)
            .expect("Failed to write to temporary test file");
        file.flush().expect("Failed to flush test file");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempTestFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn setup_test_service() -> (IntegrityService, Arc<Database>, Arc<AuditService>) {
    let db = Arc::new(Database::open(":memory:").expect("Failed to create test database"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let integrity = IntegrityService::new(Arc::clone(&db), Arc::clone(&audit));
    (integrity, db, audit)
}

#[test]
fn test_01_nist_test_vector_empty_file() {
    let file = TempTestFile::new(b"");
    let result = StreamHasher::hash_file(file.path(), HashAlgorithm::Sha256)
        .expect("hash_file failed for empty file");

    assert_eq!(
        result.digest,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(result.bytes_processed, 0);
    assert_eq!(result.target.target_type, TargetType::File);
}

#[test]
fn test_02_nist_test_vector_abc() {
    let file = TempTestFile::new(b"abc");
    let result = StreamHasher::hash_file(file.path(), HashAlgorithm::Sha256)
        .expect("hash_file failed for 'abc'");

    assert_eq!(
        result.digest,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(result.bytes_processed, 3);
}

#[test]
fn test_03_nist_test_vector_standard_pattern() {
    let input = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    let file = TempTestFile::new(input);
    let result = StreamHasher::hash_file(file.path(), HashAlgorithm::Sha256)
        .expect("hash_file failed for pattern");

    assert_eq!(
        result.digest,
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    assert_eq!(result.bytes_processed, input.len() as u64);
}

#[test]
fn test_04_streaming_multi_chunk_file() {
    // Generate data larger than STREAM_CHUNK_SIZE (64 KiB)
    let total_bytes = STREAM_CHUNK_SIZE * 3 + 1337;
    let mut data = Vec::with_capacity(total_bytes);
    for i in 0..total_bytes {
        data.push((i % 256) as u8);
    }

    // Expected digest via direct sha2 calculation
    let mut expected_hasher = Sha256::new();
    expected_hasher.update(&data);
    let expected_digest = hex::encode(expected_hasher.finalize());

    let file = TempTestFile::new(&data);
    let result = StreamHasher::hash_file(file.path(), HashAlgorithm::Sha256)
        .expect("hash_file failed on multi-chunk file");

    assert_eq!(result.digest, expected_digest);
    assert_eq!(result.bytes_processed, total_bytes as u64);
}

#[test]
fn test_05_binary_null_bytes_handling() {
    let data = vec![0u8; 100_000];
    let mut expected_hasher = Sha256::new();
    expected_hasher.update(&data);
    let expected_digest = hex::encode(expected_hasher.finalize());

    let file = TempTestFile::new(&data);
    let result = StreamHasher::hash_file(file.path(), HashAlgorithm::Sha256)
        .expect("hash_file failed on null bytes");

    assert_eq!(result.digest, expected_digest);
    assert_eq!(result.bytes_processed, 100_000);
}

#[test]
fn test_06_non_existent_file_returns_error() {
    let non_existent = std::env::temp_dir().join(format!("non_existent_{}.tmp", Uuid::new_v4()));
    let res = StreamHasher::hash_file(&non_existent, HashAlgorithm::Sha256);
    assert!(res.is_err());
    let err_msg = res.err().unwrap().to_string();
    assert!(err_msg.contains("does not exist"));
}

#[test]
fn test_07_directory_target_returns_error() {
    let temp_dir = std::env::temp_dir();
    let res = StreamHasher::hash_file(&temp_dir, HashAlgorithm::Sha256);
    assert!(res.is_err());
    let err_msg = res.err().unwrap().to_string();
    assert!(err_msg.contains("directory"));
}

#[test]
fn test_08_digest_syntax_validation() {
    // Valid 64-character lowercase hex
    let valid = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert_eq!(
        HashVerifier::validate_digest_syntax(valid, HashAlgorithm::Sha256).unwrap(),
        valid
    );

    // Valid uppercase hex is normalized to lowercase
    let upper = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
    assert_eq!(
        HashVerifier::validate_digest_syntax(upper, HashAlgorithm::Sha256).unwrap(),
        valid
    );

    // Invalid length (short)
    assert!(HashVerifier::validate_digest_syntax("ba7816bf", HashAlgorithm::Sha256).is_err());

    // Invalid non-hex characters
    let invalid_chars = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz";
    assert!(HashVerifier::validate_digest_syntax(invalid_chars, HashAlgorithm::Sha256).is_err());
}

#[test]
fn test_09_constant_time_comparison() {
    let s1 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let s2 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let s3 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    assert!(HashVerifier::constant_time_compare(s1, s2));
    assert!(!HashVerifier::constant_time_compare(s1, s3));
    assert!(!HashVerifier::constant_time_compare(s1, "short"));
}

#[test]
fn test_10_verifier_file_outcomes() {
    let file = TempTestFile::new(b"abc");
    let valid_sha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let mismatch_sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    // 1. Matching -> Verified
    let v_match = HashVerifier::verify_file(file.path(), valid_sha256, HashAlgorithm::Sha256);
    assert_eq!(v_match.status, VerificationStatus::Verified);
    assert_eq!(v_match.calculated_digest.as_deref(), Some(valid_sha256));

    // 2. Mismatched -> Mismatch
    let v_mismatch = HashVerifier::verify_file(file.path(), mismatch_sha256, HashAlgorithm::Sha256);
    match v_mismatch.status {
        VerificationStatus::Mismatch {
            expected,
            calculated,
        } => {
            assert_eq!(expected, mismatch_sha256);
            assert_eq!(calculated, valid_sha256);
        }
        other => panic!("Expected Mismatch, got {:?}", other),
    }

    // 3. Malformed expected digest -> UnableToVerify
    let v_invalid_hex = HashVerifier::verify_file(file.path(), "not-a-hash", HashAlgorithm::Sha256);
    match v_invalid_hex.status {
        VerificationStatus::UnableToVerify { reason } => {
            assert!(reason.contains("length") || reason.contains("hex"));
        }
        other => panic!("Expected UnableToVerify, got {:?}", other),
    }

    // 4. Missing target file -> UnableToVerify
    let non_existent = std::env::temp_dir().join(format!("missing_{}.tmp", Uuid::new_v4()));
    let v_missing = HashVerifier::verify_file(&non_existent, valid_sha256, HashAlgorithm::Sha256);
    match v_missing.status {
        VerificationStatus::UnableToVerify { reason } => {
            assert!(reason.contains("does not exist"));
        }
        other => panic!("Expected UnableToVerify, got {:?}", other),
    }
}

#[test]
fn test_11_integrity_service_lifecycle_and_audit() {
    let (integrity, _, audit) = setup_test_service();
    let file = TempTestFile::new(b"forensic evidence payload");
    let actor = "admin_investigator";

    // 1. Calculate hash
    let hash_res = integrity
        .calculate_file_hash(file.path(), Some(actor))
        .expect("calculate_file_hash failed");

    assert!(!hash_res.digest.is_empty());
    assert_eq!(hash_res.bytes_processed, 25);

    // 2. Verify audit chain logged the event
    let events = audit.list_events(10).expect("list_events failed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "EVIDENCE_HASH_CALCULATED");
    assert_eq!(events[0].actor_id.as_deref(), Some(actor));

    // Audit chain is cryptographically intact
    let chain_check = audit.verify_chain().expect("verify_chain failed");
    assert!(chain_check.is_valid);
    assert_eq!(chain_check.total_events, 1);

    // 3. Verify file against its calculated hash
    let verify_res = integrity
        .verify_file_hash(file.path(), &hash_res.digest, Some(actor))
        .expect("verify_file_hash failed");

    assert_eq!(verify_res.status, VerificationStatus::Verified);

    // Audit chain has 2 events now
    let events_after = audit.list_events(10).expect("list_events failed");
    assert_eq!(events_after.len(), 2);
    assert_eq!(events_after[0].event_type, "INTEGRITY_VERIFICATION_PASS");

    // 4. List integrity records
    let records = integrity
        .list_integrity_records(10)
        .expect("list_integrity_records failed");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].verification_status, "Verified");
    assert_eq!(records[1].verification_status, "Calculated");
}
