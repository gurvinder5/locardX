use crate::confidence::score::calculate_confidence;
use crate::models::{RecoveredFile, RecoveryMethod, RecoveryOptions, RecoveryProgress};
use crate::signature::detector::SignatureDetector;
use crate::signature::registry::SignatureRegistry;
use crate::structure::parser::parse_file_structure;
use crate::validation::validator::evaluate_candidate;
use chrono::Utc;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Scans raw DD / image stream using sliding window carving.
pub fn carve_stream<F: FnMut(RecoveryProgress)>(
    image_path: &Path,
    options: &RecoveryOptions,
    operation_id: &str,
    cancellation_token: Option<&Arc<AtomicBool>>,
    mut progress_cb: F,
    existing_offsets: &[u64],
) -> Result<(Vec<RecoveredFile>, u64, usize), std::io::Error> {
    let mut file = File::open(image_path)?;
    let total_bytes = file.metadata()?.len();

    let registry = match &options.target_file_types {
        Some(types) => {
            let base = SignatureRegistry::new();
            let filtered = base.filter_by_types(types);
            SignatureRegistry::from_signatures(filtered)
        }
        None => SignatureRegistry::new(),
    };

    let detector = SignatureDetector::new(registry.clone());
    let chunk_size = options.chunk_size_bytes.max(64 * 1024);
    let overlap_size = 64 * 1024usize;

    let mut current_offset: u64 = 0;
    let mut recovered_files = Vec::new();
    let mut candidates_evaluated = 0usize;
    let start_time = std::time::Instant::now();

    let mut buffer = vec![0u8; chunk_size];

    while current_offset < total_bytes {
        if let Some(token) = cancellation_token {
            if token.load(Ordering::SeqCst) {
                break;
            }
        }

        file.seek(SeekFrom::Start(current_offset))?;
        let bytes_to_read = ((total_bytes - current_offset) as usize).min(chunk_size);
        file.read_exact(&mut buffer[..bytes_to_read])?;

        let candidates = detector.scan_buffer(&buffer[..bytes_to_read], current_offset);
        tracing::trace!(
            current_offset,
            bytes_scanned = bytes_to_read,
            candidates_found = candidates.len(),
            "Completed sector buffer scan"
        );

        for candidate in candidates {
            if let Some(token) = cancellation_token {
                if token.load(Ordering::SeqCst) {
                    break;
                }
            }

            // Skip if already found via filesystem at same offset
            if existing_offsets.contains(&candidate.global_offset) {
                continue;
            }

            // Avoid carving overlapping starts if already extracted in this run
            if recovered_files.iter().any(|f: &RecoveredFile| {
                candidate.global_offset >= f.source_offset
                    && candidate.global_offset < f.source_offset + f.size_bytes
            }) {
                continue;
            }

            candidates_evaluated += 1;
            tracing::debug!(
                offset = candidate.global_offset,
                file_type = ?candidate.file_type,
                "Evaluating candidate header"
            );

            // Read candidate payload up to max_size (capped at 32 MiB to avoid memory exhaustion)
            let max_read = (candidate.signature.max_size as usize)
                .min(total_bytes.saturating_sub(candidate.global_offset) as usize)
                .min(32 * 1024 * 1024);
            if max_read < candidate.signature.min_size as usize {
                tracing::debug!(
                    offset = candidate.global_offset,
                    available = max_read,
                    min_required = candidate.signature.min_size,
                    "Candidate rejected: remaining bytes less than format minimum"
                );
                continue;
            }

            let mut candidate_buf = vec![0u8; max_read];
            if file.seek(SeekFrom::Start(candidate.global_offset)).is_err() {
                continue;
            }
            if file.read_exact(&mut candidate_buf).is_err() {
                continue;
            }

            // Parse structure to find exact size and validity
            if let Some(parsed) = parse_file_structure(&candidate.file_type, &candidate_buf) {
                let mut actual_size = parsed.detected_size.min(candidate_buf.len() as u64);
                if actual_size < candidate.signature.min_size {
                    continue;
                }

                let mut file_payload = candidate_buf[..actual_size as usize].to_vec();
                let mut is_frag = false;
                let mut rec_method = RecoveryMethod::StructureCarving;

                let (mut val_status, mut sha256_hash, mut factors) = evaluate_candidate(
                    &parsed.file_type,
                    &file_payload,
                    &registry,
                    false, // Carving only
                    true,  // Contiguous candidate
                );

                // Cluster-aware fragment reconstruction
                if options.enable_fragment_reconstruction
                    && (val_status != crate::models::ValidationStatus::Valid || !parsed.is_valid_structure)
                {
                    let cluster_size = 4096u64;
                    let remainder = actual_size % cluster_size;
                    let boundary_offset = if remainder == 0 {
                        candidate.global_offset + actual_size
                    } else {
                        candidate.global_offset + (actual_size / cluster_size + 1) * cluster_size
                    };

                    // Search forward in subsequent clusters for continuation
                    if boundary_offset < total_bytes {
                        let lookahead_max = (boundary_offset + 2 * 1024 * 1024).min(total_bytes);
                        let mut check_offset = boundary_offset;
                        while check_offset + cluster_size <= lookahead_max {
                            let mut cluster_buf = vec![0u8; cluster_size as usize];
                            if file.seek(SeekFrom::Start(check_offset)).is_ok()
                                && file.read_exact(&mut cluster_buf).is_ok()
                            {
                                // Check if this cluster contains end marker for candidate
                                if let Some(footer) = &candidate.signature.footer_magic {
                                    if cluster_buf.windows(footer.len()).any(|w| w == footer.as_slice()) {
                                        // Attempt fragment reconstruction
                                        let f1 = crate::fragmentation::fragment::create_fragment(
                                            "temp-1",
                                            0,
                                            candidate.global_offset,
                                            actual_size,
                                            crate::models::FragmentType::Header,
                                            0.8,
                                        );
                                        let f2 = crate::fragmentation::fragment::create_fragment(
                                            "temp-1",
                                            1,
                                            check_offset,
                                            cluster_size,
                                            crate::models::FragmentType::Trailer,
                                            0.8,
                                        );
                                        let outcome = crate::fragmentation::reconstruction::reconstruct_fragments(
                                            &parsed.file_type,
                                            vec![f1, f2],
                                            vec![file_payload.clone(), cluster_buf.clone()],
                                        );
                                        if outcome.result == crate::models::ReconstructionResult::Reconstructed
                                            || outcome.validation_status == crate::models::ValidationStatus::Valid
                                        {
                                            file_payload = outcome.assembled_bytes;
                                            actual_size = file_payload.len() as u64;
                                            is_frag = true;
                                            rec_method = RecoveryMethod::FragmentReconstruction;
                                            let (new_val, new_hash, new_fac) = evaluate_candidate(
                                                &parsed.file_type,
                                                &file_payload,
                                                &registry,
                                                false,
                                                false, // not physically contiguous
                                            );
                                            val_status = new_val;
                                            sha256_hash = new_hash;
                                            factors = new_fac;
                                            tracing::info!(
                                                offset = candidate.global_offset,
                                                continuation_offset = check_offset,
                                                file_type = ?parsed.file_type,
                                                "Successfully reconstructed fragmented file"
                                            );
                                            break;
                                        }
                                    }
                                }
                            }
                            check_offset += cluster_size;
                        }
                    }

                    if !is_frag && val_status == crate::models::ValidationStatus::Valid && !parsed.is_valid_structure {
                        val_status = crate::models::ValidationStatus::PartiallyValid;
                    }
                }

                let (score, grade) = calculate_confidence(&factors);
                if score < options.min_confidence_score {
                    tracing::debug!(
                        offset = candidate.global_offset,
                        file_type = ?parsed.file_type,
                        confidence_score = score,
                        min_required = options.min_confidence_score,
                        "Candidate discarded: confidence score below threshold"
                    );
                    continue;
                }

                let file_id = format!("file-rec-{}", Uuid::new_v4());
                let suggested_filename = format!(
                    "carved_0x{:08X}.{}",
                    candidate.global_offset,
                    parsed.file_type.extension()
                );

                let rel_path = if !options.output_directory.is_empty() {
                    let out_dir = Path::new(&options.output_directory);
                    let category_folder = parsed.file_type.category().to_string().to_lowercase();
                    let target_dir = out_dir.join(&category_folder);
                    let _ = std::fs::create_dir_all(&target_dir);

                    let target_path = target_dir.join(&suggested_filename);
                    if let Ok(mut out_file) = File::create(&target_path) {
                        let _ = out_file.write_all(&file_payload);
                        Some(format!("{}/{}", category_folder, suggested_filename))
                    } else {
                        None
                    }
                } else {
                    None
                };

                tracing::info!(
                    file_id = %file_id,
                    offset = candidate.global_offset,
                    file_type = %parsed.file_type,
                    size_bytes = actual_size,
                    validation = %val_status,
                    confidence = score,
                    fragmented = is_frag,
                    "Carved forensic artifact successfully"
                );

                let recovered = RecoveredFile {
                    file_id,
                    job_id: String::new(), // Populated by service
                    source_offset: candidate.global_offset,
                    size_bytes: actual_size,
                    file_type: parsed.file_type.clone(),
                    category: parsed.file_type.category(),
                    mime_type: parsed.file_type.default_mime_type().to_string(),
                    suggested_filename,
                    recovery_method: rec_method,
                    validation_status: val_status,
                    confidence_score: score,
                    confidence_grade: grade,
                    is_fragmented: is_frag,
                    sha256_hash,
                    output_relative_path: rel_path,
                    evidence_factors: factors,
                    created_at: Utc::now().to_rfc3339(),
                };

                recovered_files.push(recovered);
            } else {
                tracing::debug!(
                    offset = candidate.global_offset,
                    file_type = ?candidate.file_type,
                    "Candidate rejected: parser returned None"
                );
            }
        }

        // Advance current_offset by chunk_size minus overlap
        let step = if bytes_to_read > overlap_size {
            bytes_to_read - overlap_size
        } else {
            bytes_to_read
        };
        current_offset += step as u64;

        let elapsed = start_time.elapsed().as_secs_f64();
        let throughput = if elapsed > 0.0 {
            (current_offset as f64 / (1024.0 * 1024.0)) / elapsed
        } else {
            0.0
        };

        let pct = if total_bytes > 0 {
            (current_offset as f64 / total_bytes as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        progress_cb(RecoveryProgress {
            operation_id: operation_id.to_string(),
            bytes_scanned: current_offset.min(total_bytes),
            total_bytes,
            percentage: pct,
            throughput_mbps: throughput,
            elapsed_seconds: elapsed,
            files_found: recovered_files.len(),
            stage: "Carving raw image sectors".to_string(),
            filesystem_type: None,
            phase: Some("RECOVERING".to_string()),
            entries_examined: candidates_evaluated,
            deleted_candidates: candidates_evaluated,
            files_validated: recovered_files.len(),
            current_operation: Some(format!("Carved {} files", recovered_files.len())),
        });
    }

    Ok((recovered_files, total_bytes, candidates_evaluated))
}
