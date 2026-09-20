use std::env;

/// Application runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    /// Runtime environment: development | test | production
    pub env: String,
    /// Tracing / log level filter
    pub log_level: String,
    /// Database path or ":memory:" for in-memory testing
    pub db_path: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            env: "development".to_string(),
            log_level: "info".to_string(),
            db_path: "locardx.db".to_string(),
        }
    }
}

impl AppConfig {
    /// Loads configuration from process environment variables without hardcoding secrets or paths.
    pub fn from_env() -> Self {
        let env_mode = env::var("LOCARDX_ENV").unwrap_or_else(|_| "development".to_string());
        let log_level = env::var("LOCARDX_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        let default_db = if env_mode == "test" {
            ":memory:".to_string()
        } else {
            "locardx.db".to_string()
        };
        let db_path = env::var("LOCARDX_DB_PATH").unwrap_or(default_db);

        Self {
            env: env_mode,
            log_level,
            db_path,
        }
    }
}
