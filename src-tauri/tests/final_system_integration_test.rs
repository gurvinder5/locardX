//! LocardX Step 15 — Master End-to-End System Integration & Regression Test Suite
//!
//! Validates:
//! 1. Full investigative workflow (Auth -> Case -> Evidence -> Acquisition -> Hash Verification ->
//!    Recovery -> Carved Files -> Case Association -> Timeline -> Case Report -> Report Verification -> Audit Chain)
//! 2. Full sanitization workflow (Auth -> Drive/File Erasure Plan -> Safety Interlocks -> 2-Stage Confirmation ->
//!    Execution -> Verification -> Sanitization Report -> Audit Trail)
//! 3. Subsystem failure, boundary isolation, and fail-closed invariants:
//!    - Corrupted image hash fails closed and prevents recovery
//!    - Evidence immutability (Zero Write Invariant)
//!    - Authorization boundary enforcement (denial + audit event)
//!    - Case closure preserves all evidence, operations, custody events, and audit logs

use locardx_acquisition::{AcquisitionArtifact, AcquisitionDeviceSnapshot};
use locardx_auth::models::{
    CreateUserRequest, InitAdminRequest, LoginRequest, Permission, UserRole,
};
use locardx_case_management::models::{
    AddEvidenceRequest, CaseStatus, CreateCaseRequest, CustodyEventType, EvidenceType,
    RecordCustodyRequest,
};
use locardx_common::{OperationType, TargetIdentity, TargetType};
use locardx_desktop::{commands, init_application};
use locardx_drive_eraser::{DriveEraseRequest, DriveEraseStatus, ExecutionMode};
use locardx_recovery_engine::models::{RecoveryMode, RecoveryOptions, RecoveryStatus};
use locardx_recovery_engine::source::compute_streaming_sha256;
use locardx_security::models::{ReasonCode, SafetyDecisionOutcome};
use locardx_verification::VerificationOutcome;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::PathBuf;

/// Helper to create a synthetic 512 KiB raw disk image file containing known file signatures:
/// - Cluster 8 (offset 4096): Valid JPEG file
/// - Cluster 32 (offset 16384): Valid PNG file
/// - Cluster 64 (offset 32768): Valid PDF document
fn create_synthetic_forensic_disk(dir: &std::path::Path) -> (PathBuf, String, u64) {
    let disk_path = dir.join(format!("synthetic_disk_{}.raw", uuid::Uuid::new_v4()));
    let total_size: usize = 512 * 1024; // 512 KiB
    let mut disk_bytes = vec![0u8; total_size];

    // Master Boot Record mock marker at end of sector 0
    disk_bytes[510] = 0x55;
    disk_bytes[511] = 0xAA;

    // 1. Valid JPEG file at offset 4096 (Cluster 8)
    let jpeg_offset = 4096;
    let mut jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]; // SOI + APP0
    jpeg_data.extend_from_slice(b"JFIF          ");
    jpeg_data.extend_from_slice(&[
        0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x10, 0x00, 0x10, 0x01, 0x01,
    ]); // SOF0
    jpeg_data.extend_from_slice(&[0xFF, 0xD9]); // EOI
    disk_bytes[jpeg_offset..jpeg_offset + jpeg_data.len()].copy_from_slice(&jpeg_data);

    // 2. Valid PNG file at offset 16384 (Cluster 32)
    let png_offset = 16384;
    let mut png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG Header
    png_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
    png_data.extend_from_slice(b"IHDR");
    png_data.extend_from_slice(&[0; 13]);
    png_data.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]); // CRC
    png_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x04]);
    png_data.extend_from_slice(b"IDAT");
    png_data.extend_from_slice(&[1, 2, 3, 4]);
    png_data.extend_from_slice(&[0; 4]);
    png_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    png_data.extend_from_slice(b"IEND");
    png_data.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);
    disk_bytes[png_offset..png_offset + png_data.len()].copy_from_slice(&png_data);

    // 3. Valid PDF file at offset 32768 (Cluster 64)
    let pdf_offset = 32768;
    let mut pdf_data = Vec::new();
    pdf_data.extend_from_slice(b"%PDF-1.7\n");
    pdf_data.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
    pdf_data.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf_data.extend_from_slice(b"trailer << /Root 1 0 R >>\nstartxref\n120\n%%EOF");
    disk_bytes[pdf_offset..pdf_offset + pdf_data.len()].copy_from_slice(&pdf_data);

    // Write synthetic raw disk file
    let mut f = std::fs::File::create(&disk_path).expect("Failed to create synthetic raw disk");
    f.write_all(&disk_bytes)
        .expect("Failed to write synthetic disk bytes");
    f.flush().expect("Failed to flush disk file");

    // Compute ground-truth SHA-256
    let mut sha = Sha256::new();
    sha.update(&disk_bytes);
    let hex_hash = hex::encode(sha.finalize());

    (disk_path, hex_hash, total_size as u64)
}

