/**
 * Forensic Recovery Service API Client
 * Step 12: Advanced File Recovery & Carving
 */

import { AcquisitionArtifact } from '../types/acquisition';
import {
  CreateRecoveryPlanRequest,
  ExportRecoveredFilesRequest,
  RecoveredFile,
  RecoveryPlan,
  RecoveryProgress,
  RecoveryReport,
  RecoveryResult,
  RecoverySourceSnapshot,
  StartRecoveryRequest,
} from '../types/recovery';
import { recordAuditEvent } from './audit';

const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// In-memory store of recovery jobs and progress for local development / preview
let mockRecoveryJobs: RecoveryResult[] = [];
const mockActiveProgress = new Map<string, RecoveryProgress>();
const mockCancellationTokens = new Set<string>();

export async function listRecoverySources(): Promise<RecoverySourceSnapshot[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoverySourceSnapshot[]>('list_recovery_sources');
  }

  // Browser Fallback for dev/preview
  await new Promise((resolve) => setTimeout(resolve, 150));
  return [
    {
      source_id: 'src-mock-01',
      acquisition_id: 'acq-mock-dd-01',
      image_path: 'C:\\ForensicEvidence\\DiskImage_Case402.raw',
      image_size_bytes: 1073741824, // 1 GB
      image_sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08',
      original_device_id: '\\\\.\\PhysicalDrive1',
      original_serial: 'WD-WCC4M1234567',
      verified_at: new Date().toISOString(),
      is_trusted: true,
    },
    {
      source_id: 'src-mock-02',
      acquisition_id: 'acq-mock-dd-02',
      image_path: 'C:\\ForensicEvidence\\SanDisk_Ultra_USB.dd',
      image_size_bytes: 536870912, // 512 MB
      image_sha256: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
      original_device_id: '\\\\.\\PhysicalDrive2',
      original_serial: 'SD-998877665544',
      verified_at: new Date().toISOString(),
      is_trusted: true,
    },
  ];
}

export async function validateRecoverySource(
  artifact: AcquisitionArtifact
): Promise<RecoverySourceSnapshot> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoverySourceSnapshot>('validate_recovery_source', { artifact });
  }

  await new Promise((resolve) => setTimeout(resolve, 250));
  return {
    source_id: `src-${Date.now()}`,
    acquisition_id: artifact.acquisition_id,
    image_path: artifact.image_path,
    image_size_bytes: artifact.image_size_bytes,
    image_sha256: artifact.image_sha256,
    original_device_id: artifact.source_device_snapshot.device_id,
    original_serial: artifact.source_device_snapshot.serial_number,
    verified_at: new Date().toISOString(),
    is_trusted: true,
  };
}

export async function createRecoveryPlan(
  request: CreateRecoveryPlanRequest
): Promise<RecoveryPlan> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveryPlan>('create_recovery_plan', { request });
  }

  await new Promise((resolve) => setTimeout(resolve, 200));
  const plan: RecoveryPlan = {
    plan_id: `plan-rec-${Date.now()}`,
    acquisition_id: request.artifact.acquisition_id,
    source_image_path: request.artifact.image_path,
    source_image_sha256: request.artifact.image_sha256,
    options: request.options,
    created_at: new Date().toISOString(),
  };

  // Record evidential planning event into hash chain
  await recordAuditEvent(
    'RECOVERY_PLANNED',
    'operator',
    plan.acquisition_id,
    `Recovery plan ${plan.plan_id} created for ${plan.source_image_path} (Mode: ${plan.options.recovery_mode})`
  );

  return plan;
}

