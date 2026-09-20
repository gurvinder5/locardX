use crate::carving::carve_stream;
use crate::classification::rules::{infer_file_type_from_extension, resolve_category};
use crate::confidence::score::calculate_confidence;
use crate::models::{
    RecoveredFile, RecoveryFailureReason, RecoveryMode, RecoveryPlan, RecoveryProgress,
    RecoveryResult, RecoveryStatus,
};
use crate::signature::registry::SignatureRegistry;
use crate::tsk::bridge::run_tsk_inspection_guided;
use crate::validation::validator::evaluate_candidate;
use chrono::Utc;
use sha2::Digest;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

pub fn sanitize_filename(name: &str) -> String {
    let clean = name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'], "_");
    let trimmed = clean.trim().trim_matches('.');
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("CON")
        || trimmed.eq_ignore_ascii_case("PRN")
        || trimmed.eq_ignore_ascii_case("AUX")
        || trimmed.eq_ignore_ascii_case("NUL")
    {
        format!("file_{}", Uuid::new_v4())
    } else {
        trimmed.to_string()
    }
}

/// Core execution engine orchestrating filesystem analysis and structural carving passes.
pub struct RecoveryEngine;

impl RecoveryEngine {
    /// Executes a planned recovery against an evidence disk image strictly read-only.
    pub fn execute<F: FnMut(RecoveryProgress)>(
        plan: &RecoveryPlan,
        operation_id: &str,
        cancellation_token: Option<&Arc<AtomicBool>>,
        mut progress_cb: F,
    ) -> Result<RecoveryResult, RecoveryFailureReason> {
        let image_path = Path::new(&plan.source_image_path);
        if !image_path.exists() {
            return Err(RecoveryFailureReason::ImageNotFound(
                plan.source_image_path.clone(),
            ));
        }

        // Safety: ensure output directory is not the evidence image itself
        if !plan.options.output_directory.is_empty() {
            let out_canonical = Path::new(&plan.options.output_directory)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(&plan.options.output_directory));
            let img_canonical = image_path
                .canonicalize()
                .unwrap_or_else(|_| image_path.to_path_buf());

            if out_canonical == img_canonical {
                return Err(RecoveryFailureReason::Unknown(
                    "Output directory cannot be identical to the evidence image".to_string(),
                ));
            }
        }

        let mut file = File::open(image_path).map_err(|e| {
            RecoveryFailureReason::ReadError(format!("Cannot open evidence image: {}", e))
        })?;

        let total_bytes = file
            .metadata()
            .map_err(|e| {
                RecoveryFailureReason::ReadError(format!("Cannot read image metadata: {}", e))
            })?
            .len();

        if total_bytes < 512 {
            return Err(RecoveryFailureReason::CorruptImage(
                format!("Evidence image is too small ({} bytes, minimum 512 bytes required)", total_bytes)
            ));
        }

        let started_at = Utc::now().to_rfc3339();
        let start_instant = std::time::Instant::now();

        let mut recovered_files: Vec<RecoveredFile> = Vec::new();
        let mut existing_offsets: Vec<u64> = Vec::new();
        let mut candidates_evaluated: usize = 0;

        let registry = SignatureRegistry::new();

