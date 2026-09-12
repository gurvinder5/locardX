use locardx_audit::AuditService;
use locardx_auth::{
    models::{CreateUserRequest, InitAdminRequest, LoginRequest, UserRole},
    AuthService,
};
use locardx_common::{OperationType, TargetIdentity, TargetType};
use locardx_database::Database;
use locardx_device_manager::{
    DeviceClassification, DeviceDiscoveryProvider, DeviceManagerService, DeviceType, Filesystem,
    FilesystemType, LogicalVolume, PhysicalDevice,
};
use locardx_security::{ReasonCode, RiskLevel, SafetyDecisionOutcome, SafetyEngine};
use std::sync::Arc;

struct MockDiscoveryProvider {
    devices: Vec<PhysicalDevice>,
}

impl MockDiscoveryProvider {
    fn new_standard() -> Self {
        Self {
            devices: vec![
                PhysicalDevice {
                    device_id: "\\\\.\\PhysicalDrive0".to_string(),
                    display_name: "NVMe Samsung 980 PRO 1TB".to_string(),
                    vendor: Some("Samsung".to_string()),
                    model: Some("980 PRO".to_string()),
                    serial_number: Some("S69ENF0R123456".to_string()),
                    device_type: DeviceType::Ssd,
                    capacity_bytes: 1_000_204_886_016,
                    removable: false,
                    read_only: false,
                    is_system_device: true,
                    classification: DeviceClassification::SystemDevice,
                    volumes: vec![
                        LogicalVolume {
                            volume_id: "vol-system-0".to_string(),
                            mount_point: Some("C:\\".to_string()),
                            label: Some("Windows-OS".to_string()),
                            filesystem: Filesystem {
                                fs_type: FilesystemType::Ntfs,
                                label: Some("Windows-OS".to_string()),
                                read_only: false,
                            },
                            capacity_bytes: 900_000_000_000,
                            free_bytes: 450_000_000_000,
                            is_system_volume: true,
                            is_boot_volume: false,
                            read_only: false,
                        },
                        LogicalVolume {
                            volume_id: "vol-boot-0".to_string(),
                            mount_point: None,
                            label: Some("ESP".to_string()),
                            filesystem: Filesystem {
                                fs_type: FilesystemType::Fat32,
                                label: Some("ESP".to_string()),
                                read_only: false,
                            },
                            capacity_bytes: 512_000_000,
                            free_bytes: 400_000_000,
                            is_system_volume: false,
                            is_boot_volume: true,
                            read_only: false,
                        },
                    ],
                },
                PhysicalDevice {
                    device_id: "\\\\.\\PhysicalDrive1".to_string(),
                    display_name: "SanDisk Ultra USB 3.0".to_string(),
                    vendor: Some("SanDisk".to_string()),
                    model: Some("Ultra".to_string()),
                    serial_number: Some("SDUSB12345".to_string()),
                    device_type: DeviceType::Usb,
                    capacity_bytes: 32_000_000_000,
                    removable: true,
                    read_only: false,
                    is_system_device: false,
                    classification: DeviceClassification::RemovableDevice,
                    volumes: vec![LogicalVolume {
                        volume_id: "vol-usb-1".to_string(),
                        mount_point: Some("E:\\".to_string()),
                        label: Some("EVIDENCE_USB".to_string()),
                        filesystem: Filesystem {
                            fs_type: FilesystemType::ExFat,
                            label: Some("EVIDENCE_USB".to_string()),
                            read_only: false,
                        },
                        capacity_bytes: 32_000_000_000,
                        free_bytes: 16_000_000_000,
                        is_system_volume: false,
                        is_boot_volume: false,
                        read_only: false,
                    }],
                },
            ],
        }
    }
}

impl DeviceDiscoveryProvider for MockDiscoveryProvider {
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, locardx_common::LocardError> {
        Ok(self.devices.clone())
    }
}

struct DynamicDiscoveryProvider {
    devices: std::sync::RwLock<Vec<PhysicalDevice>>,
}

impl DynamicDiscoveryProvider {
    fn new(devices: Vec<PhysicalDevice>) -> Self {
        Self {
            devices: std::sync::RwLock::new(devices),
        }
    }

    fn set_devices(&self, devices: Vec<PhysicalDevice>) {
        *self.devices.write().unwrap() = devices;
    }
}

