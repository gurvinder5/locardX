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
} from 'lucide-react';
import {
  AcquisitionArtifact,
  AcquisitionDeviceSnapshot,
  AcquisitionPlan,
  AcquisitionProgress,
  AcquisitionResult,
  ArtifactVerificationResponse,
} from '../types/acquisition';
import {
  listAcquisitionSources,
  validateAcquisitionDestination,
  createAcquisitionPlan,
  startAcquisition,
  cancelAcquisition,
  getAcquisitionProgress,
  getAcquisitionArtifact,
  verifyAcquisitionArtifact,
} from '../services/acquisition';
import { useAuthStore } from '../stores/authStore';

export const ForensicAcquisitionPage: React.FC = () => {
  const { sessionToken } = useAuthStore();

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

  const pollIntervalRef = useRef<number | null>(null);

  const selectedSource = sources.find((s) => s.device_id === selectedSourceId);

  // Load available sources on mount
  const loadSources = async () => {
    try {
      setLoadingSources(true);
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

  useEffect(() => {
    loadSources();
  }, []);

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

  // Generate Plan Handler
  const handleGeneratePlan = async () => {
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
    if (!plan) return;
    setIsAcquiring(true);
    setExecutionError(null);
    setProgress({
      operation_id: 'pending',
      bytes_acquired: 0,
      total_bytes: plan.source.capacity_bytes,
      percentage: 0,
      throughput_mbps: 0,
      elapsed_seconds: 0,
      eta_seconds: null,
      stage: 'Initializing read-only physical device stream...',
    });

    // Start polling progress
    let activeOpId: string | null = null;
    pollIntervalRef.current = window.setInterval(async () => {
      if (!activeOpId) return;
      try {
        const prog = await getAcquisitionProgress(activeOpId);
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
      });
      activeOpId = res.operation_id;
      setResult(res);

      if (res.status === 'Completed') {
        const art = await getAcquisitionArtifact(res.operation_id);
        setArtifact(art);
      } else if (res.status === 'Cancelled') {
        setExecutionError('Forensic acquisition was cancelled by operator. Target image cleaned up.');
      } else {
        setExecutionError(
          res.failure_reason?.message || 'Forensic acquisition failed. Image discarded.'
        );
      }
    } catch (err: unknown) {
      setExecutionError(err instanceof Error ? err.message : 'Acquisition operation failed');
    } finally {
      if (pollIntervalRef.current) {
        window.clearInterval(pollIntervalRef.current);
        pollIntervalRef.current = null;
      }
      setIsAcquiring(false);
    }
  };

  // Cancel Handler
  const handleCancelAcquisition = async () => {
    if (progress?.operation_id) {
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
                  className={`p-2.5 rounded text-xs flex items-center gap-2 border ${
                    validatingDest
                      ? 'bg-slate-50 text-slate-600 border-slate-200'
                      : destValid
                      ? 'bg-emerald-50 text-emerald-800 border-emerald-200'
                      : 'bg-rose-50 text-rose-800 border-rose-200'
                  }`}
                >
                  {validatingDest ? (
                    <RefreshCw className="w-3.5 h-3.5 animate-spin" />
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
              <p>{executionError}</p>
              <p className="text-[11px] text-rose-700">
                Integrity invariant preserved: The incomplete target file was immediately removed to prevent partial evidence contamination.
              </p>
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
    </div>
  );
};

export default ForensicAcquisitionPage;
