use crate::cancellation::CancellationToken;
use crate::executor::{
    DisabledOperationExecutor, DriveEraserSimulationExecutor, FileEraserExecutor,
    FolderEraserExecutor, IntegrityHashExecutor, IntegrityVerifyExecutor, OperationExecutor,
    ProgressCallback, SanitizationPlanExecutor,
};
use crate::models::{Operation, OperationProgress, OperationState, OperationType};
use chrono::Utc;
use locardx_audit::AuditService;
use locardx_common::{LocardError, TargetIdentity, TargetType};
use locardx_database::Database;
use locardx_verification::{IntegrityService, SanitizationService};
use rusqlite::params;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Orchestration service managing the lifecycle, execution, cancellation,
/// and audit correlation of forensic and sanitization operations.
pub struct OperationManager {
    db: Arc<Database>,
    audit: Arc<AuditService>,
    integrity: Arc<IntegrityService>,
    sanitization: Option<Arc<SanitizationService>>,
    safety: Option<Arc<locardx_security::SafetyEngine>>,
    file_eraser: Option<Arc<locardx_file_eraser::FileEraserService>>,
    drive_eraser: Option<Arc<locardx_drive_eraser::DriveEraserService>>,
    active_tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
    active_progress: Arc<RwLock<HashMap<String, OperationProgress>>>,
}

impl OperationManager {
    pub fn new(
        db: Arc<Database>,
        audit: Arc<AuditService>,
        integrity: Arc<IntegrityService>,
    ) -> Self {
        Self {
            db,
            audit,
            integrity,
            sanitization: None,
            safety: None,
            file_eraser: None,
            drive_eraser: None,
            active_tokens: Arc::new(RwLock::new(HashMap::new())),
            active_progress: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Attaches the Sanitization Planning service.
    pub fn with_sanitization(mut self, sanitization: Arc<SanitizationService>) -> Self {
        self.sanitization = Some(sanitization);
        self
    }

    /// Attaches the Safety & Authorization Interlock engine.
    pub fn with_safety(mut self, safety: Arc<locardx_security::SafetyEngine>) -> Self {
        self.safety = Some(safety);
        self
    }

    /// Attaches the File Eraser service.
    pub fn with_file_eraser(
        mut self,
        file_eraser: Arc<locardx_file_eraser::FileEraserService>,
    ) -> Self {
        self.file_eraser = Some(file_eraser);
        self
    }

    /// Attaches the Drive Eraser simulation service.
    pub fn with_drive_eraser(
        mut self,
        drive_eraser: Arc<locardx_drive_eraser::DriveEraserService>,
    ) -> Self {
        self.drive_eraser = Some(drive_eraser);
        self
    }

    /// Creates and persists a new operation record in `Created` state.
    pub fn create_operation(
        &self,
        operation_type: OperationType,
        target: TargetIdentity,
        actor_id: Option<String>,
    ) -> Result<Operation, LocardError> {
        let operation_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let current_state = OperationState::Created;
        let progress = OperationProgress::default();

        let operation = Operation {
            operation_id: operation_id.clone(),
            operation_type,
            target: target.clone(),
            actor_id: actor_id.clone(),
            current_state,
            progress: progress.clone(),
            result_summary: None,
            failure_reason: None,
            cancellation_requested: false,
            created_at: created_at.clone(),
            started_at: None,
            completed_at: None,
        };

        // 1. Persist to SQLite
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO operations (
                    operation_id, operation_type, target_type, target_identifier, target_display_name,
                    target_size_bytes, actor_id, current_state, progress_percentage,
                    progress_bytes_processed, progress_total_bytes, progress_stage,
                    progress_message, result_summary, failure_reason, cancellation_requested,
                    created_at, started_at, completed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                params![
                    operation.operation_id,
                    operation.operation_type.to_string(),
                    operation.target.target_type.to_string(),
                    operation.target.identifier,
                    operation.target.display_name,
                    operation.target.size_bytes.map(|s| s as i64),
                    operation.actor_id,
                    operation.current_state.to_string(),
                    operation.progress.percentage.map(|p| p as f64),
                    operation.progress.bytes_processed.map(|b| b as i64),
                    operation.progress.total_bytes.map(|b| b as i64),
                    operation.progress.stage,
                    operation.progress.message,
                    operation.result_summary,
                    operation.failure_reason,
                    0i32,
                    operation.created_at,
                    operation.started_at,
                    operation.completed_at,
                ],
            )?;
            Ok(())
        })?;

        // 2. Register in tamper-evident audit hash-chain
        self.audit.log_structured_event(
            "OPERATION_CREATED",
            actor_id.as_deref(),
            Some(&target.identifier),
            &format!(
                "Created operation {} ({}) for target '{}'",
                operation_id, operation_type, target.display_name
            ),
        )?;

        info!(
            operation_id = %operation_id,
            op_type = %operation_type,
            target = %target.display_name,
            "Operation created"
        );

        Ok(operation)
    }

