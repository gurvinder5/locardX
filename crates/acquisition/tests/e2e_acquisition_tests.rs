use locardx_acquisition::{
    validate_destination, validate_source_device_id, AcquisitionArtifact,
    AcquisitionDeviceSnapshot, AcquisitionEngine, AcquisitionFailureReason, AcquisitionProgress,
    AcquisitionResult, AcquisitionService, AcquisitionSourceReader, AcquisitionStatus,
    FaultInjectableAcquisitionReader, FaultInjectionMode, StreamingSha256Hasher,
};
use locardx_audit::AuditService;
use locardx_database::Database;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn setup_test_env(
    device_capacity: u64,
) -> (
    Arc<Database>,
    Arc<AuditService>,
    Arc<FaultInjectableAcquisitionReader>,
    AcquisitionService,
) {
    let db = Arc::new(Database::open(":memory:").expect("Failed to open test in-memory DB"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let reader = Arc::new(FaultInjectableAcquisitionReader::new_with_test_device(
        r"\\.\PhysicalDrive1",
        device_capacity,
    ));
    let service = AcquisitionService::new(Arc::clone(&db), Arc::clone(&audit))
        .with_reader(Arc::clone(&reader) as Arc<dyn locardx_acquisition::AcquisitionSourceReader>);

    (db, audit, reader, service)
}

#[test]
fn test_01_source_validation_rejects_logical_volumes() {
    assert!(validate_source_device_id(r"C:\").is_err());
    assert!(validate_source_device_id("C:").is_err());
    assert!(validate_source_device_id(r"D:\Data").is_err());
    assert!(validate_source_device_id("/home/user").is_err());
    assert!(validate_source_device_id("").is_err());

    // Valid physical drive identifiers
    assert!(validate_source_device_id(r"\\.\PhysicalDrive1").is_ok());
    assert!(validate_source_device_id("PhysicalDrive2").is_ok());
    assert!(validate_source_device_id("/dev/sdb").is_ok());
    assert!(validate_source_device_id("/dev/nvme0n1").is_ok());
}

#[test]
fn test_02_destination_validation_rejects_collisions_and_device_nodes() {
    let (_, _, reader, _) = setup_test_env(1024 * 1024);

    // 1. Destination is device node
    let res = validate_destination(
        reader.as_ref(),
        r"\\.\PhysicalDrive1",
        r"\\.\PhysicalDrive2",
        1024 * 1024,
        true,
    );
    assert!(matches!(
        res,
        Err(AcquisitionFailureReason::InvalidDestination(_))
    ));

    // 2. Source equals destination
    let res = validate_destination(
        reader.as_ref(),
        r"\\.\PhysicalDrive1",
        r"\\.\PhysicalDrive1",
        1024 * 1024,
        true,
    );
    assert!(matches!(
        res,
        Err(AcquisitionFailureReason::InvalidDestination(_))
    ));

    // 3. Destination on source physical disk
    reader.register_destination_mapping(r"E:\", r"\\.\PhysicalDrive1");
    let res = validate_destination(
        reader.as_ref(),
        r"\\.\PhysicalDrive1",
        r"E:\evidence\case.dd",
        1024 * 1024,
        true,
    );
    assert!(matches!(
        res,
        Err(AcquisitionFailureReason::DestinationOnSourceDisk)
    ));
}

#[test]
fn test_03_destination_validation_existing_file_without_overwrite() {
    let (_, _, reader, _) = setup_test_env(1024);
    let temp_dir = tempfile::tempdir().unwrap();
    let existing_file = temp_dir.path().join("existing.raw");
    std::fs::write(&existing_file, b"existing evidence").unwrap();

    // With allow_overwrite = false -> must fail
    let res = validate_destination(
        reader.as_ref(),
        r"\\.\PhysicalDrive1",
        existing_file.to_str().unwrap(),
        1024,
        false,
    );
    assert!(matches!(
        res,
        Err(AcquisitionFailureReason::DestinationAlreadyExists(_))
    ));

    // With allow_overwrite = true -> must succeed
    let res_overwrite = validate_destination(
        reader.as_ref(),
        r"\\.\PhysicalDrive1",
        existing_file.to_str().unwrap(),
        1024,
        true,
    );
    assert!(res_overwrite.is_ok());
}

#[test]
fn test_04_toctou_mutation_detection() {
    let (_, _, reader, service) = setup_test_env(1024 * 1024);
    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("image_toctou.dd");

    let plan = service
        .create_plan(
            r"\\.\PhysicalDrive1",
            dest_file.to_str().unwrap(),
            true,
            Some("operator_1"),
        )
        .expect("Plan creation failed");

    // Inject capacity mutation before execution
    reader.set_mode(FaultInjectionMode::MutateCapacity(2048 * 1024));

    let res = service
        .execute_acquisition("op-toctou-cap", &plan, Some("operator_1"))
        .unwrap();
    assert_eq!(res.status, AcquisitionStatus::Failed);
    assert!(matches!(
        res.failure_reason,
        Some(AcquisitionFailureReason::SourceMutated(_))
    ));

    // Reset, inject serial mutation
    reader.set_mode(FaultInjectionMode::MutateSerial(
        "MUTATED-SERIAL-999".to_string(),
    ));
    let res_ser = service
        .execute_acquisition("op-toctou-ser", &plan, Some("operator_1"))
        .unwrap();
    assert_eq!(res_ser.status, AcquisitionStatus::Failed);
    assert!(matches!(
        res_ser.failure_reason,
        Some(AcquisitionFailureReason::SourceMutated(_))
    ));

    // Inject device disappearance
    reader.set_mode(FaultInjectionMode::Disappear);
    let res_disp = service
        .execute_acquisition("op-toctou-disp", &plan, Some("operator_1"))
        .unwrap();
    assert_eq!(res_disp.status, AcquisitionStatus::Failed);
    assert!(matches!(
        res_disp.failure_reason,
        Some(AcquisitionFailureReason::SourceDisconnected)
    ));
}

#[test]
fn test_05_successful_acquisition_and_streaming_sha256() {
    let capacity = 512 * 1024; // 512 KiB
    let (_, _, _, service) = setup_test_env(capacity);
    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("evidence.dd");

    let plan = service
        .create_plan(
            r"\\.\PhysicalDrive1",
            dest_file.to_str().unwrap(),
            true,
            Some("forensic_agent"),
        )
        .unwrap();

    let result = service
        .execute_acquisition("op-acq-001", &plan, Some("forensic_agent"))
        .unwrap();

    assert_eq!(result.status, AcquisitionStatus::Completed);
    assert_eq!(result.bytes_acquired, capacity);
    assert_eq!(result.image_size_bytes, capacity);
    assert_eq!(result.image_sha256.len(), 64);
    assert!(dest_file.exists());
    assert_eq!(std::fs::metadata(&dest_file).unwrap().len(), capacity);

    // Verify written file matches streaming hash independently
    let file_bytes = std::fs::read(&dest_file).unwrap();
    let mut independent_hasher = StreamingSha256Hasher::new();
    independent_hasher.update(&file_bytes);
    let (indep_hash, indep_len) = independent_hasher.finalize();

    assert_eq!(indep_len, capacity);
    assert_eq!(indep_hash, result.image_sha256);
}

#[test]
fn test_06_cooperative_cancellation_cleans_up_destination() {
    let capacity = 10 * 1024 * 1024; // 10 MiB
    let (_, _, reader, _) = setup_test_env(capacity);
    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("cancelled.raw");

    let stream = reader.open_read_only(r"\\.\PhysicalDrive1").unwrap();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_check = {
        let flag = Arc::clone(&cancel_flag);
        move || flag.load(Ordering::Relaxed)
    };

    let blocks_read = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let blocks_read_clone = Arc::clone(&blocks_read);
    let cancel_flag_clone = Arc::clone(&cancel_flag);
    let on_progress = move |_prog: AcquisitionProgress| {
        let count = blocks_read_clone.fetch_add(1, Ordering::Relaxed);
        if count >= 1 {
            cancel_flag_clone.store(true, Ordering::Relaxed);
        }
    };

    let res = AcquisitionEngine::execute(
        "op-cancel-test",
        stream,
        &dest_file,
        64 * 1024,
        capacity,
        Some(&cancel_check),
        Some(&on_progress),
    );

    assert!(matches!(res, Err(AcquisitionFailureReason::Cancelled)));
    // Incomplete file must be deleted upon cancellation cleanup
    assert!(!dest_file.exists());
}

#[test]
fn test_07_short_read_detection() {
    let capacity = 1024 * 1024; // 1 MiB
    let (_, _, reader, _) = setup_test_env(capacity);
    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("short_read.raw");

    // Inject short read at offset 256 KiB, returning only 100 bytes then EOF
    reader.set_mode(FaultInjectionMode::ShortReadAtOffset {
        trigger_offset: 256 * 1024,
        return_bytes: 100,
    });

    let stream = reader.open_read_only(r"\\.\PhysicalDrive1").unwrap();
    let res = AcquisitionEngine::execute(
        "op-short-read",
        stream,
        &dest_file,
        64 * 1024,
        capacity,
        None,
        None,
    );

    assert!(
        matches!(res, Err(AcquisitionFailureReason::ShortRead { expected, actual }) if expected == capacity && actual < capacity)
    );
    assert!(!dest_file.exists());
}

#[test]
fn test_08_mid_stream_read_io_error_handling() {
    let capacity = 512 * 1024;
    let (_, _, reader, _) = setup_test_env(capacity);
    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("io_error.dd");

    // Inject read error at offset 128 KiB
    reader.set_mode(FaultInjectionMode::FailReadAtOffset(128 * 1024));

    let stream = reader.open_read_only(r"\\.\PhysicalDrive1").unwrap();
    let res = AcquisitionEngine::execute(
        "op-io-err",
        stream,
        &dest_file,
        64 * 1024,
        capacity,
        None,
        None,
    );

    assert!(matches!(res, Err(AcquisitionFailureReason::ReadError(_))));
    assert!(!dest_file.exists());
}

#[test]
fn test_09_database_persistence_and_retrieval() {
    let capacity = 256 * 1024;
    let (_, _, _, service) = setup_test_env(capacity);
    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("persisted.raw");

    let plan = service
        .create_plan(
            r"\\.\PhysicalDrive1",
            dest_file.to_str().unwrap(),
            true,
            Some("investigator_smith"),
        )
        .unwrap();

    let result = service
        .execute_acquisition("op-persist-01", &plan, Some("investigator_smith"))
        .unwrap();

    let retrieved = service
        .get_acquisition_result("op-persist-01")
        .unwrap()
        .expect("Record not found");
    assert_eq!(retrieved.acquisition_id, result.acquisition_id);
    assert_eq!(retrieved.operation_id, result.operation_id);
    assert_eq!(retrieved.image_sha256, result.image_sha256);
    assert_eq!(retrieved.bytes_acquired, capacity);
    assert_eq!(retrieved.status, AcquisitionStatus::Completed);
}

#[test]
fn test_10_recovery_artifact_handoff_and_integrity_check() {
    let capacity = 128 * 1024;
    let (_, _, _, service) = setup_test_env(capacity);
    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("artifact.dd");

    let plan = service
        .create_plan(
            r"\\.\PhysicalDrive1",
            dest_file.to_str().unwrap(),
            true,
            Some("operator"),
        )
        .unwrap();

    let result = service
        .execute_acquisition("op-artifact-01", &plan, Some("operator"))
        .unwrap();

    let artifact = service
        .get_artifact("op-artifact-01")
        .unwrap()
        .expect("Artifact generation failed");
    assert_eq!(artifact.acquisition_id, result.acquisition_id);
    assert_eq!(artifact.image_path, result.destination_path);
    assert_eq!(artifact.image_size_bytes, capacity);
    assert_eq!(artifact.image_sha256, result.image_sha256);
    assert!(artifact.is_verified);

    // Verify artifact file integrity passes
    assert!(artifact.verify_integrity().unwrap());

    // Tampering with the image file on disk must fail integrity verification
    std::fs::write(&dest_file, b"corrupted evidence block").unwrap();
    assert!(!artifact.verify_integrity().unwrap());
}

#[test]
fn test_11_incomplete_acquisition_cannot_produce_artifact() {
    let incomplete_result = AcquisitionResult {
        acquisition_id: "acq-incomplete".to_string(),
        operation_id: "op-incomplete".to_string(),
        source: locardx_acquisition::AcquisitionDeviceSnapshot {
            device_id: r"\\.\PhysicalDrive1".to_string(),
            display_name: "Test".to_string(),
            vendor: None,
            model: None,
            serial_number: None,
            media_type: "HDD".to_string(),
            capacity_bytes: 1000,
            sector_size: 512,
            bus_type: None,
            is_removable: false,
            is_system: false,
            snapshot_timestamp: "2026-09-12T00:00:00Z".to_string(),
        },
        destination_path: "/tmp/nonexistent.dd".to_string(),
        image_format: "raw".to_string(),
        image_size_bytes: 0,
        image_sha256: String::new(),
        status: AcquisitionStatus::Failed,
        bytes_acquired: 0,
        elapsed_seconds: 0.0,
        average_throughput_mbps: 0.0,
        failure_reason: Some(AcquisitionFailureReason::SourceDisconnected),
        audit_reference: "AUDIT_REF".to_string(),
        started_at: "2026-09-12T00:00:00Z".to_string(),
        completed_at: "2026-09-12T00:00:01Z".to_string(),
    };

    let artifact_err = AcquisitionArtifact::from_result(&incomplete_result);
    assert!(artifact_err.is_err());
}

#[test]
fn test_12_startup_crash_recovery_sweep() {
    let (db, _audit, _reader, service) = setup_test_env(1024 * 1024);

    let snapshot_json = serde_json::to_string(&AcquisitionDeviceSnapshot {
        device_id: r"\\.\PhysicalDrive1".to_string(),
        display_name: "Test Disk".to_string(),
        vendor: Some("LocardX".to_string()),
        model: Some("Test".to_string()),
        serial_number: Some("TEST-SER".to_string()),
        media_type: "SSD".to_string(),
        capacity_bytes: 1048576,
        sector_size: 512,
        bus_type: Some("SATA".to_string()),
        is_removable: false,
        is_system: false,
        snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
    })
    .unwrap();

    let now = chrono::Utc::now().to_rfc3339();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO acquisition_records (
                acquisition_id, operation_id, actor_id,
                source_device_id, source_display_name, source_vendor, source_model,
                source_serial, source_media_type, source_capacity_bytes, source_sector_size,
                source_bus_type, source_snapshot_json, destination_path, image_format,
                image_size_bytes, image_sha256, status, bytes_acquired, elapsed_seconds,
                failure_reason, audit_reference, started_at, completed_at
            ) VALUES (
                'acq-crashed', 'op-crashed-01', 'operator',
                '\\\\.\\PhysicalDrive1', 'Test Disk', 'LocardX', 'Test',
                'TEST-SER', 'SSD', 1048576, 512,
                'SATA', ?1, 'C:\\crashed.dd', 'raw',
                524288, '', 'Acquiring', 524288, 2.5,
                NULL, 'AUDIT_CRASH', ?2, ?2
            )",
            rusqlite::params![snapshot_json, now],
        )?;
        Ok(())
    })
    .unwrap();

    // Startup recovery sweep
    let recovered_count = service.recover_interrupted_acquisitions().unwrap();
    assert_eq!(recovered_count, 1);

    // Incomplete record must be marked Failed, never Completed
    let recovered_record = service
        .get_acquisition_result("op-crashed-01")
        .unwrap()
        .unwrap();
    assert_eq!(recovered_record.status, AcquisitionStatus::Failed);
    assert!(matches!(
        recovered_record.failure_reason,
        Some(AcquisitionFailureReason::Interrupted)
    ));
}
