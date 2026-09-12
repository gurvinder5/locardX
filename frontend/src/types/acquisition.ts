/**
 * Types for LocardX Forensic Disk Acquisition / Imaging Layer
 * Step 11: Non-destructive, read-only bitstream physical acquisition
 */

export interface AcquisitionDeviceSnapshot {
  device_id: string;
  display_name: string;
  vendor?: string | null;
  model?: string | null;
  serial_number?: string | null;
  media_type: string;
  capacity_bytes: number;
  sector_size: number;
  bus_type?: string | null;
  is_removable: boolean;
  is_system: boolean;
  snapshot_timestamp: string;
}

export interface AcquisitionPlan {
  plan_id: string;
  source: AcquisitionDeviceSnapshot;
  destination_path: string;
  image_format: string;
  chunk_size_bytes: number;
  allow_overwrite: boolean;
  estimated_size_bytes: number;
  created_at: string;
}

export interface AcquisitionProgress {
  operation_id: string;
  bytes_acquired: number;
  total_bytes: number;
  percentage: number;
  throughput_mbps: number;
  elapsed_seconds: number;
  eta_seconds?: number | null;
  stage: string;
}

export type AcquisitionStatus =
  | 'Pending'
  | 'Validating'
  | 'Acquiring'
  | 'Completed'
  | 'Failed'
  | 'Cancelled';

export interface AcquisitionFailureReason {
  type?: string;
  message?: string;
  [key: string]: unknown;
}

export interface AcquisitionResult {
  acquisition_id: string;
  operation_id: string;
  source: AcquisitionDeviceSnapshot;
  destination_path: string;
  image_format: string;
  image_size_bytes: number;
  image_sha256: string;
  status: AcquisitionStatus;
  bytes_acquired: number;
  elapsed_seconds: number;
  average_throughput_mbps: number;
  failure_reason?: AcquisitionFailureReason | null;
  audit_reference: string;
  started_at: string;
  completed_at: string;
}

export interface AcquisitionArtifact {
  acquisition_id: string;
  image_path: string;
  image_format: string;
  image_size_bytes: number;
  image_sha256: string;
  source_device_snapshot: AcquisitionDeviceSnapshot;
  acquisition_timestamp: string;
  is_verified: boolean;
  audit_reference: string;
}

export interface CreateAcquisitionPlanRequest {
  source_device_id: string;
  destination_path: string;
  chunk_size_bytes?: number | null;
  allow_overwrite?: boolean | null;
  session_token?: string | null;
}

export interface StartAcquisitionRequest {
  plan: AcquisitionPlan;
  session_token?: string | null;
}

export interface ValidateSourceResponse {
  valid: boolean;
  message: string;
}

export interface ValidateDestinationRequest {
  destination_path: string;
  source_device_id: string;
  required_capacity_bytes: number;
  allow_overwrite: boolean;
}

export interface ValidateDestinationResponse {
  valid: boolean;
  available_free_bytes: number;
  required_bytes: number;
  message: string;
}

export interface ArtifactVerificationResponse {
  verified: boolean;
  expected_hash: string;
  calculated_hash: string;
  expected_size: number;
  actual_size: number;
  error_message?: string | null;
}
