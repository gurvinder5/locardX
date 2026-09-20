export type MediaType =
  | 'Hdd'
  | 'Ssd'
  | 'NvmeSsd'
  | 'UsbRemovable'
  | 'MemoryCard'
  | 'ExternalStorage'
  | 'Unknown';

export type SanitizationScope =
  | 'File'
  | 'Folder'
  | 'LogicalVolume'
  | 'PhysicalDevice';

export type SanitizationMethod =
  | 'Nist80088ClearZero'
  | 'Nist80088PurgeCrypto'
  | 'Dod522022M'
  | 'NvmeCryptoErase'
  | 'AtaSecureErase'
  | 'BlockZeroOverwrite'
  | 'LogicalFileShred'
  | 'Unsupported';

export type VerificationStrategy =
  | 'FullReadBack'
  | 'SampledRandomSectors'
  | 'CryptoKeyDestructionCheck'
  | 'MetadataUnlinkCheck'
  | 'NoVerification';

export interface SanitizationPlanDto {
  plan_id: string;
  target_identifier: string;
  target_type: string;
  media_type: string;
  scope: string;
  recommended_method: string;
  is_applicable: boolean;
  risk_level: string;
  verification_strategy: string;
  applicable_standard?: string | null;
  standard_method_id?: string | null;
  limitations: string[];
  reason_codes: string[];
  created_at: string;
  is_dry_run: boolean;
}

export interface SanitizationStandardDto {
  standard_id: string;
  standard_name: string;
  method_id: string;
  method_name: string;
  applicable_scopes: string[];
  applicable_media: string[];
  recommended_verification: string;
  verification_requirements: string;
  limitations: string[];
  description: string;
}

export interface SnapshotComparisonResultDto {
  matches: boolean;
  reason?: string | null;
  differences: string[];
  plan_target_identifier: string;
  current_target_identifier?: string | null;
}

export interface VerificationPlanDto {
  plan_id: string;
  target_identifier: string;
  verification_strategy: string;
  verification_requirements: string;
  limitations: string[];
  applicable_standard?: string | null;
}

export interface EvaluatePlanRequest {
  target_identifier: string;
  target_type: string;
  scope: string;
  requested_method?: string | null;
  requested_strategy?: string | null;
  session_token?: string | null;
}
