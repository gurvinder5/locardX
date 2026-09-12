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
use std::fs::{self, File};
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
        username: "admin_folder".to_string(),
        password: "AdminPassword123!".to_string(),
        confirm_password: "AdminPassword123!".to_string(),
        display_name: None,
    })
    .expect("Init admin failed");

    let session = auth
        .login(LoginRequest {
            username: "admin_folder".to_string(),
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
async fn test_recursive_folder_erasure_nested() {
    let (service, safety, token) = setup_test_service();
    let root_dir =
        std::env::temp_dir().join(format!("test_folder_recursive_{}", uuid::Uuid::new_v4()));
    let sub_a = root_dir.join("sub_a");
    let sub_b = sub_a.join("sub_b");
    fs::create_dir_all(&sub_b).unwrap();

    let file_root = root_dir.join("root_file.txt");
    let file_a = sub_a.join("file_a.log");
    let file_b = sub_b.join("file_b.dat");

    fs::write(&file_root, b"Root level payload").unwrap();
    fs::write(&file_a, b"Sub A payload").unwrap();
    fs::write(&file_b, b"Sub B nested payload").unwrap();

    let dir_str = root_dir.to_string_lossy().to_string();

    let plan = service
        .plan_folder_erasure(
            &dir_str,
            Some(SanitizationMethod::DirectoryRecursiveShred),
            Some("admin_folder".to_string()),
        )
        .await
        .expect("Folder planning failed");

    assert_eq!(plan.scope, FileEraseScope::Folder);

    let op_id = format!("op-folder-{}", uuid::Uuid::new_v4());
    let target = TargetIdentity {
        target_type: TargetType::Directory,
        identifier: plan.canonical_path.clone(),
        display_name: dir_str.clone(),
        size_bytes: Some(plan.pre_metadata.size_bytes),
    };

    let challenge = safety
        .request_destructive_confirmation(&op_id, &target, OperationType::FolderErasure, &token)
        .expect("Challenge request failed");

    let result = service
        .execute_folder_erasure(
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
        !root_dir.exists(),
        "Entire directory hierarchy must be removed"
    );

    let stats = result.folder_stats.expect("Folder stats must be present");
    assert_eq!(stats.files_sanitized, 3);
    assert_eq!(stats.directories_removed, 3); // root_dir, sub_a, sub_b
    assert_eq!(stats.failures.len(), 0);
}

#[tokio::test]
async fn test_empty_folder_erasure() {
    let (service, safety, token) = setup_test_service();
    let root_dir = std::env::temp_dir().join(format!("test_folder_empty_{}", uuid::Uuid::new_v4()));
    fs::create_dir(&root_dir).unwrap();
    let dir_str = root_dir.to_string_lossy().to_string();

    let plan = service
        .plan_folder_erasure(&dir_str, None, Some("admin_folder".to_string()))
        .await
        .expect("Empty folder planning failed");

    let op_id = format!("op-empty-f-{}", uuid::Uuid::new_v4());
    let target = TargetIdentity {
        target_type: TargetType::Directory,
        identifier: plan.canonical_path.clone(),
        display_name: dir_str.clone(),
        size_bytes: Some(0),
    };

    let challenge = safety
        .request_destructive_confirmation(&op_id, &target, OperationType::FolderErasure, &token)
        .unwrap();

    let result = service
        .execute_folder_erasure(
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
    assert!(!root_dir.exists(), "Empty directory must be unlinked");
    let stats = result.folder_stats.unwrap();
    assert_eq!(stats.files_sanitized, 0);
    assert_eq!(stats.directories_removed, 1);
}

#[tokio::test]
async fn test_folder_with_unicode_and_spaces() {
    let (service, safety, token) = setup_test_service();
    let root_dir =
        std::env::temp_dir().join(format!("test folder unicode 案件_{}", uuid::Uuid::new_v4()));
    let sub = root_dir.join("sub folder 証拠");
    fs::create_dir_all(&sub).unwrap();

    let test_file = sub.join("forensic file 報告.txt");
    fs::write(&test_file, b"Unicode path test data").unwrap();

    let dir_str = root_dir.to_string_lossy().to_string();

    let plan = service
        .plan_folder_erasure(&dir_str, None, Some("admin_folder".to_string()))
        .await
        .expect("Planning failed");

    let op_id = format!("op-uni-{}", uuid::Uuid::new_v4());
    let target = TargetIdentity {
        target_type: TargetType::Directory,
        identifier: plan.canonical_path.clone(),
        display_name: dir_str.clone(),
        size_bytes: Some(plan.pre_metadata.size_bytes),
    };

    let challenge = safety
        .request_destructive_confirmation(&op_id, &target, OperationType::FolderErasure, &token)
        .unwrap();

    let result = service
        .execute_folder_erasure(
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
        .expect("Execution failed on unicode path");

    assert_eq!(result.status, FileEraseStatus::Completed);
    assert!(!root_dir.exists());
    let stats = result.folder_stats.unwrap();
    assert_eq!(stats.files_sanitized, 1);
}

#[tokio::test]
async fn test_folder_with_readonly_files() {
    let (service, safety, token) = setup_test_service();
    let root_dir = std::env::temp_dir().join(format!("test_folder_ro_{}", uuid::Uuid::new_v4()));
    fs::create_dir(&root_dir).unwrap();

    let ro_file = root_dir.join("readonly_file.txt");
    fs::write(&ro_file, b"Read-only file inside directory").unwrap();

    let mut perms = fs::metadata(&ro_file).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&ro_file, perms).unwrap();

    let dir_str = root_dir.to_string_lossy().to_string();

    let plan = service
        .plan_folder_erasure(&dir_str, None, Some("admin_folder".to_string()))
        .await
        .unwrap();

    let op_id = format!("op-ro-f-{}", uuid::Uuid::new_v4());
    let target = TargetIdentity {
        target_type: TargetType::Directory,
        identifier: plan.canonical_path.clone(),
        display_name: dir_str.clone(),
        size_bytes: Some(plan.pre_metadata.size_bytes),
    };

    let challenge = safety
        .request_destructive_confirmation(&op_id, &target, OperationType::FolderErasure, &token)
        .unwrap();

    let result = service
        .execute_folder_erasure(
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
    assert!(!root_dir.exists());
}

#[tokio::test]
async fn test_folder_containing_locked_file_partial_failure() {
    let (service, safety, token) = setup_test_service();
    let root_dir =
        std::env::temp_dir().join(format!("test_folder_locked_{}", uuid::Uuid::new_v4()));
    fs::create_dir(&root_dir).unwrap();

    let free_file = root_dir.join("normal_file.txt");
    fs::write(&free_file, b"This file can be safely deleted").unwrap();

    let locked_file = root_dir.join("locked_file.txt");
    fs::write(&locked_file, b"This file will be held open exclusively").unwrap();

    // Hold exclusive write lock on Windows (share_mode 0 prevents any other handle from opening/writing/deleting)
    #[cfg(windows)]
    use std::os::windows::fs::OpenOptionsExt;

    let mut opts = File::options();
    opts.read(true).write(true);
    #[cfg(windows)]
    opts.share_mode(0);
    let _open_guard = opts.open(&locked_file).unwrap();

    let dir_str = root_dir.to_string_lossy().to_string();

    let plan = service
        .plan_folder_erasure(&dir_str, None, Some("admin_folder".to_string()))
        .await
        .unwrap();

    let op_id = format!("op-lock-f-{}", uuid::Uuid::new_v4());
    let target = TargetIdentity {
        target_type: TargetType::Directory,
        identifier: plan.canonical_path.clone(),
        display_name: dir_str.clone(),
        size_bytes: Some(plan.pre_metadata.size_bytes),
    };

    let challenge = safety
        .request_destructive_confirmation(&op_id, &target, OperationType::FolderErasure, &token)
        .unwrap();

    let result = service
        .execute_folder_erasure(
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

    // Partial failure: free_file deleted, locked_file failed
    assert_eq!(result.status, FileEraseStatus::Failed);
    let stats = result.folder_stats.unwrap();
    assert!(
        stats.files_sanitized >= 1,
        "At least one un-locked file should be erased"
    );
    assert!(
        !stats.failures.is_empty(),
        "Failures must record the locked file"
    );

    // Explicit drop before cleanup
    drop(_open_guard);
    let _ = fs::remove_file(&locked_file);
    let _ = fs::remove_dir_all(&root_dir);
}
