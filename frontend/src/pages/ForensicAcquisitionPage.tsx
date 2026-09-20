import React, { useEffect, useState, useRef } from 'react';
import {
  ShieldCheck,
  HardDrive,
  Download,
  AlertCircle,
  CheckCircle2,
  XCircle,
  Copy,
  Clock,
  Gauge,
  FileCheck,
  FolderOpen,
  RefreshCw,
  StopCircle,
  Info,
  AlertTriangle,
  Briefcase,
} from 'lucide-react';
import {
  AcquisitionArtifact,
  AcquisitionDeviceSnapshot,
  AcquisitionPlan,
  AcquisitionPrivilegeStatus,
  AcquisitionProgress,
  AcquisitionRecord,
  AcquisitionResult,
  ArtifactVerificationResponse,
  formatAcquisitionError,
} from '../types/acquisition';
import {
  listAcquisitionSources,
  listAcquisitionRecords,
  validateAcquisitionDestination,
  createAcquisitionPlan,
  startAcquisition,
  cancelAcquisition,
  getAcquisitionProgress,
  getAcquisitionArtifact,
  verifyAcquisitionArtifact,
  checkAcquisitionPrivileges,
} from '../services/acquisition';
import { useAuthStore } from '../stores/authStore';
import { useCaseStore } from '../stores/caseStore';
import { ActiveCaseRequiredModal } from '../components/case/ActiveCaseRequiredModal';

