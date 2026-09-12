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

#[test]
fn test_case_forensic_report_generation_and_integrity() {
    use locardx_reporting::forensic_report::*;
    use std::collections::HashMap;

    let db = Arc::new(Database::open(":memory:").unwrap());
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let reporting = ReportingService::new(Arc::clone(&db), Arc::clone(&audit));

    let case_info = CaseReportInfo {
        case_id: "case-e2e-001".to_string(),
        case_reference: "INV-2026-X".to_string(),
        title: "Intrusion Analysis Target Alpha".to_string(),
        description: "Comprehensive forensic examination of suspect machine".to_string(),
        status: "Completed".to_string(),
        lead_investigator: "lead_examiner".to_string(),
        created_at: "2026-09-12T01:00:00Z".to_string(),
        closed_at: Some("2026-09-12T05:00:00Z".to_string()),
    };

    // Insert case record into DB to satisfy foreign key constraint
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO cases (case_id, case_reference, title, description, status, lead_investigator, created_at, updated_at, closed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                case_info.case_id,
                case_info.case_reference,
                case_info.title,
                case_info.description,
                case_info.status,
                case_info.lead_investigator,
                case_info.created_at,
                case_info.created_at,
                case_info.closed_at,
            ],
        )?;
        Ok(())
    }).unwrap();

    let acquisitions = vec![CaseAcquisitionReportInfo {
        acquisition_id: "acq-1".to_string(),
        operation_id: "op-acq-1".to_string(),
        source_device_id: r"\\.\PhysicalDrive1".to_string(),
        source_display_name: "WD Black 2TB".to_string(),
        source_serial: Some("WD-1234".to_string()),
        source_capacity_bytes: 2_000_398_934_016,
        destination_path: "/evidence/disk1.raw".to_string(),
        image_format: "Raw/DD".to_string(),
        image_size_bytes: 2_000_398_934_016,
        image_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            .to_string(),
        status: "Completed".to_string(),
        started_at: "2026-09-12T01:10:00Z".to_string(),
        completed_at: "2026-09-12T02:00:00Z".to_string(),
        audit_reference: "HASH_CHAIN_ACQ_1".to_string(),
    }];

    let mut category_counts = HashMap::new();
    category_counts.insert("Documents".to_string(), 42);
    category_counts.insert("Images".to_string(), 128);

    let mut confidence_distribution = HashMap::new();
    confidence_distribution.insert("High".to_string(), 150);
    confidence_distribution.insert("Medium".to_string(), 20);

    let recoveries = vec![CaseRecoveryReportInfo {
        job_id: "rec-job-1".to_string(),
        operation_id: "op-rec-1".to_string(),
        acquisition_id: "acq-1".to_string(),
        source_image_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            .to_string(),
        recovery_mode: "All".to_string(),
        status: "Completed".to_string(),
        files_recovered: 170,
        candidates_evaluated: 210,
        elapsed_seconds: 45.3,
        category_counts,
        confidence_distribution,
        sample_files: vec![RecoveredFileSnippet {
            file_id: "f-1".to_string(),
            filename: "carved_doc_001.pdf".to_string(),
            file_type: "PDF".to_string(),
            size_bytes: 1048576,
            confidence_score: 95,
            confidence_grade: "High".to_string(),
            sha256_hash: "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234"
                .to_string(),
            recovery_method: "DeepCarving".to_string(),
        }],
        audit_reference: "HASH_CHAIN_REC_1".to_string(),
    }];

    let erasures = vec![CaseErasureReportInfo {
        operation_id: "op-era-1".to_string(),
        plan_id: "plan-era-1".to_string(),
        physical_device_id: r"\\.\PhysicalDrive3".to_string(),
        display_name: "SanDisk Cruzer 32GB".to_string(),
        serial_number: Some("SD-9988".to_string()),
        method: "NistClearSinglePassZeros".to_string(),
        execution_mode: "RealHardware".to_string(),
        status: "Completed".to_string(),
        verification_outcome: "Verified".to_string(),
        verification_strategy: "FullDeviceReadVerification".to_string(),
        evidence_digest: Some("DIGEST_ERA_1".to_string()),
        limitations: vec![],
        audit_reference: "HASH_CHAIN_ERA_1".to_string(),
        completed_at: "2026-09-12T04:30:00Z".to_string(),
    }];

    let custody_timeline = vec![CustodyTimelineEntry {
        custody_id: "cust-1".to_string(),
        evidence_id: Some("ev-1".to_string()),
        timestamp: "2026-09-12T01:05:00Z".to_string(),
        event_type: "EvidenceIntroduced".to_string(),
        actor_id: "lead_examiner".to_string(),
        action: "Introduced physical drive WD Black 2TB".to_string(),
        details: "Bagged and tagged evidence item 001".to_string(),
        audit_hash: Some("AUDIT_HASH_CUST_1".to_string()),
    }];

    let audit_integrity = ReportAuditIntegrity {
        is_valid: true,
        total_events: 24,
        last_verified_sequence: 24,
        audit_root_hash: "ROOT_AUDIT_HASH_99".to_string(),
    };

    let report_data = CaseReportData {
        case_info,
        acquisitions,
        recoveries,
        erasures,
        custody_timeline,
        audit_integrity,
    };

    // 1. Generate report
    let report = reporting
        .generate_case_report(&report_data, Some("lead_examiner"))
        .expect("Case report generation must succeed");

    // 2. Verify all sections populated
    assert_eq!(report.acquisitions.len(), 1);
    assert_eq!(report.acquisitions[0].source_display_name, "WD Black 2TB");
    assert_eq!(report.recoveries.len(), 1);
    assert_eq!(report.recoveries[0].files_recovered, 170);
    assert_eq!(report.erasures.len(), 1);
    assert_eq!(report.erasures[0].verification_outcome, "Verified");
    assert_eq!(report.custody_timeline.len(), 1);
    assert!(report.audit_integrity.is_valid);

    // 3. Digest and integrity verification
    assert!(!report.integrity.report_digest.is_empty());
    assert!(reporting.verify_case_report(&report));

    // 4. Tamper detection
    let mut tampered_report = report.clone();
    tampered_report.case_info.title = "Altered Title for Tamper Test".to_string();
    assert!(
        !reporting.verify_case_report(&tampered_report),
        "Tampered report must fail cryptographic integrity check"
    );

    // 5. Formats
    let md = ForensicReportGenerator::format_markdown_report(&report);
    assert!(md.contains("LocardX Unified Forensic Investigation Report"));
    assert!(md.contains("WD Black 2TB"));
    assert!(md.contains("carved_doc_001.pdf"));
    assert!(md.contains("SanDisk Cruzer 32GB"));
    assert!(md.contains("VERIFIED (Zero tampering detected)"));

    let cert = ForensicReportGenerator::format_text_certificate(&report);
    assert!(cert.contains("LOCARDX FORENSIC INVESTIGATION CERTIFICATE"));
    assert!(cert.contains("CRYPTOGRAPHICALLY VERIFIED"));

    // 6. Retrieval and listing
    let retrieved = reporting
        .get_case_report(&report.report_id)
        .expect("Query must succeed")
        .expect("Report must exist");
    assert_eq!(retrieved.report_id, report.report_id);
    assert_eq!(
        retrieved.integrity.report_digest,
        report.integrity.report_digest
    );
    assert!(reporting.verify_case_report(&retrieved));

    let list = reporting
        .list_case_reports(&report.case_id)
        .expect("List query must succeed");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].report_id, report.report_id);
}
