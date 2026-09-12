use locardx_audit::AuditService;
use locardx_auth::{
    models::{InitAdminRequest, LoginRequest},
    AuthService,
};
use locardx_common::{OperationType, TargetIdentity, TargetType};
use locardx_database::Database;
use locardx_device_manager::{
    DeviceClassification, DeviceDiscoveryProvider, DeviceManagerService, DeviceType, Filesystem,
    FilesystemType, LogicalVolume, PhysicalDevice,
};
use locardx_file_eraser::models::FileEraseStatus;
use locardx_file_eraser::service::FileEraserService;
use locardx_security::SafetyEngine;
use locardx_verification::sanitization::{VerificationOutcome, VerificationStrategy};
use sha2::{Digest, Sha256};
use std::fs;
use std::sync::Arc;

struct MockDiscoveryProvider;
impl DeviceDiscoveryProvider for MockDiscoveryProvider {
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, locardx_common::LocardError> {
        Ok(vec![PhysicalDevice {
            device_id: "\\\\.\\PhysicalDrive0".to_string(),
            display_name: "Mock NVMe".to_string(),
            vendor: None,
            model: None,
            serial_number: None,
            device_type: DeviceType::Ssd,
            capacity_bytes: 512_000_000_000,
            removable: false,
            read_only: false,
            is_system_device: true,
            classification: DeviceClassification::SystemDevice,
            volumes: vec![LogicalVolume {
                volume_id: "vol-0".to_string(),
                mount_point: Some("C:\\".to_string()),
                label: Some("OS".to_string()),
                filesystem: Filesystem {
                    fs_type: FilesystemType::Ntfs,
                    label: Some("OS".to_string()),
                    read_only: false,
                },
                capacity_bytes: 500_000_000_000,
                free_bytes: 250_000_000_000,
                is_system_volume: true,
                is_boot_volume: false,
                read_only: false,
            }],
        }])
    }
}

fn setup_test_service() -> (Arc<FileEraserService>, Arc<SafetyEngine>, String) {
    let db = Arc::new(Database::open(":memory:").expect("Failed to open test database"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let auth = Arc::new(AuthService::new(Arc::clone(&db), Arc::clone(&audit)));
    let dev_mgr = Arc::new(DeviceManagerService::new(Arc::new(MockDiscoveryProvider)));

    let safety = Arc::new(SafetyEngine::new(
        Arc::clone(&db),
        Arc::clone(&audit),
        Arc::clone(&auth),
        dev_mgr,
    ));

    auth.initialize_admin(InitAdminRequest {
        username: "admin_meta".to_string(),
        password: "AdminPassword123!".to_string(),
        confirm_password: "AdminPassword123!".to_string(),
        display_name: None,
    })
    .expect("Init admin failed");

    let session = auth
        .login(LoginRequest {
            username: "admin_meta".to_string(),
            password: "AdminPassword123!".to_string(),
        })
        .expect("Login failed");

    let service = Arc::new(FileEraserService::new(
        Arc::clone(&db),
        Arc::clone(&audit),
        Arc::clone(&safety),
    ));

    (service, safety, session.token)
}

#[tokio::test]
async fn test_pre_erasure_snapshot_captures_accurate_metadata() {
    let (service, _safety, _token) = setup_test_service();
    let temp_file =
        std::env::temp_dir().join(format!("test_snapshot_{}.txt", uuid::Uuid::new_v4()));
    let payload = b"Forensic hash verification test string payload 1234567890";
    fs::write(&temp_file, payload).unwrap();

    let mut hasher = Sha256::new();
    hasher.update(payload);
    let expected_hash = hex::encode(hasher.finalize());

    let plan = service
        .plan_file_erasure(
            &temp_file.to_string_lossy(),
            None,
            Some("admin_meta".to_string()),
        )
        .await
        .expect("Planning failed");

    // Clean up
    let _ = fs::remove_file(&temp_file);

    assert_eq!(plan.pre_metadata.size_bytes, payload.len() as u64);
    assert_eq!(
        plan.pre_metadata.pre_erasure_sha256.as_deref(),
        Some(expected_hash.as_str())
    );
    assert!(!plan.pre_metadata.canonical_path.is_empty());
    assert!(plan.pre_metadata.created_at.is_some());
    assert!(plan.pre_metadata.modified_at.is_some());
}

#[tokio::test]
async fn test_verification_metadata_and_physical_limitations_disclaimer() {
    let (service, safety, token) = setup_test_service();
    let temp_file =
        std::env::temp_dir().join(format!("test_verify_meta_{}.txt", uuid::Uuid::new_v4()));
    fs::write(&temp_file, b"Payload for verification disclaimer check").unwrap();
    let file_str = temp_file.to_string_lossy().to_string();

    let plan = service
        .plan_file_erasure(&file_str, None, Some("admin_meta".to_string()))
        .await
        .unwrap();

    // Check plan includes limitations
    assert!(!plan.limitations.is_empty());
    assert!(plan
        .limitations
        .iter()
        .any(|l| l.contains("FTL") || l.to_lowercase().contains("wear leveling")));

    let op_id = format!("op-disclaim-{}", uuid::Uuid::new_v4());
    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: plan.canonical_path.clone(),
        display_name: file_str.clone(),
        size_bytes: Some(plan.pre_metadata.size_bytes),
    };

    let challenge = safety
        .request_destructive_confirmation(&op_id, &target, OperationType::FileErasure, &token)
        .unwrap();

    let result = service
        .execute_file_erasure(
            &plan.plan_id,
            &challenge.confirmation_id,
            &op_id,
            &plan.canonical_path,
            true,
            &token,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.status, FileEraseStatus::Completed);
    assert_eq!(result.verification.outcome, VerificationOutcome::Verified);
    assert_eq!(
        result.verification.strategy,
        VerificationStrategy::MetadataUnlinkCheck
    );
    assert!(!result.verification.path_exists);
    assert!(result.verification.inaccessible);

    // Limitations must be preserved in result
    assert!(!result.limitations.is_empty());
    assert_eq!(result.scope_description, "LOGICAL FILESYSTEM SCOPE");
}