export async function startRecovery(
  request: StartRecoveryRequest
): Promise<RecoveryResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveryResult>('start_recovery', { request });
  }

  const opId = request.operation_id || `op-rec-${Date.now()}`;
  const jobId = `job-rec-${Date.now()}`;
  mockCancellationTokens.delete(opId);

  // 1. Log Recovery Started to tamper-evident audit ledger
  await recordAuditEvent(
    'RECOVERY_STARTED',
    'operator',
    request.plan.acquisition_id,
    `Forensic recovery job ${jobId} (Operation: ${opId}) initiated on image ${request.plan.source_image_path}`
  );

  // Initialize progress
  mockActiveProgress.set(opId, {
    operation_id: opId,
    bytes_scanned: 0,
    total_bytes: 1073741824,
    percentage: 5,
    throughput_mbps: 0,
    elapsed_seconds: 0,
    files_found: 0,
    stage: 'Analyzing',
    phase: 'Analyzing filesystem metadata and MFT records',
  });

  // Multi-phase background simulation with cancellation checks
  const phases = [
    { delay: 800, percent: 35, phase: 'Analyzing filesystem records and unallocated clusters', carved: 0, eval: 8 },
    { delay: 1000, percent: 65, phase: 'Signature carving and cluster reassembly', carved: 1, eval: 18 },
    { delay: 1000, percent: 90, phase: 'Structure validation and entropy checking', carved: 2, eval: 24 },
  ];

  for (const step of phases) {
    await new Promise((r) => setTimeout(r, step.delay));
    if (mockCancellationTokens.has(opId)) {
      mockActiveProgress.delete(opId);
      await recordAuditEvent(
        'RECOVERY_CANCELLED',
        'operator',
        request.plan.acquisition_id,
        `Recovery job ${jobId} was explicitly cancelled by operator`
      );
      const cancelResult: RecoveryResult = {
        job_id: jobId,
        operation_id: opId,
        acquisition_id: request.plan.acquisition_id,
        source_image_path: request.plan.source_image_path,
        source_image_sha256: request.plan.source_image_sha256,
        status: 'Cancelled',
        bytes_scanned: 524288000,
        files_recovered: 0,
        candidates_evaluated: 12,
        elapsed_seconds: 1.8,
        failure_reason: 'Recovery operation was cancelled by operator',
        recovered_files: [],
        audit_reference: `audit-rec-${Date.now()}`,
        started_at: new Date(Date.now() - 1800).toISOString(),
        completed_at: new Date().toISOString(),
      };
      mockRecoveryJobs.unshift(cancelResult);
      return cancelResult;
    }

    mockActiveProgress.set(opId, {
      operation_id: opId,
      bytes_scanned: Math.floor((1073741824 * step.percent) / 100),
      total_bytes: 1073741824,
      percentage: step.percent,
      throughput_mbps: 45 + Math.random() * 30,
      elapsed_seconds: step.delay / 1000,
      files_found: step.carved,
      stage: step.percent < 50 ? 'Analyzing' : step.percent < 80 ? 'Recovering' : 'Validating',
      phase: step.phase,
    });
  }

  // Final completion
  const mockFiles: RecoveredFile[] = [
    {
      file_id: `file-rec-${Date.now()}-1`,
      job_id: jobId,
      source_offset: 65536,
      size_bytes: 204800,
      file_type: 'JPEG',
      category: 'images',
      mime_type: 'image/jpeg',
      suggested_filename: 'carved_0x00010000.jpg',
      recovery_method: 'structure_carving',
      validation_status: 'valid',
      confidence_score: 95,
      confidence_grade: 'high',
      is_fragmented: false,
      sha256_hash: 'a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90',
      output_relative_path: 'images/carved_0x00010000.jpg',
      evidence_factors: [
        { factor_type: 'HEADER_MAGIC_MATCH', weight: 20, description: 'SOI verified', passed: true },
        { factor_type: 'FOOTER_MATCH', weight: 20, description: 'EOI verified', passed: true },
        { factor_type: 'STRUCTURE_VALIDATION', weight: 25, description: 'SOF/DQT validated', passed: true },
        { factor_type: 'CLUSTER_CONTIGUOUS', weight: 10, description: 'Contiguous run', passed: true },
      ],
      created_at: new Date().toISOString(),
    },
    {
      file_id: `file-rec-${Date.now()}-2`,
      job_id: jobId,
      source_offset: 524288,
      size_bytes: 1450000,
      file_type: 'PDF',
      category: 'documents',
      mime_type: 'application/pdf',
      suggested_filename: 'carved_0x00080000.pdf',
      recovery_method: 'structure_carving',
      validation_status: 'valid',
      confidence_score: 90,
      confidence_grade: 'high',
      is_fragmented: false,
      sha256_hash: 'b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1',
      output_relative_path: 'documents/carved_0x00080000.pdf',
      evidence_factors: [
        { factor_type: 'HEADER_MAGIC_MATCH', weight: 20, description: '%PDF verified', passed: true },
        { factor_type: 'FOOTER_MATCH', weight: 20, description: '%%EOF verified', passed: true },
        { factor_type: 'STRUCTURE_VALIDATION', weight: 25, description: 'XREF validated', passed: true },
      ],
      created_at: new Date().toISOString(),
    },
  ];

  const auditRef = `audit-rec-${Date.now()}`;
  const finalResult: RecoveryResult = {
    job_id: jobId,
    operation_id: opId,
    acquisition_id: request.plan.acquisition_id,
    source_image_path: request.plan.source_image_path,
    source_image_sha256: request.plan.source_image_sha256,
    status: 'Completed',
    bytes_scanned: 1073741824,
    files_recovered: mockFiles.length,
    candidates_evaluated: 24,
    elapsed_seconds: 3.2,
    failure_reason: null,
    recovered_files: mockFiles,
    audit_reference: auditRef,
    started_at: new Date(Date.now() - 3200).toISOString(),
    completed_at: new Date().toISOString(),
  };

  mockActiveProgress.delete(opId);
  mockRecoveryJobs.unshift(finalResult);

  // 2. Log Recovery Completed to tamper-evident audit ledger
  await recordAuditEvent(
    'RECOVERY_COMPLETED',
    'operator',
    request.plan.acquisition_id,
    `Recovery job ${jobId} completed successfully: ${mockFiles.length} file(s) carved (1024 MB scanned in 3.20s). Audit ref: ${auditRef}`
  );

  return finalResult;
}

