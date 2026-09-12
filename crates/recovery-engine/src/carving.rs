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

            // Read candidate payload up to max_size
            let max_read = (candidate.signature.max_size as usize)
                .min((total_bytes - candidate.global_offset) as usize);
            if max_read < candidate.signature.min_size as usize {
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
                let actual_size = parsed.detected_size.min(candidate_buf.len() as u64);
                if actual_size < candidate.signature.min_size {
                    continue;
                }

                let file_slice = &candidate_buf[..actual_size as usize];
                let (val_status, sha256_hash, factors) = evaluate_candidate(
                    &parsed.file_type,
                    file_slice,
                    &registry,
                    false, // Carving only
                    true,  // Contiguous candidate
                );

                let (score, grade) = calculate_confidence(&factors);
                if score < options.min_confidence_score {
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
                        let _ = out_file.write_all(file_slice);
                        Some(format!("{}/{}", category_folder, suggested_filename))
                    } else {
                        None
                    }
                } else {
                    None
                };

                let recovered = RecoveredFile {
                    file_id,
                    job_id: String::new(), // Populated by service
                    source_offset: candidate.global_offset,
                    size_bytes: actual_size,
                    file_type: parsed.file_type.clone(),
                    category: parsed.file_type.category(),
                    mime_type: parsed.file_type.default_mime_type().to_string(),
                    suggested_filename,
                    recovery_method: RecoveryMethod::StructureCarving,
                    validation_status: val_status,
                    confidence_score: score,
                    confidence_grade: grade,
                    is_fragmented: false,
                    sha256_hash,
                    output_relative_path: rel_path,
                    evidence_factors: factors,
                    created_at: Utc::now().to_rfc3339(),
                };

                recovered_files.push(recovered);
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

        let pct = (current_offset as f64 / total_bytes as f64 * 100.0).min(100.0);

        progress_cb(RecoveryProgress {
            operation_id: operation_id.to_string(),
            bytes_scanned: current_offset.min(total_bytes),
            total_bytes,
            percentage: pct,
            throughput_mbps: throughput,
            elapsed_seconds: elapsed,
            files_found: recovered_files.len(),
            stage: "Carving raw image sectors".to_string(),
        });
    }

    Ok((recovered_files, total_bytes, candidates_evaluated))
}
