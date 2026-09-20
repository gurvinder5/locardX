pub mod engine;
pub mod models;
pub mod policy;

pub use engine::SafetyEngine;
pub use models::{
    ConfirmationChallenge, ConfirmationRecord, ConfirmationStatus, ReasonCode, RiskLevel,
    SafetyDecision, SafetyDecisionOutcome, TargetSnapshot,
};
pub use policy::SafetyPolicy;

use locardx_common::LocardError;
use tracing::warn;

/// Legacy security interlocks and drive protection subsystem.
/// Retained for direct host path validation compatibility.
pub struct SecurityEngine;

impl SecurityEngine {
    /// Validates that a requested target path is valid and does NOT target
    /// a critical host operating system path.
    pub fn validate_target_safety(target: &str) -> Result<(), LocardError> {
        let trimmed = target.trim();
        if trimmed.is_empty() {
            return Err(LocardError::SecurityViolation(
                "Target identifier cannot be empty".to_string(),
            ));
        }

        // Defensive normalization
        let normalized = trimmed.to_uppercase();

        // Guard against direct wiping of Windows OS boot paths
        if normalized == "C:" || normalized == "C:\\" || normalized.starts_with("C:\\WINDOWS") {
            warn!(target = %trimmed, "Prevented destructive target against Windows OS system drive");
            return Err(LocardError::SecurityViolation(
                "Target is protected: System root partition cannot be modified".to_string(),
            ));
        }

        // Guard against Linux / Unix root devices
        if trimmed == "/" || trimmed == "/boot" || trimmed == "/etc" {
            warn!(target = %trimmed, "Prevented destructive target against Unix root partition");
            return Err(LocardError::SecurityViolation(
                "Target is protected: Unix system directory cannot be modified".to_string(),
            ));
        }

        Ok(())
    }
}
