use locardx_recovery_engine::confidence::factors::{
    factor_contiguity, factor_filesystem_metadata, factor_footer_match, factor_magic_header,
    factor_structure_validation,
};
use locardx_recovery_engine::confidence::score::calculate_confidence;
use locardx_recovery_engine::models::ConfidenceGrade;

#[test]
fn test_high_confidence_scoring() {
    let factors = vec![
        factor_magic_header(true),            // +20
        factor_footer_match(true),            // +20
        factor_structure_validation("valid"), // +25
        factor_filesystem_metadata(true),     // +15
        factor_contiguity(true),              // +10
    ];

    let (score, grade) = calculate_confidence(&factors);
    assert_eq!(score, 90);
    assert_eq!(grade, ConfidenceGrade::High);
}

#[test]
fn test_low_and_uncertain_confidence_scoring() {
    let factors = vec![
        factor_magic_header(true),                // +20
        factor_footer_match(false),               // -20
        factor_structure_validation("corrupted"), // -35
        factor_filesystem_metadata(false),        // 0
        factor_contiguity(false),                 // -10
    ];

    let (score, grade) = calculate_confidence(&factors);
    assert_eq!(score, 0); // clamped to 0
    assert_eq!(grade, ConfidenceGrade::Uncertain);

    let medium_factors = vec![
        factor_magic_header(true),            // +20
        factor_footer_match(true),            // +20
        factor_structure_validation("valid"), // +25
        factor_filesystem_metadata(false),    // 0
        factor_contiguity(true),              // +10
    ];
    let (score_med, grade_med) = calculate_confidence(&medium_factors);
    assert_eq!(score_med, 75);
    assert_eq!(grade_med, ConfidenceGrade::Medium);
}
