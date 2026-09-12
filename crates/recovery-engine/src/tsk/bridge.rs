use super::filesystem::{inspect_partition_filesystem, DiscoveredMetadataFile};
use super::partition::{scan_partitions, PartitionInfo};
use std::io::{Read, Seek};

/// High-level result of partition and filesystem forensic analysis.
#[derive(Debug, Clone, Default)]
pub struct TskInspectionResult {
    pub partitions: Vec<PartitionInfo>,
    pub metadata_files: Vec<DiscoveredMetadataFile>,
}

/// Orchestrates partition discovery and filesystem metadata recovery.
/// Operates strictly read-only and resiliently falls back if filesystems are absent.
pub fn run_tsk_inspection<R: Read + Seek>(reader: &mut R, total_bytes: u64) -> TskInspectionResult {
    let partitions = scan_partitions(reader, total_bytes);
    let mut metadata_files = Vec::new();

    for partition in &partitions {
        let files = inspect_partition_filesystem(
            reader,
            partition.start_byte_offset,
            partition.size_bytes,
            partition.filesystem_hint.as_deref(),
        );
        metadata_files.extend(files);
    }

    TskInspectionResult {
        partitions,
        metadata_files,
    }
}
