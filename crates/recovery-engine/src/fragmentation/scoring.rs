use super::fragment::is_cluster_aligned;

/// Computes a confidence score for candidate fragment linkage between 0.0 and 1.0.
pub fn score_fragment_link(
    first_offset: u64,
    first_len: u64,
    second_offset: u64,
    cluster_size: u64,
) -> f64 {
    let mut score: f64 = 0.5;

    // Both fragments aligning on cluster boundaries adds confidence
    if is_cluster_aligned(first_offset, cluster_size) {
        score += 0.2;
    }
    if is_cluster_aligned(second_offset, cluster_size) {
        score += 0.2;
    }

    // Closer proximity gives higher baseline probability
    let gap = if second_offset >= first_offset + first_len {
        second_offset - (first_offset + first_len)
    } else {
        (first_offset + first_len) - second_offset
    };

    if gap == 0 {
        // Perfectly contiguous
        return 1.0;
    } else if gap < 64 * 1024 {
        score += 0.1;
    } else if gap > 10 * 1024 * 1024 {
        score -= 0.2;
    }

    score.clamp(0.0, 1.0)
}
