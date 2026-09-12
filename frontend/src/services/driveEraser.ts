import {
  DriveCapabilities,
  DriveCapabilitiesAssessment,
  DriveErasePlan,
  DriveEraseResult,
  DrivePrivilegeStatus,
  DriveSanitizationReport,
  ExecuteDriveEraseSimulationRequest,
  PlanDriveEraseRequest,
} from '../types/driveEraser';
import { StorageDeviceDto } from '../types/device';

const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function getRealStorageDevices(): Promise<StorageDeviceDto[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<StorageDeviceDto[]>('get_real_storage_devices');
  }

  // Browser Fallback (Real hardware simulation)
  await new Promise((resolve) => setTimeout(resolve, 200));
  return [
    {
      device_id: '\\\\.\\PhysicalDrive0',
      display_name: 'Phison M8512GV7ECS-E152 NVMe 512GB (Host System Disk)',
      vendor: 'Phison',
      model: 'M8512GV7ECS-E152',
      device_type: 'ssd',
      capacity_bytes: 512105932800,
      removable: false,
      read_only: false,
      is_system_device: true,
      classification: 'system_device',
      classification_note: 'Active Windows OS system drive - HARD BLOCKED',
      volumes: [
        {
          volume_id: 'vol-sys-c',
          mount_point: 'C:\\',
          label: 'Windows',
          filesystem_type: 'NTFS',
          capacity_bytes: 512105932800,
          free_bytes: 250000000000,
          is_system_volume: true,
          is_boot_volume: true,
          read_only: false,
        },
      ],
    },
  ];
}

export async function refreshRealDevices(): Promise<StorageDeviceDto[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<StorageDeviceDto[]>('refresh_real_devices');
  }
  return getRealStorageDevices();
}

export async function getMockTestDevices(): Promise<StorageDeviceDto[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<StorageDeviceDto[]>('get_mock_test_devices');
  }

  // Browser Mock Fallback
  await new Promise((resolve) => setTimeout(resolve, 200));
  return [
    {
      device_id: '\\\\.\\PhysicalDrive0',
      display_name: 'Samsung PM9A1 NVMe 512GB (System Disk)',
      vendor: 'Samsung',
      model: 'PM9A1',
      device_type: 'ssd',
      capacity_bytes: 512110190592,
      removable: false,
      read_only: false,
      is_system_device: true,
      classification: 'system_device',
      classification_note: 'Active Windows OS drive - HARD BLOCKED',
      volumes: [
        {
          volume_id: 'vol-sys-0',
          mount_point: 'C:\\',
          label: 'Windows',
          filesystem_type: 'NTFS',
          capacity_bytes: 500000000000,
          free_bytes: 250000000000,
          is_system_volume: true,
          is_boot_volume: false,
          read_only: false,
        },
      ],
    },
    {
      device_id: '\\\\.\\PhysicalDrive1',
      display_name: 'Seagate Barracuda 2TB HDD',
      vendor: 'Seagate',
      model: 'ST2000DM008',
      device_type: 'hdd',
      capacity_bytes: 2000398934016,
      removable: false,
      read_only: false,
      is_system_device: false,
      classification: 'fixed_data_device',
      classification_note: 'Rotational magnetic media',
      volumes: [
        {
          volume_id: 'vol-data-1',
          mount_point: 'D:\\',
          label: 'DataStorage',
          filesystem_type: 'NTFS',
          capacity_bytes: 2000000000000,
          free_bytes: 1200000000000,
          is_system_volume: false,
          is_boot_volume: false,
          read_only: false,
        },
      ],
    },
    {
      device_id: '\\\\.\\PhysicalDrive2',
      display_name: 'Crucial MX500 1TB SATA SSD',
      vendor: 'Crucial',
      model: 'CT1000MX500SSD1',
      device_type: 'ssd',
      capacity_bytes: 1000204886016,
      removable: false,
      read_only: false,
      is_system_device: false,
      classification: 'fixed_data_device',
      classification_note: 'SATA Solid State Drive with ATA Secure Erase support',
      volumes: [],
    },
    {
      device_id: '\\\\.\\PhysicalDrive3',
      display_name: 'Samsung 980 PRO NVMe 1TB',
      vendor: 'Samsung',
      model: '980 PRO',
      device_type: 'ssd',
      capacity_bytes: 1000204886016,
      removable: false,
      read_only: false,
      is_system_device: false,
      classification: 'fixed_data_device',
      classification_note: 'PCIe NVMe SSD with Cryptographic Sanitize support',
      volumes: [],
    },
    {
      device_id: '\\\\.\\PhysicalDrive4',
      display_name: 'SanDisk Ultra USB 3.0 64GB',
      vendor: 'SanDisk',
      model: 'Ultra',
      device_type: 'usb',
      capacity_bytes: 64000000000,
      removable: true,
      read_only: false,
      is_system_device: false,
      classification: 'removable_device',
      classification_note: 'Removable USB Flash Memory',
      volumes: [],
    },
    {
      device_id: '\\\\.\\PhysicalDrive5',
      display_name: 'Kingston Canvas SD Card 32GB',
      vendor: 'Kingston',
      model: 'Canvas Select',
      device_type: 'memory_card',
      capacity_bytes: 32000000000,
      removable: true,
      read_only: false,
      is_system_device: false,
      classification: 'removable_device',
      classification_note: 'Flash Memory Card (SD/microSD)',
      volumes: [],
    },
    {
      device_id: '\\\\.\\PhysicalDrive6',
      display_name: 'UEFI System Boot Media 16GB',
      vendor: 'Generic',
      model: 'BootStorage',
      device_type: 'usb',
      capacity_bytes: 16000000000,
      removable: true,
      read_only: false,
      is_system_device: false,
      classification: 'boot_device',
      classification_note: 'Active UEFI ESP Boot device - HARD BLOCKED',
      volumes: [],
    },
  ];
}

