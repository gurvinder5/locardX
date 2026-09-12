use crate::models::{ConfidenceGrade, EvidenceFactor};

/// Computes the final deterministic confidence score and qualitative grade from evidence factors.
pub fn calculate_confidence(factors: &[EvidenceFactor]) -> (u32, ConfidenceGrade) {
    let mut raw_score: i32 = 0;

    for factor in factors {
        if factor.passed {
            if factor.weight > 0 {
                raw_score += factor.weight;
            }
        } else if factor.weight < 0 {
            raw_score += factor.weight;
        }
    }

    let score = raw_score.clamp(0, 100) as u32;
    let grade = ConfidenceGrade::from_score(score);
    (score, grade)
}
