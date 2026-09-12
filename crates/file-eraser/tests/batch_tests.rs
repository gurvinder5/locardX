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
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
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
        username: "admin_batch".to_string(),
        password: "AdminPassword123!".to_string(),
        confirm_password: "AdminPassword123!".to_string(),
    })
    .expect("Init admin failed");

    let session = auth
        .login(LoginRequest {
            username: "admin_batch".to_string(),
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
async fn test_concurrent_erasure_operations_isolated() {
    let (service, safety, token) = setup_test_service();

    let temp_a =
        std::env::temp_dir().join(format!("test_concurrent_a_{}.txt", uuid::Uuid::new_v4()));
    let temp_b =
        std::env::temp_dir().join(format!("test_concurrent_b_{}.txt", uuid::Uuid::new_v4()));
    fs::write(&temp_a, b"Parallel file A data").unwrap();
    fs::write(&temp_b, b"Parallel file B data").unwrap();

    let str_a = temp_a.to_string_lossy().to_string();
    let str_b = temp_b.to_string_lossy().to_string();

    let plan_a = service
        .plan_file_erasure(&str_a, None, Some("admin_batch".to_string()))
        .await
        .unwrap();
    let plan_b = service
        .plan_file_erasure(&str_b, None, Some("admin_batch".to_string()))
        .await
        .unwrap();

    let op_a = format!("op-conc-a-{}", uuid::Uuid::new_v4());
    let op_b = format!("op-conc-b-{}", uuid::Uuid::new_v4());

    let ch_a = safety
        .request_destructive_confirmation(
            &op_a,
            &TargetIdentity {
                target_type: TargetType::File,
                identifier: plan_a.canonical_path.clone(),
                display_name: str_a.clone(),
                size_bytes: Some(plan_a.pre_metadata.size_bytes),
            },
            OperationType::FileErasure,
            &token,
        )
        .unwrap();

    let ch_b = safety
        .request_destructive_confirmation(
            &op_b,
            &TargetIdentity {
                target_type: TargetType::File,
                identifier: plan_b.canonical_path.clone(),
                display_name: str_b.clone(),
                size_bytes: Some(plan_b.pre_metadata.size_bytes),
            },
            OperationType::FileErasure,
            &token,
        )
        .unwrap();

    let s1 = Arc::clone(&service);
    let t1 = token.clone();
    let pa = plan_a.canonical_path.clone();
    let pid_a = plan_a.plan_id.clone();
    let handle_a = tokio::spawn(async move {
        s1.execute_file_erasure(
            &pid_a,
            &ch_a.confirmation_id,
            &op_a,
            &pa,
            true,
            &t1,
            None,
            None,
        )
        .await
    });

    let s2 = Arc::clone(&service);
    let t2 = token.clone();
    let pb = plan_b.canonical_path.clone();
    let pid_b = plan_b.plan_id.clone();
    let handle_b = tokio::spawn(async move {
        s2.execute_file_erasure(
            &pid_b,
            &ch_b.confirmation_id,
            &op_b,
            &pb,
            true,
            &t2,
            None,
            None,
        )
        .await
    });

    let (res_a, res_b) = tokio::join!(handle_a, handle_b);
    let r_a = res_a.unwrap().unwrap();
    let r_b = res_b.unwrap().unwrap();

    assert_eq!(r_a.status, FileEraseStatus::Completed);
    assert_eq!(r_b.status, FileEraseStatus::Completed);
    assert!(!temp_a.exists());
    assert!(!temp_b.exists());
}

#[tokio::test]
async fn test_toctou_mutation_detected_and_fails_closed() {
    let (service, safety, token) = setup_test_service();

    let temp_file = std::env::temp_dir().join(format!("test_toctou_{}.txt", uuid::Uuid::new_v4()));
    fs::write(&temp_file, b"Original file contents at time of planning").unwrap();
    let file_str = temp_file.to_string_lossy().to_string();

    // 1. Plan erasure and capture snapshot
    let plan = service
        .plan_file_erasure(&file_str, None, Some("admin_batch".to_string()))
        .await
        .unwrap();

    // 2. Request confirmation challenge
    let op_id = format!("op-toctou-{}", uuid::Uuid::new_v4());
    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: plan.canonical_path.clone(),
        display_name: file_str.clone(),
        size_bytes: Some(plan.pre_metadata.size_bytes),
    };
    let challenge = safety
        .request_destructive_confirmation(&op_id, &target, OperationType::FileErasure, &token)
        .unwrap();

    // 3. Mutate file behind the scenes after challenge was issued!
    fs::write(
        &temp_file,
        b"MODIFIED content behind the scenes - TOCTOU attack vector",
    )
    .unwrap();

    // 4. Attempt execution - MUST be rejected due to snapshot hash/size mismatch
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
        .await;

    // Clean up
    let _ = fs::remove_file(&temp_file);

    assert!(
        result.is_err(),
        "TOCTOU mutation must cause execution to fail-closed"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("Target change detected")
            || err_msg.contains("snapshot verification failed")
            || err_msg.contains("Security policy violation")
            || err_msg.contains("Target mutated")
    );
}

#[tokio::test]
async fn test_cooperative_cancellation_flag() {
    let (service, safety, token) = setup_test_service();

    let temp_file = std::env::temp_dir().join(format!("test_cancel_{}.txt", uuid::Uuid::new_v4()));
    fs::write(&temp_file, vec![0x55; 1024 * 1024]).unwrap(); // 1 MB
    let file_str = temp_file.to_string_lossy().to_string();

    let plan = service
        .plan_file_erasure(&file_str, None, Some("admin_batch".to_string()))
        .await
        .unwrap();

    let op_id = format!("op-cancel-{}", uuid::Uuid::new_v4());
    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: plan.canonical_path.clone(),
        display_name: file_str.clone(),
        size_bytes: Some(plan.pre_metadata.size_bytes),
    };
    let challenge = safety
        .request_destructive_confirmation(&op_id, &target, OperationType::FileErasure, &token)
        .unwrap();

    let cancelled_flag = Arc::new(AtomicBool::new(true)); // Pre-cancelled
    let cancel_check: Box<dyn Fn() -> bool + Send + Sync> = {
        let flag = Arc::clone(&cancelled_flag);
        Box::new(move || flag.load(Ordering::Relaxed))
    };

    let result = service
        .execute_file_erasure(
            &plan.plan_id,
            &challenge.confirmation_id,
            &op_id,
            &plan.canonical_path,
            true,
            &token,
            Some(&cancel_check),
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.status, FileEraseStatus::Cancelled);
    // Clean up
    let _ = fs::remove_file(&temp_file);
}
