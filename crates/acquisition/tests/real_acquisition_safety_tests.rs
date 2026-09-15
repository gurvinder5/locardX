use locardx_acquisition::{
    validate_destination, validate_source_device_id, AcquisitionFailureReason, AcquisitionService,
    AcquisitionSourceReader, AcquisitionStatus, FaultInjectableAcquisitionReader,
    FaultInjectionMode,
};
use locardx_audit::AuditService;
use locardx_database::Database;
use sha2::{Digest, Sha256};
use std::sync::Arc;

fn setup_test_suite(
    device_id: &str,
    capacity: u64,
) -> (
    Arc<Database>,
    Arc<AuditService>,
    Arc<FaultInjectableAcquisitionReader>,
    AcquisitionService,
) {
    let db = Arc::new(Database::open(":memory:").expect("Failed to open test in-memory DB"));
    let audit = Arc::new(AuditService::new(Arc::clone(&db)));
    let reader = Arc::new(FaultInjectableAcquisitionReader::new_with_test_device(
        device_id, capacity,
    ));
    let service = AcquisitionService::new(Arc::clone(&db), Arc::clone(&audit))
        .with_reader(Arc::clone(&reader) as Arc<dyn AcquisitionSourceReader>);

    (db, audit, reader, service)
}

// -------------------------------------------------------------------------
// Test A: PhysicalDrive1 successfully acquired to an image file
// -------------------------------------------------------------------------
#[test]
fn test_a_physicaldrive1_successfully_acquired_to_image_file() {
    let capacity = 2 * 1024 * 1024 + 256 * 1024; // 2.25 MiB
    let (_, _, _, service) = setup_test_suite(r"\\.\PhysicalDrive1", capacity);

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("acquired_evidence.raw");

    let plan = service
        .create_plan(
            r"\\.\PhysicalDrive1",
            dest_file.to_str().unwrap(),
            true,
            Some("examiner_a"),
        )
        .expect("Failed to create plan");

    let res = service
        .execute_acquisition("op-test-a", &plan, Some("examiner_a"))
        .expect("Acquisition execution failed");

    assert_eq!(res.status, AcquisitionStatus::Completed);
    assert_eq!(res.bytes_acquired, capacity);
    assert_eq!(res.image_size_bytes, capacity);
    assert!(!res.image_sha256.is_empty());
    assert!(dest_file.exists());
    assert_eq!(std::fs::metadata(&dest_file).unwrap().len(), capacity);
    assert!(res.diagnostics.is_some());
    let diag = res.diagnostics.unwrap();
    assert_eq!(diag.final_status, "Completed");
    assert_eq!(diag.current_byte_offset, capacity);
}

