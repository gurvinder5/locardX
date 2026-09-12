import { TargetIdentity } from './integrity';

export type OperationType =
  | 'IntegrityHash'
  | 'IntegrityVerify'
  | 'FileErasure'
  | 'FolderErasure'
  | 'DriveErasure'
  | 'Recovery'
  | 'Unknown';

export type OperationState =
  | 'Created'
  | 'Queued'
  | 'Running'
  | 'Cancelling'
  | 'Cancelled'
  | 'Completed'
  | 'Failed';

export interface OperationProgress {
  percentage?: number | null;
  bytes_processed?: number | null;
  total_bytes?: number | null;
  stage: string;
  message: string;
  eta_seconds?: number | null;
}

export interface OperationDto {
  operation_id: string;
  operation_type: OperationType;
  target: TargetIdentity;
  actor_id?: string | null;
  current_state: OperationState;
  progress: OperationProgress;
  result_summary?: string | null;
  failure_reason?: string | null;
  cancellation_requested: boolean;
  created_at: string;
  started_at?: string | null;
  completed_at?: string | null;
}
