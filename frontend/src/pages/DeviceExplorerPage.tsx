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
  Database,
  Cpu,
  Info,
} from 'lucide-react';
import {
  Button,
  Input,
  Chip,
  Progress,
  Table,
  TableHeader,
  TableColumn,
  TableBody,
  TableRow,
  TableCell,
} from '@heroui/react';
import { StorageDeviceDto } from '../types/device';
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

  const stats = useMemo(() => {
    const totalPhysical = devices.length;
    const totalVolumes = devices.reduce((sum, d) => sum + d.volumes.length, 0);
    const systemDevices = devices.filter((d) => d.is_system_device).length;
    const removableDevices = devices.filter(
      (d) => d.removable || d.device_type === 'usb' || d.classification === 'removable_device'
    ).length;
    return { totalPhysical, totalVolumes, systemDevices, removableDevices };
  }, [devices]);

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
          <Chip size="sm" variant="flat" startContent={<Cpu className="w-3 h-3" />} className="bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-[#26313D] text-[10px]">
            SSD
          </Chip>
        );
      case 'hdd':
        return (
          <Chip size="sm" variant="flat" startContent={<HardDrive className="w-3 h-3" />} className="bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-[#26313D] text-[10px]">
            HDD
          </Chip>
        );
      case 'usb':
        return (
          <Chip size="sm" variant="flat" startContent={<Usb className="w-3 h-3" />} className="bg-amber-100 dark:bg-amber-950/40 text-amber-800 dark:text-amber-300 border border-amber-200 dark:border-amber-800 text-[10px]">
            USB
          </Chip>
        );
      case 'memory_card':
        return (
          <Chip size="sm" variant="flat" startContent={<FolderInput className="w-3 h-3" />} className="bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-[#26313D] text-[10px]">
            CARD
          </Chip>
        );
      default:
        return (
          <Chip size="sm" variant="flat" startContent={<Database className="w-3 h-3" />} className="bg-slate-100 dark:bg-[#1B2530] text-slate-600 dark:text-slate-400 border border-slate-200 dark:border-[#26313D] text-[10px]">
            UNKNOWN
          </Chip>
        );
    }
  };

  const renderClassificationBadge = (classification: string, isSystem: boolean) => {
    if (isSystem || classification === 'system_device' || classification === 'boot_device') {
      return (
        <Chip size="sm" variant="flat" startContent={<ShieldAlert className="w-3 h-3 text-rose-600" />} className="bg-rose-50 dark:bg-rose-950/40 text-rose-700 dark:text-rose-300 border border-rose-200 dark:border-rose-900 font-bold text-[10px]">
          PROTECTED (SYSTEM)
        </Chip>
      );
    }
    if (classification === 'removable_device') {
      return (
        <Chip size="sm" variant="flat" startContent={<Usb className="w-3 h-3 text-amber-600" />} className="bg-amber-50 dark:bg-amber-950/40 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800 font-semibold text-[10px]">
          REMOVABLE MEDIA
        </Chip>
      );
    }
    if (classification === 'external_device') {
      return (
        <Chip size="sm" variant="flat" startContent={<HardDrive className="w-3 h-3 text-slate-500" />} className="bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-[#26313D] text-[10px]">
          EXTERNAL STORAGE
        </Chip>
      );
    }
    return (
      <Chip size="sm" variant="flat" startContent={<HardDrive className="w-3 h-3 text-slate-500" />} className="bg-slate-100 dark:bg-[#1B2530] text-slate-600 dark:text-slate-400 border border-slate-200 dark:border-[#26313D] text-[10px]">
        FIXED STORAGE
      </Chip>
    );
  };

  return (
    <div className="space-y-6 text-slate-800 dark:text-slate-200 font-sans pb-10">
      {/* 1. Page Heading */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 border-b border-slate-200 dark:border-[#26313D] pb-4">
        <div>
          <h1 className="text-xl font-bold tracking-tight text-slate-900 dark:text-slate-100 flex items-center gap-2">
            <HardDrive className="w-5 h-5 text-slate-700 dark:text-slate-300" />
            Device & Filesystem Explorer
          </h1>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
            Hierarchical physical storage devices, partition schemes, hardware telemetry, and mounted logical volume inspection.
          </p>
        </div>

        <Button
          size="sm"
          variant="flat"
          onPress={loadDevices}
          isLoading={loading}
          startContent={!loading && <RefreshCw className="w-3.5 h-3.5" />}
          className="border border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300 font-semibold h-8 bg-slate-100 dark:bg-[#1B2530]"
        >
          {loading ? 'Scanning...' : 'Rescan Devices'}
        </Button>
      </div>

      {/* 2. Safety Invariant Strip (No Card) */}
      <div className="border-l-2 border-amber-500 bg-amber-500/10 px-3.5 py-2.5 text-xs text-amber-900 dark:text-amber-200 flex items-start gap-2.5">
        <Info className="w-4 h-4 text-amber-600 dark:text-amber-400 shrink-0 mt-0.5" />
        <div className="leading-relaxed">
          <span className="font-semibold">Safety Boundary:</span> Storage enumeration executes strictly in non-invasive, read-only mode. Device classification is advisory and does not grant operational sanitization authorization without safety invariant validation.
        </div>
      </div>

      {/* 3. Inline Metrics Ribbon (No Cards) */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 py-3 border-y border-slate-200 dark:border-[#26313D] text-xs">
        <div>
          <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
            Physical Disks
          </span>
          <span className="text-lg font-bold font-mono text-slate-900 dark:text-slate-100 mt-0.5 block">
            {stats.totalPhysical}
          </span>
          <span className="text-[11px] text-slate-500">Enumerated block targets</span>
        </div>

        <div className="sm:border-l sm:border-slate-200 sm:dark:border-[#26313D] sm:pl-4">
          <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
            Logical Volumes
          </span>
          <span className="text-lg font-bold font-mono text-slate-900 dark:text-slate-100 mt-0.5 block">
            {stats.totalVolumes}
          </span>
          <span className="text-[11px] text-slate-500">Mounted / mapped partitions</span>
        </div>

        <div className="sm:border-l sm:border-slate-200 sm:dark:border-[#26313D] sm:pl-4">
          <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
            Protected System Devices
          </span>
          <span className="text-lg font-bold font-mono text-rose-600 dark:text-rose-400 mt-0.5 block">
            {stats.systemDevices}
          </span>
          <span className="text-[11px] text-slate-500">Boot & OS hard-interlocked</span>
        </div>

        <div className="sm:border-l sm:border-slate-200 sm:dark:border-[#26313D] sm:pl-4">
          <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
            Removable Targets
          </span>
          <span className="text-lg font-bold font-mono text-slate-900 dark:text-slate-100 mt-0.5 block">
            {stats.removableDevices}
          </span>
          <span className="text-[11px] text-slate-500">USB flash & external media</span>
        </div>
      </div>

      {/* 4. Controls Toolbar */}
      <div className="flex flex-col sm:flex-row items-center justify-between gap-3 pt-1">
        <Input
          size="sm"
          placeholder="Filter by device ID, vendor, model, volume or filesystem..."
          value={searchQuery}
          onValueChange={setSearchQuery}
          startContent={<Search className="w-3.5 h-3.5 text-slate-400" />}
          className="max-w-md w-full text-xs"
          classNames={{
            inputWrapper: 'bg-white dark:bg-[#151C24] border border-slate-300 dark:border-[#26313D] h-8',
          }}
        />

        <div className="flex items-center gap-2 shrink-0">
          <Button
            size="sm"
            variant="light"
            onPress={expandAll}
            className="text-xs text-slate-600 dark:text-slate-400 h-7"
          >
            Expand All
          </Button>
          <span className="text-slate-300 dark:text-[#26313D]">|</span>
          <Button
            size="sm"
            variant="light"
            onPress={collapseAll}
            className="text-xs text-slate-600 dark:text-slate-400 h-7"
          >
            Collapse All
          </Button>
        </div>
      </div>

      {/* Error State */}
      {error && (
        <div className="border border-rose-300 dark:border-rose-900 bg-rose-50 dark:bg-rose-950/20 text-rose-800 dark:text-rose-300 p-3 rounded-sm text-xs flex items-center gap-2">
          <ShieldAlert className="w-4 h-4 shrink-0 text-rose-600" />
          <span>Storage Discovery Error: {error}</span>
        </div>
      )}

      {/* 5. Hierarchical Expandable Devices (Information-First Structure, No Cards) */}
      <div className="space-y-4">
        {loading ? (
          <div className="py-12 text-center text-slate-400 text-xs">
            Scanning platform physical storage buses and filesystems...
          </div>
        ) : filteredDevices.length === 0 ? (
          <div className="py-12 text-center text-slate-400 text-xs border border-dashed border-slate-300 dark:border-[#26313D] rounded-sm">
            {searchQuery ? 'No storage devices match the query.' : 'No physical block devices discovered.'}
          </div>
        ) : (
          filteredDevices.map((device) => {
            const isExpanded = expandedDeviceIds.has(device.device_id);

            return (
              <div
                key={device.device_id}
                className="border border-slate-200 dark:border-[#26313D] rounded-sm overflow-hidden bg-white dark:bg-[#151C24] transition-colors"
              >
                {/* Physical Device Master Header Row */}
                <div
                  onClick={() => toggleExpand(device.device_id)}
                  className="p-3.5 flex flex-col md:flex-row md:items-center justify-between gap-3 cursor-pointer hover:bg-slate-50 dark:hover:bg-[#1B2530]/50 select-none border-b border-transparent"
                >
                  <div className="flex items-center gap-3">
                    <button
                      type="button"
                      className="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 p-0.5 cursor-pointer"
                      aria-label="Toggle details"
                    >
                      {isExpanded ? (
                        <ChevronDown className="w-4 h-4 text-slate-600 dark:text-slate-300" />
                      ) : (
                        <ChevronRight className="w-4 h-4 text-slate-400" />
                      )}
                    </button>

                    <div>
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="font-mono font-bold text-xs text-slate-900 dark:text-slate-100">
                          {device.device_id}
                        </span>
                        {renderClassificationBadge(device.classification, device.is_system_device)}
                        {renderDeviceTypeBadge(device.device_type)}
                      </div>
                      <div className="text-xs text-slate-600 dark:text-slate-400 font-medium mt-0.5">
                        {device.display_name}
                      </div>
                    </div>
                  </div>

                  <div className="flex items-center gap-4 text-xs font-mono ml-7 md:ml-0">
                    <div>
                      <span className="text-slate-400 text-[10px] uppercase block">Type</span>
                      <span className="font-semibold text-slate-700 dark:text-slate-300 uppercase">
                        {device.device_type}
                      </span>
                    </div>

                    <div>
                      <span className="text-slate-400 text-[10px] uppercase block">Capacity</span>
                      <span className="font-bold text-slate-900 dark:text-slate-100">
                        {formatBytes(device.capacity_bytes)}
                      </span>
                    </div>

                    <div>
                      <span className="text-slate-400 text-[10px] uppercase block">Volumes</span>
                      <span className="font-semibold text-slate-700 dark:text-slate-300">
                        {device.volumes.length}
                      </span>
                    </div>
                  </div>
                </div>

                {/* Expanded Device Details: Hardware Telemetry & Logical Volumes */}
                {isExpanded && (
                  <div className="p-4 pt-2 border-t border-slate-200 dark:border-[#26313D] bg-slate-50/50 dark:bg-[#10161D]/50 space-y-4 text-xs">
                    {/* Aligned Label-Value Hardware Metadata Grid */}
                    <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 py-2 border-b border-slate-200/80 dark:border-[#26313D]">
                      <div>
                        <span className="text-[10px] uppercase font-bold text-slate-400 dark:text-slate-500 block">
                          Vendor / Manufacturer
                        </span>
                        <span className="font-medium text-slate-800 dark:text-slate-200 mt-0.5 block">
                          {device.vendor || 'Generic / Platform Default'}
                        </span>
                      </div>

                      <div>
                        <span className="text-[10px] uppercase font-bold text-slate-400 dark:text-slate-500 block">
                          Model Identifier
                        </span>
                        <span className="font-medium text-slate-800 dark:text-slate-200 mt-0.5 block truncate">
                          {device.model || 'N/A'}
                        </span>
                      </div>

                      <div>
                        <span className="text-[10px] uppercase font-bold text-slate-400 dark:text-slate-500 block">
                          Media Subsystem
                        </span>
                        <span className="font-mono font-medium text-slate-800 dark:text-slate-200 mt-0.5 block truncate">
                          {device.removable ? 'Removable Storage' : 'Fixed Media'}
                        </span>
                      </div>

                      <div>
                        <span className="text-[10px] uppercase font-bold text-slate-400 dark:text-slate-500 block">
                          Classification Note
                        </span>
                        <span className="font-mono font-medium text-slate-800 dark:text-slate-200 mt-0.5 block truncate" title={device.classification_note}>
                          {device.classification_note || 'Standard Media'}
                        </span>
                      </div>
                    </div>

                    {/* Logical Volumes Sub-Table */}
                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <span className="text-[11px] font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400">
                          Logical Volumes ({device.volumes.length})
                        </span>
                      </div>

                      {device.volumes.length === 0 ? (
                        <div className="py-3 px-3 text-slate-400 italic text-[11px] bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] rounded-sm">
                          Unpartitioned raw physical disk space or unrecognized partition scheme.
                        </div>
                      ) : (
                        <Table
                          aria-label={`Volumes for ${device.device_id}`}
                          classNames={{
                            base: 'min-w-full',
                            table: 'min-w-full text-xs',
                            thead: '[&>tr]:first:rounded-none',
                            th: 'bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 font-semibold text-[10px] uppercase py-2 px-3 border-b border-slate-200 dark:border-[#26313D]',
                            td: 'py-2 px-3 border-b border-slate-200 dark:border-[#26313D] text-slate-800 dark:text-slate-200',
                            wrapper: 'bg-transparent shadow-none p-0 border border-slate-200 dark:border-[#26313D] rounded-sm',
                          }}
                        >
                          <TableHeader>
                            <TableColumn>MOUNT POINT</TableColumn>
                            <TableColumn>VOLUME LABEL</TableColumn>
                            <TableColumn>FILESYSTEM</TableColumn>
                            <TableColumn>CAPACITY</TableColumn>
                            <TableColumn>USAGE</TableColumn>
                            <TableColumn>ACCESS</TableColumn>
                          </TableHeader>
                          <TableBody>
                            {device.volumes.map((vol, idx) => {
                              const usedBytes = vol.capacity_bytes - vol.free_bytes;
                              const usedPercent =
                                vol.capacity_bytes > 0
                                  ? Math.round((usedBytes / vol.capacity_bytes) * 100)
                                  : 0;

                              return (
                                <TableRow key={idx} className="hover:bg-slate-100/50 dark:hover:bg-[#151C24]">
                                  <TableCell className="font-mono font-bold text-slate-900 dark:text-slate-100">
                                    {vol.mount_point || 'Unmounted'}
                                  </TableCell>
                                  <TableCell className="text-slate-600 dark:text-slate-300">
                                    {vol.label || 'No Volume Label'}
                                  </TableCell>
                                  <TableCell className="font-mono uppercase text-[11px]">
                                    {vol.filesystem_type}
                                  </TableCell>
                                  <TableCell className="font-mono text-[11px]">
                                    {formatBytes(vol.capacity_bytes)}
                                  </TableCell>
                                  <TableCell className="min-w-[120px]">
                                    <div className="space-y-1">
                                      <div className="flex justify-between text-[10px] font-mono text-slate-500">
                                        <span>{usedPercent}%</span>
                                        <span>{formatBytes(vol.free_bytes)} free</span>
                                      </div>
                                      <Progress
                                        size="sm"
                                        value={usedPercent}
                                        aria-label={`Usage for ${vol.mount_point || 'volume'}`}
                                        className="h-1"
                                      />
                                    </div>
                                  </TableCell>
                                  <TableCell>
                                    {vol.read_only ? (
                                      <Chip size="sm" variant="flat" className="font-mono text-[9px] font-bold bg-amber-100 dark:bg-amber-950/40 text-amber-800 dark:text-amber-300 h-5">
                                        READ-ONLY
                                      </Chip>
                                    ) : (
                                      <Chip size="sm" variant="flat" className="font-mono text-[9px] font-semibold bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 h-5">
                                        READ-WRITE
                                      </Chip>
                                    )}
                                  </TableCell>
                                </TableRow>
                              );
                            })}
                          </TableBody>
                        </Table>
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
