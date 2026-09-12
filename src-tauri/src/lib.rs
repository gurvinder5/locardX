pub mod commands;
pub mod state;

use locardx_common::LocardError;
use state::AppState;

/// Initializes the application and shared services in a safe, non-destructive mode.
pub fn init_application() -> Result<AppState, LocardError> {
    AppState::init()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_initialization() {
        // Safe in-memory test initialization
        std::env::set_var("LOCARDX_ENV", "test");
        std::env::set_var("LOCARDX_DB_PATH", ":memory:");

        let app_state = init_application();
        assert!(app_state.is_ok(), "Application failed to initialize");

        let state = app_state.unwrap();
        assert_eq!(state.config.env, "test");
        assert!(state.db.is_healthy(), "Database was not healthy");
        assert!(
            state.auth.is_first_run().unwrap(),
            "Fresh install should report is_first_run true"
        );
    }

    #[test]
    fn test_get_app_info() {
        std::env::set_var("LOCARDX_ENV", "test");
        std::env::set_var("LOCARDX_DB_PATH", ":memory:");

        let state = init_application().unwrap();
        let info = commands::get_app_info_handler(&state).unwrap();

        assert_eq!(info.name, "LocardX");
        assert_eq!(info.build_status, "Foundation Verified");
        assert_eq!(info.environment, "test");
    }

    #[test]
    fn test_list_storage_devices() {
        std::env::set_var("LOCARDX_ENV", "test");
        std::env::set_var("LOCARDX_DB_PATH", ":memory:");

        let state = init_application().unwrap();
        let devices = commands::list_storage_devices_handler(&state).unwrap();

        assert!(
            !devices.is_empty(),
            "Should discover mock devices in test environment"
        );
        assert_eq!(devices[0].device_id, "\\\\.\\PhysicalDrive0");
        assert_eq!(devices[0].volumes.len(), 2);
    }

    #[test]
    fn test_security_interlock() {
        use locardx_security::SecurityEngine;

        // Verify protected drive paths return error
        let result = SecurityEngine::validate_target_safety("C:\\");
        assert!(result.is_err(), "Security interlock should block C:\\");

        let result_root = SecurityEngine::validate_target_safety("/");
        assert!(result_root.is_err(), "Security interlock should block /");

        // Verify valid non-system target passes
        let result_valid = SecurityEngine::validate_target_safety("E:\\test_image.raw");
        assert!(result_valid.is_ok(), "Valid test image target should pass");
    }

    #[test]
    fn test_integrity_commands() {
        use std::io::Write;
        std::env::set_var("LOCARDX_ENV", "test");
        std::env::set_var("LOCARDX_DB_PATH", ":memory:");

        let state = init_application().unwrap();

        // Create temporary test file
        let temp_path =
            std::env::temp_dir().join(format!("tauri_integrity_test_{}.tmp", uuid::Uuid::new_v4()));
        let mut temp_file = std::fs::File::create(&temp_path).unwrap();
        temp_file.write_all(b"forensic evidence data").unwrap();
        temp_file.flush().unwrap();

        let path_str = temp_path.to_string_lossy().to_string();

        // 1. Calculate file hash
        let hash_res = commands::calculate_file_hash_handler(&state, &path_str, None).unwrap();
        assert!(!hash_res.digest.is_empty());
        assert_eq!(hash_res.bytes_processed, 22);

        // 2. Verify file hash (Match)
        let verify_res =
            commands::verify_file_hash_handler(&state, &path_str, &hash_res.digest, None).unwrap();
        assert_eq!(
            verify_res.status,
            locardx_verification::VerificationStatus::Verified
        );

        // 3. Verify file hash (Mismatch)
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let mismatch_res =
            commands::verify_file_hash_handler(&state, &path_str, wrong_hash, None).unwrap();
        assert!(matches!(
            mismatch_res.status,
            locardx_verification::VerificationStatus::Mismatch { .. }
        ));

        // 4. List integrity records
        let records = commands::list_integrity_records_handler(&state, 10).unwrap();
        assert_eq!(records.len(), 3);

        let _ = std::fs::remove_file(&temp_path);
    }

    #[tokio::test]
    async fn test_operation_commands() {
        use std::io::Write;
        std::env::set_var("LOCARDX_ENV", "test");
        std::env::set_var("LOCARDX_DB_PATH", ":memory:");

        let state = init_application().unwrap();

        let temp_path =
            std::env::temp_dir().join(format!("tauri_op_test_{}.tmp", uuid::Uuid::new_v4()));
        let mut temp_file = std::fs::File::create(&temp_path).unwrap();
        temp_file.write_all(b"evidence block data").unwrap();
        temp_file.flush().unwrap();

        let path_str = temp_path.to_string_lossy().to_string();

        // 1. Submit integrity hash operation
        let op_dto = commands::submit_integrity_hash_operation_handler(&state, &path_str, None)
            .await
            .unwrap();
        assert_eq!(
            op_dto.operation_type,
            locardx_operation_manager::OperationType::IntegrityHash
        );

        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        // 2. Query operation by ID
        let fetched = commands::get_operation_handler(&state, &op_dto.operation_id).unwrap();
        assert_eq!(
            fetched.current_state,
            locardx_operation_manager::OperationState::Completed
        );
        assert!(fetched
            .result_summary
            .unwrap()
            .contains("Calculated SHA-256"));

        // 3. List operations
        let list = commands::list_operations_handler(&state, 10, None, None).unwrap();
        assert!(!list.is_empty());

        let _ = std::fs::remove_file(&temp_path);
    }

    #[tokio::test]
    async fn test_safety_commands() {
        std::env::set_var("LOCARDX_ENV", "test");
        std::env::set_var("LOCARDX_DB_PATH", ":memory:");

        let state = init_application().unwrap();

        // 1. Initialize admin
        let admin = commands::initialize_admin_handler(
            &state,
            locardx_auth::models::InitAdminRequest {
                username: "safety_admin".to_string(),
                password: "AdminPassword123!".to_string(),
                confirm_password: "AdminPassword123!".to_string(),
            },
        )
        .unwrap();

        let session = commands::login_handler(
            &state,
            locardx_auth::models::LoginRequest {
                username: admin.username,
                password: "AdminPassword123!".to_string(),
            },
        )
        .unwrap();

        // 2. Evaluate safety on Windows system root
        let sys_eval = commands::evaluate_operation_safety_handler(
            &state,
            commands::EvaluateSafetyRequest {
                target_type: "LogicalVolume".to_string(),
                target_identifier: "C:\\".to_string(),
                target_display_name: Some("System Drive".to_string()),
                target_size_bytes: None,
                operation_type: "DriveErasure".to_string(),
                session_token: Some(session.token.clone()),
            },
        )
        .unwrap();

        assert_eq!(
            sys_eval.decision,
            locardx_security::SafetyDecisionOutcome::Blocked
        );
        assert_eq!(
            sys_eval.reason_code,
            locardx_security::ReasonCode::SystemDevice
        );

        // 3. Evaluate safety on external/removable mock drive
        let usb_eval = commands::evaluate_operation_safety_handler(
            &state,
            commands::EvaluateSafetyRequest {
                target_type: "PhysicalDevice".to_string(),
                target_identifier: "\\\\.\\PhysicalDrive1".to_string(),
                target_display_name: Some("External USB".to_string()),
                target_size_bytes: Some(32_000_000_000),
                operation_type: "DriveErasure".to_string(),
                session_token: Some(session.token.clone()),
            },
        )
        .unwrap();

        assert_eq!(
            usb_eval.decision,
            locardx_security::SafetyDecisionOutcome::RequiresConfirmation
        );

        // 4. Request destructive confirmation
        let challenge = commands::request_destructive_confirmation_handler(
            &state,
            commands::RequestConfirmationRequest {
                operation_id: "op-tauri-safe-1".to_string(),
                target_type: "PhysicalDevice".to_string(),
                target_identifier: "\\\\.\\PhysicalDrive1".to_string(),
                target_display_name: Some("External USB".to_string()),
                target_size_bytes: Some(32_000_000_000),
                operation_type: "DriveErasure".to_string(),
                session_token: session.token.clone(),
            },
        )
        .unwrap();

        // 5. Confirm destructive operation (re-evaluated and safely blocked by disabled executor)
        let confirm_decision = commands::confirm_destructive_operation_handler(
            &state,
            commands::ConfirmOperationRequest {
                confirmation_id: challenge.confirmation_id,
                operation_id: "op-tauri-safe-1".to_string(),
                warning_acknowledged: true,
                typed_target_confirmation: "\\\\.\\PhysicalDrive1".to_string(),
                session_token: session.token.clone(),
            },
        )
        .unwrap();

        assert_eq!(
            confirm_decision.decision,
            locardx_security::SafetyDecisionOutcome::Blocked
        );
        assert_eq!(
            confirm_decision.reason_code,
            locardx_security::ReasonCode::OperationDisabled
        );

        // 6. List safety evaluations
        let evals = commands::list_safety_evaluations_handler(&state, Some(10)).unwrap();
        assert!(!evals.is_empty());
    }

    #[test]
    fn test_sanitization_commands() {
        std::env::set_var("LOCARDX_ENV", "test");
        std::env::set_var("LOCARDX_DB_PATH", ":memory:");

        let state = init_application().unwrap();

        // 1. Get supported standards
        let methods = commands::get_sanitization_methods_handler().unwrap();
        assert!(!methods.is_empty(), "Methods list should not be empty");

        // 2. Evaluate plan on removable external USB drive
        let plan = commands::evaluate_sanitization_plan_handler(
            &state,
            commands::EvaluatePlanRequest {
                target_identifier: "\\\\.\\PhysicalDrive1".to_string(),
                target_type: "PhysicalDevice".to_string(),
                scope: "PhysicalDevice".to_string(),
                requested_method: None,
                requested_strategy: None,
                session_token: None,
            },
        )
        .unwrap();

        assert!(
            plan.is_applicable,
            "External USB should be applicable for sanitization plan"
        );
        assert!(plan.is_dry_run, "Plan must be marked dry-run");
        assert!(
            !plan.limitations.is_empty(),
            "Limitations should be documented"
        );

        // 3. Check snapshot comparison
        let cmp = commands::compare_target_snapshot_handler(&state, &plan.plan_id).unwrap();
        assert!(cmp.matches, "Live snapshot should match plan snapshot");
        assert!(cmp.differences.is_empty());

        // 4. Check verification plan retrieval
        let v_plan =
            commands::get_sanitization_verification_plan_handler(&state, &plan.plan_id).unwrap();
        assert_eq!(v_plan.plan_id, plan.plan_id);
        assert!(!v_plan.verification_strategy.is_empty());

        // 5. Verify system disk C:\ is hard-blocked from sanitization plan
        let sys_plan = commands::evaluate_sanitization_plan_handler(
            &state,
            commands::EvaluatePlanRequest {
                target_identifier: "C:\\".to_string(),
                target_type: "LogicalVolume".to_string(),
                scope: "LogicalVolume".to_string(),
                requested_method: None,
                requested_strategy: None,
                session_token: None,
            },
        )
        .unwrap();

        assert!(
            !sys_plan.is_applicable,
            "System drive C:\\ must be hard-blocked"
        );
        assert!(sys_plan
            .reason_codes
            .iter()
            .any(|r| r.contains("SYSTEM_DEVICE") || r.contains("BOOT_DEVICE")));
    }

    #[tokio::test]
    async fn test_file_eraser_commands() {
        std::env::set_var("LOCARDX_ENV", "test");
        std::env::set_var("LOCARDX_DB_PATH", ":memory:");

        let state = init_application().unwrap();

        // Initialize admin and login
        let admin = commands::initialize_admin_handler(
            &state,
            locardx_auth::models::InitAdminRequest {
                username: "testadmin_fe".to_string(),
                password: "SecretPass123!".to_string(),
                confirm_password: "SecretPass123!".to_string(),
            },
        )
        .unwrap();

        let session = commands::login_handler(
            &state,
            locardx_auth::models::LoginRequest {
                username: admin.username,
                password: "SecretPass123!".to_string(),
            },
        )
        .unwrap();

        // 1. Create a temp test file
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!(
            "tauri_file_erase_test_{}.txt",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&file_path, b"Sensitive test forensic data").unwrap();

        let file_path_str = file_path.to_string_lossy().to_string();

        // 2. Plan file erasure
        let plan = commands::plan_file_erasure_handler(
            &state,
            commands::PlanFileEraseRequest {
                target_path: file_path_str.clone(),
                method: Some("LogicalFileShred".to_string()),
                session_token: Some(session.token.clone()),
            },
        )
        .await
        .unwrap();

        assert_eq!(plan.target_path, file_path_str);
        assert_eq!(plan.scope, "File");
        assert_eq!(plan.passes, 3);

        // 3. System path protection check
        let sys_path_attempt = commands::plan_file_erasure_handler(
            &state,
            commands::PlanFileEraseRequest {
                target_path: "C:\\Windows\\notepad.exe".to_string(),
                method: None,
                session_token: Some(session.token.clone()),
            },
        )
        .await;

        assert!(sys_path_attempt.is_err(), "System path must be rejected");

        // 4. Request confirmation challenge for the file
        let op_target = locardx_common::TargetIdentity {
            target_type: locardx_common::TargetType::File,
            identifier: plan.canonical_path.clone(),
            display_name: file_path_str.clone(),
            size_bytes: Some(plan.pre_metadata.size_bytes),
        };

        let challenge = state
            .safety
            .request_destructive_confirmation(
                "op-tauri-file-1",
                &op_target,
                locardx_common::OperationType::FileErasure,
                &session.token,
            )
            .unwrap();

        // 5. Execute file erasure
        let result = commands::execute_file_erasure_handler(
            &state,
            commands::ExecuteFileEraseRequest {
                plan_id: plan.plan_id.clone(),
                confirmation_id: challenge.confirmation_id,
                operation_id: "op-tauri-file-1".to_string(),
                typed_confirmation: plan.canonical_path.clone(),
                warning_acknowledged: true,
                session_token: session.token.clone(),
            },
        )
        .await
        .unwrap();

        assert_eq!(result.status, "Completed");
        assert_eq!(result.verification.outcome, "Verified");
        assert!(!file_path.exists(), "File must no longer exist on disk");

        // 6. Query result from DB
        let fetched = commands::get_file_erasure_result_handler(&state, "op-tauri-file-1").unwrap();
        assert!(fetched.is_some());
        let res_dto = fetched.unwrap();
        assert_eq!(res_dto.status, "Completed");
    }

    #[tokio::test]
    async fn test_drive_erasure_handlers() {
        std::env::set_var("LOCARDX_ENV", "test");
        std::env::set_var("LOCARDX_DB_PATH", ":memory:");

        let state = init_application().unwrap();

        // 1. Discover mock devices
        let devices = commands::get_mock_test_devices_handler(&state).unwrap();
        assert!(!devices.is_empty());
        assert!(devices.iter().any(|d| d.device_id == r"\\.\PhysicalDrive1"));

        // 2. Query hardware capabilities
        let caps = commands::get_drive_capabilities_handler(&state, r"\\.\PhysicalDrive1").unwrap();
        assert!(caps.supports_overwrite);

        // 3. Plan simulated drive erasure
        let plan = commands::plan_drive_erasure_handler(
            &state,
            commands::PlanDriveEraseRequest {
                target_device_id: r"\\.\PhysicalDrive1".to_string(),
                requested_method: None,
                execution_mode: None,
                session_token: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(plan.physical_device_id, r"\\.\PhysicalDrive1");
        assert_eq!(plan.passes, 1);

        // 4. Execute simulated erasure
        let op_id = "op-tauri-drive-sim-1";
        let result = commands::execute_drive_erasure_simulation_handler(
            &state,
            commands::ExecuteDriveEraseSimulationRequest {
                plan_id: plan.plan_id.clone(),
                confirmation_id: "conf-tauri-challenge-1".to_string(),
                operation_id: op_id.to_string(),
                typed_confirmation: r"\\.\PhysicalDrive1".to_string(),
                warning_acknowledged: true,
                session_token: "test-token-session".to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(
            result.status,
            locardx_drive_eraser::DriveEraseStatus::Completed
        );
        assert_eq!(
            result.execution_mode,
            locardx_drive_eraser::ExecutionMode::Simulation
        );

        // 5. Query saved result from DB
        let fetched = commands::get_drive_erasure_result_handler(&state, op_id).unwrap();
        assert!(fetched.is_some());
        let res = fetched.unwrap();
        assert_eq!(
            res.status,
            locardx_drive_eraser::DriveEraseStatus::Completed
        );

        // 6. Capability assessment via hardware provider
        let assessment =
            commands::assess_drive_hardware_capabilities_handler(&state, r"\\.\PhysicalDrive1")
                .await
                .unwrap();
        assert_eq!(
            assessment.overall_state,
            locardx_drive_eraser::CapabilityState::Supported
        );

        // 7. Verify real device enumeration handler runs cleanly
        let real_devices = commands::get_real_storage_devices_handler(&state);
        assert!(real_devices.is_ok());

        // 8. Verify real hardware execution attempt is safely rejected at desktop IPC boundary
        let real_exec_result = commands::execute_drive_erasure_hardware_handler(
            &state,
            commands::ExecuteDriveEraseSimulationRequest {
                plan_id: plan.plan_id.clone(),
                confirmation_id: "test-conf-real".to_string(),
                operation_id: "op-real-desk-1".to_string(),
                typed_confirmation: r"\\.\PhysicalDrive1".to_string(),
                warning_acknowledged: true,
                session_token: "test-token-session".to_string(),
            },
        )
        .await;
        assert!(real_exec_result.is_err());
    }
}
