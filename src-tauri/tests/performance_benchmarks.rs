//! LocardX Step 15 — Empirical Performance & Stress Evaluation Benchmarks
//!
//! Measures and outputs actual system performance across:
//! 1. Application & Database Initialization Latency
//! 2. Security & Authentication Overhead (Argon2id hashing, session verification)
//! 3. Cryptographic Streaming Throughput (SHA-256 at 1MB, 10MB, 50MB)
//! 4. Recovery & File Carving Throughput (synthetic disk carving, confidence evaluation)
//! 5. Unified Reporting Generation Latency (aggregated case report generation)
//! 6. Tamper-Evident Audit Hash Chain Verification Scalability (100+ sequential records)

use locardx_acquisition::{AcquisitionArtifact, AcquisitionDeviceSnapshot};
use locardx_auth::models::{CreateUserRequest, InitAdminRequest, LoginRequest, UserRole};
use locardx_case_management::models::{
    AddEvidenceRequest, CreateCaseRequest, CustodyEventType, EvidenceType, RecordCustodyRequest,
};
use locardx_desktop::{commands, init_application};
use locardx_recovery_engine::models::{RecoveryMode, RecoveryOptions};
use locardx_recovery_engine::source::compute_streaming_sha256;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

fn create_benchmark_disk(dir: &std::path::Path, total_mb: usize) -> (PathBuf, String, u64) {
    let disk_path = dir.join(format!("bench_disk_{}mb.raw", total_mb));
    let total_size: usize = total_mb * 1024 * 1024;
    let mut disk_bytes = vec![0u8; total_size];

    // Embed MBR signature
    disk_bytes[510] = 0x55;
    disk_bytes[511] = 0xAA;

    // Embed JPEG at offset 4096
    let mut jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    jpeg_data.extend_from_slice(b"JFIF          ");
    jpeg_data.extend_from_slice(&[
        0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x10, 0x00, 0x10, 0x01, 0x01,
    ]);
    jpeg_data.extend_from_slice(&[0xFF, 0xD9]);
    disk_bytes[4096..4096 + jpeg_data.len()].copy_from_slice(&jpeg_data);

    // Embed PNG at offset 16384
    let mut png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    png_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
    png_data.extend_from_slice(b"IHDR");
    png_data.extend_from_slice(&[0; 13]);
    png_data.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]);
    png_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x04]);
    png_data.extend_from_slice(b"IDAT");
    png_data.extend_from_slice(&[1, 2, 3, 4]);
    png_data.extend_from_slice(&[0; 4]);
    png_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    png_data.extend_from_slice(b"IEND");
    png_data.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);
    disk_bytes[16384..16384 + png_data.len()].copy_from_slice(&png_data);

    // Embed PDF at offset 32768
    let mut pdf_data = Vec::new();
    pdf_data.extend_from_slice(b"%PDF-1.7\n");
    pdf_data.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
    pdf_data.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf_data.extend_from_slice(b"trailer << /Root 1 0 R >>\nstartxref\n120\n%%EOF");
    disk_bytes[32768..32768 + pdf_data.len()].copy_from_slice(&pdf_data);

    // Fill some pseudo-random payload across remaining sectors to emulate real disk entropy
    for i in (65536..total_size).step_by(512) {
        disk_bytes[i] = ((i / 512) % 255) as u8;
        disk_bytes[i + 1] = 0xAA;
    }

    let mut f = std::fs::File::create(&disk_path).expect("Create benchmark disk");
    f.write_all(&disk_bytes).expect("Write benchmark disk");
    f.flush().expect("Flush benchmark disk");

    let mut sha = Sha256::new();
    sha.update(&disk_bytes);
    let hash = hex::encode(sha.finalize());

    (disk_path, hash, total_size as u64)
}

