use crate::models::{Permission, UserRole};
use locardx_common::{LocardError, OperationType};

/// Enforces Role-Based Access Control (RBAC) policies on backend operations.
pub struct AuthorizationEngine;

impl AuthorizationEngine {
    /// Enforces that the authenticated user possesses the Administrator role.
    pub fn require_admin(role: UserRole) -> Result<(), LocardError> {
        if role != UserRole::Administrator {
            return Err(LocardError::SecurityViolation(
                "Access denied: Administrator privileges required.".to_string(),
            ));
        }
        Ok(())
    }

    /// Returns the complete list of permissions granted to a given role.
    pub fn get_permissions_for_role(role: UserRole) -> Vec<Permission> {
        match role {
            UserRole::Administrator => vec![
                // Full Case Management
                Permission::CaseCreate,
                Permission::CaseView,
                Permission::CaseModify,
                Permission::CaseClose,
                Permission::CaseEvidenceAdd,
                Permission::CaseCustodyRecord,
                Permission::CaseReportGenerate,
                // Full Acquisition
                Permission::AcquisitionViewSources,
                Permission::AcquisitionStart,
                Permission::AcquisitionCancel,
                Permission::AcquisitionViewArtifacts,
                // Full Recovery
                Permission::RecoveryStart,
                Permission::RecoveryViewResults,
                Permission::RecoveryExportFiles,
                Permission::RecoveryGenerateReport,
                // Erasure Management
                Permission::ErasureViewCapabilities,
                Permission::ErasurePlan,
                Permission::ErasureExecute,
                Permission::ErasureGenerateReport,
                // Audit & Chain Verification
                Permission::AuditView,
                Permission::AuditVerify,
                // User & System Administration
                Permission::UserCreate,
                Permission::UserList,
                Permission::UserView,
                Permission::UserUpdate,
                Permission::UserDisable,
                Permission::UserChangeRole,
                Permission::UserUnlock,
            ],
            UserRole::Investigator => vec![
                // Full Case Management
                Permission::CaseCreate,
                Permission::CaseView,
                Permission::CaseModify,
                Permission::CaseClose,
                Permission::CaseEvidenceAdd,
                Permission::CaseCustodyRecord,
                Permission::CaseReportGenerate,
                // Full Acquisition
                Permission::AcquisitionViewSources,
                Permission::AcquisitionStart,
                Permission::AcquisitionCancel,
                Permission::AcquisitionViewArtifacts,
                // Full Recovery
                Permission::RecoveryStart,
                Permission::RecoveryViewResults,
                Permission::RecoveryExportFiles,
                Permission::RecoveryGenerateReport,
                // Erasure Planning & Read-Only Inspection
                Permission::ErasureViewCapabilities,
                Permission::ErasurePlan,
                // Audit Inspection & Verification
                Permission::AuditView,
                Permission::AuditVerify,
                // Self View
                Permission::UserView,
            ],
            UserRole::Operator => vec![
                // Operational Case Actions
                Permission::CaseView,
                Permission::CaseEvidenceAdd,
                Permission::CaseCustodyRecord,
                // Full Acquisition
                Permission::AcquisitionViewSources,
                Permission::AcquisitionStart,
                Permission::AcquisitionCancel,
                Permission::AcquisitionViewArtifacts,
                // Full Recovery
                Permission::RecoveryStart,
                Permission::RecoveryViewResults,
                Permission::RecoveryExportFiles,
                Permission::RecoveryGenerateReport,
                // Erasure Execution (Subject to Step 10 Two-Stage Confirmation)
                Permission::ErasureViewCapabilities,
                Permission::ErasurePlan,
                Permission::ErasureExecute,
                Permission::ErasureGenerateReport,
                // Audit Inspection
                Permission::AuditView,
                // Self View
                Permission::UserView,
            ],
            UserRole::Viewer => vec![
                // Strictly Read-Only Inspection
                Permission::CaseView,
                Permission::AcquisitionViewSources,
                Permission::AcquisitionViewArtifacts,
                Permission::RecoveryViewResults,
                Permission::ErasureViewCapabilities,
                Permission::AuditView,
                Permission::UserView,
            ],
        }
    }

    /// Evaluates whether a role possesses a specific permission.
    pub fn has_permission(role: UserRole, permission: Permission) -> bool {
        Self::get_permissions_for_role(role).contains(&permission)
    }

    /// Asserts that a role possesses a specific permission, failing closed with `SecurityViolation`.
    pub fn check_permission(role: UserRole, permission: Permission) -> Result<(), LocardError> {
        if !Self::has_permission(role, permission) {
            return Err(LocardError::SecurityViolation(format!(
                "Access denied: Role '{}' does not possess permission '{:?}'.",
                role, permission
            )));
        }
        Ok(())
    }

    /// Evaluates whether a role is authorized to initiate/request an operation category.
    /// Backward-compatible helper for legacy operation flows.
    pub fn can_request_operation(
        role: UserRole,
        op_type: OperationType,
    ) -> Result<(), LocardError> {
        if op_type == OperationType::Unknown {
            return Err(LocardError::SecurityViolation(
                "Cannot request unknown operation type.".to_string(),
            ));
        }

        if op_type.is_destructive() {
            match role {
                UserRole::Viewer | UserRole::Investigator => {
                    return Err(LocardError::SecurityViolation(format!(
                        "Access denied: Role '{}' cannot request destructive operations.",
                        role
                    )));
                }
                UserRole::Operator | UserRole::Administrator => {
                    // Allowed to prepare/request confirmation, but final execution will be gated
                    // by safety validation, target revalidation, and explicit confirmation.
                    Ok(())
                }
            }
        } else {
            // Read-only operations (IntegrityHash, IntegrityVerify, Recovery)
            Ok(())
        }
    }

    /// Evaluates whether a role can perform read-only operations.
    pub fn can_perform_read_only(
        _role: UserRole,
        op_type: OperationType,
    ) -> Result<(), LocardError> {
        if op_type.is_destructive() {
            return Err(LocardError::SecurityViolation(
                "Operation is destructive and cannot be executed as read-only.".to_string(),
            ));
        }
        Ok(())
    }

    /// Evaluates whether a role may authorize destructive execution.
    /// NOTE: Even with role authorization, hard safety blocks (system/boot devices)
    /// and disabled executor status take absolute precedence.
    pub fn can_authorize_destructive_execution(
        role: UserRole,
        op_type: OperationType,
    ) -> Result<(), LocardError> {
        if !op_type.is_destructive() {
            return Ok(());
        }

        match role {
            UserRole::Viewer | UserRole::Investigator => {
                Err(LocardError::SecurityViolation(format!(
                    "Access denied: Role '{}' cannot authorize destructive operations.",
                    role
                )))
            }
            UserRole::Operator | UserRole::Administrator => {
                // Roles may authorize subject to explicit confirmation and safety policy
                Ok(())
            }
        }
    }
}
