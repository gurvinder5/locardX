export type HashAlgorithm = 'Sha256';

export type TargetType =
  | 'File'
  | 'Directory'
  | 'LogicalVolume'
  | 'PhysicalDevice'
  | 'EvidenceObject'
  | 'Unknown';

export interface TargetIdentity {
  target_type: TargetType;
  identifier: string;
  display_name: string;
  size_bytes?: number | null;
}

export type HashStatus = 'Success' | { Error: string };

export interface HashResult {
  target: TargetIdentity;
  algorithm: HashAlgorithm;
  digest: string;
  bytes_processed: number;
  started_at: string;
  completed_at: string;
  duration_ms: number;
  status: HashStatus;
}

export type VerificationStatus =
  | 'Verified'
  | { Mismatch: { expected: string; calculated: string } }
  | { UnableToVerify: { reason: string } };

export interface VerificationResult {
  target: TargetIdentity;
  algorithm: HashAlgorithm;
  expected_digest: string;
  calculated_digest?: string | null;
  bytes_processed: number;
  status: VerificationStatus;
  timestamp: string;
  duration_ms: number;
}

export interface IntegrityRecord {
  record_id: string;
  operation_id: string;
  target_type: TargetType;
  target_path: string;
  target_name: string;
  algorithm: HashAlgorithm;
  digest: string;
  bytes_processed: number;
  verification_status: string;
  expected_digest?: string | null;
  actor_id?: string | null;
  created_at: string;
}
