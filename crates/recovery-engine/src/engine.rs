use crate::carving::carve_stream;
use crate::classification::rules::{infer_file_type_from_extension, resolve_category};
use crate::confidence::score::calculate_confidence;
use crate::models::{
    RecoveredFile, RecoveryFailureReason, RecoveryMode, RecoveryPlan, RecoveryProgress,
    RecoveryResult, RecoveryStatus,
};
use crate::signature::registry::SignatureRegistry;
use crate::tsk::bridge::run_tsk_inspection;
use crate::validation::validator::evaluate_candidate;
use chrono::Utc;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

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
            });

            let tsk_res = run_tsk_inspection(&mut file, total_bytes);

            for meta_file in tsk_res.metadata_files {
                if let Some(token) = cancellation_token {
                    if token.load(Ordering::SeqCst) {
                        break;
                    }
                }

                if meta_file.source_offset + meta_file.size_bytes > total_bytes {
                    continue;
                }

                candidates_evaluated += 1;
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

                let mut data_buf = vec![0u8; meta_file.size_bytes as usize];
                if file.seek(SeekFrom::Start(meta_file.source_offset)).is_ok()
                    && file.read_exact(&mut data_buf).is_ok()
                {
                    let (val_status, sha256_hash, factors) = evaluate_candidate(
                        &file_type, &data_buf, &registry, true, // From filesystem metadata
                        true,
                    );

                    let (score, grade) = calculate_confidence(&factors);
                    if score < plan.options.min_confidence_score {
                        continue;
                    }

                    let rel_path = if !plan.options.output_directory.is_empty() {
                        let out_dir = Path::new(&plan.options.output_directory);
                        let category_folder = file_type.category().to_string().to_lowercase();
                        let target_dir = out_dir.join(&category_folder);
                        let _ = std::fs::create_dir_all(&target_dir);

                        let target_path = target_dir.join(&meta_file.name);
                        if let Ok(mut out_file) = File::create(&target_path) {
                            let _ = out_file.write_all(&data_buf);
                            Some(format!("{}/{}", category_folder, meta_file.name))
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
