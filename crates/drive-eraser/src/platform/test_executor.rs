//! Non-destructive test hardware executor with deterministic fault injection.
//!
//! Satisfies LocardX Step 10 Requirement 22 (NO-DESTRUCTIVE-TEST MODE):
//! Enables complete real execution state machine testing without physical destruction.

use crate::models::{
    DriveEraseFailureReason, DriveEraseProgress, DriveVerificationResult,
    DriveVerificationStrategy, HardwareExecutionPermit,
};
use crate::platform::executor::{DriveHardwareExecutor, ExclusiveDriveHandle};
use chrono::Utc;
use locardx_verification::sanitization::VerificationOutcome;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Configurable fault injection modes for non-destructive pipeline testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultInjectionMode {
    Normal,
    FailExclusiveLock,
    FailIoError,
    FailDisconnect,
    FailVerificationMismatch,
    FailUnableToVerify,
}

/// Test handle representing an exclusively locked device in test environments.
pub struct TestExclusiveHandle {
    pub device_id: String,
    pub is_valid: Arc<AtomicBool>,
}

impl ExclusiveDriveHandle for TestExclusiveHandle {
    fn device_id(&self) -> &str {
        &self.device_id
    }

    fn is_valid(&self) -> bool {
        self.is_valid.load(Ordering::SeqCst)
    }
}

/// A non-destructive test executor capable of simulating successful and failed executions.
pub struct FaultInjectableExecutor {
    mode: FaultInjectionMode,
}

impl FaultInjectableExecutor {
    pub fn new(mode: FaultInjectionMode) -> Self {
        Self { mode }
    }

    pub fn normal() -> Self {
        Self::new(FaultInjectionMode::Normal)
    }

    pub fn with_mode(mut self, mode: FaultInjectionMode) -> Self {
        self.mode = mode;
        self
    }
}

impl DriveHardwareExecutor for FaultInjectableExecutor {
    fn acquire_exclusive_access(
        &self,
        device_id: &str,
    ) -> Result<Box<dyn ExclusiveDriveHandle>, DriveEraseFailureReason> {
        if self.mode == FaultInjectionMode::FailExclusiveLock {
            return Err(DriveEraseFailureReason::ExclusiveAccessFailed(format!(
                "Simulated fault injection: Exclusive lock denied on '{}'",
                device_id
            )));
        }

        Ok(Box::new(TestExclusiveHandle {
            device_id: device_id.to_string(),
            is_valid: Arc::new(AtomicBool::new(true)),
        }))
    }

    fn execute_sanitization(
        &self,
        handle: &mut Box<dyn ExclusiveDriveHandle>,
        permit: &HardwareExecutionPermit,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<(u64, f64), DriveEraseFailureReason> {
        if !handle.is_valid() {
            return Err(DriveEraseFailureReason::DeviceDisconnected(format!(
                "Device '{}' handle is invalid",
                handle.device_id()
            )));
        }

        if self.mode == FaultInjectionMode::FailDisconnect {
            return Err(DriveEraseFailureReason::DeviceDisconnected(format!(
                "Simulated fault injection: Device '{}' disconnected during write",
                permit.physical_device_id()
            )));
        }

        if self.mode == FaultInjectionMode::FailIoError {
            return Err(DriveEraseFailureReason::IoError(format!(
                "Simulated fault injection: Hardware I/O failure on '{}'",
                permit.physical_device_id()
            )));
        }

        let capacity = permit.device_snapshot().capacity_bytes;
        let total_steps = 5;

        for step in 1..=total_steps {
            if let Some(cancel) = is_cancelled {
                if cancel() {
                    return Err(DriveEraseFailureReason::Cancelled);
                }
            }

            let processed = (capacity / total_steps as u64) * step as u64;
            let pct = (step as f32 / total_steps as f32) * 100.0;

            if let Some(cb) = on_progress {
                cb(DriveEraseProgress {
                    operation_id: permit.operation_id().to_string(),
                    percentage: pct,
                    bytes_processed: processed,
                    total_bytes: capacity,
                    current_pass: 1,
                    total_passes: 1,
                    current_stage: format!("Simulated pass 1/1 step {}/{}", step, total_steps),
                    elapsed_seconds: 0.1 * step as f64,
                    eta_seconds: Some(0.1 * (total_steps - step) as f64),
                });
            }
        }

        Ok((capacity, 0.5))
    }

    fn verify_sanitization(
        &self,
        handle: &mut Box<dyn ExclusiveDriveHandle>,
        permit: &HardwareExecutionPermit,
    ) -> Result<DriveVerificationResult, DriveEraseFailureReason> {
        if !handle.is_valid() {
            return Err(DriveEraseFailureReason::DeviceDisconnected(format!(
                "Device '{}' handle is invalid",
                handle.device_id()
            )));
        }

        match self.mode {
            FaultInjectionMode::FailVerificationMismatch => Ok(DriveVerificationResult {
                outcome: VerificationOutcome::VerificationFailed,
                strategy: DriveVerificationStrategy::FullDeviceReadVerify,
                details:
                    "Simulated fault injection: Non-zero pattern mismatch found at offset 0x1000"
                        .to_string(),
                verified_at: Utc::now().to_rfc3339(),
            }),
            FaultInjectionMode::FailUnableToVerify => Ok(DriveVerificationResult {
                outcome: VerificationOutcome::UnableToVerify,
                strategy: DriveVerificationStrategy::FirmwareStatusVerify,
                details: "Simulated fault injection: Device firmware status log unreadable"
                    .to_string(),
                verified_at: Utc::now().to_rfc3339(),
            }),
            _ => Ok(DriveVerificationResult {
                outcome: VerificationOutcome::Verified,
                strategy: DriveVerificationStrategy::FullDeviceReadVerify,
                details: format!(
                    "Simulated non-destructive readback verification passed on '{}'",
                    permit.physical_device_id()
                ),
                verified_at: Utc::now().to_rfc3339(),
            }),
        }
    }
}
