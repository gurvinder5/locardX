use locardx_audit::AuditService;
use locardx_auth::{PublicUser, UserRole};
use locardx_case_management::models::*;
use locardx_case_management::CaseService;
use locardx_database::Database;
use locardx_reporting::ReportingService;
use std::sync::Arc;

fn setup_test_context() -> (CaseService, PublicUser) {
    let db = Arc::new(Database::open(":memory:").expect("In-memory DB must open"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let reporting = Arc::new(ReportingService::new(Arc::clone(&db), Arc::clone(&audit)));
    let service = CaseService::new(Arc::clone(&db), Arc::clone(&audit), Arc::clone(&reporting));

    let actor = PublicUser {
        user_id: "user-test-01".to_string(),
        username: "detective_miller".to_string(),
        role: UserRole::Investigator,
        display_name: None,
        enabled: true,
        created_at: "2026-09-12T00:00:00Z".to_string(),
        updated_at: "2026-09-12T00:00:00Z".to_string(),
        last_login_at: None,
        metadata_json: "{}".to_string(),
    };

    (service, actor)
}

#[test]
fn test_01_to_07_case_crud_and_associations() {
    let (service, actor) = setup_test_context();

    // 1. Case creation
    let req = CreateCaseRequest {
        case_reference: "CASE-2026-001".to_string(),
        title: "Operation Nightfall".to_string(),
        description: "Suspect storage device forensics".to_string(),
        metadata_json: Some(r#"{"priority":"High"}"#.to_string()),
    };
    let created = service
        .create_case(req, &actor)
        .expect("Case creation must succeed");
    assert_eq!(created.case_reference, "CASE-2026-001");
    assert_eq!(created.title, "Operation Nightfall");
    assert_eq!(created.status, CaseStatus::Open);
    assert_eq!(created.lead_investigator, "detective_miller");

    // 2. Case retrieval
    let fetched = service
        .get_case(&created.case_id)
        .expect("Query must succeed")
        .expect("Case must exist");
    assert_eq!(fetched.case_id, created.case_id);
    assert_eq!(fetched.case_reference, "CASE-2026-001");

    let by_ref = service
        .get_case_by_reference("case-2026-001")
        .expect("Query must succeed")
        .expect("Case must exist");
    assert_eq!(by_ref.case_id, created.case_id);

    // 3. Case update
    let update_req = UpdateCaseRequest {
        title: Some("Operation Nightfall - Priority Escalated".to_string()),
        description: Some("Updated description with warrant ref #9988".to_string()),
        metadata_json: None,
    };
    let updated = service
        .update_case(&created.case_id, update_req, &actor)
        .expect("Update must succeed");
    assert_eq!(updated.title, "Operation Nightfall - Priority Escalated");
    assert_eq!(
        updated.description,
        "Updated description with warrant ref #9988"
    );

    // 4. Case status transition
    let in_prog = service
        .update_case_status(&created.case_id, CaseStatus::InProgress, &actor)
        .expect("Status update must succeed");
    assert_eq!(in_prog.status, CaseStatus::InProgress);
    assert!(in_prog.closed_at.is_none());

    // 5. Case archival
    let archived = service
        .update_case_status(&created.case_id, CaseStatus::Archived, &actor)
        .expect("Status update must succeed");
    assert_eq!(archived.status, CaseStatus::Archived);
    assert!(archived.closed_at.is_some());

    // 6. Case-operation association
    let op_assoc = service
        .associate_operation(
            &created.case_id,
            "op-acq-99",
            "ForensicAcquisition",
            &actor,
            Some("Initial bitstream DD acquisition"),
        )
        .expect("Association must succeed");
    assert_eq!(op_assoc.case_id, created.case_id);
    assert_eq!(op_assoc.operation_id, "op-acq-99");
    assert_eq!(op_assoc.operation_type, "ForensicAcquisition");

    let ops = service
        .list_case_operations(&created.case_id)
        .expect("List operations must succeed");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].operation_id, "op-acq-99");

    // 7. Case-evidence association
    let ev_req = AddEvidenceRequest {
        evidence_type: EvidenceType::PhysicalStorage,
        identifier: r"\\.\PhysicalDrive1".to_string(),
        label: "Seized Western Digital Red 4TB HDD".to_string(),
        sha256: None,
        size_bytes: Some(4_000_787_030_016),
        notes: Some("Evidence tag: LOC-001".to_string()),
    };
    let ev = service
        .add_case_evidence(&created.case_id, ev_req, &actor)
        .expect("Add evidence must succeed");
    assert_eq!(ev.case_id, created.case_id);
    assert_eq!(ev.evidence_type, EvidenceType::PhysicalStorage);
    assert_eq!(ev.label, "Seized Western Digital Red 4TB HDD");

    let evidence_list = service
        .list_case_evidence(&created.case_id)
        .expect("List evidence must succeed");
    assert_eq!(evidence_list.len(), 1);
    assert_eq!(evidence_list[0].evidence_id, ev.evidence_id);
}

#[test]
fn test_08_to_14_audit_and_custody_timeline() {
    let (service, actor) = setup_test_context();

    // 8. Case creation audit event
    let req = CreateCaseRequest {
        case_reference: "CASE-AUDIT-01".to_string(),
        title: "Audit Investigation".to_string(),
        description: "Checking audit correlation".to_string(),
        metadata_json: None,
    };
    let case = service
        .create_case(req, &actor)
        .expect("Case must be created");

    let events = service
        .audit()
        .list_events(10)
        .expect("Audit list must succeed");
    let create_event = events.iter().find(|e| e.event_type == "CASE_CREATED");
    assert!(
        create_event.is_some(),
        "CASE_CREATED audit event must exist"
    );
    assert_eq!(
        create_event.unwrap().target_ref.as_deref(),
        Some(case.case_id.as_str())
    );

    // 9. Acquisition linked to case
    service
        .associate_operation(
            &case.case_id,
            "op-acq-sample-1",
            "ForensicAcquisition",
            &actor,
            None,
        )
        .expect("Acquisition association must succeed");

    // 10. Recovery linked to case
    service
        .associate_operation(&case.case_id, "op-rec-sample-1", "Recovery", &actor, None)
        .expect("Recovery association must succeed");

    // 11. Erasure linked to case
    service
        .associate_operation(
            &case.case_id,
            "op-era-sample-1",
            "DriveErasure",
            &actor,
            None,
        )
        .expect("Erasure association must succeed");

    // 12. Custody event and audit logging
    let cust_req = RecordCustodyRequest {
        evidence_id: None,
        event_type: CustodyEventType::EvidenceVerified,
        action: "Physical drive visual verification".to_string(),
        details: "Serial number S649NX0R confirmed against seizure receipt".to_string(),
    };
    service
        .record_custody_event(&case.case_id, cust_req, &actor)
        .expect("Custody event must succeed");

    // 13. Unified audit timeline
    let timeline = service
        .get_case_timeline(&case.case_id)
        .expect("Timeline must succeed");
    assert!(
        !timeline.is_empty(),
        "Timeline must contain custody and audit items"
    );
    for item in &timeline {
        assert!(!item.id.is_empty());
        assert!(!item.timestamp.is_empty());
    }

    // 14. Audit-chain integrity
    let chain_ver = service
        .audit()
        .verify_chain()
        .expect("Chain verify must succeed");
    assert!(
        chain_ver.is_valid,
        "Audit chain must be 100% cryptographically valid"
    );
    assert!(chain_ver.total_events >= 5);
}

#[test]
fn test_15_to_22_reporting_and_integrity() {
    let (service, actor) = setup_test_context();

    let req = CreateCaseRequest {
        case_reference: "CASE-REPORT-01".to_string(),
        title: "Report Validation Case".to_string(),
        description: "Checking end-to-end report generation".to_string(),
        metadata_json: None,
    };
    let case = service
        .create_case(req, &actor)
        .expect("Case must be created");

    // Setup dummy completed acquisition, recovery, and erasure records in DB
    service.database().with_conn(|conn| {
        conn.execute(
            "INSERT INTO acquisition_records (
                acquisition_id, operation_id, actor_id, source_device_id, source_display_name,
                source_serial, source_media_type, source_capacity_bytes, source_sector_size,
                source_snapshot_json, destination_path, image_format, image_size_bytes,
                image_sha256, status, bytes_acquired, elapsed_seconds, audit_reference, started_at, completed_at
            ) VALUES (
                'acq-100', 'op-acq-100', 'detective_miller', '\\\\.\\PhysicalDrive1', 'Samsung SSD 870',
                'SAMSUNG-001', 'SSD', 500107862016, 512, '{}', '/evidence/img.dd', 'Raw/DD',
                500107862016, 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
                'Completed', 500107862016, 25.5, 'AUDIT_ACQ_100', '2026-09-12T01:00:00Z', '2026-09-12T01:25:30Z'
            )",
            [],
        )?;

        conn.execute(
            "INSERT INTO recovery_jobs (
                job_id, operation_id, actor_id, acquisition_id, source_image_path, source_image_sha256,
                recovery_mode, status, bytes_scanned, files_recovered, candidates_evaluated,
                elapsed_seconds, audit_reference, started_at, completed_at
            ) VALUES (
                'rec-job-200', 'op-rec-200', 'detective_miller', 'acq-100', '/evidence/img.dd',
                'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
                'All', 'Completed', 500107862016, 35, 42, 18.2, 'AUDIT_REC_200',
                '2026-09-12T01:30:00Z', '2026-09-12T01:48:12Z'
            )",
            [],
        )?;

        conn.execute(
            "INSERT INTO recovered_files (
                file_id, job_id, source_offset, size_bytes, file_type, mime_type, suggested_filename,
                recovery_method, validation_status, confidence_score, confidence_grade, is_fragmented,
                sha256_hash, output_relative_path, evidence_factors_json, created_at
            ) VALUES (
                'rec-file-1', 'rec-job-200', 1048576, 204800, 'JPEG', 'image/jpeg', 'carved_evidence.jpg',
                'CARVING_SIGNATURE', 'VALID', 90, 'High', 0,
                'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
                'carved_evidence.jpg', '{}', '2026-09-12T01:45:00Z'
            )",
            [],
        )?;

        conn.execute(
            "INSERT INTO drive_erasure_records (
                record_id, operation_id, actor_id, physical_device_id, display_name, serial_number,
                media_type, capacity_bytes, sector_size, device_snapshot_json, capabilities_json,
                sanitization_method, execution_mode, verification_strategy, verification_outcome,
                status, bytes_processed, elapsed_seconds, started_at, completed_at
            ) VALUES (
                'era-rec-300', 'op-era-300', 'detective_miller', '\\\\.\\PhysicalDrive2', 'Cruzer Blade 16GB',
                'CRUZER-99', 'USB', 16000000000, 512, '{}', '{}',
                'NistClearSinglePassZeros', 'RealHardware', 'FullDeviceReadVerification',
                'Verified', 'Completed', 16000000000, 32.0, '2026-09-12T02:00:00Z', '2026-09-12T02:32:00Z'
            )",
            [],
        )?;
        Ok(())
    }).unwrap();

    // Associate operations with case
    service
        .associate_operation(
            &case.case_id,
            "op-acq-100",
            "ForensicAcquisition",
            &actor,
            None,
        )
        .unwrap();
    service
        .associate_operation(&case.case_id, "op-rec-200", "Recovery", &actor, None)
        .unwrap();
    service
        .associate_operation(&case.case_id, "op-era-300", "DriveErasure", &actor, None)
        .unwrap();

    // 15. Case report generation
    let report = service
        .generate_case_report(&case.case_id, &actor)
        .expect("Report generation must succeed");

    // 16. Contains acquisition info
    assert_eq!(report.acquisitions.len(), 1);
    assert_eq!(
        report.acquisitions[0].source_display_name,
        "Samsung SSD 870"
    );

    // 17. Contains recovery info
    assert_eq!(report.recoveries.len(), 1);
    assert_eq!(report.recoveries[0].files_recovered, 35);
    assert_eq!(report.recoveries[0].sample_files.len(), 1);
    assert_eq!(
        report.recoveries[0].sample_files[0].filename,
        "carved_evidence.jpg"
    );

    // 18. Contains erasure info
    assert_eq!(report.erasures.len(), 1);
    assert_eq!(report.erasures[0].display_name, "Cruzer Blade 16GB");
    assert_eq!(report.erasures[0].verification_outcome, "Verified");

    // 19. Contains audit information
    assert!(report.audit_integrity.is_valid);
    assert!(report.audit_integrity.total_events > 0);

    // 20. Report digest generation
    assert!(!report.integrity.report_digest.is_empty());

    // 21. Report integrity verification
    assert!(service.verify_case_report(&report));

    // 22. Report tamper detection
    let mut tampered = report.clone();
    tampered.acquisitions[0].image_sha256 =
        "0000000000000000000000000000000000000000000000000000000000000000".to_string();
    assert!(
        !service.verify_case_report(&tampered),
        "Tampered report must fail verification"
    );
}

