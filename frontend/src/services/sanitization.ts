import {
  EvaluatePlanRequest,
  SanitizationPlanDto,
  SanitizationStandardDto,
  SnapshotComparisonResultDto,
  VerificationPlanDto,
} from '../types/sanitization';

function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

const MOCK_STANDARDS: SanitizationStandardDto[] = [
  {
    standard_id: 'NIST_SP_800_88_REV1_CLEAR',
    standard_name: 'NIST SP 800-88 Rev. 1 (Clear)',
    method_id: 'nist_800_88_clear_zero',
    method_name: 'Single-Pass Overwrite (Logical Zero)',
    applicable_scopes: ['LogicalVolume', 'PhysicalDevice'],
    applicable_media: ['HDD (Magnetic Rotational)', 'USB / Removable Flash', 'External Storage Enclosure'],
    recommended_verification: 'Sampled Pseudo-Random Sectors (10%)',
    verification_requirements: 'Sequential sample verification across 10% of logical block addresses confirming zero bytes.',
    limitations: [
      'Flash Translation Layer (FTL) wear leveling prevents guaranteed software overwrite.',
      'Reallocated bad sectors are hidden by the disk controller.',
      'Host OS buffer cache may report writes before physical commit.',
    ],
    description: 'Overwrites user-addressable storage space with a single pass of fixed zero bytes to prevent simple non-invasive recovery.',
  },
  {
    standard_id: 'NIST_SP_800_88_REV1_PURGE',
    standard_name: 'NIST SP 800-88 Rev. 1 (Purge)',
    method_id: 'nist_800_88_purge_crypto',
    method_name: 'Cryptographic Erase (Purge)',
    applicable_scopes: ['PhysicalDevice'],
    applicable_media: ['SSD (Solid State Flash)', 'NVMe SSD (High Performance Flash)'],
    recommended_verification: 'Cryptographic Key Destruction Verification',
    verification_requirements: 'Verify controller cryptographic master key generation and invalidation of previous encryption key.',
    limitations: [
      'Firmware cryptographic erasure depends on drive controller hardware compliance.',
      'Only applicable if the drive was continuously hardware-encrypted using AES before erasure.',
    ],
    description: 'Leverages drive hardware controller encryption to cryptographically discard the Media Encryption Key (MEK).',
  },
  {
    standard_id: 'DOD_5220_22_M',
    standard_name: 'DoD 5220.22-M (NISPOM)',
    method_id: 'dod_5220_22_m_3pass',
    method_name: '3-Pass Overwrite (0x00, 0xFF, Pseudo-Random)',
    applicable_scopes: ['LogicalVolume', 'PhysicalDevice'],
    applicable_media: ['HDD (Magnetic Rotational)', 'External Storage Enclosure'],
    recommended_verification: 'Full Sequential Read-Back (100% Sectors)',
    verification_requirements: '100% full sequential read-back verifying all sectors match the final pseudo-random character mask.',
    limitations: [
      'Hardware-reallocated bad sectors cannot be verified from host user space.',
      'Not recommended for SSDs due to high write amplification and FTL wear-leveling remanence.',
    ],
    description: 'Three-pass overwrite using a fixed character, its complement, and pseudo-random bytes followed by verification.',
  },
  {
    standard_id: 'NVME_SPEC_CRYPTO',
    standard_name: 'NVM Express Base Specification (Crypto Erase)',
    method_id: 'nvme_format_crypto',
    method_name: 'NVMe Format Cryptographic Erase',
    applicable_scopes: ['PhysicalDevice'],
    applicable_media: ['NVMe SSD (High Performance Flash)'],
    recommended_verification: 'Cryptographic Key Destruction Verification',
    verification_requirements: 'Validate completion of NVMe Format command with Cryptographic Erase (SES=2) bit set.',
    limitations: [
      'Target drive must support NVMe Cryptographic Erase command in controller capabilities.',
    ],
    description: 'Issues low-level NVMe Format command requesting controller-level cryptographic key regeneration.',
  },
  {
    standard_id: 'LOGICAL_FILE_SHRED',
    standard_name: 'LocardX Forensic File Sanitization Standard',
    method_id: 'logical_file_shred_unlink',
    method_name: 'Logical File Payload Overwrite & Unlink',
    applicable_scopes: ['File', 'Folder'],
    applicable_media: ['HDD (Magnetic Rotational)', 'SSD (Solid State Flash)', 'NVMe SSD (High Performance Flash)', 'USB / Removable Flash'],
    recommended_verification: 'Filesystem Metadata & Unlink Verification',
    verification_requirements: 'Verify file cluster zeroing, truncation to 0 bytes, and directory entry unlinking.',
    limitations: [
      'Filesystem journaling (NTFS $LogFile) may retain metadata fragments outside the file boundary.',
      'Volume Shadow Copies (VSS) may preserve historical file contents unless explicitly purged.',
      'Flash wear leveling may retain physical remnants outside the logical cluster map.',
    ],
    description: 'Overwrites file contents with pseudo-random and zero patterns, flushes caches, truncates to 0 bytes, and unlinks from the filesystem.',
  },
];

