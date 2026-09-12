use crate::capabilities::detect_device_capabilities;
use crate::hardware::{
    DefaultDriveHardwareProvider, DeviceLockRegistry, DriveHardwareProvider,
    RealHardwareExecutionGate, RealHardwareSanitizer,
};
use crate::models::{
    DriveCapabilitiesAssessment, DriveEraseFailureReason, DriveErasePlan, DriveEraseProgress,
    DriveEraseRequest, DriveEraseResult, DriveEraseStatus, DriveVerificationResult, ExecutionMode,
};
use crate::planner::{generate_drive_erase_plan, generate_real_hardware_drive_erase_plan};
use crate::platform::{DriveHardwareExecutor, RealDriveHardwareExecutor};
use crate::simulator::resolve_sanitizer_backend;
use crate::target::{inspect_physical_device, verify_live_device_integrity};
use crate::verification::execute_simulated_verification;
use chrono::Utc;
use locardx_audit::AuditService;
use locardx_common::LocardError;
use locardx_database::Database;
use locardx_device_manager::{DeviceDiscoveryProvider, PhysicalDevice};
use locardx_security::SafetyEngine;
use rusqlite::params;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Core service for planning, authorizing, simulating, verifying, and recording drive erasure.
pub struct DriveEraserService {
    db: Arc<Database>,
    audit: Arc<AuditService>,
    safety: Arc<SafetyEngine>,
    device_provider: Arc<dyn DeviceDiscoveryProvider>,
    hardware_provider: Arc<dyn DriveHardwareProvider>,
    execution_gate: Arc<RealHardwareExecutionGate>,
    lock_registry: Arc<DeviceLockRegistry>,
    executor: Option<Arc<dyn DriveHardwareExecutor>>,
    plans: Arc<RwLock<HashMap<String, DriveErasePlan>>>,
}

