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
use locardx_file_eraser::models::{FileEraseScope, FileEraseStatus};
use locardx_file_eraser::service::FileEraserService;
use locardx_security::SafetyEngine;
use locardx_verification::sanitization::SanitizationMethod;
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
        username: "admin".to_string(),
        password: "AdminPassword123!".to_string(),
        confirm_password: "AdminPassword123!".to_string(),
        display_name: None,
    })
    .expect("Init admin failed");

    let session = auth
        .login(LoginRequest {
            username: "admin".to_string(),
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
async fn test_single_file_erasure_3_pass_shred() {
    let (service, safety, token) = setup_test_service();
    let temp_file = std::env::temp_dir().join(format!(
        "test_file_shred_3pass_{}.txt",
        uuid::Uuid::new_v4()
    ));
    fs::write(
        &temp_file,
        b"Forensic confidential data payload requiring 3-pass sanitization",
    )
    .unwrap();
    let file_str = temp_file.to_string_lossy().to_string();

    // 1. Plan erasure
    let plan = service
        .plan_file_erasure(
            &file_str,
            Some(SanitizationMethod::LogicalFileShred),
            Some("admin".to_string()),
        )
        .await
        .expect("Planning 3-pass shred failed");

    assert_eq!(plan.passes, 3);
    assert_eq!(plan.scope, FileEraseScope::File);

    // 2. Request confirmation challenge
    let op_id = format!("op-shred-{}", uuid::Uuid::new_v4());
    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: plan.canonical_path.clone(),
        display_name: file_str.clone(),
        size_bytes: Some(plan.pre_metadata.size_bytes),
    };

    let challenge = safety
        .request_destructive_confirmation(&op_id, &target, OperationType::FileErasure, &token)
        .expect("Challenge request failed");

    // 3. Execute erasure
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
        .expect("Execution failed");

    assert_eq!(result.status, FileEraseStatus::Completed);
    assert!(
        !temp_file.exists(),
        "Target file must be deleted from filesystem"
    );
    assert_eq!(
        result.verification.outcome,
        locardx_verification::sanitization::VerificationOutcome::Verified
    );
    assert!(result.bytes_processed > 0);
}

#[tokio::test]
async fn test_single_file_erasure_nist80088_clear() {
    let (service, safety, token) = setup_test_service();
    let temp_file =
        std::env::temp_dir().join(format!("test_file_nist_clear_{}.txt", uuid::Uuid::new_v4()));
    fs::write(&temp_file, vec![0xAB; 4096]).unwrap();
    let file_str = temp_file.to_string_lossy().to_string();

    // 1. Plan erasure with NIST 800-88 Clear (1 pass zeros)
    let plan = service
        .plan_file_erasure(
            &file_str,
            Some(SanitizationMethod::Nist80088ClearZero),
            Some("admin".to_string()),
        )
        .await
        .expect("Planning NIST Clear failed");

    assert_eq!(plan.passes, 1);

    // 2. Challenge
    let op_id = format!("op-nist-{}", uuid::Uuid::new_v4());
    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: plan.canonical_path.clone(),
        display_name: file_str.clone(),
        size_bytes: Some(plan.pre_metadata.size_bytes),
    };
    let challenge = safety
        .request_destructive_confirmation(&op_id, &target, OperationType::FileErasure, &token)
        .expect("Challenge failed");

    // 3. Execute
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
        .expect("Execution failed");

    assert_eq!(result.status, FileEraseStatus::Completed);
    assert!(!temp_file.exists());
    assert_eq!(result.bytes_processed, 4096);
}

#[tokio::test]
async fn test_read_only_file_handling() {
    let (service, safety, token) = setup_test_service();
    let temp_file =
        std::env::temp_dir().join(format!("test_readonly_file_{}.txt", uuid::Uuid::new_v4()));
    fs::write(
        &temp_file,
        b"Read only file content to be safely stripped and erased",
    )
    .unwrap();

    // Set read-only attribute
    let mut perms = fs::metadata(&temp_file).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&temp_file, perms).unwrap();

    let file_str = temp_file.to_string_lossy().to_string();

    let plan = service
        .plan_file_erasure(&file_str, None, Some("admin".to_string()))
        .await
        .expect("Planning failed");

    assert!(
        plan.pre_metadata.is_readonly,
        "Pre-metadata must record read-only attribute"
    );

    let op_id = format!("op-ro-{}", uuid::Uuid::new_v4());
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
        .expect("Execution should succeed even on read-only files");

    assert_eq!(result.status, FileEraseStatus::Completed);
    assert!(!temp_file.exists());
}

#[tokio::test]
async fn test_empty_file_handling() {
    let (service, safety, token) = setup_test_service();
    let temp_file =
        std::env::temp_dir().join(format!("test_empty_file_{}.txt", uuid::Uuid::new_v4()));
    fs::write(&temp_file, b"").unwrap();
    let file_str = temp_file.to_string_lossy().to_string();

    let plan = service
        .plan_file_erasure(&file_str, None, Some("admin".to_string()))
        .await
        .expect("Planning empty file failed");

    assert_eq!(plan.pre_metadata.size_bytes, 0);

    let op_id = format!("op-empty-{}", uuid::Uuid::new_v4());
    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: plan.canonical_path.clone(),
        display_name: file_str.clone(),
        size_bytes: Some(0),
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
        .expect("Empty file erasure failed");

    assert_eq!(result.status, FileEraseStatus::Completed);
    assert!(!temp_file.exists());
    assert_eq!(result.bytes_processed, 0);
}

#[tokio::test]
async fn test_non_existent_file_rejected() {
    let (service, _safety, _token) = setup_test_service();
    let nonexistent =
        std::env::temp_dir().join(format!("does_not_exist_{}.txt", uuid::Uuid::new_v4()));

    let res = service
        .plan_file_erasure(
            &nonexistent.to_string_lossy(),
            None,
            Some("admin".to_string()),
        )
        .await;

    assert!(res.is_err(), "Non-existent file must fail planning");
}

#[tokio::test]
async fn test_directory_passed_to_file_eraser_rejected() {
    let (service, _safety, _token) = setup_test_service();
    let temp_dir = std::env::temp_dir().join(format!("test_dir_as_file_{}", uuid::Uuid::new_v4()));
    fs::create_dir(&temp_dir).unwrap();

    let res = service
        .plan_file_erasure(&temp_dir.to_string_lossy(), None, Some("admin".to_string()))
        .await;

    // Clean up
    let _ = fs::remove_dir(&temp_dir);

    assert!(
        res.is_err(),
        "Passing a directory to plan_file_erasure must be rejected"
    );
}

#[tokio::test]
async fn test_system_and_boot_paths_rejected() {
    let (service, _safety, _token) = setup_test_service();

    let system_paths = vec![
        "C:\\Windows\\notepad.exe",
        "C:\\Windows\\System32\\cmd.exe",
        "C:\\Program Files\\Common Files",
        "C:\\pagefile.sys",
        "C:\\bootmgr",
        "C:\\",
        "/boot/vmlinuz",
        "/etc/shadow",
    ];

    for path in system_paths {
        let res = service
            .plan_file_erasure(path, None, Some("admin".to_string()))
            .await;
        assert!(
            res.is_err(),
            "System or boot path '{}' must be rejected unconditionally",
            path
        );
    }
}