#[tokio::test]
async fn run_master_performance_evaluation() {
    std::env::set_var("LOCARDX_ENV", "test");
    std::env::set_var("LOCARDX_DB_PATH", ":memory:");

    println!("\n=======================================================");
    println!("   LOCARDX FORENSIC WORKSTATION BENCHMARK SUITE");
    println!("=======================================================\n");

    // 1. Startup and Database Initialization Latency
    let t_start = Instant::now();
    let state = init_application().expect("Init application core");
    let init_duration = t_start.elapsed();
    println!(
        "[METRIC] App Startup & DB Schema Migration: {:.2} ms",
        init_duration.as_secs_f64() * 1000.0
    );

    // 2. Closed User Provisioning & Argon2id Authentication Latency
    let t_admin_init = Instant::now();
    let _admin_user = commands::initialize_admin_handler(
        &state,
        InitAdminRequest {
            username: "bench_admin".to_string(),
            password: "BenchmarkPassword2026!".to_string(),
            confirm_password: "BenchmarkPassword2026!".to_string(),
            display_name: Some("Bench Admin".to_string()),
        },
    )
    .unwrap();
    let admin_init_dur = t_admin_init.elapsed();
    println!(
        "[METRIC] Initial Admin Provisioning (Argon2id Hash + Audit): {:.2} ms",
        admin_init_dur.as_secs_f64() * 1000.0
    );

    let t_login = Instant::now();
    let admin_login = commands::login_handler(
        &state,
        LoginRequest {
            username: "bench_admin".to_string(),
            password: "BenchmarkPassword2026!".to_string(),
        },
    )
    .unwrap();
    let login_dur = t_login.elapsed();
    println!(
        "[METRIC] User Login Authentication (Argon2id Verify + Token + Audit): {:.2} ms",
        login_dur.as_secs_f64() * 1000.0
    );

    let t_create_user = Instant::now();
    let inv_user = commands::create_user_handler(
        &state,
        &admin_login.token,
        CreateUserRequest {
            username: "bench_investigator".to_string(),
            password: "InvestigatorPassword2026!".to_string(),
            role: UserRole::Investigator,
            display_name: Some("Bench Investigator".to_string()),
            metadata_json: None,
        },
    )
    .unwrap();
    let create_user_dur = t_create_user.elapsed();
    println!(
        "[METRIC] Investigator User Provisioning: {:.2} ms",
        create_user_dur.as_secs_f64() * 1000.0
    );

    // 3. Cryptographic Streaming SHA-256 Throughput at 1MB, 10MB, 50MB
    let temp_dir = std::env::temp_dir().join(format!("locardx_bench_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let sizes_mb = [1, 10, 50];
    for &mb in &sizes_mb {
        let (disk_file, expected_hash, total_bytes) = create_benchmark_disk(&temp_dir, mb);

        let t_hash = Instant::now();
        let computed_hash = compute_streaming_sha256(&disk_file).expect("Compute streaming hash");
        let hash_dur = t_hash.elapsed();

        assert_eq!(computed_hash, expected_hash);

        let secs = hash_dur.as_secs_f64();
        let mb_per_sec = (mb as f64) / secs;
        println!(
            "[METRIC] Streaming SHA-256 ({} MB): {:.2} ms ({:.2} MB/s)",
            mb,
            secs * 1000.0,
            mb_per_sec
        );

        let _ = std::fs::remove_file(disk_file);
    }

    // 4. File Carving & Recovery Throughput (10 MB synthetic forensic disk)
    let (carve_disk, carve_hash, carve_capacity) = create_benchmark_disk(&temp_dir, 10);
    let artifact = AcquisitionArtifact {
        acquisition_id: "acq-bench-001".to_string(),
        image_path: carve_disk.to_string_lossy().to_string(),
        image_format: "raw".to_string(),
        image_size_bytes: carve_capacity,
        image_sha256: carve_hash.clone(),
        source_device_snapshot: AcquisitionDeviceSnapshot {
            device_id: carve_disk.to_string_lossy().to_string(),
            display_name: "Benchmark Drive 10MB".to_string(),
            vendor: Some("LocardX".to_string()),
            model: Some("Virtual Test Disk".to_string()),
            serial_number: Some("BENCH-SN-01".to_string()),
            media_type: "PhysicalDisk".to_string(),
            capacity_bytes: carve_capacity,
            sector_size: 512,
            bus_type: Some("USB".to_string()),
            is_removable: true,
            is_system: false,
            snapshot_timestamp: chrono::Utc::now().to_rfc3339(),
        },
        acquisition_timestamp: chrono::Utc::now().to_rfc3339(),
        is_verified: true,
        audit_reference: "audit-bench-01".to_string(),
    };

    let rec_opts = RecoveryOptions {
        recovery_mode: RecoveryMode::CarvingOnly,
        target_file_types: None,
        output_directory: temp_dir.join("bench_carved").to_string_lossy().to_string(),
        enable_fragment_reconstruction: true,
        min_confidence_score: 50,
        chunk_size_bytes: 65536,
    };

    let t_plan = Instant::now();
    let rec_plan = state
        .recovery
        .create_plan(&artifact, rec_opts, Some("bench_admin"))
        .unwrap();
    let plan_dur = t_plan.elapsed();
    println!(
        "[METRIC] Recovery Plan Creation: {:.2} ms",
        plan_dur.as_secs_f64() * 1000.0
    );

    let t_carve = Instant::now();
    let rec_res = state
        .recovery
        .execute_recovery(&rec_plan, Some("bench_admin"))
        .unwrap();
    let carve_dur = t_carve.elapsed();
    let carve_secs = carve_dur.as_secs_f64();
    let carve_mb_per_sec = 10.0 / carve_secs;
    println!(
        "[METRIC] Raw File Carving (10 MB, 3 files carved): {:.2} ms ({:.2} MB/s)",
        carve_secs * 1000.0,
        carve_mb_per_sec
    );
    assert_eq!(rec_res.files_recovered, 3);

    // 5. Case Management & Unified Forensic Reporting Latency
    let case = state
        .case_service
        .create_case(
            CreateCaseRequest {
                case_reference: "CASE-BENCH-2026".to_string(),
                title: "Benchmark Case".to_string(),
                description: "Latency evaluation case".to_string(),
                metadata_json: None,
            },
            &inv_user,
        )
        .unwrap();

    let _ev1 = state
        .case_service
        .add_case_evidence(
            &case.case_id,
            AddEvidenceRequest {
                evidence_type: EvidenceType::PhysicalStorage,
                identifier: carve_disk.to_string_lossy().to_string(),
                label: "Benchmark Physical Drive".to_string(),
                sha256: Some(carve_hash.clone()),
                size_bytes: Some(carve_capacity),
                notes: None,
            },
            &inv_user,
        )
        .unwrap();

    let _op1 = state
        .case_service
        .associate_operation(
            &case.case_id,
            &rec_res.job_id,
            "FileRecovery",
            &inv_user,
            Some("Bench recovery operation"),
        )
        .unwrap();

    let _cust = state
        .case_service
        .record_custody_event(
            &case.case_id,
            RecordCustodyRequest {
                evidence_id: None,
                event_type: CustodyEventType::EvidenceAnalyzed,
                action: "Carving analysis executed".to_string(),
                details: "Bench custody tracking".to_string(),
            },
            &inv_user,
        )
        .unwrap();

    let t_report = Instant::now();
    let report = state
        .case_service
        .generate_case_report(&case.case_id, &inv_user)
        .unwrap();
    let report_dur = t_report.elapsed();
    println!(
        "[METRIC] Unified Forensic Case Report Generation: {:.2} ms (Digest: {})",
        report_dur.as_secs_f64() * 1000.0,
        &report.integrity.report_digest[..16]
    );

    let t_verify_rep = Instant::now();
    assert!(state.case_service.verify_case_report(&report));
    let rep_ver_dur = t_verify_rep.elapsed();
    println!(
        "[METRIC] Forensic Case Report Canonical Verification: {:.2} ms",
        rep_ver_dur.as_secs_f64() * 1000.0
    );

    // 6. Audit Chain Scalability & Verification Benchmark (Scale to 100+ events)
    println!("\n--- Audit Chain Scalability Test (100 sequential events) ---");
    let t_audit_append = Instant::now();
    for i in 0..100 {
        state
            .audit
            .log_structured_event(
                "BENCHMARK_OPERATION",
                Some("bench_admin"),
                Some(&format!("target_{}", i)),
                &format!("Sequential stress record #{}", i),
            )
            .unwrap();
    }
    let append_dur = t_audit_append.elapsed();
    println!(
        "[METRIC] 100 Audit Log Appends (SHA-256 Chain + DB Write): {:.2} ms ({:.2} ms/event)",
        append_dur.as_secs_f64() * 1000.0,
        (append_dur.as_secs_f64() * 1000.0) / 100.0
    );

    let t_audit_verify = Instant::now();
    let audit_ver = state.audit.verify_chain().unwrap();
    let verify_dur = t_audit_verify.elapsed();
    assert!(audit_ver.is_valid);
    assert!(audit_ver.total_events >= 100);
    println!(
        "[METRIC] Audit Chain Full Verification ({} events): {:.2} ms ({:.2} us/event)",
        audit_ver.total_events,
        verify_dur.as_secs_f64() * 1000.0,
        (verify_dur.as_secs_f64() * 1_000_000.0) / (audit_ver.total_events as f64)
    );

    println!("\n=======================================================");
    println!("   BENCHMARK SUITE COMPLETED SUCCESSFULLY");
    println!("=======================================================\n");

    let _ = std::fs::remove_dir_all(&temp_dir);
}
