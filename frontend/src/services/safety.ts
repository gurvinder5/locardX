import {
  ConfirmationChallenge,
  ConfirmOperationRequest,
  EvaluateSafetyRequest,
  RequestConfirmationRequest,
  SafetyDecision,
} from '../types/safety';

const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

const MOCK_DECISIONS: SafetyDecision[] = [
  {
    evaluation_id: 'mock-eval-1',
    target: {
      target_type: 'LogicalVolume',
      identifier: 'C:\\',
      display_name: 'Windows-OS (C:)',
      size_bytes: 512000000000,
    },
    operation_type: 'DriveErasure',
    actor_id: 'admin',
    decision: 'Blocked',
    reason_code: 'SYSTEM_DEVICE',
    risk_level: 'Critical',
    message: 'HARD SAFETY BLOCK: Target is an active System Device and cannot be targeted for destructive sanitization.',
    evaluated_at: new Date(Date.now() - 600000).toISOString(),
    target_snapshot: {
      target_identifier: 'C:\\',
      target_type: 'LogicalVolume',
      capacity_bytes: 512000000000,
      filesystem: 'NTFS',
      device_id: '\\\\.\\PhysicalDrive0',
      classification: 'system_device',
      is_system: true,
      is_boot: false,
      snapshot_timestamp: new Date().toISOString(),
    },
    requires_confirmation: false,
  },
  {
    evaluation_id: 'mock-eval-2',
    target: {
      target_type: 'PhysicalDevice',
      identifier: '\\\\.\\PhysicalDrive1',
      display_name: 'SanDisk Ultra USB 3.0',
      size_bytes: 32000000000,
    },
    operation_type: 'DriveErasure',
    actor_id: 'operator',
    decision: 'RequiresConfirmation',
    reason_code: 'MISSING_CONFIRMATION',
    risk_level: 'High',
    message: 'Target is eligible for review. NOTE: Execution engine status is OPERATION_DISABLED.',
    evaluated_at: new Date(Date.now() - 300000).toISOString(),
    target_snapshot: {
      target_identifier: '\\\\.\\PhysicalDrive1',
      target_type: 'PhysicalDevice',
      capacity_bytes: 32000000000,
      filesystem: 'exFAT',
      device_id: '\\\\.\\PhysicalDrive1',
      classification: 'removable_device',
      is_system: false,
      is_boot: false,
      snapshot_timestamp: new Date().toISOString(),
    },
    requires_confirmation: true,
  },
];

export async function evaluateOperationSafety(
  request: EvaluateSafetyRequest
): Promise<SafetyDecision> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<SafetyDecision>('evaluate_operation_safety', { request });
  }

  // Browser Mock Fallback
  await new Promise((resolve) => setTimeout(resolve, 300));
  const isSys = request.target_identifier.toUpperCase().startsWith('C:');
  const isDestructive = ['DriveErasure', 'FileErasure', 'FolderErasure'].includes(request.operation_type);

  if (isSys && isDestructive) {
    return {
      evaluation_id: `eval-${Date.now()}`,
      target: {
        target_type: request.target_type as any,
        identifier: request.target_identifier,
        display_name: request.target_display_name || request.target_identifier,
        size_bytes: request.target_size_bytes,
      },
      operation_type: request.operation_type as any,
      actor_id: 'browser_user',
      decision: 'Blocked',
      reason_code: 'SYSTEM_DEVICE',
      risk_level: 'Critical',
      message: 'HARD SAFETY BLOCK: System drive C: is protected from all destructive operations.',
      evaluated_at: new Date().toISOString(),
      requires_confirmation: false,
    };
  }

  if (isDestructive) {
    return {
      evaluation_id: `eval-${Date.now()}`,
      target: {
        target_type: request.target_type as any,
        identifier: request.target_identifier,
        display_name: request.target_display_name || request.target_identifier,
        size_bytes: request.target_size_bytes,
      },
      operation_type: request.operation_type as any,
      actor_id: 'browser_user',
      decision: 'RequiresConfirmation',
      reason_code: 'MISSING_CONFIRMATION',
      risk_level: 'High',
      message: 'Target is external/removable. Requires two-stage explicit confirmation. NOTE: Destructive executors are currently disabled.',
      evaluated_at: new Date().toISOString(),
      requires_confirmation: true,
    };
  }

  return {
    evaluation_id: `eval-${Date.now()}`,
    target: {
      target_type: request.target_type as any,
      identifier: request.target_identifier,
      display_name: request.target_display_name || request.target_identifier,
      size_bytes: request.target_size_bytes,
    },
    operation_type: request.operation_type as any,
    actor_id: 'browser_user',
    decision: 'Allowed',
    reason_code: 'VALID_TARGET',
    risk_level: 'Low',
    message: 'Operation is read-only and target is safe.',
    evaluated_at: new Date().toISOString(),
    requires_confirmation: false,
  };
}

export async function requestDestructiveConfirmation(
  request: RequestConfirmationRequest
): Promise<ConfirmationChallenge> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<ConfirmationChallenge>('request_destructive_confirmation', { request });
  }

  // Browser Mock Fallback
  await new Promise((resolve) => setTimeout(resolve, 300));
  return {
    confirmation_id: `conf-mock-${Date.now()}`,
    operation_id: request.operation_id,
    actor_id: 'browser_user',
    target: {
      target_type: request.target_type as any,
      identifier: request.target_identifier,
      display_name: request.target_display_name || request.target_identifier,
      size_bytes: request.target_size_bytes,
    },
    operation_type: request.operation_type as any,
    risk_level: 'High',
    target_snapshot: {
      target_identifier: request.target_identifier,
      target_type: request.target_type as any,
      capacity_bytes: request.target_size_bytes,
      filesystem: 'exFAT',
      device_id: request.target_identifier,
      classification: 'removable_device',
      is_system: false,
      is_boot: false,
      snapshot_timestamp: new Date().toISOString(),
    },
    created_at: new Date().toISOString(),
    expires_at: new Date(Date.now() + 300000).toISOString(),
  };
}

export async function confirmDestructiveOperation(
  request: ConfirmOperationRequest
): Promise<SafetyDecision> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<SafetyDecision>('confirm_destructive_operation', { request });
  }

  // Browser Mock Fallback
  await new Promise((resolve) => setTimeout(resolve, 300));
  return {
    evaluation_id: `eval-conf-${Date.now()}`,
    target: {
      target_type: 'PhysicalDevice',
      identifier: request.typed_target_confirmation,
      display_name: request.typed_target_confirmation,
    },
    operation_type: 'DriveErasure',
    actor_id: 'browser_user',
    decision: 'Blocked',
    reason_code: 'OPERATION_DISABLED',
    risk_level: 'High',
    message: 'Confirmation verified and committed. INVARIANT: Destructive erasure executors are permanently disabled in this foundation phase.',
    evaluated_at: new Date().toISOString(),
    requires_confirmation: false,
  };
}

export async function listSafetyEvaluations(limit: number = 50): Promise<SafetyDecision[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<SafetyDecision[]>('list_safety_evaluations', { limit });
  }

  await new Promise((resolve) => setTimeout(resolve, 200));
  return MOCK_DECISIONS.slice(0, limit);
}
