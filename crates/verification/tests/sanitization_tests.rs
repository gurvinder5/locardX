use chrono::Utc;
use locardx_audit::AuditService;
use locardx_auth::AuthService;
use locardx_common::{LocardError, TargetIdentity, TargetType};
use locardx_database::Database;
use locardx_device_manager::{
    DeviceClassification, DeviceDiscoveryProvider, DeviceManagerService, DeviceType, Filesystem,
    FilesystemType, LogicalVolume, PhysicalDevice,
};
use locardx_security::{ReasonCode, RiskLevel, SafetyEngine, TargetSnapshot};
use locardx_verification::sanitization::{
    compare_target_snapshots, verify_snapshot_integrity, FailureReason, MediaType,
    SanitizationMethod, SanitizationPlanner, SanitizationScope, SanitizationService,
    VerificationOutcome, VerificationStrategy,
};
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
                    model: Some("980 PRO NVMe".to_string()),
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
                    model: Some("Ultra USB".to_string()),
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
                PhysicalDevice {
                    device_id: "\\\\.\\PhysicalDrive2".to_string(),
                    display_name: "Seagate Barracuda 2TB HDD".to_string(),
                    vendor: Some("Seagate".to_string()),
                    model: Some("ST2000DM008 Hard Disk".to_string()),
                    serial_number: Some("W340ABC123".to_string()),
                    device_type: DeviceType::Hdd,
                    capacity_bytes: 2_000_398_934_016,
                    removable: false,
                    read_only: false,
                    is_system_device: false,
                    classification: DeviceClassification::FixedDataDevice,
                    volumes: vec![LogicalVolume {
                        volume_id: "vol-hdd-2".to_string(),
                        mount_point: Some("D:\\".to_string()),
                        label: Some("DataStorage".to_string()),
                        filesystem: Filesystem {
                            fs_type: FilesystemType::Ntfs,
                            label: Some("DataStorage".to_string()),
                            read_only: false,
                        },
                        capacity_bytes: 2_000_398_934_016,
                        free_bytes: 1_200_000_000_000,
                        is_system_volume: false,
                        is_boot_volume: false,
                        read_only: false,
                    }],
                },
                PhysicalDevice {
                    device_id: "\\\\.\\PhysicalDrive3".to_string(),
                    display_name: "Samsung SSD 990 PRO 2TB NVMe".to_string(),
                    vendor: Some("Samsung".to_string()),
                    model: Some("990 PRO NVMe".to_string()),
                    serial_number: Some("S70ENF0R999999".to_string()),
                    device_type: DeviceType::Ssd,
                    capacity_bytes: 2_000_398_934_016,
                    removable: false,
                    read_only: false,
                    is_system_device: false,
                    classification: DeviceClassification::FixedDataDevice,
                    volumes: vec![LogicalVolume {
                        volume_id: "vol-nvme-3".to_string(),
                        mount_point: Some("F:\\".to_string()),
                        label: Some("FastStorage".to_string()),
                        filesystem: Filesystem {
                            fs_type: FilesystemType::Ntfs,
                            label: Some("FastStorage".to_string()),
                            read_only: false,
                        },
                        capacity_bytes: 2_000_398_934_016,
                        free_bytes: 1_800_000_000_000,
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
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError> {
        Ok(self.devices.clone())
    }
}

fn setup_test_env() -> (
    SanitizationService,
    Arc<Database>,
    Arc<AuditService>,
    Arc<SafetyEngine>,
) {
    let db = Arc::new(Database::open(":memory:").expect("Failed to initialize memory DB"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let dev_mgr = Arc::new(DeviceManagerService::new(Arc::new(
        MockDiscoveryProvider::new_standard(),
    )));
    let auth = Arc::new(AuthService::new(Arc::clone(&db), Arc::clone(&audit)));
    let safety = Arc::new(SafetyEngine::new(
        Arc::clone(&db),
        Arc::clone(&audit),
        Arc::clone(&auth),
        Arc::clone(&dev_mgr),
    ));
    let sanitization = SanitizationService::new(
        Arc::clone(&db),
        Arc::clone(&audit),
        Arc::clone(&dev_mgr),
        Some(Arc::clone(&safety)),
    );

    (sanitization, db, audit, safety)
}

#[test]
fn test_01_hdd_method_selection_clear_and_dod() {
    let (sanitization, _, _, _) = setup_test_env();

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive2".to_string(),
        display_name: "Seagate 2TB HDD".to_string(),
        size_bytes: Some(2_000_398_934_016),
    };

    // Auto-recommendation for HDD
    let plan = sanitization
        .evaluate_plan(
            &target,
            SanitizationScope::PhysicalDevice,
            None,
            None,
            Some("test_operator".to_string()),
        )
        .expect("evaluate_plan failed");

    assert!(plan.is_applicable);
    assert_eq!(plan.media_type, MediaType::Hdd);
    assert_eq!(
        plan.recommended_method,
        SanitizationMethod::Nist80088ClearZero
    );
    assert_eq!(
        plan.verification_strategy,
        VerificationStrategy::SampledRandomSectors
    );
    assert_eq!(
        plan.applicable_standard.as_deref(),
        Some("NIST SP 800-88 Rev. 1 (Clear)")
    );

    // Request DoD 5220.22-M on HDD
    let dod_plan = sanitization
        .evaluate_plan(
            &target,
            SanitizationScope::PhysicalDevice,
            Some(SanitizationMethod::Dod522022M),
            None,
            Some("test_operator".to_string()),
        )
        .expect("DoD plan evaluation failed");

    assert!(dod_plan.is_applicable);
    assert_eq!(dod_plan.recommended_method, SanitizationMethod::Dod522022M);
    assert_eq!(
        dod_plan.applicable_standard.as_deref(),
        Some("DoD 5220.22-M (NISPOM)")
    );
}

#[test]
fn test_02_ssd_nvme_method_selection_and_ftl_limitations() {
    let (sanitization, _, _, _) = setup_test_env();

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive3".to_string(),
        display_name: "Samsung 990 PRO NVMe".to_string(),
        size_bytes: Some(2_000_398_934_016),
    };

    let plan = sanitization
        .evaluate_plan(
            &target,
            SanitizationScope::PhysicalDevice,
            None,
            None,
            Some("test_operator".to_string()),
        )
        .expect("evaluate_plan failed");

    assert!(plan.is_applicable);
    assert_eq!(plan.media_type, MediaType::NvmeSsd);
    assert_eq!(plan.recommended_method, SanitizationMethod::NvmeCryptoErase);
    assert_eq!(
        plan.verification_strategy,
        VerificationStrategy::CryptoKeyDestructionCheck
    );

    // Verify FTL limitation is explicitly documented
    assert!(
        plan.limitations
            .iter()
            .any(|l| l.contains("Flash Translation Layer")),
        "FTL wear-leveling limitation must be documented for NVMe SSD"
    );

    // Requesting DoD 5220.22-M on NVMe SSD should be flagged as not applicable due to write amplification
    let dod_on_ssd = sanitization
        .evaluate_plan(
            &target,
            SanitizationScope::PhysicalDevice,
            Some(SanitizationMethod::Dod522022M),
            None,
            Some("test_operator".to_string()),
        )
        .expect("evaluation should complete");

    assert!(
        !dod_on_ssd.is_applicable,
        "DoD 3-pass overwrite must not be applicable on NVMe SSD"
    );
}

#[test]
fn test_03_usb_removable_handling() {
    let (sanitization, _, _, _) = setup_test_env();

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive1".to_string(),
        display_name: "SanDisk USB".to_string(),
        size_bytes: Some(32_000_000_000),
    };

    let plan = sanitization
        .evaluate_plan(
            &target,
            SanitizationScope::PhysicalDevice,
            None,
            None,
            Some("test_operator".to_string()),
        )
        .expect("evaluate_plan failed");

    assert!(plan.is_applicable);
    assert_eq!(plan.media_type, MediaType::UsbRemovable);
    assert_eq!(
        plan.recommended_method,
        SanitizationMethod::Nist80088ClearZero
    );
    assert!(
        plan.limitations
            .iter()
            .any(|l| l.contains("wear-out") || l.contains("wear leveling")),
        "Removable flash wear limitations must be documented"
    );
}

#[test]
fn test_04_unknown_media_rejection() {
    let snapshot = TargetSnapshot {
        target_identifier: "\\\\.\\PhysicalDrive99".to_string(),
        target_type: TargetType::PhysicalDevice,
        capacity_bytes: Some(500_000_000),
        filesystem: None,
        device_id: Some("\\\\.\\PhysicalDrive99".to_string()),
        classification: Some(DeviceClassification::Unknown),
        is_system: false,
        is_boot: false,
        snapshot_timestamp: Utc::now().to_rfc3339(),
    };

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive99".to_string(),
        display_name: "Unknown Device".to_string(),
        size_bytes: Some(500_000_000),
    };

    let plan = SanitizationPlanner::evaluate_plan(
        &target,
        &snapshot,
        MediaType::Unknown,
        SanitizationScope::PhysicalDevice,
        None,
        None,
        None,
    );

    assert!(!plan.is_applicable, "Unknown media should be rejected");
    assert_eq!(plan.risk_level, RiskLevel::Critical);
    assert!(plan
        .reason_codes
        .contains(&ReasonCode::UnsupportedOperation));
}