impl DeviceDiscoveryProvider for DynamicDiscoveryProvider {
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, locardx_common::LocardError> {
        Ok(self.devices.read().unwrap().clone())
    }
}

fn setup_test_environment() -> (
    SafetyEngine,
    Arc<AuthService>,
    Arc<AuditService>,
    Arc<Database>,
) {
    let db = Arc::new(Database::open(":memory:").expect("Failed to open DB"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let auth = Arc::new(AuthService::new(Arc::clone(&db), Arc::clone(&audit)));
    let dev_mgr = Arc::new(DeviceManagerService::new(Arc::new(
        MockDiscoveryProvider::new_standard(),
    )));

    let safety = SafetyEngine::new(
        Arc::clone(&db),
        Arc::clone(&audit),
        Arc::clone(&auth),
        dev_mgr,
    );

    (safety, auth, audit, db)
}

fn create_test_user(auth: &AuthService, username: &str, role: UserRole) -> String {
    // Ensure admin exists
    if auth.is_first_run().unwrap_or(false) {
        auth.initialize_admin(InitAdminRequest {
            username: "root_admin".to_string(),
            password: "AdminPassword123!".to_string(),
            confirm_password: "AdminPassword123!".to_string(),
            display_name: None,
        })
        .expect("Admin init failed");
    }

    let admin_session = auth
        .login(LoginRequest {
            username: "root_admin".to_string(),
            password: "AdminPassword123!".to_string(),
        })
        .expect("Admin login failed");

    if username == "root_admin" {
        return admin_session.token;
    }

    auth.create_user(
        &admin_session.token,
        CreateUserRequest {
            username: username.to_string(),
            password: "UserPassword123!".to_string(),
            role,
            display_name: None,
            metadata_json: None,
        },
    )
    .expect("Create user failed");

    let session = auth
        .login(LoginRequest {
            username: username.to_string(),
            password: "UserPassword123!".to_string(),
        })
        .expect("User login failed");

    session.token
}

#[test]
fn test_01_viewer_role_denied_destructive_requests() {
    let (safety, auth, _, _) = setup_test_environment();
    let viewer_token = create_test_user(&auth, "test_viewer", UserRole::Viewer);

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive1".to_string(),
        display_name: "SanDisk USB".to_string(),
        size_bytes: Some(32_000_000_000),
    };

    let decision = safety
        .evaluate_safety(&target, OperationType::DriveErasure, Some(&viewer_token))
        .expect("evaluate_safety failed");

    assert_eq!(decision.decision, SafetyDecisionOutcome::Denied);
    assert_eq!(decision.reason_code, ReasonCode::UnauthorizedRole);
}

#[test]
fn test_02_operator_role_requires_confirmation_for_external() {
    let (safety, auth, _, _) = setup_test_environment();
    let op_token = create_test_user(&auth, "test_operator", UserRole::Operator);

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive1".to_string(),
        display_name: "SanDisk USB".to_string(),
        size_bytes: Some(32_000_000_000),
    };

    let decision = safety
        .evaluate_safety(&target, OperationType::DriveErasure, Some(&op_token))
        .expect("evaluate_safety failed");

    assert_eq!(
        decision.decision,
        SafetyDecisionOutcome::RequiresConfirmation
    );
    assert_eq!(decision.reason_code, ReasonCode::MissingConfirmation);
    assert_eq!(decision.risk_level, RiskLevel::High);
    assert!(decision.requires_confirmation);
}

#[test]
fn test_03_system_device_is_hard_blocked() {
    let (safety, auth, _, _) = setup_test_environment();
    let admin_token = create_test_user(&auth, "root_admin", UserRole::Administrator);

    // Target the system physical disk
    let disk_target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive0".to_string(),
        display_name: "Samsung NVMe 980".to_string(),
        size_bytes: Some(1_000_204_886_016),
    };

    let decision = safety
        .evaluate_safety(
            &disk_target,
            OperationType::DriveErasure,
            Some(&admin_token),
        )
        .expect("evaluate_safety failed");

    assert_eq!(decision.decision, SafetyDecisionOutcome::Blocked);
    assert_eq!(decision.reason_code, ReasonCode::SystemDevice);
    assert_eq!(decision.risk_level, RiskLevel::Critical);

    // Target the C: system volume
    let vol_target = TargetIdentity {
        target_type: TargetType::LogicalVolume,
        identifier: "C:\\".to_string(),
        display_name: "Windows-OS".to_string(),
        size_bytes: Some(900_000_000_000),
    };

    let vol_decision = safety
        .evaluate_safety(
            &vol_target,
            OperationType::FolderErasure,
            Some(&admin_token),
        )
        .expect("evaluate_safety failed");

    assert_eq!(vol_decision.decision, SafetyDecisionOutcome::Blocked);
    assert_eq!(vol_decision.reason_code, ReasonCode::SystemDevice);
    assert_eq!(vol_decision.risk_level, RiskLevel::Critical);
}

