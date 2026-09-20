export type FileEraseScope = 'File' | 'Folder';

export type FileEraseStatus =
  | 'Planned'
  | 'Authorized'
  | 'Running'
  | 'Completed'
  | 'Failed'
  | 'Cancelled'
  | 'VerificationFailed'
  | 'UnableToVerify';

export interface FileMetadataSnapshotDto {
  path: string;
  canonical_path: string;
  is_directory: boolean;
  size_bytes: number;
  created_at?: string | null;
  modified_at?: string | null;
  is_readonly: boolean;
  pre_erasure_sha256?: string | null;
  snapshot_timestamp: string;
}

export interface FileVerificationResultDto {
  outcome: string;
  strategy: string;
  path_exists: boolean;
  inaccessible: boolean;
  details: string;
  verified_at: string;
}

export interface FileErasePlanDto {
  plan_id: string;
  target_path: string;
  canonical_path: string;
  scope: FileEraseScope;
  method: string;
  passes: number;
  verification_strategy: string;
  limitations: string[];
  pre_metadata: FileMetadataSnapshotDto;
  risk_level: string;
  created_at: string;
  scope_description: string;
}

export interface FolderEraseStatsDto {
  total_files: number;
  files_sanitized: number;
  files_failed: number;
  files_cancelled: number;
  total_directories: number;
  directories_removed: number;
  bytes_sanitized: number;
}

export interface FileEraseResultDto {
  operation_id: string;
  plan_id: string;
  target_path: string;
  canonical_path: string;
  scope: FileEraseScope;
  method: string;
  status: FileEraseStatus;
  bytes_processed: number;
  verification: FileVerificationResultDto;
  folder_stats?: FolderEraseStatsDto | null;
  failure_reason?: string | null;
  started_at: string;
  completed_at: string;
  limitations: string[];
  scope_description: string;
}

export interface PlanFileEraseRequest {
  target_path: string;
  method?: string | null;
  session_token?: string | null;
}

export interface PlanFolderEraseRequest {
  target_path: string;
  method?: string | null;
  session_token?: string | null;
}

export interface ExecuteFileEraseRequest {
  plan_id: string;
  confirmation_id: string;
  operation_id: string;
  typed_confirmation: string;
  warning_acknowledged: boolean;
  session_token: string;
}

export interface ExecuteFolderEraseRequest {
  plan_id: string;
  confirmation_id: string;
  operation_id: string;
  typed_confirmation: string;
  warning_acknowledged: boolean;
  session_token: string;
}