#[test]
fn test_23_to_30_cross_module_integration_and_invariants() {
    let (service, actor) = setup_test_context();

    let req = CreateCaseRequest {
        case_reference: "CASE-INTEG-01".to_string(),
        title: "Integration Testing Case".to_string(),
        description: "Checking cross-module persistence".to_string(),
        metadata_json: None,
    };
    let case = service
        .create_case(req, &actor)
        .expect("Case must be created");

    // 23. AcquisitionArtifact associated with a case
    let ev_acq = AddEvidenceRequest {
        evidence_type: EvidenceType::AcquisitionImage,
        identifier: "/evidence/disk_sample.dd".to_string(),
        label: "Acquisition Artifact Image".to_string(),
        sha256: Some(
            "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8".to_string(),
        ),
        size_bytes: Some(104857600),
        notes: Some("Evidential bitstream image from Step 11".to_string()),
    };
    let ev = service
        .add_case_evidence(&case.case_id, ev_acq.clone(), &actor)
        .expect("Evidence must be added");
    assert_eq!(ev.evidence_type, EvidenceType::AcquisitionImage);

    // 24. Recovery job referencing acquisition evidence
    service
        .associate_operation(
            &case.case_id,
            "op-rec-job-99",
            "Recovery",
            &actor,
            Some("Carving from AcquisitionArtifact"),
        )
        .unwrap();

    // 25. Recovery results appear in case dashboard
    let summary = service
        .get_case_summary(&case.case_id)
        .expect("Summary must succeed");
    assert_eq!(summary.operation_count, 1);
    assert_eq!(summary.evidence_count, 1);
    assert_eq!(summary.case.status, CaseStatus::Open);

    // 26. Existing Step 10 reports remain accessible
    service.database().with_conn(|conn| {
        conn.execute(
            "INSERT INTO sanitization_reports (
                report_id, operation_id, plan_id, target_identifier, execution_mode, is_simulation,
                status, verification_outcome, report_digest, audit_chain_reference, report_json, generated_at
            ) VALUES (
                'rep-san-historical', 'op-san-hist', 'plan-hist', '\\\\.\\PhysicalDrive0', 'RealHardware', 0,
                'Completed', 'Verified', 'DIGEST_HIST', 'AUDIT_REF_HIST', '{}', '2026-09-11T12:00:00Z'
            )",
            [],
        )?;
        Ok(())
    }).unwrap();

    let hist_report_exists: bool = service
        .database()
        .with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT COUNT(*) FROM sanitization_reports WHERE report_id = 'rep-san-historical'",
            )?;
            let count: i64 = stmt.query_row([], |r| r.get(0))?;
            Ok(count > 0)
        })
        .unwrap();
    assert!(
        hist_report_exists,
        "Step 10 historical reports must remain accessible"
    );

    // 27. Existing Step 11 acquisition records remain accessible
    service.database().with_conn(|conn| {
        conn.execute(
            "INSERT INTO acquisition_records (
                acquisition_id, operation_id, source_device_id, source_display_name, source_media_type,
                source_capacity_bytes, source_sector_size, source_snapshot_json, destination_path,
                image_format, image_size_bytes, image_sha256, status, bytes_acquired, elapsed_seconds,
                audit_reference, started_at, completed_at
            ) VALUES (
                'acq-hist', 'op-acq-hist', '\\\\.\\PhysicalDrive1', 'Historical Drive', 'HDD',
                1000000000, 512, '{}', '/evidence/hist.dd', 'Raw/DD', 1000000000, 'SHA256_HIST',
                'Completed', 1000000000, 10.0, 'AUDIT_HIST', '2026-09-11T13:00:00Z', '2026-09-11T13:10:00Z'
            )",
            [],
        )?;
        Ok(())
    }).unwrap();

    let hist_acq_exists: bool = service
        .database()
        .with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT COUNT(*) FROM acquisition_records WHERE acquisition_id = 'acq-hist'",
            )?;
            let count: i64 = stmt.query_row([], |r| r.get(0))?;
            Ok(count > 0)
        })
        .unwrap();
    assert!(
        hist_acq_exists,
        "Step 11 historical acquisitions must remain accessible"
    );

    // 28. Existing Step 12 recovery records remain accessible
    service
        .database()
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO recovery_jobs (
                job_id, operation_id, acquisition_id, source_image_path, source_image_sha256,
                recovery_mode, status, audit_reference, started_at, completed_at
            ) VALUES (
                'rec-hist', 'op-rec-hist', 'acq-hist', '/evidence/hist.dd', 'SHA256_HIST',
                'All', 'Completed', 'AUDIT_HIST', '2026-09-11T14:00:00Z', '2026-09-11T14:20:00Z'
            )",
                [],
            )?;
            Ok(())
        })
        .unwrap();

    let hist_rec_exists: bool = service
        .database()
        .with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT COUNT(*) FROM recovery_jobs WHERE job_id = 'rec-hist'")?;
            let count: i64 = stmt.query_row([], |r| r.get(0))?;
            Ok(count > 0)
        })
        .unwrap();
    assert!(
        hist_rec_exists,
        "Step 12 historical recoveries must remain accessible"
    );

    // 29. Closing a case does NOT delete evidence or operations
    let closed_case = service
        .update_case_status(&case.case_id, CaseStatus::Completed, &actor)
        .expect("Close must succeed");
    assert_eq!(closed_case.status, CaseStatus::Completed);

    let remaining_evidence = service
        .list_case_evidence(&case.case_id)
        .expect("Evidence query must succeed");
    assert_eq!(
        remaining_evidence.len(),
        1,
        "Evidence must be preserved when case is closed"
    );

    let remaining_ops = service
        .list_case_operations(&case.case_id)
        .expect("Operations query must succeed");
    assert_eq!(
        remaining_ops.len(),
        1,
        "Operations must be preserved when case is closed"
    );

    // 30. Invalid case/evidence references fail safely
    let invalid_case_res = service.add_case_evidence("non-existent-case-id", ev_acq, &actor);
    assert!(
        invalid_case_res.is_err(),
        "Non-existent case ID must fail closed with error"
    );

    let invalid_op_res = service.associate_operation(
        "non-existent-case-id",
        "op-1",
        "ForensicAcquisition",
        &actor,
        None,
    );
    assert!(
        invalid_op_res.is_err(),
        "Non-existent case ID must fail closed with error"
    );
}