#[test]
fn test_04_boot_device_is_hard_blocked() {
    let (safety, auth, _, _) = setup_test_environment();
    let admin_token = create_test_user(&auth, "root_admin", UserRole::Administrator);

    let boot_target = TargetIdentity {
        target_type: TargetType::LogicalVolume,
        identifier: "vol-boot-0".to_string(),
        display_name: "ESP".to_string(),
        size_bytes: Some(512_000_000),
    };

    let decision = safety
        .evaluate_safety(
            &boot_target,
            OperationType::DriveErasure,
            Some(&admin_token),
        )
        .expect("evaluate_safety failed");

    assert_eq!(decision.decision, SafetyDecisionOutcome::Blocked);
    assert_eq!(decision.reason_code, ReasonCode::BootDevice);
    assert_eq!(decision.risk_level, RiskLevel::Critical);
}

#[test]
fn test_05_unknown_or_missing_target_is_blocked() {
    let (safety, auth, _, _) = setup_test_environment();
    let admin_token = create_test_user(&auth, "root_admin", UserRole::Administrator);

    let missing_target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive99".to_string(),
        display_name: "Ghost Drive".to_string(),
        size_bytes: None,
    };

    let decision = safety
        .evaluate_safety(
            &missing_target,
            OperationType::DriveErasure,
            Some(&admin_token),
        )
        .expect("evaluate_safety failed");

    assert_eq!(decision.decision, SafetyDecisionOutcome::Blocked);
    assert_eq!(decision.reason_code, ReasonCode::InvalidTarget);
}

#[test]
fn test_06_read_only_integrity_hash_allowed_without_confirmation() {
    let (safety, auth, _, _) = setup_test_environment();
    let viewer_token = create_test_user(&auth, "test_viewer", UserRole::Viewer);

    // Create a temporary file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("locardx_safety_test_file.bin");
    std::fs::write(&temp_file, b"test content").unwrap();

    let target = TargetIdentity {
        target_type: TargetType::File,
        identifier: temp_file.to_str().unwrap().to_string(),
        display_name: "test_file.bin".to_string(),
        size_bytes: Some(12),
    };

    let decision = safety
        .evaluate_safety(&target, OperationType::IntegrityHash, Some(&viewer_token))
        .expect("evaluate_safety failed");

    let _ = std::fs::remove_file(temp_file);

    assert_eq!(decision.decision, SafetyDecisionOutcome::Allowed);
    assert_eq!(decision.reason_code, ReasonCode::ValidTarget);
    assert!(!decision.requires_confirmation);
}

#[test]
fn test_07_two_stage_confirmation_workflow() {
    let (safety, auth, audit, _) = setup_test_environment();
    let op_token = create_test_user(&auth, "test_operator", UserRole::Operator);

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive1".to_string(),
        display_name: "SanDisk USB".to_string(),
        size_bytes: Some(32_000_000_000),
    };

    // Stage 1: Request Confirmation
    let challenge = safety
        .request_destructive_confirmation("op-100", &target, OperationType::DriveErasure, &op_token)
        .expect("request_destructive_confirmation failed");

    assert_eq!(challenge.operation_id, "op-100");
    assert_eq!(challenge.actor_id, "test_operator");
    assert_eq!(challenge.target.identifier, "\\\\.\\PhysicalDrive1");

    // Stage 2: Confirm Destructive Operation
    let decision = safety
        .confirm_destructive_operation(
            &challenge.confirmation_id,
            "op-100",
            true,
            "\\\\.\\PhysicalDrive1",
            &op_token,
        )
        .expect("confirm_destructive_operation failed");

    // Must be blocked because destructive executors remain permanently disabled
    assert_eq!(decision.decision, SafetyDecisionOutcome::Blocked);
    assert_eq!(decision.reason_code, ReasonCode::OperationDisabled);

    // Verify audit chain
    let events = audit.list_events(10).expect("list_events failed");
    let event_types: Vec<String> = events.into_iter().map(|e| e.event_type).collect();
    assert!(event_types.contains(&"CONFIRMATION_REQUESTED".to_string()));
    assert!(event_types.contains(&"CONFIRMATION_ACCEPTED".to_string()));
}