    /// Submits an operation, marks it `Queued` -> `Running`, and dispatches it asynchronously via Tokio.
    pub async fn submit_and_run(
        &self,
        operation_type: OperationType,
        target: TargetIdentity,
        actor_id: Option<String>,
        parameters: Option<serde_json::Value>,
    ) -> Result<Operation, LocardError> {
        // Enforce execution boundary: destructive and carving types are not executable
        // Exception: DriveErasure is permitted in simulation mode when drive_eraser is explicitly configured
        let is_simulated_drive_erasure =
            operation_type == OperationType::DriveErasure && self.drive_eraser.is_some();
        if !operation_type.is_executable() && !is_simulated_drive_erasure {
            return Err(LocardError::Operation(format!(
                "Operation type '{}' is disabled / not implemented in this build.",
                operation_type
            )));
        }

        // Enforce safety interlocks if safety engine is attached
        if let Some(safety) = &self.safety {
            let decision = safety.evaluate_safety(&target, operation_type, None)?;
            if decision.decision == locardx_security::SafetyDecisionOutcome::Blocked
                || decision.decision == locardx_security::SafetyDecisionOutcome::Denied
            {
                return Err(LocardError::SecurityViolation(format!(
                    "Safety interlock rejected execution: {} ({})",
                    decision.message, decision.reason_code
                )));
            }
        }

        // 1. Create operation in Created state
        let mut op = self.create_operation(operation_type, target, actor_id)?;
        let op_id = op.operation_id.clone();

        // 2. Transition Created -> Queued -> Running
        self.transition_state(&op_id, OperationState::Queued, None, None)?;
        let started_at = Utc::now().to_rfc3339();
        self.transition_state(&op_id, OperationState::Running, Some(&started_at), None)?;

        op.current_state = OperationState::Running;
        op.started_at = Some(started_at);

        // 3. Register cancellation token
        let token = CancellationToken::new();
        {
            let mut tokens = self.active_tokens.write().await;
            tokens.insert(op_id.clone(), token.clone());
        }

        // 4. Instantiate executor
        let executor: Arc<dyn OperationExecutor> = match operation_type {
            OperationType::IntegrityHash => {
                Arc::new(IntegrityHashExecutor::new(Arc::clone(&self.integrity)))
            }
            OperationType::IntegrityVerify => {
                Arc::new(IntegrityVerifyExecutor::new(Arc::clone(&self.integrity)))
            }
            OperationType::SanitizationPlanEvaluation => {
                if let Some(sanitization) = &self.sanitization {
                    Arc::new(SanitizationPlanExecutor::new(Arc::clone(sanitization)))
                } else {
                    return Err(LocardError::Operation(
                        "SanitizationService not configured on OperationManager".to_string(),
                    ));
                }
            }
            OperationType::FileErasure => {
                if let Some(eraser) = &self.file_eraser {
                    Arc::new(FileEraserExecutor::new(Arc::clone(eraser)))
                } else {
                    return Err(LocardError::Operation(
                        "FileEraserService not configured on OperationManager".to_string(),
                    ));
                }
            }
            OperationType::FolderErasure => {
                if let Some(eraser) = &self.file_eraser {
                    Arc::new(FolderEraserExecutor::new(Arc::clone(eraser)))
                } else {
                    return Err(LocardError::Operation(
                        "FileEraserService not configured on OperationManager".to_string(),
                    ));
                }
            }
            OperationType::DriveErasure => {
                if let Some(eraser) = &self.drive_eraser {
                    Arc::new(DriveEraserSimulationExecutor::new(Arc::clone(eraser)))
                } else {
                    return Err(LocardError::Operation(
                        "DriveEraserService not configured on OperationManager".to_string(),
                    ));
                }
            }
            other => Arc::new(DisabledOperationExecutor::new(other)),
        };

        // 5. Setup progress callback
        let active_progress_map = Arc::clone(&self.active_progress);
        let op_id_for_progress = op_id.clone();
        let db_for_progress = Arc::clone(&self.db);

        let on_progress: ProgressCallback = Arc::new(move |progress: OperationProgress| {
            let active_progress_map = Arc::clone(&active_progress_map);
            let op_id = op_id_for_progress.clone();
            let db = Arc::clone(&db_for_progress);
            let p_clone = progress.clone();

            // Fire-and-forget async update of in-memory cache and SQLite
            tokio::spawn(async move {
                {
                    let mut map = active_progress_map.write().await;
                    map.insert(op_id.clone(), p_clone.clone());
                }

                let _ = db.with_conn(|conn| {
                    conn.execute(
                        "UPDATE operations SET
                            progress_percentage = ?1,
                            progress_bytes_processed = ?2,
                            progress_total_bytes = ?3,
                            progress_stage = ?4,
                            progress_message = ?5
                        WHERE operation_id = ?6",
                        params![
                            p_clone.percentage.map(|p| p as f64),
                            p_clone.bytes_processed.map(|b| b as i64),
                            p_clone.total_bytes.map(|b| b as i64),
                            p_clone.stage,
                            p_clone.message,
                            op_id
                        ],
                    )?;
                    Ok(())
                });
            });
        });

        // 6. Spawn asynchronous background task
        let op_clone = op.clone();
        let db_complete = Arc::clone(&self.db);
        let audit_complete = Arc::clone(&self.audit);
        let active_tokens_complete = Arc::clone(&self.active_tokens);
        let active_progress_complete = Arc::clone(&self.active_progress);

        tokio::spawn(async move {
            let exec_res = executor
                .execute(&op_clone, parameters, token.clone(), on_progress)
                .await;

            let completed_at = Utc::now().to_rfc3339();

            if token.is_cancelled() {
                let _ = db_complete.with_conn(|conn| {
                    conn.execute(
                        "UPDATE operations SET
                            current_state = 'Cancelled',
                            completed_at = ?1
                        WHERE operation_id = ?2",
                        params![completed_at, op_clone.operation_id],
                    )?;
                    Ok(())
                });

                let _ = audit_complete.log_structured_event(
                    "OPERATION_CANCELLED",
                    op_clone.actor_id.as_deref(),
                    Some(&op_clone.target.identifier),
                    &format!(
                        "Operation {} ({}) cancelled by operator",
                        op_clone.operation_id, op_clone.operation_type
                    ),
                );

                info!(operation_id = %op_clone.operation_id, "Operation cancelled");
            } else {
                match exec_res {
                    Ok(summary) => {
                        let _ = db_complete.with_conn(|conn| {
                            conn.execute(
                                "UPDATE operations SET
                                    current_state = 'Completed',
                                    result_summary = ?1,
                                    completed_at = ?2
                                WHERE operation_id = ?3",
                                params![summary, completed_at, op_clone.operation_id],
                            )?;
                            Ok(())
                        });

                        let _ = audit_complete.log_structured_event(
                            "OPERATION_COMPLETED",
                            op_clone.actor_id.as_deref(),
                            Some(&op_clone.target.identifier),
                            &format!(
                                "Operation {} ({}) completed: {}",
                                op_clone.operation_id, op_clone.operation_type, summary
                            ),
                        );

                        info!(operation_id = %op_clone.operation_id, "Operation completed successfully");
                    }
                    Err(err) => {
                        let err_msg = err.to_string();
                        let _ = db_complete.with_conn(|conn| {
                            conn.execute(
                                "UPDATE operations SET
                                    current_state = 'Failed',
                                    failure_reason = ?1,
                                    completed_at = ?2
                                WHERE operation_id = ?3",
                                params![err_msg, completed_at, op_clone.operation_id],
                            )?;
                            Ok(())
                        });

                        let _ = audit_complete.log_structured_event(
                            "OPERATION_FAILED",
                            op_clone.actor_id.as_deref(),
                            Some(&op_clone.target.identifier),
                            &format!(
                                "Operation {} ({}) failed: {}",
                                op_clone.operation_id, op_clone.operation_type, err_msg
                            ),
                        );

                        warn!(operation_id = %op_clone.operation_id, error = %err_msg, "Operation failed");
                    }
                }
            }

            // Cleanup active maps
            {
                let mut tokens = active_tokens_complete.write().await;
                tokens.remove(&op_clone.operation_id);
            }
            {
                let mut progress = active_progress_complete.write().await;
                progress.remove(&op_clone.operation_id);
            }
        });

        Ok(op)
    }

