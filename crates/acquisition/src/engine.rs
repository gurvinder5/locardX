use crate::hasher::StreamingSha256Hasher;
use crate::models::{
    AcquisitionFailureReason, AcquisitionPerformanceStats, AcquisitionProgress,
    ArtifactCleanupStatus,
};
use crate::platform::ReadOnlyDeviceStream;
use crate::writer::{staging_path_for, ArtifactCleanupResult, RawImageWriter};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// Output of a successfully executed acquisition stream pass.
#[derive(Debug, Clone)]
pub struct AcquisitionEngineOutput {
    pub image_path: PathBuf,
    pub image_size_bytes: u64,
    pub image_sha256: String,
    pub bytes_acquired: u64,
    pub elapsed_seconds: f64,
    pub diagnostics: AcquisitionPerformanceStats,
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

enum PipelineCommand {
    Write { buffer: Vec<u8>, valid_len: usize },
    Abort(AcquisitionFailureReason),
    Complete,
}

/// The core read-only streaming acquisition engine.
pub struct AcquisitionEngine;

impl AcquisitionEngine {
    /// Alias for `execute` satisfying the `AcquisitionEngine::acquire` interface.
    #[allow(clippy::result_large_err)]
    pub fn acquire(
        operation_id: &str,
        stream: Box<dyn ReadOnlyDeviceStream>,
        destination_path: &Path,
        chunk_size_bytes: usize,
        expected_total_bytes: u64,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(AcquisitionProgress) + Send + Sync)>,
    ) -> Result<AcquisitionEngineOutput, AcquisitionEngineError> {
        Self::execute(
            operation_id,
            stream,
            destination_path,
            chunk_size_bytes,
            expected_total_bytes,
            is_cancelled,
            on_progress,
        )
    }

    /// Executes a bitstream raw/dd acquisition from `stream` to `destination_path`.
    /// Simultaneously computes the streaming SHA-256 hash and emits progress telemetry.
    #[allow(clippy::result_large_err)]
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

    #[allow(clippy::result_large_err, clippy::too_many_arguments)]
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

        // High-throughput Bounded Producer-Consumer Double Buffering Pipeline:
        // Pre-allocate 2 reusable buffers sized to `chunk_size` upfront.
        // Zero allocations inside the acquisition loop.
        let (free_tx, free_rx): (SyncSender<Vec<u8>>, Receiver<Vec<u8>>) = sync_channel(2);
        let (work_tx, work_rx): (SyncSender<PipelineCommand>, Receiver<PipelineCommand>) =
            sync_channel(2);
        let (writer_err_tx, writer_err_rx): (
            SyncSender<(AcquisitionFailureReason, ArtifactCleanupResult)>,
            Receiver<(AcquisitionFailureReason, ArtifactCleanupResult)>,
        ) = sync_channel(1);

        free_tx.send(vec![0u8; chunk_size]).unwrap();
        free_tx.send(vec![0u8; chunk_size]).unwrap();

        let writer_free_tx = free_tx.clone();
        let writer_handle = thread::spawn(move || {
            let mut write_duration = Duration::ZERO;
            let mut write_errors = 0u32;

            while let Ok(cmd) = work_rx.recv() {
                match cmd {
                    PipelineCommand::Write { buffer, valid_len } => {
                        let w_start = Instant::now();
                        match writer.write_chunk(&buffer[..valid_len]) {
                            Ok(()) => {
                                write_duration += w_start.elapsed();
                                let _ = writer_free_tx.send(buffer);
                            }
                            Err(e) => {
                                write_duration += w_start.elapsed();
                                write_errors += 1;
                                let cleanup = writer.abort_and_cleanup();
                                let _ = writer_err_tx.send((e.clone(), cleanup.clone()));
                                return Err((e, cleanup, write_duration, write_errors));
                            }
                        }
                    }
                    PipelineCommand::Abort(reason) => {
                        let cleanup = writer.abort_and_cleanup();
                        return Err((reason, cleanup, write_duration, write_errors));
                    }
                    PipelineCommand::Complete => break,
                }
            }

            let flush_start = Instant::now();
            let final_size = match writer.flush_and_sync() {
                Ok(s) => s,
                Err(e) => {
                    let cleanup = writer.abort_and_cleanup();
                    return Err((e, cleanup, write_duration, write_errors));
                }
            };
            write_duration += flush_start.elapsed();

            Ok((writer, final_size, write_duration, write_errors))
        });

        let start_time = Instant::now();
        let mut last_progress_time = Instant::now();
        let mut total_bytes_acquired: u64 = 0;
        let mut chunks_processed: u64 = 0;
        let mut read_duration = Duration::ZERO;
        let mut hash_duration = Duration::ZERO;
        let read_errors = 0u32;

