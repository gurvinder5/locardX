use crate::models::{FileFragment, FileType, ReconstructionResult, ValidationStatus};
use crate::structure::validator::validate_file_candidate;

/// Result of evaluating and assembling fragments for a file candidate.
#[derive(Debug, Clone)]
pub struct ReconstructionOutcome {
    pub result: ReconstructionResult,
    pub assembled_bytes: Vec<u8>,
    pub validation_status: ValidationStatus,
    pub fragments: Vec<FileFragment>,
}

/// Evaluates fragments for a file candidate.
/// Strictly enforces evidence-based reconstruction: never fabricates missing bytes.
pub fn reconstruct_fragments(
    file_type: &FileType,
    fragments: Vec<FileFragment>,
    raw_fragments_data: Vec<Vec<u8>>,
) -> ReconstructionOutcome {
    if fragments.is_empty() || raw_fragments_data.is_empty() {
        return ReconstructionOutcome {
            result: ReconstructionResult::Failed,
            assembled_bytes: Vec::new(),
            validation_status: ValidationStatus::Invalid,
            fragments,
        };
    }

    if fragments.len() == 1 {
        let bytes = raw_fragments_data[0].clone();
        let (status, _) = validate_file_candidate(file_type, &bytes);
        return ReconstructionOutcome {
            result: ReconstructionResult::NotRequired,
            assembled_bytes: bytes,
            validation_status: status,
            fragments,
        };
    }

    // Multiple fragments: check if consecutive or scattered
    let mut is_consecutive = true;
    for i in 0..fragments.len().saturating_sub(1) {
        if fragments[i].source_offset + fragments[i].size_bytes != fragments[i + 1].source_offset {
            is_consecutive = false;
            break;
        }
    }

    let mut assembled = Vec::new();
    for chunk in raw_fragments_data {
        assembled.extend_from_slice(&chunk);
    }

    let (status, _) = validate_file_candidate(file_type, &assembled);

    let result = match status {
        ValidationStatus::Valid => {
            if is_consecutive {
                ReconstructionResult::NotRequired
            } else {
                ReconstructionResult::Reconstructed
            }
        }
        ValidationStatus::PartiallyValid => ReconstructionResult::Uncertain,
        ValidationStatus::Incomplete => ReconstructionResult::Uncertain,
        ValidationStatus::Corrupted | ValidationStatus::Invalid => ReconstructionResult::Failed,
    };

    ReconstructionOutcome {
        result,
        assembled_bytes: assembled,
        validation_status: status,
        fragments,
    }
}
