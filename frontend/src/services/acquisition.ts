/**
 * Forensic Acquisition Service API Client
 * Step 11: Dedicated, read-only disk acquisition & raw bitstream imaging
 */

import {
  AcquisitionArtifact,
  AcquisitionDeviceSnapshot,
  AcquisitionPlan,
  AcquisitionProgress,
  AcquisitionResult,
  ArtifactVerificationResponse,
  CreateAcquisitionPlanRequest,
  StartAcquisitionRequest,
  ValidateDestinationRequest,
  ValidateDestinationResponse,
  ValidateSourceResponse,
} from '../types/acquisition';

const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function listAcquisitionSources(): Promise<AcquisitionDeviceSnapshot[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AcquisitionDeviceSnapshot[]>('list_acquisition_sources');
  }

  // Browser Fallback (development and simulation)
  await new Promise((resolve) => setTimeout(resolve, 200));
  return [
    {
      device_id: '\\\\.\\PhysicalDrive1',
      display_name: 'Crucial CT1000MX500SSD1 1TB SATA SSD [EVIDENTIAL DISK]',
      vendor: 'Crucial',
      model: 'CT1000MX500SSD1',
      serial_number: '2104E489A123',
      media_type: 'SSD',
      capacity_bytes: 1000204886016,
      sector_size: 512,
      bus_type: 'SATA',
      is_removable: false,
      is_system: false,
      snapshot_timestamp: new Date().toISOString(),
    },
    {
      device_id: '\\\\.\\PhysicalDrive2',
      display_name: 'Kingston DataTraveler 3.0 64GB USB Flash Drive',
      vendor: 'Kingston',
      model: 'DataTraveler 3.0',
      serial_number: '001A92B60021FE',
      media_type: 'USB',
      capacity_bytes: 64000000000,
      sector_size: 512,
      bus_type: 'USB',
      is_removable: true,
      is_system: false,
      snapshot_timestamp: new Date().toISOString(),
    },
    {
      device_id: '\\\\.\\PhysicalDrive0',
      display_name: 'Samsung PM9A1 NVMe 512GB (Host System Disk)',
      vendor: 'Samsung',
      model: 'PM9A1',
      serial_number: 'S676NF0M100001',
      media_type: 'SSD',
      capacity_bytes: 512110190592,
      sector_size: 512,
      bus_type: 'NVMe',
      is_removable: false,
      is_system: true,
      snapshot_timestamp: new Date().toISOString(),
    },
  ];
}

export async function validateAcquisitionSource(
  deviceId: string
): Promise<ValidateSourceResponse> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<ValidateSourceResponse>('validate_acquisition_source', {
      deviceId,
    });
  }

  // Browser fallback
  const isLogical = /^[A-Za-z]:|^\\\\\.\\([A-Za-z]:)/.test(deviceId) || deviceId.includes('/');
  if (isLogical) {
    return {
      valid: false,
      message: `Invalid source: '${deviceId}' is a logical partition or mount path. Forensic acquisition requires a physical storage device.`,
    };
  }
  return {
    valid: true,
    message: `Physical device source '${deviceId}' is valid for read-only acquisition.`,
  };
}

export async function validateAcquisitionDestination(
  request: ValidateDestinationRequest
): Promise<ValidateDestinationResponse> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<ValidateDestinationResponse>(
      'validate_acquisition_destination',
      { request }
    );
  }

  // Browser fallback
  await new Promise((resolve) => setTimeout(resolve, 150));
  const availableBytes = 500 * 1024 * 1024 * 1024; // 500 GB mock free space
  const requiredWithMargin = request.required_capacity_bytes + 100 * 1024 * 1024;

  if (availableBytes < requiredWithMargin) {
    return {
      valid: false,
      available_free_bytes: availableBytes,
      required_bytes: requiredWithMargin,
      message: `Insufficient free disk space on destination volume. Required: ${(requiredWithMargin / (1024 ** 3)).toFixed(2)} GB, Available: ${(availableBytes / (1024 ** 3)).toFixed(2)} GB.`,
    };
  }

  return {
    valid: true,
    available_free_bytes: availableBytes,
    required_bytes: requiredWithMargin,
    message: `Destination valid: ${(availableBytes / (1024 ** 3)).toFixed(2)} GB available, ${(request.required_capacity_bytes / (1024 ** 3)).toFixed(2)} GB required.`,
  };
}

export async function createAcquisitionPlan(
  request: CreateAcquisitionPlanRequest
): Promise<AcquisitionPlan> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AcquisitionPlan>('create_acquisition_plan', { request });
  }

  // Browser fallback
  await new Promise((resolve) => setTimeout(resolve, 200));
  const sources = await listAcquisitionSources();
  const source =
    sources.find((s) => s.device_id === request.source_device_id) || sources[0];

  return {
    plan_id: `plan-acq-${Date.now()}`,
    source,
    destination_path: request.destination_path,
    image_format: 'raw',
    chunk_size_bytes: request.chunk_size_bytes || 1024 * 1024,
    allow_overwrite: !!request.allow_overwrite,
    estimated_size_bytes: source.capacity_bytes,
    created_at: new Date().toISOString(),
  };
}

export async function startAcquisition(
  request: StartAcquisitionRequest
): Promise<AcquisitionResult> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AcquisitionResult>('start_acquisition', { request });
  }

  // Browser fallback simulated execution
  await new Promise((resolve) => setTimeout(resolve, 800));
  return {
    acquisition_id: `acq-${Date.now()}`,
    operation_id: `op-acq-${Date.now()}`,
    source: request.plan.source,
    destination_path: request.plan.destination_path,
    image_format: request.plan.image_format,
    image_size_bytes: request.plan.source.capacity_bytes,
    image_sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08',
    status: 'Completed',
    bytes_acquired: request.plan.source.capacity_bytes,
    elapsed_seconds: 4.2,
    average_throughput_mbps: 238.1,
    failure_reason: null,
    audit_reference: 'AUDIT-ACQ-SIM-001',
    started_at: new Date(Date.now() - 4200).toISOString(),
    completed_at: new Date().toISOString(),
  };
}

export async function cancelAcquisition(operationId: string): Promise<boolean> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<boolean>('cancel_acquisition', { operationId });
  }
  return true;
}

export async function getAcquisitionProgress(
  operationId: string
): Promise<AcquisitionProgress | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AcquisitionProgress | null>('get_acquisition_progress', {
      operationId,
    });
  }
  return null;
}

export async function getAcquisitionResult(
  operationId: string
): Promise<AcquisitionResult | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AcquisitionResult | null>('get_acquisition_result', {
      operationId,
    });
  }
  return null;
}

export async function getAcquisitionArtifact(
  operationId: string
): Promise<AcquisitionArtifact | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AcquisitionArtifact | null>('get_acquisition_artifact', {
      operationId,
    });
  }
  return null;
}

export async function verifyAcquisitionArtifact(
  artifact: AcquisitionArtifact
): Promise<ArtifactVerificationResponse> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<ArtifactVerificationResponse>('verify_acquisition_artifact', {
      artifact,
    });
  }

  // Browser fallback
  return {
    verified: true,
    expected_hash: artifact.image_sha256,
    calculated_hash: artifact.image_sha256,
    expected_size: artifact.image_size_bytes,
    actual_size: artifact.image_size_bytes,
    error_message: null,
  };
}
