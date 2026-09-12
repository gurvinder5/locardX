use crate::models::{DriveErasePlan, DriveVerificationResult, DriveVerificationStrategy};
use chrono::Utc;
use locardx_verification::sanitization::VerificationOutcome;

/// Executes simulated post-erasure verification according to the media-aware verification plan.
pub fn execute_simulated_verification(
    plan: &DriveErasePlan,
    sanitization_succeeded: bool,
) -> DriveVerificationResult {
    if !sanitization_succeeded {
        return DriveVerificationResult {
            outcome: VerificationOutcome::NotApplicable,
            strategy: plan.verification_plan.strategy,
            details: "Verification omitted because sanitization did not complete successfully"
                .to_string(),
            verified_at: Utc::now().to_rfc3339(),
        };
    }

    match plan.verification_plan.strategy {
        DriveVerificationStrategy::FullDeviceReadVerify => DriveVerificationResult {
            outcome: VerificationOutcome::Verified,
            strategy: DriveVerificationStrategy::FullDeviceReadVerify,
            details: format!(
                "Simulated full device sequential readback verified across {} bytes. All addressed blocks confirmed cleared (0x00 pattern).",
                plan.capacity_bytes
            ),
            verified_at: Utc::now().to_rfc3339(),
        },
        DriveVerificationStrategy::FirmwareStatusVerify => DriveVerificationResult {
            outcome: VerificationOutcome::Verified,
            strategy: DriveVerificationStrategy::FirmwareStatusVerify,
            details: "Simulated drive controller status register inspected. Sanitize command completion flag confirmed with status 0x00 (Success).".to_string(),
            verified_at: Utc::now().to_rfc3339(),
        },
        DriveVerificationStrategy::CryptoKeyDestructionCheck => DriveVerificationResult {
            outcome: VerificationOutcome::Verified,
            strategy: DriveVerificationStrategy::CryptoKeyDestructionCheck,
            details: "Simulated NVMe cryptographic key destruction verified. Prior media encryption key eliminated; new cryptographically random key installed.".to_string(),
            verified_at: Utc::now().to_rfc3339(),
        },
        DriveVerificationStrategy::SampledSectorVerification => DriveVerificationResult {
            outcome: VerificationOutcome::Verified,
            strategy: DriveVerificationStrategy::SampledSectorVerification,
            details: format!(
                "Simulated sampled verification verified {}% of sectors across start, middle, and end LBA extents.",
                plan.verification_plan.sample_percentage.unwrap_or(10.0)
            ),
            verified_at: Utc::now().to_rfc3339(),
        },
        DriveVerificationStrategy::NotApplicable => DriveVerificationResult {
            outcome: VerificationOutcome::NotApplicable,
            strategy: DriveVerificationStrategy::NotApplicable,
            details: "No verification strategy applicable for this target media".to_string(),
            verified_at: Utc::now().to_rfc3339(),
        },
    }
}
