import { StorageDeviceDto } from '../types/device';

const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

const MOCK_STORAGE_DEVICES: StorageDeviceDto[] = [
  {
    device_id: '\\\\.\\PhysicalDrive0',
    display_name: 'Samsung SSD 980 PRO 1TB',
    vendor: 'Samsung',
    model: 'SSD 980 PRO 1TB',
    device_type: 'ssd',
    capacity_bytes: 1000204886016,
    removable: false,
    read_only: false,
    is_system_device: true,
    classification: 'system_device',
    classification_note: 'Device classification is not authorization.',
    volumes: [
      {
        volume_id: 'volume-c',
        mount_point: 'C:',
        label: 'Windows-OS',
        filesystem_type: 'NTFS',
        capacity_bytes: 512000000000,
        free_bytes: 230000000000,
        is_system_volume: true,
        is_boot_volume: true,
        read_only: false,
      },
      {
        volume_id: 'volume-d',
        mount_point: 'D:',
        label: 'Workstation Data',
        filesystem_type: 'NTFS',
        capacity_bytes: 488204886016,
        free_bytes: 190000000000,
        is_system_volume: false,
        is_boot_volume: false,
        read_only: false,
      },
    ],
  },
  {
    device_id: '\\\\.\\PhysicalDrive1',
    display_name: 'SanDisk Ultra USB 3.0',
    vendor: 'SanDisk',
    model: 'Ultra USB 3.0',
    device_type: 'usb',
    capacity_bytes: 64000000000,
    removable: true,
    read_only: false,
    is_system_device: false,
    classification: 'removable_device',
    classification_note: 'Device classification is not authorization.',
    volumes: [
      {
        volume_id: 'volume-e',
        mount_point: 'E:',
        label: 'EVIDENCE_USB',
        filesystem_type: 'exFAT',
        capacity_bytes: 64000000000,
        free_bytes: 48000000000,
        is_system_volume: false,
        is_boot_volume: false,
        read_only: false,
      },
    ],
  },
];

/**
 * Discovers and returns all attached physical storage devices and logical volumes.
 * Read-only safe query.
 */
export async function listStorageDevices(): Promise<StorageDeviceDto[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<StorageDeviceDto[]>('list_storage_devices');
  }

  // Fallback for standalone browser development / preview
  return [...MOCK_STORAGE_DEVICES];
}