#[test]
fn test_active_case_enforcement() {
    let (service, actor) = setup_test_context();

    // 1. When no active case is set, require_active_case fails closed with ACTIVE_CASE_REQUIRED
    let err = service.require_active_case().unwrap_err();
    match err {
        locardx_common::LocardError::SecurityViolation(msg) => {
            assert!(msg.contains("ACTIVE_CASE_REQUIRED"), "Expected ACTIVE_CASE_REQUIRED error, got: {}", msg);
        }
        other => panic!("Expected SecurityViolation, got {:?}", other),
    }

    // 2. Create an open case and set it as active
    let req = CreateCaseRequest {
        case_reference: "CASE-2026-ACT1".to_string(),
        title: "Active Case Test".to_string(),
        description: "Testing active case enforcement".to_string(),
        metadata_json: None,
    };
    let case1 = service.create_case(req, &actor).expect("Create case");
    let active = service.set_active_case(&case1.case_id).expect("Set active case");
    assert_eq!(active.case_id, case1.case_id);

    // 3. Now require_active_case succeeds and returns case1
    let req_case = service.require_active_case().expect("Require active case must succeed");
    assert_eq!(req_case.case_id, case1.case_id);

    // 4. Close the case -> require_active_case must fail closed with CASE_CLOSED
    service.update_case_status(&case1.case_id, CaseStatus::Completed, &actor).expect("Complete case");
    let err_closed = service.require_active_case().unwrap_err();
    match err_closed {
        locardx_common::LocardError::SecurityViolation(msg) => {
            assert!(msg.contains("CASE_CLOSED"), "Expected CASE_CLOSED error, got: {}", msg);
        }
        other => panic!("Expected SecurityViolation, got {:?}", other),
    }

    // 5. Clear active case -> require_active_case returns ACTIVE_CASE_REQUIRED
    service.clear_active_case();
    assert!(service.get_active_case().unwrap().is_none());
    let err_cleared = service.require_active_case().unwrap_err();
    match err_cleared {
        locardx_common::LocardError::SecurityViolation(msg) => {
            assert!(msg.contains("ACTIVE_CASE_REQUIRED"));
        }
        other => panic!("Expected SecurityViolation, got {:?}", other),
    }

    // 6. Setting non-existent case fails
    assert!(service.set_active_case("non-existent-case").is_err());
}

