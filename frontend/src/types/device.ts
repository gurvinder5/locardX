export type DeviceType =
  | 'hdd'
  | 'ssd'
  | 'usb'
  | 'memory_card'
  | 'external_storage'
  | 'unknown';

export type DeviceClassification =
  | 'system_device'
  | 'boot_device'
  | 'removable_device'
  | 'external_device'
  | 'fixed_data_device'
  | 'unknown';

export interface LogicalVolumeDto {
  volume_id: string;
  mount_point: string | null;
  label: string | null;
  filesystem_type: string;
  capacity_bytes: number;
  free_bytes: number;
  is_system_volume: boolean;
  is_boot_volume: boolean;
  read_only: boolean;
}

export interface StorageDeviceDto {
  device_id: string;
  display_name: string;
  vendor: string | null;
  model: string | null;
  device_type: DeviceType;
  capacity_bytes: number;
  removable: boolean;
  read_only: boolean;
  is_system_device: boolean;
  classification: DeviceClassification;
  classification_note: string;
  volumes: LogicalVolumeDto[];
}
