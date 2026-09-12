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

const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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
      original_device_id: '\\.\\PhysicalDrive1',
      original_serial: 'WD-WCC4M1234567',
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

  await new Promise((resolve) => setTimeout(resolve, 300));
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
  return {
    plan_id: `plan-rec-${Date.now()}`,
    acquisition_id: request.artifact.acquisition_id,
    source_image_path: request.artifact.image_path,
    source_image_sha256: request.artifact.image_sha256,
    options: request.options,
    created_at: new Date().toISOString(),
  };
}

export async function startRecovery(
  request: StartRecoveryRequest
): Promise<RecoveryResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveryResult>('start_recovery', { request });
  }

  await new Promise((resolve) => setTimeout(resolve, 1000));
  const mockFiles: RecoveredFile[] = [
    {
      file_id: 'file-rec-01',
      job_id: 'job-mock-01',
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
      file_id: 'file-rec-02',
      job_id: 'job-mock-01',
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

  return {
    job_id: `job-rec-${Date.now()}`,
    operation_id: `op-rec-${Date.now()}`,
    acquisition_id: request.plan.acquisition_id,
    source_image_path: request.plan.source_image_path,
    source_image_sha256: request.plan.source_image_sha256,
    status: 'Completed',
    bytes_scanned: 1073741824,
    files_recovered: mockFiles.length,
    candidates_evaluated: 12,
    elapsed_seconds: 3.45,
    failure_reason: null,
    recovered_files: mockFiles,
    audit_reference: `audit-rec-${Date.now()}`,
    started_at: new Date().toISOString(),
    completed_at: new Date().toISOString(),
  };
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

  return true;
}

export async function getRecoveryProgress(
  operationId: string
): Promise<RecoveryProgress | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveryProgress | null>('get_recovery_progress', { operationId });
  }

  return null;
}

export async function getRecoveryJob(jobId: string): Promise<RecoveryResult | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveryResult | null>('get_recovery_job', { jobId });
  }

  return null;
}

export async function listRecoveryJobs(): Promise<RecoveryResult[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveryResult[]>('list_recovery_jobs');
  }

  return [];
}

export async function getRecoveredFiles(jobId: string): Promise<RecoveredFile[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveredFile[]>('get_recovered_files', { jobId });
  }

  return [];
}

export async function exportRecoveredFiles(
  request: ExportRecoveredFilesRequest
): Promise<number> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<number>('export_recovered_files', { request });
  }

  return 2;
}

export async function getRecoveryReport(jobId: string): Promise<RecoveryReport | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<RecoveryReport | null>('get_recovery_report', { jobId });
  }

  return null;
}
