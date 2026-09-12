use locardx_audit::AuditService;
use locardx_auth::AuthService;
use locardx_common::{AppConfig, LocardError};
use locardx_database::Database;
use locardx_device_manager::{DeviceManagerService, MockDeviceProvider};
use locardx_operation_manager::OperationManager;
use locardx_verification::{IntegrityService, SanitizationService};
use std::sync::Arc;
use tracing::info;

/// Core application state holding initialized shared services.
/// Safe, non-destructive startup path.
pub struct AppState {
    pub config: AppConfig,
    pub db: Arc<Database>,
    pub audit: Arc<AuditService>,
    pub auth: Arc<AuthService>,
    pub device_manager: Arc<DeviceManagerService>,
    pub integrity: Arc<IntegrityService>,
    pub sanitization: Arc<SanitizationService>,
    pub operation_manager: Arc<OperationManager>,
    pub safety: Arc<locardx_security::SafetyEngine>,
    pub file_eraser: Arc<locardx_file_eraser::FileEraserService>,
    pub drive_eraser: Arc<locardx_drive_eraser::DriveEraserService>,
    pub mock_device_registry: Arc<locardx_drive_eraser::MockDeviceRegistry>,
    pub real_hardware_provider: Arc<dyn locardx_drive_eraser::DriveHardwareProvider>,
    pub reporting: Arc<locardx_reporting::ReportingService>,
}

impl AppState {
    /// Initializes application core services.
    /// Non-destructive: opens SQLite DB (in-memory or at configured path) and prepares audit, auth, and device services.
    pub fn init() -> Result<Self, LocardError> {
        let config = AppConfig::from_env();
        locardx_common::init_logging(&config.log_level);

        info!(
            env = %config.env,
            log_level = %config.log_level,
            "Initializing LocardX Application Core"
        );

        let db = Arc::new(Database::open(&config.db_path)?);
        let audit = Arc::new(AuditService::new(Arc::clone(&db)));
        let auth = Arc::new(AuthService::new(Arc::clone(&db), Arc::clone(&audit)));
        let integrity = Arc::new(IntegrityService::new(Arc::clone(&db), Arc::clone(&audit)));
        let device_manager = if config.env == "test" {
            info!("Initializing test mock device manager");
            Arc::new(DeviceManagerService::new(Arc::new(
                MockDeviceProvider::new_default(),
            )))
        } else {
            info!("Initializing platform storage device manager");
            Arc::new(DeviceManagerService::new_system())
        };

        let safety = Arc::new(locardx_security::SafetyEngine::new(
            Arc::clone(&db),
            Arc::clone(&audit),
            Arc::clone(&auth),
            Arc::clone(&device_manager),
        ));

        let sanitization = Arc::new(SanitizationService::new(
            Arc::clone(&db),
            Arc::clone(&audit),
            Arc::clone(&device_manager),
            Some(Arc::clone(&safety)),
        ));

        let file_eraser = Arc::new(locardx_file_eraser::FileEraserService::new(
            Arc::clone(&db),
            Arc::clone(&audit),
            Arc::clone(&safety),
        ));

        let mock_device_registry =
            Arc::new(locardx_drive_eraser::MockDeviceRegistry::new_standard_test_set());
        let real_hardware_provider: Arc<dyn locardx_drive_eraser::DriveHardwareProvider> =
            Arc::new(locardx_drive_eraser::RealDriveHardwareProvider::new());
        let drive_eraser = Arc::new(
            locardx_drive_eraser::DriveEraserService::new(
                Arc::clone(&db),
                Arc::clone(&audit),
                Arc::clone(&safety),
                Arc::clone(&mock_device_registry)
                    as Arc<dyn locardx_device_manager::DeviceDiscoveryProvider>,
            )
            .with_hardware_provider(Arc::clone(&real_hardware_provider)),
        );

        let operation_manager = Arc::new(
            OperationManager::new(Arc::clone(&db), Arc::clone(&audit), Arc::clone(&integrity))
                .with_safety(Arc::clone(&safety))
                .with_sanitization(Arc::clone(&sanitization))
                .with_file_eraser(Arc::clone(&file_eraser))
                .with_drive_eraser(Arc::clone(&drive_eraser)),
        );

        let reporting = Arc::new(locardx_reporting::ReportingService::new(
            Arc::clone(&db),
            Arc::clone(&audit),
        ));

        // Automatic crash recovery: fail closed on any operations interrupted by prior termination
        let rec_ops = operation_manager
            .recover_interrupted_operations()
            .unwrap_or(0);
        let rec_drives = drive_eraser.recover_interrupted_erasures().unwrap_or(0);
        if rec_ops > 0 || rec_drives > 0 {
            info!(
                recovered_operations = rec_ops,
                recovered_drive_erasures = rec_drives,
                "Recovered interrupted operations from abnormal termination"
            );
        }

        audit.log_event(
            "SYSTEM_STARTUP",
            "Application core initialized successfully",
        )?;

        Ok(Self {
            config,
            db,
            audit,
            auth,
            device_manager,
            integrity,
            sanitization,
            operation_manager,
            safety,
            file_eraser,
            drive_eraser,
            mock_device_registry,
            real_hardware_provider,
            reporting,
        })
    }
}
