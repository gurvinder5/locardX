use locardx_audit::AuditService;
use locardx_auth::AuthService;
use locardx_common::{OperationType, TargetIdentity, TargetType};
use locardx_database::Database;
use locardx_device_manager::{DeviceManagerService, MockDeviceProvider};
use locardx_operation_manager::OperationManager;
use locardx_security::SafetyEngine;
use locardx_verification::{IntegrityService, SanitizationScope, SanitizationService};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::sync::Arc;

/// Generates a test file with known deterministic pseudo-random bytes.
fn create_test_file_with_fixed_payload(path: &std::path::Path, size_bytes: usize) -> String {
    let mut file = File::create(path).expect("Failed to create test file");
    let mut hasher = Sha256::new();

    let mut buf = Vec::with_capacity(size_bytes);
    for i in 0..size_bytes {
        let byte = ((i * 31 + 17) % 256) as u8;
        buf.push(byte);
    }

    file.write_all(&buf).expect("Failed to write test bytes");
    file.flush().expect("Failed to flush test file");
    hasher.update(&buf);

    hex::encode(hasher.finalize())
}

/// Computes the SHA-256 digest of a file on disk.
fn compute_file_sha256(path: &std::path::Path) -> String {
    let mut file = File::open(path).expect("Failed to open file for hashing");
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let count = file.read(&mut buffer).expect("Read failed");
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    hex::encode(hasher.finalize())
}

#[tokio::test]
async fn test_16_no_write_invariant_proves_zero_storage_modification() {
    let temp_dir = std::env::temp_dir();
    let file_id = uuid::Uuid::new_v4().to_string();
    let test_file_path = temp_dir.join(format!("locardx_no_write_test_{}.bin", file_id));

    // 1. Create a 512 KB test file with fixed non-zero byte pattern
    let original_size = 512 * 1024;
    let original_digest = create_test_file_with_fixed_payload(&test_file_path, original_size);

    // 2. Initialize application core subsystems
    let db = Arc::new(Database::open(":memory:").expect("Failed to open memory DB"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let integrity = Arc::new(IntegrityService::new(Arc::clone(&db), Arc::clone(&audit)));
    let dev_mgr = Arc::new(DeviceManagerService::new(Arc::new(
        MockDeviceProvider::new_default(),
    )));
    let auth = Arc::new(AuthService::new(Arc::clone(&db), Arc::clone(&audit)));
    let safety = Arc::new(SafetyEngine::new(
        Arc::clone(&db),
        Arc::clone(&audit),
        Arc::clone(&auth),
        Arc::clone(&dev_mgr),
    ));

    let sanitization = Arc::new(SanitizationService::new(
        Arc::clone(&db),
        Arc::clone(&audit),
        Arc::clone(&dev_mgr),
        Some(Arc::clone(&safety)),
    ));

    let op_mgr = Arc::new(
        OperationManager::new(Arc::clone(&db), Arc::clone(&audit), Arc::clone(&integrity))
            .with_safety(Arc::clone(&safety))
            .with_sanitization(Arc::clone(&sanitization)),
    );

    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: test_file_path.to_str().unwrap().to_string(),
        display_name: "Invariant Test File".to_string(),
        size_bytes: Some(original_size as u64),
    };

    // 3. Run SanitizationService::evaluate_plan
    let plan = sanitization
        .evaluate_plan(
            &target,
            SanitizationScope::File,
            None,
            None,
            Some("safety_auditor".to_string()),
        )
        .expect("Sanitization plan evaluation failed");

    assert!(plan.is_applicable);

    // Verify file remains completely untouched after plan evaluation
    let digest_after_planning = compute_file_sha256(&test_file_path);
    assert_eq!(
        original_digest, digest_after_planning,
        "NO-WRITE VIOLATION: File bytes mutated during plan evaluation"
    );

    // 4. Run SanitizationPlanEvaluation through OperationManager
    let mut params = serde_json::Map::new();
    params.insert(
        "scope".to_string(),
        serde_json::Value::String("File".to_string()),
    );

    let op = op_mgr
        .submit_and_run(
            OperationType::SanitizationPlanEvaluation,
            target.clone(),
            Some("safety_auditor".to_string()),
            Some(serde_json::Value::Object(params)),
        )
        .await
        .expect("submit_and_run failed for SanitizationPlanEvaluation");

    // Wait for background execution to complete
    let mut completed = false;
    for _ in 0..50 {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        if let Ok(updated_op) = op_mgr.get_operation(&op.operation_id) {
            if updated_op.current_state == locardx_operation_manager::OperationState::Completed {
                completed = true;
                break;
            }
        }
    }
    assert!(completed, "Operation did not reach Completed state");

    // 5. INVARIANT VERIFICATION: Prove zero storage modification
    let digest_after_operation = compute_file_sha256(&test_file_path);
    assert_eq!(
        original_digest, digest_after_operation,
        "CRITICAL NO-WRITE INVARIANT VIOLATION: File contents mutated during operation execution!"
    );

    let metadata_after = std::fs::metadata(&test_file_path).expect("Failed to get file metadata");
    assert_eq!(
        metadata_after.len(),
        original_size as u64,
        "CRITICAL NO-WRITE INVARIANT VIOLATION: File length changed!"
    );

    // 6. Verify destructive operations fail closed and do NOT modify storage
    let destructive_attempt = op_mgr
        .submit_and_run(
            OperationType::FileErasure,
            target.clone(),
            Some("malicious_attempt".to_string()),
            None,
        )
        .await;

    assert!(
        destructive_attempt.is_err(),
        "Destructive FileErasure MUST be rejected in Step 8"
    );

    let digest_after_destructive_attempt = compute_file_sha256(&test_file_path);
    assert_eq!(
        original_digest, digest_after_destructive_attempt,
        "CRITICAL NO-WRITE INVARIANT VIOLATION: File contents mutated after rejected destructive operation!"
    );

    // Clean up temporary invariant test file
    let _ = std::fs::remove_file(&test_file_path);
}
