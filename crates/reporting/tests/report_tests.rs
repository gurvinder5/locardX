use locardx_audit::AuditService;
use locardx_database::Database;
use locardx_device_manager::DeviceType;
use locardx_drive_eraser::models::{
    DriveEraseResult, DriveEraseStatus, DriveSanitizationMethod, DriveVerificationResult,
    DriveVerificationStrategy, ExecutionMode,
};
use locardx_reporting::{ReportingService, SanitizationReportGenerator};
use locardx_verification::sanitization::VerificationOutcome;
use std::sync::Arc;

fn create_sample_result(mode: ExecutionMode, status: DriveEraseStatus) -> DriveEraseResult {
    let outcome = match status {
        DriveEraseStatus::Completed => VerificationOutcome::Verified,
        DriveEraseStatus::VerificationFailed => VerificationOutcome::VerificationFailed,
        _ => VerificationOutcome::UnableToVerify,
    };

    DriveEraseResult {
        operation_id: "op-test-123".to_string(),
        plan_id: "dplan-test-456".to_string(),
        physical_device_id: r"\\.\PhysicalDrive2".to_string(),
        display_name: "Samsung SSD 980 1TB".to_string(),
        vendor: Some("Samsung".to_string()),
        model: Some("SSD 980".to_string()),
        serial_number: Some("S649NX0R123456K".to_string()),
        media_type: DeviceType::Ssd,
        capacity_bytes: 1_000_204_886_016,
        sector_size: 4096,
        method: DriveSanitizationMethod::NvmeCryptoErase,
        execution_mode: mode,
        status,
        bytes_processed: 1_000_204_886_016,
        elapsed_seconds: 4.25,
        verification: DriveVerificationResult {
            outcome,
            strategy: DriveVerificationStrategy::CryptoKeyDestructionCheck,
            details: "Cryptographic erase key destruction verified via NVMe log page 0x15"
                .to_string(),
            verified_at: "2026-09-12T05:00:00Z".to_string(),
        },
        failure_reason: None,
        audit_references: vec![
            "TEST_AUDIT_HASH_1".to_string(),
            "TEST_AUDIT_HASH_2".to_string(),
        ],
        started_at: "2026-09-12T04:59:55Z".to_string(),
        completed_at: "2026-09-12T05:00:00Z".to_string(),
        limitations: vec!["Wear-leveling reserve blocks sanitized via firmware".to_string()],
    }
}

#[test]
fn test_simulation_report_generation_and_warning() {
    let result = create_sample_result(ExecutionMode::Simulation, DriveEraseStatus::Completed);
    let report = SanitizationReportGenerator::generate_from_result(
        &result,
        Some("AUDIT_CHAIN_ROOT".to_string()),
        Some("forensic_admin".to_string()),
    );

    assert!(report.is_simulation());
    assert!(!report.is_verified_physical_erasure());
    assert_eq!(report.operation_info.execution_mode, "Simulation");
    assert_eq!(report.device_info.physical_device_id, r"\\.\PhysicalDrive2");
    assert_eq!(report.device_info.capacity_bytes, 1_000_204_886_016);

    let text_cert = SanitizationReportGenerator::format_text_certificate(&report);
    assert!(text_cert.contains("SIMULATION CERTIFICATE"));
    assert!(text_cert.contains("NO PHYSICAL STORAGE MEDIA WAS ACCESSED"));

    let md_report = SanitizationReportGenerator::format_markdown_report(&report);
    assert!(md_report.contains("SIMULATION REPORT"));
}

#[test]
fn test_real_hardware_report_generation() {
    let result = create_sample_result(ExecutionMode::RealHardware, DriveEraseStatus::Completed);
    let report = SanitizationReportGenerator::generate_from_result(
        &result,
        Some("AUDIT_CHAIN_ROOT_REAL".to_string()),
        Some("lead_investigator".to_string()),
    );

    assert!(!report.is_simulation());
    assert!(report.is_verified_physical_erasure());
    assert_eq!(report.operation_info.execution_mode, "RealHardware");

    let text_cert = SanitizationReportGenerator::format_text_certificate(&report);
    assert!(text_cert.contains("REAL HARDWARE PHYSICAL DRIVE SANITIZATION CERTIFICATE"));
    assert!(text_cert.contains("IRREVERSIBLE PHYSICAL SANITIZATION"));

    let md_report = SanitizationReportGenerator::format_markdown_report(&report);
    assert!(md_report.contains("VERIFIED HARDWARE SANITIZATION"));
}

#[test]
fn test_report_integrity_verification_and_tamper_detection() {
    let result = create_sample_result(ExecutionMode::RealHardware, DriveEraseStatus::Completed);
    let mut report = SanitizationReportGenerator::generate_from_result(
        &result,
        Some("AUDIT_REF_123".to_string()),
        Some("admin".to_string()),
    );

    // Initial generated report must verify
    assert!(SanitizationReportGenerator::verify_report_integrity(
        &report
    ));

    // Tampering with device capacity must cause integrity check to fail
    report.device_info.capacity_bytes = 500_000_000_000;
    assert!(!SanitizationReportGenerator::verify_report_integrity(
        &report
    ));

    // Restore capacity, tamper with serial number
    report.device_info.capacity_bytes = 1_000_204_886_016;
    report.device_info.serial_number = Some("TAMPERED_SERIAL".to_string());
    assert!(!SanitizationReportGenerator::verify_report_integrity(
        &report
    ));

    // Restore serial, tamper with status
    report.device_info.serial_number = Some("S649NX0R123456K".to_string());
    report.execution_metrics.status = "Failed".to_string();
    assert!(!SanitizationReportGenerator::verify_report_integrity(
        &report
    ));
}

#[test]
fn test_reporting_service_persistence_and_retrieval() {
    let db = Arc::new(Database::open(":memory:").expect("Failed to open in-memory SQLite"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let reporting = ReportingService::new(Arc::clone(&db), Arc::clone(&audit));

    // Create sanitization_reports table in SQLite if not yet migrated in 009
    db.with_conn(|conn| {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sanitization_reports (
                report_id              TEXT PRIMARY KEY,
                operation_id           TEXT NOT NULL,
                plan_id                TEXT NOT NULL,
                actor_id               TEXT,
                target_identifier      TEXT NOT NULL,
                execution_mode         TEXT NOT NULL,
                is_simulation          INTEGER NOT NULL,
                status                 TEXT NOT NULL,
                verification_outcome   TEXT NOT NULL,
                report_digest          TEXT NOT NULL,
                audit_chain_reference  TEXT NOT NULL,
                report_json            TEXT NOT NULL,
                generated_at           TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    })
    .unwrap();

    let result = create_sample_result(ExecutionMode::Simulation, DriveEraseStatus::Completed);
    let report = reporting
        .generate_drive_erasure_report(&result, Some("test_operator"))
        .expect("Report generation must succeed");

    assert!(reporting.verify_report(&report));

    // Retrieve from DB
    let retrieved = reporting
        .get_report_by_operation("op-test-123")
        .expect("Query must succeed")
        .expect("Report must exist");

    assert_eq!(retrieved.report_id, report.report_id);
    assert_eq!(retrieved.operation_id, report.operation_id);
    assert_eq!(
        retrieved.integrity.report_digest,
        report.integrity.report_digest
    );
    assert!(reporting.verify_report(&retrieved));
}
