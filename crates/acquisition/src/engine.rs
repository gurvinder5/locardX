use crate::hasher::StreamingSha256Hasher;
use crate::models::{AcquisitionFailureReason, AcquisitionProgress, ArtifactCleanupStatus};
use crate::platform::ReadOnlyDeviceStream;
use crate::writer::{staging_path_for, ArtifactCleanupResult, RawImageWriter};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{error, info, warn};

/// Output of a successfully executed acquisition stream pass.
#[derive(Debug, Clone)]
pub struct AcquisitionEngineOutput {
    pub image_path: PathBuf,
    pub image_size_bytes: u64,
    pub image_sha256: String,
    pub bytes_acquired: u64,
    pub elapsed_seconds: f64,
}

/// Failed engine execution with a filesystem-verified cleanup outcome.
#[derive(Debug, Clone)]
pub struct AcquisitionEngineError {
    pub reason: AcquisitionFailureReason,
    pub cleanup_status: ArtifactCleanupStatus,
    pub cleanup_error: Option<String>,
    pub leftover_path: Option<PathBuf>,
}

impl AcquisitionEngineError {
    fn from_reason_before_writer(reason: AcquisitionFailureReason) -> Self {
        Self {
            reason,
            cleanup_status: ArtifactCleanupStatus::CleanupNotRequired,
            cleanup_error: None,
            leftover_path: None,
        }
    }

    fn from_cleanup(reason: AcquisitionFailureReason, cleanup: ArtifactCleanupResult) -> Self {
        let leftover_path = if cleanup.is_confirmed() {
            None
        } else {
            Some(cleanup.path.clone())
        };
        Self {
            reason,
            cleanup_status: cleanup.status,
            cleanup_error: cleanup.error,
            leftover_path,
        }
    }
}

impl std::fmt::Display for AcquisitionEngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

/// Optional I/O fault injection used by automated tests. Production callers use `Default`.
#[derive(Debug, Clone, Default)]
pub struct AcquisitionEngineOptions {
    pub write_fail_after: Option<u64>,
    pub force_integrity_mismatch: bool,
}

/// The core read-only streaming acquisition engine.
pub struct AcquisitionEngine;

