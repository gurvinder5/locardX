//! Platform-neutral hardware execution abstraction for physical drive sanitization.
//!
//! SAFETY INVARIANTS:
//! - Exclusive access must be established before any destructive I/O.
//! - The executor must execute ONLY the approved method in the immutable plan.
//! - No silent method downgrades or substitutions.
//! - Native platform APIs only; zero shell commands (no diskpart, format, cipher).

use crate::models::{
    DriveEraseFailureReason, DriveEraseProgress, DriveVerificationResult, HardwareExecutionPermit,
};

/// Trait representing an open, exclusively locked physical drive handle.
pub trait ExclusiveDriveHandle: Send + Sync {
    /// Returns the physical device identifier for this handle.
    fn device_id(&self) -> &str;

    /// Checks whether the underlying hardware handle is still open and valid.
    fn is_valid(&self) -> bool;
}

/// Core abstraction for native platform physical hardware sanitization execution.
pub trait DriveHardwareExecutor: Send + Sync {
    /// Attempts to establish exclusive physical access to the target drive.
    /// Fails closed if the device cannot be locked or is in use.
    fn acquire_exclusive_access(
        &self,
        device_id: &str,
    ) -> Result<Box<dyn ExclusiveDriveHandle>, DriveEraseFailureReason>;

    /// Executes the planned sanitization method using native platform mechanisms.
    /// Never substitutes another method or generic overwrite.
    fn execute_sanitization(
        &self,
        handle: &mut Box<dyn ExclusiveDriveHandle>,
        permit: &HardwareExecutionPermit,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<(u64, f64), DriveEraseFailureReason>;

    /// Performs post-sanitization forensic verification using native platform mechanisms.
    fn verify_sanitization(
        &self,
        handle: &mut Box<dyn ExclusiveDriveHandle>,
        permit: &HardwareExecutionPermit,
    ) -> Result<DriveVerificationResult, DriveEraseFailureReason>;
}