    /// Requests cooperative cancellation of an active or queued operation.
    pub async fn request_cancellation(
        &self,
        operation_id: &str,
        actor_id: Option<&str>,
    ) -> Result<(), LocardError> {
        let op = self.get_operation(operation_id)?;

        if op.current_state.is_terminal() {
            return Err(LocardError::Operation(format!(
                "Cannot cancel operation {} in terminal state '{}'",
                operation_id, op.current_state
            )));
        }

        // Trigger active token if in-flight
        {
            let tokens = self.active_tokens.read().await;
            if let Some(token) = tokens.get(operation_id) {
                token.cancel();
            }
        }

        // State update based on current state
        let target_state = match op.current_state {
            OperationState::Created | OperationState::Queued => OperationState::Cancelled,
            OperationState::Running => OperationState::Cancelling,
            OperationState::Cancelling => return Ok(()), // Already cancelling
            _ => return Ok(()),
        };

        self.db.with_conn(|conn| {
            conn.execute(
                "UPDATE operations SET
                    cancellation_requested = 1,
                    current_state = ?1
                WHERE operation_id = ?2",
                params![target_state.to_string(), operation_id],
            )?;
            Ok(())
        })?;

        self.audit.log_structured_event(
            "OPERATION_CANCELLATION_REQUESTED",
            actor_id,
            Some(&op.target.identifier),
            &format!(
                "Cancellation requested for operation {} ({})",
                operation_id, op.operation_type
            ),
        )?;

        info!(operation_id = %operation_id, "Cancellation requested");
        Ok(())
    }