#[test]
fn test_case_validation_guards_and_closure_prevention() {
    let (service, actor) = setup_test_context();

    // 1. Validation for operation without case: fails closed
    let val_err = service.validate_case_for_operation(None).unwrap_err();
    match val_err {
        locardx_common::LocardError::SecurityViolation(msg) => {
            assert!(msg.contains("ACTIVE_CASE_REQUIRED"));
        }
        other => panic!("Expected SecurityViolation, got {:?}", other),
    }

    // 2. Dual-mode validation for standalone (None): succeeds
    let standalone = service
        .validate_optional_case_or_standalone(None)
        .expect("Standalone allowed");
    assert!(standalone.is_none());

    // 3. Create a case
    let req = CreateCaseRequest {
        case_reference: "CASE-VAL-01".to_string(),
        title: "Validation Guards Test".to_string(),
        description: "Testing operation validation and closure prevention".to_string(),
        metadata_json: None,
    };
    let case = service.create_case(req, &actor).expect("Create case");

    // 4. Validate with valid open case
    let val_ok = service
        .validate_case_for_operation(Some(&case.case_id))
        .expect("Valid case");
    assert_eq!(val_ok.case_id, case.case_id);

    let dual_ok = service
        .validate_optional_case_or_standalone(Some(&case.case_id))
        .expect("Valid case dual");
    assert_eq!(dual_ok.unwrap().case_id, case.case_id);

    // 5. Validation with non-existent case ID fails
    assert!(service
        .validate_case_for_operation(Some("ghost-case"))
        .is_err());
    assert!(service
        .validate_optional_case_or_standalone(Some("ghost-case"))
        .is_err());

    // 6. Test closure prevention when non-terminal operations exist
    service
        .database()
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO operations (
                operation_id, operation_type, target_type, target_identifier, target_display_name, actor_id, current_state, created_at, started_at, case_id
            ) VALUES (
                'op-running-1', 'DriveErasure', 'Device', '\\\\.\\PhysicalDrive1', 'Test Drive', 'detective_miller', 'Running', '2026-09-12T00:00:00Z', '2026-09-12T00:00:00Z', ?1
            )",
                [&case.case_id],
            )?;
            Ok(())
        })
        .unwrap();

    // Attempting to close or archive case while op-running-1 is Running MUST fail
    let close_err = service
        .update_case_status(&case.case_id, CaseStatus::Completed, &actor)
        .unwrap_err();
    match close_err {
        locardx_common::LocardError::Operation(msg) => {
            assert!(msg.contains("Cannot close case"));
        }
        other => panic!("Expected Operation error, got {:?}", other),
    }

    // Mark operation as completed in database
    service
        .database()
        .with_conn(|conn| {
            conn.execute(
                "UPDATE operations SET current_state = 'Completed', completed_at = '2026-09-12T01:00:00Z' WHERE operation_id = 'op-running-1'",
                [],
            )?;
            Ok(())
        })
        .unwrap();

    // Now closing case succeeds!
    let closed = service
        .update_case_status(&case.case_id, CaseStatus::Completed, &actor)
        .expect("Close succeeds now");
    assert_eq!(closed.status, CaseStatus::Completed);

    // 7. Validate operation against closed case fails
    let closed_val_err = service
        .validate_case_for_operation(Some(&case.case_id))
        .unwrap_err();
    match closed_val_err {
        locardx_common::LocardError::SecurityViolation(msg) => {
            assert!(msg.contains("CASE_CLOSED"));
        }
        other => panic!("Expected SecurityViolation error, got {:?}", other),
    }
}

