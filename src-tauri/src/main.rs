// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    println!("[LocardX] Initializing application core...");

    let state = match locardx_desktop::init_application() {
        Ok(s) => s,
        Err(err) => {
            eprintln!("[LocardX] Critical initialization failure: {}", err);
            std::process::exit(1);
        }
    };

    println!("[LocardX] Core initialized. Launching Tauri desktop shell...");

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            locardx_desktop::commands::get_app_info,
            locardx_desktop::commands::is_first_run,
            locardx_desktop::commands::initialize_admin,
            locardx_desktop::commands::login,
            locardx_desktop::commands::logout,
            locardx_desktop::commands::get_current_user,
            locardx_desktop::commands::create_user,
            locardx_desktop::commands::set_user_enabled,
            locardx_desktop::commands::list_users,
            locardx_desktop::commands::change_password,
            locardx_desktop::commands::list_storage_devices,
            locardx_desktop::commands::calculate_file_hash,
            locardx_desktop::commands::verify_file_hash,
            locardx_desktop::commands::list_integrity_records,
            locardx_desktop::commands::submit_integrity_hash_operation,
            locardx_desktop::commands::submit_integrity_verify_operation,
            locardx_desktop::commands::get_operation,
            locardx_desktop::commands::list_operations,
            locardx_desktop::commands::cancel_operation,
            locardx_desktop::commands::evaluate_operation_safety,
            locardx_desktop::commands::request_destructive_confirmation,
            locardx_desktop::commands::confirm_destructive_operation,
            locardx_desktop::commands::list_safety_evaluations,
            locardx_desktop::commands::evaluate_sanitization_plan,
            locardx_desktop::commands::get_sanitization_methods,
            locardx_desktop::commands::compare_target_snapshot,
            locardx_desktop::commands::get_sanitization_verification_plan,
            locardx_desktop::commands::plan_file_erasure,
            locardx_desktop::commands::plan_folder_erasure,
            locardx_desktop::commands::execute_file_erasure,
            locardx_desktop::commands::execute_folder_erasure,
            locardx_desktop::commands::get_file_erasure_result,
            locardx_desktop::commands::plan_drive_erasure,
            locardx_desktop::commands::execute_drive_erasure_simulation,
            locardx_desktop::commands::execute_drive_erasure_hardware,
            locardx_desktop::commands::get_drive_capabilities,
            locardx_desktop::commands::assess_drive_hardware_capabilities,
            locardx_desktop::commands::get_mock_test_devices,
            locardx_desktop::commands::get_real_storage_devices,
            locardx_desktop::commands::refresh_real_devices,
            locardx_desktop::commands::get_drive_erasure_result,
            locardx_desktop::commands::check_drive_eraser_privileges,
            locardx_desktop::commands::generate_drive_erasure_report,
            locardx_desktop::commands::get_drive_erasure_report,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running LocardX Tauri desktop application");
}