#[test]
fn test_05_unsupported_method_rejection() {
    let snapshot = TargetSnapshot {
        target_identifier: "\\\\.\\PhysicalDrive2".to_string(),
        target_type: TargetType::PhysicalDevice,
        capacity_bytes: Some(2_000_000_000),
        filesystem: None,
        device_id: Some("\\\\.\\PhysicalDrive2".to_string()),
        classification: Some(DeviceClassification::FixedDataDevice),
        is_system: false,
        is_boot: false,
        snapshot_timestamp: Utc::now().to_rfc3339(),
    };

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive2".to_string(),
        display_name: "Seagate HDD".to_string(),
        size_bytes: Some(2_000_000_000),
    };

    let plan = SanitizationPlanner::evaluate_plan(
        &target,
        &snapshot,
        MediaType::Hdd,
        SanitizationScope::PhysicalDevice,
        Some(SanitizationMethod::Unsupported),
        None,
        None,
    );

    assert!(!plan.is_applicable);
    assert!(plan
        .reason_codes
        .contains(&ReasonCode::UnsupportedOperation));
}

#[test]
fn test_06_filesystem_scope_mismatch() {
    let snapshot = TargetSnapshot {
        target_identifier: "\\\\.\\PhysicalDrive2".to_string(),
        target_type: TargetType::PhysicalDevice,
        capacity_bytes: Some(2_000_000_000),
        filesystem: None,
        device_id: Some("\\\\.\\PhysicalDrive2".to_string()),
        classification: Some(DeviceClassification::FixedDataDevice),
        is_system: false,
        is_boot: false,
        snapshot_timestamp: Utc::now().to_rfc3339(),
    };

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive2".to_string(),
        display_name: "Physical Disk".to_string(),
        size_bytes: Some(2_000_000_000),
    };

    // Attempting to evaluate File scope on a PhysicalDevice target
    let plan = SanitizationPlanner::evaluate_plan(
        &target,
        &snapshot,
        MediaType::Hdd,
        SanitizationScope::File,
        None,
        None,
        None,
    );

    assert!(!plan.is_applicable, "Scope mismatch must be rejected");
    assert!(plan.reason_codes.contains(&ReasonCode::InvalidTarget));
}

