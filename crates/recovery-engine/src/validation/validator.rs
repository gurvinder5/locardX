use super::file_integrity::compute_candidate_sha256;
use crate::models::{EvidenceFactor, FileType, ValidationStatus};
use crate::signature::registry::SignatureRegistry;
use crate::structure::validator::validate_file_candidate;

/// Evaluates a candidate byte stream, generating validation status, SHA-256, and evidence factors.
pub fn evaluate_candidate(
    file_type: &FileType,
    data: &[u8],
    registry: &SignatureRegistry,
    has_filesystem_metadata: bool,
    is_contiguous: bool,
) -> (ValidationStatus, String, Vec<EvidenceFactor>) {
    let mut factors = Vec::new();

    // 1. Header Magic Match Factor
    let sig_opt = registry.all().iter().find(|s| &s.file_type == file_type);
    if let Some(sig) = sig_opt {
        if data.len() >= sig.header_magic.len()
            && &data[0..sig.header_magic.len()] == sig.header_magic.as_slice()
        {
            factors.push(EvidenceFactor {
                factor_type: "HEADER_MAGIC_MATCH".to_string(),
                weight: 20,
                description: format!("Validated magic signature header for {}", file_type),
                passed: true,
            });
        } else {
            factors.push(EvidenceFactor {
                factor_type: "HEADER_MAGIC_MATCH".to_string(),
                weight: -50,
                description: "Missing or corrupted magic signature header".to_string(),
                passed: false,
            });
        }

        // 2. Footer Match Factor
        if let Some(footer) = &sig.footer_magic {
            let footer_len = footer.len();
            if data.len() >= footer_len && &data[data.len() - footer_len..] == footer.as_slice() {
                factors.push(EvidenceFactor {
                    factor_type: "FOOTER_MATCH".to_string(),
                    weight: 20,
                    description: "Located valid EOF/trailer signature".to_string(),
                    passed: true,
                });
            } else {
                factors.push(EvidenceFactor {
                    factor_type: "FOOTER_MATCH".to_string(),
                    weight: -20,
                    description: "Missing or anomalous trailer marker".to_string(),
                    passed: false,
                });
            }
        }
    }

    // 3. Structure Parser Validation
    let (struct_status, struct_notes) = validate_file_candidate(file_type, data);
    match struct_status {
        ValidationStatus::Valid => {
            factors.push(EvidenceFactor {
                factor_type: "STRUCTURE_VALIDATION".to_string(),
                weight: 25,
                description: format!(
                    "Internal structure fully validated: {}",
                    struct_notes.join("; ")
                ),
                passed: true,
            });
        }
        ValidationStatus::PartiallyValid => {
            factors.push(EvidenceFactor {
                factor_type: "STRUCTURE_VALIDATION".to_string(),
                weight: 10,
                description: format!(
                    "Internal structure partially valid: {}",
                    struct_notes.join("; ")
                ),
                passed: true,
            });
        }
        ValidationStatus::Incomplete => {
            factors.push(EvidenceFactor {
                factor_type: "STRUCTURE_VALIDATION".to_string(),
                weight: -20,
                description: format!("Incomplete structure: {}", struct_notes.join("; ")),
                passed: false,
            });
        }
        ValidationStatus::Corrupted | ValidationStatus::Invalid => {
            factors.push(EvidenceFactor {
                factor_type: "STRUCTURE_VALIDATION".to_string(),
                weight: -35,
                description: format!(
                    "Structure parser rejected payload: {}",
                    struct_notes.join("; ")
                ),
                passed: false,
            });
        }
    }

    // 4. Filesystem Correlation
    if has_filesystem_metadata {
        factors.push(EvidenceFactor {
            factor_type: "FILESYSTEM_METADATA".to_string(),
            weight: 15,
            description: "Correlated with filesystem directory / MFT record".to_string(),
            passed: true,
        });
    }

    // 5. Contiguity
    if is_contiguous {
        factors.push(EvidenceFactor {
            factor_type: "CLUSTER_CONTIGUOUS".to_string(),
            weight: 10,
            description: "Contiguous physical cluster run confirmed".to_string(),
            passed: true,
        });
    }

    let sha256_hash = compute_candidate_sha256(data);
    (struct_status, sha256_hash, factors)
}