    /// Recovers operations interrupted by an unexpected application termination or crash.
    ///
    /// Per LocardX safety policy:
    /// - In-flight destructive or sensitive operations must NEVER silently resume or auto-retry.
    /// - Interrupted operations transition to `Failed` with explicit failure reasons.
    /// - Cryptographic audit events are recorded for each recovered operation.
    /// - Fresh validation and operator authorization is strictly required for any future attempt.
    pub fn recover_interrupted_operations(&self) -> Result<usize, LocardError> {
        let interrupted = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT operation_id, operation_type, target_identifier, actor_id
                 FROM operations
                 WHERE current_state IN ('Running', 'Queued', 'Cancelling')",
            )?;
            let mut rows = stmt.query([])?;
            let mut list = Vec::new();
            while let Some(row) = rows.next()? {
                let op_id: String = row.get(0)?;
                let op_type: String = row.get(1)?;
                let target_id: String = row.get(2)?;
                let actor: Option<String> = row.get(3)?;
                list.push((op_id, op_type, target_id, actor));
            }
            Ok(list)
        })?;

        if interrupted.is_empty() {
            return Ok(0);
        }

        let completed_at = Utc::now().to_rfc3339();
        let reason = "Application terminated during execution (interrupted). Fresh validation and authorization required.";

        for (op_id, op_type, target_id, actor) in &interrupted {
            self.db.with_conn(|conn| {
                conn.execute(
                    "UPDATE operations SET
                        current_state = 'Failed',
                        failure_reason = ?1,
                        completed_at = ?2
                     WHERE operation_id = ?3",
                    params![reason, completed_at, op_id],
                )?;
                Ok(())
            })?;

            let _ = self.audit.log_structured_event(
                "OPERATION_INTERRUPTED_RECOVERED",
                actor.as_deref(),
                Some(target_id),
                &format!(
                    "Operation {} ({}) marked Failed after unexpected application termination",
                    op_id, op_type
                ),
            );

            warn!(
                operation_id = %op_id,
                operation_type = %op_type,
                "Interrupted operation recovered and marked Failed"
            );
        }

        Ok(interrupted.len())
    }

    /// Fetches a single operation by ID, merging active in-memory progress telemetry.
    pub fn get_operation(&self, operation_id: &str) -> Result<Operation, LocardError> {
        let mut op = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT
                    operation_id, operation_type, target_type, target_identifier, target_display_name,
                    target_size_bytes, actor_id, current_state, progress_percentage,
                    progress_bytes_processed, progress_total_bytes, progress_stage,
                    progress_message, result_summary, failure_reason, cancellation_requested,
                    created_at, started_at, completed_at
                FROM operations
                WHERE operation_id = ?1",
            )?;

            stmt.query_row(params![operation_id], |row| {
                let op_type_str: String = row.get(1)?;
                let target_type_str: String = row.get(2)?;
                let state_str: String = row.get(7)?;

                let op_type = OperationType::from_str(&op_type_str)
                    .unwrap_or(OperationType::Unknown);
                let target_type = match target_type_str.as_str() {
                    "File" => TargetType::File,
                    "Directory" => TargetType::Directory,
                    "LogicalVolume" => TargetType::LogicalVolume,
                    "PhysicalDevice" => TargetType::PhysicalDevice,
                    "EvidenceObject" => TargetType::EvidenceObject,
                    _ => TargetType::Unknown,
                };
                let current_state = OperationState::from_str(&state_str)
                    .unwrap_or(OperationState::Failed);

                let size_bytes: Option<i64> = row.get(5)?;
                let progress_pct: Option<f64> = row.get(8)?;
                let progress_bytes: Option<i64> = row.get(9)?;
                let progress_total: Option<i64> = row.get(10)?;
                let cancel_req_int: i32 = row.get(15)?;

                Ok(Operation {
                    operation_id: row.get(0)?,
                    operation_type: op_type,
                    target: TargetIdentity {
                        target_type,
                        identifier: row.get(3)?,
                        display_name: row.get(4)?,
                        size_bytes: size_bytes.map(|s| s as u64),
                    },
                    actor_id: row.get(6)?,
                    current_state,
                    progress: OperationProgress {
                        percentage: progress_pct.map(|p| p as f32),
                        bytes_processed: progress_bytes.map(|b| b as u64),
                        total_bytes: progress_total.map(|b| b as u64),
                        stage: row.get(11)?,
                        message: row.get(12)?,
                        eta_seconds: None,
                    },
                    result_summary: row.get(13)?,
                    failure_reason: row.get(14)?,
                    cancellation_requested: cancel_req_int != 0,
                    created_at: row.get(16)?,
                    started_at: row.get(17)?,
                    completed_at: row.get(18)?,
                })
            })
        })?;

        // Merge latest active in-memory progress if operation is running
        if op.current_state == OperationState::Running {
            if let Ok(guard) = self.active_progress.try_read() {
                if let Some(active_p) = guard.get(operation_id) {
                    op.progress = active_p.clone();
                }
            }
        }

        Ok(op)
    }

    /// Lists recent operations, supporting optional filtering by state and type.
    pub fn list_operations(
        &self,
        limit: usize,
        state_filter: Option<&str>,
        type_filter: Option<&str>,
    ) -> Result<Vec<Operation>, LocardError> {
        let operations = self.db.with_conn(|conn| {
            let mut query = "SELECT
                operation_id, operation_type, target_type, target_identifier, target_display_name,
                target_size_bytes, actor_id, current_state, progress_percentage,
                progress_bytes_processed, progress_total_bytes, progress_stage,
                progress_message, result_summary, failure_reason, cancellation_requested,
                created_at, started_at, completed_at
            FROM operations WHERE 1=1"
                .to_string();

            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(state) = state_filter {
                params_vec.push(Box::new(state.to_string()));
                query.push_str(&format!(" AND current_state = ?{}", params_vec.len()));
            }
            if let Some(op_type) = type_filter {
                params_vec.push(Box::new(op_type.to_string()));
                query.push_str(&format!(" AND operation_type = ?{}", params_vec.len()));
            }
            params_vec.push(Box::new(limit as i64));
            query.push_str(&format!(
                " ORDER BY created_at DESC LIMIT ?{}",
                params_vec.len()
            ));

            let mut stmt = conn.prepare(&query)?;
            let params_slice: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|b| b.as_ref()).collect();
            let mut rows = stmt.query(&*params_slice)?;

            let mut list = Vec::new();
            while let Some(row) = rows.next()? {
                let op_type_str: String = row.get(1)?;
                let target_type_str: String = row.get(2)?;
                let state_str: String = row.get(7)?;

                let op_type =
                    OperationType::from_str(&op_type_str).unwrap_or(OperationType::Unknown);
                let target_type = match target_type_str.as_str() {
                    "File" => TargetType::File,
                    "Directory" => TargetType::Directory,
                    "LogicalVolume" => TargetType::LogicalVolume,
                    "PhysicalDevice" => TargetType::PhysicalDevice,
                    "EvidenceObject" => TargetType::EvidenceObject,
                    _ => TargetType::Unknown,
                };
                let current_state =
                    OperationState::from_str(&state_str).unwrap_or(OperationState::Failed);

                let size_bytes: Option<i64> = row.get(5)?;
                let progress_pct: Option<f64> = row.get(8)?;
                let progress_bytes: Option<i64> = row.get(9)?;
                let progress_total: Option<i64> = row.get(10)?;
                let cancel_req_int: i32 = row.get(15)?;

                list.push(Operation {
                    operation_id: row.get(0)?,
                    operation_type: op_type,
                    target: TargetIdentity {
                        target_type,
                        identifier: row.get(3)?,
                        display_name: row.get(4)?,
                        size_bytes: size_bytes.map(|s| s as u64),
                    },
                    actor_id: row.get(6)?,
                    current_state,
                    progress: OperationProgress {
                        percentage: progress_pct.map(|p| p as f32),
                        bytes_processed: progress_bytes.map(|b| b as u64),
                        total_bytes: progress_total.map(|b| b as u64),
                        stage: row.get(11)?,
                        message: row.get(12)?,
                        eta_seconds: None,
                    },
                    result_summary: row.get(13)?,
                    failure_reason: row.get(14)?,
                    cancellation_requested: cancel_req_int != 0,
                    created_at: row.get(16)?,
                    started_at: row.get(17)?,
                    completed_at: row.get(18)?,
                });
            }

            Ok(list)
        })?;

        // Merge active in-memory progress for running tasks
        if let Ok(guard) = self.active_progress.try_read() {
            let mut enriched = Vec::with_capacity(operations.len());
            for mut op in operations {
                if op.current_state == OperationState::Running {
                    if let Some(active_p) = guard.get(&op.operation_id) {
                        op.progress = active_p.clone();
                    }
                }
                enriched.push(op);
            }
            Ok(enriched)
        } else {
            Ok(operations)
        }
    }

    /// Internal helper validating and executing state transitions.
    fn transition_state(
        &self,
        operation_id: &str,
        next_state: OperationState,
        started_at: Option<&str>,
        completed_at: Option<&str>,
    ) -> Result<(), LocardError> {
        let current = self.get_operation(operation_id)?;
        if !current.current_state.can_transition_to(next_state) {
            return Err(LocardError::Operation(format!(
                "Invalid state transition for operation {}: cannot move from '{}' to '{}'",
                operation_id, current.current_state, next_state
            )));
        }

        self.db.with_conn(|conn| {
            if let Some(s) = started_at {
                conn.execute(
                    "UPDATE operations SET current_state = ?1, started_at = ?2 WHERE operation_id = ?3",
                    params![next_state.to_string(), s, operation_id],
                )?;
            } else if let Some(c) = completed_at {
                conn.execute(
                    "UPDATE operations SET current_state = ?1, completed_at = ?2 WHERE operation_id = ?3",
                    params![next_state.to_string(), c, operation_id],
                )?;
            } else {
                conn.execute(
                    "UPDATE operations SET current_state = ?1 WHERE operation_id = ?2",
                    params![next_state.to_string(), operation_id],
                )?;
            }
            Ok(())
        })?;

        let event_type = match next_state {
            OperationState::Queued => "OPERATION_QUEUED",
            OperationState::Running => "OPERATION_STARTED",
            OperationState::Cancelling => "OPERATION_CANCELLATION_REQUESTED",
            OperationState::Cancelled => "OPERATION_CANCELLED",
            OperationState::Completed => "OPERATION_COMPLETED",
            OperationState::Failed => "OPERATION_FAILED",
            OperationState::Created => "OPERATION_CREATED",
        };

        self.audit.log_structured_event(
            event_type,
            current.actor_id.as_deref(),
            Some(&current.target.identifier),
            &format!(
                "Operation {} transitioned: {} -> {}",
                operation_id, current.current_state, next_state
            ),
        )?;

        Ok(())
    }
}