#[test]
fn test_07_target_snapshot_match() {
    let plan_snapshot = TargetSnapshot {
        target_identifier: "\\\\.\\PhysicalDrive1".to_string(),
        target_type: TargetType::PhysicalDevice,
        capacity_bytes: Some(32_000_000_000),
        filesystem: None,
        device_id: Some("\\\\.\\PhysicalDrive1".to_string()),
        classification: Some(DeviceClassification::RemovableDevice),
        is_system: false,
        is_boot: false,
        snapshot_timestamp: Utc::now().to_rfc3339(),
    };

    let current_snapshot = plan_snapshot.clone();

    let result = compare_target_snapshots(&plan_snapshot, Some(&current_snapshot));
    assert!(result.matches);
    assert!(result.differences.is_empty());
    assert!(verify_snapshot_integrity(&plan_snapshot, Some(&current_snapshot)).is_ok());
}

#[test]
fn test_08_target_snapshot_mismatch_toctou() {
    let plan_snapshot = TargetSnapshot {
        target_identifier: "\\\\.\\PhysicalDrive1".to_string(),
        target_type: TargetType::PhysicalDevice,
        capacity_bytes: Some(32_000_000_000),
        filesystem: None,
        device_id: Some("\\\\.\\PhysicalDrive1".to_string()),
        classification: Some(DeviceClassification::RemovableDevice),
        is_system: false,
        is_boot: false,
        snapshot_timestamp: Utc::now().to_rfc3339(),
    };

    // Mutate capacity (e.g. drive replaced with 64GB drive)
    let mut modified = plan_snapshot.clone();
    modified.capacity_bytes = Some(64_000_000_000);

    let result = compare_target_snapshots(&plan_snapshot, Some(&modified));
    assert!(!result.matches);
    assert!(result.reason.unwrap().contains("TargetChanged"));
    assert!(!result.differences.is_empty());

    let err = verify_snapshot_integrity(&plan_snapshot, Some(&modified));
    assert_eq!(err, Err(ReasonCode::TargetChanged));

    // Device disconnected
    let disconnected_result = compare_target_snapshots(&plan_snapshot, None);
    assert!(!disconnected_result.matches);
    assert_eq!(
        verify_snapshot_integrity(&plan_snapshot, None),
        Err(ReasonCode::InvalidTarget)
    );
}

