use crate::models::EvidenceFactor;

/// Formulates standard evidence factors for scoring file recovery authenticity.
pub fn factor_magic_header(passed: bool) -> EvidenceFactor {
    if passed {
        EvidenceFactor {
            factor_type: "HEADER_MAGIC_MATCH".to_string(),
            weight: 20,
            description: "File format magic signature verified at origin offset".to_string(),
            passed: true,
        }
    } else {
        EvidenceFactor {
            factor_type: "HEADER_MAGIC_MATCH".to_string(),
            weight: -50,
            description: "Missing or corrupted header magic signature".to_string(),
            passed: false,
        }
    }
}

pub fn factor_footer_match(passed: bool) -> EvidenceFactor {
    if passed {
        EvidenceFactor {
            factor_type: "FOOTER_MATCH".to_string(),
            weight: 20,
            description: "Format trailer / EOF delimiter verified".to_string(),
            passed: true,
        }
    } else {
        EvidenceFactor {
            factor_type: "FOOTER_MATCH".to_string(),
            weight: -20,
            description: "Missing or premature trailer marker".to_string(),
            passed: false,
        }
    }
}

pub fn factor_structure_validation(status_grade: &str) -> EvidenceFactor {
    match status_grade {
        "valid" => EvidenceFactor {
            factor_type: "STRUCTURE_VALIDATION".to_string(),
            weight: 25,
            description: "Complete internal structure and chunk hierarchy verified".to_string(),
            passed: true,
        },
        "partially_valid" => EvidenceFactor {
            factor_type: "STRUCTURE_VALIDATION".to_string(),
            weight: 10,
            description: "Partial internal structure verified with non-fatal warnings".to_string(),
            passed: true,
        },
        "incomplete" => EvidenceFactor {
            factor_type: "STRUCTURE_VALIDATION".to_string(),
            weight: -20,
            description: "Truncated payload missing essential data segments".to_string(),
            passed: false,
        },
        _ => EvidenceFactor {
            factor_type: "STRUCTURE_VALIDATION".to_string(),
            weight: -35,
            description: "Corrupted structure failed parser validation".to_string(),
            passed: false,
        },
    }
}

pub fn factor_filesystem_metadata(present: bool) -> EvidenceFactor {
    if present {
        EvidenceFactor {
            factor_type: "FILESYSTEM_METADATA".to_string(),
            weight: 15,
            description: "Matched against filesystem directory/MFT table entry".to_string(),
            passed: true,
        }
    } else {
        EvidenceFactor {
            factor_type: "FILESYSTEM_METADATA".to_string(),
            weight: 0,
            description: "Pure carving discovery without filesystem entry correlation".to_string(),
            passed: false,
        }
    }
}

pub fn factor_contiguity(is_contiguous: bool) -> EvidenceFactor {
    if is_contiguous {
        EvidenceFactor {
            factor_type: "CLUSTER_CONTIGUOUS".to_string(),
            weight: 10,
            description: "Continuous sequential cluster run without gaps".to_string(),
            passed: true,
        }
    } else {
        EvidenceFactor {
            factor_type: "CLUSTER_CONTIGUOUS".to_string(),
            weight: -10,
            description: "Discontinuous clusters requiring reconstruction".to_string(),
            passed: false,
        }
    }
}
