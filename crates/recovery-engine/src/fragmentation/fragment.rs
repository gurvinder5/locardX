use crate::models::{FileFragment, FragmentStatus, FragmentType};
use uuid::Uuid;

pub const DEFAULT_CLUSTER_SIZE: u64 = 4096;

/// Evaluates whether a contiguous byte span aligns with file storage clusters.
pub fn is_cluster_aligned(offset: u64, cluster_size: u64) -> bool {
    offset % cluster_size == 0
}

/// Creates a new FileFragment descriptor.
pub fn create_fragment(
    file_id: &str,
    fragment_index: usize,
    source_offset: u64,
    size_bytes: u64,
    fragment_type: FragmentType,
    confidence: f64,
) -> FileFragment {
    FileFragment {
        fragment_id: format!("frag-{}", Uuid::new_v4()),
        file_id: file_id.to_string(),
        fragment_index,
        source_offset,
        size_bytes,
        fragment_type,
        confidence,
    }
}

/// Assesses contiguity status based on fragment count and gaps.
pub fn evaluate_contiguity(fragments: &[FileFragment]) -> FragmentStatus {
    if fragments.len() <= 1 {
        FragmentStatus::Contiguous
    } else {
        FragmentStatus::PotentiallyFragmented
    }
}
