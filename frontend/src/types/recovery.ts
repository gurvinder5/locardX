/**
 * Types for LocardX Advanced File Recovery & Carving Module
 * Step 12: Read-only evidential file recovery and structural carving
 */

import { AcquisitionArtifact } from './acquisition';

export interface RecoverySourceSnapshot {
  source_id: string;
  acquisition_id: string;
  image_path: string;
  image_size_bytes: number;
  image_sha256: string;
  original_device_id: string;
  original_serial?: string | null;
  verified_at: string;
  is_trusted: boolean;
}

export type FileCategory =
  | 'images'
  | 'documents'
  | 'archives'
  | 'databases'
  | 'text_files'
  | 'unknown';

export type RecoveryMethod =
  | 'filesystem_metadata'
  | 'signature_carving'
  | 'structure_carving'
  | 'fragment_reconstruction';

export type ValidationStatus =
  | 'valid'
  | 'partially_valid'
  | 'corrupted'
  | 'incomplete'
  | 'invalid';

export type ConfidenceGrade = 'high' | 'medium' | 'low' | 'uncertain';

export interface EvidenceFactor {
  factor_type: string;
  weight: number;
  description: string;
  passed: boolean;
}

export interface RecoveredFile {
  file_id: string;
  job_id: string;
  source_offset: number;
  size_bytes: number;
  file_type: string;
  category: FileCategory;
  mime_type: string;
  suggested_filename: string;
  recovery_method: RecoveryMethod;
  validation_status: ValidationStatus;
  confidence_score: number;
  confidence_grade: ConfidenceGrade;
  is_fragmented: boolean;
  sha256_hash: string;
  output_relative_path?: string | null;
  evidence_factors: EvidenceFactor[];
  created_at: string;
}

export type RecoveryMode = 'all' | 'filesystem_only' | 'carving_only' | 'custom';

export interface RecoveryOptions {
  recovery_mode: RecoveryMode;
  target_file_types?: string[] | null;
  output_directory: string;
  enable_fragment_reconstruction: boolean;
  min_confidence_score: number;
  chunk_size_bytes: number;
}

export interface RecoveryPlan {
  plan_id: string;
  acquisition_id: string;
  source_image_path: string;
  source_image_sha256: string;
  options: RecoveryOptions;
  created_at: string;
}

export interface RecoveryProgress {
  operation_id: string;
  bytes_scanned: number;
  total_bytes: number;
  percentage: number;
  throughput_mbps: number;
  elapsed_seconds: number;
  files_found: number;
  stage: string;
}

export type RecoveryStatus =
  | 'Pending'
  | 'Validating'
  | 'Scanning'
  | 'Completed'
  | 'Failed'
  | 'Cancelled';

export interface RecoveryResult {
  job_id: string;
  operation_id: string;
  acquisition_id: string;
  source_image_path: string;
  source_image_sha256: string;
  status: RecoveryStatus;
  bytes_scanned: number;
  files_recovered: number;
  candidates_evaluated: number;
  elapsed_seconds: number;
  failure_reason?: string | null;
  recovered_files: RecoveredFile[];
  audit_reference: string;
  started_at: string;
  completed_at: string;
}

export interface RecoveryReport {
  report_id: string;
  job_id: string;
  acquisition_id: string;
  source_image_sha256: string;
  total_files_recovered: number;
  category_counts: Record<string, number>;
  average_confidence: number;
  report_digest: string;
  audit_reference: string;
  generated_at: string;
}

export interface CreateRecoveryPlanRequest {
  artifact: AcquisitionArtifact;
  options: RecoveryOptions;
  session_token?: string | null;
}

export interface StartRecoveryRequest {
  plan: RecoveryPlan;
  session_token?: string | null;
}

export interface ExportRecoveredFilesRequest {
  job_id: string;
  export_dir: string;
}
