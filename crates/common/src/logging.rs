use tracing::Level;
use tracing_subscriber::FmtSubscriber;

/// Safely initializes the structured tracing subscriber.
/// Ensures standard levels (trace, debug, info, warn, error) are filtered according to configuration.
///
/// # Security Invariant
/// Handlers and logging macros MUST NOT log:
/// - Plaintext passwords or encryption keys
/// - Evidence byte contents or file stream dumps
/// - Raw sector buffers
pub fn init_logging(log_level: &str) {
    let level = match log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .finish();

    // Use try_init to avoid panicking if already initialized in test harnesses
    let _ = tracing::subscriber::set_global_default(subscriber);
}
