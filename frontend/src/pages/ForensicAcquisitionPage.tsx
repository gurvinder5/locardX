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
  Button,
  Input,
  Select,
  SelectItem,
  Checkbox,
  Progress,
  Chip,
  Table,
  TableHeader,
  TableColumn,
  TableBody,
  TableRow,
  TableCell,
} from '@heroui/react';
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
    <div className="space-y-6 text-slate-800 dark:text-slate-200 font-sans">
      {/* Active Case Warning Banner if not active */}
      {!isCaseActive && (
        <div className="border-l-2 border-amber-500 bg-amber-500/10 p-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
          <div className="flex items-center gap-3">
            <Briefcase className="w-5 h-5 text-amber-600 dark:text-amber-400 shrink-0" />
            <div>
              <h4 className="text-xs font-bold text-amber-950 dark:text-amber-100 uppercase tracking-wider">
                Active Investigation Case Required
              </h4>
              <p className="text-xs text-amber-800 dark:text-amber-300">
                Forensic disk acquisitions must be cryptographically associated with an open case to maintain chain of custody.
              </p>
            </div>
          </div>
          <Button
            size="sm"
            onPress={() => setShowCaseModal(true)}
            className="bg-amber-600 hover:bg-amber-700 text-white font-bold h-8 shrink-0 shadow-xs text-xs px-3"
          >
            Select or Create Case
          </Button>
        </div>
      )}

      {/* Header with Strict Read-Only Guarantee */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-slate-200 dark:border-[#26313D] pb-4">
        <div>
          <div className="flex items-center gap-2 flex-wrap">
            <Download className="w-5 h-5 text-slate-700 dark:text-slate-300" />
            <h2 className="text-base font-bold text-slate-900 dark:text-white">
              Forensic Disk Acquisition / Bitstream Imaging
            </h2>
            <Chip size="sm" variant="flat" className="font-mono text-[10px] font-bold bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700">
              READ-ONLY INPUT
            </Chip>
          </div>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
            Acquires raw bitstream (DD) disk images from physical devices with simultaneous streaming SHA-256 hash generation.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Chip size="sm" variant="flat" startContent={<ShieldCheck className="w-4 h-4 text-slate-700 dark:text-slate-300" />} className="text-xs font-semibold bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700">
            STRICT READ-ONLY GUARANTEE
          </Chip>
          <Button
            isIconOnly
            size="sm"
            variant="bordered"
            onPress={loadSources}
            isDisabled={loadingSources || isAcquiring}
            className="border-slate-200 dark:border-[#26313D] text-slate-600 dark:text-slate-300"
            title="Refresh physical devices"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loadingSources ? 'animate-spin' : ''}`} />
          </Button>
        </div>
      </div>

      {/* Safety Invariant Notice */}
      <div className="border-l-2 border-slate-400 bg-slate-50 dark:bg-[#10161D] p-3 text-xs text-slate-800 dark:text-slate-200 flex items-start gap-3">
        <Info className="w-4 h-4 text-slate-600 dark:text-slate-400 shrink-0 mt-0.5" />
        <div className="space-y-1">
          <p className="font-semibold text-slate-900 dark:text-white">Forensic Acquisition Pipeline Constraints:</p>
          <ul className="list-disc list-inside space-y-0.5 text-slate-700 dark:text-slate-300">
            <li>Physical device sources only. Logical volume drive letters (e.g. C:\) are strictly rejected.</li>
            <li>Zero write handles requested on the physical device (<code className="font-mono bg-slate-200 dark:bg-[#26313D] px-1 rounded text-slate-800 dark:text-slate-200">GENERIC_READ</code> only).</li>
            <li>Pre-flight collision detection prevents writing the image onto the disk being acquired.</li>
            <li>Single-pass cryptographic SHA-256 verification and fail-closed transaction cleanup.</li>
          </ul>
        </div>
      </div>

      {/* Elevation Requirement Notice */}
      {privilegeStatus && !privilegeStatus.is_elevated && (
        <div className="border-l-2 border-amber-500 bg-amber-500/10 p-3 text-xs text-amber-900 dark:text-amber-200 flex items-start gap-3">
          <AlertCircle className="w-5 h-5 text-amber-600 dark:text-amber-400 shrink-0 mt-0.5" />
          <div className="space-y-1">
            <p className="font-bold text-amber-800 dark:text-amber-200">
              ADMINISTRATIVE PRIVILEGES REQUIRED (Non-Elevated Session Detected)
            </p>
            <p className="text-amber-700 dark:text-amber-300 leading-relaxed">
              Direct raw sector reading from physical storage devices (<code className="font-mono bg-amber-100 dark:bg-amber-900/40 px-1 rounded">\\.\PhysicalDrive1</code>) requires elevated Windows Administrator privileges.
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
          <div className="border border-slate-200 dark:border-[#26313D] p-4 space-y-4 rounded-xs bg-white dark:bg-[#151C24]">
            <div className="flex items-center justify-between border-b border-slate-100 dark:border-[#26313D] pb-3">
              <h3 className="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
                <HardDrive className="w-4 h-4 text-slate-500" />
                <span>1. Select Physical Source Device</span>
              </h3>
              <Chip size="sm" variant="flat" className="font-mono text-[11px] bg-slate-100 dark:bg-[#1B2530] text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]">
                {sources.length} detected
              </Chip>
            </div>

            <div className="space-y-2">
              {loadingSources ? (
                <div className="py-6 text-center text-xs text-slate-400">Probing physical storage hardware...</div>
              ) : sources.length === 0 ? (
                <div className="py-6 text-center text-xs text-amber-600 bg-amber-50 dark:bg-amber-950/20 rounded-lg border border-amber-200 dark:border-amber-800">
                  No storage devices available for acquisition.
                </div>
              ) : (
                sources.map((src) => {
                  const isSelected = src.device_id === selectedSourceId;
                  return (
                    <div
                      key={src.device_id}
                      onClick={() => !isAcquiring && setSelectedSourceId(src.device_id)}
                      className={`p-3 rounded border text-xs cursor-pointer transition-all ${
                        isSelected
                          ? 'border-slate-900 dark:border-white bg-slate-50 dark:bg-[#1B2530]'
                          : 'border-slate-200 dark:border-[#26313D] bg-white dark:bg-[#151C24] hover:border-slate-300 dark:hover:border-slate-600'
                      } ${isAcquiring ? 'opacity-60 cursor-not-allowed' : ''}`}
                    >
                      <div className="flex items-start justify-between">
                        <div className="space-y-1">
                          <div className="flex items-center gap-2 flex-wrap">
                            <span className="font-semibold text-slate-900 dark:text-white">{src.display_name}</span>
                            {src.is_system && (
                              <Chip size="sm" variant="flat" className="text-[10px] font-semibold bg-rose-50 dark:bg-rose-950/40 text-rose-700 dark:text-rose-300 border border-rose-200 dark:border-rose-900 h-4">
                                HOST OS
                              </Chip>
                            )}
                            {src.is_removable && (
                              <Chip size="sm" variant="flat" className="text-[10px] font-semibold bg-amber-50 dark:bg-amber-950/40 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800 h-4">
                                REMOVABLE
                              </Chip>
                            )}
                          </div>
                          <div className="font-mono text-[11px] text-slate-500 dark:text-slate-400 flex items-center gap-3">
                            <span>ID: {src.device_id}</span>
                            {src.serial_number && <span>S/N: {src.serial_number}</span>}
                            <span>Type: {src.media_type}</span>
                          </div>
                        </div>

                        <div className="text-right font-mono font-bold text-slate-800 dark:text-slate-200 text-xs">
                          {formatBytes(src.capacity_bytes)}
                        </div>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Step 2: Destination & Options */}
          <div className="border border-slate-200 dark:border-[#26313D] p-4 space-y-4 rounded-xs bg-white dark:bg-[#151C24]">
            <div className="border-b border-slate-100 dark:border-[#26313D] pb-3">
              <h3 className="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
                <FolderOpen className="w-4 h-4 text-slate-500" />
                <span>2. Destination File & Parameters</span>
              </h3>
            </div>

            <div className="space-y-3.5">
              <Input
                size="sm"
                variant="bordered"
                label="Destination Image File (.raw / .dd)"
                placeholder="e.g. D:\Forensics\evidence_disk.raw"
                value={destinationPath}
                onValueChange={setDestinationPath}
                isDisabled={isAcquiring}
                classNames={{
                  input: 'font-mono text-xs',
                }}
              />

              {/* Destination Validation Feedback */}
              {destinationPath && (
                <div
                  className={`p-2.5 rounded text-xs flex items-center justify-between gap-2 border ${
                    validatingDest
                      ? 'bg-slate-50 dark:bg-[#10161D] text-slate-600 dark:text-slate-400 border-slate-200 dark:border-[#26313D]'
                      : destValid
                      ? 'bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border-slate-300 dark:border-slate-700'
                      : 'bg-rose-50 dark:bg-rose-950/40 text-rose-800 dark:text-rose-200 border border-rose-200 dark:border-rose-900'
                  }`}
                >
                  <div className="flex items-center gap-2">
                    {validatingDest ? (
                      <RefreshCw className="w-3.5 h-3.5 animate-spin shrink-0" />
                    ) : destValid ? (
                      <CheckCircle2 className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300 shrink-0" />
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
                    <Button
                      size="sm"
                      color="danger"
                      onPress={() => {
                        const clean = destinationPath.trim().replace(/[\\/]+$/, '');
                        setDestinationPath(`${clean}\\evidence.raw`);
                      }}
                      className="h-6 text-[10px] px-2"
                    >
                      + Append \evidence.raw
                    </Button>
                  )}
                </div>
              )}

              <div className="grid grid-cols-2 gap-3 pt-1">
                <Select
                  size="sm"
                  variant="bordered"
                  label="Chunk Read Buffer"
                  selectedKeys={[String(chunkSizeBytes)]}
                  onChange={(e) => setChunkSizeBytes(Number(e.target.value))}
                  isDisabled={isAcquiring}
                >
                  <SelectItem key={String(64 * 1024)}>64 KiB</SelectItem>
                  <SelectItem key={String(512 * 1024)}>512 KiB</SelectItem>
                  <SelectItem key={String(1024 * 1024)}>1 MiB (Recommended)</SelectItem>
                  <SelectItem key={String(4 * 1024 * 1024)}>4 MiB (Fast SSD)</SelectItem>
                </Select>

                <div className="flex items-center pt-5">
                  <Checkbox
                    size="sm"
                    isSelected={allowOverwrite}
                    onValueChange={setAllowOverwrite}
                    isDisabled={isAcquiring}
                    classNames={{
                      label: 'text-xs text-slate-700 dark:text-slate-300',
                    }}
                  >
                    Allow Overwrite
                  </Checkbox>
                </div>
              </div>

              {/* Action: Generate Pre-Acquisition Plan */}
              <div className="pt-2">
                <Button
                  size="sm"
                  color="primary"
                  onPress={handleGeneratePlan}
                  isDisabled={!selectedSource || !destValid || isAcquiring}
                  startContent={<FileCheck className="w-4 h-4" />}
                  className="w-full bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold h-9 shadow-xs"
                >
                  Generate Forensic Acquisition Plan
                </Button>
              </div>

              {planningError && (
                <div className="p-2.5 rounded-lg text-xs bg-rose-50 dark:bg-rose-950/40 text-rose-800 dark:text-rose-200 border border-rose-200 dark:border-rose-900 flex items-center gap-2">
                  <XCircle className="w-4 h-4 text-rose-600 shrink-0" />
                  <span>{planningError}</span>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Right Column: Plan Review, Progress & Results */}
        <div className="lg:col-span-6 space-y-6">
          {/* Plan Review */}
          {plan && !isAcquiring && !result && (
            <div className="border border-slate-200 dark:border-[#26313D] p-4 space-y-4 rounded-xs bg-white dark:bg-[#151C24]">
              <div className="flex items-center justify-between border-b border-slate-100 dark:border-[#26313D] pb-3">
                <h3 className="text-xs font-bold uppercase tracking-wider text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
                  <ShieldCheck className="w-4 h-4 text-slate-700 dark:text-slate-300" />
                  <span>3. Review Acquisition Plan</span>
                </h3>
                <span className="font-mono text-[10px] text-slate-400">{plan.plan_id}</span>
              </div>

              <div className="space-y-2 text-xs">
                <div className="flex justify-between py-1 border-b border-slate-50 dark:border-[#1E2732]">
                  <span className="text-slate-500 dark:text-slate-400">Source Device:</span>
                  <span className="font-semibold text-slate-800 dark:text-slate-200">{plan.source.display_name}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-50 dark:border-[#1E2732]">
                  <span className="text-slate-500 dark:text-slate-400">Physical Identifier:</span>
                  <span className="font-mono text-slate-700 dark:text-slate-300">{plan.source.device_id}</span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-50 dark:border-[#1E2732]">
                  <span className="text-slate-500 dark:text-slate-400">Exact Capacity:</span>
                  <span className="font-mono font-bold text-slate-800 dark:text-slate-200">
                    {formatBytes(plan.source.capacity_bytes)} ({plan.source.capacity_bytes.toLocaleString()} bytes)
                  </span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-50 dark:border-[#1E2732]">
                  <span className="text-slate-500 dark:text-slate-400">Target Image Format:</span>
                  <span className="font-mono text-slate-900 dark:text-white font-semibold uppercase">
                    Raw DD Bitstream (.raw)
                  </span>
                </div>
                <div className="flex justify-between py-1 border-b border-slate-50 dark:border-[#1E2732]">
                  <span className="text-slate-500 dark:text-slate-400">Destination File:</span>
                  <span className="font-mono text-slate-700 dark:text-slate-300 truncate max-w-[280px]" title={plan.destination_path}>
                    {plan.destination_path}
                  </span>
                </div>
                <div className="flex justify-between py-1">
                  <span className="text-slate-500 dark:text-slate-400">Hashing Standard:</span>
                  <span className="font-mono font-semibold text-slate-900 dark:text-white">
                    Single-Pass Streaming SHA-256
                  </span>
                </div>

                <div className="pt-3">
                  <Button
                    size="sm"
                    color="primary"
                    onPress={handleStartAcquisition}
                    startContent={<Download className="w-4 h-4" />}
                    className="w-full bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-bold h-10 shadow-xs"
                  >
                    Execute Read-Only Forensic Acquisition
                  </Button>
                </div>
              </div>
            </div>
          )}

          {/* Active Progress Telemetry */}
          {isAcquiring && progress && (
            <div className="border border-slate-200 dark:border-[#26313D] p-4 space-y-4 rounded-xs bg-white dark:bg-[#151C24]">
              <div className="flex items-center justify-between border-b border-slate-100 dark:border-[#26313D] pb-3">
                <div className="flex items-center gap-2">
                  <div className="w-2.5 h-2.5 rounded-full bg-slate-700 dark:bg-slate-300 animate-pulse" />
                  <h3 className="text-xs font-bold uppercase tracking-wider text-slate-900 dark:text-white">
                    Acquisition in Progress
                  </h3>
                </div>
                <span className="font-mono text-xs font-bold text-slate-900 dark:text-white">
                  {progress.percentage.toFixed(1)}%
                </span>
              </div>

              <div className="space-y-4">
                <Progress
                  size="md"
                  value={progress.percentage}
                  color="default"
                  aria-label="Acquisition Progress"
                />

                {/* Telemetry Metrics */}
                <div className="grid grid-cols-3 gap-3 text-xs">
                  <div className="bg-slate-50 dark:bg-[#10161D] p-3 rounded border border-slate-100 dark:border-[#26313D]">
                    <div className="text-[10px] text-slate-500 dark:text-slate-400 flex items-center gap-1">
                      <Gauge className="w-3 h-3 text-slate-700 dark:text-slate-300" />
                      <span>Throughput</span>
                    </div>
                    <div className="font-mono font-bold text-slate-900 dark:text-white mt-1">
                      {progress.throughput_mbps.toFixed(1)} MB/s
                    </div>
                  </div>

                  <div className="bg-slate-50 dark:bg-[#10161D] p-3 rounded border border-slate-100 dark:border-[#26313D]">
                    <div className="text-[10px] text-slate-500 dark:text-slate-400 flex items-center gap-1">
                      <Clock className="w-3 h-3 text-slate-500" />
                      <span>Elapsed</span>
                    </div>
                    <div className="font-mono font-bold text-slate-900 dark:text-white mt-1">
                      {progress.elapsed_seconds.toFixed(1)}s
                    </div>
                  </div>

                  <div className="bg-slate-50 dark:bg-[#10161D] p-3 rounded border border-slate-100 dark:border-[#26313D]">
                    <div className="text-[10px] text-slate-500 dark:text-slate-400 flex items-center gap-1">
                      <Clock className="w-3 h-3 text-slate-500" />
                      <span>Est. Remaining</span>
                    </div>
                    <div className="font-mono font-bold text-slate-900 dark:text-white mt-1">
                      {progress.eta_seconds != null ? `${progress.eta_seconds.toFixed(1)}s` : '--'}
                    </div>
                  </div>
                </div>

                <div className="text-[11px] font-mono text-slate-500 dark:text-slate-400 text-center">
                  Acquired {formatBytes(progress.bytes_acquired)} of {formatBytes(progress.total_bytes)}
                </div>

                <div className="pt-2">
                  <Button
                    size="sm"
                    color="danger"
                    variant="flat"
                    onPress={handleCancelAcquisition}
                    startContent={<StopCircle className="w-4 h-4 text-rose-600" />}
                    className="w-full text-rose-700 dark:text-rose-300 font-semibold h-9"
                  >
                    Cancel Acquisition
                  </Button>
                </div>
              </div>
            </div>
          )}

          {/* Execution Error / Disruption */}
          {executionError && (
            <div className="border border-rose-200 dark:border-rose-900/40 p-4 space-y-2 rounded-xs bg-white dark:bg-[#151C24] text-xs text-rose-900 dark:text-rose-200">
              <div className="flex items-center gap-2 font-bold text-rose-800 dark:text-rose-300">
                <XCircle className="w-4 h-4 text-rose-600" />
                <span>Acquisition Interrupted / Failed</span>
              </div>
              <p className="font-semibold text-rose-950 dark:text-rose-100">{executionError}</p>

              {result?.diagnostics && (
                <div className="mt-2 p-3 bg-white/80 dark:bg-[#10161D] border border-rose-200 dark:border-rose-900 rounded-lg text-[11px] font-mono space-y-1 text-slate-700 dark:text-slate-300">
                  <div className="font-bold text-[10px] text-rose-800 dark:text-rose-400 uppercase tracking-wider mb-1">
                    Structured Diagnostic Telemetry
                  </div>
                  <div>Source: <span className="font-semibold text-slate-900 dark:text-white">{result.diagnostics.source_device_id}</span> ({formatBytes(result.diagnostics.source_size_bytes)})</div>
                  <div>Offset: <span className="font-semibold text-slate-900 dark:text-white">{result.diagnostics.current_byte_offset.toLocaleString()} bytes</span></div>
                  <div>Stage: <span>{result.diagnostics.current_stage}</span></div>
                  {result.diagnostics.win32_error_code !== undefined && result.diagnostics.win32_error_code !== null && (
                    <div className="text-rose-700 dark:text-rose-400 font-bold">Win32 Error Code: {result.diagnostics.win32_error_code}</div>
                  )}
                  <div>Status: <span className="uppercase text-rose-700 dark:text-rose-400 font-bold">{result.diagnostics.final_status}</span></div>
                </div>
              )}

              {result?.cleanup_status === 'CLEANUP_CONFIRMED' && (
                <div className="p-2.5 rounded-lg bg-slate-100 dark:bg-[#1B2530] border border-slate-300 dark:border-slate-700 text-slate-800 dark:text-slate-200 text-[11px] flex items-center gap-2">
                  <CheckCircle2 className="w-4 h-4 text-slate-700 dark:text-slate-300 shrink-0" />
                  <span>Filesystem Verified: Incomplete target artifact cleanly removed.</span>
                </div>
              )}
              {result?.cleanup_status === 'CLEANUP_FAILED' && (
                <div className="p-2.5 rounded-lg bg-amber-50 dark:bg-amber-950/40 border border-amber-300 dark:border-amber-700 text-amber-900 dark:text-amber-200 text-[11px] space-y-1">
                  <div className="font-bold flex items-center gap-1.5 text-amber-800 dark:text-amber-300">
                    <AlertTriangle className="w-3.5 h-3.5 text-amber-600 shrink-0" />
                    <span>Warning: Incomplete Target Artifact Could Not Be Cleaned Up</span>
                  </div>
                  {result.leftover_artifact_path && (
                    <div className="font-mono text-[10px] break-all">
                      Artifact Path: {result.leftover_artifact_path}
                    </div>
                  )}
                </div>
              )}
            </div>
          )}

          {/* Results & Evidential Artifact */}
          {result && result.status === 'Completed' && (
            <div className="border border-slate-200 dark:border-[#26313D] p-4 space-y-4 rounded-xs bg-white dark:bg-[#151C24]">
              <div className="flex items-center justify-between border-b border-slate-100 dark:border-[#26313D] pb-3">
                <div className="flex items-center gap-2">
                  <CheckCircle2 className="w-5 h-5 text-slate-700 dark:text-slate-300" />
                  <h3 className="text-xs font-bold uppercase tracking-wider text-slate-900 dark:text-white">
                    Acquisition Completed & Verified
                  </h3>
                </div>
                <span className="font-mono text-[10px] text-slate-400">{result.acquisition_id}</span>
              </div>

              <div className="space-y-3 text-xs">
                {/* SHA-256 Digest Box */}
                <div className="bg-slate-50 dark:bg-[#10161D] border border-slate-200 dark:border-[#26313D] rounded-lg p-3 space-y-1.5">
                  <div className="flex items-center justify-between text-[11px] text-slate-600 dark:text-slate-400 font-semibold">
                    <span>Cryptographic SHA-256 Hash</span>
                    <Button
                      size="sm"
                      variant="light"
                      onPress={() => handleCopyHash(result.image_sha256)}
                      startContent={<Copy className="w-3 h-3" />}
                      className="h-6 text-[10px] text-slate-600 dark:text-slate-400"
                    >
                      {copiedHash ? 'Copied!' : 'Copy Hash'}
                    </Button>
                  </div>
                  <div className="font-mono text-[11px] text-slate-800 dark:text-slate-200 break-all select-all font-semibold bg-white dark:bg-[#151C24] p-2 rounded border border-slate-200 dark:border-[#26313D]">
                    {result.image_sha256}
                  </div>
                </div>

                <div className="space-y-1.5 pt-1">
                  <div className="flex justify-between py-1 border-b border-slate-50 dark:border-[#1E2732]">
                    <span className="text-slate-500 dark:text-slate-400">Image Size:</span>
                    <span className="font-mono font-bold text-slate-800 dark:text-slate-200">
                      {formatBytes(result.image_size_bytes)} ({result.image_size_bytes.toLocaleString()} bytes)
                    </span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-slate-50 dark:border-[#1E2732]">
                    <span className="text-slate-500 dark:text-slate-400">Average Throughput:</span>
                    <span className="font-mono text-slate-700 dark:text-slate-300">{result.average_throughput_mbps} MB/s</span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-slate-50 dark:border-[#1E2732]">
                    <span className="text-slate-500 dark:text-slate-400">Elapsed Time:</span>
                    <span className="font-mono text-slate-700 dark:text-slate-300">{result.elapsed_seconds}s</span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-slate-50 dark:border-[#1E2732]">
                    <span className="text-slate-500 dark:text-slate-400">Audit Reference:</span>
                    <span className="font-mono text-slate-600 dark:text-slate-400 text-[11px] truncate max-w-[260px]">
                      {result.audit_reference}
                    </span>
                  </div>
                </div>

                {/* Artifact Verification Section */}
                <div className="pt-2 border-t border-slate-100 dark:border-[#26313D] space-y-2">
                  <Button
                    size="sm"
                    variant="bordered"
                    onPress={handleVerifyArtifact}
                    startContent={<ShieldCheck className="w-4 h-4 text-slate-700 dark:text-slate-300" />}
                    className="w-full h-9 border-slate-200 dark:border-[#26313D] text-slate-800 dark:text-slate-200 font-semibold"
                  >
                    Verify Image Disk Integrity On-Demand
                  </Button>

                  {verification && (
                    <div
                      className={`p-2.5 rounded-lg text-xs flex items-center gap-2 border ${
                        verification.verified
                          ? 'bg-slate-100 dark:bg-[#1B2530] text-slate-900 dark:text-white border-slate-300 dark:border-slate-700'
                          : 'bg-rose-50 dark:bg-rose-950/40 text-rose-900 dark:text-rose-200 border-rose-200 dark:border-rose-900'
                      }`}
                    >
                      {verification.verified ? (
                        <CheckCircle2 className="w-4 h-4 text-slate-700 dark:text-slate-300 shrink-0" />
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
            </div>
          )}
        </div>
      </div>

      {/* Evidential Acquisition History */}
      <div className="border border-slate-200 dark:border-[#26313D] rounded-xs overflow-hidden bg-white dark:bg-[#151C24]">
        <div className="p-3.5 border-b border-slate-200 dark:border-[#26313D] flex items-center justify-between bg-slate-50/60 dark:bg-[#10161D]">
          <div className="flex items-center gap-2">
            <Download className="w-4 h-4 text-slate-700 dark:text-slate-300" />
            <h3 className="text-xs font-bold text-slate-800 dark:text-slate-200 uppercase tracking-wider">
              Forensic Acquisition Ledger ({historyRecords.length})
            </h3>
          </div>
          <Button
            size="sm"
            variant="light"
            onPress={loadHistoryRecords}
            isDisabled={loadingHistory}
            startContent={<RefreshCw className={`w-3.5 h-3.5 ${loadingHistory ? 'animate-spin' : ''}`} />}
            className="text-xs text-slate-500 dark:text-slate-400 h-8"
          >
            Refresh
          </Button>
        </div>

        <div>
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
            <Table
              aria-label="Forensic Acquisition Ledger"
              removeWrapper
              classNames={{
                th: 'bg-slate-50 dark:bg-[#10161D] text-slate-600 dark:text-slate-400 text-[11px] font-semibold border-b border-slate-200 dark:border-[#26313D]',
                td: 'py-2.5 px-4 text-xs border-b border-slate-100 dark:border-[#1E2732]',
              }}
            >
              <TableHeader>
                <TableColumn>DATE / TIME</TableColumn>
                <TableColumn>CASE REFERENCE</TableColumn>
                <TableColumn>SOURCE DEVICE</TableColumn>
                <TableColumn>DESTINATION IMAGE</TableColumn>
                <TableColumn>SIZE</TableColumn>
                <TableColumn>SHA-256 DIGEST</TableColumn>
                <TableColumn>STATUS</TableColumn>
              </TableHeader>
              <TableBody>
                {historyRecords.map((rec) => (
                  <TableRow key={rec.acquisition_id} className="hover:bg-slate-50/80 dark:hover:bg-[#1B2530]/50 transition-colors">
                    <TableCell className="font-mono text-slate-600 dark:text-slate-400 whitespace-nowrap text-[11px]">
                      {(() => {
                        const ts = rec.started_at || rec.created_at || rec.completed_at;
                        return ts && !isNaN(Date.parse(ts)) ? new Date(ts).toLocaleString() : '—';
                      })()}
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      {rec.case_id ? (
                        <Chip size="sm" variant="flat" className="font-mono text-[10px] bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700 h-5">
                          {rec.case_id}
                        </Chip>
                      ) : (
                        <span className="text-slate-400 italic text-[11px]">Unassigned</span>
                      )}
                    </TableCell>
                    <TableCell className="max-w-[180px] truncate" title={rec.source_display_name}>
                      <span className="font-mono font-semibold text-[11px] text-slate-900 dark:text-white">{rec.source_device_id}</span>
                      <br />
                      <span className="text-[10px] text-slate-500 dark:text-slate-400 truncate block">{rec.source_display_name}</span>
                    </TableCell>
                    <TableCell className="max-w-[200px] truncate font-mono text-[11px] text-slate-700 dark:text-slate-300" title={rec.destination_path}>
                      {rec.destination_path}
                    </TableCell>
                    <TableCell className="font-mono text-slate-800 dark:text-slate-200 whitespace-nowrap">
                      {formatBytes(rec.image_size_bytes)}
                    </TableCell>
                    <TableCell className="max-w-[150px] font-mono text-[10px] text-slate-600 dark:text-slate-400">
                      <span className="truncate block" title={rec.image_sha256}>
                        {rec.image_sha256 ? `${rec.image_sha256.substring(0, 16)}...` : 'Pending'}
                      </span>
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      <Chip
                        size="sm"
                        variant="flat"
                        className={`text-[10px] font-semibold uppercase h-5 ${
                          rec.status.toLowerCase() === 'completed'
                            ? 'bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700'
                            : rec.status.toLowerCase() === 'failed'
                            ? 'bg-rose-50 dark:bg-rose-950/40 text-rose-700 dark:text-rose-300 border border-rose-200 dark:border-rose-900'
                            : 'bg-amber-50 dark:bg-amber-950/40 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800'
                        }`}
                      >
                        {rec.status}
                      </Chip>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </div>
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