#[test]
fn test_09_system_boot_target_remains_blocked() {
    let (sanitization, _, _, _) = setup_test_env();

    // PhysicalDrive0 is a system disk
    let sys_target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive0".to_string(),
        display_name: "NVMe Samsung System".to_string(),
        size_bytes: Some(1_000_204_886_016),
    };

    let plan = sanitization
        .evaluate_plan(
            &sys_target,
            SanitizationScope::PhysicalDevice,
            None,
            None,
            Some("root_admin".to_string()),
        )
        .expect("evaluation should return rejected plan");

    assert!(!plan.is_applicable, "System device must be hard-blocked");
    assert_eq!(plan.risk_level, RiskLevel::Critical);
    assert!(plan.reason_codes.contains(&ReasonCode::SystemDevice));

    // C: volume is a system volume
    let vol_target = TargetIdentity {
        target_type: TargetType::LogicalVolume,
        identifier: "C:\\".to_string(),
        display_name: "Windows-OS".to_string(),
        size_bytes: Some(900_000_000_000),
    };

    let vol_plan = sanitization
        .evaluate_plan(
            &vol_target,
            SanitizationScope::LogicalVolume,
            None,
            None,
            Some("root_admin".to_string()),
        )
        .expect("evaluation should return rejected plan");

    assert!(!vol_plan.is_applicable);
    assert!(vol_plan.reason_codes.contains(&ReasonCode::SystemDevice));
}

#[test]
fn test_10_safety_authorization_remains_separate_from_classification() {
    let (sanitization, _, _, safety) = setup_test_env();

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive1".to_string(),
        display_name: "Removable USB".to_string(),
        size_bytes: Some(32_000_000_000),
    };

    // 1. SanitizationPlanner confirms technical method applicability
    let plan = sanitization
        .evaluate_plan(
            &target,
            SanitizationScope::PhysicalDevice,
            None,
            None,
            Some("operator".to_string()),
        )
        .unwrap();
    assert!(plan.is_applicable);

    // 2. But SafetyEngine authorization still denies/blocks without explicit token and confirmation
    let safety_decision = safety
        .evaluate_safety(&target, locardx_common::OperationType::DriveErasure, None)
        .unwrap();

    assert_eq!(
        safety_decision.decision,
        locardx_security::SafetyDecisionOutcome::Denied
    );
    assert_eq!(
        safety_decision.reason_code,
        locardx_security::ReasonCode::UnauthorizedRole
    );
}

