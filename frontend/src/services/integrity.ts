import { HashResult, IntegrityRecord, VerificationResult } from '../types/integrity';

const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// In-memory mock records for standalone browser development
const MOCK_RECORDS: IntegrityRecord[] = [
  {
    record_id: 'rec-mock-001',
    operation_id: 'op-mock-001',
    target_type: 'File',
    target_path: 'C:\\Evidence\\disk_image_01.raw',
    target_name: 'disk_image_01.raw',
    algorithm: 'Sha256',
    digest: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
    bytes_processed: 104857600,
    verification_status: 'Verified',
    expected_digest: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
    actor_id: 'admin',
    created_at: new Date(Date.now() - 3600000).toISOString(),
  },
  {
    record_id: 'rec-mock-002',
    operation_id: 'op-mock-002',
    target_type: 'File',
    target_path: 'D:\\Cases\\case_892_payload.bin',
    target_name: 'case_892_payload.bin',
    algorithm: 'Sha256',
    digest: 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',
    bytes_processed: 2097152,
    verification_status: 'Calculated',
    expected_digest: null,
    actor_id: 'admin',
    created_at: new Date(Date.now() - 1800000).toISOString(),
  },
];

/**
 * Computes the cryptographic hash of a file on the local host.
 * Strictly read-only operation.
 */
export async function calculateFileHash(
  path: string,
  sessionToken?: string
): Promise<HashResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<HashResult>('calculate_file_hash', {
      path,
      sessionToken: sessionToken || null,
    });
  }

  // Browser mock fallback
  const mockDigest = 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad';
  const fileName = path.split(/[\/\\]/).pop() || 'target_file.bin';
  const result: HashResult = {
    target: {
      target_type: 'File',
      identifier: path,
      display_name: fileName,
      size_bytes: 1048576,
    },
    algorithm: 'Sha256',
    digest: mockDigest,
    bytes_processed: 1048576,
    started_at: new Date().toISOString(),
    completed_at: new Date().toISOString(),
    duration_ms: 12,
    status: 'Success',
  };

  MOCK_RECORDS.unshift({
    record_id: `rec-mock-${Date.now()}`,
    operation_id: `op-mock-${Date.now()}`,
    target_type: 'File',
    target_path: path,
    target_name: fileName,
    algorithm: 'Sha256',
    digest: mockDigest,
    bytes_processed: 1048576,
    verification_status: 'Calculated',
    expected_digest: null,
    actor_id: 'browser_dev',
    created_at: new Date().toISOString(),
  });

  return result;
}

/**
 * Verifies the cryptographic hash of a target file against an expected digest.
 * Strictly read-only operation.
 */
export async function verifyFileHash(
  path: string,
  expectedDigest: string,
  sessionToken?: string
): Promise<VerificationResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<VerificationResult>('verify_file_hash', {
      path,
      expectedDigest,
      sessionToken: sessionToken || null,
    });
  }

  // Browser mock fallback
  const cleaned = expectedDigest.trim().toLowerCase();
  const fileName = path.split(/[\/\\]/).pop() || 'target_file.bin';
  const mockCalc = 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad';

  const isMatch = cleaned === mockCalc;
  const status = isMatch
    ? ('Verified' as const)
    : { Mismatch: { expected: cleaned, calculated: mockCalc } };

  const result: VerificationResult = {
    target: {
      target_type: 'File',
      identifier: path,
      display_name: fileName,
      size_bytes: 1048576,
    },
    algorithm: 'Sha256',
    expected_digest: cleaned,
    calculated_digest: mockCalc,
    bytes_processed: 1048576,
    status,
    timestamp: new Date().toISOString(),
    duration_ms: 14,
  };

  MOCK_RECORDS.unshift({
    record_id: `rec-mock-${Date.now()}`,
    operation_id: `op-mock-${Date.now()}`,
    target_type: 'File',
    target_path: path,
    target_name: fileName,
    algorithm: 'Sha256',
    digest: mockCalc,
    bytes_processed: 1048576,
    verification_status: isMatch ? 'Verified' : 'Mismatch',
    expected_digest: cleaned,
    actor_id: 'browser_dev',
    created_at: new Date().toISOString(),
  });

  return result;
}

/**
 * Retrieves the history of cryptographic integrity operations.
 */
export async function listIntegrityRecords(limit = 50): Promise<IntegrityRecord[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<IntegrityRecord[]>('list_integrity_records', { limit });
  }

  return [...MOCK_RECORDS].slice(0, limit);
}
