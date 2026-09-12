export type ExecutionMode = 'Simulation' | 'RealHardware';

export type DriveCapability =
  | 'SequentialWrite'
  | 'FullDeviceRead'
  | 'SectorAccess'
  | 'AtaSecureErase'
  | 'AtaSanitize'
  | 'NvmeFormat'
  | 'NvmeSanitize'
  | 'NvmeCryptoErase'
  | 'RemovableMedia';

export interface DriveCapabilities {
  supported_capabilities: DriveCapability[];
  interface_bus: string;
  sector_size: number;
  is_rotational: boolean;
  supports_crypto_erase: boolean;
  supports_firmware_sanitize: boolean;
  supports_overwrite: boolean;
}

export type CapabilityState = 'Supported' | 'Unsupported' | 'Unknown' | 'DetectionFailed';

export interface DriveCapabilitiesAssessment {
  overall_state: CapabilityState;
  capabilities: DriveCapabilities;
  capability_states: Record<DriveCapability, CapabilityState>;
  method_support: Record<DriveSanitizationMethod, CapabilityState>;
  assessment_notes: string[];
}

export type DriveSanitizationMethod =
  | 'Nist80088ClearZero'
  | 'Dod522022M'
  | 'AtaSecureErase'
  | 'NvmeFormatSanitize'
  | 'NvmeCryptoErase'
  | 'BlockZeroOverwrite';

export type DriveVerificationStrategy =
  | 'FullDeviceReadVerify'
  | 'FirmwareStatusVerify'
  | 'CryptoKeyDestructionCheck'
  | 'SampledSectorVerification'
  | 'NotApplicable';

export interface DriveVerificationPlan {
  strategy: DriveVerificationStrategy;
  sample_percentage: number | null;
  requirements: string;
  limitations: string[];
}

export type DriveExecutionState =
  | 'PLANNED'
  | 'AUTHORIZED'
  | 'PRE_EXECUTION_CHECK'
  | 'EXECUTION_READY'
  | 'EXECUTING'
  | 'EXECUTION_COMPLETE'
  | 'VERIFYING'
  | 'COMPLETED'
  | 'FAILED'
  | 'CANCELLED'
  | 'UNKNOWN'
  | 'VERIFICATION_FAILED'
  | 'UNABLE_TO_VERIFY';

export type DriveEraseStatus =
  | 'Planned'
  | 'Authorized'
  | 'PreExecutionCheck'
  | 'ExecutionReady'
  | 'Executing'
  | 'ExecutionComplete'
  | 'Verifying'
  | 'Simulating'
  | 'Completed'
  | 'Failed'
  | 'Cancelled'
  | 'Unknown'
  | 'VerificationFailed'
  | 'UnableToVerify';

export interface PhysicalDeviceSnapshot {
  device_id: string;
  display_name: string;
  vendor: string | null;
  model: string | null;
  serial_number: string | null;
  media_type: string;
  capacity_bytes: number;
  sector_size: number;
  physical_sector_size?: number | null;
  bus_type?: string | null;
  is_system: boolean;
  is_boot: boolean;
  is_removable: boolean;
  is_read_only?: boolean;
  exists?: boolean;
  classification: string;
  partition_count: number;
  volume_labels: string[];
  snapshot_timestamp: string;
}

export interface DriveErasePlan {
  plan_id: string;
  physical_device_id: string;
  display_name: string;
  vendor: string | null;
  model: string | null;
  serial_number: string | null;
  media_type: string;
  capacity_bytes: number;
  sector_size: number;
  method: DriveSanitizationMethod;
  passes: number;
  execution_mode: ExecutionMode;
  verification_plan: DriveVerificationPlan;
  limitations: string[];
  risk_level: string;
  device_snapshot: PhysicalDeviceSnapshot;
  created_at: string;
}

export interface DriveVerificationResult {
  outcome: string;
  strategy: DriveVerificationStrategy;
  details: string;
  verified_at: string;
}

export interface DriveEraseResult {
  operation_id: string;
  plan_id: string;
  physical_device_id: string;
  display_name: string;
  vendor: string | null;
  model: string | null;
  serial_number: string | null;
  media_type: string;
  capacity_bytes: number;
  sector_size: number;
  method: DriveSanitizationMethod;
  execution_mode: ExecutionMode;
  status: DriveEraseStatus;
  bytes_processed: number;
  elapsed_seconds: number;
  verification: DriveVerificationResult;
  failure_reason: string | null;
  audit_references: string[];
  started_at: string;
  completed_at: string;
  limitations: string[];
}

export interface DriveEraseProgress {
  operation_id: string;
  percentage: number;
  bytes_processed: number;
  total_bytes: number;
  current_pass: number;
  total_passes: number;
  current_stage: string;
  elapsed_seconds: number;
  eta_seconds: number | null;
}

export interface PlanDriveEraseRequest {
  target_device_id: string;
  requested_method?: string | null;
  execution_mode?: ExecutionMode | null;
  session_token?: string | null;
}

export interface ExecuteDriveEraseSimulationRequest {
  plan_id: string;
  confirmation_id: string;
  operation_id: string;
  typed_confirmation: string;
  warning_acknowledged: boolean;
  session_token: string;
}

export interface DrivePrivilegeStatus {
  is_elevated: boolean;
  platform: string;
  message: string;
}

export interface ReportDeviceInfo {
  physical_device_id: string;
  display_name: string;
  vendor: string | null;
  model: string | null;
  serial_number: string | null;
  media_type: string;
  capacity_bytes: number;
  sector_size: number;
  bus_type: string | null;
}

export interface ReportOperationInfo {
  sanitization_method: string;
  execution_mode: string;
  is_simulation: boolean;
  started_at: string;
  completed_at: string;
  elapsed_seconds: number;
}

export interface ReportExecutionMetrics {
  status: string;
  bytes_processed: number;
  total_bytes: number;
  failure_reason: string | null;
  limitations: string[];
}

export interface ReportVerificationInfo {
  strategy: string;
  outcome: string;
  details: string;
  evidence_digest: string;
  verified_at: string;
}

export interface ReportIntegrity {
  audit_chain_reference: string;
  report_digest: string;
  generated_at: string;
}

export interface DriveSanitizationReport {
  report_id: string;
  operation_id: string;
  plan_id: string;
  actor_id: string | null;
  device_info: ReportDeviceInfo;
  operation_info: ReportOperationInfo;
  execution_metrics: ReportExecutionMetrics;
  verification: ReportVerificationInfo;
  integrity: ReportIntegrity;
}

