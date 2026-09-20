use locardx_recovery_engine::fragmentation::fragment::{
    create_fragment, evaluate_contiguity, is_cluster_aligned,
};
use locardx_recovery_engine::fragmentation::reconstruction::reconstruct_fragments;
use locardx_recovery_engine::fragmentation::scoring::score_fragment_link;
use locardx_recovery_engine::models::{
    FileType, FragmentStatus, FragmentType, ReconstructionResult, ValidationStatus,
};

#[test]
fn test_contiguity_and_alignment() {
    assert!(is_cluster_aligned(0, 4096));
    assert!(is_cluster_aligned(4096, 4096));
    assert!(is_cluster_aligned(8192, 4096));
    assert!(!is_cluster_aligned(512, 4096));

    let frag1 = create_fragment("file-1", 0, 4096, 4096, FragmentType::Header, 0.9);
    assert_eq!(
        evaluate_contiguity(&[frag1.clone()]),
        FragmentStatus::Contiguous
    );

    let frag2 = create_fragment("file-1", 1, 8192, 4096, FragmentType::Interior, 0.9);
    assert_eq!(
        evaluate_contiguity(&[frag1, frag2]),
        FragmentStatus::PotentiallyFragmented
    );
}

#[test]
fn test_fragment_scoring_metrics() {
    let score_exact = score_fragment_link(4096, 4096, 8192, 4096);
    assert_eq!(score_exact, 1.0);

    let score_gap = score_fragment_link(4096, 4096, 16384, 4096);
    assert!(score_gap > 0.5 && score_gap <= 1.0);
}

#[test]
fn test_reconstruct_consecutive_fragments() {
    let mut part1 = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG Header
    part1.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
    part1.extend_from_slice(b"IHDR");
    part1.extend_from_slice(&[0; 13]);
    part1.extend_from_slice(&[0; 4]);
    // IDAT chunk
    part1.extend_from_slice(&[0x00, 0x00, 0x00, 0x04]);
    part1.extend_from_slice(b"IDAT");
    part1.extend_from_slice(&[1, 2, 3, 4]);
    part1.extend_from_slice(&[0; 4]);

    let mut part2 = vec![0x00, 0x00, 0x00, 0x00];
    part2.extend_from_slice(b"IEND");
    part2.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);

    let f1 = create_fragment(
        "file-png",
        0,
        0,
        part1.len() as u64,
        FragmentType::Header,
        0.95,
    );
    let f2 = create_fragment(
        "file-png",
        1,
        part1.len() as u64,
        part2.len() as u64,
        FragmentType::Trailer,
        0.95,
    );

    let outcome = reconstruct_fragments(&FileType::Png, vec![f1, f2], vec![part1, part2]);
    assert_eq!(outcome.result, ReconstructionResult::NotRequired);
    assert_eq!(outcome.validation_status, ValidationStatus::Valid);
    assert!(!outcome.assembled_bytes.is_empty());
}

#[test]
fn test_reconstruction_incomplete_without_fabrication() {
    // Fragment without trailer marker
    let mut part1 = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    part1.extend_from_slice(b"incomplete data chunk");

    let f1 = create_fragment(
        "file-inc",
        0,
        0,
        part1.len() as u64,
        FragmentType::Header,
        0.7,
    );
    let outcome = reconstruct_fragments(&FileType::Png, vec![f1], vec![part1]);
    assert_eq!(outcome.validation_status, ValidationStatus::Corrupted);
}
