use crate::capabilities::assess_device_capabilities;
use crate::models::{
    DriveCapabilitiesAssessment, DriveEraseFailureReason, DriveErasePlan, DriveEraseProgress,
    ExecutionMode, HardwareExecutionPermit, PhysicalDeviceSnapshot,
};
use crate::simulator::DriveSanitizer;
use crate::target::{inspect_physical_device, validate_physical_device_identifier};
use locardx_common::LocardError;
use locardx_device_manager::{DeviceDiscoveryProvider, PhysicalDevice};
use std::sync::Arc;

/// Platform-neutral hardware provider interface for storage device discovery and probing.
///
/// Decouples OS-specific Windows (IOCTL_STORAGE_QUERY_PROPERTY, NVMe Sanitize) and
/// Linux (/sys/block, hdparm, nvme-cli) probing mechanisms from the core domain engine.
pub trait DriveHardwareProvider: Send + Sync {
    /// Discovers physical storage devices present on the platform.
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError>;

    /// Probes hardware-level sanitization capabilities for a device.
    fn probe_capabilities(
        &self,
        device: &PhysicalDevice,
    ) -> Result<DriveCapabilitiesAssessment, LocardError>;

    /// Live re-probe of device hardware snapshot for TOCTOU validation.
    fn probe_device_snapshot(
        &self,
        device_id: &str,
    ) -> Result<Option<PhysicalDeviceSnapshot>, LocardError>;

    /// Live refresh/re-enumeration of physical storage devices.
    fn refresh_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError> {
        self.discover_devices()
    }
}

/// Default platform provider bridging the existing `DeviceDiscoveryProvider`
/// into the new `DriveHardwareProvider` abstraction.
pub struct DefaultDriveHardwareProvider {
    discovery_provider: Arc<dyn DeviceDiscoveryProvider>,
}

impl DefaultDriveHardwareProvider {
    pub fn new(discovery_provider: Arc<dyn DeviceDiscoveryProvider>) -> Self {
        Self { discovery_provider }
    }
}

impl DriveHardwareProvider for DefaultDriveHardwareProvider {
    fn discover_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError> {
        self.discovery_provider.discover_devices()
    }

    fn probe_capabilities(
        &self,
        device: &PhysicalDevice,
    ) -> Result<DriveCapabilitiesAssessment, LocardError> {
        Ok(assess_device_capabilities(device))
    }

    fn probe_device_snapshot(
        &self,
        device_id: &str,
    ) -> Result<Option<PhysicalDeviceSnapshot>, LocardError> {
        match inspect_physical_device(device_id, &self.discovery_provider, 512) {
            Ok((_, snapshot)) => Ok(Some(snapshot)),
            Err(DriveEraseFailureReason::DeviceNotFound(_)) => Ok(None),
            Err(e) => Err(LocardError::Operation(e.to_string())),
        }
    }
}

/// Security gate controlling whether real hardware execution can proceed
/// and issuing single-use `HardwareExecutionPermit` tokens after mandatory checks.
pub struct RealHardwareExecutionGate {
    hardware_execution_enabled: bool,
}

impl Default for RealHardwareExecutionGate {
    fn default() -> Self {
        Self::new()
    }
}

impl RealHardwareExecutionGate {
    /// Creates a new execution gate.
    /// In Step 10B.1, real hardware execution is strictly disabled.
    pub fn new() -> Self {
        Self {
            hardware_execution_enabled: false,
        }
    }

