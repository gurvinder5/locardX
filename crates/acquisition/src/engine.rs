use crate::hasher::StreamingSha256Hasher;
use crate::models::{AcquisitionFailureReason, AcquisitionProgress};
use crate::platform::ReadOnlyDeviceStream;
use crate::writer::RawImageWriter;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Output of a successfully executed acquisition stream pass.
#[derive(Debug, Clone)]
pub struct AcquisitionEngineOutput {
    pub image_path: PathBuf,
    pub image_size_bytes: u64,
    pub image_sha256: String,
    pub bytes_acquired: u64,
    pub elapsed_seconds: f64,
}

/// The core read-only streaming acquisition engine.
pub struct AcquisitionEngine;

impl AcquisitionEngine {
    /// Executes a bitstream raw/dd acquisition from `stream` to `destination_path`.
    /// Simultaneously computes the streaming SHA-256 hash and emits progress telemetry.
    pub fn execute(
        operation_id: &str,
        mut stream: Box<dyn ReadOnlyDeviceStream>,
        destination_path: &Path,
        chunk_size_bytes: usize,
        expected_total_bytes: u64,
        is_cancelled: Option<&(dyn Fn() -> bool + Send + Sync)>,
        on_progress: Option<&(dyn Fn(AcquisitionProgress) + Send + Sync)>,
    ) -> Result<AcquisitionEngineOutput, AcquisitionFailureReason> {
        let chunk_size = if chunk_size_bytes == 0 {
            1024 * 1024 // 1 MiB default
        } else {
            chunk_size_bytes
        };

        let mut writer = RawImageWriter::create(destination_path)?;
        let mut hasher = StreamingSha256Hasher::new();
        let mut buffer = vec![0u8; chunk_size];

        let start_time = Instant::now();
        let mut last_progress_time = Instant::now();
        let mut total_bytes_acquired: u64 = 0;

        loop {
            // 1. Cooperative cancellation check
            if let Some(cancelled) = is_cancelled {
                if cancelled() {
                    writer.abort_and_cleanup();
                    return Err(AcquisitionFailureReason::Cancelled);
                }
            }

            // 2. Sequential read from source physical stream
            let read_bytes = match stream.read_chunk(&mut buffer) {
                Ok(n) => n,
                Err(e) => {
                    writer.abort_and_cleanup();
                    return Err(e);
                }
            };

            // 3. Handle EOF
            if read_bytes == 0 {
                // If expected size was non-zero and we read less than expected, treat as short read
                if expected_total_bytes > 0 && total_bytes_acquired < expected_total_bytes {
                    writer.abort_and_cleanup();
                    return Err(AcquisitionFailureReason::ShortRead {
                        expected: expected_total_bytes,
                        actual: total_bytes_acquired,
                    });
                }
                break;
            }

            let chunk = &buffer[..read_bytes];

            // 4. Update streaming SHA-256
            hasher.update(chunk);

            // 5. Write chunk to destination image
            if let Err(e) = writer.write_chunk(chunk) {
                writer.abort_and_cleanup();
                return Err(e);
            }

            total_bytes_acquired += read_bytes as u64;

            // 6. Telemetry and Progress Callback (throttle to ~100ms or 100%)
            let now = Instant::now();
            if now.duration_since(last_progress_time).as_millis() >= 100
                || (expected_total_bytes > 0 && total_bytes_acquired >= expected_total_bytes)
            {
                last_progress_time = now;
                let elapsed_secs = start_time.elapsed().as_secs_f64();
                let throughput_mbps = if elapsed_secs > 0.0 {
                    (total_bytes_acquired as f64 / (1024.0 * 1024.0)) / elapsed_secs
                } else {
                    0.0
                };

                let percentage = if expected_total_bytes > 0 {
                    ((total_bytes_acquired as f64 / expected_total_bytes as f64) * 100.0).min(100.0)
                } else {
                    100.0
                };

                let eta_seconds =
                    if throughput_mbps > 0.0 && expected_total_bytes > total_bytes_acquired {
                        let remaining_mb = (expected_total_bytes - total_bytes_acquired) as f64
                            / (1024.0 * 1024.0);
                        Some(remaining_mb / throughput_mbps)
                    } else {
                        Some(0.0)
                    };

                if let Some(cb) = on_progress {
                    cb(AcquisitionProgress {
                        operation_id: operation_id.to_string(),
                        bytes_acquired: total_bytes_acquired,
                        total_bytes: expected_total_bytes,
                        percentage: (percentage * 10.0).round() / 10.0,
                        throughput_mbps: (throughput_mbps * 10.0).round() / 10.0,
                        elapsed_seconds: (elapsed_secs * 10.0).round() / 10.0,
                        eta_seconds: eta_seconds.map(|e| (e * 10.0).round() / 10.0),
                        stage: "Acquiring raw physical sectors & calculating SHA-256".to_string(),
                    });
                }
            }
        }

        // 7. Flush and sync image writer
        let final_size = match writer.flush_and_sync() {
            Ok(s) => s,
            Err(e) => {
                writer.abort_and_cleanup();
                return Err(e);
            }
        };

        // 8. Finalize streaming SHA-256
        let (image_sha256, hash_bytes) = hasher.finalize();
        let elapsed_seconds = start_time.elapsed().as_secs_f64();

        if final_size != hash_bytes {
            return Err(AcquisitionFailureReason::WriteError(format!(
                "Integrity mismatch: written bytes {} != hashed bytes {}",
                final_size, hash_bytes
            )));
        }

        // Final progress update at 100%
        if let Some(cb) = on_progress {
            let throughput_mbps = if elapsed_seconds > 0.0 {
                (total_bytes_acquired as f64 / (1024.0 * 1024.0)) / elapsed_seconds
            } else {
                0.0
            };

            cb(AcquisitionProgress {
                operation_id: operation_id.to_string(),
                bytes_acquired: total_bytes_acquired,
                total_bytes: total_bytes_acquired,
                percentage: 100.0,
                throughput_mbps: (throughput_mbps * 10.0).round() / 10.0,
                elapsed_seconds: (elapsed_seconds * 10.0).round() / 10.0,
                eta_seconds: Some(0.0),
                stage: "Acquisition & hashing complete".to_string(),
            });
        }

        Ok(AcquisitionEngineOutput {
            image_path: destination_path.to_path_buf(),
            image_size_bytes: final_size,
            image_sha256,
            bytes_acquired: total_bytes_acquired,
            elapsed_seconds,
        })
    }
}
