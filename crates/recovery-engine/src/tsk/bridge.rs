use super::filesystem::{
    inspect_partition_filesystem_guided, DiscoveredMetadataFile, TskProgressUpdate,
};
use super::partition::{scan_partitions, PartitionInfo};
use std::io::{Read, Seek};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// High-level result of partition and filesystem forensic analysis.
#[derive(Debug, Clone, Default)]
pub struct TskInspectionResult {
    pub partitions: Vec<PartitionInfo>,
    pub metadata_files: Vec<DiscoveredMetadataFile>,
    pub detected_filesystems: Vec<String>,
    pub total_entries_examined: usize,
    pub analysis_error: Option<String>,
}

/// Orchestrates partition discovery and filesystem metadata recovery.
/// Operates strictly read-only and resiliently falls back if filesystems are absent.
pub fn run_tsk_inspection<R: Read + Seek>(reader: &mut R, total_bytes: u64) -> TskInspectionResult {
    run_tsk_inspection_guided(reader, total_bytes, None, &mut |_| {})
}

/// Orchestrates partition discovery and filesystem metadata recovery with real-time telemetry.
pub fn run_tsk_inspection_guided<R: Read + Seek, P: FnMut(TskProgressUpdate)>(
    reader: &mut R,
    total_bytes: u64,
    cancellation_token: Option<&Arc<AtomicBool>>,
    progress_cb: &mut P,
) -> TskInspectionResult {
    let partitions = scan_partitions(reader, total_bytes);
    let mut metadata_files = Vec::new();
    let mut detected_filesystems = Vec::new();
    let mut total_entries_examined = 0;
    let mut analysis_error = None;

    for partition in &partitions {
        if let Some(token) = cancellation_token {
            if token.load(Ordering::SeqCst) {
                break;
            }
        }

        let report = inspect_partition_filesystem_guided(
            reader,
            partition.start_byte_offset,
            partition.size_bytes,
            partition.filesystem_hint.as_deref(),
            progress_cb,
        );

        if let Some(fs) = report.filesystem {
            if !detected_filesystems.contains(&fs) {
                detected_filesystems.push(fs);
            }
        }

        total_entries_examined += report.entries_examined;
        metadata_files.extend(report.files);

        if report.error.is_some() && analysis_error.is_none() {
            analysis_error = report.error;
        }
    }

    TskInspectionResult {
        partitions,
        metadata_files,
        detected_filesystems,
        total_entries_examined,
        analysis_error,
    }
}
