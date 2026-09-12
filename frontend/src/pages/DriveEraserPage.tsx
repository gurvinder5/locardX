import React, { useState, useEffect } from 'react';
import {
  Cpu,
  ShieldAlert,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Copy,
  Check,
  RotateCcw,
  FileCheck,
  Download,
  ShieldCheck,
  Layers,
  ArrowRight,
  Loader2,
  Info,
  Lock,
  RefreshCw,
} from 'lucide-react';
import { StorageDeviceDto } from '../types/device';
import {
  DriveCapabilities,
  DriveCapabilitiesAssessment,
  DriveErasePlan,
  DriveEraseProgress,
  DriveEraseResult,
  ExecutionMode,
  DrivePrivilegeStatus,
  DriveSanitizationReport,
} from '../types/driveEraser';
import {
  getMockTestDevices,
  getRealStorageDevices,
  refreshRealDevices,
  getDriveCapabilities,
  assessDriveHardwareCapabilities,
  planDriveErasure,
  executeDriveErasureSimulation,
  executeDriveErasureHardware,
  checkDriveEraserPrivileges,
  generateDriveErasureReport,
} from '../services/driveEraser';
import { requestDestructiveConfirmation } from '../services/safety';
import { useAuthStore } from '../stores/authStore';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

export const DriveEraserPage: React.FC = () => {
  const sessionToken = useAuthStore((s) => s.sessionToken);

  // Device Discovery & Mode Selection
  const [deviceSource, setDeviceSource] = useState<'real' | 'mock'>('real');
  const [executionMode, setExecutionMode] = useState<ExecutionMode>('Simulation');
  const [devices, setDevices] = useState<StorageDeviceDto[]>([]);
  const [loadingDevices, setLoadingDevices] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>('');
  const [customDeviceId, setCustomDeviceId] = useState<string>('');

  // Capabilities & Plan
  const [capabilities, setCapabilities] = useState<DriveCapabilities | null>(null);
  const [assessment, setAssessment] = useState<DriveCapabilitiesAssessment | null>(null);
  const [, setLoadingCaps] = useState(false);
  const [plan, setPlan] = useState<DriveErasePlan | null>(null);
  const [isPlanning, setIsPlanning] = useState(false);

  // Execution & Simulation
  const [isExecuting, setIsExecuting] = useState(false);
  const [simulationProgress, setSimulationProgress] = useState<DriveEraseProgress | null>(null);
  const [result, setResult] = useState<DriveEraseResult | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Privilege Verification & Reporting State
  const [privilegeStatus, setPrivilegeStatus] = useState<DrivePrivilegeStatus | null>(null);
  const [report, setReport] = useState<DriveSanitizationReport | null>(null);
  const [showReportModal, setShowReportModal] = useState(false);
  const [loadingReport, setLoadingReport] = useState(false);
  const [copiedDigest, setCopiedDigest] = useState(false);

  // Two-Stage Confirmation Modal
  const [showConfirmModal, setShowConfirmModal] = useState(false);
  const [confirmationId, setConfirmationId] = useState<string>('');
  const [typedConfirmation, setTypedConfirmation] = useState('');
  const [warningAcknowledged, setWarningAcknowledged] = useState(false);
  const [copiedText, setCopiedText] = useState(false);

  // Check administrative privileges on mount and mode change
  useEffect(() => {
    let isMounted = true;
    async function loadPrivileges() {
      try {
        const status = await checkDriveEraserPrivileges();
        if (isMounted) {
          setPrivilegeStatus(status);
        }
      } catch {
        // Non-blocking in simulation
      }
    }
    loadPrivileges();
    return () => {
      isMounted = false;
    };
  }, [executionMode, deviceSource]);

  // Load storage devices based on active mode (Real vs Mock)
  useEffect(() => {
    let isMounted = true;
    async function loadDevices() {
      try {
        setLoadingDevices(true);
        setErrorMessage(null);
        const list =
          deviceSource === 'real'
            ? await getRealStorageDevices()
            : await getMockTestDevices();

        if (isMounted) {
          setDevices(list);
          if (list.length > 0) {
            const nonSys = list.find(
              (d) =>
                !d.is_system_device &&
                d.classification !== 'system_device' &&
                d.classification !== 'boot_device'
            );
            const defaultTarget = nonSys ? nonSys.device_id : list[0].device_id;
            setSelectedDeviceId(defaultTarget);
            setCustomDeviceId(defaultTarget);
          } else {
            setSelectedDeviceId('');
            setCustomDeviceId('');
          }
        }
      } catch (err: any) {
        if (isMounted) {
          setErrorMessage(err?.message || 'Failed to load storage devices.');
        }
      } finally {
        if (isMounted) {
          setLoadingDevices(false);
        }
      }
    }
    loadDevices();
    return () => {
      isMounted = false;
    };
  }, [deviceSource]);

  // Refresh devices handler
  const handleRefresh = async () => {
    try {
      setIsRefreshing(true);
      setErrorMessage(null);
      const list =
        deviceSource === 'real'
          ? await refreshRealDevices()
          : await getMockTestDevices();
      setDevices(list);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Failed to refresh devices.');
    } finally {
      setIsRefreshing(false);
    }
  };

  // Fetch capabilities & assessment when device selection changes
  useEffect(() => {
    let isMounted = true;
    const target = customDeviceId.trim() || selectedDeviceId;
    if (!target) {
      setCapabilities(null);
      setAssessment(null);
      return;
    }

    // Check logical drive rejection immediately
    if (
      target.includes(':\\') ||
      target.endsWith(':') ||
      (target.length === 2 && target[1] === ':')
    ) {
      setCapabilities(null);
      setAssessment(null);
      return;
    }

    async function loadCaps() {
      try {
        setLoadingCaps(true);
        const assessed = await assessDriveHardwareCapabilities(target);
        if (isMounted) {
          setAssessment(assessed);
          setCapabilities(assessed.capabilities);
        }
      } catch {
        if (isMounted) {
          setAssessment(null);
          // Fallback to basic capability probing
          getDriveCapabilities(target)
            .then((caps) => {
              if (isMounted) setCapabilities(caps);
            })
            .catch(() => {
              if (isMounted) setCapabilities(null);
            });
        }
      } finally {
        if (isMounted) {
          setLoadingCaps(false);
        }
      }
    }

    loadCaps();
    return () => {
      isMounted = false;
    };
  }, [selectedDeviceId, customDeviceId]);

  const activeDevice = devices.find(
    (d) => d.device_id.toLowerCase() === (customDeviceId.trim() || selectedDeviceId).toLowerCase()
  );

  const isSystemOrBoot =
    activeDevice?.is_system_device ||
    activeDevice?.classification === 'system_device' ||
    activeDevice?.classification === 'boot_device' ||
    (customDeviceId.toLowerCase().includes('physicaldrive0') || customDeviceId.toLowerCase().includes('physicaldrive6'));

  const isLogicalVolume =
    customDeviceId.includes(':\\') ||
    customDeviceId.endsWith(':') ||
    (customDeviceId.length === 2 && customDeviceId[1] === ':') ||
    (customDeviceId.startsWith('/') && !customDeviceId.startsWith('/dev/'));

  // Handle Plan Generation
  const handleGeneratePlan = async () => {
    setErrorMessage(null);
    setResult(null);

    const target = customDeviceId.trim() || selectedDeviceId;
    if (!target) {
      setErrorMessage('Please select or specify a valid physical device.');
      return;
    }

    if (isLogicalVolume) {
      setErrorMessage(
        `Target '${target}' is a logical volume or drive letter. Whole-disk erasure strictly requires selecting a physical device (e.g. \\\\.\\PhysicalDrive1).`
      );
      return;
    }

    if (isSystemOrBoot) {
      setErrorMessage(
        `Target '${target}' is recognized as an active System or Boot device. Sanitization is unconditionally blocked.`
      );
      return;
    }

    try {
      setIsPlanning(true);
      const planned = await planDriveErasure({
        target_device_id: target,
        requested_method: null,
        execution_mode: executionMode,
        session_token: sessionToken,
      });
      setPlan(planned);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Failed to generate drive erasure plan.');
      setPlan(null);
    } finally {
      setIsPlanning(false);
    }
  };

  // Open Confirmation Modal
  const handleOpenConfirmation = async () => {
    if (!plan) return;
    setErrorMessage(null);

    try {
      const challenge = await requestDestructiveConfirmation({
        operation_id: `op-drive-${Date.now()}`,
        target_type: 'PhysicalDevice',
        target_identifier: plan.physical_device_id,
        target_display_name: plan.display_name,
        target_size_bytes: plan.capacity_bytes,
        operation_type: plan.execution_mode === 'RealHardware' ? 'DriveErasure' : 'DriveErasure',
        session_token: sessionToken || '',
      });

      setConfirmationId(challenge.confirmation_id);
      setTypedConfirmation('');
      setWarningAcknowledged(false);
      setShowConfirmModal(true);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Failed to generate safety confirmation challenge.');
    }
  };

  // Execute Erasure (Real Hardware or Simulation)
  const handleExecute = async () => {
    if (!plan || !confirmationId) return;

    try {
      setIsExecuting(true);
      setShowConfirmModal(false);
      setErrorMessage(null);

      const isHardware = plan.execution_mode === 'RealHardware';

      // Live progress tracking
      setSimulationProgress({
        operation_id: `op-drive-${Date.now()}`,
        percentage: 10,
        bytes_processed: 0,
        total_bytes: plan.capacity_bytes,
        current_pass: 1,
        total_passes: plan.passes,
        current_stage: isHardware
          ? 'Acquiring exclusive device lock & pre-execution validation'
          : 'Simulating block overwrite',
        elapsed_seconds: 0.1,
        eta_seconds: isHardware ? null : 1.5,
      });

      const interval = setInterval(() => {
        setSimulationProgress((prev) => {
          if (!prev || prev.percentage >= 95) return prev;
          const nextPct = prev.percentage + 15;
          return {
            ...prev,
            percentage: nextPct,
            bytes_processed: Math.floor((nextPct / 100) * plan.capacity_bytes),
            current_stage: isHardware
              ? nextPct > 70
                ? 'Forensic post-sanitization readback verification'
                : 'Writing sanitization patterns to physical sectors'
              : 'Simulating block overwrite',
            elapsed_seconds: parseFloat((prev.elapsed_seconds + 0.2).toFixed(1)),
          };
        });
      }, 200);

      const res = isHardware
        ? await executeDriveErasureHardware({
            plan_id: plan.plan_id,
            confirmation_id: confirmationId,
            operation_id: `op-drive-hw-${Date.now()}`,
            typed_confirmation: typedConfirmation.trim(),
            warning_acknowledged: warningAcknowledged,
            session_token: sessionToken || 'hw-session',
          })
        : await executeDriveErasureSimulation({
            plan_id: plan.plan_id,
            confirmation_id: confirmationId,
            operation_id: `op-drive-sim-${Date.now()}`,
            typed_confirmation: typedConfirmation.trim(),
            warning_acknowledged: warningAcknowledged,
            session_token: sessionToken || 'sim-session',
          });

      clearInterval(interval);
      setSimulationProgress({
        operation_id: res.operation_id,
        percentage: 100,
        bytes_processed: res.bytes_processed,
        total_bytes: plan.capacity_bytes,
        current_pass: plan.passes,
        total_passes: plan.passes,
        current_stage: 'Sanitization & verification complete',
        elapsed_seconds: res.elapsed_seconds,
        eta_seconds: 0,
      });

      setResult(res);
    } catch (err: any) {
      setErrorMessage(
        err?.message ||
          (plan.execution_mode === 'RealHardware'
            ? 'Real hardware drive erasure failed.'
            : 'Drive erasure simulation failed.')
      );
    } finally {
      setIsExecuting(false);
    }
  };

  const handleReset = () => {
    setPlan(null);
    setResult(null);
    setSimulationProgress(null);
    setErrorMessage(null);
    setTypedConfirmation('');
    setWarningAcknowledged(false);
    setReport(null);
    setShowReportModal(false);
  };

  const handleCopyCertificate = () => {
    if (!result) return;
    navigator.clipboard.writeText(JSON.stringify(result, null, 2));
    setCopiedText(true);
    setTimeout(() => setCopiedText(false), 2000);
  };

  const handleViewReport = async () => {
    if (!result) return;
    try {
      setLoadingReport(true);
      setShowReportModal(true);
      const rep = await generateDriveErasureReport(result.operation_id, sessionToken || undefined);
      setReport(rep);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Failed to generate sanitization report.');
    } finally {
      setLoadingReport(false);
    }
  };

  const handleCopyReportDigest = () => {
    if (!report) return;
    navigator.clipboard.writeText(report.integrity.report_digest);
    setCopiedDigest(true);
    setTimeout(() => setCopiedDigest(false), 2000);
  };

  const handleDownloadReport = () => {
    if (!report) return;
    const content = `# LOCARDX FORENSIC DRIVE SANITIZATION CERTIFICATE
Report ID: ${report.report_id}
Operation ID: ${report.operation_id}
Generated: ${report.integrity.generated_at}
Execution Mode: ${report.operation_info.execution_mode}
${report.operation_info.is_simulation ? '*** SIMULATION ONLY - NO PHYSICAL MEDIA WAS SANITIZED ***\n' : ''}
## Target Device
Device ID: ${report.device_info.physical_device_id}
Display Name: ${report.device_info.display_name}
Vendor: ${report.device_info.vendor || 'N/A'}
Model: ${report.device_info.model || 'N/A'}
Serial Number: ${report.device_info.serial_number || 'N/A'}
Capacity: ${formatBytes(report.device_info.capacity_bytes)} (${report.device_info.capacity_bytes} bytes)
Sector Size: ${report.device_info.sector_size} bytes
Bus Type: ${report.device_info.bus_type || 'N/A'}

## Sanitization Execution
Method: ${report.operation_info.sanitization_method}
Start Time: ${report.operation_info.started_at}
Completion Time: ${report.operation_info.completed_at}
Elapsed Time: ${report.operation_info.elapsed_seconds.toFixed(2)}s
Status: ${report.execution_metrics.status}
Bytes Processed: ${formatBytes(report.execution_metrics.bytes_processed)}

## Verification
Strategy: ${report.verification.strategy}
Outcome: ${report.verification.outcome}
Details: ${report.verification.details}
Evidence Digest (SHA-256): ${report.verification.evidence_digest}

## Tamper-Evident Integrity
Canonical Report Digest (SHA-256): ${report.integrity.report_digest}
Audit Chain Reference: ${report.integrity.audit_chain_reference}
`;

    const blob = new Blob([content], { type: 'text/markdown;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `sanitization-certificate-${report.operation_id}.md`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-6">
      {/* Privilege Status Warning for Real Hardware */}
      {executionMode === 'RealHardware' && privilegeStatus && !privilegeStatus.is_elevated && (
        <div className="bg-amber-50 border-2 border-amber-500 rounded-md p-4 flex items-start space-x-3 shadow-xs">
          <AlertTriangle className="w-6 h-6 text-amber-600 shrink-0 mt-0.5" />
          <div className="space-y-1">
            <div className="flex items-center space-x-2">
              <span className="font-bold text-amber-950 tracking-wide text-xs uppercase bg-amber-200 px-2 py-0.5 rounded-sm">
                ADMINISTRATIVE PRIVILEGES REQUIRED
              </span>
              <span className="text-xs font-semibold text-amber-800">
                (Non-Elevated Session Detected)
              </span>
            </div>
            <p className="text-xs text-amber-900 leading-relaxed">
              {privilegeStatus.message} Real hardware device locking and sanitization execution will fail closed with an Access Denied error. To execute real hardware sanitization, restart LocardX with elevated administrator / root privileges.
            </p>
          </div>
        </div>
      )}

      {/* Execution Mode Top Banner */}
      {executionMode === 'RealHardware' ? (
        <div className="bg-rose-50 border-2 border-rose-400 rounded-md p-4 flex items-start space-x-3 shadow-xs">
          <AlertTriangle className="w-6 h-6 text-rose-600 shrink-0 mt-0.5" />
          <div className="space-y-1">
            <div className="flex items-center space-x-2">
              <span className="font-bold text-rose-950 tracking-wide text-xs uppercase bg-rose-200/80 px-2 py-0.5 rounded-sm">
                PRODUCTION HARDWARE BACKEND ACTIVE
              </span>
              <span className="text-xs font-semibold text-rose-800">
                (Step 10B.3 Real Hardware Sanitizer Boundary)
              </span>
            </div>
            <p className="text-xs text-rose-900 leading-relaxed">
              Real hardware execution is enabled. Authorizing sanitization against an approved physical drive will permanently overwrite sectors or dispatch controller sanitize firmware routines. Active OS, boot, EFI, and logical volumes remain hard-blocked by 15-point pre-execution safety invariants.
            </p>
          </div>
        </div>
      ) : (
        <div className="bg-amber-50 border-2 border-amber-300 rounded-md p-4 flex items-start space-x-3 shadow-xs">
          <ShieldAlert className="w-6 h-6 text-amber-600 shrink-0 mt-0.5" />
          <div className="space-y-1">
            <div className="flex items-center space-x-2">
              <span className="font-bold text-amber-950 tracking-wide text-xs uppercase bg-amber-200/80 px-2 py-0.5 rounded-sm">
                SIMULATION MODE — NO PHYSICAL STORAGE MODIFICATION
              </span>
              <span className="text-xs font-semibold text-amber-800">
                (Safe Verification Harness)
              </span>
            </div>
            <p className="text-xs text-amber-900 leading-relaxed">
              All write operations, sector overwrites, and controller commands are simulated deterministically. No physical media or host workstation sectors are modified.
            </p>
          </div>
        </div>
      )}

      {/* Page Header */}
      <div className="border-b border-slate-200 pb-4">
        <h1 className="text-xl font-bold text-slate-900 flex items-center gap-2">
          <Cpu className="w-5 h-5 text-slate-700" />
          Secure Drive Eraser
        </h1>
        <p className="text-xs text-slate-500 mt-1">
          Whole-disk media-aware sanitization engine supporting HDD, SATA SSD, NVMe, USB, and memory cards.
        </p>
      </div>

      {/* Error Alert */}
      {errorMessage && (
        <div className="bg-rose-50 border border-rose-200 text-rose-800 rounded-md p-3.5 flex items-start space-x-3 text-xs">
          <AlertTriangle className="w-4 h-4 text-rose-600 shrink-0 mt-0.5" />
          <div className="flex-1">
            <span className="font-semibold">Safety Boundary Block: </span>
            {errorMessage}
          </div>
        </div>
      )}

      {/* Workflow Step 1: Device Selection */}
      <div className="bg-white border border-slate-200 rounded-md shadow-xs p-5 space-y-4">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-slate-100 pb-3">
          <div className="flex items-center space-x-2">
            <span className="w-5 h-5 rounded-full bg-slate-900 text-white text-[10px] font-bold flex items-center justify-center">
              1
            </span>
            <h2 className="text-sm font-bold text-slate-900">Physical Storage Device Selection</h2>
          </div>

          <div className="flex flex-wrap items-center gap-2.5">
            {/* Device Source: Real vs Mock */}
            <div className="inline-flex rounded-md p-0.5 bg-slate-100 border border-slate-200 text-xs">
              <button
                type="button"
                onClick={() => {
                  setDeviceSource('real');
                  setPlan(null);
                }}
                className={`px-3 py-1 rounded-sm font-semibold transition-all cursor-pointer ${
                  deviceSource === 'real'
                    ? 'bg-white text-blue-700 shadow-2xs font-bold'
                    : 'text-slate-600 hover:text-slate-900'
                }`}
              >
                Real Storage Devices
              </button>
              <button
                type="button"
                onClick={() => {
                  setDeviceSource('mock');
                  setExecutionMode('Simulation');
                  setPlan(null);
                }}
                className={`px-3 py-1 rounded-sm font-semibold transition-all cursor-pointer ${
                  deviceSource === 'mock'
                    ? 'bg-white text-purple-700 shadow-2xs font-bold'
                    : 'text-slate-600 hover:text-slate-900'
                }`}
              >
                Mock Test Devices
              </button>
            </div>

            {/* Execution Mode Toggle */}
            <div className="inline-flex rounded-md p-0.5 bg-slate-100 border border-slate-200 text-xs">
              <button
                type="button"
                onClick={() => {
                  setExecutionMode('Simulation');
                  setPlan(null);
                }}
                className={`px-2.5 py-1 rounded-sm font-semibold transition-all cursor-pointer ${
                  executionMode === 'Simulation'
                    ? 'bg-white text-amber-800 shadow-2xs font-bold'
                    : 'text-slate-600 hover:text-slate-900'
                }`}
              >
                Simulation
              </button>
              <button
                type="button"
                disabled={deviceSource === 'mock'}
                onClick={() => {
                  setExecutionMode('RealHardware');
                  setPlan(null);
                }}
                title={deviceSource === 'mock' ? 'Mock devices only support Simulation mode' : 'Direct physical disk sanitization'}
                className={`px-2.5 py-1 rounded-sm font-semibold transition-all cursor-pointer ${
                  executionMode === 'RealHardware'
                    ? 'bg-rose-600 text-white shadow-2xs font-bold'
                    : deviceSource === 'mock'
                    ? 'text-slate-400 opacity-40 cursor-not-allowed'
                    : 'text-rose-700 hover:text-rose-900'
                }`}
              >
                Real Hardware
              </button>
            </div>

            {/* Refresh Button */}
            <button
              type="button"
              onClick={handleRefresh}
              disabled={isRefreshing || loadingDevices}
              title="Refresh device enumeration"
              className="p-1.5 text-slate-500 hover:text-slate-800 hover:bg-slate-100 rounded-md border border-slate-200 transition-all cursor-pointer disabled:opacity-50 flex items-center gap-1 text-xs"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${isRefreshing ? 'animate-spin' : ''}`} />
              <span className="hidden sm:inline text-[11px] font-medium">Refresh</span>
            </button>

            <span className="text-[11px] text-slate-500 font-mono">
              {devices.length} Target(s)
            </span>
          </div>
        </div>

        {loadingDevices ? (
          <div className="py-6 text-center text-slate-400 flex items-center justify-center gap-2 text-xs">
            <Loader2 className="w-4 h-4 animate-spin" />
            Discovering platform storage devices...
          </div>
        ) : (
          <div className="space-y-3">
            {/* Quick select buttons */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2.5">
              {devices.map((d) => {
                const isSystem =
                  d.is_system_device || d.classification === 'system_device';
                const isBoot = d.classification === 'boot_device';
                const isSelected =
                  (customDeviceId.trim() || selectedDeviceId).toLowerCase() ===
                  d.device_id.toLowerCase();

                return (
                  <button
                    key={d.device_id}
                    type="button"
                    onClick={() => {
                      setSelectedDeviceId(d.device_id);
                      setCustomDeviceId(d.device_id);
                      setPlan(null);
                      setResult(null);
                      setErrorMessage(null);
                    }}
                    className={`text-left p-3 rounded-md border text-xs transition-all relative ${
                      isSelected
                        ? 'border-blue-600 bg-blue-50/50 shadow-2xs ring-1 ring-blue-500'
                        : 'border-slate-200 hover:border-slate-300 bg-slate-50/40'
                    }`}
                  >
                    <div className="flex items-center justify-between gap-1">
                      <span className="font-mono font-bold text-slate-800 truncate">{d.device_id}</span>
                      <span
                        className={`text-[9px] font-bold px-1.5 py-0.5 rounded-xs uppercase tracking-wider shrink-0 ${
                          deviceSource === 'real'
                            ? 'bg-blue-100 text-blue-800'
                            : 'bg-purple-100 text-purple-800'
                        }`}
                      >
                        {deviceSource === 'real' ? 'Real Device' : 'Mock Device'}
                      </span>
                    </div>

                    <p className="text-[11px] text-slate-600 font-medium truncate mt-1">
                      {d.display_name}
                    </p>

                    <div className="flex items-center justify-between mt-2 pt-1 border-t border-slate-100">
                      <span className="text-[10px] text-slate-400 font-mono">
                        {formatBytes(d.capacity_bytes)}
                      </span>

                      {isSystem ? (
                        <span className="bg-rose-100 text-rose-800 text-[9px] font-bold px-1.5 py-0.5 rounded-xs flex items-center gap-1">
                          <Lock className="w-2.5 h-2.5" />
                          SYSTEM DEVICE
                        </span>
                      ) : isBoot ? (
                        <span className="bg-amber-100 text-amber-800 text-[9px] font-bold px-1.5 py-0.5 rounded-xs flex items-center gap-1">
                          <Lock className="w-2.5 h-2.5" />
                          BOOT DEVICE
                        </span>
                      ) : (
                        <span className="bg-slate-200 text-slate-700 text-[9px] font-semibold px-1.5 py-0.5 rounded-xs uppercase">
                          {d.device_type}
                        </span>
                      )}
                    </div>
                  </button>
                );
              })}
            </div>

            {/* Custom Device Path Input */}
            <div className="pt-2">
              <label className="block text-xs font-semibold text-slate-700 mb-1">
                Target Physical Device Path
              </label>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={customDeviceId}
                  onChange={(e) => {
                    setCustomDeviceId(e.target.value);
                    setPlan(null);
                    setResult(null);
                    setErrorMessage(null);
                  }}
                  placeholder="e.g. \\.\PhysicalDrive1 or /dev/sdb"
                  className={`flex-1 px-3 py-2 text-xs font-mono border rounded-md focus:outline-hidden focus:ring-1 ${
                    isLogicalVolume
                      ? 'border-rose-300 bg-rose-50/50 text-rose-900 focus:ring-rose-400'
                      : isSystemOrBoot
                      ? 'border-amber-300 bg-amber-50/50 text-amber-900 focus:ring-amber-400'
                      : 'border-slate-300 bg-white text-slate-900 focus:ring-blue-500'
                  }`}
                />
                <button
                  type="button"
                  disabled={isPlanning || !customDeviceId.trim() || isSystemOrBoot || isLogicalVolume}
                  onClick={handleGeneratePlan}
                  className="bg-slate-900 hover:bg-slate-800 text-white font-semibold text-xs px-4 py-2 rounded-md disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-1.5 cursor-pointer"
                >
                  {isPlanning ? (
                    <>
                      <Loader2 className="w-3.5 h-3.5 animate-spin" />
                      Evaluating...
                    </>
                  ) : (
                    <>
                      Generate Plan
                      <ArrowRight className="w-3.5 h-3.5" />
                    </>
                  )}
                </button>
              </div>

              {/* Input validation banners */}
              {isLogicalVolume && (
                <p className="text-[11px] text-rose-600 font-medium mt-1.5 flex items-center gap-1">
                  <XCircle className="w-3.5 h-3.5 shrink-0" />
                  Target rejected: '{customDeviceId}' is a logical volume or drive letter. Physical drive identifier required.
                </p>
              )}
              {isSystemOrBoot && !isLogicalVolume && (
                <p className="text-[11px] text-rose-600 font-medium mt-1.5 flex items-center gap-1">
                  <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                  Hard Block: '{customDeviceId}' is recognized as an active OS system or boot device. Erasure is forbidden.
                </p>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Step 2: Hardware Capabilities Assessment */}
      {capabilities && (
        <div className="bg-white border border-slate-200 rounded-md shadow-xs p-5 space-y-3">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-slate-100 pb-3">
            <div className="flex items-center space-x-2">
              <span className="w-5 h-5 rounded-full bg-slate-900 text-white text-[10px] font-bold flex items-center justify-center">
                2
              </span>
              <h2 className="text-sm font-bold text-slate-900">Hardware Capability Assessment</h2>
            </div>

            <div className="flex items-center gap-2">
              {assessment && (
                <>
                  {assessment.overall_state === 'Supported' && (
                    <span className="bg-emerald-100 text-emerald-800 text-[10px] font-bold px-2 py-0.5 rounded-xs flex items-center gap-1">
                      <CheckCircle2 className="w-3 h-3 text-emerald-600" />
                      Capability Supported
                    </span>
                  )}
                  {assessment.overall_state === 'Unknown' && (
                    <span className="bg-amber-100 text-amber-800 text-[10px] font-bold px-2 py-0.5 rounded-xs flex items-center gap-1">
                      <AlertTriangle className="w-3 h-3 text-amber-600" />
                      Capability Unknown (Fails Closed)
                    </span>
                  )}
                  {assessment.overall_state === 'DetectionFailed' && (
                    <span className="bg-rose-100 text-rose-800 text-[10px] font-bold px-2 py-0.5 rounded-xs flex items-center gap-1">
                      <XCircle className="w-3 h-3 text-rose-600" />
                      Capability Detection Failed (Fails Closed)
                    </span>
                  )}
                </>
              )}
              <span className="text-[11px] text-slate-500 font-mono">
                Bus: {capabilities.interface_bus} | Sector: {capabilities.sector_size}B
              </span>
            </div>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
            <div className="p-2.5 bg-slate-50 border border-slate-200 rounded-xs">
              <span className="text-[10px] font-semibold text-slate-500 uppercase block">Media Mechanism</span>
              <span className="font-medium text-slate-800">
                {capabilities.is_rotational ? 'Magnetic Platter (HDD)' : 'Solid State (NAND Flash)'}
              </span>
            </div>
            <div className="p-2.5 bg-slate-50 border border-slate-200 rounded-xs">
              <span className="text-[10px] font-semibold text-slate-500 uppercase block">Hardware Overwrite</span>
              <span className={`font-medium ${capabilities.supports_overwrite ? 'text-emerald-700' : 'text-slate-500'}`}>
                {capabilities.supports_overwrite ? 'Supported (LBA Range)' : 'Not Supported / Read-Only'}
              </span>
            </div>
            <div className="p-2.5 bg-slate-50 border border-slate-200 rounded-xs">
              <span className="text-[10px] font-semibold text-slate-500 uppercase block">Firmware Sanitize</span>
              <span className={`font-medium ${capabilities.supports_firmware_sanitize ? 'text-emerald-700' : 'text-slate-500'}`}>
                {capabilities.supports_firmware_sanitize ? 'Supported' : 'Not Supported'}
              </span>
            </div>
            <div className="p-2.5 bg-slate-50 border border-slate-200 rounded-xs">
              <span className="text-[10px] font-semibold text-slate-500 uppercase block">Crypto Key Erase</span>
              <span className={`font-medium ${capabilities.supports_crypto_erase ? 'text-emerald-700' : 'text-slate-500'}`}>
                {capabilities.supports_crypto_erase ? 'Supported (Hardware)' : 'Not Available'}
              </span>
            </div>
          </div>

          {assessment && assessment.assessment_notes.length > 0 && (
            <div className="bg-slate-50 border border-slate-200 p-2.5 rounded-xs text-[11px] text-slate-600 space-y-0.5">
              <span className="font-semibold text-slate-700 block text-[10px] uppercase">Assessment Telemetry:</span>
              {assessment.assessment_notes.map((note, idx) => (
                <p key={idx} className="font-mono text-[10px]">{note}</p>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Step 3: Media-Aware Plan Display */}
      {plan && !result && (
        <div className="bg-white border border-slate-200 rounded-md shadow-xs p-5 space-y-4">
          <div className="flex items-center justify-between border-b border-slate-100 pb-3">
            <div className="flex items-center space-x-2">
              <span className="w-5 h-5 rounded-full bg-slate-900 text-white text-[10px] font-bold flex items-center justify-center">
                3
              </span>
              <h2 className="text-sm font-bold text-slate-900">Media-Aware Sanitization & Verification Plan</h2>
            </div>
            <span className="bg-blue-100 text-blue-800 text-[10px] font-bold px-2 py-0.5 rounded-xs">
              PLAN ID: {plan.plan_id.substring(0, 16)}...
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs">
            <div className="space-y-2 border border-slate-200 p-3 rounded-xs bg-slate-50/50">
              <h3 className="font-semibold text-slate-800 flex items-center gap-1.5 text-xs">
                <Layers className="w-3.5 h-3.5 text-blue-600" />
                Sanitization Specifications
              </h3>
              <div className="space-y-1 text-[11px] text-slate-700">
                <p><span className="font-semibold text-slate-500">Target Device:</span> {plan.physical_device_id} ({plan.display_name})</p>
                <p><span className="font-semibold text-slate-500">Method:</span> <strong className="text-slate-900">{plan.method}</strong></p>
                <p><span className="font-semibold text-slate-500">Passes:</span> {plan.passes} pass(es)</p>
                <p><span className="font-semibold text-slate-500">Execution Mode:</span> <strong className="text-amber-700 font-bold">{plan.execution_mode}</strong></p>
                <p><span className="font-semibold text-slate-500">Capacity:</span> {formatBytes(plan.capacity_bytes)} ({plan.capacity_bytes} bytes)</p>
              </div>
            </div>

            <div className="space-y-2 border border-slate-200 p-3 rounded-xs bg-slate-50/50">
              <h3 className="font-semibold text-slate-800 flex items-center gap-1.5 text-xs">
                <FileCheck className="w-3.5 h-3.5 text-emerald-600" />
                Post-Sanitization Verification Plan
              </h3>
              <div className="space-y-1 text-[11px] text-slate-700">
                <p><span className="font-semibold text-slate-500">Strategy:</span> {plan.verification_plan.strategy}</p>
                <p><span className="font-semibold text-slate-500">Coverage:</span> {plan.verification_plan.sample_percentage ? `${plan.verification_plan.sample_percentage}% LBA read` : 'Controller Firmware Register'}</p>
                <p><span className="font-semibold text-slate-500">Requirements:</span> {plan.verification_plan.requirements}</p>
              </div>
            </div>
          </div>

          {/* Physical Limitations */}
          {plan.limitations.length > 0 && (
            <div className="bg-amber-50/60 border border-amber-200 p-3 rounded-xs text-xs space-y-1">
              <span className="font-bold text-amber-900 text-[11px] uppercase tracking-wider flex items-center gap-1">
                <Info className="w-3.5 h-3.5 text-amber-700" />
                Physical Media & Hardware Limitations
              </span>
              <ul className="list-disc list-inside text-[11px] text-amber-800 space-y-0.5">
                {plan.limitations.map((lim, idx) => (
                  <li key={idx}>{lim}</li>
                ))}
              </ul>
            </div>
          )}

          {/* Action to proceed */}
          <div className="flex justify-end gap-2 pt-2">
            <button
              type="button"
              onClick={handleReset}
              className="px-3 py-1.5 border border-slate-200 text-slate-600 rounded-md text-xs hover:bg-slate-50 cursor-pointer"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={handleOpenConfirmation}
              className={`text-white font-semibold text-xs px-4 py-2 rounded-md shadow-xs flex items-center gap-1.5 cursor-pointer ${
                plan.execution_mode === 'RealHardware'
                  ? 'bg-rose-700 hover:bg-rose-800'
                  : 'bg-blue-600 hover:bg-blue-700'
              }`}
            >
              {plan.execution_mode === 'RealHardware' ? 'Authorize Real Hardware Sanitization' : 'Authorize Simulation'}
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      )}

      {/* Step 4: Live Execution Progress */}
      {isExecuting && simulationProgress && (
        <div className="bg-white border border-slate-200 rounded-md shadow-xs p-5 space-y-4">
          <div className="flex items-center justify-between border-b border-slate-100 pb-3">
            <div className="flex items-center space-x-2">
              <Loader2 className={`w-4 h-4 animate-spin ${plan?.execution_mode === 'RealHardware' ? 'text-rose-600' : 'text-blue-600'}`} />
              <h2 className="text-sm font-bold text-slate-900">
                {plan?.execution_mode === 'RealHardware'
                  ? 'Executing Real Hardware Sanitization'
                  : 'Simulating Drive Sanitization'}
              </h2>
            </div>
            <span className="text-[11px] font-mono text-slate-500">
              Pass {simulationProgress.current_pass} of {simulationProgress.total_passes}
            </span>
          </div>

          <div className="space-y-2">
            <div className="flex justify-between text-xs text-slate-600 font-medium">
              <span>{simulationProgress.current_stage}</span>
              <span className="font-mono">{simulationProgress.percentage.toFixed(0)}%</span>
            </div>
            <div className="w-full bg-slate-100 h-2.5 rounded-full overflow-hidden">
              <div
                className={`h-full transition-all duration-300 rounded-full ${
                  plan?.execution_mode === 'RealHardware' ? 'bg-rose-600' : 'bg-blue-600'
                }`}
                style={{ width: `${simulationProgress.percentage}%` }}
              />
            </div>
            <div className="flex justify-between text-[11px] font-mono text-slate-400">
              <span>
                {plan?.execution_mode === 'RealHardware' ? 'Processed' : 'Simulated'}:{' '}
                {formatBytes(simulationProgress.bytes_processed)} / {formatBytes(simulationProgress.total_bytes)}
              </span>
              <span>Elapsed: {simulationProgress.elapsed_seconds}s</span>
            </div>
          </div>
        </div>
      )}

      {/* Step 5: Final Result & Forensic Verification Certificate */}
      {result && (
        <div className="bg-white border border-slate-200 rounded-md shadow-xs p-6 space-y-5">
          <div className="flex items-center justify-between border-b border-slate-100 pb-4">
            <div className="flex items-center space-x-3">
              {result.status === 'Completed' && result.verification.outcome === 'Verified' ? (
                <div className="w-10 h-10 rounded-full bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-600">
                  <CheckCircle2 className="w-6 h-6" />
                </div>
              ) : result.status === 'VerificationFailed' ? (
                <div className="w-10 h-10 rounded-full bg-rose-50 border border-rose-200 flex items-center justify-center text-rose-600">
                  <XCircle className="w-6 h-6" />
                </div>
              ) : result.status === 'UnableToVerify' ? (
                <div className="w-10 h-10 rounded-full bg-amber-50 border border-amber-200 flex items-center justify-center text-amber-600">
                  <AlertTriangle className="w-6 h-6" />
                </div>
              ) : (
                <div className="w-10 h-10 rounded-full bg-rose-50 border border-rose-200 flex items-center justify-center text-rose-600">
                  <XCircle className="w-6 h-6" />
                </div>
              )}

              <div>
                <h2 className="text-base font-bold text-slate-900">
                  {result.status === 'Completed'
                    ? result.execution_mode === 'RealHardware'
                      ? 'Real Hardware Drive Sanitization Completed'
                      : 'Drive Erasure Simulation Completed'
                    : result.status === 'VerificationFailed'
                    ? 'Drive Sanitization Verification Failed'
                    : result.status === 'UnableToVerify'
                    ? 'Drive Sanitization Verification Inconclusive (Fail-Closed)'
                    : 'Drive Sanitization Failed'}
                </h2>
                <p className="text-xs text-slate-500">
                  {result.status === 'Completed'
                    ? 'Forensic post-sanitization verification confirmed. Operation logged to tamper-evident audit chain.'
                    : result.status === 'VerificationFailed'
                    ? 'Post-sanitization readback verification detected non-null sectors. Media CANNOT be cleared.'
                    : result.status === 'UnableToVerify'
                    ? 'Verification could not conclusively read back sectors. Fail-closed: Media CANNOT be cleared.'
                    : result.failure_reason || 'Sanitization failed pre-execution checks or encountered an I/O fault.'}
                </p>
              </div>
            </div>

            <span
              className={`text-xs font-bold px-2.5 py-1 rounded-sm uppercase tracking-wide ${
                result.status === 'Completed'
                  ? 'bg-emerald-100 text-emerald-800'
                  : result.status === 'UnableToVerify'
                  ? 'bg-amber-100 text-amber-800'
                  : 'bg-rose-100 text-rose-800'
              }`}
            >
              {result.status}
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-xs">
            <div className="p-3 bg-slate-50 border border-slate-200 rounded-xs space-y-1">
              <span className="text-[10px] font-semibold text-slate-500 uppercase block">Target Device</span>
              <p className="font-mono font-bold text-slate-900">{result.physical_device_id}</p>
              <p className="text-slate-600">{result.display_name}</p>
            </div>
            <div className="p-3 bg-slate-50 border border-slate-200 rounded-xs space-y-1">
              <span className="text-[10px] font-semibold text-slate-500 uppercase block">Sanitization Method</span>
              <p className="font-bold text-slate-900">{result.method}</p>
              <span
                className={`inline-block text-[10px] font-bold px-1.5 py-0.5 rounded-xs mt-0.5 ${
                  result.execution_mode === 'RealHardware'
                    ? 'bg-rose-100 text-rose-800 border border-rose-200'
                    : 'bg-amber-100 text-amber-800 border border-amber-200'
                }`}
              >
                Mode: {result.execution_mode}
              </span>
            </div>
            <div className="p-3 bg-slate-50 border border-slate-200 rounded-xs space-y-1">
              <span className="text-[10px] font-semibold text-slate-500 uppercase block">Verification Outcome</span>
              <p
                className={`font-bold flex items-center gap-1 ${
                  result.verification.outcome === 'Verified'
                    ? 'text-emerald-700'
                    : result.verification.outcome === 'UnableToVerify'
                    ? 'text-amber-700'
                    : 'text-rose-700'
                }`}
              >
                {result.verification.outcome === 'Verified' ? (
                  <Check className="w-3.5 h-3.5" />
                ) : (
                  <AlertTriangle className="w-3.5 h-3.5" />
                )}
                {result.verification.outcome}
              </p>
              <p className="text-[11px] text-slate-600">{result.verification.strategy}</p>
            </div>
          </div>

          <div className="border border-slate-200 rounded-xs p-3 bg-slate-50 text-xs space-y-2 font-mono">
            <div className="flex justify-between text-[11px] text-slate-500 border-b border-slate-200 pb-1">
              <span>OPERATION AUDIT REFERENCE</span>
              <span>{result.operation_id}</span>
            </div>
            <div className="text-[11px] text-slate-600 space-y-0.5">
              <p>Processed: {formatBytes(result.bytes_processed)} in {result.elapsed_seconds.toFixed(2)}s</p>
              <p>Verification Details: {result.verification.details}</p>
              <p>Audit Events Emitted: {result.audit_references.join(', ')}</p>
            </div>
          </div>

          <div className="flex justify-between items-center pt-2">
            <button
              type="button"
              onClick={handleReset}
              className="px-4 py-2 border border-slate-300 text-slate-700 hover:bg-slate-50 text-xs font-semibold rounded-md flex items-center gap-1.5 cursor-pointer"
            >
              <RotateCcw className="w-3.5 h-3.5" />
              Plan Another Drive
            </button>
            <div className="flex items-center space-x-2">
              <button
                type="button"
                onClick={handleViewReport}
                disabled={loadingReport}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white text-xs font-semibold rounded-md flex items-center gap-1.5 cursor-pointer"
              >
                {loadingReport ? (
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <FileCheck className="w-3.5 h-3.5" />
                )}
                View Sanitization Certificate
              </button>
              <button
                type="button"
                onClick={handleCopyCertificate}
                className="px-4 py-2 bg-slate-900 hover:bg-slate-800 text-white text-xs font-semibold rounded-md flex items-center gap-1.5 cursor-pointer"
              >
                {copiedText ? (
                  <>
                    <Check className="w-3.5 h-3.5" />
                    Copied JSON
                  </>
                ) : (
                  <>
                    <Copy className="w-3.5 h-3.5" />
                    Copy Forensic Record
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Two-Stage Confirmation Modal */}
      {showConfirmModal && plan && (
        <div className="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4">
          <div className="bg-white border border-slate-200 rounded-lg max-w-lg w-full p-6 shadow-xl space-y-5">
            {plan.execution_mode === 'RealHardware' ? (
              <>
                <div className="flex items-center space-x-3 text-rose-600 border-b border-slate-100 pb-3">
                  <AlertTriangle className="w-6 h-6 shrink-0" />
                  <div>
                    <h3 className="text-base font-bold text-rose-900">
                      HIGH-RISK PHYSICAL DRIVE SANITIZATION
                    </h3>
                    <p className="text-[11px] text-rose-700 font-medium">
                      Real Hardware Execution Authorization
                    </p>
                  </div>
                </div>

                <div className="bg-rose-50 border border-rose-300 rounded-md p-3.5 text-xs text-rose-950 space-y-2">
                  <p className="font-bold uppercase tracking-wider text-[11px] text-rose-900">
                    TARGET: {plan.physical_device_id} ({plan.display_name})
                  </p>
                  <div className="grid grid-cols-2 gap-2 text-[11px] bg-white/80 p-2.5 rounded-xs border border-rose-200">
                    <div>
                      <span className="text-slate-500">Vendor/Model:</span>{' '}
                      <span className="font-medium text-slate-800">{plan.vendor || 'Unknown'} {plan.model || ''}</span>
                    </div>
                    <div>
                      <span className="text-slate-500">Serial Number:</span>{' '}
                      <span className="font-mono text-slate-800">{plan.serial_number || 'N/A'}</span>
                    </div>
                    <div>
                      <span className="text-slate-500">Capacity:</span>{' '}
                      <span className="font-bold text-slate-900">{formatBytes(plan.capacity_bytes)}</span>
                    </div>
                    <div>
                      <span className="text-slate-500">Sanitize Method:</span>{' '}
                      <span className="font-bold text-rose-900">{plan.method}</span>
                    </div>
                    <div>
                      <span className="text-slate-500">Passes:</span>{' '}
                      <span className="font-medium text-slate-800">{plan.passes} pass(es)</span>
                    </div>
                    <div>
                      <span className="text-slate-500">Verification:</span>{' '}
                      <span className="font-medium text-slate-800">{plan.verification_plan.strategy}</span>
                    </div>
                  </div>
                  <p className="font-bold text-rose-800 leading-snug">
                    CRITICAL WARNING: THIS IS A REAL PHYSICAL DRIVE ERASURE - DATA CANNOT BE RECOVERED.
                  </p>
                  <p className="text-[11px] text-rose-900">
                    All sector contents, partition tables, and volume extents across the physical drive will be irreversibly overwritten or sanitized. This operation cannot be cancelled once active sector writing begins.
                  </p>
                </div>

                {/* Checkbox Acknowledgment */}
                <label className="flex items-start space-x-2.5 text-xs text-slate-800 cursor-pointer select-none">
                  <input
                    type="checkbox"
                    checked={warningAcknowledged}
                    onChange={(e) => setWarningAcknowledged(e.target.checked)}
                    className="rounded-xs border-rose-300 text-rose-600 focus:ring-rose-500 mt-0.5"
                  />
                  <span>
                    I confirm that I want to physically erase <strong className="font-mono">{plan.physical_device_id}</strong> on real hardware. I understand all data on this drive cannot be recovered.
                  </span>
                </label>
              </>
            ) : (
              <>
                <div className="flex items-center space-x-3 text-amber-600 border-b border-slate-100 pb-3">
                  <ShieldAlert className="w-6 h-6 shrink-0" />
                  <h3 className="text-base font-bold text-slate-900">
                    Two-Stage Safety Authorization (Simulation)
                  </h3>
                </div>

                <div className="bg-amber-50 border border-amber-200 rounded-md p-3 text-xs text-amber-900 space-y-1">
                  <p className="font-bold uppercase tracking-wider text-[11px]">
                    Target: {plan.physical_device_id} ({plan.display_name})
                  </p>
                  <p>
                    This execution runs inside the safe simulation engine. Storage hardware sectors will not be modified.
                  </p>
                </div>

                {/* Checkbox Acknowledgment */}
                <label className="flex items-start space-x-2.5 text-xs text-slate-700 cursor-pointer select-none">
                  <input
                    type="checkbox"
                    checked={warningAcknowledged}
                    onChange={(e) => setWarningAcknowledged(e.target.checked)}
                    className="rounded-xs border-slate-300 text-blue-600 focus:ring-blue-500 mt-0.5"
                  />
                  <span>
                    I understand whole-disk sanitization specifications and confirm this simulation run.
                  </span>
                </label>
              </>
            )}

            {/* Typed Confirmation */}
            <div className="space-y-1.5">
              <label className="block text-xs font-medium text-slate-700">
                To confirm, type the exact device path (
                <strong className="font-mono">{plan.physical_device_id}</strong> or{' '}
                <strong className="font-mono">{plan.physical_device_id.replace(/^\\\\\.\\\\/, '')}</strong>
                ):
              </label>
              <input
                type="text"
                value={typedConfirmation}
                onChange={(e) => setTypedConfirmation(e.target.value)}
                placeholder={plan.physical_device_id}
                className="w-full px-3 py-2 text-xs font-mono border border-slate-300 rounded-md focus:outline-hidden focus:ring-1 focus:ring-blue-500"
              />
            </div>

            <div className="flex justify-end space-x-2 pt-2 border-t border-slate-100">
              <button
                type="button"
                onClick={() => setShowConfirmModal(false)}
                className="px-3 py-1.5 border border-slate-300 text-slate-600 rounded-md text-xs hover:bg-slate-50 cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                disabled={
                  !warningAcknowledged ||
                  (typedConfirmation.trim().toLowerCase() !== plan.physical_device_id.toLowerCase() &&
                    typedConfirmation.trim().toLowerCase() !==
                      plan.physical_device_id.replace(/^\\\\\.\\\\/, '').toLowerCase())
                }
                onClick={handleExecute}
                className={`font-semibold text-xs px-4 py-2 rounded-md shadow-xs disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer ${
                  plan.execution_mode === 'RealHardware'
                    ? 'bg-rose-700 hover:bg-rose-800 text-white'
                    : 'bg-amber-600 hover:bg-amber-700 text-white'
                }`}
              >
                {plan.execution_mode === 'RealHardware' ? 'Execute Real Hardware Sanitization' : 'Start Simulation'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Sanitization Certificate & Forensic Report Modal */}
      {showReportModal && (
        <div className="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4">
          <div className="bg-white border border-slate-200 rounded-lg max-w-2xl w-full p-6 shadow-2xl space-y-5 max-h-[90vh] overflow-y-auto">
            {loadingReport || !report ? (
              <div className="py-12 flex flex-col items-center justify-center space-y-3 text-slate-500">
                <Loader2 className="w-8 h-8 animate-spin text-blue-600" />
                <p className="text-xs font-semibold">Generating forensic sanitization certificate...</p>
              </div>
            ) : (
              <>
                <div className="flex items-center justify-between border-b border-slate-100 pb-3">
                  <div className="flex items-center space-x-2.5">
                    <ShieldCheck className="w-6 h-6 text-blue-600 shrink-0" />
                    <div>
                      <h3 className="text-base font-bold text-slate-900">
                        Forensic Drive Sanitization Certificate
                      </h3>
                      <p className="text-[11px] text-slate-500 font-mono">
                        Report ID: {report.report_id}
                      </p>
                    </div>
                  </div>
                  <span className="text-[10px] font-mono text-slate-400">
                    {new Date(report.integrity.generated_at).toLocaleString()}
                  </span>
                </div>

                {/* Simulation vs Production Banner */}
                {report.operation_info.is_simulation ? (
                  <div className="bg-amber-50 border-2 border-amber-300 rounded-md p-3 text-xs text-amber-950 flex items-center space-x-2">
                    <ShieldAlert className="w-4 h-4 text-amber-600 shrink-0" />
                    <span className="font-bold tracking-wide uppercase text-[11px]">
                      SIMULATION ONLY — NO PHYSICAL STORAGE WAS SANITIZED
                    </span>
                  </div>
                ) : (
                  <div className="bg-emerald-50 border-2 border-emerald-400 rounded-md p-3 text-xs text-emerald-950 flex items-center space-x-2">
                    <CheckCircle2 className="w-4 h-4 text-emerald-600 shrink-0" />
                    <span className="font-bold tracking-wide uppercase text-[11px]">
                      PRODUCTION HARDWARE SANITIZATION CERTIFICATE — VERIFIED PURGE
                    </span>
                  </div>
                )}

                {/* Target Device Information */}
                <div className="border border-slate-200 rounded-md p-3.5 bg-slate-50 text-xs space-y-2">
                  <span className="text-[10px] font-bold text-slate-500 uppercase tracking-wider block">
                    Sanitized Storage Target
                  </span>
                  <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
                    <div>
                      <span className="text-slate-500 text-[11px] block">Physical Device:</span>
                      <span className="font-mono font-semibold text-slate-900">{report.device_info.physical_device_id}</span>
                    </div>
                    <div>
                      <span className="text-slate-500 text-[11px] block">Media / Bus:</span>
                      <span className="font-medium text-slate-800">{report.device_info.media_type} ({report.device_info.bus_type || 'N/A'})</span>
                    </div>
                    <div>
                      <span className="text-slate-500 text-[11px] block">Capacity:</span>
                      <span className="font-bold text-slate-900">{formatBytes(report.device_info.capacity_bytes)}</span>
                    </div>
                    <div>
                      <span className="text-slate-500 text-[11px] block">Make & Model:</span>
                      <span className="font-medium text-slate-800">{report.device_info.vendor || 'Unknown'} {report.device_info.model || ''}</span>
                    </div>
                    <div>
                      <span className="text-slate-500 text-[11px] block">Serial Number:</span>
                      <span className="font-mono text-slate-800">{report.device_info.serial_number || 'N/A'}</span>
                    </div>
                    <div>
                      <span className="text-slate-500 text-[11px] block">Sector Geometry:</span>
                      <span className="font-mono text-slate-800">{report.device_info.sector_size} bytes/sector</span>
                    </div>
                  </div>
                </div>

                {/* Execution & Verification Summary */}
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div className="border border-slate-200 rounded-md p-3 bg-white space-y-2 text-xs">
                    <span className="text-[10px] font-bold text-slate-500 uppercase tracking-wider block">
                      Execution Metrics
                    </span>
                    <div className="space-y-1 text-[11px]">
                      <div className="flex justify-between">
                        <span className="text-slate-500">Sanitization Method:</span>
                        <span className="font-semibold text-slate-900">{report.operation_info.sanitization_method}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-500">Execution Status:</span>
                        <span className="font-bold text-emerald-700">{report.execution_metrics.status}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-500">Bytes Processed:</span>
                        <span className="font-mono text-slate-800">{formatBytes(report.execution_metrics.bytes_processed)}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-500">Elapsed Time:</span>
                        <span className="font-mono text-slate-800">{report.operation_info.elapsed_seconds.toFixed(2)}s</span>
                      </div>
                    </div>
                  </div>

                  <div className="border border-slate-200 rounded-md p-3 bg-white space-y-2 text-xs">
                    <span className="text-[10px] font-bold text-slate-500 uppercase tracking-wider block">
                      Post-Sanitization Verification
                    </span>
                    <div className="space-y-1 text-[11px]">
                      <div className="flex justify-between items-center">
                        <span className="text-slate-500">Outcome:</span>
                        <span className={`font-bold flex items-center gap-1 ${
                          report.verification.outcome === 'Verified' ? 'text-emerald-700' : 'text-amber-700'
                        }`}>
                          {report.verification.outcome === 'Verified' ? (
                            <Check className="w-3 h-3" />
                          ) : (
                            <AlertTriangle className="w-3 h-3" />
                          )}
                          {report.verification.outcome}
                        </span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-500">Strategy:</span>
                        <span className="font-medium text-slate-800">{report.verification.strategy}</span>
                      </div>
                      <p className="text-[10px] text-slate-600 pt-1 border-t border-slate-100">
                        {report.verification.details}
                      </p>
                    </div>
                  </div>
                </div>

                {/* Evidence & Report Integrity */}
                <div className="border border-slate-200 rounded-md p-3.5 bg-slate-900 text-slate-100 text-xs font-mono space-y-2">
                  <div className="flex justify-between items-center text-[10px] text-slate-400 border-b border-slate-700 pb-1">
                    <span>TAMPER-EVIDENT FORENSIC INTEGRITY</span>
                    <button
                      type="button"
                      onClick={handleCopyReportDigest}
                      className="text-blue-400 hover:text-blue-300 flex items-center gap-1 cursor-pointer"
                    >
                      {copiedDigest ? <Check className="w-3 h-3" /> : <Copy className="w-3 h-3" />}
                      {copiedDigest ? 'Copied SHA-256' : 'Copy Digest'}
                    </button>
                  </div>
                  <div>
                    <span className="text-slate-400 text-[10px] block">Canonical Report SHA-256 Digest:</span>
                    <span className="text-[11px] text-emerald-400 break-all">{report.integrity.report_digest}</span>
                  </div>
                  <div>
                    <span className="text-slate-400 text-[10px] block">Evidence Digest (Readback verification):</span>
                    <span className="text-[11px] text-blue-300 break-all">{report.verification.evidence_digest}</span>
                  </div>
                  <div>
                    <span className="text-slate-400 text-[10px] block">Tamper-Evident Audit Event Reference:</span>
                    <span className="text-[11px] text-slate-300">{report.integrity.audit_chain_reference}</span>
                  </div>
                </div>

                {/* Modal Actions */}
                <div className="flex justify-between items-center pt-2 border-t border-slate-100">
                  <button
                    type="button"
                    onClick={() => setShowReportModal(false)}
                    className="px-3.5 py-1.5 border border-slate-300 text-slate-600 rounded-md text-xs hover:bg-slate-50 cursor-pointer"
                  >
                    Close
                  </button>
                  <div className="flex items-center space-x-2">
                    <button
                      type="button"
                      onClick={handleDownloadReport}
                      className="px-3.5 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-xs font-semibold rounded-md flex items-center gap-1.5 cursor-pointer shadow-xs"
                    >
                      <Download className="w-3.5 h-3.5" />
                      Download Certificate (.md)
                    </button>
                  </div>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