#[test]
fn test_case_reopening_authorization() {
    let (service, actor) = setup_test_context();

    // Create and close case
    let req = CreateCaseRequest {
        case_reference: "CASE-REOPEN-01".to_string(),
        title: "Reopening Test".to_string(),
        description: "Testing reopening authorization".to_string(),
        metadata_json: None,
    };
    let case = service.create_case(req, &actor).expect("Create case");
    service
        .update_case_status(&case.case_id, CaseStatus::Completed, &actor)
        .expect("Close case");

    // Viewer actor should be rejected
    let viewer_actor = PublicUser {
        user_id: "user-viewer-01".to_string(),
        username: "viewer_bob".to_string(),
        role: UserRole::Viewer,
        display_name: None,
        enabled: true,
        created_at: "2026-09-12T00:00:00Z".to_string(),
        updated_at: "2026-09-12T00:00:00Z".to_string(),
        last_login_at: None,
        metadata_json: "{}".to_string(),
    };

    let reopen_err = service
        .reopen_case(&case.case_id, &viewer_actor)
        .unwrap_err();
    match reopen_err {
        locardx_common::LocardError::SecurityViolation(msg) => {
            assert!(msg.contains("Only an Administrator or Investigator"));
        }
        other => panic!("Expected SecurityViolation, got {:?}", other),
    }

    // Authorized investigator actor succeeds
    let reopened = service
        .reopen_case(&case.case_id, &actor)
        .expect("Reopen succeeds");
    assert_eq!(reopened.status, CaseStatus::InProgress);
    assert!(reopened.closed_at.is_none());

    // Verify CASE_REOPENED audit event
    let events = service.audit().list_events(10).expect("List audit");
    let reopen_ev = events.iter().find(|e| e.event_type == "CASE_REOPENED");
    assert!(
        reopen_ev.is_some(),
        "CASE_REOPENED audit event must be logged"
    );
}

