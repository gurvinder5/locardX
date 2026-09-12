use locardx_audit::AuditService;
use locardx_auth::{
    models::{InitAdminRequest, LoginRequest},
    AuthService,
};
use locardx_common::{OperationType, TargetIdentity, TargetType};
use locardx_database::Database;
use locardx_device_manager::{
    DeviceClassification, DeviceDiscoveryProvider, DeviceManagerService, DeviceType, PhysicalDevice,
};
use locardx_security::{ReasonCode, SafetyDecisionOutcome, SafetyEngine};
use std::sync::Arc;

struct MockInvariantProvider;

impl DeviceDiscoveryProvider for MockInvariantProvider {
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, locardx_common::LocardError> {
        Ok(vec![
            PhysicalDevice {
                device_id: "\\\\.\\PhysicalDrive0".to_string(),
                display_name: "Host NVMe System Disk".to_string(),
                vendor: Some("Samsung".to_string()),
                model: Some("980".to_string()),
                serial_number: Some("NVME001".to_string()),
                device_type: DeviceType::Ssd,
                capacity_bytes: 512_000_000_000,
                removable: false,
                read_only: false,
                is_system_device: true,
                classification: DeviceClassification::SystemDevice,
                volumes: vec![],
            },
            PhysicalDevice {
                device_id: "\\\\.\\PhysicalDrive1".to_string(),
                display_name: "External USB Stick".to_string(),
                vendor: Some("Kingston".to_string()),
                model: Some("DataTraveler".to_string()),
                serial_number: Some("USB002".to_string()),
                device_type: DeviceType::Usb,
                capacity_bytes: 16_000_000_000,
                removable: true,
                read_only: false,
                is_system_device: false,
                classification: DeviceClassification::RemovableDevice,
                volumes: vec![],
            },
        ])
    }
}

#[test]
fn test_security_invariant_client_cannot_force_destructive_execution() {
    let db = Arc::new(Database::open(":memory:").expect("DB failed"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let auth = Arc::new(AuthService::new(Arc::clone(&db), Arc::clone(&audit)));
    let dev_mgr = Arc::new(DeviceManagerService::new(Arc::new(MockInvariantProvider)));
    let safety = SafetyEngine::new(
        Arc::clone(&db),
        Arc::clone(&audit),
        Arc::clone(&auth),
        dev_mgr,
    );

    // Initialize root administrator
    auth.initialize_admin(InitAdminRequest {
        username: "admin_user".to_string(),
        password: "AdminPassword123!".to_string(),
        confirm_password: "AdminPassword123!".to_string(),
        display_name: None,
    })
    .unwrap();

    let session = auth
        .login(LoginRequest {
            username: "admin_user".to_string(),
            password: "AdminPassword123!".to_string(),
        })
        .unwrap();

    // INVARIANT SCENARIO 1: Malicious client attempts direct DriveErasure on system drive
    // with client-side claimed "confirmed = true"
    let system_target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive0".to_string(),
        display_name: "Host NVMe System Disk".to_string(),
        size_bytes: Some(512_000_000_000),
    };

    let eval_res = safety.evaluate_safety(
        &system_target,
        OperationType::DriveErasure,
        Some(&session.token),
    );

    assert!(eval_res.is_ok());
    let decision = eval_res.unwrap();
    // Absolute hard block: Cannot be confirmed or overridden
    assert_eq!(decision.decision, SafetyDecisionOutcome::Blocked);
    assert_eq!(decision.reason_code, ReasonCode::SystemDevice);
    assert!(!decision.requires_confirmation);

    // INVARIANT SCENARIO 2: Client attempts to bypass confirmation workflow on external target
    let usb_target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive1".to_string(),
        display_name: "External USB Stick".to_string(),
        size_bytes: Some(16_000_000_000),
    };

    // Submitting a fake or arbitrary confirmation_id fails
    let fake_confirm_res = safety.confirm_destructive_operation(
        "fake-confirmation-token-12345",
        "op-arbitrary",
        true,
        "\\\\.\\PhysicalDrive1",
        &session.token,
    );
    assert!(
        fake_confirm_res.is_err(),
        "Arbitrary client-provided confirmation token must be rejected"
    );

    // Even with a legitimate full two-stage confirmation, the final execution outcome
    // must report ReasonCode::OperationDisabled and NO destructive executor can run.
    let challenge = safety
        .request_destructive_confirmation(
            "op-real-challenge",
            &usb_target,
            OperationType::DriveErasure,
            &session.token,
        )
        .unwrap();

    let final_decision = safety
        .confirm_destructive_operation(
            &challenge.confirmation_id,
            "op-real-challenge",
            true,
            "\\\\.\\PhysicalDrive1",
            &session.token,
        )
        .unwrap();

    // The safety layer commits the confirmation, but terminates with Blocked / OperationDisabled
    assert_eq!(final_decision.decision, SafetyDecisionOutcome::Blocked);
    assert_eq!(final_decision.reason_code, ReasonCode::OperationDisabled);
    assert!(final_decision.message.contains("permanently disabled"));
}
