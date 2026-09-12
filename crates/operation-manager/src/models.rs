pub use locardx_common::OperationType;
use locardx_common::{LocardError, TargetIdentity};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Lifecycle states of a managed operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationState {
    Created,
    Queued,
    Running,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
}

impl OperationState {
    /// Returns true if the state is terminal (cannot transition further).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Cancelled | Self::Completed | Self::Failed)
    }

    /// Validates whether a lifecycle state transition is legally permissible.
    pub fn can_transition_to(&self, next: OperationState) -> bool {
        match (self, next) {
            // Created can be queued or cancelled before starting
            (Self::Created, Self::Queued) => true,
            (Self::Created, Self::Cancelled) => true,

            // Queued can start running or be cancelled
            (Self::Queued, Self::Running) => true,
            (Self::Queued, Self::Cancelled) => true,

            // Running can finish, fail, or request cancellation
            (Self::Running, Self::Completed) => true,
            (Self::Running, Self::Failed) => true,
            (Self::Running, Self::Cancelling) => true,

            // Cancelling can only transition to Cancelled or Failed
            (Self::Cancelling, Self::Cancelled) => true,
            (Self::Cancelling, Self::Failed) => true,

            // Terminal states or any other transitions are strictly rejected
            _ => false,
        }
    }
}

impl fmt::Display for OperationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::Queued => write!(f, "Queued"),
            Self::Running => write!(f, "Running"),
            Self::Cancelling => write!(f, "Cancelling"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

impl std::str::FromStr for OperationState {
    type Err = LocardError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Created" => Ok(Self::Created),
            "Queued" => Ok(Self::Queued),
            "Running" => Ok(Self::Running),
            "Cancelling" => Ok(Self::Cancelling),
            "Cancelled" => Ok(Self::Cancelled),
            "Completed" => Ok(Self::Completed),
            "Failed" => Ok(Self::Failed),
            other => Err(LocardError::Operation(format!(
                "Unrecognized operation state: {}",
                other
            ))),
        }
    }
}

/// Progress telemetry for an active operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationProgress {
    /// Bounded completion percentage (0.0 ..= 100.0) or None if indeterminate
    pub percentage: Option<f32>,
    pub bytes_processed: Option<u64>,
    pub total_bytes: Option<u64>,
    pub stage: String,
    pub message: String,
    pub eta_seconds: Option<u64>,
}

impl OperationProgress {
    pub fn new(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            percentage: None,
            bytes_processed: None,
            total_bytes: None,
            stage: stage.into(),
            message: message.into(),
            eta_seconds: None,
        }
    }

    pub fn with_percentage(mut self, pct: f32) -> Result<Self, LocardError> {
        if !(0.0..=100.0).contains(&pct) {
            return Err(LocardError::Operation(format!(
                "Progress percentage out of bounds (must be 0.0 - 100.0): {}",
                pct
            )));
        }
        self.percentage = Some(pct);
        Ok(self)
    }

    pub fn with_bytes(mut self, processed: u64, total: Option<u64>) -> Self {
        self.bytes_processed = Some(processed);
        self.total_bytes = total;
        if let Some(total_b) = total {
            if total_b > 0 {
                let ratio = (processed as f64 / total_b as f64) * 100.0;
                let bounded = ratio.clamp(0.0, 100.0) as f32;
                self.percentage = Some(bounded);
            }
        }
        self
    }
}

impl Default for OperationProgress {
    fn default() -> Self {
        Self {
            percentage: None,
            bytes_processed: None,
            total_bytes: None,
            stage: "Initialized".to_string(),
            message: "Operation record created".to_string(),
            eta_seconds: None,
        }
    }
}

/// Core domain entity representing an orchestrated operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    pub operation_id: String,
    pub operation_type: OperationType,
    pub target: TargetIdentity,
    pub actor_id: Option<String>,
    pub current_state: OperationState,
    pub progress: OperationProgress,
    pub result_summary: Option<String>,
    pub failure_reason: Option<String>,
    pub cancellation_requested: bool,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

/// DTO for creating a new managed operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOperationRequest {
    pub operation_type: OperationType,
    pub target: TargetIdentity,
    pub parameters: Option<serde_json::Value>,
}

/// DTO representing an operation in API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationDto {
    pub operation_id: String,
    pub operation_type: OperationType,
    pub target: TargetIdentity,
    pub actor_id: Option<String>,
    pub current_state: OperationState,
    pub progress: OperationProgress,
    pub result_summary: Option<String>,
    pub failure_reason: Option<String>,
    pub cancellation_requested: bool,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl From<Operation> for OperationDto {
    fn from(op: Operation) -> Self {
        Self {
            operation_id: op.operation_id,
            operation_type: op.operation_type,
            target: op.target,
            actor_id: op.actor_id,
            current_state: op.current_state,
            progress: op.progress,
            result_summary: op.result_summary,
            failure_reason: op.failure_reason,
            cancellation_requested: op.cancellation_requested,
            created_at: op.created_at,
            started_at: op.started_at,
            completed_at: op.completed_at,
        }
    }
}
