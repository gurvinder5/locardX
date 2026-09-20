import { OperationDto } from '../types/operation';

const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// In-memory mock operations for standalone browser development
const MOCK_OPERATIONS: OperationDto[] = [
  {
    operation_id: 'op-mock-101',
    operation_type: 'IntegrityHash',
    target: {
      target_type: 'File',
      identifier: 'C:\\Evidence\\image_case_001.raw',
      display_name: 'image_case_001.raw',
      size_bytes: 524288000,
    },
    actor_id: 'admin',
    current_state: 'Completed',
    progress: {
      percentage: 100.0,
      bytes_processed: 524288000,
      total_bytes: 524288000,
      stage: 'Completed',
      message: 'SHA-256 calculated successfully',
    },
    result_summary: 'Calculated SHA-256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
    failure_reason: null,
    cancellation_requested: false,
    created_at: new Date(Date.now() - 3600000).toISOString(),
    started_at: new Date(Date.now() - 3599000).toISOString(),
    completed_at: new Date(Date.now() - 3595000).toISOString(),
  },
  {
    operation_id: 'op-mock-102',
    operation_type: 'IntegrityVerify',
    target: {
      target_type: 'File',
      identifier: 'D:\\Cases\\payload_89.bin',
      display_name: 'payload_89.bin',
      size_bytes: 1048576,
    },
    actor_id: 'admin',
    current_state: 'Completed',
    progress: {
      percentage: 100.0,
      bytes_processed: 1048576,
      total_bytes: 1048576,
      stage: 'Completed',
      message: 'Status: VERIFIED',
    },
    result_summary: 'VERIFIED (Match: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad)',
    failure_reason: null,
    cancellation_requested: false,
    created_at: new Date(Date.now() - 1800000).toISOString(),
    started_at: new Date(Date.now() - 1799000).toISOString(),
    completed_at: new Date(Date.now() - 1797000).toISOString(),
  },
];

/**
 * Submits and launches an asynchronous SHA-256 evidence hashing operation.
 */
export async function submitIntegrityHashOperation(
  path: string,
  sessionToken?: string
): Promise<OperationDto> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<OperationDto>('submit_integrity_hash_operation', {
      path,
      sessionToken: sessionToken || null,
    });
  }

  // Browser mock fallback
  const op: OperationDto = {
    operation_id: `op-mock-${Date.now()}`,
    operation_type: 'IntegrityHash',
    target: {
      target_type: 'File',
      identifier: path,
      display_name: path.split(/[\\/]/).pop() || 'file.raw',
      size_bytes: 10485760,
    },
    actor_id: 'browser_dev',
    current_state: 'Completed',
    progress: {
      percentage: 100.0,
      bytes_processed: 10485760,
      total_bytes: 10485760,
      stage: 'Completed',
      message: 'Calculated SHA-256',
    },
    result_summary: 'Calculated SHA-256: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',
    failure_reason: null,
    cancellation_requested: false,
    created_at: new Date().toISOString(),
    started_at: new Date().toISOString(),
    completed_at: new Date().toISOString(),
  };

  MOCK_OPERATIONS.unshift(op);
  return op;
}

/**
 * Submits and launches an asynchronous cryptographic integrity verification operation.
 */
export async function submitIntegrityVerifyOperation(
  path: string,
  expectedDigest: string,
  sessionToken?: string
): Promise<OperationDto> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<OperationDto>('submit_integrity_verify_operation', {
      path,
      expectedDigest,
      sessionToken: sessionToken || null,
    });
  }

  // Browser mock fallback
  const op: OperationDto = {
    operation_id: `op-mock-${Date.now()}`,
    operation_type: 'IntegrityVerify',
    target: {
      target_type: 'File',
      identifier: path,
      display_name: path.split(/[\\/]/).pop() || 'file.raw',
      size_bytes: 10485760,
    },
    actor_id: 'browser_dev',
    current_state: 'Completed',
    progress: {
      percentage: 100.0,
      bytes_processed: 10485760,
      total_bytes: 10485760,
      stage: 'Completed',
      message: 'Status: VERIFIED',
    },
    result_summary: `VERIFIED (Match: ${expectedDigest})`,
    failure_reason: null,
    cancellation_requested: false,
    created_at: new Date().toISOString(),
    started_at: new Date().toISOString(),
    completed_at: new Date().toISOString(),
  };

  MOCK_OPERATIONS.unshift(op);
  return op;
}

/**
 * Fetches single operation details by operation ID.
 */
export async function getOperation(operationId: string): Promise<OperationDto> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<OperationDto>('get_operation', { operationId });
  }

  const found = MOCK_OPERATIONS.find((o) => o.operation_id === operationId);
  if (!found) {
    throw new Error(`Operation ${operationId} not found`);
  }
  return found;
}

/**
 * Lists operations from the orchestrator, supporting filtering by state and type.
 */
export async function listOperations(
  limit = 50,
  stateFilter?: string,
  typeFilter?: string
): Promise<OperationDto[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<OperationDto[]>('list_operations', {
      limit,
      stateFilter: stateFilter || null,
      typeFilter: typeFilter || null,
    });
  }

  let list = [...MOCK_OPERATIONS];
  if (stateFilter && stateFilter !== 'ALL') {
    list = list.filter((o) => o.current_state === stateFilter);
  }
  if (typeFilter && typeFilter !== 'ALL') {
    list = list.filter((o) => o.operation_type === typeFilter);
  }
  return list.slice(0, limit);
}

/**
 * Requests cooperative cancellation of an active or queued operation.
 */
export async function cancelOperation(
  operationId: string,
  sessionToken?: string
): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<void>('cancel_operation', {
      operationId,
      sessionToken: sessionToken || null,
    });
  }

  const op = MOCK_OPERATIONS.find((o) => o.operation_id === operationId);
  if (op) {
    op.cancellation_requested = true;
    op.current_state = 'Cancelled';
  }
}