#[test]
fn test_case_module_queries() {
    let (service, actor) = setup_test_context();

    let req = CreateCaseRequest {
        case_reference: "CASE-QUERIES-01".to_string(),
        title: "Module Queries Test".to_string(),
        description: "Testing listing acquisitions, recoveries, erasures".to_string(),
        metadata_json: None,
    };
    let case = service.create_case(req, &actor).expect("Create case");

    // Seed acquisition, recovery, drive erasure, and file erasure records with case_id
    service
        .database()
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO acquisition_records (
                acquisition_id, operation_id, actor_id, source_device_id, source_display_name,
                source_serial, source_media_type, source_capacity_bytes, source_sector_size,
                source_snapshot_json, destination_path, image_format, image_size_bytes,
                image_sha256, status, bytes_acquired, elapsed_seconds, audit_reference, started_at, completed_at, case_id
            ) VALUES (
                'acq-q-1', 'op-acq-q1', 'detective_miller', '\\\\.\\PhysicalDrive1', 'Seagate Barracuda',
                'SN-123', 'HDD', 2000000000000, 512, '{}', '/evidence/acq1.raw', 'Raw/DD',
                2000000000000, 'hash1', 'Completed', 2000000000000, 100.0, 'AUDIT_Q1',
                '2026-09-12T01:00:00Z', '2026-09-12T01:30:00Z', ?1
            )",
                [&case.case_id],
            )?;

            conn.execute(
                "INSERT INTO recovery_jobs (
                job_id, operation_id, actor_id, acquisition_id, source_image_path, source_image_sha256,
                recovery_mode, status, bytes_scanned, files_recovered, candidates_evaluated,
                elapsed_seconds, audit_reference, started_at, completed_at, case_id
            ) VALUES (
                'rec-q-1', 'op-rec-q1', 'detective_miller', 'acq-q-1', '/evidence/acq1.raw', 'hash1',
                'All', 'Completed', 2000000000000, 15, 20, 50.0, 'AUDIT_REC_Q1',
                '2026-09-12T02:00:00Z', '2026-09-12T02:25:00Z', ?1
            )",
                [&case.case_id],
            )?;

            conn.execute(
                "INSERT INTO drive_erasure_records (
                record_id, operation_id, actor_id, physical_device_id, display_name, serial_number,
                media_type, capacity_bytes, sector_size, device_snapshot_json, capabilities_json,
                sanitization_method, execution_mode, verification_strategy, verification_outcome,
                status, bytes_processed, elapsed_seconds, started_at, completed_at, case_id
            ) VALUES (
                'era-drv-q1', 'op-drv-q1', 'detective_miller', '\\\\.\\PhysicalDrive3', 'SanDisk Ultra',
                'SANDISK-9', 'USB', 32000000000, 512, '{}', '{}',
                'NistClearSinglePassZeros', 'Simulation', 'FullDeviceReadVerification',
                'Verified', 'Completed', 32000000000, 20.0, '2026-09-12T03:00:00Z', '2026-09-12T03:10:00Z', ?1
            )",
                [&case.case_id],
            )?;

            conn.execute(
                "INSERT INTO file_erasure_records (
                record_id, operation_id, actor_id, target_path, canonical_path, scope,
                sanitization_method, status, verification_outcome, bytes_processed,
                total_files, files_sanitized, files_failed, directories_removed,
                pre_metadata_json, limitations, started_at, completed_at, case_id
            ) VALUES (
                'era-file-q1', 'op-file-q1', 'detective_miller', 'C:\\temp\\secret.txt', 'C:\\temp\\secret.txt',
                'File', 'LogicalFileShred', 'Completed', 'Verified', 1024,
                1, 1, 0, 0,
                '{}', '{}', '2026-09-12T04:00:00Z', '2026-09-12T04:01:00Z', ?1
            )",
                [&case.case_id],
            )?;

            Ok(())
        })
        .unwrap();

    // Query acquisitions
    let acqs = service
        .list_case_acquisitions(&case.case_id)
        .expect("List acquisitions");
    assert_eq!(acqs.len(), 1);
    assert_eq!(acqs[0].acquisition_id, "acq-q-1");
    assert_eq!(acqs[0].source_display_name, "Seagate Barracuda");

    // Query recoveries
    let recs = service
        .list_case_recoveries(&case.case_id)
        .expect("List recoveries");
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].job_id, "rec-q-1");
    assert_eq!(recs[0].files_recovered, 15);

    // Query erasures (both drive and file)
    let erasures = service
        .list_case_erasures(&case.case_id)
        .expect("List erasures");
    assert_eq!(erasures.len(), 2);
    let drv = erasures
        .iter()
        .find(|e| e.erasure_type == "Drive")
        .expect("Drive erasure found");
    assert_eq!(drv.target_identifier, "SanDisk Ultra");
    let fil = erasures
        .iter()
        .find(|e| e.erasure_type == "File")
        .expect("File erasure found");
    assert_eq!(fil.target_identifier, "C:\\temp\\secret.txt");
}