#[test]
fn test_08_confirmation_actor_mismatch_rejected() {
    let (safety, auth, _, _) = setup_test_environment();
    let op_token = create_test_user(&auth, "test_operator", UserRole::Operator);
    let admin_token = create_test_user(&auth, "root_admin", UserRole::Administrator);

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive1".to_string(),
        display_name: "SanDisk USB".to_string(),
        size_bytes: Some(32_000_000_000),
    };

    let challenge = safety
        .request_destructive_confirmation("op-200", &target, OperationType::DriveErasure, &op_token)
        .unwrap();

    // Admin tries to submit confirmation that belonged to operator
    let result = safety.confirm_destructive_operation(
        &challenge.confirmation_id,
        "op-200",
        true,
        "\\\\.\\PhysicalDrive1",
        &admin_token,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("actor mismatch"));
}

#[test]
fn test_09_target_change_detection_prevents_toctou() {
    let db = Arc::new(Database::open(":memory:").unwrap());
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let auth = Arc::new(AuthService::new(Arc::clone(&db), Arc::clone(&audit)));

    let initial_device = PhysicalDevice {
        device_id: "\\\\.\\PhysicalDrive1".to_string(),
        display_name: "Removable USB".to_string(),
        vendor: Some("Generic".to_string()),
        model: Some("USB".to_string()),
        serial_number: Some("12345".to_string()),
        device_type: DeviceType::Usb,
        capacity_bytes: 16_000_000_000,
        removable: true,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::RemovableDevice,
        volumes: vec![],
    };

    let dynamic_provider = Arc::new(DynamicDiscoveryProvider::new(vec![initial_device]));
    let dev_mgr = Arc::new(DeviceManagerService::new(
        Arc::clone(&dynamic_provider) as Arc<dyn DeviceDiscoveryProvider>
    ));

    let safety = SafetyEngine::new(
        Arc::clone(&db),
        Arc::clone(&audit),
        Arc::clone(&auth),
        dev_mgr,
    );
    let op_token = create_test_user(&auth, "test_operator", UserRole::Operator);

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive1".to_string(),
        display_name: "Removable USB".to_string(),
        size_bytes: Some(16_000_000_000),
    };

    // Stage 1: Request confirmation with 16GB disk
    let challenge = safety
        .request_destructive_confirmation(
            "op-toctou",
            &target,
            OperationType::DriveErasure,
            &op_token,
        )
        .unwrap();

    // TOCTOU event: Physical device replaced with a different 64GB drive under same identifier
    let swapped_device = PhysicalDevice {
        device_id: "\\\\.\\PhysicalDrive1".to_string(),
        display_name: "Different Swapped USB".to_string(),
        vendor: Some("Malicious".to_string()),
        model: Some("Swapped".to_string()),
        serial_number: Some("99999".to_string()),
        device_type: DeviceType::Usb,
        capacity_bytes: 64_000_000_000, // Capacity changed!
        removable: true,
        read_only: false,
        is_system_device: false,
        classification: DeviceClassification::RemovableDevice,
        volumes: vec![],
    };
    dynamic_provider.set_devices(vec![swapped_device]);

    // Stage 2: Attempt confirmation
    let result = safety.confirm_destructive_operation(
        &challenge.confirmation_id,
        "op-toctou",
        true,
        "\\\\.\\PhysicalDrive1",
        &op_token,
    );

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Target change detected") || err_msg.contains("TARGET_CHANGED"));
}

#[test]
fn test_10_confirmation_cannot_override_system_device() {
    let (safety, auth, _, _) = setup_test_environment();
    let admin_token = create_test_user(&auth, "root_admin", UserRole::Administrator);

    let system_target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive0".to_string(),
        display_name: "System NVMe".to_string(),
        size_bytes: Some(1_000_204_886_016),
    };

    // Cannot even request confirmation for system device
    let result = safety.request_destructive_confirmation(
        "op-sys-fail",
        &system_target,
        OperationType::DriveErasure,
        &admin_token,
    );

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("System and Boot devices cannot be targeted"));
}