export async function getDriveCapabilities(
  targetDeviceId: string
): Promise<DriveCapabilities> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<DriveCapabilities>('get_drive_capabilities', {
      targetDeviceId,
    });
  }

  // Browser Mock Fallback
  await new Promise((resolve) => setTimeout(resolve, 250));
  const isNvme = targetDeviceId.includes('3');
  const isSsd = targetDeviceId.includes('2');
  const isUsb = targetDeviceId.includes('4') || targetDeviceId.includes('5');

  if (isNvme) {
    return {
      supported_capabilities: [
        'NvmeSanitize',
        'NvmeCryptoErase',
        'NvmeFormat',
        'SequentialWrite',
        'FullDeviceRead',
        'SectorAccess',
      ],
      interface_bus: 'NVMe',
      sector_size: 4096,
      is_rotational: false,
      supports_crypto_erase: true,
      supports_firmware_sanitize: true,
      supports_overwrite: true,
    };
  } else if (isSsd) {
    return {
      supported_capabilities: [
        'AtaSecureErase',
        'AtaSanitize',
        'SequentialWrite',
        'FullDeviceRead',
        'SectorAccess',
      ],
      interface_bus: 'SATA',
      sector_size: 512,
      is_rotational: false,
      supports_crypto_erase: false,
      supports_firmware_sanitize: true,
      supports_overwrite: true,
    };
  } else if (isUsb) {
    return {
      supported_capabilities: [
        'SequentialWrite',
        'FullDeviceRead',
        'SectorAccess',
        'RemovableMedia',
      ],
      interface_bus: 'USB',
      sector_size: 512,
      is_rotational: false,
      supports_crypto_erase: false,
      supports_firmware_sanitize: false,
      supports_overwrite: true,
    };
  } else {
    return {
      supported_capabilities: [
        'SequentialWrite',
        'FullDeviceRead',
        'SectorAccess',
      ],
      interface_bus: 'SATA',
      sector_size: 512,
      is_rotational: true,
      supports_crypto_erase: false,
      supports_firmware_sanitize: false,
      supports_overwrite: true,
    };
  }
}