export async function evaluateSanitizationPlan(
  req: EvaluatePlanRequest
): Promise<SanitizationPlanDto> {
  if (!isTauriEnvironment()) {
    const isSys =
      req.target_identifier.toUpperCase().includes('C:') ||
      req.target_identifier === '/';
    return {
      plan_id: `mock-plan-${Date.now()}`,
      target_identifier: req.target_identifier,
      target_type: req.target_type,
      media_type: req.target_identifier.toLowerCase().includes('nvme')
        ? 'NVMe SSD (High Performance Flash)'
        : req.target_identifier.toLowerCase().includes('usb')
        ? 'USB / Removable Flash'
        : 'SSD (Solid State Flash)',
      scope: req.scope,
      recommended_method: isSys
        ? 'Unsupported Method'
        : req.scope === 'File' || req.scope === 'Folder'
        ? 'Logical File Shred & Unlink'
        : req.target_identifier.toLowerCase().includes('nvme')
        ? 'NVMe Cryptographic Erase (Format Command)'
        : 'NIST SP 800-88 Clear (Single-Pass Zero)',
      is_applicable: !isSys,
      risk_level: isSys ? 'Critical' : 'High',
      verification_strategy: isSys
        ? 'No Post-Sanitization Verification'
        : req.scope === 'File'
        ? 'Filesystem Metadata & Unlink Verification'
        : 'Sampled Pseudo-Random Sectors (10%)',
      applicable_standard: isSys
        ? null
        : req.scope === 'File'
        ? 'LocardX Forensic File Sanitization Standard'
        : 'NIST SP 800-88 Rev. 1 (Clear)',
      standard_method_id: isSys ? null : 'nist_800_88_clear_zero',
      limitations: isSys
        ? ['Active system disk is hard-blocked from sanitization by software safety policy.']
        : [
            'Flash Translation Layer (FTL) wear leveling prevents guaranteed software overwrite of retired NAND blocks.',
            'Host write caching may acknowledge writes before physical commit.',
          ],
      reason_codes: isSys ? ['SYSTEM_DEVICE'] : ['VALID_TARGET'],
      created_at: new Date().toISOString(),
      is_dry_run: true,
    };
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return await invoke<SanitizationPlanDto>('evaluate_sanitization_plan', {
    request: req,
  });
}

export async function getSanitizationMethods(): Promise<SanitizationStandardDto[]> {
  if (!isTauriEnvironment()) {
    return MOCK_STANDARDS;
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return await invoke<SanitizationStandardDto[]>('get_sanitization_methods');
}

export async function compareTargetSnapshot(
  planId: string
): Promise<SnapshotComparisonResultDto> {
  if (!isTauriEnvironment()) {
    return {
      matches: true,
      reason: null,
      differences: [],
      plan_target_identifier: '\\\\.\\PhysicalDrive1',
      current_target_identifier: '\\\\.\\PhysicalDrive1',
    };
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return await invoke<SnapshotComparisonResultDto>('compare_target_snapshot', {
    planId,
  });
}

export async function getSanitizationVerificationPlan(
  planId: string
): Promise<VerificationPlanDto> {
  if (!isTauriEnvironment()) {
    return {
      plan_id: planId,
      target_identifier: '\\\\.\\PhysicalDrive1',
      verification_strategy: 'Sampled Pseudo-Random Sectors (10%)',
      verification_requirements: 'Sequential sample verification across 10% of logical block addresses confirming zero bytes.',
      limitations: [
        'Flash Translation Layer (FTL) wear leveling prevents guaranteed software overwrite.',
      ],
      applicable_standard: 'NIST SP 800-88 Rev. 1 (Clear)',
    };
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return await invoke<VerificationPlanDto>('get_sanitization_verification_plan', {
    planId,
  });
}