    /// Constructor for testing gate behavior under enabled/disabled modes.
    pub fn with_enabled(enabled: bool) -> Self {
        Self {
            hardware_execution_enabled: enabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.hardware_execution_enabled
    }

    /// Evaluates all safety requirements and, if valid, issues a single-use permit.
    ///
    /// Mandatory checks performed:
    /// 1. Execution mode check: If RealHardware, gate MUST be enabled.
    /// 2. Target path check: Logical volume identifiers (e.g. C:\) are hard-rejected.
    /// 3. System and boot device check: System/boot devices are hard-blocked unconditionally.
    /// 4. Capability state check: Fails closed if capability is Unknown or DetectionFailed.
    /// 5. Two-stage confirmation validation: Warning acknowledged, typed target matches device ID.
    /// 6. TOCTOU live re-probe: Verifies live device exists and attributes have not mutated.
    ///
    /// Returns `HardwareExecutionPermit` on success, or `DriveEraseFailureReason`.
    pub fn request_permit(
        &self,
        plan: &DriveErasePlan,
        operation_id: &str,
        typed_target_confirmation: &str,
        warning_acknowledged: bool,
        hardware_provider: &dyn DriveHardwareProvider,
    ) -> Result<HardwareExecutionPermit, DriveEraseFailureReason> {
        // 1. Gate check for real hardware execution
        if plan.execution_mode == ExecutionMode::RealHardware && !self.hardware_execution_enabled {
            return Err(DriveEraseFailureReason::RealHardwareExecutionNotEnabled(
                "Real hardware execution gate is CLOSED in Step 10B.1".to_string(),
            ));
        }

        // 2. Logical volume rejection (must be physical drive)
        validate_physical_device_identifier(&plan.physical_device_id)?;

        // 3. System and boot hard block
        if plan.device_snapshot.is_system || plan.device_snapshot.is_boot {
            return Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(
                format!(
                    "Device '{}' is an active system or boot device",
                    plan.physical_device_id
                ),
            ));
        }

        let norm_id = plan.physical_device_id.to_uppercase();
        if (norm_id == r"\\.\PHYSICALDRIVE0" || norm_id == "PHYSICALDRIVE0")
            && (!plan.device_snapshot.is_removable || plan.device_snapshot.is_system)
        {
            return Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(
                format!(
                    "Device '{}' is the primary system drive (PhysicalDrive0); sanitization is permanently blocked.",
                    plan.physical_device_id
                ),
            ));
        }

        // 4. Two-stage confirmation check
        if !warning_acknowledged {
            return Err(DriveEraseFailureReason::SecurityViolation(
                "Destructive consequences warning must be explicitly acknowledged".to_string(),
            ));
        }