export async function assessDriveHardwareCapabilities(
  targetDeviceId: string
): Promise<DriveCapabilitiesAssessment> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<DriveCapabilitiesAssessment>('assess_drive_hardware_capabilities', {
      targetDeviceId,
    });
  }

  // Browser Mock Fallback
  const caps = await getDriveCapabilities(targetDeviceId);
  const isNvme = targetDeviceId.includes('3') || targetDeviceId.includes('0');
  const isSsd = targetDeviceId.includes('2');

  return {
    overall_state: 'Supported',
    capabilities: caps,
    capability_states: {
      SequentialWrite: 'Supported',
      FullDeviceRead: 'Supported',
      SectorAccess: 'Supported',
      AtaSecureErase: isSsd ? 'Supported' : 'Unsupported',
      AtaSanitize: isSsd ? 'Supported' : 'Unsupported',
      NvmeFormat: isNvme ? 'Supported' : 'Unsupported',
      NvmeSanitize: isNvme ? 'Supported' : 'Unsupported',
      NvmeCryptoErase: isNvme ? 'Supported' : 'Unsupported',
      RemovableMedia: caps.supported_capabilities.includes('RemovableMedia')
        ? 'Supported'
        : 'Unsupported',
    },
    method_support: {
      Nist80088ClearZero: 'Supported',
      Dod522022M: caps.is_rotational ? 'Supported' : 'Unsupported',
      AtaSecureErase: isSsd ? 'Supported' : 'Unsupported',
      NvmeFormatSanitize: isNvme ? 'Supported' : 'Unsupported',
      NvmeCryptoErase: isNvme ? 'Supported' : 'Unsupported',
      BlockZeroOverwrite: 'Supported',
    },
    assessment_notes: [
      `Hardware capability assessment completed for ${targetDeviceId}`,
      `Bus: ${caps.interface_bus} | Sector: ${caps.sector_size}B`,
    ],
  };
}

export async function planDriveErasure(
  request: PlanDriveEraseRequest
): Promise<DriveErasePlan> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<DriveErasePlan>('plan_drive_erasure', { request });
  }

  // Browser Mock Fallback
  await new Promise((resolve) => setTimeout(resolve, 300));
  if (request.target_device_id.includes('0') || request.target_device_id.includes('6')) {
    throw new Error('Target device is an active System or Boot Device. Erasure is unconditionally blocked.');
  }

  const isNvme = request.target_device_id.includes('3');
  const isSsd = request.target_device_id.includes('2');

  const method = isNvme
    ? 'NvmeCryptoErase'
    : isSsd
    ? 'AtaSecureErase'
    : 'Nist80088ClearZero';

  return {
    plan_id: `dplan-${Date.now()}`,
    physical_device_id: request.target_device_id,
    display_name: `Storage Device (${request.target_device_id})`,
    vendor: isNvme ? 'Samsung' : isSsd ? 'Crucial' : 'Seagate',
    model: isNvme ? '980 PRO' : isSsd ? 'MX500' : 'Barracuda',
    serial_number: 'TEST-SERIAL-MOCK-999',
    media_type: isNvme || isSsd ? 'Ssd' : 'Hdd',
    capacity_bytes: isNvme || isSsd ? 1000204886016 : 2000398934016,
    sector_size: isNvme ? 4096 : 512,
    method,
    passes: 1,
    execution_mode: 'Simulation',
    verification_plan: {
      strategy: isNvme ? 'CryptoKeyDestructionCheck' : isSsd ? 'FirmwareStatusVerify' : 'FullDeviceReadVerify',
      sample_percentage: isNvme || isSsd ? null : 100.0,
      requirements: isNvme
        ? 'Verify cryptographic media key destruction via NVMe controller log'
        : isSsd
        ? 'Verify ATA command completion register and IDENTIFY DEVICE security status'
        : 'Sequential readback of all logical block addresses verifying 0x00 null bytes',
      limitations: [
        'Simulation verification does not query real flash memory cells in Step 10A.',
      ],
    },
    limitations: [
      'SIMULATION MODE: Storage hardware is not written to or modified.',
      'Protected HPA/DCO sectors remain physically untouched in simulation.',
    ],
    risk_level: 'High',
    device_snapshot: {
      device_id: request.target_device_id,
      display_name: `Storage Device (${request.target_device_id})`,
      vendor: isNvme ? 'Samsung' : isSsd ? 'Crucial' : 'Seagate',
      model: isNvme ? '980 PRO' : isSsd ? 'MX500' : 'Barracuda',
      serial_number: 'TEST-SERIAL-MOCK-999',
      media_type: isNvme || isSsd ? 'Ssd' : 'Hdd',
      capacity_bytes: isNvme || isSsd ? 1000204886016 : 2000398934016,
      sector_size: isNvme ? 4096 : 512,
      is_system: false,
      is_boot: false,
      is_removable: false,
      classification: 'FixedDataDevice',
      partition_count: 1,
      volume_labels: ['DataStorage'],
      snapshot_timestamp: new Date().toISOString(),
    },
    created_at: new Date().toISOString(),
  };
}

