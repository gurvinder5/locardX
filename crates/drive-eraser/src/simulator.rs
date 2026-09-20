use crate::models::{
    DriveEraseFailureReason, DriveErasePlan, DriveEraseProgress, DriveSanitizationMethod,
    ExecutionMode, HardwareExecutionPermit,
};
use std::time::Instant;

/// Trait defining the execution interface for drive sanitization.
/// In Step 10B.1, supports both Simulation backend and Real hardware backend.
pub trait DriveSanitizer: Send + Sync {
    /// Identifies the execution mode of this backend.
    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Simulation
    }

    /// Executes drive sanitization under an authorized HardwareExecutionPermit.
    fn execute(
        &self,
        permit: &HardwareExecutionPermit,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<(u64, f64), DriveEraseFailureReason> {
        if permit.execution_mode() != ExecutionMode::Simulation {
            return Err(DriveEraseFailureReason::PreExecutionCheckFailed(
                "Simulation backend only supports permits with ExecutionMode::Simulation"
                    .to_string(),
            ));
        }
        permit.consume()?;
        let passes = match permit.method() {
            DriveSanitizationMethod::Dod522022M => 3,
            _ => 1,
        };
        let plan = DriveErasePlan {
            plan_id: permit.plan_id().to_string(),
            physical_device_id: permit.physical_device_id().to_string(),
            display_name: permit.device_snapshot().display_name.clone(),
            vendor: permit.device_snapshot().vendor.clone(),
            model: permit.device_snapshot().model.clone(),
            serial_number: permit.device_snapshot().serial_number.clone(),
            media_type: permit.device_snapshot().media_type,
            capacity_bytes: permit.device_snapshot().capacity_bytes,
            sector_size: permit.device_snapshot().sector_size,
            method: permit.method(),
            passes,
            risk_level: locardx_security::RiskLevel::High,
            capabilities: crate::capabilities::detect_device_capabilities(
                &locardx_device_manager::PhysicalDevice {
                    device_id: permit.physical_device_id().to_string(),
                    display_name: permit.device_snapshot().display_name.clone(),
                    vendor: permit.device_snapshot().vendor.clone(),
                    model: permit.device_snapshot().model.clone(),
                    serial_number: permit.device_snapshot().serial_number.clone(),
                    device_type: permit.device_snapshot().media_type,
                    capacity_bytes: permit.device_snapshot().capacity_bytes,
                    removable: permit.device_snapshot().is_removable,
                    read_only: false,
                    is_system_device: permit.device_snapshot().is_system,
                    classification: permit.device_snapshot().classification,
                    volumes: vec![],
                },
            ),
            verification_plan: crate::models::DriveVerificationPlan {
                strategy: crate::models::DriveVerificationStrategy::FullDeviceReadVerify,
                sample_percentage: None,
                requirements: "Verification".to_string(),
                limitations: vec![],
            },
            limitations: vec![],
            device_snapshot: permit.device_snapshot().clone(),
            execution_mode: permit.execution_mode(),
            created_at: permit.issued_at().to_string(),
        };
        self.simulate_erasure(&plan, is_cancelled, on_progress)
    }

    /// Backwards-compatible simulated erasure entry point.
    fn simulate_erasure(
        &self,
        plan: &DriveErasePlan,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<(u64, f64), DriveEraseFailureReason>;
}

/// Simulated HDD sequential block overwrite engine.
pub struct SimulatedHddOverwrite;

impl DriveSanitizer for SimulatedHddOverwrite {
    fn simulate_erasure(
        &self,
        plan: &DriveErasePlan,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<(u64, f64), DriveEraseFailureReason> {
        let start = Instant::now();
        let total_bytes = plan.capacity_bytes;
        let passes = plan.passes;

        for pass in 1..=passes {
            let pass_name = match (plan.method, pass) {
                (DriveSanitizationMethod::Dod522022M, 1) => "Pass 1/3 (0x00 Null Bytes)",
                (DriveSanitizationMethod::Dod522022M, 2) => "Pass 2/3 (0xFF Inverted Pattern)",
                (DriveSanitizationMethod::Dod522022M, 3) => "Pass 3/3 (Cryptographic Random)",
                _ => "Pass 1/1 (NIST SP 800-88 Clear Zeros)",
            };

            for step in 1..=10 {
                if let Some(check) = is_cancelled {
                    if check() {
                        return Err(DriveEraseFailureReason::Cancelled);
                    }
                }

                let fraction =
                    (pass - 1) as f32 / passes as f32 + (step as f32 / 10.0) / passes as f32;
                let bytes_processed = (total_bytes as f64 * fraction as f64) as u64;
                let elapsed = start.elapsed().as_secs_f64();

                if let Some(cb) = on_progress {
                    cb(DriveEraseProgress {
                        operation_id: plan.plan_id.clone(),
                        percentage: (fraction * 100.0).min(100.0),
                        bytes_processed,
                        total_bytes: total_bytes * passes as u64,
                        current_pass: pass,
                        total_passes: passes,
                        current_stage: pass_name.to_string(),
                        elapsed_seconds: elapsed,
                        eta_seconds: Some((1.0 - fraction as f64) * 5.0),
                    });
                }
            }
        }

        let elapsed = start.elapsed().as_secs_f64();
        Ok((total_bytes * passes as u64, elapsed))
    }
}

/// Simulated ATA Secure Erase controller command.
pub struct SimulatedAtaSecureErase;

impl DriveSanitizer for SimulatedAtaSecureErase {
    fn simulate_erasure(
        &self,
        plan: &DriveErasePlan,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<(u64, f64), DriveEraseFailureReason> {
        let start = Instant::now();
        let total_bytes = plan.capacity_bytes;

        let stages = [
            "Sending ATA SECURITY ERASE UNIT command to drive controller",
            "Controller purging internal flash translation table (FTL)",
            "Controller triggering bulk block erase across NAND dies",
            "Waiting for ATA command completion and status register update",
        ];

        for (idx, stage) in stages.iter().enumerate() {
            if let Some(check) = is_cancelled {
                if check() {
                    return Err(DriveEraseFailureReason::Cancelled);
                }
            }

            let percentage = ((idx + 1) as f32 / stages.len() as f32) * 100.0;
            let bytes_processed = (total_bytes as f64 * (percentage as f64 / 100.0)) as u64;
            let elapsed = start.elapsed().as_secs_f64();

            if let Some(cb) = on_progress {
                cb(DriveEraseProgress {
                    operation_id: plan.plan_id.clone(),
                    percentage,
                    bytes_processed,
                    total_bytes,
                    current_pass: 1,
                    total_passes: 1,
                    current_stage: stage.to_string(),
                    elapsed_seconds: elapsed,
                    eta_seconds: Some(1.0),
                });
            }
        }

        let elapsed = start.elapsed().as_secs_f64();
        Ok((total_bytes, elapsed))
    }
}

/// Simulated NVMe Format and Sanitize controller engine.
pub struct SimulatedNvmeSanitize;

impl DriveSanitizer for SimulatedNvmeSanitize {
    fn simulate_erasure(
        &self,
        plan: &DriveErasePlan,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<(u64, f64), DriveEraseFailureReason> {
        let start = Instant::now();
        let total_bytes = plan.capacity_bytes;

        let stages = [
            "Issuing NVMe Sanitize command (Block Erase mode)",
            "Controller erasing user data pages across all namespaces",
            "Erasing controller over-provisioned blocks and wear-leveling reserves",
            "Polling NVMe Sanitize Status log page (0x15) for completion",
        ];

        for (idx, stage) in stages.iter().enumerate() {
            if let Some(check) = is_cancelled {
                if check() {
                    return Err(DriveEraseFailureReason::Cancelled);
                }
            }

            let percentage = ((idx + 1) as f32 / stages.len() as f32) * 100.0;
            let bytes_processed = (total_bytes as f64 * (percentage as f64 / 100.0)) as u64;
            let elapsed = start.elapsed().as_secs_f64();

            if let Some(cb) = on_progress {
                cb(DriveEraseProgress {
                    operation_id: plan.plan_id.clone(),
                    percentage,
                    bytes_processed,
                    total_bytes,
                    current_pass: 1,
                    total_passes: 1,
                    current_stage: stage.to_string(),
                    elapsed_seconds: elapsed,
                    eta_seconds: Some(0.5),
                });
            }
        }

        let elapsed = start.elapsed().as_secs_f64();
        Ok((total_bytes, elapsed))
    }
}

/// Simulated NVMe Cryptographic Key Erasure engine.
pub struct SimulatedNvmeCryptoErase;

impl DriveSanitizer for SimulatedNvmeCryptoErase {
    fn simulate_erasure(
        &self,
        plan: &DriveErasePlan,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<(u64, f64), DriveEraseFailureReason> {
        let start = Instant::now();
        let total_bytes = plan.capacity_bytes;

        if let Some(check) = is_cancelled {
            if check() {
                return Err(DriveEraseFailureReason::Cancelled);
            }
        }

        let stages = [
            "Initiating NVMe Cryptographic Sanitize (Crypto Erase mode)",
            "Obliterating internal Media Encryption Key (MEK)",
            "Generating and persisting new cryptographically secure AES-XTS key",
            "Invalidating all previous namespace data encryption contexts",
        ];

        for (idx, stage) in stages.iter().enumerate() {
            if let Some(check) = is_cancelled {
                if check() {
                    return Err(DriveEraseFailureReason::Cancelled);
                }
            }

            let percentage = ((idx + 1) as f32 / stages.len() as f32) * 100.0;
            let bytes_processed = total_bytes;
            let elapsed = start.elapsed().as_secs_f64();

            if let Some(cb) = on_progress {
                cb(DriveEraseProgress {
                    operation_id: plan.plan_id.clone(),
                    percentage,
                    bytes_processed,
                    total_bytes,
                    current_pass: 1,
                    total_passes: 1,
                    current_stage: stage.to_string(),
                    elapsed_seconds: elapsed,
                    eta_seconds: Some(0.1),
                });
            }
        }

        let elapsed = start.elapsed().as_secs_f64();
        Ok((total_bytes, elapsed))
    }
}

/// Resolves the appropriate simulated sanitizer backend for a planned method.
pub fn resolve_sanitizer_backend(method: DriveSanitizationMethod) -> Box<dyn DriveSanitizer> {
    resolve_sanitizer_for_mode(method, ExecutionMode::Simulation)
}

/// Resolves the appropriate sanitizer backend for a planned method and execution mode.
pub fn resolve_sanitizer_for_mode(
    method: DriveSanitizationMethod,
    mode: ExecutionMode,
) -> Box<dyn DriveSanitizer> {
    match mode {
        ExecutionMode::Simulation => match method {
            DriveSanitizationMethod::Nist80088ClearZero
            | DriveSanitizationMethod::Dod522022M
            | DriveSanitizationMethod::BlockZeroOverwrite => Box::new(SimulatedHddOverwrite),
            DriveSanitizationMethod::AtaSecureErase => Box::new(SimulatedAtaSecureErase),
            DriveSanitizationMethod::NvmeFormatSanitize => Box::new(SimulatedNvmeSanitize),
            DriveSanitizationMethod::NvmeCryptoErase => Box::new(SimulatedNvmeCryptoErase),
        },
        ExecutionMode::RealHardware => Box::new(crate::hardware::RealHardwareSanitizer::new()),
    }
}