impl AcquisitionEngine {
    /// Executes a bitstream raw/dd acquisition from `stream` to `destination_path`.
    /// Simultaneously computes the streaming SHA-256 hash and emits progress telemetry.
    pub fn execute(
        operation_id: &str,
        stream: Box<dyn ReadOnlyDeviceStream>,
        destination_path: &Path,
        chunk_size_bytes: usize,
        expected_total_bytes: u64,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(AcquisitionProgress) + Send + Sync)>,
    ) -> Result<AcquisitionEngineOutput, AcquisitionEngineError> {
        Self::execute_with_options(
            operation_id,
            stream,
            destination_path,
            chunk_size_bytes,
            expected_total_bytes,
            is_cancelled,
            on_progress,
            AcquisitionEngineOptions::default(),
        )
    }

    pub fn execute_with_options(
        operation_id: &str,
        mut stream: Box<dyn ReadOnlyDeviceStream>,
        destination_path: &Path,
        chunk_size_bytes: usize,
        expected_total_bytes: u64,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(AcquisitionProgress) + Send + Sync)>,
        options: AcquisitionEngineOptions,
    ) -> Result<AcquisitionEngineOutput, AcquisitionEngineError> {
        let chunk_size = if chunk_size_bytes == 0 {
            1024 * 1024 // 1 MiB default
        } else {
            chunk_size_bytes
        };

        info!(
            operation_id,
            chunk_size,
            expected_total_bytes,
            destination = %destination_path.display(),
            "[ACQUISITION_START] Beginning forensic acquisition stream"
        );

        let mut writer = match RawImageWriter::create_with_write_limit(
            destination_path,
            options.write_fail_after,
        ) {
            Ok(w) => {
                info!(
                    operation_id,
                    destination = %destination_path.display(),
                    staging = %w.staging_path().display(),
                    "[DESTINATION_OPEN] Destination evidence staging file opened for writing"
                );
                w
            }
            Err(e) => {
                error!(
                    operation_id,
                    error = %e,
                    "[DESTINATION_OPEN_FAILED] Failed creating destination file"
                );
                return Err(AcquisitionEngineError::from_reason_before_writer(e));
            }
        };

        let mut hasher = StreamingSha256Hasher::new();

        // Allocate 4096-byte aligned buffer to satisfy direct hardware I/O DMA requirements
        let mut raw_buffer = vec![0u8; chunk_size + 4096];
        let align_offset = {
            let addr = raw_buffer.as_ptr() as usize;
            (4096 - (addr % 4096)) % 4096
        };

        let start_time = Instant::now();
        let mut last_progress_time = Instant::now();
        let mut total_bytes_acquired: u64 = 0;
        let mut chunk_index: u64 = 0;

        loop {
            // 1. Cooperative cancellation check
            if let Some(cancelled) = is_cancelled {
                if cancelled() {
                    warn!(
                        operation_id,
                        total_bytes_acquired,
                        "[CLEANUP] Acquisition cancelled by operator. Attempting incomplete artifact removal."
                    );
                    let cleanup = writer.abort_and_cleanup();
                    log_cleanup(operation_id, &cleanup);
                    return Err(AcquisitionEngineError::from_cleanup(
                        AcquisitionFailureReason::Cancelled,
                        cleanup,
                    ));
                }
            }

            let current_offset = total_bytes_acquired;
            let aligned_slice = &mut raw_buffer[align_offset..align_offset + chunk_size];

            // 2. Sequential read from source physical stream
            let read_bytes = match stream.read_chunk(aligned_slice) {
                Ok(n) => n,
                Err(e) => {
                    error!(
                        operation_id,
                        offset = current_offset,
                        error = %e,
                        "[READ_ERROR] Read failure from physical source stream"
                    );
                    let cleanup = writer.abort_and_cleanup();
                    log_cleanup(operation_id, &cleanup);
                    return Err(AcquisitionEngineError::from_cleanup(e, cleanup));
                }
            };

            // 3. Handle EOF
            if read_bytes == 0 {
                // If expected size was non-zero and we read less than expected, treat as short read
                if expected_total_bytes > 0 && total_bytes_acquired < expected_total_bytes {
                    error!(
                        operation_id,
                        expected = expected_total_bytes,
                        actual = total_bytes_acquired,
                        "[READ_ERROR] Unexpected EOF: Short read detected"
                    );
                    let cleanup = writer.abort_and_cleanup();
                    log_cleanup(operation_id, &cleanup);
                    return Err(AcquisitionEngineError::from_cleanup(
                        AcquisitionFailureReason::ShortRead {
                            expected: expected_total_bytes,
                            actual: total_bytes_acquired,
                        },
                        cleanup,
                    ));
                }
                break;
            }

            let chunk = &aligned_slice[..read_bytes];

            // 4. Update streaming SHA-256 with exact bytes read
            hasher.update(chunk);

            // 5. Write chunk to destination image
            if let Err(e) = writer.write_chunk(chunk) {
                error!(
                    operation_id,
                    offset = current_offset,
                    bytes = read_bytes,
                    error = %e,
                    "[IMAGE_WRITE_FAILED] Write failure to target evidence file"
                );
                let cleanup = writer.abort_and_cleanup();
                log_cleanup(operation_id, &cleanup);
                return Err(AcquisitionEngineError::from_cleanup(e, cleanup));
            }

            total_bytes_acquired += read_bytes as u64;
            chunk_index += 1;

            if chunk_index <= 3 || chunk_index % 100 == 0 {
                info!(
                    operation_id,
                    chunk_index,
                    offset = current_offset,
                    bytes_read = read_bytes,
                    total_bytes_acquired,
                    "[READ_COMPLETE] Successfully acquired and hashed chunk"
                );
            }

            // 6. Telemetry: byte-accurate progress. Never report 100% before flush + integrity + finalize.
            let now = Instant::now();
            if chunk_index == 1
                || now.duration_since(last_progress_time).as_millis() >= 100
                || (expected_total_bytes > 0 && total_bytes_acquired >= expected_total_bytes)
            {
                last_progress_time = now;
                emit_progress(
                    operation_id,
                    total_bytes_acquired,
                    expected_total_bytes,
                    start_time,
                    false,
                    on_progress,
                );
            }
        }

        // 7. Flush and sync image writer (closes the staging file handle)
        let final_size = match writer.flush_and_sync() {
            Ok(s) => s,
            Err(e) => {
                error!(operation_id, error = %e, "[CLEANUP] Flush failure, aborting");
                let cleanup = writer.abort_and_cleanup();
                log_cleanup(operation_id, &cleanup);
                return Err(AcquisitionEngineError::from_cleanup(e, cleanup));
            }
        };

        // 8. Finalize streaming SHA-256
        let (image_sha256, hash_bytes) = hasher.finalize();
        let elapsed_seconds = start_time.elapsed().as_secs_f64();

        let compared_hash_bytes = if options.force_integrity_mismatch {
            hash_bytes.saturating_add(1)
        } else {
            hash_bytes
        };

        if final_size != compared_hash_bytes {
            let err = AcquisitionFailureReason::WriteError(format!(
                "Integrity mismatch: written bytes {} != hashed bytes {}",
                final_size, hash_bytes
            ));
            error!(operation_id, error = %err, "[CLEANUP] Integrity verification mismatch");
            let cleanup = writer.abort_and_cleanup();
            log_cleanup(operation_id, &cleanup);
            return Err(AcquisitionEngineError::from_cleanup(err, cleanup));
        }

        // 9. Promote staging artifact to the final image name only after verification.
        let (promoted_size, promoted_path) = match writer.finalize_to_destination() {
            Ok(v) => v,
            Err(e) => {
                error!(operation_id, error = %e, "[FINALIZE_FAILED] Staging image could not be promoted");
                let leftover = staging_path_for(destination_path);
                let leftover_exists = leftover.exists();
                return Err(AcquisitionEngineError {
                    reason: e,
                    cleanup_status: if leftover_exists {
                        ArtifactCleanupStatus::CleanupFailed
                    } else {
                        ArtifactCleanupStatus::CleanupNotRequired
                    },
                    cleanup_error: Some(format!(
                        "Verified staging artifact could not be promoted to '{}'. Leftover: {}",
                        destination_path.display(),
                        leftover.display()
                    )),
                    leftover_path: leftover_exists.then_some(leftover),
                });
            }
        };

        info!(
            operation_id,
            final_size = promoted_size,
            sha256 = %image_sha256,
            elapsed_seconds,
            "[ACQUISITION_COMPLETE] Bitstream acquisition successfully completed and verified"
        );

        emit_progress(
            operation_id,
            total_bytes_acquired,
            total_bytes_acquired,
            start_time,
            true,
            on_progress,
        );

        Ok(AcquisitionEngineOutput {
            image_path: promoted_path,
            image_size_bytes: promoted_size,
            image_sha256,
            bytes_acquired: total_bytes_acquired,
            elapsed_seconds,
        })
    }
}

