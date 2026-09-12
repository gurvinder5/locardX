import React, { useEffect, useState, useMemo } from 'react';
import {
  HardDrive,
  Usb,
  ShieldAlert,
  FolderInput,
  RefreshCw,
  Search,
  ChevronDown,
  ChevronRight,
  Lock,
  Layers,
  Info,
  AlertCircle,
  Database,
  Cpu,
} from 'lucide-react';
import { StorageDeviceDto, LogicalVolumeDto } from '../types/device';
import { listStorageDevices } from '../services/device';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export const DeviceExplorerPage: React.FC = () => {
  const [devices, setDevices] = useState<StorageDeviceDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedDeviceIds, setExpandedDeviceIds] = useState<Set<string>>(new Set());

  const loadDevices = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await listStorageDevices();
      setDevices(data);
      // Default to expanding all discovered devices for dense workstation readability
      setExpandedDeviceIds(new Set(data.map((d) => d.device_id)));
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Failed to discover storage devices';
      setError(msg);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadDevices();
  }, []);

  const toggleExpand = (deviceId: string) => {
    setExpandedDeviceIds((prev) => {
      const next = new Set(prev);
      if (next.has(deviceId)) {
        next.delete(deviceId);
      } else {
        next.add(deviceId);
      }
      return next;
    });
  };

  const expandAll = () => {
    setExpandedDeviceIds(new Set(devices.map((d) => d.device_id)));
  };

  const collapseAll = () => {
    setExpandedDeviceIds(new Set());
  };

  // Quick statistics
  const stats = useMemo(() => {
    const totalPhysical = devices.length;
    const totalVolumes = devices.reduce((sum, d) => sum + d.volumes.length, 0);
    const systemDevices = devices.filter((d) => d.is_system_device).length;
    const removableDevices = devices.filter(
      (d) => d.removable || d.device_type === 'usb' || d.classification === 'removable_device'
    ).length;
    return { totalPhysical, totalVolumes, systemDevices, removableDevices };
  }, [devices]);

  // Filtered devices
  const filteredDevices = useMemo(() => {
    if (!searchQuery.trim()) return devices;
    const q = searchQuery.toLowerCase();
    return devices.filter(
      (d) =>
        d.display_name.toLowerCase().includes(q) ||
        d.device_id.toLowerCase().includes(q) ||
        (d.vendor && d.vendor.toLowerCase().includes(q)) ||
        (d.model && d.model.toLowerCase().includes(q)) ||
        d.volumes.some(
          (v) =>
            (v.mount_point && v.mount_point.toLowerCase().includes(q)) ||
            (v.label && v.label.toLowerCase().includes(q)) ||
            v.filesystem_type.toLowerCase().includes(q)
        )
    );
  }, [devices, searchQuery]);

  const renderDeviceTypeBadge = (type: string) => {
    switch (type) {
      case 'ssd':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-sky-50 text-sky-700 border border-sky-200">
            <Cpu className="w-3 h-3" />
            SSD
          </span>
        );
      case 'hdd':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-slate-100 text-slate-700 border border-slate-300">
            <HardDrive className="w-3 h-3" />
            HDD
          </span>
        );
      case 'usb':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-amber-50 text-amber-700 border border-amber-200">
            <Usb className="w-3 h-3" />
            USB
          </span>
        );
      case 'memory_card':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-emerald-50 text-emerald-700 border border-emerald-200">
            <FolderInput className="w-3 h-3" />
            Card
          </span>
        );
      default:
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-slate-100 text-slate-600 border border-slate-200">
            <Database className="w-3 h-3" />
            Unknown
          </span>
        );
    }
  };

  const renderClassificationBadge = (classification: string, isSystem: boolean) => {
    if (isSystem || classification === 'system_device' || classification === 'boot_device') {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-semibold bg-rose-50 text-rose-700 border border-rose-200">
          <ShieldAlert className="w-3 h-3 text-rose-600" />
          System Device
        </span>
      );
    }
    if (classification === 'removable_device') {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-amber-50 text-amber-700 border border-amber-200">
          <Usb className="w-3 h-3 text-amber-600" />
          Removable Media
        </span>
      );
    }
    if (classification === 'external_device') {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-indigo-50 text-indigo-700 border border-indigo-200">
          <HardDrive className="w-3 h-3 text-indigo-600" />
          External Storage
        </span>
      );
    }
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-slate-100 text-slate-600 border border-slate-200">
        <HardDrive className="w-3 h-3 text-slate-500" />
        Fixed Data Device
      </span>
    );
  };

  return (
    <div className="space-y-5 text-slate-800 font-sans">
      {/* Page Header */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 border-b border-slate-200 pb-4">
        <div>
          <h1 className="text-xl font-bold tracking-tight text-slate-900 flex items-center gap-2">
            <HardDrive className="w-5 h-5 text-slate-700" />
            Device & Filesystem Explorer
          </h1>
          <p className="text-xs text-slate-500 mt-0.5">
            Read-only physical block device and logical volume hierarchy inspection.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={loadDevices}
            disabled={loading}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-slate-700 bg-white hover:bg-slate-50 active:bg-slate-100 border border-slate-300 rounded-md shadow-2xs transition-colors disabled:opacity-50"
            title="Rescan host storage devices"
          >
            <RefreshCw className={`w-3.5 h-3.5 text-slate-600 ${loading ? 'animate-spin' : ''}`} />
            <span>{loading ? 'Scanning...' : 'Refresh Devices'}</span>
          </button>
        </div>
      </div>

      {/* Safety Invariant Notice */}
      <div className="p-3 bg-amber-50/70 border border-amber-200/80 rounded-md text-amber-900 text-xs flex items-start gap-2.5">
        <Info className="w-4 h-4 text-amber-600 flex-shrink-0 mt-0.5" />
        <div className="leading-relaxed">
          <span className="font-semibold">Forensic Safety Rule:</span> Device classification is provided for situational awareness only.
          <span className="font-mono text-amber-800 ml-1">"Device classification is not authorization."</span>
          Discovery routines operate strictly in read-only query mode without modifying partition tables, filesystem structures, or raw disk sectors.
        </div>
      </div>

      {/* Statistics Banner */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <div className="bg-white border border-slate-200 rounded-md p-3 shadow-2xs">
          <div className="text-[11px] font-medium text-slate-500 uppercase tracking-wider">
            Physical Disks
          </div>
          <div className="text-lg font-bold text-slate-900 mt-0.5">{stats.totalPhysical}</div>
        </div>
        <div className="bg-white border border-slate-200 rounded-md p-3 shadow-2xs">
          <div className="text-[11px] font-medium text-slate-500 uppercase tracking-wider">
            Logical Volumes
          </div>
          <div className="text-lg font-bold text-slate-900 mt-0.5">{stats.totalVolumes}</div>
        </div>
        <div className="bg-white border border-slate-200 rounded-md p-3 shadow-2xs">
          <div className="text-[11px] font-medium text-slate-500 uppercase tracking-wider">
            System Devices
          </div>
          <div className="text-lg font-bold text-rose-600 mt-0.5">{stats.systemDevices}</div>
        </div>
        <div className="bg-white border border-slate-200 rounded-md p-3 shadow-2xs">
          <div className="text-[11px] font-medium text-slate-500 uppercase tracking-wider">
            Removable / USB
          </div>
          <div className="text-lg font-bold text-amber-600 mt-0.5">{stats.removableDevices}</div>
        </div>
      </div>

      {/* Controls Bar: Search & Collapse Toggles */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-3 bg-white p-3 border border-slate-200 rounded-md shadow-2xs">
        <div className="relative w-full sm:w-72">
          <Search className="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-2.5" />
          <input
            type="text"
            placeholder="Filter by name, mount point, fs..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-8 pr-3 py-1.5 bg-slate-50 border border-slate-200 rounded text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:border-slate-400 focus:bg-white transition-colors"
          />
        </div>

        <div className="flex items-center gap-2 text-xs">
          <button
            onClick={expandAll}
            className="px-2 py-1 text-slate-600 hover:text-slate-900 hover:bg-slate-100 rounded transition-colors"
          >
            Expand All
          </button>
          <span className="text-slate-300">|</span>
          <button
            onClick={collapseAll}
            className="px-2 py-1 text-slate-600 hover:text-slate-900 hover:bg-slate-100 rounded transition-colors"
          >
            Collapse All
          </button>
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="p-3 bg-rose-50 border border-rose-200 rounded-md text-rose-800 text-xs flex items-center gap-2">
          <AlertCircle className="w-4 h-4 text-rose-600 flex-shrink-0" />
          <span>{error}</span>
        </div>
      )}

      {/* Device Hierarchy List */}
      <div className="space-y-3">
        {loading && devices.length === 0 ? (
          <div className="bg-white border border-slate-200 rounded-md p-8 text-center text-slate-500 text-xs">
            Scanning host bus and logical storage devices...
          </div>
        ) : filteredDevices.length === 0 ? (
          <div className="bg-white border border-slate-200 rounded-md p-8 text-center text-slate-500 text-xs">
            {searchQuery ? 'No storage devices match filter criteria.' : 'No storage devices detected.'}
          </div>
        ) : (
          filteredDevices.map((device) => {
            const isExpanded = expandedDeviceIds.has(device.device_id);
            return (
              <div
                key={device.device_id}
                className="bg-white border border-slate-200 rounded-md shadow-2xs overflow-hidden transition-colors"
              >
                {/* Physical Device Header Row */}
                <div
                  onClick={() => toggleExpand(device.device_id)}
                  className="px-4 py-3 bg-slate-50/80 hover:bg-slate-100/70 border-b border-slate-200 cursor-pointer flex flex-col sm:flex-row sm:items-center justify-between gap-3 select-none transition-colors"
                >
                  <div className="flex items-center gap-3">
                    <button className="text-slate-500 hover:text-slate-700">
                      {isExpanded ? (
                        <ChevronDown className="w-4 h-4 text-slate-600" />
                      ) : (
                        <ChevronRight className="w-4 h-4 text-slate-600" />
                      )}
                    </button>

                    <div className="w-8 h-8 rounded bg-white border border-slate-200 flex items-center justify-center text-slate-700">
                      {device.device_type === 'usb' ? (
                        <Usb className="w-4 h-4 text-amber-600" />
                      ) : (
                        <HardDrive className="w-4 h-4 text-slate-700" />
                      )}
                    </div>

                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-semibold text-xs text-slate-900">
                          {device.display_name}
                        </span>
                        <span className="font-mono text-[10px] text-slate-500 bg-slate-100 px-1.5 py-0.5 rounded border border-slate-200">
                          {device.device_id}
                        </span>
                      </div>
                      <div className="text-[11px] text-slate-500 mt-0.5 flex items-center gap-2">
                        <span>Physical Capacity: {formatBytes(device.capacity_bytes)}</span>
                        <span>&bull;</span>
                        <span>{device.volumes.length} Logical Volume{device.volumes.length === 1 ? '' : 's'}</span>
                      </div>
                    </div>
                  </div>

                  <div className="flex items-center gap-2 self-end sm:self-auto">
                    {renderDeviceTypeBadge(device.device_type)}
                    {renderClassificationBadge(device.classification, device.is_system_device)}
                    {device.read_only && (
                      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-slate-100 text-slate-700 border border-slate-300">
                        <Lock className="w-3 h-3" />
                        Read-Only
                      </span>
                    )}
                  </div>
                </div>

                {/* Expanded Device Details & Volumes */}
                {isExpanded && (
                  <div className="p-4 space-y-4 bg-white">
                    {/* Hardware Metadata Summary */}
                    <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-xs bg-slate-50/60 p-2.5 rounded border border-slate-200/80">
                      <div>
                        <span className="text-slate-400 block text-[10px] uppercase font-semibold">Vendor</span>
                        <span className="font-medium text-slate-700">{device.vendor || 'Generic / Undetected'}</span>
                      </div>
                      <div>
                        <span className="text-slate-400 block text-[10px] uppercase font-semibold">Model</span>
                        <span className="font-medium text-slate-700">{device.model || 'Undetected'}</span>
                      </div>
                      <div>
                        <span className="text-slate-400 block text-[10px] uppercase font-semibold">Bus / Media</span>
                        <span className="font-medium text-slate-700">
                          {device.removable ? 'Removable Media' : 'Fixed Internal Storage'}
                        </span>
                      </div>
                      <div>
                        <span className="text-slate-400 block text-[10px] uppercase font-semibold">Raw Capacity</span>
                        <span className="font-mono text-slate-700">{device.capacity_bytes.toLocaleString()} bytes</span>
                      </div>
                    </div>

                    {/* Logical Volumes Table */}
                    <div>
                      <div className="text-[11px] font-semibold text-slate-600 uppercase tracking-wider mb-2 flex items-center gap-1.5">
                        <Layers className="w-3.5 h-3.5 text-slate-500" />
                        Logical Volumes on Physical Device
                      </div>

                      {device.volumes.length === 0 ? (
                        <div className="text-xs text-slate-500 p-3 bg-slate-50 rounded border border-slate-200">
                          No mounted filesystem volumes detected on this physical device. (Unpartitioned space or unrecognized partition table).
                        </div>
                      ) : (
                        <div className="overflow-x-auto border border-slate-200 rounded">
                          <table className="w-full text-left text-xs">
                            <thead className="bg-slate-50 text-slate-500 font-semibold border-b border-slate-200">
                              <tr>
                                <th className="px-3 py-2">Mount</th>
                                <th className="px-3 py-2">Label</th>
                                <th className="px-3 py-2">Filesystem</th>
                                <th className="px-3 py-2">Role</th>
                                <th className="px-3 py-2">Capacity</th>
                                <th className="px-3 py-2">Usage</th>
                                <th className="px-3 py-2">Free Space</th>
                              </tr>
                            </thead>
                            <tbody className="divide-y divide-slate-100">
                              {device.volumes.map((vol: LogicalVolumeDto) => {
                                const usedBytes = vol.capacity_bytes > vol.free_bytes ? vol.capacity_bytes - vol.free_bytes : 0;
                                const usedPercent = vol.capacity_bytes > 0 ? Math.round((usedBytes / vol.capacity_bytes) * 100) : 0;

                                return (
                                  <tr key={vol.volume_id} className="hover:bg-slate-50/50 transition-colors">
                                    <td className="px-3 py-2 font-mono font-bold text-slate-900">
                                      {vol.mount_point || 'Unmounted'}
                                    </td>
                                    <td className="px-3 py-2 text-slate-700">
                                      {vol.label || <span className="text-slate-400 italic">No Label</span>}
                                    </td>
                                    <td className="px-3 py-2">
                                      <span className="font-mono text-[11px] px-1.5 py-0.5 rounded bg-slate-100 text-slate-700 border border-slate-200">
                                        {vol.filesystem_type}
                                      </span>
                                    </td>
                                    <td className="px-3 py-2">
                                      {vol.is_system_volume ? (
                                        <span className="inline-flex items-center gap-1 text-[10px] font-bold text-rose-600 bg-rose-50 px-1.5 py-0.5 rounded border border-rose-200 uppercase">
                                          System Root
                                        </span>
                                      ) : (
                                        <span className="text-[10px] font-medium text-slate-500 bg-slate-100 px-1.5 py-0.5 rounded">
                                          Data
                                        </span>
                                      )}
                                    </td>
                                    <td className="px-3 py-2 font-mono text-slate-700">
                                      {formatBytes(vol.capacity_bytes)}
                                    </td>
                                    <td className="px-3 py-2 w-36">
                                      <div className="space-y-1">
                                        <div className="w-full bg-slate-100 rounded-full h-1.5 overflow-hidden">
                                          <div
                                            className={`h-1.5 rounded-full ${
                                              vol.is_system_volume
                                                ? 'bg-rose-500'
                                                : usedPercent > 85
                                                ? 'bg-amber-500'
                                                : 'bg-sky-500'
                                            }`}
                                            style={{ width: `${usedPercent}%` }}
                                          />
                                        </div>
                                        <div className="text-[10px] text-slate-400 font-mono">
                                          {usedPercent}% used
                                        </div>
                                      </div>
                                    </td>
                                    <td className="px-3 py-2 font-mono text-slate-700">
                                      {formatBytes(vol.free_bytes)}
                                    </td>
                                  </tr>
                                );
                              })}
                            </tbody>
                          </table>
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};

export default DeviceExplorerPage;