        loop {
            // 1. Cooperative cancellation check
            if let Some(cancelled) = is_cancelled {
                if cancelled() {
                    warn!(
                        operation_id,
                        total_bytes_acquired,
                        "[CLEANUP] Acquisition cancelled by operator. Attempting incomplete artifact removal."
                    );
                    let _ =
                        work_tx.send(PipelineCommand::Abort(AcquisitionFailureReason::Cancelled));
                    let cleanup = match writer_handle.join() {
                        Ok(Err((_, cleanup, _, _))) => cleanup,
                        _ => ArtifactCleanupResult::confirmed(destination_path.to_path_buf()),
                    };
                    log_cleanup(operation_id, &cleanup);
                    return Err(AcquisitionEngineError::from_cleanup(
                        AcquisitionFailureReason::Cancelled,
                        cleanup,
                    ));
                }
            }

            // 2. Check if writer thread reported an early failure
            if let Ok((err, cleanup)) = writer_err_rx.try_recv() {
                let _ = writer_handle.join();
                log_cleanup(operation_id, &cleanup);
                return Err(AcquisitionEngineError::from_cleanup(err, cleanup));
            }

            // 3. Acquire reusable buffer from free pool (bounded backpressure)
            let mut buffer = match free_rx.recv() {
                Ok(buf) => buf,
                Err(_) => {
                    // Writer thread exited prematurely
                    let (err, cleanup, _, _) = match writer_handle.join() {
                        Ok(Err(res)) => res,
                        _ => (
                            AcquisitionFailureReason::WriteError(
                                "Writer pipeline unexpectedly terminated".to_string(),
                            ),
                            ArtifactCleanupResult::confirmed(destination_path.to_path_buf()),
                            Duration::ZERO,
                            0,
                        ),
                    };
                    log_cleanup(operation_id, &cleanup);
                    return Err(AcquisitionEngineError::from_cleanup(err, cleanup));
                }
            };

            let current_offset = total_bytes_acquired;

            // 4. Sequential read from source physical stream
            let r_start = Instant::now();
            let read_bytes = match stream.read_chunk(&mut buffer[..chunk_size]) {
                Ok(n) => {
                    read_duration += r_start.elapsed();
                    n
                }
                Err(e) => {
                    read_duration += r_start.elapsed();
                    let _ = read_errors;
                    error!(
                        operation_id,
                        offset = current_offset,
                        error = %e,
                        "[READ_ERROR] Read failure from physical source stream"
                    );
                    let _ = work_tx.send(PipelineCommand::Abort(e.clone()));
                    let cleanup = match writer_handle.join() {
                        Ok(Err((_, cleanup, _, _))) => cleanup,
                        _ => ArtifactCleanupResult::confirmed(destination_path.to_path_buf()),
                    };
                    log_cleanup(operation_id, &cleanup);
                    return Err(AcquisitionEngineError::from_cleanup(e, cleanup));
                }
            };

            // 5. Handle EOF
            if read_bytes == 0 {
                // If expected size was non-zero and we read less than expected, treat as short read
                if expected_total_bytes > 0 && total_bytes_acquired < expected_total_bytes {
                    error!(
                        operation_id,
                        expected = expected_total_bytes,
                        actual = total_bytes_acquired,
                        "[READ_ERROR] Unexpected EOF: Short read detected"
                    );
                    let err = AcquisitionFailureReason::ShortRead {
                        expected: expected_total_bytes,
                        actual: total_bytes_acquired,
                    };
                    let _ = work_tx.send(PipelineCommand::Abort(err.clone()));
                    let cleanup = match writer_handle.join() {
                        Ok(Err((_, cleanup, _, _))) => cleanup,
                        _ => ArtifactCleanupResult::confirmed(destination_path.to_path_buf()),
                    };
                    log_cleanup(operation_id, &cleanup);
                    return Err(AcquisitionEngineError::from_cleanup(err, cleanup));
                }

                // Normal EOF reached
                let _ = work_tx.send(PipelineCommand::Complete);
                break;
            }

            // 6. Update streaming SHA-256 with exact bytes read (strictly in-order)
            let h_start = Instant::now();
            hasher.update(&buffer[..read_bytes]);
            hash_duration += h_start.elapsed();

            // 7. Enqueue buffer to writer thread via bounded channel
            total_bytes_acquired += read_bytes as u64;
            chunks_processed += 1;

            if let Err(_) = work_tx.send(PipelineCommand::Write {
                buffer,
                valid_len: read_bytes,
            }) {
                let (err, cleanup, _, _) = match writer_handle.join() {
                    Ok(Err(res)) => res,
                    _ => (
                        AcquisitionFailureReason::WriteError(
                            "Writer thread closed pipeline channel".to_string(),
                        ),
                        ArtifactCleanupResult::confirmed(destination_path.to_path_buf()),
                        Duration::ZERO,
                        0,
                    ),
                };
                log_cleanup(operation_id, &cleanup);
                return Err(AcquisitionEngineError::from_cleanup(err, cleanup));
            }

            if chunks_processed <= 3 || chunks_processed.is_multiple_of(100) {
                info!(
                    operation_id,
                    chunks_processed,
                    offset = current_offset,
                    bytes_read = read_bytes,
                    total_bytes_acquired,
                    "[READ_COMPLETE] Successfully acquired and hashed chunk"
                );
            }

            // 8. Telemetry: byte-accurate progress with performance stats
            let now = Instant::now();
            if chunks_processed == 1
                || now.duration_since(last_progress_time).as_millis() >= 100
                || (expected_total_bytes > 0 && total_bytes_acquired >= expected_total_bytes)
            {
                last_progress_time = now;
                let current_stats = AcquisitionPerformanceStats {
                    chunk_size_bytes: chunk_size,
                    chunks_processed,
                    read_duration_ms: read_duration.as_millis() as u64,
                    hash_duration_ms: hash_duration.as_millis() as u64,
                    write_duration_ms: 0,
                    total_duration_ms: start_time.elapsed().as_millis() as u64,
                    effective_throughput_mbps: if start_time.elapsed().as_secs_f64() > 0.0 {
                        (total_bytes_acquired as f64 / (1024.0 * 1024.0))
                            / start_time.elapsed().as_secs_f64()
                    } else {
                        0.0
                    },
                    read_errors_encountered: read_errors,
                    write_errors_encountered: 0,
                };

                emit_progress(
                    operation_id,
                    total_bytes_acquired,
                    expected_total_bytes,
                    start_time,
                    false,
                    Some(current_stats),
                    on_progress,
                );
            }
        }

