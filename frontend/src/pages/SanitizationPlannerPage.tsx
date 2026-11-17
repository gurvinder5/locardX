import React, { useEffect, useState } from 'react';
import {
  ShieldAlert,
  ShieldCheck,
  AlertTriangle,
  HardDrive,
  Cpu,
  Layers,
  FileText,
  Folder,
  RefreshCw,
  CheckCircle2,
  XCircle,
  BookOpen,
} from 'lucide-react';
import {
  Button,
  Input,
  Chip,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from '@heroui/react';
import { StorageDeviceDto } from '../types/device';
import { listStorageDevices } from '../services/device';
import {
  evaluateSanitizationPlan,
  getSanitizationMethods,
  compareTargetSnapshot,
} from '../services/sanitization';
import {
  SanitizationPlanDto,
  SanitizationStandardDto,
  SnapshotComparisonResultDto,
} from '../types/sanitization';
import { useAuthStore } from '../stores/authStore';

function formatBytes(bytes?: number | null): string {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

export const SanitizationPlannerPage: React.FC = () => {
  const { sessionToken } = useAuthStore();

  const [devices, setDevices] = useState<StorageDeviceDto[]>([]);
  const [loadingDevices, setLoadingDevices] = useState<boolean>(false);
  const [standards, setStandards] = useState<SanitizationStandardDto[]>([]);
  const [showStandardsModal, setShowStandardsModal] = useState<boolean>(false);

  // Form selections
  const [targetType, setTargetType] = useState<string>('PhysicalDevice');
  const [selectedTargetId, setSelectedTargetId] = useState<string>('');
  const [customPath, setCustomPath] = useState<string>('');
  const [scope, setScope] = useState<string>('PhysicalDevice');
  const [requestedMethod, setRequestedMethod] = useState<string>('');
  const [requestedStrategy, setRequestedStrategy] = useState<string>('');

  // Evaluation & TOCTOU states
  const [evaluating, setEvaluating] = useState<boolean>(false);
  const [currentPlan, setCurrentPlan] = useState<SanitizationPlanDto | null>(null);
  const [snapshotResult, setSnapshotResult] = useState<SnapshotComparisonResultDto | null>(null);
  const [checkingSnapshot, setCheckingSnapshot] = useState<boolean>(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    loadHardwareDevices();
    loadStandards();
  }, []);

  async function loadHardwareDevices() {
    try {
      setLoadingDevices(true);
      const list = await listStorageDevices();
      setDevices(list);
      if (list.length > 0 && !selectedTargetId) {
        setSelectedTargetId(list[0].device_id);
      }
    } catch (err) {
      console.error('Failed to query storage devices:', err);
    } finally {
      setLoadingDevices(false);
    }
  }

  async function loadStandards() {
    try {
      const data = await getSanitizationMethods();
      setStandards(data);
    } catch (err) {
      console.error('Failed to load standards registry:', err);
    }
  }

  const handleTargetTypeChange = (type: string) => {
    setTargetType(type);
    if (type === 'PhysicalDevice') {
      setScope('PhysicalDevice');
      if (devices.length > 0) setSelectedTargetId(devices[0].device_id);
    } else if (type === 'LogicalVolume') {
      setScope('LogicalVolume');
      const firstVol = devices.flatMap((d) => d.volumes)[0];
      if (firstVol) {
        setSelectedTargetId(firstVol.mount_point || firstVol.volume_id);
      }
    } else if (type === 'File') {
      setScope('File');
      setCustomPath('C:\\Evidence\\sample.dd');
    } else if (type === 'Directory') {
      setScope('Folder');
      setCustomPath('C:\\Evidence\\Case01');
    }
  };

  const handleEvaluate = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorMsg(null);
    setCurrentPlan(null);
    setSnapshotResult(null);

    const targetIdentifier =
      targetType === 'File' || targetType === 'Directory'
        ? customPath.trim()
        : selectedTargetId.trim();

    if (!targetIdentifier) {
      setErrorMsg('Please specify or select a target storage identifier.');
      return;
    }

    try {
      setEvaluating(true);
      const plan = await evaluateSanitizationPlan({
        target_identifier: targetIdentifier,
        target_type: targetType,
        scope,
        requested_method: requestedMethod || null,
        requested_strategy: requestedStrategy || null,
        session_token: sessionToken || null,
      });
      setCurrentPlan(plan);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMsg(msg);
    } finally {
      setEvaluating(false);
    }
  };

  const handleCheckSnapshot = async () => {
    if (!currentPlan) return;
    try {
      setCheckingSnapshot(true);
      const result = await compareTargetSnapshot(currentPlan.plan_id);
      setSnapshotResult(result);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMsg(`Failed to probe target snapshot: ${msg}`);
    } finally {
      setCheckingSnapshot(false);
    }
  };

  const selectedDevice = devices.find((d) => d.device_id === selectedTargetId);
  const selectedVolume = devices
    .flatMap((d) => d.volumes)
    .find((v) => v.mount_point === selectedTargetId || v.volume_id === selectedTargetId);

  return (
    <div className="space-y-6 pb-10">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-slate-200 dark:border-[#26313D] pb-4">
        <div>
          <h1 className="text-xl font-bold text-slate-900 dark:text-slate-100 tracking-tight flex items-center gap-2">
            <Cpu className="w-5 h-5 text-slate-700 dark:text-slate-300" />
            Sanitization Planner & Standards Engine
          </h1>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
            Media-aware method selection, verification requirements, and standards mapping.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="flat"
            onPress={() => setShowStandardsModal(true)}
            startContent={<BookOpen className="w-3.5 h-3.5 text-slate-600 dark:text-slate-400" />}
            className="text-xs font-semibold bg-white dark:bg-[#1B2530] border border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300 h-8"
          >
            Standards Registry
          </Button>
          <Button
            size="sm"
            variant="flat"
            onPress={loadHardwareDevices}
            isDisabled={loadingDevices}
            startContent={<RefreshCw className={`w-3.5 h-3.5 ${loadingDevices ? 'animate-spin' : ''}`} />}
            className="text-xs font-semibold bg-white dark:bg-[#1B2530] border border-slate-300 dark:border-[#26313D] text-slate-700 dark:text-slate-300 h-8"
          >
            Refresh Devices
          </Button>
        </div>
      </div>

      {/* Strict Non-Destructive Banner (Flat Alert) */}
      <div className="border-l-2 border-amber-500 bg-amber-500/10 p-3 text-xs text-amber-900 dark:text-amber-200 flex items-start space-x-3">
        <AlertTriangle className="w-4 h-4 text-amber-600 dark:text-amber-400 mt-0.5 shrink-0" />
        <div className="space-y-0.5 leading-relaxed">
          <span className="font-bold uppercase tracking-wider text-[11px] block text-amber-800 dark:text-amber-300">
            Planning / Dry Run Mode — Zero Storage Modification
          </span>
          <p className="text-[11px] text-amber-900 dark:text-amber-200">
            This workstation interface evaluates media compatibility, verification requirements, and technical limitations. All destructive storage modification engines remain disabled.
          </p>
        </div>
      </div>

      {/* Main Two-Column Layout */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 pt-1">
        {/* Left Column: Form Configuration */}
        <div className="lg:col-span-5 space-y-4">
          <div className="space-y-4">
            <div className="flex items-center gap-2 pb-2 border-b border-slate-200 dark:border-[#26313D]">
              <Layers className="w-4 h-4 text-slate-700 dark:text-slate-300" />
              <h2 className="text-sm font-bold text-slate-900 dark:text-slate-100">
                Target & Scope Configuration
              </h2>
            </div>

            <form onSubmit={handleEvaluate} className="space-y-4">
              {/* Target Type Selector */}
              <div>
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
                  Target Category
                </label>
                <div className="grid grid-cols-2 gap-2 text-xs">
                  {[
                    { key: 'PhysicalDevice', label: 'Physical Drive', icon: <HardDrive className="w-3.5 h-3.5" /> },
                    { key: 'LogicalVolume', label: 'Logical Volume', icon: <Layers className="w-3.5 h-3.5" /> },
                    { key: 'Directory', label: 'Folder / Tree', icon: <Folder className="w-3.5 h-3.5" /> },
                    { key: 'File', label: 'Single File', icon: <FileText className="w-3.5 h-3.5" /> },
                  ].map((item) => (
                    <Button
                      key={item.key}
                      size="sm"
                      variant={targetType === item.key ? 'solid' : 'flat'}
                      onPress={() => handleTargetTypeChange(item.key)}
                      startContent={item.icon}
                      className={`justify-start text-xs h-8 rounded-xs ${
                        targetType === item.key
                          ? 'bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 font-bold shadow-2xs'
                          : 'bg-white dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]'
                      }`}
                    >
                      {item.label}
                    </Button>
                  ))}
                </div>
              </div>

              {/* Target Selector / Input */}
              {targetType === 'PhysicalDevice' && (
                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                    Select Physical Drive
                  </label>
                  <select
                    value={selectedTargetId}
                    onChange={(e) => setSelectedTargetId(e.target.value)}
                    className="w-full text-xs bg-white dark:bg-[#151C24] border border-slate-300 dark:border-[#26313D] rounded-xs px-2.5 py-1.5 text-slate-900 dark:text-slate-100 focus:outline-none focus:border-slate-500 font-mono"
                  >
                    {devices.map((d) => (
                      <option key={d.device_id} value={d.device_id}>
                        {d.device_id} — {d.display_name} ({formatBytes(d.capacity_bytes)}) [{d.classification}]
                      </option>
                    ))}
                  </select>
                </div>
              )}

              {targetType === 'LogicalVolume' && (
                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                    Select Logical Volume
                  </label>
                  <select
                    value={selectedTargetId}
                    onChange={(e) => setSelectedTargetId(e.target.value)}
                    className="w-full text-xs bg-white dark:bg-[#151C24] border border-slate-300 dark:border-[#26313D] rounded-xs px-2.5 py-1.5 text-slate-900 dark:text-slate-100 focus:outline-none focus:border-slate-500 font-mono"
                  >
                    {devices.flatMap((d) =>
                      d.volumes.map((v) => (
                        <option key={v.volume_id} value={v.mount_point || v.volume_id}>
                          {v.mount_point || v.volume_id} ({v.label || 'No Label'}) — {formatBytes(v.capacity_bytes)} [{v.filesystem_type}]
                        </option>
                      ))
                    )}
                  </select>
                </div>
              )}

              {(targetType === 'File' || targetType === 'Directory') && (
                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                    Path Identifier
                  </label>
                  <Input
                    size="sm"
                    value={customPath}
                    onValueChange={setCustomPath}
                    placeholder="C:\path\to\target"
                    className="font-mono text-xs"
                    classNames={{
                      inputWrapper: 'bg-white dark:bg-[#151C24] border border-slate-300 dark:border-[#26313D] h-8',
                    }}
                  />
                </div>
              )}

              {/* Scope Selector */}
              <div>
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                  Sanitization Scope
                </label>
                <select
                  value={scope}
                  onChange={(e) => setScope(e.target.value)}
                  className="w-full text-xs bg-white dark:bg-[#151C24] border border-slate-300 dark:border-[#26313D] rounded-xs px-2.5 py-1.5 text-slate-900 dark:text-slate-100 focus:outline-none focus:border-slate-500"
                >
                  <option value="PhysicalDevice">Physical Device (Whole Disk Overwrite/Crypto)</option>
                  <option value="LogicalVolume">Logical Volume (Partition Container)</option>
                  <option value="Folder">Folder (Recursive Directory Shred)</option>
                  <option value="File">File (Payload Overwrite & Unlink)</option>
                </select>
              </div>

              {/* Optional Method Override */}
              <div>
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                  Method Selection <span className="text-slate-400 font-normal">(Auto-Selected if Default)</span>
                </label>
                <select
                  value={requestedMethod}
                  onChange={(e) => setRequestedMethod(e.target.value)}
                  className="w-full text-xs bg-white dark:bg-[#151C24] border border-slate-300 dark:border-[#26313D] rounded-xs px-2.5 py-1.5 text-slate-900 dark:text-slate-100 focus:outline-none focus:border-slate-500"
                >
                  <option value="">Auto-Recommend based on Media Type</option>
                  <option value="Nist80088ClearZero">1. NIST SP 800-88 Rev. 1 (Clear - Single Pass Zero)</option>
                  <option value="Nist80088PurgeCrypto">2. NIST SP 800-88 Rev. 1 (Purge - Cryptographic Erase)</option>
                  <option value="Dod522022M">3. DoD 5220.22-M (3-Pass Overwrite & Verify)</option>
                  <option value="NvmeCryptoErase">4. NVMe Base Spec Cryptographic Erase (Format SES=2)</option>
                  <option value="AtaSecureErase">5. ATA Firmware Secure Erase / Sanitize (ACS-4)</option>
                  <option value="BlockZeroOverwrite">6. Raw Block-Level Zero Overwrite</option>
                  <option value="LogicalFileShred">7. LocardX Forensic File & Folder Shred & Unlink</option>
                </select>
              </div>

              {/* Optional Verification Strategy Override */}
              <div>
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                  Verification Strategy <span className="text-slate-400 font-normal">(Auto-Selected if Default)</span>
                </label>
                <select
                  value={requestedStrategy}
                  onChange={(e) => setRequestedStrategy(e.target.value)}
                  className="w-full text-xs bg-white dark:bg-[#151C24] border border-slate-300 dark:border-[#26313D] rounded-xs px-2.5 py-1.5 text-slate-900 dark:text-slate-100 focus:outline-none focus:border-slate-500"
                >
                  <option value="">Auto-Selected for Method</option>
                  <option value="SampledRandomSectors">Sampled Pseudo-Random Sectors (10%)</option>
                  <option value="FullReadBack">Full Sequential Read-Back (100% Sectors)</option>
                  <option value="CryptoKeyDestructionCheck">Cryptographic Key Destruction Check</option>
                  <option value="MetadataUnlinkCheck">Filesystem Metadata & Unlink Verification</option>
                  <option value="NoVerification">No Verification</option>
                </select>
              </div>

              {/* Submit Button */}
              <div className="pt-2">
                <Button
                  type="submit"
                  isDisabled={evaluating}
                  isLoading={evaluating}
                  startContent={!evaluating && <Cpu className="w-3.5 h-3.5" />}
                  className="w-full bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 font-semibold text-xs h-8 shadow-xs"
                >
                  {evaluating ? 'Evaluating Standards & Media...' : 'Evaluate Sanitization Plan (Dry Run)'}
                </Button>
              </div>
            </form>
          </div>

          {/* Target Physical Telemetry (Aligned Label-Value Grid, No Card) */}
          {(selectedDevice || selectedVolume) && (
            <div className="border border-slate-200 dark:border-[#26313D] p-3 rounded-xs bg-slate-50 dark:bg-[#10161D]/50 text-xs space-y-1.5">
              <span className="font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider text-[10px] block">
                Hardware Telemetry Snapshot
              </span>
              {selectedDevice && (
                <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-slate-600 dark:text-slate-400">
                  <div>Device ID: <span className="font-mono text-slate-900 dark:text-slate-100 font-semibold">{selectedDevice.device_id}</span></div>
                  <div>Type: <span className="text-slate-900 dark:text-slate-100">{selectedDevice.device_type}</span></div>
                  <div>Capacity: <span className="text-slate-900 dark:text-slate-100">{formatBytes(selectedDevice.capacity_bytes)}</span></div>
                  <div>Classification: <span className="text-slate-900 dark:text-slate-100">{selectedDevice.classification}</span></div>
                  <div>System Device: <span className={selectedDevice.is_system_device ? 'text-rose-700 dark:text-rose-400 font-semibold' : 'text-slate-900 dark:text-slate-100'}>{selectedDevice.is_system_device ? 'YES' : 'No'}</span></div>
                  <div>Removable: <span className="text-slate-900 dark:text-slate-100">{selectedDevice.removable ? 'Yes' : 'No'}</span></div>
                </div>
              )}
              {selectedVolume && (
                <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-slate-600 dark:text-slate-400 pt-1 border-t border-slate-200 dark:border-[#26313D]">
                  <div>Mount: <span className="font-mono text-slate-900 dark:text-slate-100 font-semibold">{selectedVolume.mount_point || 'None'}</span></div>
                  <div>Filesystem: <span className="text-slate-900 dark:text-slate-100">{selectedVolume.filesystem_type}</span></div>
                  <div>System Volume: <span className={selectedVolume.is_system_volume ? 'text-rose-700 dark:text-rose-400 font-semibold' : 'text-slate-900 dark:text-slate-100'}>{selectedVolume.is_system_volume ? 'YES' : 'No'}</span></div>
                  <div>Boot Volume: <span className={selectedVolume.is_boot_volume ? 'text-rose-700 dark:text-rose-400 font-semibold' : 'text-slate-900 dark:text-slate-100'}>{selectedVolume.is_boot_volume ? 'YES' : 'No'}</span></div>
                </div>
              )}
            </div>
          )}

          {errorMsg && (
            <div className="p-3 bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 rounded-xs text-xs text-rose-800 dark:text-rose-300 flex items-start gap-2">
              <XCircle className="w-4 h-4 text-rose-600 shrink-0 mt-0.5" />
              <span>{errorMsg}</span>
            </div>
          )}
        </div>

        {/* Right Column: Evaluated Plan Display (Flat Information-First) */}
        <div className="lg:col-span-7 space-y-4">
          {currentPlan ? (
            <div className="space-y-4">
              {/* Verdict Header Banner */}
              <div
                className={`p-3.5 border-l-2 flex items-center justify-between ${
                  currentPlan.is_applicable
                    ? 'border-l-slate-700 bg-slate-100 dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D]'
                    : 'border-l-rose-600 bg-rose-50 dark:bg-rose-950/20 border border-rose-200 dark:border-rose-900/40'
                }`}
              >
                <div className="flex items-center gap-2.5">
                  {currentPlan.is_applicable ? (
                    <ShieldCheck className="w-5 h-5 text-slate-700 dark:text-slate-300" />
                  ) : (
                    <ShieldAlert className="w-5 h-5 text-rose-600 dark:text-rose-400" />
                  )}
                  <div>
                    <div className="text-xs font-bold uppercase tracking-wider text-slate-900 dark:text-slate-100">
                      {currentPlan.is_applicable
                        ? 'Sanitization Plan Formulated (Dry Run)'
                        : 'Sanitization Hard-Blocked / Incompatible'}
                    </div>
                    <div className="text-[11px] text-slate-500 font-mono mt-0.5">
                      Plan ID: {currentPlan.plan_id}
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  <Chip
                    size="sm"
                    variant="flat"
                    className={`text-[10px] font-bold uppercase ${
                      currentPlan.is_applicable
                        ? 'bg-slate-200 dark:bg-[#26313D] text-slate-800 dark:text-slate-200'
                        : 'bg-rose-100 dark:bg-rose-950/50 text-rose-800 dark:text-rose-300'
                    }`}
                  >
                    {currentPlan.is_applicable ? 'Applicable' : 'Blocked'}
                  </Chip>
                  <Chip
                    size="sm"
                    variant="flat"
                    className={`text-[10px] font-bold uppercase ${
                      currentPlan.risk_level === 'Critical'
                        ? 'bg-rose-100 dark:bg-rose-950/50 text-rose-800 dark:text-rose-300'
                        : currentPlan.risk_level === 'High'
                        ? 'bg-amber-100 dark:bg-amber-950/50 text-amber-800 dark:text-amber-300'
                        : 'bg-slate-200 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300'
                    }`}
                  >
                    Risk: {currentPlan.risk_level}
                  </Chip>
                </div>
              </div>

              {/* Plan Metadata Aligned Grid */}
              <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 p-3.5 bg-slate-50 dark:bg-[#151C24] rounded-xs border border-slate-200 dark:border-[#26313D] text-xs">
                <div>
                  <span className="text-slate-400 block text-[10px] uppercase font-bold">Target</span>
                  <span className="font-mono font-medium text-slate-900 dark:text-slate-100 truncate block mt-0.5">
                    {currentPlan.target_identifier}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 block text-[10px] uppercase font-bold">Media Category</span>
                  <span className="text-slate-900 dark:text-slate-100 font-medium mt-0.5 block">{currentPlan.media_type}</span>
                </div>
                <div>
                  <span className="text-slate-400 block text-[10px] uppercase font-bold">Scope</span>
                  <span className="text-slate-900 dark:text-slate-100 font-medium mt-0.5 block">{currentPlan.scope}</span>
                </div>
                <div>
                  <span className="text-slate-400 block text-[10px] uppercase font-bold">Recommended Method</span>
                  <span className="text-slate-900 dark:text-slate-100 font-bold mt-0.5 block">{currentPlan.recommended_method}</span>
                </div>
                <div>
                  <span className="text-slate-400 block text-[10px] uppercase font-bold">Formal Standard</span>
                  <span className="text-slate-900 dark:text-slate-100 font-medium mt-0.5 block">{currentPlan.applicable_standard || 'N/A'}</span>
                </div>
                <div>
                  <span className="text-slate-400 block text-[10px] uppercase font-bold">Verification Strategy</span>
                  <span className="text-slate-900 dark:text-slate-100 font-medium mt-0.5 block">{currentPlan.verification_strategy}</span>
                </div>
              </div>

              {/* Technical Limitations Section */}
              {currentPlan.limitations.length > 0 && (
                <div className="space-y-2">
                  <span className="text-xs font-bold text-slate-900 dark:text-slate-100 uppercase tracking-wider block">
                    Documented Verification Limitations
                  </span>
                  <ul className="space-y-1">
                    {currentPlan.limitations.map((lim, idx) => (
                      <li
                        key={idx}
                        className="p-2.5 bg-amber-500/10 border-l-2 border-amber-500 text-[11px] text-amber-900 dark:text-amber-200"
                      >
                        {lim}
                      </li>
                    ))}
                  </ul>
                </div>
              )}

              {/* Reason Codes */}
              {currentPlan.reason_codes.length > 0 && (
                <div className="space-y-1.5">
                  <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
                    Reason Telemetry
                  </span>
                  <div className="flex flex-wrap gap-1.5">
                    {currentPlan.reason_codes.map((rc, idx) => (
                      <Chip
                        key={idx}
                        size="sm"
                        variant="flat"
                        className="font-mono text-[10px] bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]"
                      >
                        {rc}
                      </Chip>
                    ))}
                  </div>
                </div>
              )}

              {/* TOCTOU Live Snapshot Integrity Guard */}
              <div className="pt-3 border-t border-slate-200 dark:border-[#26313D] space-y-3">
                <div className="flex items-center justify-between">
                  <div>
                    <h4 className="font-bold text-slate-900 dark:text-slate-100 text-xs uppercase tracking-wider">
                      Snapshot Integrity & TOCTOU Guard
                    </h4>
                    <p className="text-[11px] text-slate-500 dark:text-slate-400">
                      Re-probes hardware to verify device capacity, serial, and geometry have not changed.
                    </p>
                  </div>
                  <Button
                    size="sm"
                    variant="flat"
                    onPress={handleCheckSnapshot}
                    isDisabled={checkingSnapshot}
                    isLoading={checkingSnapshot}
                    startContent={!checkingSnapshot && <RefreshCw className="w-3 h-3" />}
                    className="bg-white dark:bg-[#1B2530] border border-slate-300 dark:border-[#26313D] text-xs font-semibold text-slate-700 dark:text-slate-300 h-8"
                  >
                    Verify Live Snapshot
                  </Button>
                </div>

                {snapshotResult && (
                  <div
                    className={`p-3 rounded-xs border text-xs ${
                      snapshotResult.matches
                        ? 'bg-slate-100 dark:bg-[#1B2530] border-slate-300 dark:border-[#26313D] text-slate-800 dark:text-slate-200'
                        : 'bg-rose-50 dark:bg-rose-950/30 border-rose-200 dark:border-rose-900/40 text-rose-900 dark:text-rose-300'
                    }`}
                  >
                    <div className="flex items-center gap-2 font-semibold">
                      {snapshotResult.matches ? (
                        <>
                          <CheckCircle2 className="w-4 h-4 text-slate-700 dark:text-slate-300" />
                          Hardware Match Confirmed: Target geometry and identity unchanged.
                        </>
                      ) : (
                        <>
                          <XCircle className="w-4 h-4 text-rose-600 shrink-0" />
                          Target Changed: {snapshotResult.reason}
                        </>
                      )}
                    </div>
                    {snapshotResult.differences.length > 0 && (
                      <ul className="mt-1.5 list-disc list-inside text-[11px]">
                        {snapshotResult.differences.map((diff, idx) => (
                          <li key={idx}>{diff}</li>
                        ))}
                      </ul>
                    )}
                  </div>
                )}
              </div>
            </div>
          ) : (
            <div className="text-center py-16 border border-dashed border-slate-300 dark:border-[#26313D] rounded-sm space-y-2">
              <Cpu className="w-8 h-8 text-slate-300 dark:text-slate-600 mx-auto" />
              <h3 className="text-sm font-semibold text-slate-700 dark:text-slate-300">No Plan Evaluated Yet</h3>
              <p className="text-xs text-slate-500 dark:text-slate-400 max-w-sm mx-auto">
                Configure a target storage identifier and scope on the left, then click <strong>Evaluate Sanitization Plan</strong>.
              </p>
            </div>
          )}
        </div>
      </div>

      {/* Standards Registry Reference Modal */}
      <Modal
        isOpen={showStandardsModal}
        onClose={() => setShowStandardsModal(false)}
        size="2xl"
        scrollBehavior="inside"
        classNames={{
          base: 'bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D]',
          header: 'border-b border-slate-200 dark:border-[#26313D]',
          footer: 'border-t border-slate-200 dark:border-[#26313D]',
        }}
      >
        <ModalContent>
          <ModalHeader className="flex items-center gap-2">
            <BookOpen className="w-4 h-4 text-slate-700 dark:text-slate-300" />
            <span className="text-sm font-bold text-slate-900 dark:text-slate-100">
              Sanitization Standards & Verification Registry
            </span>
          </ModalHeader>

          <ModalBody className="p-5 space-y-4 text-xs">
            {standards.map((std) => (
              <div
                key={std.standard_id}
                className="border border-slate-200 dark:border-[#26313D] p-3 rounded-xs space-y-1.5 bg-slate-50/50 dark:bg-[#10161D]/50"
              >
                <div className="flex items-center justify-between">
                  <span className="font-bold text-slate-900 dark:text-slate-100">{std.standard_name || std.method_name}</span>
                  <Chip size="sm" variant="flat" className="font-mono text-[9px] uppercase">
                    {std.standard_id}
                  </Chip>
                </div>
                <p className="text-slate-600 dark:text-slate-400 text-[11px]">{std.description}</p>
                <div className="flex flex-wrap gap-1.5 pt-1">
                  {(std.applicable_media || []).map((med: string, i: number) => (
                    <span
                      key={i}
                      className="text-[9px] bg-slate-200 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 px-1.5 py-0.5 rounded-xs"
                    >
                      {med}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </ModalBody>

          <ModalFooter>
            <Button
              size="sm"
              variant="flat"
              onPress={() => setShowStandardsModal(false)}
              className="text-xs bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 border border-slate-300 dark:border-[#26313D]"
            >
              Close Registry
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  );
};

export default SanitizationPlannerPage;