fn log_cleanup(operation_id: &str, cleanup: &ArtifactCleanupResult) {
    match cleanup.status {
        ArtifactCleanupStatus::CleanupConfirmed => {
            info!(
                operation_id,
                path = %cleanup.path.display(),
                "[CLEANUP] CLEANUP_CONFIRMED: partial artifact is absent from the filesystem"
            );
        }
        ArtifactCleanupStatus::CleanupFailed => {
            error!(
                operation_id,
                path = %cleanup.path.display(),
                error = cleanup.error.as_deref().unwrap_or("unknown"),
                "[CLEANUP] CLEANUP_FAILED: partial artifact still exists"
            );
        }
        ArtifactCleanupStatus::CleanupNotRequired => {}
    }
}

fn emit_progress(
    operation_id: &str,
    bytes_acquired: u64,
    total_bytes: u64,
    start_time: Instant,
    completed: bool,
    on_progress: Option<&(dyn Fn(AcquisitionProgress) + Send + Sync)>,
) {
    let Some(cb) = on_progress else {
        return;
    };

    let elapsed_secs = start_time.elapsed().as_secs_f64();
    let throughput_mbps = if elapsed_secs > 0.0 {
        (bytes_acquired as f64 / (1024.0 * 1024.0)) / elapsed_secs
    } else {
        0.0
    };

    let percentage = if completed {
        100.0
    } else if total_bytes > 0 {
        let raw = (bytes_acquired as f64 / total_bytes as f64) * 100.0;
        raw.min(99.9)
    } else {
        0.0
    };

    let eta_seconds = if completed {
        Some(0.0)
    } else if throughput_mbps > 0.0 && total_bytes > bytes_acquired {
        let remaining_mb = (total_bytes - bytes_acquired) as f64 / (1024.0 * 1024.0);
        Some(remaining_mb / throughput_mbps)
    } else {
        None
    };

    let stage = if completed {
        "Acquisition & hashing complete".to_string()
    } else {
        "Acquiring raw physical sectors & calculating SHA-256".to_string()
    };

    cb(AcquisitionProgress {
        operation_id: operation_id.to_string(),
        bytes_acquired,
        total_bytes,
        percentage: (percentage * 10.0).round() / 10.0,
        throughput_mbps: (throughput_mbps * 10.0).round() / 10.0,
        elapsed_seconds: (elapsed_secs * 10.0).round() / 10.0,
        eta_seconds: eta_seconds.map(|e| (e * 10.0).round() / 10.0),
        stage,
    });
}