export async function cancelRecovery(
  operationId: string,
  sessionToken?: string | null
): Promise<boolean> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<boolean>('cancel_recovery', {
      operationId,
      sessionToken,
    });
  }

  mockCancellationTokens.add(operationId);
  return true;
}

export async function getRecoveryProgress(
  operationId: string
): Promise<RecoveryProgress | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveryProgress | null>('get_recovery_progress', { operationId });
  }

  return mockActiveProgress.get(operationId) || null;
}

export async function getRecoveryJob(jobId: string): Promise<RecoveryResult | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveryResult | null>('get_recovery_job', { jobId });
  }

  return mockRecoveryJobs.find((j) => j.job_id === jobId) || null;
}

export async function listRecoveryJobs(): Promise<RecoveryResult[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveryResult[]>('list_recovery_jobs');
  }

  return [...mockRecoveryJobs];
}

export async function getRecoveredFiles(jobId: string): Promise<RecoveredFile[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveredFile[]>('get_recovered_files', { jobId });
  }

  const job = mockRecoveryJobs.find((j) => j.job_id === jobId);
  return job?.recovered_files || [];
}

export async function exportRecoveredFiles(
  request: ExportRecoveredFilesRequest
): Promise<number> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<number>('export_recovered_files', { request });
  }

  await new Promise((r) => setTimeout(r, 400));
  const job = mockRecoveryJobs.find((j) => j.job_id === request.job_id);
  const count = job?.files_recovered || 2;

  await recordAuditEvent(
    'RECOVERY_FILES_EXPORTED',
    'operator',
    request.job_id,
    `Exported ${count} carved file(s) to destination directory: ${request.export_dir}`
  );

  return count;
}

export async function getRecoveryReport(jobId: string): Promise<RecoveryReport | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveryReport | null>('get_recovery_report', { jobId });
  }

  const job = mockRecoveryJobs.find((j) => j.job_id === jobId);
  if (!job) return null;

  return {
    report_id: `rep-rec-${job.job_id}`,
    job_id: job.job_id,
    acquisition_id: job.acquisition_id,
    source_image_sha256: job.source_image_sha256,
    total_files_recovered: job.files_recovered,
    category_counts: { images: 1, documents: 1 },
    average_confidence: 82,
    report_digest: `digest-${job.job_id}-${Date.now()}`,
    audit_reference: job.audit_reference,
    generated_at: new Date().toISOString(),
  };
}

export async function openRecoveredFile(jobId: string, fileId: string): Promise<string> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string>('open_recovered_file', {
      request: { job_id: jobId, file_id: fileId },
    });
  }

  return `file://${jobId}/${fileId}`;
}

export async function revealRecoveredFile(jobId: string, fileId: string): Promise<string> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string>('reveal_recovered_file', {
      request: { job_id: jobId, file_id: fileId },
    });
  }

  return `workspace://${jobId}/${fileId}`;
}