#[test]
fn test_11_verification_result_states() {
    let states = vec![
        VerificationOutcome::Verified,
        VerificationOutcome::VerificationFailed,
        VerificationOutcome::PartiallyVerified,
        VerificationOutcome::UnableToVerify,
        VerificationOutcome::NotApplicable,
    ];

    for s in states {
        let display = format!("{}", s);
        assert!(!display.is_empty());
        let json = serde_json::to_string(&s).unwrap();
        let parsed: VerificationOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(s, parsed);
    }
}

#[test]
fn test_12_cancellation_and_failure_states() {
    let reasons = vec![
        FailureReason::Cancelled,
        FailureReason::DeviceDisconnected,
        FailureReason::TargetChanged,
        FailureReason::PermissionDenied,
        FailureReason::IoFailure,
        FailureReason::UnsupportedMedia,
        FailureReason::UnsupportedMethod,
        FailureReason::VerificationFailed,
        FailureReason::Unexpected("test".to_string()),
    ];

    for r in reasons {
        let msg = format!("{}", r);
        assert!(!msg.is_empty());
        let json = serde_json::to_string(&r).unwrap();
        let parsed: FailureReason = serde_json::from_str(&json).unwrap();
        assert_eq!(r, parsed);
    }
}

#[test]
fn test_13_audit_event_generation_and_hash_chain() {
    let (sanitization, _, audit, _) = setup_test_env();

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive1".to_string(),
        display_name: "Removable USB".to_string(),
        size_bytes: Some(32_000_000_000),
    };

    let plan = sanitization
        .evaluate_plan(
            &target,
            SanitizationScope::PhysicalDevice,
            None,
            None,
            Some("auditor".to_string()),
        )
        .unwrap();

    let _evidence = sanitization
        .record_pre_erasure_evidence("op-audit-123", &plan, None, Some("auditor".to_string()))
        .unwrap();

    let events = audit.list_events(20).unwrap();
    let event_types: Vec<String> = events.iter().map(|e| e.event_type.clone()).collect();

    assert!(event_types.contains(&"SANITIZATION_PLAN_REQUESTED".to_string()));
    assert!(event_types.contains(&"SANITIZATION_PLAN_CREATED".to_string()));
    assert!(event_types.contains(&"SANITIZATION_VERIFICATION_PLANNED".to_string()));
    assert!(event_types.contains(&"PRE_ERASURE_EVIDENCE_RECORDED".to_string()));

    // Verify hash chain validity across all generated events
    let chain_status = audit.verify_chain().unwrap();
    assert!(
        chain_status.is_valid,
        "Audit hash chain must remain intact: {}",
        chain_status.details
    );
}

#[test]
fn test_14_database_persistence_of_plans_and_evidence() {
    let (sanitization, db, _, _) = setup_test_env();

    let target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: "\\\\.\\PhysicalDrive2".to_string(),
        display_name: "Seagate 2TB HDD".to_string(),
        size_bytes: Some(2_000_398_934_016),
    };

    let plan = sanitization
        .evaluate_plan(
            &target,
            SanitizationScope::PhysicalDevice,
            None,
            None,
            Some("db_user".to_string()),
        )
        .unwrap();

    // Query plan back from database
    let fetched_plan = sanitization.get_plan(&plan.plan_id).unwrap();
    assert_eq!(fetched_plan.plan_id, plan.plan_id);
    assert_eq!(fetched_plan.target.identifier, "\\\\.\\PhysicalDrive2");
    assert_eq!(fetched_plan.media_type, MediaType::Hdd);

    // Record pre-erasure evidence
    let evidence = sanitization
        .record_pre_erasure_evidence(
            "op-db-456",
            &plan,
            Some("eval-789".to_string()),
            Some("db_user".to_string()),
        )
        .unwrap();

    // Verify database row in pre_erasure_evidence
    let count: i64 = db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM pre_erasure_evidence WHERE evidence_id = ?1",
                rusqlite::params![evidence.evidence_id],
                |row| row.get(0),
            )
        })
        .unwrap();

    assert_eq!(count, 1, "Pre-erasure evidence row must exist in database");
}