        // Stage 1: Filesystem Metadata Analysis (if mode is All or FilesystemOnly)
        if plan.options.recovery_mode == RecoveryMode::All
            || plan.options.recovery_mode == RecoveryMode::FilesystemOnly
        {
            progress_cb(RecoveryProgress {
                operation_id: operation_id.to_string(),
                bytes_scanned: 0,
                total_bytes,
                percentage: 5.0,
                throughput_mbps: 0.0,
                elapsed_seconds: start_instant.elapsed().as_secs_f64(),
                files_found: 0,
                stage: "Inspecting partition tables and filesystem metadata".to_string(),
                filesystem_type: None,
                phase: Some("ANALYZING".to_string()),
                entries_examined: 0,
                deleted_candidates: 0,
                files_validated: 0,
                current_operation: Some("Scanning partition tables and volume boot headers".to_string()),
            });

            let mut files_validated_count = 0;

            let tsk_res = {
                let mut p_cb = |tsk_prog: crate::tsk::filesystem::TskProgressUpdate| {
                    progress_cb(RecoveryProgress {
                        operation_id: operation_id.to_string(),
                        bytes_scanned: (total_bytes / 4).min(total_bytes),
                        total_bytes,
                        percentage: 25.0,
                        throughput_mbps: 0.0,
                        elapsed_seconds: start_instant.elapsed().as_secs_f64(),
                        files_found: tsk_prog.deleted_candidates,
                        stage: format!("Analyzing {} metadata", tsk_prog.filesystem_type),
                        filesystem_type: Some(tsk_prog.filesystem_type),
                        phase: Some("ANALYZING".to_string()),
                        entries_examined: tsk_prog.entries_examined,
                        deleted_candidates: tsk_prog.deleted_candidates,
                        files_validated: files_validated_count,
                        current_operation: Some(tsk_prog.current_operation),
                    });
                };
                run_tsk_inspection_guided(&mut file, total_bytes, cancellation_token, &mut p_cb)
            };

            // If user explicitly requested FilesystemOnly mode, verify filesystem inspection succeeded
            if plan.options.recovery_mode == RecoveryMode::FilesystemOnly {
                if tsk_res.detected_filesystems.is_empty() {
                    return Err(RecoveryFailureReason::UnsupportedFilesystem(
                        "No recognized or supported filesystem (NTFS, FAT32, ext4) detected on evidence image".to_string(),
                    ));
                }
                if let Some(err) = &tsk_res.analysis_error {
                    if tsk_res.metadata_files.is_empty() {
                        return Err(RecoveryFailureReason::FilesystemAnalysisFailed(err.clone()));
                    }
                }
            }

            let detected_fs_display = if !tsk_res.detected_filesystems.is_empty() {
                Some(tsk_res.detected_filesystems.join(", "))
            } else {
                None
            };

            candidates_evaluated += tsk_res.total_entries_examined;
            let total_meta_candidates = tsk_res.metadata_files.len();

            for (idx, meta_file) in tsk_res.metadata_files.into_iter().enumerate() {
                if let Some(token) = cancellation_token {
                    if token.load(Ordering::SeqCst) {
                        break;
                    }
                }

                let file_end = match meta_file.source_offset.checked_add(meta_file.size_bytes) {
                    Some(end) => end,
                    None => continue,
                };
                if file_end > total_bytes {
                    continue;
                }

                existing_offsets.push(meta_file.source_offset);

                let ext = Path::new(&meta_file.name)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let file_type = infer_file_type_from_extension(ext);

                // Filter by target file types if requested
                if let Some(targets) = &plan.options.target_file_types {
                    if !targets.iter().any(|t| t == &file_type) {
                        continue;
                    }
                }

                progress_cb(RecoveryProgress {
                    operation_id: operation_id.to_string(),
                    bytes_scanned: (total_bytes / 2).min(total_bytes),
                    total_bytes,
                    percentage: 50.0 + ((idx as f64 / total_meta_candidates.max(1) as f64) * 45.0),
                    throughput_mbps: 0.0,
                    elapsed_seconds: start_instant.elapsed().as_secs_f64(),
                    files_found: recovered_files.len() + 1,
                    stage: "Validating and extracting deleted file artifact".to_string(),
                    filesystem_type: detected_fs_display.clone(),
                    phase: Some("VALIDATING".to_string()),
                    entries_examined: candidates_evaluated,
                    deleted_candidates: total_meta_candidates,
                    files_validated: files_validated_count,
                    current_operation: Some(format!("Extracting & validating candidate: {}", meta_file.name)),
                });

                // Cap in-memory validation buffer to at most 16 MiB to prevent memory exhaustion
                let val_buf_size = (meta_file.size_bytes.min(16 * 1024 * 1024)) as usize;
                let mut data_buf = vec![0u8; val_buf_size];
                if file.seek(SeekFrom::Start(meta_file.source_offset)).is_ok()
                    && file.read_exact(&mut data_buf).is_ok()
                {
                    let (val_status, mut sha256_hash, factors) = evaluate_candidate(
                        &file_type, &data_buf, &registry, true, // From filesystem metadata
                        true,
                    );

                    let (score, grade) = calculate_confidence(&factors);
                    files_validated_count += 1;

                    if score < plan.options.min_confidence_score {
                        continue;
                    }

                    let safe_name = sanitize_filename(&meta_file.name);

                    let rel_path = if !plan.options.output_directory.is_empty() {
                        let out_dir = Path::new(&plan.options.output_directory);
                        let category_folder = file_type.category().to_string().to_lowercase();
                        let target_dir = out_dir.join(&category_folder);
                        let _ = std::fs::create_dir_all(&target_dir);

                        let target_path = target_dir.join(&safe_name);
                        if let Ok(mut out_file) = File::create(&target_path) {
                            if meta_file.size_bytes <= 16 * 1024 * 1024 {
                                let _ = out_file.write_all(&data_buf);
                            } else {
                                // Stream large file in 1 MiB chunks to prevent OOM
                                let _ = file.seek(SeekFrom::Start(meta_file.source_offset));
                                let mut hasher = sha2::Sha256::new();
                                let mut chunk = vec![0u8; 1024 * 1024];
                                let mut rem = meta_file.size_bytes;
                                while rem > 0 {
                                    let to_read = (rem as usize).min(chunk.len());
                                    if file.read_exact(&mut chunk[..to_read]).is_err() {
                                        break;
                                    }
                                    let _ = out_file.write_all(&chunk[..to_read]);
                                    hasher.update(&chunk[..to_read]);
                                    rem -= to_read as u64;
                                }
                                if rem == 0 {
                                    sha256_hash = format!("{:x}", hasher.finalize());
                                }
                            }
                            Some(format!("{}/{}", category_folder, safe_name))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    recovered_files.push(RecoveredFile {
                        file_id: format!("file-rec-{}", Uuid::new_v4()),
                        job_id: String::new(),
                        source_offset: meta_file.source_offset,
                        size_bytes: meta_file.size_bytes,
                        file_type: file_type.clone(),
                        category: resolve_category(&file_type),
                        mime_type: file_type.default_mime_type().to_string(),
                        suggested_filename: meta_file.name,
                        recovery_method: crate::models::RecoveryMethod::FilesystemMetadata,
                        validation_status: val_status,
                        confidence_score: score,
                        confidence_grade: grade,
                        is_fragmented: false,
                        sha256_hash,
                        output_relative_path: rel_path,
                        evidence_factors: factors,
                        created_at: Utc::now().to_rfc3339(),
                    });
                }
            }

            progress_cb(RecoveryProgress {
                operation_id: operation_id.to_string(),
                bytes_scanned: if plan.options.recovery_mode == RecoveryMode::FilesystemOnly { total_bytes } else { total_bytes / 2 },
                total_bytes,
                percentage: if plan.options.recovery_mode == RecoveryMode::FilesystemOnly { 100.0 } else { 50.0 },
                throughput_mbps: 0.0,
                elapsed_seconds: start_instant.elapsed().as_secs_f64(),
                files_found: recovered_files.len(),
                stage: "Filesystem metadata analysis completed".to_string(),
                filesystem_type: detected_fs_display,
                phase: Some(if plan.options.recovery_mode == RecoveryMode::FilesystemOnly { "COMPLETED".to_string() } else { "RECOVERING".to_string() }),
                entries_examined: candidates_evaluated,
                deleted_candidates: total_meta_candidates,
                files_validated: files_validated_count,
                current_operation: Some(format!("Recovered {} files from filesystem metadata", recovered_files.len())),
            });
        }

        // Stage 2: Sliding Window Raw Carving (if mode is All or CarvingOnly)
        let mut bytes_scanned = total_bytes;
        if plan.options.recovery_mode == RecoveryMode::All
            || plan.options.recovery_mode == RecoveryMode::CarvingOnly
        {
            if let Some(token) = cancellation_token {
                if !token.load(Ordering::SeqCst) {
                    let (carved, scanned, c_evaluated) = carve_stream(
                        image_path,
                        &plan.options,
                        operation_id,
                        cancellation_token,
                        &mut progress_cb,
                        &existing_offsets,
                    )
                    .map_err(|e| RecoveryFailureReason::ReadError(e.to_string()))?;

                    bytes_scanned = scanned;
                    candidates_evaluated += c_evaluated;
                    recovered_files.extend(carved);
                }
            } else {
                let (carved, scanned, c_evaluated) = carve_stream(
                    image_path,
                    &plan.options,
                    operation_id,
                    cancellation_token,
                    &mut progress_cb,
                    &existing_offsets,
                )
                .map_err(|e| RecoveryFailureReason::ReadError(e.to_string()))?;

                bytes_scanned = scanned;
                candidates_evaluated += c_evaluated;
                recovered_files.extend(carved);
            }
        }

        let is_cancelled = cancellation_token
            .map(|t| t.load(Ordering::SeqCst))
            .unwrap_or(false);

        let status = if is_cancelled {
            RecoveryStatus::Cancelled
        } else {
            RecoveryStatus::Completed
        };

        let elapsed_seconds = start_instant.elapsed().as_secs_f64();
        let completed_at = Utc::now().to_rfc3339();

        Ok(RecoveryResult {
            job_id: String::new(), // Assigned by service
            operation_id: operation_id.to_string(),
            acquisition_id: plan.acquisition_id.clone(),
            source_image_path: plan.source_image_path.clone(),
            source_image_sha256: plan.source_image_sha256.clone(),
            status,
            bytes_scanned,
            files_recovered: recovered_files.len(),
            candidates_evaluated,
            elapsed_seconds,
            failure_reason: if is_cancelled {
                Some(RecoveryFailureReason::Cancelled)
            } else {
                None
            },
            recovered_files,
            audit_reference: format!("audit-rec-{}", Uuid::new_v4()),
            started_at,
            completed_at,
        })
    }
}
