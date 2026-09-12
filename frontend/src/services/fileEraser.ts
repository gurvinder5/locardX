import {
  ExecuteFileEraseRequest,
  ExecuteFolderEraseRequest,
  FileErasePlanDto,
  FileEraseResultDto,
  PlanFileEraseRequest,
  PlanFolderEraseRequest,
} from '../types/fileEraser';

const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function planFileErasure(
  request: PlanFileEraseRequest
): Promise<FileErasePlanDto> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<FileErasePlanDto>('plan_file_erasure', { request });
  }

  // Browser Mock Fallback
  await new Promise((resolve) => setTimeout(resolve, 300));
  if (request.target_path.toUpperCase().includes('WINDOWS') || request.target_path.toUpperCase().startsWith('C:\\')) {
    throw new Error('Target is a protected system or boot critical location.');
  }

  return {
    plan_id: `fplan-${Date.now()}`,
    target_path: request.target_path,
    canonical_path: request.target_path,
    scope: 'File',
    method: request.method || 'LogicalFileShred',
    passes: request.method === 'Nist80088ClearZero' ? 1 : 3,
    verification_strategy: 'MetadataUnlinkCheck',
    limitations: [
      'Logical filesystem erasure does not guarantee physical block erasure on SSDs/NVMe due to Flash Translation Layer (FTL) wear leveling.',
      'Filesystem journaling (NTFS USN / ext4 journal) may retain file name or metadata copies.',
      'Volume Shadow Copies (VSS) or automated backups may retain historical versions.',
    ],
    pre_metadata: {
      path: request.target_path,
      canonical_path: request.target_path,
      is_directory: false,
      size_bytes: 65536,
      created_at: new Date(Date.now() - 86400000).toISOString(),
      modified_at: new Date(Date.now() - 3600000).toISOString(),
      is_readonly: false,
      pre_erasure_sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08',
      snapshot_timestamp: new Date().toISOString(),
    },
    risk_level: 'High',
    created_at: new Date().toISOString(),
    scope_description: 'LOGICAL FILESYSTEM SCOPE',
  };
}

export async function planFolderErasure(
  request: PlanFolderEraseRequest
): Promise<FileErasePlanDto> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<FileErasePlanDto>('plan_folder_erasure', { request });
  }

  // Browser Mock Fallback
  await new Promise((resolve) => setTimeout(resolve, 300));
  if (request.target_path.toUpperCase().includes('WINDOWS') || request.target_path.toUpperCase() === 'C:\\') {
    throw new Error('Target directory is a system or boot critical location.');
  }

  return {
    plan_id: `dplan-${Date.now()}`,
    target_path: request.target_path,
    canonical_path: request.target_path,
    scope: 'Folder',
    method: request.method || 'DirectoryRecursiveShred',
    passes: 3,
    verification_strategy: 'MetadataUnlinkCheck',
    limitations: [
      'Recursive folder sanitization processes child files individually; FTL wear leveling applies to all underlying blocks.',
      'Directory index entries and access timestamps in filesystem metadata may remain in unallocated space.',
      'Operating system shadow copies and snapshot replicas are not purged by folder erasure.',
    ],
    pre_metadata: {
      path: request.target_path,
      canonical_path: request.target_path,
      is_directory: true,
      size_bytes: 0,
      created_at: new Date(Date.now() - 86400000).toISOString(),
      modified_at: new Date(Date.now() - 3600000).toISOString(),
      is_readonly: false,
      pre_erasure_sha256: null,
      snapshot_timestamp: new Date().toISOString(),
    },
    risk_level: 'High',
    created_at: new Date().toISOString(),
    scope_description: 'LOGICAL FILESYSTEM SCOPE',
  };
}

export async function executeFileErasure(
  request: ExecuteFileEraseRequest
): Promise<FileEraseResultDto> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<FileEraseResultDto>('execute_file_erasure', { request });
  }

  // Browser Mock Fallback
  await new Promise((resolve) => setTimeout(resolve, 1500));
  return {
    operation_id: request.operation_id,
    plan_id: request.plan_id,
    target_path: request.typed_confirmation,
    canonical_path: request.typed_confirmation,
    scope: 'File',
    method: 'LogicalFileShred',
    status: 'Completed',
    bytes_processed: 65536,
    verification: {
      outcome: 'Verified',
      strategy: 'MetadataUnlinkCheck',
      path_exists: false,
      inaccessible: true,
      details: 'Filesystem directory entry was unlinked and path is inaccessible.',
      verified_at: new Date().toISOString(),
    },
    folder_stats: null,
    failure_reason: null,
    started_at: new Date(Date.now() - 1500).toISOString(),
    completed_at: new Date().toISOString(),
    limitations: [
      'Logical filesystem erasure does not guarantee physical block erasure on SSDs/NVMe due to Flash Translation Layer (FTL) wear leveling.',
    ],
    scope_description: 'LOGICAL FILESYSTEM SCOPE',
  };
}

export async function executeFolderErasure(
  request: ExecuteFolderEraseRequest
): Promise<FileEraseResultDto> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<FileEraseResultDto>('execute_folder_erasure', { request });
  }

  // Browser Mock Fallback
  await new Promise((resolve) => setTimeout(resolve, 2000));
  return {
    operation_id: request.operation_id,
    plan_id: request.plan_id,
    target_path: request.typed_confirmation,
    canonical_path: request.typed_confirmation,
    scope: 'Folder',
    method: 'DirectoryRecursiveShred',
    status: 'Completed',
    bytes_processed: 262144,
    verification: {
      outcome: 'Verified',
      strategy: 'MetadataUnlinkCheck',
      path_exists: false,
      inaccessible: true,
      details: 'Directory tree traversed; 4 files sanitized and 2 directories removed.',
      verified_at: new Date().toISOString(),
    },
    folder_stats: {
      total_files: 4,
      files_sanitized: 4,
      files_failed: 0,
      files_cancelled: 0,
      total_directories: 2,
      directories_removed: 2,
      bytes_sanitized: 262144,
    },
    failure_reason: null,
    started_at: new Date(Date.now() - 2000).toISOString(),
    completed_at: new Date().toISOString(),
    limitations: [
      'Recursive folder sanitization processes child files individually; FTL wear leveling applies to all underlying blocks.',
    ],
    scope_description: 'LOGICAL FILESYSTEM SCOPE',
  };
}

export async function getFileErasureResult(
  operationId: string
): Promise<FileEraseResultDto | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<FileEraseResultDto | null>('get_file_erasure_result', { operationId });
  }

  return null;
}