impl DriveEraserService {
    pub fn new(
        db: Arc<Database>,
        audit: Arc<AuditService>,
        safety: Arc<SafetyEngine>,
        device_provider: Arc<dyn DeviceDiscoveryProvider>,
    ) -> Self {
        let hardware_provider = Arc::new(DefaultDriveHardwareProvider::new(Arc::clone(
            &device_provider,
        )));
        let execution_gate = Arc::new(RealHardwareExecutionGate::new());
        let lock_registry = Arc::new(DeviceLockRegistry::new());
        Self {
            db,
            audit,
            safety,
            device_provider,
            hardware_provider,
            execution_gate,
            lock_registry,
            executor: None,
            plans: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_execution_gate(mut self, execution_gate: Arc<RealHardwareExecutionGate>) -> Self {
        self.execution_gate = execution_gate;
        self
    }

    pub fn with_hardware_provider(
        mut self,
        hardware_provider: Arc<dyn DriveHardwareProvider>,
    ) -> Self {
        self.hardware_provider = hardware_provider;
        self
    }

    pub fn with_lock_registry(mut self, lock_registry: Arc<DeviceLockRegistry>) -> Self {
        self.lock_registry = lock_registry;
        self
    }

    pub fn with_executor(mut self, executor: Arc<dyn DriveHardwareExecutor>) -> Self {
        self.executor = Some(executor);
        self
    }

    pub fn lock_registry(&self) -> &Arc<DeviceLockRegistry> {
        &self.lock_registry
    }

    pub fn executor(&self) -> Option<&Arc<dyn DriveHardwareExecutor>> {
        self.executor.as_ref()
    }

    pub fn safety(&self) -> &Arc<SafetyEngine> {
        &self.safety
    }

    pub fn database(&self) -> &Arc<Database> {
        &self.db
    }

    pub fn audit(&self) -> &Arc<AuditService> {
        &self.audit
    }

    pub fn device_provider(&self) -> &Arc<dyn DeviceDiscoveryProvider> {
        &self.device_provider
    }

    pub fn hardware_provider(&self) -> &Arc<dyn DriveHardwareProvider> {
        &self.hardware_provider
    }

    pub fn execution_gate(&self) -> &Arc<RealHardwareExecutionGate> {
        &self.execution_gate
    }

    /// Discovers real physical storage devices using the registered hardware provider.
    pub fn discover_real_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError> {
        self.hardware_provider.discover_devices()
    }

    /// Refreshes/re-enumerates real physical storage devices.
    pub fn refresh_real_devices(&self) -> Result<Vec<PhysicalDevice>, LocardError> {
        self.hardware_provider.refresh_devices()
    }

    /// Assesses hardware capabilities for a drive with explicit fail-closed states.
    pub async fn assess_drive_capabilities(
        &self,
        target_device_id: &str,
    ) -> Result<DriveCapabilitiesAssessment, LocardError> {
        let normalized = crate::target::validate_physical_device_identifier(target_device_id)
            .map_err(|e| match e {
                DriveEraseFailureReason::LogicalVolumeTargetRejected(msg) => {
                    LocardError::SecurityViolation(msg)
                }
                other => LocardError::Operation(other.to_string()),
            })?;

        let device_opt = match inspect_physical_device(&normalized, &self.device_provider, 512) {
            Ok((dev, _)) => Some(dev),
            Err(DriveEraseFailureReason::SystemOrBootDeviceProtected(msg)) => {
                return Err(LocardError::SecurityViolation(msg));
            }
            Err(DriveEraseFailureReason::LogicalVolumeTargetRejected(msg)) => {
                return Err(LocardError::SecurityViolation(msg));
            }
            Err(_) => {
                let real_devices = self
                    .hardware_provider
                    .discover_devices()
                    .unwrap_or_default();
                let real_dev = real_devices
                    .into_iter()
                    .find(|d| d.device_id.eq_ignore_ascii_case(&normalized));

                if let Some(ref dev) = real_dev {
                    if dev.is_system_device
                        || dev.classification
                            == locardx_device_manager::DeviceClassification::SystemDevice
                        || dev.classification
                            == locardx_device_manager::DeviceClassification::BootDevice
                        || dev
                            .volumes
                            .iter()
                            .any(|v| v.is_system_volume || v.is_boot_volume)
                    {
                        return Err(LocardError::SecurityViolation(format!(
                            "Target '{}' is an active system or boot device; capability probing rejected.",
                            normalized
                        )));
                    }
                }
                real_dev
            }
        };

        let device = device_opt.ok_or_else(|| {
            LocardError::Operation(format!("Physical device '{}' not found", normalized))
        })?;

        self.hardware_provider.probe_capabilities(&device)
    }

    /// Evaluates and generates a deterministic, media-aware plan for whole-disk sanitization.
    pub async fn plan_drive_erasure(
        &self,
        request: DriveEraseRequest,
        actor_id: Option<String>,
    ) -> Result<DriveErasePlan, LocardError> {
        let execution_mode = request.execution_mode.unwrap_or(ExecutionMode::Simulation);

        if execution_mode == ExecutionMode::RealHardware {
            if !self.execution_gate.is_enabled() {
                return Err(LocardError::SecurityViolation(
                    "Real hardware execution is permanently disabled in Step 10A; only Simulation is permitted.".to_string(),
                ));
            }

            // 1. Inspect physical device (strictly rejecting logical volume paths like C:\)
            let (device, snapshot) =
                inspect_physical_device(&request.target_device_id, &self.device_provider, 512)
                    .map_err(|e| match e {
                        DriveEraseFailureReason::LogicalVolumeTargetRejected(msg) => {
                            LocardError::SecurityViolation(msg)
                        }
                        DriveEraseFailureReason::SystemOrBootDeviceProtected(msg) => {
                            LocardError::SecurityViolation(msg)
                        }
                        other => LocardError::Operation(other.to_string()),
                    })?;

            // 2. Hardware capability probing with fail-closed assessment
            let assessment = self.hardware_provider.probe_capabilities(&device)?;
            if assessment.overall_state.should_fail_closed() {
                return Err(LocardError::Operation(format!(
                    "Device capability assessment state '{:?}' fails closed; planning rejected.",
                    assessment.overall_state
                )));
            }

            // 3. Generate media-aware plan authorized for RealHardware
            let plan = generate_real_hardware_drive_erase_plan(
                snapshot,
                assessment.capabilities,
                request.requested_method.as_deref(),
            )
            .map_err(|e| LocardError::Operation(e.to_string()))?;

            // 4. Cache plan
            {
                let mut map = self.plans.write().await;
                map.insert(plan.plan_id.clone(), plan.clone());
            }

            // 5. Audit planning event
            let _ = self.audit.log_structured_event(
                "DRIVE_ERASURE_PLANNED",
                actor_id.as_deref(),
                Some(&plan.physical_device_id),
                &format!(
                    "Drive erasure plan '{}' created for device '{}' ({}) using method {:?} [REAL_HARDWARE]",
                    plan.plan_id, plan.physical_device_id, plan.display_name, plan.method
                ),
            );

            return Ok(plan);
        }

        // 1. Inspect physical device (strictly rejecting logical volume paths like C:\)
        let (device, snapshot) =
            inspect_physical_device(&request.target_device_id, &self.device_provider, 512)
                .map_err(|e| match e {
                    DriveEraseFailureReason::LogicalVolumeTargetRejected(msg) => {
                        LocardError::SecurityViolation(msg)
                    }
                    DriveEraseFailureReason::SystemOrBootDeviceProtected(msg) => {
                        LocardError::SecurityViolation(msg)
                    }
                    other => LocardError::Operation(other.to_string()),
                })?;

        // 2. Detect hardware capabilities
        let capabilities = detect_device_capabilities(&device);

        // 3. Generate media-aware plan
        let plan = generate_drive_erase_plan(
            snapshot,
            capabilities,
            request.requested_method.as_deref(),
            execution_mode,
        )
        .map_err(|e| LocardError::Operation(e.to_string()))?;

        // 4. Cache plan
        {
            let mut map = self.plans.write().await;
            map.insert(plan.plan_id.clone(), plan.clone());
        }

        // 5. Audit planning event
        let _ = self.audit.log_structured_event(
            "DRIVE_ERASURE_PLANNED",
            actor_id.as_deref(),
            Some(&plan.physical_device_id),
            &format!(
                "Drive erasure plan '{}' created for device '{}' ({}) using method {:?}",
                plan.plan_id, plan.physical_device_id, plan.display_name, plan.method
            ),
        );

        Ok(plan)
    }

    /// Fetches a previously generated plan by ID.
    pub async fn get_plan(&self, plan_id: &str) -> Result<DriveErasePlan, LocardError> {
        let map = self.plans.read().await;
        map.get(plan_id)
            .cloned()
            .ok_or_else(|| LocardError::Operation(format!("Plan '{}' not found", plan_id)))
    }

    /// Executes simulated drive sanitization, enforcing two-stage confirmation and live TOCTOU validation.
    pub async fn execute_drive_erasure_simulation(
        &self,
        plan_id: &str,
        confirmation_id: &str,
        operation_id: &str,
        typed_target_confirmation: &str,
        warning_acknowledged: bool,
        session_token: &str,
        execution_mode: ExecutionMode,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<DriveEraseResult, LocardError> {
        // INVARIANT 1: Structural prohibition of real hardware writes
        if execution_mode == ExecutionMode::RealHardware {
            return Err(LocardError::SecurityViolation(
                "CRITICAL INVARIANT: Real hardware execution is permanently disabled in Step 10A; only Simulation is permitted.".to_string(),
            ));
        }

        let plan = self.get_plan(plan_id).await?;
        let started_at = Utc::now().to_rfc3339();

        // 1. Verify confirmation details
        if !warning_acknowledged {
            return Err(LocardError::SecurityViolation(
                "Destructive consequences warning must be explicitly acknowledged.".to_string(),
            ));
        }

        let typed_trimmed = typed_target_confirmation.trim();
        let id_trimmed = plan.physical_device_id.trim();
        let matches_id = typed_trimmed.eq_ignore_ascii_case(id_trimmed)
            || typed_trimmed.eq_ignore_ascii_case(&id_trimmed.replace(r"\\.\", ""));

        if !matches_id {
            return Err(LocardError::SecurityViolation(format!(
                "Typed confirmation '{}' does not match physical device ID '{}'",
                typed_trimmed, plan.physical_device_id
            )));
        }

        // 2. Hard block on system or boot device
        if plan.device_snapshot.is_system || plan.device_snapshot.is_boot {
            let _ = self.audit.log_structured_event(
                "DRIVE_ERASURE_REJECTED",
                Some(session_token),
                Some(&plan.physical_device_id),
                "Drive erasure rejected: Target is an active system or boot device",
            );
            return Err(LocardError::SecurityViolation(
                "Hard block: System and boot devices cannot be sanitized.".to_string(),
            ));
        }

        // 3. LIVE HARDWARE RE-PROBE (TOCTOU Defense)
        if let Err(e) = verify_live_device_integrity(&plan.device_snapshot, &self.device_provider) {
            let _ = self.audit.log_structured_event(
                "DRIVE_ERASURE_TARGET_CHANGED",
                Some(session_token),
                Some(&plan.physical_device_id),
                &format!("Device mutation detected prior to simulation: {}", e),
            );
            return Err(LocardError::SecurityViolation(format!(
                "Live target re-probe failed: {}. Erasure cancelled.",
                e
            )));
        }

        // 4. Audit simulation start
        let _ = self.audit.log_structured_event(
            "DRIVE_ERASURE_SIMULATION_STARTED",
            Some(session_token),
            Some(&plan.physical_device_id),
            &format!(
                "Beginning simulated drive sanitization for op {} (challenge {}) on device {} (Method: {:?}, Passes: {})",
                operation_id, confirmation_id, plan.physical_device_id, plan.method, plan.passes
            ),
        );

        // 5. Execute simulated sanitization
        let sanitizer = resolve_sanitizer_backend(plan.method);
        let sim_result = sanitizer.simulate_erasure(&plan, is_cancelled, on_progress);

        match sim_result {
            Ok((bytes_processed, elapsed_seconds)) => {
                let _ = self.audit.log_structured_event(
                    "DRIVE_ERASURE_SIMULATION_COMPLETED",
                    Some(session_token),
                    Some(&plan.physical_device_id),
                    &format!(
                        "Simulated drive sanitization completed ({} bytes processed in {:.2}s)",
                        bytes_processed, elapsed_seconds
                    ),
                );

                // 6. Execute simulated post-erasure verification
                let _ = self.audit.log_structured_event(
                    "DRIVE_ERASURE_VERIFICATION_STARTED",
                    Some(session_token),
                    Some(&plan.physical_device_id),
                    &format!(
                        "Executing verification strategy {:?}",
                        plan.verification_plan.strategy
                    ),
                );

                let verification = execute_simulated_verification(&plan, true);

                let _ = self.audit.log_structured_event(
                    "DRIVE_ERASURE_VERIFICATION_COMPLETED",
                    Some(session_token),
                    Some(&plan.physical_device_id),
                    &format!(
                        "Verification outcome: {:?} ({})",
                        verification.outcome, verification.details
                    ),
                );

                let completed_at = Utc::now().to_rfc3339();
                let res = DriveEraseResult {
                    operation_id: operation_id.to_string(),
                    plan_id: plan.plan_id.clone(),
                    physical_device_id: plan.physical_device_id.clone(),
                    display_name: plan.display_name.clone(),
                    vendor: plan.vendor.clone(),
                    model: plan.model.clone(),
                    serial_number: plan.serial_number.clone(),
                    media_type: plan.media_type,
                    capacity_bytes: plan.capacity_bytes,
                    sector_size: plan.sector_size,
                    method: plan.method,
                    execution_mode: ExecutionMode::Simulation,
                    status: DriveEraseStatus::Completed,
                    bytes_processed,
                    elapsed_seconds,
                    verification,
                    failure_reason: None,
                    audit_references: vec![
                        "DRIVE_ERASURE_SIMULATION_STARTED".to_string(),
                        "DRIVE_ERASURE_SIMULATION_COMPLETED".to_string(),
                        "DRIVE_ERASURE_VERIFICATION_COMPLETED".to_string(),
                    ],
                    started_at,
                    completed_at,
                    limitations: plan.limitations,
                    identity_discrepancies: Vec::new(),
                };

                self.record_result(&res)?;
                Ok(res)
            }
            Err(DriveEraseFailureReason::Cancelled) => {
                let _ = self.audit.log_structured_event(
                    "DRIVE_ERASURE_CANCELLED",
                    Some(session_token),
                    Some(&plan.physical_device_id),
                    "Drive erasure simulation cancelled cooperatively by operator",
                );

                let completed_at = Utc::now().to_rfc3339();
                let res = DriveEraseResult {
                    operation_id: operation_id.to_string(),
                    plan_id: plan.plan_id.clone(),
                    physical_device_id: plan.physical_device_id.clone(),
                    display_name: plan.display_name.clone(),
                    vendor: plan.vendor.clone(),
                    model: plan.model.clone(),
                    serial_number: plan.serial_number.clone(),
                    media_type: plan.media_type,
                    capacity_bytes: plan.capacity_bytes,
                    sector_size: plan.sector_size,
                    method: plan.method,
                    execution_mode: ExecutionMode::Simulation,
                    status: DriveEraseStatus::Cancelled,
                    bytes_processed: 0,
                    elapsed_seconds: 0.0,
                    verification: execute_simulated_verification(&plan, false),
                    failure_reason: Some(DriveEraseFailureReason::Cancelled),
                    audit_references: vec!["DRIVE_ERASURE_CANCELLED".to_string()],
                    started_at,
                    completed_at,
                    limitations: plan.limitations,
                    identity_discrepancies: Vec::new(),
                };

                self.record_result(&res)?;
                Ok(res)
            }
            Err(err) => {
                let _ = self.audit.log_structured_event(
                    "DRIVE_ERASURE_REJECTED",
                    Some(session_token),
                    Some(&plan.physical_device_id),
                    &format!("Drive erasure simulation failed: {}", err),
                );

                let completed_at = Utc::now().to_rfc3339();
                let res = DriveEraseResult {
                    operation_id: operation_id.to_string(),
                    plan_id: plan.plan_id.clone(),
                    physical_device_id: plan.physical_device_id.clone(),
                    display_name: plan.display_name.clone(),
                    vendor: plan.vendor.clone(),
                    model: plan.model.clone(),
                    serial_number: plan.serial_number.clone(),
                    media_type: plan.media_type,
                    capacity_bytes: plan.capacity_bytes,
                    sector_size: plan.sector_size,
                    method: plan.method,
                    execution_mode: ExecutionMode::Simulation,
                    status: DriveEraseStatus::Failed,
                    bytes_processed: 0,
                    elapsed_seconds: 0.0,
                    verification: execute_simulated_verification(&plan, false),
                    failure_reason: Some(err),
                    audit_references: vec!["DRIVE_ERASURE_REJECTED".to_string()],
                    started_at,
                    completed_at,
                    limitations: plan.limitations,
                    identity_discrepancies: Vec::new(),
                };

                self.record_result(&res)?;
                Ok(res)
            }
        }
    }

    /// Executes drive sanitization through the formal HardwareExecutionPermit and RealHardwareExecutionGate boundary.
    ///
    /// Preserves all Step 10A safety invariants:
    /// - Logical-volume rejection
    /// - System-device hard block
    /// - Boot-device hard block
    /// - Two-stage confirmation validation
    /// - TOCTOU snapshot validation
    /// - Fail-closed behavior on Unknown or DetectionFailed capability states
    /// - Interrupted operations never automatically become COMPLETED
    /// - Non-destructive stub rejection if RealHardware mode is requested
    pub async fn execute_drive_erasure_with_gate(
        &self,
        plan_id: &str,
        confirmation_id: &str,
        operation_id: &str,
        typed_target_confirmation: &str,
        warning_acknowledged: bool,
        session_token: &str,
        execution_mode: ExecutionMode,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(DriveEraseProgress) + Send + Sync)>,
    ) -> Result<DriveEraseResult, LocardError> {
        let plan = self.get_plan(plan_id).await?;
        let started_at = Utc::now().to_rfc3339();

        // 1. Validate requested execution mode matches plan
        if plan.execution_mode != execution_mode {
            return Err(LocardError::SecurityViolation(format!(
                "Execution mode mismatch: plan specifies {:?}, execution requested {:?}",
                plan.execution_mode, execution_mode
            )));
        }

        // 2. Request permit through the RealHardwareExecutionGate
        let permit = self
            .execution_gate
            .request_permit(
                &plan,
                operation_id,
                typed_target_confirmation,
                warning_acknowledged,
                self.hardware_provider.as_ref(),
            )
            .map_err(|e| match e {
                DriveEraseFailureReason::LogicalVolumeTargetRejected(msg)
                | DriveEraseFailureReason::SystemOrBootDeviceProtected(msg)
                | DriveEraseFailureReason::SecurityViolation(msg)
                | DriveEraseFailureReason::RealHardwareExecutionNotEnabled(msg) => {
                    LocardError::SecurityViolation(msg)
                }
                other => LocardError::Operation(other.to_string()),
            })?;

        // 3. Audit execution start
        let _ = self.audit.log_structured_event(
            "DRIVE_ERASURE_EXECUTION_STARTED",
            Some(session_token),
            Some(&plan.physical_device_id),
            &format!(
                "Beginning drive sanitization under permit '{}' for op {} (challenge {}) on device {} (Mode: {:?}, Method: {:?})",
                permit.permit_id(), operation_id, confirmation_id, plan.physical_device_id, execution_mode, plan.method
            ),
        );

        if !permit.identity_discrepancies().is_empty() {
            let _ = self.audit.log_structured_event(
                "IDENTITY_REVALIDATION_WARNING",
                Some(session_token),
                Some(&plan.physical_device_id),
                &format!(
                    "Drive identity revalidation accepted with discrepancies on '{}': {:?}",
                    plan.physical_device_id,
                    permit.identity_discrepancies()
                ),
            );
        }

        // 4. Dispatch based on execution mode
        let (bytes_processed, elapsed_seconds, verification, status, failure_reason, audit_events) =
            if execution_mode == ExecutionMode::Simulation {
                let sanitizer = resolve_sanitizer_backend(plan.method);
                let sim_result = sanitizer.execute(&permit, is_cancelled, on_progress);

                match sim_result {
                    Ok((bytes, elapsed)) => {
                        let verification = execute_simulated_verification(&plan, true);
                        (
                            bytes,
                            elapsed,
                            verification,
                            DriveEraseStatus::Completed,
                            None,
                            vec![
                                "DRIVE_ERASURE_EXECUTION_STARTED".to_string(),
                                "DRIVE_ERASURE_EXECUTION_COMPLETED".to_string(),
                                "DRIVE_ERASURE_VERIFICATION_COMPLETED".to_string(),
                            ],
                        )
                    }
                    Err(DriveEraseFailureReason::Cancelled) => {
                        let verification = execute_simulated_verification(&plan, false);
                        (
                            0,
                            0.0,
                            verification,
                            DriveEraseStatus::Cancelled,
                            Some(DriveEraseFailureReason::Cancelled),
                            vec!["DRIVE_ERASURE_CANCELLED".to_string()],
                        )
                    }
                    Err(err) => {
                        let verification = execute_simulated_verification(&plan, false);
                        (
                            0,
                            0.0,
                            verification,
                            DriveEraseStatus::Failed,
                            Some(err),
                            vec!["DRIVE_ERASURE_REJECTED".to_string()],
                        )
                    }
                }
            } else {
                // Real Hardware execution
                let executor: Arc<dyn DriveHardwareExecutor> = self
                    .executor
                    .clone()
                    .unwrap_or_else(|| Arc::new(RealDriveHardwareExecutor::new()));

                let sanitizer = RealHardwareSanitizer::with_components(
                    executor,
                    Arc::clone(&self.lock_registry),
                    Some(Arc::clone(&self.hardware_provider)),
                );

                let real_result = sanitizer.execute_and_verify(&permit, is_cancelled, on_progress);

                match real_result {
                    Ok((bytes, elapsed, verification)) => {
                        let status = match verification.outcome {
                            locardx_verification::sanitization::VerificationOutcome::Verified => {
                                DriveEraseStatus::Completed
                            }
                            locardx_verification::sanitization::VerificationOutcome::VerificationFailed => {
                                DriveEraseStatus::VerificationFailed
                            }
                            locardx_verification::sanitization::VerificationOutcome::UnableToVerify => {
                                DriveEraseStatus::UnableToVerify
                            }
                            _ => DriveEraseStatus::Unknown,
                        };

                        let failure_reason = match status {
                            DriveEraseStatus::VerificationFailed => {
                                Some(DriveEraseFailureReason::VerificationFailed(
                                    verification.details.clone(),
                                ))
                            }
                            DriveEraseStatus::UnableToVerify => {
                                Some(DriveEraseFailureReason::UnableToVerify(
                                    verification.details.clone(),
                                ))
                            }
                            _ => None,
                        };

                        let audit_events = match status {
                            DriveEraseStatus::Completed => vec![
                                "DRIVE_ERASURE_EXECUTION_STARTED".to_string(),
                                "DRIVE_ERASURE_EXECUTION_COMPLETED".to_string(),
                                "DRIVE_ERASURE_VERIFICATION_COMPLETED".to_string(),
                            ],
                            DriveEraseStatus::VerificationFailed => vec![
                                "DRIVE_ERASURE_EXECUTION_STARTED".to_string(),
                                "DRIVE_ERASURE_EXECUTION_COMPLETED".to_string(),
                                "DRIVE_ERASURE_VERIFICATION_FAILED".to_string(),
                            ],
                            _ => vec![
                                "DRIVE_ERASURE_EXECUTION_STARTED".to_string(),
                                "DRIVE_ERASURE_EXECUTION_COMPLETED".to_string(),
                                "DRIVE_ERASURE_UNABLE_TO_VERIFY".to_string(),
                            ],
                        };

                        (
                            bytes,
                            elapsed,
                            verification,
                            status,
                            failure_reason,
                            audit_events,
                        )
                    }
                    Err(DriveEraseFailureReason::Cancelled) => {
                        let verification = execute_simulated_verification(&plan, false);
                        (
                            0,
                            0.0,
                            verification,
                            DriveEraseStatus::Cancelled,
                            Some(DriveEraseFailureReason::Cancelled),
                            vec!["DRIVE_ERASURE_CANCELLED".to_string()],
                        )
                    }
                    Err(err) => {
                        let verification = execute_simulated_verification(&plan, false);
                        (
                            0,
                            0.0,
                            verification,
                            DriveEraseStatus::Failed,
                            Some(err),
                            vec!["DRIVE_ERASURE_REJECTED".to_string()],
                        )
                    }
                }
            };

        // 5. Audit final outcome
        for event in &audit_events {
            let _ = self.audit.log_structured_event(
                event,
                Some(session_token),
                Some(&plan.physical_device_id),
                &format!(
                    "Drive erasure status {:?} for device {} under permit '{}'",
                    status,
                    plan.physical_device_id,
                    permit.permit_id()
                ),
            );
        }

        let completed_at = Utc::now().to_rfc3339();
        let res = DriveEraseResult {
            operation_id: operation_id.to_string(),
            plan_id: plan.plan_id.clone(),
            physical_device_id: plan.physical_device_id.clone(),
            display_name: plan.display_name.clone(),
            vendor: plan.vendor.clone(),
            model: plan.model.clone(),
            serial_number: plan.serial_number.clone(),
            media_type: plan.media_type,
            capacity_bytes: plan.capacity_bytes,
            sector_size: plan.sector_size,
            method: plan.method,
            execution_mode,
            status,
            bytes_processed,
            elapsed_seconds,
            verification,
            failure_reason,
            audit_references: audit_events,
            started_at,
            completed_at,
            limitations: plan.limitations,
            identity_discrepancies: permit.identity_discrepancies().to_vec(),
        };

        self.record_result(&res)?;
        Ok(res)
    }

    /// Recovers drive erasure operations interrupted by an abnormal application exit or system crash.
    ///
    /// Per LocardX safety requirements:
    /// - Interrupted drive erasure operations must NEVER silently resume.
    /// - Incomplete operations are marked `Unknown` with `UnableToVerify` outcome.
    /// - Cryptographic audit events are generated.
    /// - Fresh validation and authorization is strictly required.
    pub fn recover_interrupted_erasures(&self) -> Result<usize, LocardError> {
        let interrupted = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT record_id, operation_id, physical_device_id
                 FROM drive_erasure_records
                 WHERE status IN ('Executing', 'Simulating', 'PreExecutionCheck', 'ExecutionReady', 'Verifying')",
            )?;
            let mut rows = stmt.query([])?;
            let mut list = Vec::new();
            while let Some(row) = rows.next()? {
                let rec_id: String = row.get(0)?;
                let op_id: String = row.get(1)?;
                let dev_id: String = row.get(2)?;
                list.push((rec_id, op_id, dev_id));
            }
            Ok(list)
        })?;

        if interrupted.is_empty() {
            return Ok(0);
        }

        let completed_at = Utc::now().to_rfc3339();
        let reason = "Application terminated during execution (interrupted). Final physical media state is unverified. Fresh validation and authorization required.";

        for (_rec_id, op_id, dev_id) in &interrupted {
            self.db.with_conn(|conn| {
                conn.execute(
                    "UPDATE drive_erasure_records SET
                        status = 'Unknown',
                        verification_outcome = 'UnableToVerify',
                        failure_reason = ?1,
                        completed_at = ?2
                     WHERE operation_id = ?3",
                    params![reason, completed_at, op_id],
                )?;
                Ok(())
            })?;

            let _ = self.audit.log_structured_event(
                "DRIVE_ERASURE_INTERRUPTED_RECOVERED",
                None,
                Some(dev_id),
                &format!(
                    "Drive erasure operation '{}' on device '{}' marked Unknown after unexpected application termination",
                    op_id, dev_id
                ),
            );
        }

        Ok(interrupted.len())
    }

    /// Persists a drive erasure result record in SQLite.
    fn record_result(&self, result: &DriveEraseResult) -> Result<(), LocardError> {
        let record_id = Uuid::new_v4().to_string();
        let snapshot_json = serde_json::to_string(&result.physical_device_id).unwrap_or_default();
        let capabilities_json = serde_json::to_string(&result.method).unwrap_or_default();
        let audit_json = serde_json::to_string(&result.audit_references).unwrap_or_default();
        let failure_str = result.failure_reason.as_ref().map(|f| f.to_string());
        let evidence_digest = format!("ev-{}", &result.operation_id);

        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO drive_erasure_records (
                    record_id, operation_id, actor_id, physical_device_id, display_name,
                    vendor, model, serial_number, media_type, capacity_bytes, sector_size,
                    device_snapshot_json, capabilities_json, sanitization_method, execution_mode,
                    verification_strategy, verification_outcome, status, bytes_processed,
                    elapsed_seconds, failure_reason, audit_references, started_at, completed_at,
                    plan_id, bus_type, evidence_digest
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)",
                params![
                    record_id,
                    result.operation_id,
                    None as Option<String>,
                    result.physical_device_id,
                    result.display_name,
                    result.vendor,
                    result.model,
                    result.serial_number,
                    format!("{:?}", result.media_type),
                    result.capacity_bytes as i64,
                    result.sector_size as i64,
                    snapshot_json,
                    capabilities_json,
                    result.method.to_string(),
                    result.execution_mode.to_string(),
                    result.verification.strategy.to_string(),
                    result.verification.outcome.to_string(),
                    result.status.to_string(),
                    result.bytes_processed as i64,
                    result.elapsed_seconds,
                    failure_str,
                    audit_json,
                    result.started_at,
                    result.completed_at,
                    result.plan_id,
                    None as Option<String>,
                    evidence_digest,
                ],
            )?;
            Ok(())
        })
    }

    /// Fetches a recorded drive erasure result by operation ID.
    pub fn get_drive_erasure_result(
        &self,
        operation_id: &str,
    ) -> Result<Option<DriveEraseResult>, LocardError> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT
                    operation_id, physical_device_id, display_name, vendor, model,
                    serial_number, media_type, capacity_bytes, sector_size,
                    sanitization_method, execution_mode, verification_strategy,
                    verification_outcome, status, bytes_processed, elapsed_seconds,
                    failure_reason, started_at, completed_at, plan_id
                FROM drive_erasure_records
                WHERE operation_id = ?1",
            )?;

            let mut rows = stmt.query(params![operation_id])?;
            if let Some(row) = rows.next()? {
                let op_id: String = row.get(0)?;
                let physical_id: String = row.get(1)?;
                let display: String = row.get(2)?;
                let vendor: Option<String> = row.get(3)?;
                let model: Option<String> = row.get(4)?;
                let serial: Option<String> = row.get(5)?;
                let media_str: String = row.get(6)?;
                let capacity: i64 = row.get(7)?;
                let sector: i64 = row.get(8)?;
                let method_str: String = row.get(9)?;
                let mode_str: String = row.get(10)?;
                let v_strat_str: String = row.get(11)?;
                let v_out_str: String = row.get(12)?;
                let status_str: String = row.get(13)?;
                let bytes: i64 = row.get(14)?;
                let elapsed: f64 = row.get(15)?;
                let failure: Option<String> = row.get(16)?;
                let started: String = row.get(17)?;
                let completed: String = row.get(18)?;
                let plan_id: Option<String> = row.get(19)?;

                let media_type = match media_str.as_str() {
                    "Hdd" => locardx_device_manager::DeviceType::Hdd,
                    "Ssd" => locardx_device_manager::DeviceType::Ssd,
                    "Usb" => locardx_device_manager::DeviceType::Usb,
                    "MemoryCard" => locardx_device_manager::DeviceType::MemoryCard,
                    "ExternalStorage" => locardx_device_manager::DeviceType::ExternalStorage,
                    _ => locardx_device_manager::DeviceType::Unknown,
                };

                let method = match method_str.as_str() {
                    "Dod522022M" => crate::models::DriveSanitizationMethod::Dod522022M,
                    "AtaSecureErase" => crate::models::DriveSanitizationMethod::AtaSecureErase,
                    "NvmeFormatSanitize" => {
                        crate::models::DriveSanitizationMethod::NvmeFormatSanitize
                    }
                    "NvmeCryptoErase" => crate::models::DriveSanitizationMethod::NvmeCryptoErase,
                    _ => crate::models::DriveSanitizationMethod::Nist80088ClearZero,
                };

                let status = match status_str.as_str() {
                    "Completed" => DriveEraseStatus::Completed,
                    "Cancelled" => DriveEraseStatus::Cancelled,
                    "Failed" => DriveEraseStatus::Failed,
                    "Authorized" => DriveEraseStatus::Authorized,
                    "VerificationFailed" => DriveEraseStatus::VerificationFailed,
                    "UnableToVerify" => DriveEraseStatus::UnableToVerify,
                    "Unknown" => DriveEraseStatus::Unknown,
                    _ => DriveEraseStatus::Planned,
                };

                let v_strat = match v_strat_str.as_str() {
                    "FullDeviceReadVerify" => {
                        crate::models::DriveVerificationStrategy::FullDeviceReadVerify
                    }
                    "FirmwareStatusVerify" => {
                        crate::models::DriveVerificationStrategy::FirmwareStatusVerify
                    }
                    "CryptoKeyDestructionCheck" => {
                        crate::models::DriveVerificationStrategy::CryptoKeyDestructionCheck
                    }
                    "SampledSectorVerification" => {
                        crate::models::DriveVerificationStrategy::SampledSectorVerification
                    }
                    _ => crate::models::DriveVerificationStrategy::NotApplicable,
                };

                let v_out = match v_out_str.as_str() {
                    "Verified" => locardx_verification::sanitization::VerificationOutcome::Verified,
                    "VerificationFailed" => {
                        locardx_verification::sanitization::VerificationOutcome::VerificationFailed
                    }
                    "UnableToVerify" => {
                        locardx_verification::sanitization::VerificationOutcome::UnableToVerify
                    }
                    _ => locardx_verification::sanitization::VerificationOutcome::NotApplicable,
                };

                Ok(Some(DriveEraseResult {
                    operation_id: op_id,
                    plan_id: plan_id.unwrap_or_default(),
                    physical_device_id: physical_id,
                    display_name: display,
                    vendor,
                    model,
                    serial_number: serial,
                    media_type,
                    capacity_bytes: capacity as u64,
                    sector_size: sector as u32,
                    method,
                    execution_mode: if mode_str == "Simulation" {
                        ExecutionMode::Simulation
                    } else {
                        ExecutionMode::RealHardware
                    },
                    status,
                    bytes_processed: bytes as u64,
                    elapsed_seconds: elapsed,
                    verification: DriveVerificationResult {
                        outcome: v_out,
                        strategy: v_strat,
                        details: "Queried from database record".to_string(),
                        verified_at: completed.clone(),
                    },
                    failure_reason: failure.map(DriveEraseFailureReason::SimulationError),
                    audit_references: vec![],
                    started_at: started,
                    completed_at: completed,
                    limitations: vec![],
                    identity_discrepancies: vec![],
                }))
            } else {
                Ok(None)
            }
        })
    }
}
