use super::models::SnapshotComparisonResult;
use locardx_security::{ReasonCode, TargetSnapshot};

/// Compares a pre-captured snapshot at plan time with a live snapshot at execution time.
/// Returns a detailed `SnapshotComparisonResult`.
pub fn compare_target_snapshots(
    plan_snapshot: &TargetSnapshot,
    current_snapshot: Option<&TargetSnapshot>,
) -> SnapshotComparisonResult {
    let current = match current_snapshot {
        Some(s) => s,
        None => {
            return SnapshotComparisonResult {
                matches: false,
                reason: Some("Target device is disconnected or no longer exists".to_string()),
                differences: vec!["Device was disconnected or unmounted".to_string()],
                plan_snapshot: plan_snapshot.clone(),
                current_snapshot: None,
            };
        }
    };

    let mut differences = Vec::new();

    if plan_snapshot.target_identifier != current.target_identifier {
        differences.push(format!(
            "Target identifier mismatch: expected '{}', got '{}'",
            plan_snapshot.target_identifier, current.target_identifier
        ));
    }

    if plan_snapshot.target_type != current.target_type {
        differences.push(format!(
            "Target type mismatch: expected '{:?}', got '{:?}'",
            plan_snapshot.target_type, current.target_type
        ));
    }

    if plan_snapshot.device_id != current.device_id {
        differences.push(format!(
            "Parent physical device ID mismatch: expected '{:?}', got '{:?}'",
            plan_snapshot.device_id, current.device_id
        ));
    }

    if plan_snapshot.capacity_bytes != current.capacity_bytes {
        differences.push(format!(
            "Capacity changed: expected {:?} bytes, got {:?} bytes",
            plan_snapshot.capacity_bytes, current.capacity_bytes
        ));
    }

    if plan_snapshot.is_system != current.is_system {
        differences.push(format!(
            "System status changed: expected is_system={}, got is_system={}",
            plan_snapshot.is_system, current.is_system
        ));
    }

    if plan_snapshot.is_boot != current.is_boot {
        differences.push(format!(
            "Boot status changed: expected is_boot={}, got is_boot={}",
            plan_snapshot.is_boot, current.is_boot
        ));
    }

    if plan_snapshot.classification != current.classification {
        differences.push(format!(
            "Classification changed: expected '{:?}', got '{:?}'",
            plan_snapshot.classification, current.classification
        ));
    }

    if plan_snapshot.filesystem != current.filesystem {
        differences.push(format!(
            "Filesystem changed: expected '{:?}', got '{:?}'",
            plan_snapshot.filesystem, current.filesystem
        ));
    }

    if differences.is_empty() {
        SnapshotComparisonResult {
            matches: true,
            reason: None,
            differences: Vec::new(),
            plan_snapshot: plan_snapshot.clone(),
            current_snapshot: Some(current.clone()),
        }
    } else {
        SnapshotComparisonResult {
            matches: false,
            reason: Some(
                "TargetChanged: target properties altered since plan evaluation".to_string(),
            ),
            differences,
            plan_snapshot: plan_snapshot.clone(),
            current_snapshot: Some(current.clone()),
        }
    }
}

/// Helper returning standard `Result<(), ReasonCode>` for TOCTOU gating.
pub fn verify_snapshot_integrity(
    plan_snapshot: &TargetSnapshot,
    current_snapshot: Option<&TargetSnapshot>,
) -> Result<(), ReasonCode> {
    let comparison = compare_target_snapshots(plan_snapshot, current_snapshot);
    if comparison.matches {
        Ok(())
    } else if comparison.current_snapshot.is_none() {
        Err(ReasonCode::InvalidTarget)
    } else {
        Err(ReasonCode::TargetChanged)
    }
}
