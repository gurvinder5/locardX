//! Complete End-to-End Module Acceptance Test for LocardX Secure Drive Eraser.
//!
//! Satisfies LocardX Step 10 Requirement 27 (FINAL MODULE ACCEPTANCE TEST):
//! Covers the full lifecycle from discovery through report verification:
//! DEVICE DISCOVERY
//!         ↓
//! CAPABILITY ASSESSMENT
//!         ↓
//! TARGET VALIDATION
//!         ↓
//! PLAN GENERATION
//!         ↓
//! TWO-STAGE AUTHORIZATION
//!         ↓
//! TOCTOU REVALIDATION
//!         ↓
//! EXCLUSIVE LOCK
//!         ↓
//! REAL/SIMULATION EXECUTION
//!         ↓
//! VERIFICATION
//!         ↓
//! AUDIT
//!         ↓
//! DATABASE PERSISTENCE
//!         ↓
//! REPORT GENERATION
//!         ↓
//! FINAL RESULT

use locardx_audit::AuditService;
use locardx_auth::AuthService;
use locardx_database::Database;
use locardx_device_manager::{DeviceClassification, DeviceDiscoveryProvider, DeviceManagerService};
use locardx_drive_eraser::mock_devices::MockDeviceRegistry;
use locardx_drive_eraser::models::{
    DriveEraseProgress, DriveEraseRequest, DriveEraseStatus, ExecutionMode,
};
use locardx_drive_eraser::platform::test_executor::FaultInjectableExecutor;
use locardx_drive_eraser::service::DriveEraserService;
use locardx_reporting::{ReportingService, SanitizationReportGenerator};
use locardx_security::SafetyEngine;
use locardx_verification::sanitization::VerificationOutcome;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn test_full_end_to_end_drive_erasure_acceptance() {
    // ==========================================
    // 0. Environment Setup & Core Invariants
    // ==========================================
    let db = Arc::new(Database::open(":memory:").expect("Failed to initialize in-memory SQLite"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let auth = Arc::new(AuthService::new(Arc::clone(&db), Arc::clone(&audit)));
    let mock_registry = MockDeviceRegistry::new_standard_test_set();
    let provider: Arc<dyn DeviceDiscoveryProvider> = Arc::new(mock_registry.clone());
    let device_manager = Arc::new(DeviceManagerService::new(Arc::clone(&provider)));
    let safety = Arc::new(SafetyEngine::new(
        Arc::clone(&db),
        Arc::clone(&audit),
        auth,
        device_manager,
    ));
    let reporting = Arc::new(ReportingService::new(Arc::clone(&db), Arc::clone(&audit)));

    // Configure test fault-injectable executor (non-destructive pipeline validation)
    let test_executor = Arc::new(FaultInjectableExecutor::normal());

    let service = Arc::new(
        DriveEraserService::new(
            Arc::clone(&db),
            Arc::clone(&audit),
            Arc::clone(&safety),
            provider,
        )
        .with_execution_gate(Arc::new(
            locardx_drive_eraser::hardware::RealHardwareExecutionGate::with_enabled(true),
        ))
        .with_executor(test_executor),
    );

    // ==========================================
    // 1. Device Discovery
    // ==========================================
    let devices = service
        .device_provider()
        .discover_devices()
        .expect("Device discovery must succeed");
    assert!(!devices.is_empty(), "Must discover mock devices");

    // Select the target secondary test drive (non-system, non-boot: PhysicalDrive1)
    let target_dev = devices
        .iter()
        .find(|d| d.device_id == r"\\.\PhysicalDrive1")
        .expect("Target PhysicalDrive1 must exist");

    assert_eq!(target_dev.device_id, r"\\.\PhysicalDrive1");
    assert_eq!(
        target_dev.classification,
        DeviceClassification::FixedDataDevice
    );
    assert!(
        !target_dev.is_system_device,
        "Target must NOT be system device"
    );

    // Verify system disk (PhysicalDrive0) is hard blocked from plan generation
    let system_dev = devices
        .iter()
        .find(|d| d.is_system_device)
        .expect("System device must exist in mock test set");
    let system_plan_res = service
        .plan_drive_erasure(
            DriveEraseRequest {
                target_device_id: system_dev.device_id.clone(),
                requested_method: None,
                execution_mode: Some(ExecutionMode::RealHardware),
                session_token: Some("session-admin-001".to_string()),
            },
            Some("lead_examiner".to_string()),
        )
        .await;
    assert!(
        system_plan_res.is_err(),
        "System disk erasure must fail closed"
    );

    // ==========================================
    // 2. Capability Assessment
    // ==========================================
    let assessment = service
        .assess_drive_capabilities(&target_dev.device_id)
        .await
        .expect("Capability assessment must succeed");

    assert!(assessment.overall_state.is_supported());
    assert!(assessment.capabilities.supports_overwrite);

    // ==========================================
    // 3. Target Validation & Plan Generation
    // ==========================================
    let plan_req = DriveEraseRequest {
        target_device_id: target_dev.device_id.clone(),
        requested_method: None,
        execution_mode: Some(ExecutionMode::RealHardware),
        session_token: Some("session-admin-001".to_string()),
    };

    let plan = service
        .plan_drive_erasure(plan_req, Some("lead_examiner".to_string()))
        .await
        .expect("Plan generation must succeed");

    assert_eq!(plan.physical_device_id, r"\\.\PhysicalDrive1");
    assert_eq!(plan.execution_mode, ExecutionMode::RealHardware);
    assert_eq!(plan.capacity_bytes, target_dev.capacity_bytes);

    // ==========================================
    // 4. Two-Stage Safety Authorization Challenge & TOCTOU
    // ==========================================
    let op_id = format!("op-acceptance-{}", uuid::Uuid::new_v4());
    let confirmation_challenge = "conf-challenge-789";

    // Rejection of invalid confirmation challenge
    let invalid_confirm_res = service
        .execute_drive_erasure_with_gate(
            &plan.plan_id,
            confirmation_challenge,
            &op_id,
            "WRONG_DEVICE_ID",
            true,
            "session-admin-001",
            ExecutionMode::RealHardware,
            None,
            None,
        )
        .await;
    assert!(
        invalid_confirm_res.is_err(),
        "Mismatched challenge confirmation must fail closed"
    );

    // ==========================================
    // 5. TOCTOU Validation, Lock Acquisition, Execution & Verification
    // ==========================================
    let progress_events = Arc::new(AtomicUsize::new(0));
    let progress_counter = Arc::clone(&progress_events);

    let on_progress = move |_p: DriveEraseProgress| {
        progress_counter.fetch_add(1, Ordering::SeqCst);
    };

    let result = service
        .execute_drive_erasure_with_gate(
            &plan.plan_id,
            confirmation_challenge,
            &op_id,
            &target_dev.device_id,
            true, // Warning acknowledged
            "session-admin-001",
            ExecutionMode::RealHardware,
            None,
            Some(&on_progress),
        )
        .await
        .expect("Execution and verification must succeed");

    // ==========================================
    // 6. Verification Outcome Verification
    // ==========================================
    assert_eq!(result.status, DriveEraseStatus::Completed);
    assert_eq!(result.verification.outcome, VerificationOutcome::Verified);
    assert_eq!(result.execution_mode, ExecutionMode::RealHardware);
    assert_eq!(result.bytes_processed, target_dev.capacity_bytes);
    assert!(
        progress_events.load(Ordering::SeqCst) > 0,
        "Telemetry must be emitted"
    );

    // ==========================================
    // 7. Cryptographic Audit Chain Integrity
    // ==========================================
    let audit_status = audit
        .verify_chain()
        .expect("Audit verification must succeed");
    assert!(
        audit_status.is_valid,
        "Tamper-evident audit chain must be verified intact"
    );

    // ==========================================
    // 8. Database Persistence Verification
    // ==========================================
    let db_record = service
        .get_drive_erasure_result(&op_id)
        .expect("Database query must succeed")
        .expect("Drive erasure record must exist in SQLite");

    assert_eq!(db_record.operation_id, op_id);
    assert_eq!(db_record.plan_id, plan.plan_id);
    assert_eq!(db_record.physical_device_id, target_dev.device_id);
    assert_eq!(db_record.status, DriveEraseStatus::Completed);
    assert_eq!(
        db_record.verification.outcome,
        VerificationOutcome::Verified
    );

    // ==========================================
    // 9. Reporting Service Certificate Generation & Integrity
    // ==========================================
    let report = reporting
        .generate_drive_erasure_report(&result, Some("lead_examiner"))
        .expect("Report generation must succeed");

    assert_eq!(report.operation_id, op_id);
    assert_eq!(report.plan_id, plan.plan_id);
    assert!(
        !report.is_simulation(),
        "Report must reflect Real Hardware mode"
    );
    assert!(report.is_verified_physical_erasure());

    // Verify cryptographic report integrity
    assert!(
        reporting.verify_report(&report),
        "Report digest must verify against calculated SHA-256"
    );

    // Verify certificate formatting clearly warns about real hardware
    let cert_text = SanitizationReportGenerator::format_text_certificate(&report);
    assert!(cert_text.contains("REAL HARDWARE PHYSICAL DRIVE SANITIZATION CERTIFICATE"));
    assert!(cert_text.contains("IRREVERSIBLE PHYSICAL SANITIZATION"));
    assert!(!cert_text.contains("SIMULATION CERTIFICATE"));

    // Verify database persistence of report
    let retrieved_report = reporting
        .get_report_by_operation(&op_id)
        .expect("Report query must succeed")
        .expect("Report must exist in SQLite");

    assert_eq!(retrieved_report.report_id, report.report_id);
    assert_eq!(
        retrieved_report.integrity.report_digest,
        report.integrity.report_digest
    );
    assert!(reporting.verify_report(&retrieved_report));
}
