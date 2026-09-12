use locardx_common::LocardError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Thread-safe cooperative cancellation token for asynchronous operations.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    is_cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            is_cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signals cancellation request to any observing executor.
    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }

    /// Checks if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }

    /// Helper that returns an Err if cancellation was requested.
    pub fn check_cancelled(&self) -> Result<(), LocardError> {
        if self.is_cancelled() {
            Err(LocardError::Operation(
                "Operation cancelled by operator".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}