        let typed_trimmed = typed_target_confirmation.trim();
        let id_trimmed = plan.physical_device_id.trim();
        let matches_id = typed_trimmed.eq_ignore_ascii_case(id_trimmed)
            || typed_trimmed.eq_ignore_ascii_case(&id_trimmed.replace(r"\\.\", ""));

        if !matches_id {
            return Err(DriveEraseFailureReason::SecurityViolation(format!(
                "Typed confirmation '{}' does not match physical device ID '{}'",
                typed_trimmed, plan.physical_device_id
            )));
        }

        // 5. Capability assessment & fail-closed check
        if plan.capabilities.is_empty() {
            return Err(DriveEraseFailureReason::NoSupportedCapabilities(
                "Device has no supported capabilities".to_string(),
            ));
        }

        // 6. Live TOCTOU re-probe
        let live_snapshot_opt = hardware_provider
            .probe_device_snapshot(&plan.physical_device_id)
            .map_err(|e| DriveEraseFailureReason::DeviceNotFound(e.to_string()))?;

        let live_snapshot = live_snapshot_opt.ok_or_else(|| {
            DriveEraseFailureReason::DeviceNotFound(format!(
                "Target device '{}' was disconnected or is unavailable",
                plan.physical_device_id
            ))
        })?;

        let reval = plan.device_snapshot.revalidate_identity(&live_snapshot);
        let discrepancies = match reval {
            crate::models::IdentityRevalidationOutcome::Verified => Vec::new(),
            crate::models::IdentityRevalidationOutcome::VerifiedWithDiscrepancy(disc) => disc,
            crate::models::IdentityRevalidationOutcome::Failed(err) => {
                return Err(DriveEraseFailureReason::DeviceMutated(err));
            }
        };

        // 7. Issue single-use permit bound to operation, plan, device, snapshot, method, and mode
        let mut permit = HardwareExecutionPermit::issue(
            operation_id.to_string(),
            plan.plan_id.clone(),
            plan.physical_device_id.clone(),
            plan.device_snapshot.clone(),
            plan.method,
            plan.execution_mode,
        );
        if !discrepancies.is_empty() {
            permit = permit.with_identity_discrepancies(discrepancies);
        }
        Ok(permit)
    }
}

use crate::platform::{DriveHardwareExecutor, RealDriveHardwareExecutor};
use locardx_device_manager::DeviceClassification;
use std::collections::HashSet;
use std::sync::Mutex;

/// Registry managing exclusive target device concurrency locks.
#[derive(Clone, Default)]
pub struct DeviceLockRegistry {
    locked_devices: Arc<Mutex<HashSet<String>>>,
}

/// RAII lock guard releasing device concurrency lock upon drop.
#[derive(Debug)]
pub struct DeviceLockGuard {
    device_id: String,
    locked_devices: Arc<Mutex<HashSet<String>>>,
}

impl Drop for DeviceLockGuard {
    fn drop(&mut self) {
        if let Ok(mut locked) = self.locked_devices.lock() {
            locked.remove(&self.device_id);
        }
    }
}

impl DeviceLockRegistry {
    pub fn new() -> Self {
        Self {
            locked_devices: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn is_locked(&self, device_id: &str) -> bool {
        let normalized = device_id.trim().to_uppercase();
        self.locked_devices
            .lock()
            .map(|l| l.contains(&normalized))
            .unwrap_or(false)
    }

    pub fn acquire_lock(
        &self,
        device_id: &str,
    ) -> Result<DeviceLockGuard, DriveEraseFailureReason> {
        let normalized = device_id.trim().to_uppercase();
        let mut locked = self.locked_devices.lock().unwrap();
        if locked.contains(&normalized) {
            return Err(DriveEraseFailureReason::ExclusiveAccessFailed(format!(
                "Device '{}' is already being sanitized by another active operation",
                device_id
            )));
        }
        locked.insert(normalized.clone());
        Ok(DeviceLockGuard {
            device_id: normalized,
            locked_devices: Arc::clone(&self.locked_devices),
        })
    }
}

/// Production implementation of `DriveSanitizer` for real physical hardware.
///
/// STRICT SAFETY INVARIANTS:
/// - Rejects execution if execution mode != RealHardware
/// - Validates single-use `HardwareExecutionPermit` (non-consumed, non-expired TTL, binding)
/// - Executes the 15-point Pre-Execution Checkpoint
/// - Prevents concurrent operations on the same target (DeviceLockRegistry)
/// - Executes ONLY the exact method authorized in DriveErasePlan
/// - Never silently downgrades or substitutes sanitization methods
/// - Communicates with platform-specific DriveHardwareExecutor
/// - Emits live progress events
/// - Re-evaluates verification outcome
pub struct RealHardwareSanitizer {
    executor: Arc<dyn DriveHardwareExecutor>,
    lock_registry: Arc<DeviceLockRegistry>,
    hardware_provider: Option<Arc<dyn DriveHardwareProvider>>,
    is_stub: bool,
}

impl Default for RealHardwareSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

impl RealHardwareSanitizer {
    pub fn new() -> Self {
        Self {
            executor: Arc::new(RealDriveHardwareExecutor::new()),
            lock_registry: Arc::new(DeviceLockRegistry::new()),
            hardware_provider: None,
            is_stub: false,
        }
    }

    pub fn stub() -> Self {
        Self {
            executor: Arc::new(RealDriveHardwareExecutor::new()),
            lock_registry: Arc::new(DeviceLockRegistry::new()),
            hardware_provider: None,
            is_stub: true,
        }
    }

    pub fn with_executor(executor: Arc<dyn DriveHardwareExecutor>) -> Self {
        Self {
            executor,
            lock_registry: Arc::new(DeviceLockRegistry::new()),
            hardware_provider: None,
            is_stub: false,
        }
    }

    pub fn with_components(
        executor: Arc<dyn DriveHardwareExecutor>,
        lock_registry: Arc<DeviceLockRegistry>,
        hardware_provider: Option<Arc<dyn DriveHardwareProvider>>,
    ) -> Self {
        Self {
            executor,
            lock_registry,
            hardware_provider,
            is_stub: false,
        }
    }

    pub fn lock_registry(&self) -> &Arc<DeviceLockRegistry> {
        &self.lock_registry
    }

    pub fn executor(&self) -> &Arc<dyn DriveHardwareExecutor> {
        &self.executor
    }

    /// Executes sanitization and immediately follows with post-sanitization verification.
    pub fn execute_and_verify(
        &self,
        permit: &HardwareExecutionPermit,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<(u64, f64, crate::models::DriveVerificationResult), DriveEraseFailureReason> {
        if self.is_stub {
            let _ = permit.consume();
            return Err(DriveEraseFailureReason::RealHardwareExecutionNotEnabled(
                format!(
                    "Real hardware backend execution on '{}' is disabled in Step 10B.1. Only read-only capability probing and simulation are supported.",
                    permit.physical_device_id()
                ),
            ));
        }

        // Check 13: Execution mode must be RealHardware
        if permit.execution_mode() != ExecutionMode::RealHardware {
            return Err(DriveEraseFailureReason::RealHardwareExecutionDisabled(
                "Execution permit mode is not RealHardware".to_string(),
            ));
        }

        // Check 10 & 14: Permit validity and TTL expiration
        if permit.is_consumed() {
            return Err(DriveEraseFailureReason::PermitAlreadyConsumed(
                permit.permit_id().to_string(),
            ));
        }
        permit.validate_not_expired()?;

        // Check 10, 11, 12: Binding check
        permit.validate_binding(
            permit.operation_id(),
            permit.plan_id(),
            permit.physical_device_id(),
        )?;

        // Check 6: Target validation (rejects logical volumes like C:\)
        validate_physical_device_identifier(permit.physical_device_id())?;

        // Check 4 & 5: System and boot hard blocks on snapshot
        if permit.device_snapshot().is_system || permit.device_snapshot().is_boot {
            return Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(
                format!(
                    "Device '{}' is an active system or boot device; sanitization is permanently blocked",
                    permit.physical_device_id()
                ),
            ));
        }

        if permit.device_snapshot().classification == DeviceClassification::SystemDevice
            || permit.device_snapshot().classification == DeviceClassification::BootDevice
        {
            return Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(
                format!(
                    "Device '{}' classification is {:?}; sanitization is permanently blocked",
                    permit.physical_device_id(),
                    permit.device_snapshot().classification
                ),
            ));
        }

        // Check 8: Write-protection status
        if permit.device_snapshot().is_read_only {
            return Err(DriveEraseFailureReason::PreExecutionCheckFailed(format!(
                "Device '{}' is write-protected or read-only",
                permit.physical_device_id()
            )));
        }

        // Check 1, 2, 3, 7, 9: Live TOCTOU re-probe if provider is configured
        if let Some(ref provider) = self.hardware_provider {
            let live_snapshot_opt = provider
                .probe_device_snapshot(permit.physical_device_id())
                .map_err(|e| DriveEraseFailureReason::DeviceNotFound(e.to_string()))?;

            let live_snapshot = live_snapshot_opt.ok_or_else(|| {
                DriveEraseFailureReason::DeviceNotFound(format!(
                    "Target device '{}' was disconnected or is unavailable on system bus",
                    permit.physical_device_id()
                ))
            })?;

            let reval = permit.device_snapshot().revalidate_identity(&live_snapshot);
            match reval {
                crate::models::IdentityRevalidationOutcome::Verified
                | crate::models::IdentityRevalidationOutcome::VerifiedWithDiscrepancy(_) => {}
                crate::models::IdentityRevalidationOutcome::Failed(err) => {
                    return Err(DriveEraseFailureReason::DeviceMutated(err));
                }
            }

            if live_snapshot.is_system || live_snapshot.is_boot {
                return Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(
                    format!(
                        "Target device '{}' is an active system or boot device",
                        permit.physical_device_id()
                    ),
                ));
            }

            if live_snapshot.is_read_only {
                return Err(DriveEraseFailureReason::PreExecutionCheckFailed(format!(
                    "Target device '{}' write status mutated: now read-only",
                    permit.physical_device_id()
                )));
            }
        }

        // Check 15: Device-level concurrency lock
        let _lock_guard = self
            .lock_registry
            .acquire_lock(permit.physical_device_id())?;

        // Check 15: Platform-level exclusive hardware handle acquisition
        let mut handle = self
            .executor
            .acquire_exclusive_access(permit.physical_device_id())?;

        // Enforce single-use permit consumption BEFORE issuing destructive I/O
        permit.consume()?;

        // Execute approved sanitization method through platform executor
        let (bytes, elapsed) =
            self.executor
                .execute_sanitization(&mut handle, permit, is_cancelled, on_progress)?;

        // Execute post-sanitization forensic readback / controller verification
        let verification = self.executor.verify_sanitization(&mut handle, permit)?;

        Ok((bytes, elapsed, verification))
    }
}

impl DriveSanitizer for RealHardwareSanitizer {
    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::RealHardware
    }

    fn execute(
        &self,
        permit: &HardwareExecutionPermit,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<(u64, f64), DriveEraseFailureReason> {
        let (bytes, elapsed, _) = self.execute_and_verify(permit, is_cancelled, on_progress)?;
        Ok((bytes, elapsed))
    }

    fn simulate_erasure(
        &self,
        plan: &DriveErasePlan,
        _is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        _on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<(u64, f64), DriveEraseFailureReason> {
        Err(DriveEraseFailureReason::RealHardwareExecutionDisabled(
            format!(
                "RealHardwareSanitizer cannot perform simulation for '{}'.",
                plan.physical_device_id
            ),
        ))
    }
}