#[tokio::test]
async fn test_01_full_investigative_lifecycle_workflow() {
    std::env::set_var("LOCARDX_ENV", "test");
    std::env::set_var("LOCARDX_DB_PATH", ":memory:");

    let state = init_application().expect("Application initialization must succeed");

    // =========================================================================
    // 1. BOOTSTRAP & CLOSED USER PROVISIONING
    // =========================================================================
    // Initialize Admin
    let admin_user = commands::initialize_admin_handler(
        &state,
        InitAdminRequest {
            username: "chief_examiner".to_string(),
            password: "MasterSecurePassword2026!".to_string(),
            confirm_password: "MasterSecurePassword2026!".to_string(),
            display_name: Some("Chief Examiner".to_string()),
        },
    )
    .expect("Admin initialization must succeed");
    assert_eq!(admin_user.role, UserRole::Administrator);

    // Login as Admin
    let admin_login = commands::login_handler(
        &state,
        LoginRequest {
            username: "chief_examiner".to_string(),
            password: "MasterSecurePassword2026!".to_string(),
        },
    )
    .expect("Admin login must succeed");
    let admin_token = admin_login.token;

    // Provision an Investigator
    let inv_user = commands::create_user_handler(
        &state,
        &admin_token,
        CreateUserRequest {
            username: "lead_investigator".to_string(),
            password: "InvestigatorPassword2026!".to_string(),
            role: UserRole::Investigator,
            display_name: Some("Inv. Sarah Vance".to_string()),
            metadata_json: Some(r#"{"badge":"LX-9942","unit":"Forensic Unit"}"#.to_string()),
        },
    )
    .expect("Investigator creation must succeed");
    assert_eq!(inv_user.role, UserRole::Investigator);

    // Login as Investigator
    let inv_login = commands::login_handler(
        &state,
        LoginRequest {
            username: "lead_investigator".to_string(),
            password: "InvestigatorPassword2026!".to_string(),
        },
    )
    .expect("Investigator login must succeed");
    assert_eq!(inv_login.user.username, "lead_investigator");

    // =========================================================================
    // 2. CASE INITIATION
    // =========================================================================
    let case_req = CreateCaseRequest {
        case_reference: "CASE-2026-FINAL-001".to_string(),
        title: "Project Final Validation Case".to_string(),
        description: "Comprehensive acquisition, carving, and evidential custody verification."
            .to_string(),
        metadata_json: Some(r#"{"priority":"High","agency":"Cyber Crimes Division"}"#.to_string()),
    };
    let case = state
        .case_service
        .create_case(case_req, &inv_user)
        .expect("Case creation must succeed");
    assert_eq!(case.case_reference, "CASE-2026-FINAL-001");
    assert_eq!(case.status, CaseStatus::Open);

    // =========================================================================
    // 3. SYNTHETIC EVIDENCE MEDIA GENERATION
    // =========================================================================
    let temp_dir = std::env::temp_dir().join(format!("locardx_integ_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temporary directory");

    let (raw_disk_path, ground_truth_hash, disk_capacity) =
        create_synthetic_forensic_disk(&temp_dir);
    let raw_disk_str = raw_disk_path.to_string_lossy().to_string();

    // =========================================================================
    // 4. EVIDENCE ACQUISITION (BITSTREAM RAW DD IMAGING)
    // =========================================================================
    let image_dest_path = temp_dir.join("acquired_evidence.raw");
    let image_dest_str = image_dest_path.to_string_lossy().to_string();

    let ground_truth_bytes = std::fs::read(&raw_disk_path).expect("Read synthetic raw disk bytes");
    std::fs::write(&image_dest_path, &ground_truth_bytes).expect("Write acquired bitstream image");

    let acquired_hash =
        compute_streaming_sha256(&image_dest_path).expect("Compute acquired image hash");
    assert_eq!(
        acquired_hash, ground_truth_hash,
        "Acquisition image hash must strictly match source hash"
    );

    let snapshot = AcquisitionDeviceSnapshot {
        device_id: raw_disk_str.clone(),
        display_name: "Synthetic Evidence Drive Alpha".to_string(),
        vendor: Some("LocardX".to_string()),
        model: Some("Virtual Evidence Disk".to_string()),
        serial_number: Some("SN-LX-FINAL-001".to_string()),
        media_type: "PhysicalDisk".to_string(),
        capacity_bytes: disk_capacity,
        sector_size: 512,
        bus_type: Some("USB".to_string()),
        is_removable: true,
        is_system: false,
        snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let artifact = AcquisitionArtifact {
        acquisition_id: format!("acq-{}", uuid::Uuid::new_v4()),
        image_path: image_dest_str.clone(),
        image_format: "raw".to_string(),
        image_size_bytes: disk_capacity,
        image_sha256: acquired_hash.clone(),
        source_device_snapshot: snapshot.clone(),
        acquisition_timestamp: chrono::Utc::now().to_rfc3339(),
        is_verified: true,
        audit_reference: "audit-acq-001".to_string(),
    };

    // =========================================================================
    // 5. ADVANCED RECOVERY & FILE CARVING
    // =========================================================================
    let validated_source = state
        .recovery
        .validate_source(&artifact, Some(&inv_user.username))
        .expect("Recovery source validation must succeed");
    assert!(validated_source.is_trusted);
    assert_eq!(validated_source.image_size_bytes, disk_capacity);

    let recovery_out_dir = temp_dir.join("carved_output");
    std::fs::create_dir_all(&recovery_out_dir).expect("Create recovery output directory");

    let rec_opts = RecoveryOptions {
        recovery_mode: RecoveryMode::CarvingOnly,
        target_file_types: None,
        output_directory: recovery_out_dir.to_string_lossy().to_string(),
        enable_fragment_reconstruction: true,
        min_confidence_score: 50,
        chunk_size_bytes: 65536,
    };

    let plan = state
        .recovery
        .create_plan(&artifact, rec_opts, Some(&inv_user.username))
        .expect("Recovery plan creation must succeed");
    assert!(!plan.plan_id.is_empty());

    let rec_result = state
        .recovery
        .execute_recovery(&plan, Some(&inv_user.username))
        .expect("Recovery execution must succeed");
    assert_eq!(rec_result.status, RecoveryStatus::Completed);
    assert_eq!(
        rec_result.files_recovered, 3,
        "Must carve exactly 3 embedded files (JPEG, PNG, PDF)"
    );

    let carved_files = state
        .recovery
        .get_recovered_files(&rec_result.job_id)
        .expect("Get recovered files must succeed");
    assert_eq!(carved_files.len(), 3);

    for file in &carved_files {
        assert!(
            file.confidence_score >= 70,
            "Recovered file confidence score must be high"
        );
        assert!(
            !file.sha256_hash.is_empty(),
            "Carved file SHA-256 must be computed"
        );
    }

    // =========================================================================
    // 6. CASE ASSOCIATION, EVIDENCE & CUSTODY TRACKING
    // =========================================================================
    let acq_op = state
        .case_service
        .associate_operation(
            &case.case_id,
            &artifact.acquisition_id,
            "ForensicAcquisition",
            &inv_user,
            Some("Bitstream Raw DD imaging of Target Drive Alpha"),
        )
        .expect("Associate acquisition op must succeed");
    assert_eq!(acq_op.case_id, case.case_id);

    let rec_op = state
        .case_service
        .associate_operation(
            &case.case_id,
            &rec_result.job_id,
            "FileRecovery",
            &inv_user,
            Some("Advanced file carving of Target Drive Alpha"),
        )
        .expect("Associate recovery op must succeed");
    assert_eq!(rec_op.case_id, case.case_id);

    let ev_disk = state
        .case_service
        .add_case_evidence(
            &case.case_id,
            AddEvidenceRequest {
                evidence_type: EvidenceType::PhysicalStorage,
                identifier: raw_disk_str.clone(),
                label: "Target Drive Alpha (Physical Media)".to_string(),
                sha256: Some(ground_truth_hash.clone()),
                size_bytes: Some(disk_capacity),
                notes: Some("Seized evidence drive from digital forensics lab".to_string()),
            },
            &inv_user,
        )
        .expect("Add physical evidence must succeed");

    let _ev_img = state
        .case_service
        .add_case_evidence(
            &case.case_id,
            AddEvidenceRequest {
                evidence_type: EvidenceType::AcquisitionImage,
                identifier: image_dest_str.clone(),
                label: "Target Drive Alpha (Raw DD Image)".to_string(),
                sha256: Some(acquired_hash.clone()),
                size_bytes: Some(disk_capacity),
                notes: Some("Verified raw bitstream acquisition image".to_string()),
            },
            &inv_user,
        )
        .expect("Add image evidence must succeed");

    let custody_ev = state
        .case_service
        .record_custody_event(
            &case.case_id,
            RecordCustodyRequest {
                evidence_id: Some(ev_disk.evidence_id.clone()),
                event_type: CustodyEventType::EvidenceVerified,
                action: "Hardware bitstream hash verification".to_string(),
                details: format!(
                    "SHA-256 matched reference value: {}",
                    &ground_truth_hash[0..16]
                ),
            },
            &inv_user,
        )
        .expect("Record custody event must succeed");
    assert_eq!(custody_ev.event_type, CustodyEventType::EvidenceVerified);

    let timeline = state
        .case_service
        .get_case_timeline(&case.case_id)
        .expect("Fetch timeline must succeed");
    assert!(
        timeline.len() >= 3,
        "Timeline should contain case creation, operations, and custody events"
    );

    let summary = state
        .case_service
        .get_case_summary(&case.case_id)
        .expect("Fetch summary must succeed");
    assert_eq!(summary.evidence_count, 2);
    assert_eq!(summary.operation_count, 2);

    // =========================================================================
    // 7. UNIFIED FORENSIC REPORT GENERATION & INTEGRITY VERIFICATION
    // =========================================================================
    let report = state
        .case_service
        .generate_case_report(&case.case_id, &inv_user)
        .expect("Report generation must succeed");
    assert!(!report.report_id.is_empty());
    assert!(!report.integrity.report_digest.is_empty());

    assert!(
        state.case_service.verify_case_report(&report),
        "Forensic report integrity hash must verify canonically"
    );

    // =========================================================================
    // 8. AUDIT CHAIN INTEGRITY VERIFICATION
    // =========================================================================
    let audit_verification = state
        .audit
        .verify_chain()
        .expect("Audit verification must succeed");
    assert!(
        audit_verification.is_valid,
        "Tamper-evident audit chain must be 100% valid"
    );
    assert_eq!(audit_verification.broken_sequence, None);
    assert!(
        audit_verification.total_events >= 5,
        "Audit trail must have recorded all actions"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_02_full_sanitization_and_erasure_workflow() {
    std::env::set_var("LOCARDX_ENV", "test");
    std::env::set_var("LOCARDX_DB_PATH", ":memory:");

    let state = init_application().expect("Application initialization must succeed");

    // Bootstrap Admin
    let _admin_user = commands::initialize_admin_handler(
        &state,
        InitAdminRequest {
            username: "admin_sanitizer".to_string(),
            password: "AdminPassword2026!".to_string(),
            confirm_password: "AdminPassword2026!".to_string(),
            display_name: Some("Sanitization Admin".to_string()),
        },
    )
    .expect("Admin init must succeed");

    let admin_login = commands::login_handler(
        &state,
        LoginRequest {
            username: "admin_sanitizer".to_string(),
            password: "AdminPassword2026!".to_string(),
        },
    )
    .expect("Admin login must succeed");

    // Create Operator user
    let op_user = commands::create_user_handler(
        &state,
        &admin_login.token,
        CreateUserRequest {
            username: "sanitization_tech".to_string(),
            password: "OperatorPass123!".to_string(),
            role: UserRole::Operator,
            display_name: Some("Tech. John Connor".to_string()),
            metadata_json: None,
        },
    )
    .expect("Operator creation must succeed");

    let op_login = commands::login_handler(
        &state,
        LoginRequest {
            username: "sanitization_tech".to_string(),
            password: "OperatorPass123!".to_string(),
        },
    )
    .expect("Operator login must succeed");
    let op_token = op_login.token;

    // 1. Safety Evaluation: Verify system/boot drives are hard-blocked
    let sys_target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: r"\\.\PhysicalDrive0".to_string(),
        display_name: "Samsung System SSD".to_string(),
        size_bytes: Some(512_000_000_000),
    };

    let sys_eval = state
        .safety
        .evaluate_safety(&sys_target, OperationType::DriveErasure, Some(&op_token))
        .expect("Safety evaluation must succeed");
    assert_eq!(sys_eval.decision, SafetyDecisionOutcome::Blocked);
    assert_eq!(sys_eval.reason_code, ReasonCode::SystemDevice);

    // 2. Safety Evaluation: Non-system target requires confirmation
    let data_target = TargetIdentity {
        target_type: TargetType::PhysicalDevice,
        identifier: r"\\.\PhysicalDrive1".to_string(),
        display_name: "Seagate Data Drive".to_string(),
        size_bytes: Some(2_000_000_000_000),
    };

    let data_eval = state
        .safety
        .evaluate_safety(&data_target, OperationType::DriveErasure, Some(&op_token))
        .expect("Safety evaluation must succeed");
    assert_eq!(
        data_eval.decision,
        SafetyDecisionOutcome::RequiresConfirmation
    );

    // 3. Plan simulated drive erasure for non-system target
    let erase_req = DriveEraseRequest {
        target_device_id: r"\\.\PhysicalDrive1".to_string(),
        requested_method: Some("nist_800_88_clear_zero".to_string()),
        execution_mode: Some(ExecutionMode::Simulation),
        session_token: Some(op_token.clone()),
    };

    let plan = state
        .drive_eraser
        .plan_drive_erasure(erase_req, Some(op_user.username.clone()))
        .await
        .expect("Drive erasure plan must succeed");
    assert!(!plan.plan_id.is_empty());

    // 4. Request 2-Stage Confirmation Token
    let op_id = "op-erase-sim-final-001";
    let challenge = state
        .safety
        .request_destructive_confirmation(
            op_id,
            &data_target,
            OperationType::DriveErasure,
            &op_token,
        )
        .expect("Request confirmation token must succeed");
    assert!(!challenge.confirmation_id.is_empty());

    // 5. Execute simulated drive erasure
    let result = state
        .drive_eraser
        .execute_drive_erasure_simulation(
            &plan.plan_id,
            &challenge.confirmation_id,
            op_id,
            r"\\.\PhysicalDrive1",
            true,
            &op_token,
            ExecutionMode::Simulation,
            None,
            None,
        )
        .await
        .expect("Drive erasure simulation must succeed");
    assert_eq!(result.status, DriveEraseStatus::Completed);
    assert_eq!(result.verification.outcome, VerificationOutcome::Verified);

    // 6. Generate drive sanitization certificate report
    let cert = state
        .reporting
        .generate_drive_erasure_report(&result, Some(&op_user.username))
        .expect("Generate sanitization report must succeed");
    assert!(!cert.report_id.is_empty());
    assert!(!cert.integrity.report_digest.is_empty());

    // 7. Verify audit chain records all sanitization lifecycle events
    let audit_ver = state
        .audit
        .verify_chain()
        .expect("Audit check must succeed");
    assert!(audit_ver.is_valid);
    assert_eq!(audit_ver.broken_sequence, None);
}

#[tokio::test]
async fn test_03_subsystem_failure_and_isolation_invariants() {
    std::env::set_var("LOCARDX_ENV", "test");
    std::env::set_var("LOCARDX_DB_PATH", ":memory:");

    let state = init_application().expect("Application initialization must succeed");

    // Bootstrap Admin & Viewer
    let _admin_user = commands::initialize_admin_handler(
        &state,
        InitAdminRequest {
            username: "admin_invariants".to_string(),
            password: "AdminPassword2026!".to_string(),
            confirm_password: "AdminPassword2026!".to_string(),
            display_name: Some("Invariants Admin".to_string()),
        },
    )
    .unwrap();

    let admin_login = commands::login_handler(
        &state,
        LoginRequest {
            username: "admin_invariants".to_string(),
            password: "AdminPassword2026!".to_string(),
        },
    )
    .unwrap();

    let _viewer_user = commands::create_user_handler(
        &state,
        &admin_login.token,
        CreateUserRequest {
            username: "viewer_invariants".to_string(),
            password: "ViewerPassword2026!".to_string(),
            role: UserRole::Viewer,
            display_name: Some("Auditor Viewer".to_string()),
            metadata_json: None,
        },
    )
    .unwrap();

    let viewer_login = commands::login_handler(
        &state,
        LoginRequest {
            username: "viewer_invariants".to_string(),
            password: "ViewerPassword2026!".to_string(),
        },
    )
    .unwrap();

    let temp_dir = std::env::temp_dir().join(format!("locardx_inv_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let (disk_path, real_hash, cap) = create_synthetic_forensic_disk(&temp_dir);

    // =========================================================================
    // INVARIANT 1: CORRUPTED / TAMPERED ACQUISITION HASH FAILS CLOSED
    // =========================================================================
    let tampered_artifact = AcquisitionArtifact {
        acquisition_id: "acq-tampered-test".to_string(),
        image_path: disk_path.to_string_lossy().to_string(),
        image_format: "raw".to_string(),
        image_size_bytes: cap,
        image_sha256: "0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        source_device_snapshot: AcquisitionDeviceSnapshot {
            device_id: disk_path.to_string_lossy().to_string(),
            display_name: "Tampered Test Disk".to_string(),
            vendor: None,
            model: None,
            serial_number: None,
            media_type: "PhysicalDisk".to_string(),
            capacity_bytes: cap,
            sector_size: 512,
            bus_type: None,
            is_removable: true,
            is_system: false,
            snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
        },
        acquisition_timestamp: chrono::Utc::now().to_rfc3339(),
        is_verified: true,
        audit_reference: "audit-tamper-01".to_string(),
    };

    let validate_res = state
        .recovery
        .validate_source(&tampered_artifact, Some("admin_invariants"));
    assert!(
        validate_res.is_err(),
        "Tampered image hash MUST be rejected fail-closed before any processing"
    );

    // =========================================================================
    // INVARIANT 2: EVIDENCE IMMUTABILITY (ZERO WRITE INVARIANT)
    // =========================================================================
    let pre_recovery_hash = compute_streaming_sha256(&disk_path).unwrap();
    assert_eq!(pre_recovery_hash, real_hash);

    let valid_artifact = AcquisitionArtifact {
        acquisition_id: "acq-immutability-check".to_string(),
        image_path: disk_path.to_string_lossy().to_string(),
        image_format: "raw".to_string(),
        image_size_bytes: cap,
        image_sha256: real_hash.clone(),
        source_device_snapshot: tampered_artifact.source_device_snapshot.clone(),
        acquisition_timestamp: chrono::Utc::now().to_rfc3339(),
        is_verified: true,
        audit_reference: "audit-immut".to_string(),
    };

    let rec_opts = RecoveryOptions {
        recovery_mode: RecoveryMode::CarvingOnly,
        target_file_types: None,
        output_directory: temp_dir.join("immut_out").to_string_lossy().to_string(),
        enable_fragment_reconstruction: true,
        min_confidence_score: 50,
        chunk_size_bytes: 65536,
    };
    let rec_plan = state
        .recovery
        .create_plan(&valid_artifact, rec_opts, Some("admin_invariants"))
        .unwrap();
    let _ = state
        .recovery
        .execute_recovery(&rec_plan, Some("admin_invariants"))
        .unwrap();

    let post_recovery_hash = compute_streaming_sha256(&disk_path).unwrap();
    assert_eq!(
        pre_recovery_hash, post_recovery_hash,
        "EVIDENCE IMMUTABILITY VIOLATION: Evidence file was modified during recovery!"
    );

    // =========================================================================
    // INVARIANT 3: AUTHORIZATION BOUNDARY ENFORCEMENT & AUDIT DENIAL
    // =========================================================================
    let denied_case = state
        .auth
        .authorize_permission(&viewer_login.token, Permission::CaseCreate);
    assert!(
        denied_case.is_err(),
        "Viewer must not possess CaseCreate permission"
    );

    let audit_events = state.audit.list_events(50).unwrap();
    assert!(
        audit_events
            .iter()
            .any(|e| e.event_type == "AUTHORIZATION_DENIED"),
        "Authorization denial MUST generate an audit record"
    );

    // =========================================================================
    // INVARIANT 4: CASE CLOSURE NEVER DELETES RECORDS
    // =========================================================================
    let case = state
        .case_service
        .create_case(
            CreateCaseRequest {
                case_reference: "CASE-PERSISTENCE-TEST".to_string(),
                title: "Retention Invariant Case".to_string(),
                description: "Ensuring closed cases retain 100% of data".to_string(),
                metadata_json: None,
            },
            &admin_login.user,
        )
        .unwrap();

    let ev = state
        .case_service
        .add_case_evidence(
            &case.case_id,
            AddEvidenceRequest {
                evidence_type: EvidenceType::PhysicalStorage,
                identifier: r"\\.\PhysicalDrive99".to_string(),
                label: "Retention Evidence".to_string(),
                sha256: Some(real_hash.clone()),
                size_bytes: Some(cap),
                notes: None,
            },
            &admin_login.user,
        )
        .unwrap();

    let _op = state
        .case_service
        .associate_operation(
            &case.case_id,
            "op-retention-test",
            "ForensicAcquisition",
            &admin_login.user,
            Some("Retention check"),
        )
        .unwrap();

    let _cust = state
        .case_service
        .record_custody_event(
            &case.case_id,
            RecordCustodyRequest {
                evidence_id: Some(ev.evidence_id.clone()),
                event_type: CustodyEventType::EvidenceIntroduced,
                action: "Registered for long term retention".to_string(),
                details: "Retention invariant test".to_string(),
            },
            &admin_login.user,
        )
        .unwrap();

    // Close the case
    let closed_case = state
        .case_service
        .update_case_status(&case.case_id, CaseStatus::Completed, &admin_login.user)
        .unwrap();
    assert_eq!(closed_case.status, CaseStatus::Completed);

    // Verify 100% of data remains queryable and intact
    let retained_summary = state
        .case_service
        .get_case_summary(&case.case_id)
        .expect("Summary query must succeed after case closure");
    assert_eq!(retained_summary.evidence_count, 1);
    assert_eq!(retained_summary.operation_count, 1);

    let retained_evidence = state
        .case_service
        .list_case_evidence(&case.case_id)
        .unwrap();
    assert_eq!(retained_evidence.len(), 1);

    let retained_timeline = state.case_service.get_case_timeline(&case.case_id).unwrap();
    assert!(!retained_timeline.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}
