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
  Info,
  CheckCircle2,
  XCircle,
  BookOpen,
  X,
} from 'lucide-react';
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

  // Load storage devices and standards on mount
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

  // Auto-align scope when target type changes
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

  // Find selected hardware telemetry
  const selectedDevice = devices.find((d) => d.device_id === selectedTargetId);
  const selectedVolume = devices
    .flatMap((d) => d.volumes)
    .find((v) => v.mount_point === selectedTargetId || v.volume_id === selectedTargetId);

  return (
    <div className="space-y-6">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-slate-200 pb-4">
        <div>
          <h1 className="text-xl font-semibold text-slate-900 tracking-tight flex items-center gap-2">
            <Cpu className="w-6 h-6 text-slate-700" />
            Sanitization Planner & Standards Engine
          </h1>
          <p className="text-xs text-slate-700 mt-1">
            Media-aware method selection, verification requirements, and standards mapping.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowStandardsModal(true)}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-slate-700 bg-white border border-slate-300 rounded hover:bg-slate-50 transition"
          >
            <BookOpen className="w-3.5 h-3.5 text-slate-500" />
            Standards Registry
          </button>
          <button
            onClick={loadHardwareDevices}
            disabled={loadingDevices}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-slate-700 bg-white border border-slate-300 rounded hover:bg-slate-50 transition disabled:opacity-50"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loadingDevices ? 'animate-spin text-slate-500' : ''}`} />
            Refresh Devices
          </button>
        </div>
      </div>

      {/* Strict Non-Destructive Banner */}
      <div className="bg-amber-50 border-l-4 border-amber-500 p-3.5 rounded-r">
        <div className="flex items-center gap-2">
          <AlertTriangle className="w-4 h-4 text-amber-700 shrink-0" />
          <span className="text-xs font-semibold text-amber-900 uppercase tracking-wider">
            PLANNING / DRY RUN — NO STORAGE MODIFICATION
          </span>
        </div>
        <p className="text-xs text-amber-800 mt-1 ml-6">
          This workstation interface evaluates media compatibility, verification requirements, and technical limitations.
          <strong> All destructive storage modification engines remain disabled.</strong> No sectors, partitions, or files will be modified.
        </p>
      </div>

      {/* Main Two-Column Layout */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left Column: Form Configuration */}
        <div className="lg:col-span-5 space-y-4">
          <div className="bg-white border border-slate-200 rounded p-4 shadow-sm">
            <h2 className="text-sm font-semibold text-slate-900 mb-3 flex items-center gap-2">
              <Layers className="w-4 h-4 text-slate-500" />
              Target & Scope Configuration
            </h2>

            <form onSubmit={handleEvaluate} className="space-y-3.5">
              {/* Target Type Selector */}
              <div>
                <label className="block text-xs font-medium text-slate-700 mb-1">Target Category</label>
                <div className="grid grid-cols-2 gap-2 text-xs">
                  <button
                    type="button"
                    onClick={() => handleTargetTypeChange('PhysicalDevice')}
                    className={`py-1.5 px-2.5 rounded border text-left flex items-center gap-1.5 transition ${
                      targetType === 'PhysicalDevice'
                        ? 'bg-slate-900 text-white border-slate-900'
                        : 'bg-white text-slate-700 border-slate-200 hover:bg-slate-50'
                    }`}
                  >
                    <HardDrive className="w-3.5 h-3.5" />
                    Physical Drive
                  </button>
                  <button
                    type="button"
                    onClick={() => handleTargetTypeChange('LogicalVolume')}
                    className={`py-1.5 px-2.5 rounded border text-left flex items-center gap-1.5 transition ${
                      targetType === 'LogicalVolume'
                        ? 'bg-slate-900 text-white border-slate-900'
                        : 'bg-white text-slate-700 border-slate-200 hover:bg-slate-50'
                    }`}
                  >
                    <Layers className="w-3.5 h-3.5" />
                    Logical Volume
                  </button>
                  <button
                    type="button"
                    onClick={() => handleTargetTypeChange('Directory')}
                    className={`py-1.5 px-2.5 rounded border text-left flex items-center gap-1.5 transition ${
                      targetType === 'Directory'
                        ? 'bg-slate-900 text-white border-slate-900'
                        : 'bg-white text-slate-700 border-slate-200 hover:bg-slate-50'
                    }`}
                  >
                    <Folder className="w-3.5 h-3.5" />
                    Folder / Tree
                  </button>
                  <button
                    type="button"
                    onClick={() => handleTargetTypeChange('File')}
                    className={`py-1.5 px-2.5 rounded border text-left flex items-center gap-1.5 transition ${
                      targetType === 'File'
                        ? 'bg-slate-900 text-white border-slate-900'
                        : 'bg-white text-slate-700 border-slate-200 hover:bg-slate-50'
                    }`}
                  >
                    <FileText className="w-3.5 h-3.5" />
                    Single File
                  </button>
                </div>
              </div>

              {/* Target Selector / Input */}
              {targetType === 'PhysicalDevice' && (
                <div>
                  <label className="block text-xs font-medium text-slate-700 mb-1">Select Physical Drive</label>
                  <select
                    value={selectedTargetId}
                    onChange={(e) => setSelectedTargetId(e.target.value)}
                    className="w-full text-xs bg-white border border-slate-300 rounded px-2.5 py-1.5 text-slate-900 focus:outline-none focus:border-slate-500 font-mono"
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
                  <label className="block text-xs font-medium text-slate-700 mb-1">Select Logical Volume</label>
                  <select
                    value={selectedTargetId}
                    onChange={(e) => setSelectedTargetId(e.target.value)}
                    className="w-full text-xs bg-white border border-slate-300 rounded px-2.5 py-1.5 text-slate-900 focus:outline-none focus:border-slate-500 font-mono"
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
                  <label className="block text-xs font-medium text-slate-700 mb-1">Path Identifier</label>
                  <input
                    type="text"
                    value={customPath}
                    onChange={(e) => setCustomPath(e.target.value)}
                    placeholder="C:\path\to\target"
                    className="w-full text-xs bg-white border border-slate-300 rounded px-2.5 py-1.5 text-slate-900 focus:outline-none focus:border-slate-500 font-mono"
                  />
                </div>
              )}

              {/* Scope Selector */}
              <div>
                <label className="block text-xs font-medium text-slate-700 mb-1">Sanitization Scope</label>
                <select
                  value={scope}
                  onChange={(e) => setScope(e.target.value)}
                  className="w-full text-xs bg-white border border-slate-300 rounded px-2.5 py-1.5 text-slate-900 focus:outline-none focus:border-slate-500"
                >
                  <option value="PhysicalDevice">Physical Device (Whole Disk Overwrite/Crypto)</option>
                  <option value="LogicalVolume">Logical Volume (Partition Container)</option>
                  <option value="Folder">Folder (Recursive Directory Shred)</option>
                  <option value="File">File (Payload Overwrite & Unlink)</option>
                </select>
              </div>

              {/* Optional Method Override */}
              <div>
                <label className="block text-xs font-medium text-slate-700 mb-1">
                  Method Selection <span className="text-slate-400 font-normal">(Auto-Selected if Default)</span>
                </label>
                <select
                  value={requestedMethod}
                  onChange={(e) => setRequestedMethod(e.target.value)}
                  className="w-full text-xs bg-white border border-slate-300 rounded px-2.5 py-1.5 text-slate-900 focus:outline-none focus:border-slate-500"
                >
                  <option value="">Auto-Recommend based on Media Type</option>
                  <option value="Nist80088ClearZero">NIST SP 800-88 Rev. 1 (Clear - Single Pass Zero)</option>
                  <option value="Nist80088PurgeCrypto">NIST SP 800-88 Rev. 1 (Purge - Cryptographic Erase)</option>
                  <option value="Dod522022M">DoD 5220.22-M (3-Pass Overwrite & Verify)</option>
                  <option value="NvmeCryptoErase">NVMe Base Spec Cryptographic Erase (Format)</option>
                  <option value="LogicalFileShred">LocardX Logical File Shred & Unlink</option>
                </select>
              </div>

              {/* Optional Verification Strategy Override */}
              <div>
                <label className="block text-xs font-medium text-slate-700 mb-1">
                  Verification Strategy <span className="text-slate-400 font-normal">(Auto-Selected if Default)</span>
                </label>
                <select
                  value={requestedStrategy}
                  onChange={(e) => setRequestedStrategy(e.target.value)}
                  className="w-full text-xs bg-white border border-slate-300 rounded px-2.5 py-1.5 text-slate-900 focus:outline-none focus:border-slate-500"
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
                <button
                  type="submit"
                  disabled={evaluating}
                  className="w-full py-2 px-4 bg-slate-900 text-white rounded text-xs font-medium hover:bg-slate-800 transition disabled:opacity-50 flex items-center justify-center gap-2 shadow-sm"
                >
                  {evaluating ? (
                    <>
                      <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                      Evaluating Standards & Media...
                    </>
                  ) : (
                    <>
                      <Cpu className="w-3.5 h-3.5" />
                      Evaluate Sanitization Plan (Dry Run)
                    </>
                  )}
                </button>
              </div>
            </form>
          </div>

          {/* Target Physical Telemetry Card */}
          {(selectedDevice || selectedVolume) && (
            <div className="bg-slate-50 border border-slate-200 rounded p-3 text-xs space-y-1.5">
              <div className="font-semibold text-slate-700 uppercase tracking-wider text-[10px]">
                Hardware Telemetry Snapshot
              </div>
              {selectedDevice && (
                <div className="grid grid-cols-2 gap-x-2 gap-y-1 text-slate-600">
                  <div>Device ID: <span className="font-mono text-slate-900">{selectedDevice.device_id}</span></div>
                  <div>Type: <span className="text-slate-900">{selectedDevice.device_type}</span></div>
                  <div>Capacity: <span className="text-slate-900">{formatBytes(selectedDevice.capacity_bytes)}</span></div>
                  <div>Classification: <span className="text-slate-900">{selectedDevice.classification}</span></div>
                  <div>System Device: <span className={selectedDevice.is_system_device ? 'text-red-700 font-semibold' : 'text-slate-900'}>{selectedDevice.is_system_device ? 'YES' : 'No'}</span></div>
                  <div>Removable: <span className="text-slate-900">{selectedDevice.removable ? 'Yes' : 'No'}</span></div>
                </div>
              )}
              {selectedVolume && (
                <div className="grid grid-cols-2 gap-x-2 gap-y-1 text-slate-600 pt-1 border-t border-slate-200">
                  <div>Mount: <span className="font-mono text-slate-900">{selectedVolume.mount_point || 'None'}</span></div>
                  <div>Filesystem: <span className="text-slate-900">{selectedVolume.filesystem_type}</span></div>
                  <div>System Volume: <span className={selectedVolume.is_system_volume ? 'text-red-700 font-semibold' : 'text-slate-900'}>{selectedVolume.is_system_volume ? 'YES' : 'No'}</span></div>
                  <div>Boot Volume: <span className={selectedVolume.is_boot_volume ? 'text-red-700 font-semibold' : 'text-slate-900'}>{selectedVolume.is_boot_volume ? 'YES' : 'No'}</span></div>
                </div>
              )}
            </div>
          )}

          {errorMsg && (
            <div className="p-3 bg-red-50 border border-red-200 rounded text-xs text-red-800 flex items-start gap-2">
              <XCircle className="w-4 h-4 text-red-600 shrink-0 mt-0.5" />
              <span>{errorMsg}</span>
            </div>
          )}
        </div>

        {/* Right Column: Evaluated Plan Display */}
        <div className="lg:col-span-7 space-y-4">
          {currentPlan ? (
            <div className="bg-white border border-slate-200 rounded shadow-sm overflow-hidden">
              {/* Verdict Header Banner */}
              <div
                className={`px-4 py-3.5 border-b flex items-center justify-between ${
                  currentPlan.is_applicable
                    ? 'bg-emerald-50 border-emerald-200'
                    : 'bg-red-50 border-red-200'
                }`}
              >
                <div className="flex items-center gap-2.5">
                  {currentPlan.is_applicable ? (
                    <ShieldCheck className="w-5 h-5 text-emerald-600" />
                  ) : (
                    <ShieldAlert className="w-5 h-5 text-red-600" />
                  )}
                  <div>
                    <div className="text-xs font-semibold uppercase tracking-wider text-slate-900">
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
                  <span
                    className={`px-2 py-0.5 rounded text-[11px] font-medium ${
                      currentPlan.is_applicable
                        ? 'bg-emerald-100 text-emerald-800'
                        : 'bg-red-100 text-red-800'
                    }`}
                  >
                    {currentPlan.is_applicable ? 'Applicable' : 'Blocked'}
                  </span>
                  <span
                    className={`px-2 py-0.5 rounded text-[11px] font-medium ${
                      currentPlan.risk_level === 'Critical'
                        ? 'bg-red-100 text-red-800'
                        : currentPlan.risk_level === 'High'
                        ? 'bg-amber-100 text-amber-800'
                        : 'bg-slate-100 text-slate-700'
                    }`}
                  >
                    Risk: {currentPlan.risk_level}
                  </span>
                </div>
              </div>

              {/* Plan Metadata Grid */}
              <div className="p-4 space-y-4 text-xs">
                <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 p-3 bg-slate-50 rounded border border-slate-100">
                  <div>
                    <span className="text-slate-400 block text-[10px] uppercase font-semibold">Target</span>
                    <span className="font-mono font-medium text-slate-900 truncate block">
                      {currentPlan.target_identifier}
                    </span>
                  </div>
                  <div>
                    <span className="text-slate-400 block text-[10px] uppercase font-semibold">Media Category</span>
                    <span className="text-slate-900 font-medium">{currentPlan.media_type}</span>
                  </div>
                  <div>
                    <span className="text-slate-400 block text-[10px] uppercase font-semibold">Scope</span>
                    <span className="text-slate-900 font-medium">{currentPlan.scope}</span>
                  </div>
                  <div>
                    <span className="text-slate-400 block text-[10px] uppercase font-semibold">Recommended Method</span>
                    <span className="text-slate-900 font-medium">{currentPlan.recommended_method}</span>
                  </div>
                  <div>
                    <span className="text-slate-400 block text-[10px] uppercase font-semibold">Formal Standard</span>
                    <span className="text-slate-900 font-medium">{currentPlan.applicable_standard || 'N/A'}</span>
                  </div>
                  <div>
                    <span className="text-slate-400 block text-[10px] uppercase font-semibold">Verification Strategy</span>
                    <span className="text-slate-900 font-medium">{currentPlan.verification_strategy}</span>
                  </div>
                </div>

                {/* Technical Limitations Section */}
                <div>
                  <h3 className="text-xs font-semibold text-slate-900 mb-2 flex items-center gap-1.5">
                    <Info className="w-3.5 h-3.5 text-slate-500" />
                    Documented Verification Limitations
                  </h3>
                  <ul className="space-y-1.5">
                    {currentPlan.limitations.map((lim, idx) => (
                      <li
                        key={idx}
                        className="p-2 bg-amber-50/60 border border-amber-200/60 rounded text-[11px] text-amber-900 flex items-start gap-2"
                      >
                        <span className="text-amber-500 font-bold shrink-0">•</span>
                        <span>{lim}</span>
                      </li>
                    ))}
                  </ul>
                </div>

                {/* Reason Codes if any */}
                {currentPlan.reason_codes.length > 0 && (
                  <div>
                    <h3 className="text-xs font-semibold text-slate-900 mb-1.5">Reason Telemetry</h3>
                    <div className="flex flex-wrap gap-1.5">
                      {currentPlan.reason_codes.map((rc, idx) => (
                        <span
                          key={idx}
                          className="px-2 py-0.5 font-mono text-[10px] rounded bg-slate-100 text-slate-700 border border-slate-200"
                        >
                          {rc}
                        </span>
                      ))}
                    </div>
                  </div>
                )}

                {/* TOCTOU Live Snapshot Integrity Guard Card */}
                <div className="pt-2 border-t border-slate-200">
                  <div className="flex items-center justify-between">
                    <div>
                      <h4 className="font-semibold text-slate-900 text-xs">Snapshot Integrity & TOCTOU Guard</h4>
                      <p className="text-[11px] text-slate-500">
                        Re-probes hardware to verify device capacity, serial, and geometry have not changed.
                      </p>
                    </div>
                    <button
                      onClick={handleCheckSnapshot}
                      disabled={checkingSnapshot}
                      className="px-3 py-1.5 bg-white border border-slate-300 rounded text-xs font-medium text-slate-700 hover:bg-slate-50 transition disabled:opacity-50 flex items-center gap-1.5"
                    >
                      <RefreshCw className={`w-3 h-3 ${checkingSnapshot ? 'animate-spin' : ''}`} />
                      Verify Live Snapshot
                    </button>
                  </div>

                  {snapshotResult && (
                    <div
                      className={`mt-2.5 p-2.5 rounded border text-xs ${
                        snapshotResult.matches
                          ? 'bg-emerald-50 border-emerald-200 text-emerald-900'
                          : 'bg-red-50 border-red-200 text-red-900'
                      }`}
                    >
                      <div className="flex items-center gap-2 font-medium">
                        {snapshotResult.matches ? (
                          <>
                            <CheckCircle2 className="w-4 h-4 text-emerald-600" />
                            Hardware Match Confirmed: Target geometry and identity unchanged.
                          </>
                        ) : (
                          <>
                            <XCircle className="w-4 h-4 text-red-600" />
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
            </div>
          ) : (
            <div className="bg-white border border-slate-200 rounded p-8 text-center shadow-sm">
              <Cpu className="w-10 h-10 text-slate-300 mx-auto mb-3" />
              <h3 className="text-sm font-semibold text-slate-700">No Plan Evaluated Yet</h3>
              <p className="text-xs text-slate-500 max-w-sm mx-auto mt-1">
                Configure a target storage identifier and scope on the left, then click <strong>Evaluate Sanitization Plan</strong> to generate a deterministic standards plan.
              </p>
            </div>
          )}
        </div>
      </div>

      {/* Standards Registry Reference Modal */}
      {showStandardsModal && (
        <div className="fixed inset-0 z-50 bg-slate-900/40 flex items-center justify-center p-4 backdrop-blur-sm">
          <div className="bg-white border border-slate-200 rounded-lg shadow-xl max-w-2xl w-full max-h-[85vh] flex flex-col">
            <div className="px-5 py-4 border-b border-slate-200 flex items-center justify-between">
              <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-2">
                <BookOpen className="w-4 h-4 text-slate-700" />
                Sanitization Standards & Verification Registry
              </h3>
              <button
                onClick={() => setShowStandardsModal(false)}
                className="text-slate-400 hover:text-slate-700 transition"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="p-5 overflow-y-auto space-y-4 text-xs">
              {standards.map((std) => (
                <div key={std.standard_id} className="p-3.5 border border-slate-200 rounded bg-slate-50/50 space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="font-semibold text-slate-900">{std.standard_name}</span>
                    <span className="px-2 py-0.5 rounded font-mono text-[10px] bg-slate-200 text-slate-800">
                      {std.method_id}
                    </span>
                  </div>
                  <p className="text-slate-600 text-[11px]">{std.description}</p>

                  <div className="grid grid-cols-2 gap-2 text-[11px] pt-1">
                    <div>
                      <span className="text-slate-400 block text-[10px] uppercase font-semibold">Applicable Media</span>
                      <span className="text-slate-800">{std.applicable_media.join(', ')}</span>
                    </div>
                    <div>
                      <span className="text-slate-400 block text-[10px] uppercase font-semibold">Recommended Verification</span>
                      <span className="text-slate-800">{std.recommended_verification}</span>
                    </div>
                  </div>

                  <div className="text-[11px] pt-1">
                    <span className="text-slate-400 block text-[10px] uppercase font-semibold">Verification Requirements</span>
                    <span className="text-slate-700 italic">{std.verification_requirements}</span>
                  </div>

                  {std.limitations.length > 0 && (
                    <div className="text-[11px] pt-1">
                      <span className="text-slate-400 block text-[10px] uppercase font-semibold">Limitations</span>
                      <ul className="list-disc list-inside text-amber-900">
                        {std.limitations.map((l, i) => (
                          <li key={i}>{l}</li>
                        ))}
                      </ul>
                    </div>
                  )}
                </div>
              ))}
            </div>

            <div className="px-5 py-3 border-t border-slate-200 bg-slate-50 flex justify-end">
              <button
                onClick={() => setShowStandardsModal(false)}
                className="px-3.5 py-1.5 bg-slate-900 text-white rounded text-xs font-medium hover:bg-slate-800 transition"
              >
                Close Registry
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