export async function executeDriveErasureSimulation(
  request: ExecuteDriveEraseSimulationRequest
): Promise<DriveEraseResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<DriveEraseResult>('execute_drive_erasure_simulation', {
      request,
    });
  }

  // Browser Mock Fallback
  await new Promise((resolve) => setTimeout(resolve, 800));

  if (!request.warning_acknowledged) {
    throw new Error('Destructive consequences warning must be explicitly acknowledged.');
  }

  return {
    operation_id: request.operation_id,
    plan_id: request.plan_id,
    physical_device_id: request.typed_confirmation,
    display_name: `Simulated Drive (${request.typed_confirmation})`,
    vendor: 'MockVendor',
    model: 'MockModel',
    serial_number: 'TEST-SERIAL-MOCK-999',
    media_type: 'Hdd',
    capacity_bytes: 1000000000,
    sector_size: 512,
    method: 'Nist80088ClearZero',
    execution_mode: 'Simulation',
    status: 'Completed',
    bytes_processed: 1000000000,
    elapsed_seconds: 0.85,
    verification: {
      outcome: 'Verified',
      strategy: 'FullDeviceReadVerify',
      details: 'Simulation verification confirmed all simulated blocks nullified (0x00)',
      verified_at: new Date().toISOString(),
    },
    failure_reason: null,
    audit_references: [
      'DRIVE_ERASURE_SIMULATION_STARTED',
      'DRIVE_ERASURE_SIMULATION_COMPLETED',
      'DRIVE_ERASURE_VERIFICATION_COMPLETED',
    ],
    started_at: new Date(Date.now() - 1000).toISOString(),
    completed_at: new Date().toISOString(),
    limitations: [
      'SIMULATION MODE: Storage hardware is not written to or modified.',
    ],
  };
}

export async function executeDriveErasureHardware(
  request: ExecuteDriveEraseSimulationRequest
): Promise<DriveEraseResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<DriveEraseResult>('execute_drive_erasure_hardware', {
      request,
    });
  }

  throw new Error('Real hardware execution requires running inside the LocardX native desktop application.');
}

export async function getDriveErasureResult(
  operationId: string
): Promise<DriveEraseResult | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<DriveEraseResult | null>('get_drive_erasure_result', {
      operationId,
    });
  }

  return null;
}

export async function checkDriveEraserPrivileges(): Promise<DrivePrivilegeStatus> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<DrivePrivilegeStatus>('check_drive_eraser_privileges');
  }

  return {
    is_elevated: false,
    platform: 'web',
    message: 'Browser environment: Hardware privilege probe not applicable; simulation mode active.',
  };
}

export async function generateDriveErasureReport(
  operationId: string,
  sessionToken?: string
): Promise<DriveSanitizationReport> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<DriveSanitizationReport>('generate_drive_erasure_report', {
      operationId,
      sessionToken,
    });
  }

  // Web fallback report
  return {
    report_id: `rep-mock-${Date.now()}`,
    operation_id: operationId,
    plan_id: `dplan-mock-${Date.now()}`,
    actor_id: 'web_operator',
    device_info: {
      physical_device_id: '\\\\.\\PhysicalDrive1',
      display_name: 'Simulated Drive',
      vendor: 'LocardX',
      model: 'SimDisk-1',
      serial_number: 'SIM-001-TEST',
      media_type: 'Hdd',
      capacity_bytes: 500107862016,
      sector_size: 512,
      bus_type: 'SATA',
    },
    operation_info: {
      sanitization_method: 'Nist80088ClearZero',
      execution_mode: 'Simulation',
      is_simulation: true,
      started_at: new Date(Date.now() - 5000).toISOString(),
      completed_at: new Date().toISOString(),
      elapsed_seconds: 5.0,
    },
    execution_metrics: {
      status: 'Completed',
      bytes_processed: 500107862016,
      total_bytes: 500107862016,
      failure_reason: null,
      limitations: ['Simulation mode dry-run only.'],
    },
    verification: {
      strategy: 'FullDeviceReadVerify',
      outcome: 'Verified',
      details: 'Simulation readback verification confirmed all sectors cleared.',
      evidence_digest: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
      verified_at: new Date().toISOString(),
    },
    integrity: {
      audit_chain_reference: 'MOCK_AUDIT_REF',
      report_digest: '4a44dc15364204a80fe80e9039455ec16e32d295e24e22207b415a7702580c85',
      generated_at: new Date().toISOString(),
    },
  };
}

export async function getDriveErasureReport(
  operationId: string
): Promise<DriveSanitizationReport | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<DriveSanitizationReport | null>('get_drive_erasure_report', {
      operationId,
    });
  }

  return null;
}

