use locardx_acquisition::{AcquisitionArtifact, AcquisitionDeviceSnapshot};
use locardx_recovery_engine::models::RecoveryFailureReason;
use locardx_recovery_engine::source::{compute_streaming_sha256, validate_acquisition_artifact};
use std::io::Write;
use tempfile::NamedTempFile;

fn create_mock_snapshot() -> AcquisitionDeviceSnapshot {
    AcquisitionDeviceSnapshot {
        device_id: "disk-mock-1".to_string(),
        display_name: "Mock Disk".to_string(),
        vendor: Some("LocardX".to_string()),
        model: Some("Forensic Virtual Disk".to_string()),
        serial_number: Some("SN-12345".to_string()),
        media_type: "HDD".to_string(),
        capacity_bytes: 1024 * 1024,
        sector_size: 512,
        bus_type: Some("USB".to_string()),
        is_removable: true,
        is_system: false,
        snapshot_timestamp: "2026-09-12T00:00:00Z".to_string(),
    }
}

#[test]
fn test_validate_acquisition_artifact_success() {
    let mut temp = NamedTempFile::new().unwrap();
    let content = vec![0xAB; 64 * 1024];
    temp.write_all(&content).unwrap();

    let computed_hash = compute_streaming_sha256(temp.path()).unwrap();

    let artifact = AcquisitionArtifact {
        acquisition_id: "acq-test-1".to_string(),
        image_path: temp.path().to_string_lossy().to_string(),
        image_format: "raw".to_string(),
        image_size_bytes: content.len() as u64,
        image_sha256: computed_hash.clone(),
        source_device_snapshot: create_mock_snapshot(),
        acquisition_timestamp: "2026-09-12T00:00:00Z".to_string(),
        is_verified: true,
        audit_reference: "audit-1".to_string(),
    };

    let snapshot = validate_acquisition_artifact(&artifact).expect("Validation should succeed");
    assert_eq!(snapshot.acquisition_id, "acq-test-1");
    assert_eq!(snapshot.image_sha256, computed_hash);
    assert!(snapshot.is_trusted);
}

#[test]
fn test_validate_missing_image_fails_closed() {
    let artifact = AcquisitionArtifact {
        acquisition_id: "acq-missing".to_string(),
        image_path: "C:\\path\\does\\not\\exist.raw".to_string(),
        image_format: "raw".to_string(),
        image_size_bytes: 1024,
        image_sha256: "0123456789abcdef".to_string(),
        source_device_snapshot: create_mock_snapshot(),
        acquisition_timestamp: "2026-09-12T00:00:00Z".to_string(),
        is_verified: true,
        audit_reference: "audit-missing".to_string(),
    };

    match validate_acquisition_artifact(&artifact) {
        Err(RecoveryFailureReason::ImageNotFound(_)) => {}
        other => panic!("Expected ImageNotFound, got {:?}", other),
    }
}

#[test]
fn test_validate_invalid_format_fails_closed() {
    let artifact = AcquisitionArtifact {
        acquisition_id: "acq-fmt".to_string(),
        image_path: "some_image.vmdk".to_string(),
        image_format: "vmdk".to_string(),
        image_size_bytes: 1024,
        image_sha256: "abcd".to_string(),
        source_device_snapshot: create_mock_snapshot(),
        acquisition_timestamp: "2026-09-12T00:00:00Z".to_string(),
        is_verified: true,
        audit_reference: "audit-fmt".to_string(),
    };

    match validate_acquisition_artifact(&artifact) {
        Err(RecoveryFailureReason::InvalidFormat(_)) => {}
        other => panic!("Expected InvalidFormat, got {:?}", other),
    }
}

#[test]
fn test_validate_size_mismatch_fails_closed() {
    let mut temp = NamedTempFile::new().unwrap();
    temp.write_all(&[1, 2, 3, 4, 5]).unwrap();

    let artifact = AcquisitionArtifact {
        acquisition_id: "acq-size".to_string(),
        image_path: temp.path().to_string_lossy().to_string(),
        image_format: "raw".to_string(),
        image_size_bytes: 9999, // Mismatched size
        image_sha256: "abcd".to_string(),
        source_device_snapshot: create_mock_snapshot(),
        acquisition_timestamp: "2026-09-12T00:00:00Z".to_string(),
        is_verified: true,
        audit_reference: "audit-size".to_string(),
    };

    match validate_acquisition_artifact(&artifact) {
        Err(RecoveryFailureReason::SizeMismatch { expected, actual }) => {
            assert_eq!(expected, 9999);
            assert_eq!(actual, 5);
        }
        other => panic!("Expected SizeMismatch, got {:?}", other),
    }
}

#[test]
fn test_validate_hash_mismatch_tampering_fails_closed() {
    let mut temp = NamedTempFile::new().unwrap();
    temp.write_all(b"original evidence data").unwrap();

    let artifact = AcquisitionArtifact {
        acquisition_id: "acq-tampered".to_string(),
        image_path: temp.path().to_string_lossy().to_string(),
        image_format: "dd".to_string(),
        image_size_bytes: 22,
        image_sha256: "0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        source_device_snapshot: create_mock_snapshot(),
        acquisition_timestamp: "2026-09-12T00:00:00Z".to_string(),
        is_verified: true,
        audit_reference: "audit-tampered".to_string(),
    };

    match validate_acquisition_artifact(&artifact) {
        Err(RecoveryFailureReason::HashMismatch { .. }) => {}
        other => panic!("Expected HashMismatch, got {:?}", other),
    }
}

#[test]
fn test_validate_unverified_acquisition_fails_closed() {
    let artifact = AcquisitionArtifact {
        acquisition_id: "acq-unverified".to_string(),
        image_path: "test.raw".to_string(),
        image_format: "raw".to_string(),
        image_size_bytes: 100,
        image_sha256: "abc".to_string(),
        source_device_snapshot: create_mock_snapshot(),
        acquisition_timestamp: "2026-09-12T00:00:00Z".to_string(),
        is_verified: false, // Unverified
        audit_reference: "audit-unverified".to_string(),
    };

    match validate_acquisition_artifact(&artifact) {
        Err(RecoveryFailureReason::IncompleteAcquisition(_)) => {}
        other => panic!("Expected IncompleteAcquisition, got {:?}", other),
    }
}