        // 9. Join writer thread and retrieve results
        let (writer, final_size, write_duration, write_errors) = match writer_handle.join() {
            Ok(Ok(res)) => res,
            Ok(Err((err, cleanup, _, _))) => {
                log_cleanup(operation_id, &cleanup);
                return Err(AcquisitionEngineError::from_cleanup(err, cleanup));
            }
            Err(_) => {
                let cleanup =
                    crate::writer::delete_artifact_verified(&staging_path_for(destination_path));
                log_cleanup(operation_id, &cleanup);
                return Err(AcquisitionEngineError::from_cleanup(
                    AcquisitionFailureReason::WriteError("Writer thread panicked".to_string()),
                    cleanup,
                ));
            }
        };

        // 10. Finalize streaming SHA-256
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

        // 11. Promote staging artifact to final image name only after verified integrity
        let (promoted_size, promoted_path) = match writer.finalize_to_destination() {
            Ok(v) => v,
            Err(e) => {
                error!(
                    operation_id,
                    error = %e,
                    "[FINALIZE_FAILED] Staging image could not be promoted"
                );
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

        let total_duration_ms = start_time.elapsed().as_millis() as u64;
        let avg_throughput = if elapsed_seconds > 0.0 {
            (total_bytes_acquired as f64 / (1024.0 * 1024.0)) / elapsed_seconds
        } else {
            0.0
        };

        let perf_stats = AcquisitionPerformanceStats {
            chunk_size_bytes: chunk_size,
            chunks_processed,
            read_duration_ms: read_duration.as_millis() as u64,
            hash_duration_ms: hash_duration.as_millis() as u64,
            write_duration_ms: write_duration.as_millis() as u64,
            total_duration_ms,
            effective_throughput_mbps: (avg_throughput * 10.0).round() / 10.0,
            read_errors_encountered: read_errors,
            write_errors_encountered: write_errors,
        };

        info!(
            operation_id,
            final_size = promoted_size,
            sha256 = %image_sha256,
            elapsed_seconds,
            effective_throughput = perf_stats.effective_throughput_mbps,
            "[ACQUISITION_COMPLETE] Bitstream acquisition successfully completed and verified"
        );

        emit_progress(
            operation_id,
            total_bytes_acquired,
            total_bytes_acquired,
            start_time,
            true,
            Some(perf_stats.clone()),
            on_progress,
        );

        Ok(AcquisitionEngineOutput {
            image_path: promoted_path,
            image_size_bytes: promoted_size,
            image_sha256,
            bytes_acquired: total_bytes_acquired,
            elapsed_seconds,
            diagnostics: perf_stats,
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
    perf_stats: Option<AcquisitionPerformanceStats>,
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
        performance_stats: perf_stats,
    });
}
