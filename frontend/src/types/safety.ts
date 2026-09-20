import { OperationType } from './operation';
import { TargetIdentity, TargetType } from './integrity';

export type SafetyDecisionOutcome = 'Allowed' | 'Denied' | 'RequiresConfirmation' | 'Blocked';

export type ReasonCode =
  | 'VALID_TARGET'
  | 'SYSTEM_DEVICE'
  | 'BOOT_DEVICE'
  | 'UNKNOWN_TARGET'
  | 'INVALID_TARGET'
  | 'UNAUTHORIZED_ROLE'
  | 'MISSING_CONFIRMATION'
  | 'CONFIRMATION_EXPIRED'
  | 'CONFIRMATION_INVALID'
  | 'TARGET_CHANGED'
  | 'UNSUPPORTED_OPERATION'
  | 'UNSUPPORTED_DEVICE_TYPE'
  | 'UNSUPPORTED_FILESYSTEM'
  | 'SAFETY_POLICY_VIOLATION'
  | 'OPERATION_DISABLED';

export type RiskLevel = 'Low' | 'Medium' | 'High' | 'Critical';

export interface TargetSnapshot {
  target_identifier: string;
  target_type: TargetType;
  capacity_bytes?: number | null;
  filesystem?: string | null;
  device_id?: string | null;
  classification?: string | null;
  is_system: boolean;
  is_boot: boolean;
  snapshot_timestamp: string;
}

export interface SafetyDecision {
  evaluation_id: string;
  target: TargetIdentity;
  operation_type: OperationType;
  actor_id?: string | null;
  decision: SafetyDecisionOutcome;
  reason_code: ReasonCode;
  risk_level: RiskLevel;
  message: string;
  evaluated_at: string;
  target_snapshot?: TargetSnapshot | null;
  requires_confirmation: boolean;
}

export interface ConfirmationChallenge {
  confirmation_id: string;
  operation_id: string;
  actor_id: string;
  target: TargetIdentity;
  operation_type: OperationType;
  risk_level: RiskLevel;
  target_snapshot: TargetSnapshot;
  created_at: string;
  expires_at: string;
}

export interface EvaluateSafetyRequest {
  target_type: string;
  target_identifier: string;
  target_display_name?: string;
  target_size_bytes?: number;
  operation_type: string;
  session_token?: string;
}

export interface RequestConfirmationRequest {
  operation_id: string;
  target_type: string;
  target_identifier: string;
  target_display_name?: string;
  target_size_bytes?: number;
  operation_type: string;
  session_token: string;
}

export interface ConfirmOperationRequest {
  confirmation_id: string;
  operation_id: string;
  warning_acknowledged: boolean;
  typed_target_confirmation: string;
  session_token: string;
}
