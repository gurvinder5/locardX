use serde::{Deserialize, Serialize};
use std::fmt;

/// Role of a physical storage segment within a fragmented file candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FragmentType {
    Header,
    Interior,
    Trailer,
    Slack,
}

impl fmt::Display for FragmentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header => write!(f, "Header"),
            Self::Interior => write!(f, "Interior"),
            Self::Trailer => write!(f, "Trailer"),
            Self::Slack => write!(f, "Slack"),
        }
    }
}

/// A discrete cluster or sector segment belonging to a file candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileFragment {
    pub fragment_id: String,
    pub file_id: String,
    pub fragment_index: usize,
    pub source_offset: u64,
    pub size_bytes: u64,
    pub fragment_type: FragmentType,
    pub confidence: f64,
}

/// Contiguity assessment for a discovered candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FragmentStatus {
    Contiguous,
    PotentiallyFragmented,
}

/// Outcome of fragmented file reconstruction attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconstructionResult {
    NotRequired,
    Reconstructed,
    Uncertain,
    Failed,
}

impl fmt::Display for ReconstructionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRequired => write!(f, "Not Required (Contiguous)"),
            Self::Reconstructed => write!(f, "Successfully Reconstructed"),
            Self::Uncertain => write!(f, "Uncertain (Incomplete Fragments)"),
            Self::Failed => write!(f, "Reconstruction Failed"),
        }
    }
}
