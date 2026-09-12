use crate::models::UserRole;
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

    /// Evaluates whether a role is authorized to initiate/request an operation category.
    ///
    /// Role Policy:
    /// - Viewer: Read-only inspection and integrity verification only. Cannot request destructive operations.
    /// - Operator: Read-only operations, may prepare/request destructive operations (subject to safety interlocks).
    /// - Investigator: Forensic and investigative operations, destructive operations only where explicitly permitted by policy.
    /// - Administrator: System administration, destructive operations only where policy explicitly allows.
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
                UserRole::Viewer => {
                    return Err(LocardError::SecurityViolation(
                        "Access denied: Role 'Viewer' is strictly read-only and cannot request destructive operations.".to_string(),
                    ));
                }
                UserRole::Operator | UserRole::Investigator | UserRole::Administrator => {
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
            UserRole::Viewer => Err(LocardError::SecurityViolation(
                "Access denied: Role 'Viewer' cannot authorize destructive operations.".to_string(),
            )),
            UserRole::Operator | UserRole::Investigator | UserRole::Administrator => {
                // Roles may authorize subject to explicit confirmation and safety policy
                Ok(())
            }
        }
    }
}