export const ForensicAcquisitionPage: React.FC = () => {
  const { sessionToken } = useAuthStore();
  const { activeCase } = useCaseStore();

  const [showCaseModal, setShowCaseModal] = useState(false);
  const [historyRecords, setHistoryRecords] = useState<AcquisitionRecord[]>([]);
  const [loadingHistory, setLoadingHistory] = useState(false);

  // State: Sources
  const [sources, setSources] = useState<AcquisitionDeviceSnapshot[]>([]);
  const [selectedSourceId, setSelectedSourceId] = useState<string>('');
  const [loadingSources, setLoadingSources] = useState<boolean>(true);

  // State: Destination & Settings
  const [destinationPath, setDestinationPath] = useState<string>('D:\\Forensics\\disk_image.raw');
  const [allowOverwrite, setAllowOverwrite] = useState<boolean>(false);
  const [chunkSizeBytes, setChunkSizeBytes] = useState<number>(1024 * 1024);

  // State: Validation & Planning
  const [validatingDest, setValidatingDest] = useState<boolean>(false);
  const [destValidationMessage, setDestValidationMessage] = useState<string | null>(null);
  const [destValid, setDestValid] = useState<boolean | null>(null);
  const [plan, setPlan] = useState<AcquisitionPlan | null>(null);
  const [planningError, setPlanningError] = useState<string | null>(null);

  // State: Execution & Telemetry
  const [isAcquiring, setIsAcquiring] = useState<boolean>(false);
  const [progress, setProgress] = useState<AcquisitionProgress | null>(null);
  const [result, setResult] = useState<AcquisitionResult | null>(null);
  const [artifact, setArtifact] = useState<AcquisitionArtifact | null>(null);
  const [verification, setVerification] = useState<ArtifactVerificationResponse | null>(null);
  const [executionError, setExecutionError] = useState<string | null>(null);
  const [copiedHash, setCopiedHash] = useState<boolean>(false);
  const [privilegeStatus, setPrivilegeStatus] = useState<AcquisitionPrivilegeStatus | null>(null);

  const pollIntervalRef = useRef<number | null>(null);

  const selectedSource = sources.find((s) => s.device_id === selectedSourceId);

  // Check elevation privileges
  const checkPrivileges = async () => {
    try {
      const status = await checkAcquisitionPrivileges();
      setPrivilegeStatus(status);
    } catch (err) {
      console.error('Failed to check acquisition privileges:', err);
    }
  };

  // Load available sources on mount
  const loadSources = async () => {
    try {
      setLoadingSources(true);
      checkPrivileges();
      const devs = await listAcquisitionSources();
      setSources(devs);
      if (devs.length > 0 && !selectedSourceId) {
        // Default to first non-system disk if available
        const preferred = devs.find((d) => !d.is_system) || devs[0];
        setSelectedSourceId(preferred.device_id);
      }
    } catch (err: unknown) {
      console.error('Failed to enumerate acquisition sources:', err);
    } finally {
      setLoadingSources(false);
    }
  };

  const loadHistoryRecords = async () => {
    try {
      setLoadingHistory(true);
      const records = await listAcquisitionRecords();
      setHistoryRecords(records);
    } catch (err) {
      console.error('Failed to load acquisition history:', err);
    } finally {
      setLoadingHistory(false);
    }
  };

  useEffect(() => {
    loadSources();
    checkPrivileges();
    loadHistoryRecords();
  }, []);

  // Reset plan when selected source changes
  useEffect(() => {
    setPlan(null);
    setResult(null);
    setArtifact(null);
    setVerification(null);
    setPlanningError(null);
  }, [selectedSourceId]);

  // Destination validation debounce
  useEffect(() => {
    if (!selectedSource || !destinationPath.trim()) {
      setDestValid(null);
      setDestValidationMessage(null);
      return;
    }

    const timer = setTimeout(async () => {
      try {
        setValidatingDest(true);
        const res = await validateAcquisitionDestination({
          destination_path: destinationPath.trim(),
          source_device_id: selectedSource.device_id,
          required_capacity_bytes: selectedSource.capacity_bytes,
          allow_overwrite: allowOverwrite,
        });
        setDestValid(res.valid);
        setDestValidationMessage(res.message);
      } catch (err: unknown) {
        setDestValid(false);
        setDestValidationMessage(err instanceof Error ? err.message : 'Validation failed');
      } finally {
        setValidatingDest(false);
      }
    }, 400);

    return () => clearTimeout(timer);
  }, [destinationPath, selectedSourceId, allowOverwrite, selectedSource]);

  // Clean polling interval on unmount
  useEffect(() => {
    return () => {
      if (pollIntervalRef.current) {
        window.clearInterval(pollIntervalRef.current);
      }
    };
  }, []);

  // Format bytes helper
  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  const isCaseActive = activeCase && (activeCase.status === 'open' || activeCase.status === 'in_progress');

  // Generate Plan Handler
  const handleGeneratePlan = async () => {
    if (!isCaseActive) {
      setShowCaseModal(true);
      return;
    }
    if (!selectedSource) return;
    try {
      setPlanningError(null);
      setResult(null);
      setArtifact(null);
      setVerification(null);
      const generatedPlan = await createAcquisitionPlan({
        source_device_id: selectedSource.device_id,
        destination_path: destinationPath.trim(),
        chunk_size_bytes: chunkSizeBytes,
        allow_overwrite: allowOverwrite,
        session_token: sessionToken,
      });
      setPlan(generatedPlan);
    } catch (err: unknown) {
      setPlanningError(err instanceof Error ? err.message : 'Failed to generate acquisition plan');
    }
  };

  // Start Acquisition Handler
  const handleStartAcquisition = async () => {
    if (!isCaseActive) {
      setShowCaseModal(true);
      return;
    }
    if (!plan) return;
    setIsAcquiring(true);
    setExecutionError(null);
    const opId = 'op-acq-' + Date.now() + '-' + Math.random().toString(36).substring(2, 9);
    setProgress({
      operation_id: opId,
      bytes_acquired: 0,
      total_bytes: plan.source.capacity_bytes,
      percentage: 0,
      throughput_mbps: 0,
      elapsed_seconds: 0,
      eta_seconds: null,
      stage: 'Initializing read-only physical device stream...',
    });

    // Start polling progress immediately using the generated operation_id
    pollIntervalRef.current = window.setInterval(async () => {
      try {
        const prog = await getAcquisitionProgress(opId);
        if (prog) {
          setProgress(prog);
        }
      } catch {
        // Ignore transient polling glitches
      }
    }, 250);

    try {
      const res = await startAcquisition({
        plan,
        session_token: sessionToken,
        operation_id: opId,
      });
      setResult(res);

      if (res.status === 'Completed') {
        const art = await getAcquisitionArtifact(res.operation_id);
        setArtifact(art);
        loadHistoryRecords();
      } else if (res.status === 'Cancelled') {
        setExecutionError('Forensic acquisition was cancelled by operator.');
      } else {
        const formattedErr = formatAcquisitionError(res.failure_reason);
        setExecutionError(formattedErr);
      }
    } catch (err: unknown) {
      setExecutionError(err instanceof Error ? err.message : 'Acquisition operation failed');
    } finally {
      if (pollIntervalRef.current) {
        window.clearInterval(pollIntervalRef.current);
        pollIntervalRef.current = null;
      }
      setIsAcquiring(false);
      loadHistoryRecords();
    }
  };

  // Cancel Handler
  const handleCancelAcquisition = async () => {
    if (progress?.operation_id && progress.operation_id !== 'pending') {
      await cancelAcquisition(progress.operation_id);
    }
  };

  // Verify Artifact Handler
  const handleVerifyArtifact = async () => {
    if (!artifact) return;
    try {
      const ver = await verifyAcquisitionArtifact(artifact);
      setVerification(ver);
    } catch (err: unknown) {
      console.error('Failed to verify artifact:', err);
    }
  };

  // Copy SHA-256
  const handleCopyHash = (hash: string) => {
    navigator.clipboard.writeText(hash);
    setCopiedHash(true);
    setTimeout(() => setCopiedHash(false), 2000);
  };

  return (
    <div className="space-y-6">
      {/* Active Case Warning Banner if not active */}
      {!isCaseActive && (
        <div className="bg-amber-50 border border-amber-300 rounded-lg p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3 shadow-2xs">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-amber-100 text-amber-800 border border-amber-200 shrink-0">
              <Briefcase className="w-5 h-5" />
            </div>
            <div>
              <h4 className="text-xs font-bold text-amber-950 uppercase tracking-wider">
                Active Investigation Case Required
              </h4>
              <p className="text-xs text-amber-800">
                Forensic disk acquisitions must be cryptographically associated with an open case to maintain chain of custody.
              </p>
            </div>
          </div>
          <button
            onClick={() => setShowCaseModal(true)}
            className="px-3.5 py-2 text-xs font-bold text-amber-900 bg-amber-200 hover:bg-amber-300 border border-amber-300 rounded-lg shadow-xs transition-colors shrink-0"
          >
            Select or Create Case
          </button>
        </div>
      )}

      {/* Header with Strict Read-Only Guarantee */}
      <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-2xs">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
          <div>
            <div className="flex items-center gap-2">
              <Download className="w-5 h-5 text-blue-600" />
              <h2 className="text-base font-bold text-slate-900">
                Forensic Disk Acquisition / Bitstream Imaging
              </h2>
              <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-blue-50 text-blue-700 border border-blue-200">
                PILLAR 3 &bull; READ-ONLY INPUT
              </span>
            </div>
            <p className="text-xs text-slate-500 mt-1">
              Acquires raw bitstream (DD) disk images from physical devices with simultaneous streaming SHA-256 hash generation.
            </p>
          </div>

          <div className="flex items-center gap-2">
            <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-semibold bg-emerald-50 text-emerald-800 border border-emerald-200 shadow-2xs">
              <ShieldCheck className="w-4 h-4 text-emerald-600" />
              <span>STRICT READ-ONLY SOURCE GUARANTEE</span>
            </span>
            <button
              onClick={loadSources}
              disabled={loadingSources || isAcquiring}
              className="p-1.5 rounded-md hover:bg-slate-100 text-slate-600 border border-slate-200 transition-colors"
              title="Refresh physical devices"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${loadingSources ? 'animate-spin' : ''}`} />
            </button>
          </div>
        </div>
      </div>

      {/* Safety Invariant Notice */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 text-xs text-blue-900 flex items-start gap-3">
        <Info className="w-4 h-4 text-blue-600 shrink-0 mt-0.5" />
        <div className="space-y-1">
          <p className="font-semibold">Forensic Acquisition Pipeline Constraints:</p>
          <ul className="list-disc list-inside space-y-0.5 text-blue-800">
            <li>Physical device sources only. Logical volume drive letters (e.g. C:\) are strictly rejected.</li>
            <li>Zero write handles requested on the physical device (<code className="font-mono bg-blue-100 px-1 rounded">GENERIC_READ</code> only).</li>
            <li>Pre-flight collision detection prevents writing the image onto the disk being acquired.</li>
            <li>Single-pass cryptographic SHA-256 verification and fail-closed transaction cleanup.</li>
          </ul>
        </div>
      </div>

      {/* Elevation Requirement Notice */}
      {privilegeStatus && !privilegeStatus.is_elevated && (
        <div className="bg-amber-50 border border-amber-200 rounded-lg p-4 text-xs text-amber-900 flex items-start gap-3 shadow-2xs">
          <AlertCircle className="w-5 h-5 text-amber-600 shrink-0 mt-0.5" />
          <div className="space-y-1">
            <p className="font-bold text-amber-800">
              ADMINISTRATIVE PRIVILEGES REQUIRED (Non-Elevated Session Detected)
            </p>
            <p className="text-amber-700">
              Direct raw sector reading from physical storage devices (<code className="font-mono bg-amber-100 px-1 rounded">\\.\PhysicalDrive1</code>) requires elevated Windows Administrator privileges.
              Reading raw physical devices without elevation fails closed with an Access Denied error (Win32 error 5).
              To acquire physical storage devices, please restart LocardX using <strong>Run as administrator</strong>.
            </p>
          </div>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left Column: Source & Destination Setup */}
        <div className="lg:col-span-6 space-y-6">
          {/* Step 1: Select Physical Source */}
          <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-2xs space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-xs font-bold uppercase tracking-wider text-slate-700 flex items-center gap-1.5">
                <HardDrive className="w-4 h-4 text-slate-500" />
                <span>1. Select Physical Source Device</span>
              </h3>
              <span className="text-[11px] text-slate-400 font-mono">
                {sources.length} device(s) detected
              </span>
            </div>

            {loadingSources ? (
              <div className="py-6 text-center text-xs text-slate-400">Probing physical storage hardware...</div>
            ) : sources.length === 0 ? (
              <div className="py-6 text-center text-xs text-amber-600 bg-amber-50 rounded border border-amber-200">
                No storage devices available for acquisition.
              </div>
            ) : (
              <div className="space-y-2">
                {sources.map((src) => {
                  const isSelected = src.device_id === selectedSourceId;
                  return (
                    <div
                      key={src.device_id}
                      onClick={() => !isAcquiring && setSelectedSourceId(src.device_id)}
                      className={`p-3 rounded-lg border text-xs cursor-pointer transition-all ${
                        isSelected
                          ? 'border-blue-500 bg-blue-50/50 shadow-2xs ring-1 ring-blue-500/20'
                          : 'border-slate-200 bg-white hover:border-slate-300'
                      } ${isAcquiring ? 'opacity-60 cursor-not-allowed' : ''}`}
                    >
                      <div className="flex items-start justify-between">
                        <div className="space-y-1">
                          <div className="flex items-center gap-2">
                            <span className="font-semibold text-slate-800">{src.display_name}</span>
                            {src.is_system && (
                              <span className="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-rose-50 text-rose-700 border border-rose-200">
                                HOST OS
                              </span>
                            )}
                            {src.is_removable && (
                              <span className="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-amber-50 text-amber-700 border border-amber-200">
                                REMOVABLE
                              </span>
                            )}
                          </div>
                          <div className="font-mono text-[11px] text-slate-500 flex items-center gap-3">
                            <span>ID: {src.device_id}</span>
                            {src.serial_number && <span>S/N: {src.serial_number}</span>}
                            <span>Type: {src.media_type}</span>
                          </div>
                        </div>

                        <div className="text-right font-mono font-bold text-slate-700 text-xs">
                          {formatBytes(src.capacity_bytes)}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          {/* Step 2: Destination & Options */}
          <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-2xs space-y-4">
            <h3 className="text-xs font-bold uppercase tracking-wider text-slate-700 flex items-center gap-1.5">
              <FolderOpen className="w-4 h-4 text-slate-500" />
              <span>2. Destination File & Parameters</span>
            </h3>

            <div className="space-y-3">
              <div>
                <label className="block text-[11px] font-semibold text-slate-600 mb-1">
                  Destination Image File (.raw / .dd)
                </label>
                <input
                  type="text"
                  value={destinationPath}
                  disabled={isAcquiring}
                  onChange={(e) => setDestinationPath(e.target.value)}
                  className="w-full px-3 py-2 text-xs font-mono border border-slate-300 rounded-md focus:outline-hidden focus:ring-1 focus:ring-blue-500 focus:border-blue-500 disabled:bg-slate-100"
                  placeholder="e.g. D:\Forensics\evidence_disk.raw"
                />
              </div>

              {/* Destination Validation Feedback */}
              {destinationPath && (
                <div
                  className={`p-2.5 rounded text-xs flex items-center justify-between gap-2 border ${
                    validatingDest
                      ? 'bg-slate-50 text-slate-600 border-slate-200'
                      : destValid
                      ? 'bg-emerald-50 text-emerald-800 border-emerald-200'
                      : 'bg-rose-50 text-rose-800 border-rose-200'
                  }`}
                >
                  <div className="flex items-center gap-2">
                    {validatingDest ? (
                      <RefreshCw className="w-3.5 h-3.5 animate-spin shrink-0" />
                    ) : destValid ? (
                      <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600 shrink-0" />
                    ) : (
                      <AlertCircle className="w-3.5 h-3.5 text-rose-600 shrink-0" />
                    )}
                    <span className="font-medium">
                      {validatingDest
                        ? 'Validating target path and free disk space...'
                        : destValidationMessage || 'Invalid destination'}
                    </span>
                  </div>

                  {!validatingDest && !destValid && (destValidationMessage?.toLowerCase().includes('directory') || destValidationMessage?.toLowerCase().includes('separator')) && (
                    <button
                      type="button"
                      onClick={() => {
                        const clean = destinationPath.trim().replace(/[\\/]+$/, '');
                        setDestinationPath(`${clean}\\evidence.raw`);
                      }}
                      className="px-2 py-1 text-[11px] font-semibold bg-rose-600 hover:bg-rose-700 text-white rounded shadow-xs shrink-0 whitespace-nowrap cursor-pointer transition-colors"
                    >
                      + Append \evidence.raw
                    </button>
                  )}
                </div>
              )}

              <div className="grid grid-cols-2 gap-3 pt-1">
                <div>
                  <label className="block text-[11px] font-semibold text-slate-600 mb-1">
                    Chunk Read Buffer
                  </label>
                  <select
                    value={chunkSizeBytes}
                    disabled={isAcquiring}
                    onChange={(e) => setChunkSizeBytes(Number(e.target.value))}
                    className="w-full px-2.5 py-1.5 text-xs border border-slate-300 rounded-md focus:outline-hidden focus:ring-1 focus:ring-blue-500 disabled:bg-slate-100 font-mono"
                  >
                    <option value={64 * 1024}>64 KiB</option>
                    <option value={512 * 1024}>512 KiB</option>
                    <option value={1024 * 1024}>1 MiB (Recommended)</option>
                    <option value={4 * 1024 * 1024}>4 MiB (Fast SSD)</option>
                  </select>
                </div>

                <div className="flex items-center gap-2 pt-5">
                  <input
                    type="checkbox"
                    id="overwriteCheck"
                    checked={allowOverwrite}
                    disabled={isAcquiring}
                    onChange={(e) => setAllowOverwrite(e.target.checked)}
                    className="rounded border-slate-300 text-blue-600 focus:ring-blue-500"
                  />
                  <label htmlFor="overwriteCheck" className="text-xs text-slate-700 cursor-pointer">
                    Allow Overwriting Existing File
                  </label>
                </div>
              </div>

              {/* Action: Generate Pre-Acquisition Plan */}
              <div className="pt-2">
                <button
                  onClick={handleGeneratePlan}
                  disabled={!selectedSource || !destValid || isAcquiring}
                  className="w-full py-2 px-4 rounded-md text-xs font-semibold bg-blue-600 hover:bg-blue-700 text-white disabled:bg-slate-200 disabled:text-slate-400 transition-colors flex items-center justify-center gap-2 shadow-2xs"
                >
                  <FileCheck className="w-4 h-4" />
                  <span>Generate Forensic Acquisition Plan</span>
                </button>
              </div>

              {planningError && (
                <div className="p-2.5 rounded text-xs bg-rose-50 text-rose-800 border border-rose-200 flex items-center gap-2">
                  <XCircle className="w-4 h-4 text-rose-600 shrink-0" />
                  <span>{planningError}</span>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Right Column: Plan Review, Progress & Results */}
        <div className="lg:col-span-6 space-y-6">
          {/* Plan Review Card */}
          {plan && !isAcquiring && !result && (
            <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-2xs space-y-4">
              <div className="flex items-center justify-between border-b border-slate-100 pb-3">
                <h3 className="text-xs font-bold uppercase tracking-wider text-slate-800 flex items-center gap-1.5">
                  <ShieldCheck className="w-4 h-4 text-blue-600" />
                  <span>3. Review Acquisition Plan</span>
                </h3>
                <span className="font-mono text-[10px] text-slate-400">{plan.plan_id}</span>
              </div>

              <div className="space-y-2 text-xs">
                <div className="flex justify-between py-1 border-b border-slate-50">
                  <span className="text-slate-500">Source Device:</span>
                  <span className="font-semibold text-slate-800">{plan.source.display_name}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-50">
                  <span className="text-slate-500">Physical Identifier:</span>
                  <span className="font-mono text-slate-700">{plan.source.device_id}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-50">
                  <span className="text-slate-500">Exact Capacity:</span>
                  <span className="font-mono font-bold text-slate-800">
                    {formatBytes(plan.source.capacity_bytes)} ({plan.source.capacity_bytes.toLocaleString()} bytes)
                  </span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-50">
                  <span className="text-slate-500">Target Image Format:</span>
                  <span className="font-mono text-blue-700 font-semibold uppercase">
                    Raw DD Bitstream (.raw)
                  </span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-50">
                  <span className="text-slate-500">Destination File:</span>
                  <span className="font-mono text-slate-700 truncate max-w-[280px]" title={plan.destination_path}>
                    {plan.destination_path}
                  </span>
                </div>
                <div className="flex justify-between py-1">
                  <span className="text-slate-500">Hashing Standard:</span>
                  <span className="font-mono font-semibold text-emerald-700">
                    Single-Pass Streaming SHA-256
                  </span>
                </div>
              </div>

              <div className="pt-2">
                <button
                  onClick={handleStartAcquisition}
                  className="w-full py-2.5 px-4 rounded-md text-xs font-bold bg-emerald-600 hover:bg-emerald-700 text-white transition-colors flex items-center justify-center gap-2 shadow-xs"
                >
                  <Download className="w-4 h-4" />
                  <span>Execute Read-Only Forensic Acquisition</span>
                </button>
              </div>
            </div>
          )}

          {/* Active Progress Telemetry */}
          {isAcquiring && progress && (
            <div className="bg-white border border-blue-200 rounded-lg p-5 shadow-sm space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <div className="w-2.5 h-2.5 rounded-full bg-blue-500 animate-pulse" />
                  <h3 className="text-xs font-bold uppercase tracking-wider text-slate-900">
                    Acquisition in Progress
                  </h3>
                </div>
                <span className="font-mono text-xs font-bold text-blue-700">
                  {progress.percentage.toFixed(1)}%
                </span>
              </div>

              {/* Progress Bar */}
              <div className="w-full bg-slate-100 rounded-full h-3 overflow-hidden border border-slate-200">
                <div
                  className="bg-blue-600 h-full transition-all duration-200"
                  style={{ width: `${Math.min(100, Math.max(0, progress.percentage))}%` }}
                />
              </div>

              {/* Telemetry Metrics */}
              <div className="grid grid-cols-3 gap-3 pt-1 text-xs">
                <div className="bg-slate-50 p-2.5 rounded border border-slate-100">
                  <div className="text-[10px] text-slate-500 flex items-center gap-1">
                    <Gauge className="w-3 h-3 text-blue-600" />
                    <span>Throughput</span>
                  </div>
                  <div className="font-mono font-bold text-slate-800 mt-1">
                    {progress.throughput_mbps.toFixed(1)} MB/s
                  </div>
                </div>

                <div className="bg-slate-50 p-2.5 rounded border border-slate-100">
                  <div className="text-[10px] text-slate-500 flex items-center gap-1">
                    <Clock className="w-3 h-3 text-slate-500" />
                    <span>Elapsed</span>
                  </div>
                  <div className="font-mono font-bold text-slate-800 mt-1">
                    {progress.elapsed_seconds.toFixed(1)}s
                  </div>
                </div>

                <div className="bg-slate-50 p-2.5 rounded border border-slate-100">
                  <div className="text-[10px] text-slate-500 flex items-center gap-1">
                    <Clock className="w-3 h-3 text-slate-500" />
                    <span>Est. Remaining</span>
                  </div>
                  <div className="font-mono font-bold text-slate-800 mt-1">
                    {progress.eta_seconds != null ? `${progress.eta_seconds.toFixed(1)}s` : '--'}
                  </div>
                </div>
              </div>

              <div className="text-[11px] font-mono text-slate-500 text-center">
                Acquired {formatBytes(progress.bytes_acquired)} of {formatBytes(progress.total_bytes)}
              </div>

              <div className="pt-2">
                <button
                  onClick={handleCancelAcquisition}
                  className="w-full py-2 px-4 rounded-md text-xs font-semibold bg-rose-50 hover:bg-rose-100 text-rose-700 border border-rose-200 transition-colors flex items-center justify-center gap-2"
                >
                  <StopCircle className="w-4 h-4 text-rose-600" />
                  <span>Cancel Acquisition</span>
                </button>
              </div>
            </div>
          )}

          {/* Execution Error / Disruption */}
          {executionError && (
            <div className="bg-rose-50 border border-rose-200 rounded-lg p-5 shadow-2xs space-y-2 text-xs text-rose-900">
              <div className="flex items-center gap-2 font-bold text-rose-800">
                <XCircle className="w-4 h-4 text-rose-600" />
                <span>Acquisition Interrupted / Failed</span>
              </div>
              <p className="font-semibold text-rose-950">{executionError}</p>

              {result?.diagnostics && (
                <div className="mt-2 p-3 bg-white/80 border border-rose-200 rounded text-[11px] font-mono space-y-1 text-slate-700">
                  <div className="font-bold text-[10px] text-rose-800 uppercase tracking-wider mb-1">
                    Structured Diagnostic Telemetry
                  </div>
                  <div>Source: <span className="font-semibold text-slate-900">{result.diagnostics.source_device_id}</span> ({formatBytes(result.diagnostics.source_size_bytes)})</div>
                  <div>Offset: <span className="font-semibold text-slate-900">{result.diagnostics.current_byte_offset.toLocaleString()} bytes</span></div>
                  <div>Stage: <span className="text-slate-800">{result.diagnostics.current_stage}</span></div>
                  {result.diagnostics.win32_error_code !== undefined && result.diagnostics.win32_error_code !== null && (
                    <div className="text-rose-700 font-bold">Win32 Error Code: {result.diagnostics.win32_error_code}</div>
                  )}
                  <div>Status: <span className="uppercase text-rose-700 font-bold">{result.diagnostics.final_status}</span></div>
                </div>
              )}

              {result?.cleanup_status === 'CLEANUP_CONFIRMED' && (
                <div className="p-2.5 rounded bg-emerald-50 border border-emerald-200 text-emerald-800 text-[11px] flex items-center gap-2">
                  <CheckCircle2 className="w-4 h-4 text-emerald-600 shrink-0" />
                  <span>Filesystem Verified: Incomplete target artifact was cleanly removed to prevent evidence contamination.</span>
                </div>
              )}
              {result?.cleanup_status === 'CLEANUP_FAILED' && (
                <div className="p-2.5 rounded bg-amber-50 border border-amber-300 text-amber-900 text-[11px] space-y-1">
                  <div className="font-bold flex items-center gap-1.5 text-amber-800">
                    <AlertTriangle className="w-3.5 h-3.5 text-amber-600 shrink-0" />
                    <span>Warning: Incomplete Target Artifact Could Not Be Cleaned Up</span>
                  </div>
                  {result.leftover_artifact_path && (
                    <div className="font-mono text-[10px] break-all">
                      Artifact Path: {result.leftover_artifact_path}
                    </div>
                  )}
                  {result.cleanup_error && (
                    <div className="text-[10px] text-amber-800">
                      Error: {result.cleanup_error}
                    </div>
                  )}
                  <p className="text-[10px] text-amber-700">
                    Operator action required: Please inspect and manually delete this partial file before re-attempting acquisition.
                  </p>
                </div>
              )}
              {result?.cleanup_status === 'CLEANUP_NOT_REQUIRED' && (
                <p className="text-[11px] text-slate-500 pt-1">
                  Validation failed before any disk stream was opened; no artifacts were created on disk.
                </p>
              )}
              {!result?.cleanup_status && (
                <p className="text-[11px] text-rose-700 pt-1">
                  Integrity invariant: Acquisition stream aborted.
                </p>
              )}
            </div>
          )}

          {/* Results & Evidential Artifact Card */}
          {result && result.status === 'Completed' && (
            <div className="bg-white border border-emerald-200 rounded-lg p-5 shadow-2xs space-y-4">
              <div className="flex items-center justify-between border-b border-emerald-100 pb-3">
                <div className="flex items-center gap-2">
                  <CheckCircle2 className="w-5 h-5 text-emerald-600" />
                  <h3 className="text-xs font-bold uppercase tracking-wider text-emerald-900">
                    Acquisition Completed & Verified
                  </h3>
                </div>
                <span className="font-mono text-[10px] text-slate-400">{result.acquisition_id}</span>
              </div>

              <div className="space-y-3 text-xs">
                {/* SHA-256 Digest Box */}
                <div className="bg-slate-50 border border-slate-200 rounded-md p-3 space-y-1.5">
                  <div className="flex items-center justify-between text-[11px] text-slate-600 font-semibold">
                    <span>Cryptographic SHA-256 Hash</span>
                    <button
                      onClick={() => handleCopyHash(result.image_sha256)}
                      className="text-blue-600 hover:text-blue-700 flex items-center gap-1 font-sans text-[10px]"
                    >
                      <Copy className="w-3 h-3" />
                      <span>{copiedHash ? 'Copied!' : 'Copy Hash'}</span>
                    </button>
                  </div>
                  <div className="font-mono text-[11px] text-slate-800 break-all select-all font-semibold bg-white p-2 rounded border border-slate-200">
                    {result.image_sha256}
                  </div>
                </div>

                <div className="space-y-1.5 pt-1">
                  <div className="flex justify-between py-1 border-b border-slate-50">
                    <span className="text-slate-500">Image Size:</span>
                    <span className="font-mono font-bold text-slate-800">
                      {formatBytes(result.image_size_bytes)} ({result.image_size_bytes.toLocaleString()} bytes)
                    </span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-slate-50">
                    <span className="text-slate-500">Average Throughput:</span>
                    <span className="font-mono text-slate-700">{result.average_throughput_mbps} MB/s</span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-slate-50">
                    <span className="text-slate-500">Elapsed Time:</span>
                    <span className="font-mono text-slate-700">{result.elapsed_seconds}s</span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-slate-50">
                    <span className="text-slate-500">Audit Reference:</span>
                    <span className="font-mono text-slate-600 text-[11px] truncate max-w-[260px]">
                      {result.audit_reference}
                    </span>
                  </div>
                  <div className="flex justify-between py-1">
                    <span className="text-slate-500">Recovery Handoff:</span>
                    <span className="px-2 py-0.5 rounded text-[10px] font-mono font-semibold bg-emerald-50 text-emerald-800 border border-emerald-200">
                      AcquisitionArtifact Ready
                    </span>
                  </div>
                </div>
              </div>

              {/* Artifact Verification Section */}
              <div className="pt-2 border-t border-slate-100 space-y-2">
                <button
                  onClick={handleVerifyArtifact}
                  className="w-full py-2 px-3 rounded-md text-xs font-semibold bg-slate-100 hover:bg-slate-200 text-slate-800 transition-colors flex items-center justify-center gap-2 border border-slate-200"
                >
                  <ShieldCheck className="w-4 h-4 text-emerald-600" />
                  <span>Verify Image Disk Integrity On-Demand</span>
                </button>

                {verification && (
                  <div
                    className={`p-2.5 rounded text-xs flex items-center gap-2 border ${
                      verification.verified
                        ? 'bg-emerald-50 text-emerald-900 border-emerald-200'
                        : 'bg-rose-50 text-rose-900 border-rose-200'
                    }`}
                  >
                    {verification.verified ? (
                      <CheckCircle2 className="w-4 h-4 text-emerald-600 shrink-0" />
                    ) : (
                      <XCircle className="w-4 h-4 text-rose-600 shrink-0" />
                    )}
                    <span className="font-medium">
                      {verification.verified
                        ? 'Bitstream integrity verified: On-disk SHA-256 matches acquisition record.'
                        : verification.error_message || 'Integrity verification failed!'}
                    </span>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Evidential Acquisition History */}
      <div className="bg-white border border-slate-200 rounded-lg shadow-2xs overflow-hidden">
        <div className="p-4 border-b border-slate-200 flex items-center justify-between bg-slate-50/60">
          <div className="flex items-center gap-2">
            <Download className="w-4 h-4 text-blue-600" />
            <h3 className="text-xs font-bold text-slate-800 uppercase tracking-wider">
              Forensic Acquisition Ledger ({historyRecords.length})
            </h3>
          </div>
          <button
            onClick={loadHistoryRecords}
            disabled={loadingHistory}
            className="text-xs text-slate-500 hover:text-slate-800 flex items-center gap-1 font-medium transition-colors"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loadingHistory ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </div>

        {loadingHistory ? (
          <div className="p-8 text-center text-slate-400 text-xs flex items-center justify-center gap-2">
            <RefreshCw className="w-4 h-4 animate-spin text-slate-400" />
            Loading acquisition ledger...
          </div>
        ) : historyRecords.length === 0 ? (
          <div className="p-8 text-center text-slate-400 text-xs">
            No past acquisition records found in local forensic database.
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-left text-xs border-collapse">
              <thead>
                <tr className="bg-slate-50 text-[11px] font-semibold text-slate-600 border-b border-slate-200">
                  <th className="py-2.5 px-4">Date / Time</th>
                  <th className="py-2.5 px-4">Case Reference</th>
                  <th className="py-2.5 px-4">Source Device</th>
                  <th className="py-2.5 px-4">Destination Image</th>
                  <th className="py-2.5 px-4">Size</th>
                  <th className="py-2.5 px-4">SHA-256 Digest</th>
                  <th className="py-2.5 px-4">Status</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100 font-mono text-[11px]">
                {historyRecords.map((rec) => (
                  <tr key={rec.acquisition_id} className="hover:bg-slate-50/80 transition-colors">
                    <td className="py-2.5 px-4 text-slate-600 whitespace-nowrap">
                      {new Date(rec.created_at).toLocaleString()}
                    </td>
                    <td className="py-2.5 px-4 font-semibold text-sky-800 whitespace-nowrap">
                      {rec.case_id ? (
                        <span className="px-2 py-0.5 rounded bg-sky-50 text-sky-800 border border-sky-200 text-[10px]">
                          {rec.case_id}
                        </span>
                      ) : (
                        <span className="text-slate-400 font-normal italic">Unassigned</span>
                      )}
                    </td>
                    <td className="py-2.5 px-4 text-slate-800 font-sans max-w-[180px] truncate" title={rec.source_display_name}>
                      <span className="font-mono font-semibold text-[11px]">{rec.source_device_id}</span>
                      <br />
                      <span className="text-[10px] text-slate-500 truncate block">{rec.source_display_name}</span>
                    </td>
                    <td className="py-2.5 px-4 text-slate-700 max-w-[200px] truncate" title={rec.destination_path}>
                      {rec.destination_path}
                    </td>
                    <td className="py-2.5 px-4 text-slate-800 whitespace-nowrap">
                      {formatBytes(rec.image_size_bytes)}
                    </td>
                    <td className="py-2.5 px-4 text-slate-600 max-w-[150px]">
                      <span className="font-mono text-[10px] truncate block" title={rec.image_sha256}>
                        {rec.image_sha256 ? `${rec.image_sha256.substring(0, 16)}...` : 'Pending'}
                      </span>
                    </td>
                    <td className="py-2.5 px-4 whitespace-nowrap">
                      <span className={`px-2 py-0.5 rounded text-[10px] font-semibold uppercase ${
                        rec.status.toLowerCase() === 'completed'
                          ? 'bg-emerald-50 text-emerald-700 border border-emerald-200'
                          : rec.status.toLowerCase() === 'failed'
                          ? 'bg-rose-50 text-rose-700 border border-rose-200'
                          : 'bg-amber-50 text-amber-700 border border-amber-200'
                      }`}>
                        {rec.status}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <ActiveCaseRequiredModal
        isOpen={showCaseModal}
        onClose={() => setShowCaseModal(false)}
        operationName="Forensic Disk Acquisition"
      />
    </div>
  );
};

export default ForensicAcquisitionPage;