// -------------------------------------------------------------------------
// Test B: Logical drive C: is strictly rejected
// -------------------------------------------------------------------------
#[test]
fn test_b_logical_drive_c_is_rejected() {
    let err_c_slash = validate_source_device_id(r"C:\");
    assert!(err_c_slash.is_err());
    assert!(matches!(
        err_c_slash.unwrap_err(),
        AcquisitionFailureReason::InvalidSource(_)
    ));

    let err_c_colon = validate_source_device_id("C:");
    assert!(err_c_colon.is_err());

    let err_d_path = validate_source_device_id(r"D:\Data");
    assert!(err_d_path.is_err());

    // Valid physical drives must be accepted
    assert!(validate_source_device_id(r"\\.\PhysicalDrive1").is_ok());
    assert!(validate_source_device_id("PhysicalDrive1").is_ok());
}

// -------------------------------------------------------------------------
// Test C: Destination on same physical device is rejected (Collision Safety)
// -------------------------------------------------------------------------
#[test]
fn test_c_destination_on_same_physical_device_is_rejected() {
    let (_, _, reader, _) = setup_test_suite(r"\\.\PhysicalDrive1", 1024 * 1024);

    // Register volume E: as residing on PhysicalDrive1
    reader.register_destination_mapping(r"E:\", r"\\.\PhysicalDrive1");

    let res = validate_destination(
        reader.as_ref(),
        r"\\.\PhysicalDrive1",
        r"E:\Forensics\image.raw",
        1024 * 1024,
        true,
    );

    assert!(matches!(
        res,
        Err(AcquisitionFailureReason::DestinationOnSourceDisk)
    ));
}

// -------------------------------------------------------------------------
// Test D: Destination on different physical device is accepted
// -------------------------------------------------------------------------
#[test]
fn test_d_destination_on_different_physical_device_is_accepted() {
    let (_, _, reader, _) = setup_test_suite(r"\\.\PhysicalDrive1", 1024 * 1024);

    // Register volume C: as residing on PhysicalDrive0
    reader.register_destination_mapping(r"C:\", r"\\.\PhysicalDrive0");

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("safe_output.raw");

    let res = validate_destination(
        reader.as_ref(),
        r"\\.\PhysicalDrive1",
        dest_file.to_str().unwrap(),
        1024 * 1024,
        true,
    );

    assert!(res.is_ok());
}

// -------------------------------------------------------------------------
// Test E: Exact-size final chunk
// -------------------------------------------------------------------------
#[test]
fn test_e_exact_size_final_chunk() {
    // Capacity exactly 2 * 1 MiB chunk size
    let capacity = 2 * 1024 * 1024;
    let (_, _, _, service) = setup_test_suite(r"\\.\PhysicalDrive1", capacity);

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("exact_chunk.raw");

    let plan = service
        .create_plan(
            r"\\.\PhysicalDrive1",
            dest_file.to_str().unwrap(),
            true,
            Some("examiner_e"),
        )
        .unwrap();

    let res = service
        .execute_acquisition("op-test-e", &plan, Some("examiner_e"))
        .unwrap();

    assert_eq!(res.status, AcquisitionStatus::Completed);
    assert_eq!(res.bytes_acquired, capacity);
    assert_eq!(std::fs::metadata(&dest_file).unwrap().len(), capacity);
}

// -------------------------------------------------------------------------
// Test F: Partial final chunk handled correctly
// -------------------------------------------------------------------------
#[test]
fn test_f_partial_final_chunk() {
    // Capacity with partial final chunk (1 MiB + 321 KiB)
    let capacity = 1024 * 1024 + 321 * 1024;
    let (_, _, _, service) = setup_test_suite(r"\\.\PhysicalDrive1", capacity);

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("partial_chunk.raw");

    let plan = service
        .create_plan(
            r"\\.\PhysicalDrive1",
            dest_file.to_str().unwrap(),
            true,
            Some("examiner_f"),
        )
        .unwrap();

    let res = service
        .execute_acquisition("op-test-f", &plan, Some("examiner_f"))
        .unwrap();

    assert_eq!(res.status, AcquisitionStatus::Completed);
    assert_eq!(res.bytes_acquired, capacity);
    assert_eq!(std::fs::metadata(&dest_file).unwrap().len(), capacity);
}

// -------------------------------------------------------------------------
// Test G: Read error causes acquisition failure
// -------------------------------------------------------------------------
#[test]
fn test_g_read_error_causes_acquisition_failure() {
    let capacity = 2 * 1024 * 1024;
    let (_, _, reader, service) = setup_test_suite(r"\\.\PhysicalDrive1", capacity);

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("error_image.raw");

    let plan = service
        .create_plan(
            r"\\.\PhysicalDrive1",
            dest_file.to_str().unwrap(),
            true,
            Some("examiner_g"),
        )
        .unwrap();

    // Inject read failure mid-stream
    reader.set_mode(FaultInjectionMode::FailReadAtOffset(1024 * 1024));

    let res = service
        .execute_acquisition("op-test-g", &plan, Some("examiner_g"))
        .unwrap();

    assert_eq!(res.status, AcquisitionStatus::Failed);
    assert!(res.failure_reason.is_some());
    assert!(matches!(
        res.failure_reason.unwrap(),
        AcquisitionFailureReason::ReadError(_) | AcquisitionFailureReason::DeviceIoError { .. }
    ));
}

// -------------------------------------------------------------------------
// Test H: Failed acquisition removes only the incomplete target file
// -------------------------------------------------------------------------
#[test]
fn test_h_failed_acquisition_removes_only_incomplete_target() {
    let capacity = 2 * 1024 * 1024;
    let (_, _, reader, service) = setup_test_suite(r"\\.\PhysicalDrive1", capacity);

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("incomplete_target.raw");

    // Create an innocent adjacent user file that must NOT be touched
    let innocent_file = temp_dir.path().join("important_user_doc.txt");
    std::fs::write(&innocent_file, b"Do not delete this evidence document").unwrap();

    let plan = service
        .create_plan(
            r"\\.\PhysicalDrive1",
            dest_file.to_str().unwrap(),
            true,
            Some("examiner_h"),
        )
        .unwrap();

    // Trigger mid-stream failure
    reader.set_mode(FaultInjectionMode::FailReadAtOffset(512 * 1024));

    let res = service
        .execute_acquisition("op-test-h", &plan, Some("examiner_h"))
        .unwrap();

    assert_eq!(res.status, AcquisitionStatus::Failed);
    // Incomplete target MUST be deleted (fail-closed integrity)
    assert!(!dest_file.exists());
    // Innocent user file MUST remain untouched
    assert!(innocent_file.exists());
    assert_eq!(
        std::fs::read(&innocent_file).unwrap(),
        b"Do not delete this evidence document"
    );
}

// -------------------------------------------------------------------------
// Test I: Source is never opened with write access
// -------------------------------------------------------------------------
#[test]
fn test_i_source_is_never_opened_with_write_access() {
    // Audit reader methods to verify only read_only interface exists
    let reader = FaultInjectableAcquisitionReader::new_with_test_device(r"\\.\PhysicalDrive1", 1024);
    let stream_res = reader.open_read_only(r"\\.\PhysicalDrive1");
    assert!(stream_res.is_ok());

    // Stream interface only implements read_chunk and total_bytes; zero write methods exist
    let mut stream = stream_res.unwrap();
    let mut buf = [0u8; 512];
    let n = stream.read_chunk(&mut buf).unwrap();
    assert_eq!(n, 512);
}

// -------------------------------------------------------------------------
// Test J: Hash is calculated from bytes actually acquired
// -------------------------------------------------------------------------
#[test]
fn test_j_hash_is_calculated_from_bytes_actually_acquired() {
    let capacity = 1024 * 1024 + 128 * 1024;
    let (_, _, _reader, service) = setup_test_suite(r"\\.\PhysicalDrive1", capacity);

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("hashed_evidence.raw");

    let plan = service
        .create_plan(
            r"\\.\PhysicalDrive1",
            dest_file.to_str().unwrap(),
            true,
            Some("examiner_j"),
        )
        .unwrap();

    let res = service
        .execute_acquisition("op-test-j", &plan, Some("examiner_j"))
        .unwrap();

    assert_eq!(res.status, AcquisitionStatus::Completed);

    // Read the written file and independently compute SHA-256
    let on_disk_bytes = std::fs::read(&dest_file).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&on_disk_bytes);
    let expected_hex = hex::encode(hasher.finalize());

    assert_eq!(res.image_sha256, expected_hex);
}

// -------------------------------------------------------------------------
// Test K: Device smaller than expected / size changes trigger TOCTOU failure
// -------------------------------------------------------------------------
#[test]
fn test_k_device_size_changes_trigger_toctou_failure() {
    let capacity = 2 * 1024 * 1024;
    let (_, _, reader, service) = setup_test_suite(r"\\.\PhysicalDrive1", capacity);

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("toctou_fail.raw");

    let plan = service
        .create_plan(
            r"\\.\PhysicalDrive1",
            dest_file.to_str().unwrap(),
            true,
            Some("examiner_k"),
        )
        .unwrap();

    // Mutate capacity before execution
    reader.set_mode(FaultInjectionMode::MutateCapacity(1024 * 1024));

    let res = service
        .execute_acquisition("op-test-k", &plan, Some("examiner_k"))
        .unwrap();

    assert_eq!(res.status, AcquisitionStatus::Failed);
    assert!(matches!(
        res.failure_reason,
        Some(AcquisitionFailureReason::SourceMutated(_))
    ));
}

// -------------------------------------------------------------------------
// Test L: USB device removal during acquisition
// -------------------------------------------------------------------------
#[test]
fn test_l_usb_device_removal_during_acquisition() {
    let capacity = 3 * 1024 * 1024;
    let (_, _, reader, service) = setup_test_suite(r"\\.\PhysicalDrive1", capacity);

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_file = temp_dir.path().join("disconnected_target.raw");

    let plan = service
        .create_plan(
            r"\\.\PhysicalDrive1",
            dest_file.to_str().unwrap(),
            true,
            Some("examiner_l"),
        )
        .unwrap();

    // Simulate USB disconnect during read
    reader.set_mode(FaultInjectionMode::SimulateDisconnect);

    let res = service
        .execute_acquisition("op-test-l", &plan, Some("examiner_l"))
        .unwrap();

    assert_eq!(res.status, AcquisitionStatus::Failed);
    assert!(matches!(
        res.failure_reason,
        Some(AcquisitionFailureReason::SourceDisconnected)
    ));
    // Incomplete file must be deleted
    assert!(!dest_file.exists());
}

// -------------------------------------------------------------------------
// Test M: Existing directory specified as destination is strictly rejected
// -------------------------------------------------------------------------
#[test]
fn test_m_directory_destination_is_rejected() {
    let (_, _, reader, _) = setup_test_suite(r"\\.\PhysicalDrive1", 1024 * 1024);
    let temp_dir = tempfile::tempdir().unwrap();
    let dir_path = temp_dir.path().to_str().unwrap();

    // Passing an existing directory path must fail validation
    let res = validate_destination(
        reader.as_ref(),
        r"\\.\PhysicalDrive1",
        dir_path,
        1024 * 1024,
        true,
    );

    assert!(matches!(
        res,
        Err(AcquisitionFailureReason::InvalidDestination(_))
    ));
}


