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
import {
  Button,
  Input,
  Progress,
  Chip,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  Checkbox,
} from '@heroui/react';
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
  getDriveErasureProgress,
  checkDriveEraserPrivileges,
  generateDriveErasureReport,
} from '../services/driveEraser';
import { requestDestructiveConfirmation } from '../services/safety';
import { useAuthStore } from '../stores/authStore';
import { useCaseStore } from '../stores/caseStore';
import { ActiveCaseBanner } from '../components/case/ActiveCaseBanner';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

export const DriveEraserPage: React.FC = () => {
  const sessionToken = useAuthStore((s) => s.sessionToken);
  const { activeCase } = useCaseStore();
  const [associateWithCase, setAssociateWithCase] = useState<boolean>(true);

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
  const [downloadedReport, setDownloadedReport] = useState(false);

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
      } catch (err: unknown) {
        if (isMounted) {
          const msg = err && typeof err === 'object' && 'message' in err ? String((err as { message: string }).message) : 'Failed to load storage devices.';
          setErrorMessage(msg);
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

  // Refresh Real Devices (invalidates cache)
  const handleRefresh = async () => {
    if (deviceSource === 'mock') {
      try {
        setIsRefreshing(true);
        const list = await getMockTestDevices();
        setDevices(list);
      } finally {
        setIsRefreshing(false);
      }
      return;
    }

    try {
      setIsRefreshing(true);
      setErrorMessage(null);
      const list = await refreshRealDevices();
      setDevices(list);
    } catch (err: unknown) {
      const msg = err && typeof err === 'object' && 'message' in err ? String((err as { message: string }).message) : 'Failed to refresh devices.';
      setErrorMessage(msg);
    } finally {
      setIsRefreshing(false);
    }
  };

  // Evaluate Capabilities on device selection change
  useEffect(() => {
    let isMounted = true;
    const target = customDeviceId.trim() || selectedDeviceId;
    if (!target) {
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

  // Reset plan when target device selection or execution mode changes
  useEffect(() => {
    setPlan(null);
    setResult(null);
    setReport(null);
    setErrorMessage(null);
  }, [selectedDeviceId, customDeviceId, executionMode]);

  const activeDevice = devices.find(
    (d) => d.device_id.toLowerCase() === (customDeviceId.trim() || selectedDeviceId).toLowerCase()
  );

  const isSystemOrBoot =
    activeDevice?.is_system_device ||
    activeDevice?.classification === 'system_device' ||
    activeDevice?.classification === 'boot_device' ||
    customDeviceId.toLowerCase().includes('physicaldrive0');

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
      const targetCaseId = associateWithCase && activeCase ? activeCase.case_id : null;
      const planned = await planDriveErasure({
        target_device_id: target,
        requested_method: null,
        execution_mode: executionMode,
        session_token: sessionToken,
        case_id: targetCaseId,
      });
      setPlan(planned);
    } catch (err: unknown) {
      const msg = err && typeof err === 'object' && 'message' in err ? String((err as { message: string }).message) : 'Failed to generate drive erasure plan.';
      setErrorMessage(msg);
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
        operation_type: 'DriveErasure',
        session_token: sessionToken || '',
      });

      setConfirmationId(challenge.confirmation_id);
      setTypedConfirmation('');
      setWarningAcknowledged(false);
      setShowConfirmModal(true);
    } catch (err: unknown) {
      const msg = err && typeof err === 'object' && 'message' in err ? String((err as { message: string }).message) : 'Failed to generate safety confirmation challenge.';
      setErrorMessage(msg);
    }
  };

  // Execute Erasure (Real Hardware or Simulation)
  const handleExecute = async () => {
    if (!plan || !confirmationId) return;

    let pollTimer: ReturnType<typeof setInterval> | null = null;

    try {
      setIsExecuting(true);
      setShowConfirmModal(false);
      setErrorMessage(null);

      const isHardware = plan.execution_mode === 'RealHardware';
      const operationId = isHardware
        ? `op-drive-hw-${Date.now()}`
        : `op-drive-sim-${Date.now()}`;

      setSimulationProgress({
        operation_id: operationId,
        percentage: 0,
        bytes_processed: 0,
        total_bytes: plan.capacity_bytes,
        current_pass: 1,
        total_passes: plan.passes,
        current_stage: isHardware
          ? 'Acquiring exclusive device lock & dismounting volumes'
          : 'Initializing simulation environment',
        elapsed_seconds: 0.0,
        eta_seconds: null,
      });

      pollTimer = setInterval(async () => {
        try {
          const prog = await getDriveErasureProgress(operationId);
          if (prog) {
            setSimulationProgress(prog);
          }
        } catch {
          // Ignore transient polling error
        }
      }, 250);

      let confirmValue = typedConfirmation.trim();
      if (confirmValue.toLowerCase().startsWith('confirm-erase ')) {
        confirmValue = confirmValue.substring('confirm-erase '.length).trim();
      } else if (confirmValue.toLowerCase().startsWith('confirm ')) {
        confirmValue = confirmValue.substring('confirm '.length).trim();
      }
      if (!confirmValue.startsWith('\\\\.\\') && plan.physical_device_id.startsWith('\\\\.\\')) {
        confirmValue = `\\\\.\\${confirmValue}`;
      }

      const targetCaseId = associateWithCase && activeCase ? activeCase.case_id : null;

      const res = isHardware
        ? await executeDriveErasureHardware({
            plan_id: plan.plan_id,
            confirmation_id: confirmationId,
            operation_id: operationId,
            typed_confirmation: confirmValue,
            warning_acknowledged: warningAcknowledged,
            session_token: sessionToken || 'hw-session',
            case_id: targetCaseId,
          })
        : await executeDriveErasureSimulation({
            plan_id: plan.plan_id,
            confirmation_id: confirmationId,
            operation_id: operationId,
            typed_confirmation: confirmValue,
            warning_acknowledged: warningAcknowledged,
            session_token: sessionToken || 'sim-session',
            case_id: targetCaseId,
          });

      if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }

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
    } catch (err: unknown) {
      if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
      const msg = err && typeof err === 'object' && 'message' in err ? String((err as { message: string }).message) : (plan.execution_mode === 'RealHardware' ? 'Real hardware drive erasure failed.' : 'Drive erasure simulation failed.');
      setErrorMessage(msg);
    } finally {
      if (pollTimer) {
        clearInterval(pollTimer);
      }
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
    } catch (err: unknown) {
      const msg = err && typeof err === 'object' && 'message' in err ? String((err as { message: string }).message) : 'Failed to generate sanitization report.';
      setErrorMessage(msg);
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
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    setDownloadedReport(true);
    setTimeout(() => setDownloadedReport(false), 3000);
  };

  return (
    <div className="space-y-6">
      {/* Active Forensic Case Banner & Context */}
      <ActiveCaseBanner
        associateWithCase={associateWithCase}
        onToggleAssociate={setAssociateWithCase}
      />

      {/* Privilege Status Warning for Real Hardware */}
      {executionMode === 'RealHardware' && privilegeStatus && !privilegeStatus.is_elevated && (
        <div className="border-l-2 border-amber-500 bg-amber-500/10 p-3 text-xs text-amber-900 dark:text-amber-200 flex items-start space-x-3">
          <AlertTriangle className="w-5 h-5 text-amber-600 dark:text-amber-400 shrink-0 mt-0.5" />
          <div className="space-y-0.5">
            <span className="font-bold uppercase tracking-wider text-[11px] block text-amber-800 dark:text-amber-300">
              Administrative Privileges Required (Non-Elevated Session)
            </span>
            <p className="text-[11px] text-amber-900 dark:text-amber-200 leading-relaxed">
              {privilegeStatus.message} Real hardware device locking and sanitization execution will fail closed with an Access Denied error. Restart LocardX with elevated administrator / root privileges.
            </p>
          </div>
        </div>
      )}

      {/* Execution Mode Top Banner */}
      {executionMode === 'RealHardware' ? (
        <div className="border-l-2 border-rose-600 bg-rose-600/10 p-3 text-xs text-rose-900 dark:text-rose-200 flex items-start space-x-3">
          <AlertTriangle className="w-5 h-5 text-rose-600 dark:text-rose-400 shrink-0 mt-0.5" />
          <div>
            <span className="font-bold uppercase tracking-wider text-[11px] block text-rose-800 dark:text-rose-300">
              Production Hardware Backend Active — Permanent Destructive Mode
            </span>
            <p className="text-[11px] text-rose-900 dark:text-rose-200 mt-0.5">
              Targeted media will be permanently overwritten. Active system/boot partitions remain blocked by 15-point safety interlocks.
            </p>
          </div>
        </div>
      ) : (
        <div className="border-l-2 border-amber-500 bg-amber-500/10 p-3 text-xs text-amber-900 dark:text-amber-200 flex items-start space-x-3">
          <ShieldAlert className="w-5 h-5 text-amber-600 dark:text-amber-400 shrink-0 mt-0.5" />
          <div>
            <span className="font-bold uppercase tracking-wider text-[11px] block text-amber-800 dark:text-amber-300">
              Simulation Mode Active (Safe Verification Harness)
            </span>
            <p className="text-[11px] text-amber-900 dark:text-amber-200 mt-0.5">
              All write operations, sector overwrites, and controller commands are simulated deterministically. No physical media is modified.
            </p>
          </div>
        </div>
      )}

      {/* Page Header */}
      <div className="border-b border-slate-200 dark:border-workstation-border pb-4">
        <h1 className="text-xl font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
          <Cpu className="w-5 h-5 text-slate-700 dark:text-slate-300" />
          Secure Drive Eraser
        </h1>
        <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
          Whole-disk media-aware sanitization engine supporting HDD, SATA SSD, NVMe, USB, and memory cards.
        </p>
      </div>

      {/* Error Alert */}
      {errorMessage && (
        <div className="bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 text-rose-800 dark:text-rose-300 rounded-md p-3.5 flex items-start space-x-3 text-xs">
          <AlertTriangle className="w-4 h-4 text-rose-600 dark:text-rose-400 shrink-0 mt-0.5" />
          <div className="flex-1">
            <span className="font-semibold">Safety Boundary Block: </span>
            {errorMessage}
          </div>
        </div>
      )}

      {/* Step 1: Physical Storage Device Selection */}
      <div className="space-y-4 pt-2">
        <div className="space-y-4">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-slate-200 dark:border-workstation-border pb-3">
            <div className="flex items-center space-x-2">
              <span className="w-5 h-5 rounded-full bg-slate-900 dark:bg-slate-100 text-white dark:text-slate-900 text-[10px] font-bold flex items-center justify-center">
                1
              </span>
              <h2 className="text-sm font-bold text-slate-900 dark:text-slate-100">Physical Storage Device Selection</h2>
            </div>

            <div className="flex flex-wrap items-center gap-2.5">
              {/* Device Source: Real vs Mock */}
              <div className="inline-flex rounded-md p-0.5 bg-slate-100 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border text-xs">
                <button
                  type="button"
                  onClick={() => {
                    setDeviceSource('real');
                    setPlan(null);
                  }}
                  className={`px-3 py-1 rounded-sm font-semibold transition-all cursor-pointer ${
                    deviceSource === 'real'
                      ? 'bg-white dark:bg-workstation-panel text-slate-900 dark:text-slate-100 shadow-2xs font-bold'
                      : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100'
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
                      ? 'bg-white dark:bg-workstation-panel text-slate-900 dark:text-slate-100 shadow-2xs font-bold'
                      : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100'
                  }`}
                >
                  Mock Test Devices
                </button>
              </div>

              {/* Execution Mode Toggle */}
              <div className="inline-flex rounded-md p-0.5 bg-slate-100 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border text-xs">
                <button
                  type="button"
                  onClick={() => {
                    setExecutionMode('Simulation');
                    setPlan(null);
                  }}
                  className={`px-2.5 py-1 rounded-sm font-semibold transition-all cursor-pointer ${
                    executionMode === 'Simulation'
                      ? 'bg-white dark:bg-workstation-panel text-amber-800 dark:text-amber-300 shadow-2xs font-bold'
                      : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100'
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
                      : 'text-rose-700 dark:text-rose-400 hover:text-rose-900'
                  }`}
                >
                  Real Hardware
                </button>
              </div>

              {/* Refresh Button */}
              <Button
                size="sm"
                variant="flat"
                onPress={handleRefresh}
                isDisabled={isRefreshing || loadingDevices}
                startContent={<RefreshCw className={`w-3.5 h-3.5 ${isRefreshing ? 'animate-spin' : ''}`} />}
                className="text-xs font-medium h-7 px-2.5 bg-slate-100 dark:bg-workstation-surface text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-workstation-border"
              >
                Refresh
              </Button>

              <Chip size="sm" variant="flat" className="font-mono text-[10px] bg-slate-100 dark:bg-workstation-surface text-slate-700 dark:text-slate-300">
                {devices.length} Target(s)
              </Chip>
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
                    <div
                      key={d.device_id}
                      onClick={() => {
                        setSelectedDeviceId(d.device_id);
                        setCustomDeviceId(d.device_id);
                        setPlan(null);
                        setResult(null);
                        setErrorMessage(null);
                      }}
                      className={`text-left p-3 rounded-md border text-xs transition-all relative cursor-pointer ${
                        isSelected
                          ? 'border-slate-900 dark:border-slate-100 bg-slate-100 dark:bg-workstation-surface shadow-2xs ring-1 ring-slate-900 dark:ring-slate-100'
                          : 'border-slate-200 dark:border-workstation-border hover:border-slate-300 dark:hover:border-slate-700 bg-slate-50/40 dark:bg-workstation-panel'
                      }`}
                    >
                      <div className="flex items-center justify-between gap-1">
                        <span className="font-mono font-bold text-slate-800 dark:text-slate-200 truncate">{d.device_id}</span>
                        <span className="text-[9px] font-bold px-1.5 py-0.5 rounded-xs uppercase tracking-wider shrink-0 bg-slate-200 dark:bg-workstation-surface text-slate-800 dark:text-slate-300">
                          {deviceSource === 'real' ? 'Real Device' : 'Mock Device'}
                        </span>
                      </div>

                      <p className="text-[11px] text-slate-600 dark:text-slate-400 font-medium truncate mt-1">
                        {d.display_name}
                      </p>

                      <div className="flex items-center justify-between mt-2 pt-1 border-t border-slate-100 dark:border-workstation-border">
                        <span className="text-[10px] text-slate-400 font-mono">
                          {formatBytes(d.capacity_bytes)}
                        </span>

                        {isSystem || d.device_id.toLowerCase().includes('physicaldrive0') ? (
                          <Chip size="sm" variant="flat" className="bg-rose-100 dark:bg-rose-950/40 text-rose-800 dark:text-rose-300 text-[9px] font-bold h-5 px-1">
                            <Lock className="w-2.5 h-2.5 inline mr-1" />
                            PROTECTED (SYSTEM)
                          </Chip>
                        ) : isBoot ? (
                          <Chip size="sm" variant="flat" className="bg-amber-100 dark:bg-amber-950/40 text-amber-800 dark:text-amber-300 text-[9px] font-bold h-5 px-1">
                            <Lock className="w-2.5 h-2.5 inline mr-1" />
                            BOOT DEVICE
                          </Chip>
                        ) : d.removable || d.classification === 'removable_device' || d.classification === 'external_device' || d.device_type === 'usb' || d.device_type === 'memory_card' ? (
                          <Chip size="sm" variant="flat" className="bg-slate-200 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 text-[9px] font-bold h-5 px-1">
                            <CheckCircle2 className="w-2.5 h-2.5 inline mr-1 text-slate-600 dark:text-slate-400" />
                            READY FOR SANITIZATION
                          </Chip>
                        ) : (
                          <Chip size="sm" variant="flat" className="bg-slate-200 dark:bg-workstation-surface text-slate-700 dark:text-slate-300 text-[9px] font-semibold h-5 uppercase">
                            {d.device_type}
                          </Chip>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>

              {/* Custom Device Path Input */}
              <div className="pt-2">
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                  Target Physical Device Path
                </label>
                <div className="flex gap-2">
                  <Input
                    size="sm"
                    value={customDeviceId}
                    onValueChange={(val) => {
                      setCustomDeviceId(val);
                      setPlan(null);
                      setResult(null);
                      setErrorMessage(null);
                    }}
                    placeholder="e.g. \\.\PhysicalDrive1 or /dev/sdb"
                    className="flex-1 font-mono text-xs"
                    classNames={{
                      inputWrapper: `border ${
                        isLogicalVolume
                          ? 'border-rose-300 bg-rose-50/50 dark:bg-rose-950/20 text-rose-900'
                          : isSystemOrBoot
                          ? 'border-amber-300 bg-amber-50/50 dark:bg-amber-950/20 text-amber-900'
                          : 'border-slate-300 dark:border-workstation-border bg-white dark:bg-workstation-panel'
                      }`,
                    }}
                  />
                  <Button
                    size="sm"
                    isDisabled={isPlanning || !customDeviceId.trim() || isSystemOrBoot || isLogicalVolume}
                    isLoading={isPlanning}
                    onPress={handleGeneratePlan}
                    endContent={!isPlanning && <ArrowRight className="w-3.5 h-3.5" />}
                    className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 font-semibold text-xs px-4"
                  >
                    {isPlanning ? 'Evaluating...' : 'Generate Plan'}
                  </Button>
                </div>

                {isLogicalVolume && (
                  <p className="text-[11px] text-rose-600 dark:text-rose-400 font-medium mt-1.5 flex items-center gap-1">
                    <XCircle className="w-3.5 h-3.5 shrink-0" />
                    Target rejected: '{customDeviceId}' is a logical volume or drive letter. Physical drive identifier required.
                  </p>
                )}
                {isSystemOrBoot && !isLogicalVolume && (
                  <p className="text-[11px] text-rose-600 dark:text-rose-400 font-medium mt-1.5 flex items-center gap-1">
                    <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                    Hard Block: '{customDeviceId}' is recognized as an active OS system or boot device. Erasure is forbidden.
                  </p>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Step 2: Hardware Capabilities Assessment */}
      {capabilities && (
        <div className="space-y-3 pt-4 border-t border-slate-200 dark:border-workstation-border">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-slate-200 dark:border-workstation-border pb-3">
            <div className="flex items-center space-x-2">
              <span className="w-5 h-5 rounded-full bg-slate-900 dark:bg-slate-100 text-white dark:text-slate-900 text-[10px] font-bold flex items-center justify-center">
                2
              </span>
              <h2 className="text-sm font-bold text-slate-900 dark:text-slate-100">Hardware Capability Assessment</h2>
            </div>

              <div className="flex items-center gap-2">
                {assessment && (
                  <>
                    {assessment.overall_state === 'Supported' && (
                      <Chip size="sm" variant="flat" startContent={<CheckCircle2 className="w-3 h-3 text-slate-700 dark:text-slate-300" />} className="bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 text-[10px] font-bold border border-slate-200 dark:border-workstation-border">
                        Capability Supported
                      </Chip>
                    )}
                    {assessment.overall_state === 'Unknown' && (
                      <Chip size="sm" variant="flat" startContent={<AlertTriangle className="w-3 h-3 text-amber-600" />} className="bg-amber-100 dark:bg-amber-950/40 text-amber-800 dark:text-amber-300 text-[10px] font-bold">
                        Capability Unknown (Fails Closed)
                      </Chip>
                    )}
                    {assessment.overall_state === 'DetectionFailed' && (
                      <Chip size="sm" variant="flat" startContent={<XCircle className="w-3 h-3 text-rose-600" />} className="bg-rose-100 dark:bg-rose-950/40 text-rose-800 dark:text-rose-300 text-[10px] font-bold">
                        Capability Detection Failed (Fails Closed)
                      </Chip>
                    )}
                  </>
                )}
                <span className="text-[11px] text-slate-500 dark:text-slate-400 font-mono">
                  Bus: {capabilities.interface_bus} | Sector: {capabilities.sector_size}B
                </span>
              </div>
            </div>

            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
              <div className="p-2.5 bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs">
                <span className="text-[10px] font-semibold text-slate-500 uppercase block">Media Mechanism</span>
                <span className="font-medium text-slate-800 dark:text-slate-200">
                  {capabilities.is_rotational ? 'Magnetic Platter (HDD)' : 'Solid State (NAND Flash)'}
                </span>
              </div>
              <div className="p-2.5 bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs">
                <span className="text-[10px] font-semibold text-slate-500 uppercase block">Hardware Overwrite</span>
                <span className={`font-medium ${capabilities.supports_overwrite ? 'text-slate-900 dark:text-slate-100 font-semibold' : 'text-slate-500'}`}>
                  {capabilities.supports_overwrite ? 'Supported (LBA Range)' : 'Not Supported / Read-Only'}
                </span>
              </div>
              <div className="p-2.5 bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs">
                <span className="text-[10px] font-semibold text-slate-500 uppercase block">Firmware Sanitize</span>
                <span className={`font-medium ${capabilities.supports_firmware_sanitize ? 'text-slate-900 dark:text-slate-100 font-semibold' : 'text-slate-500'}`}>
                  {capabilities.supports_firmware_sanitize ? 'Supported' : 'Not Supported'}
                </span>
              </div>
              <div className="p-2.5 bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs">
                <span className="text-[10px] font-semibold text-slate-500 uppercase block">Crypto Key Erase</span>
                <span className={`font-medium ${capabilities.supports_crypto_erase ? 'text-slate-900 dark:text-slate-100 font-semibold' : 'text-slate-500'}`}>
                  {capabilities.supports_crypto_erase ? 'Supported (Hardware)' : 'Not Available'}
                </span>
              </div>
            </div>

            {assessment && assessment.assessment_notes.length > 0 && (
              <div className="bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border p-2.5 rounded-xs text-[11px] text-slate-600 dark:text-slate-300 space-y-0.5">
                <span className="font-semibold text-slate-700 dark:text-slate-300 block text-[10px] uppercase">Assessment Telemetry:</span>
                {assessment.assessment_notes.map((note, idx) => (
                  <p key={idx} className="font-mono text-[10px]">{note}</p>
                ))}
              </div>
            )}
        </div>
      )}

      {/* Step 3: Media-Aware Plan Display */}
      {plan && !result && (
        <div className="space-y-4 pt-4 border-t border-slate-200 dark:border-workstation-border">
          <div className="flex items-center justify-between border-b border-slate-200 dark:border-workstation-border pb-3">
            <div className="flex items-center space-x-2">
              <span className="w-5 h-5 rounded-full bg-slate-900 dark:bg-slate-100 text-white dark:text-slate-900 text-[10px] font-bold flex items-center justify-center">
                3
              </span>
              <h2 className="text-sm font-bold text-slate-900 dark:text-slate-100">Media-Aware Sanitization & Verification Plan</h2>
            </div>
              <Chip size="sm" variant="flat" className="bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 text-[10px] font-bold border border-slate-200 dark:border-workstation-border">
                PLAN ID: {plan.plan_id.substring(0, 16)}...
              </Chip>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs">
              <div className="space-y-2 border border-slate-200 dark:border-workstation-border p-3 rounded-xs bg-slate-50/50 dark:bg-workstation-surface">
                <h3 className="font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5 text-xs">
                  <Layers className="w-3.5 h-3.5 text-slate-600 dark:text-slate-400" />
                  Sanitization Specifications
                </h3>
                <div className="space-y-1 text-[11px] text-slate-700 dark:text-slate-300">
                  <p><span className="font-semibold text-slate-500">Target Device:</span> {plan.physical_device_id} ({plan.display_name})</p>
                  <p><span className="font-semibold text-slate-500">Method:</span> <strong className="text-slate-900 dark:text-slate-100">{plan.method}</strong></p>
                  <p><span className="font-semibold text-slate-500">Passes:</span> {plan.passes} pass(es)</p>
                  <p><span className="font-semibold text-slate-500">Execution Mode:</span> <strong className="text-amber-700 dark:text-amber-400 font-bold">{plan.execution_mode}</strong></p>
                  <p><span className="font-semibold text-slate-500">Capacity:</span> {formatBytes(plan.capacity_bytes)} ({plan.capacity_bytes} bytes)</p>
                </div>
              </div>

              <div className="space-y-2 border border-slate-200 dark:border-workstation-border p-3 rounded-xs bg-slate-50/50 dark:bg-workstation-surface">
                <h3 className="font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5 text-xs">
                  <FileCheck className="w-3.5 h-3.5 text-slate-600 dark:text-slate-400" />
                  Post-Sanitization Verification Plan
                </h3>
                <div className="space-y-1 text-[11px] text-slate-700 dark:text-slate-300">
                  <p><span className="font-semibold text-slate-500">Strategy:</span> {plan.verification_plan.strategy}</p>
                  <p><span className="font-semibold text-slate-500">Coverage:</span> {plan.verification_plan.sample_percentage ? `${plan.verification_plan.sample_percentage}% LBA read` : 'Controller Firmware Register'}</p>
                  <p><span className="font-semibold text-slate-500">Requirements:</span> {plan.verification_plan.requirements}</p>
                </div>
              </div>
            </div>

            {/* Physical Limitations */}
            {plan.limitations.length > 0 && (
              <div className="bg-amber-50/60 dark:bg-amber-950/20 border border-amber-200 dark:border-amber-900/40 p-3 rounded-xs text-xs space-y-1">
                <span className="font-bold text-amber-900 dark:text-amber-300 text-[11px] uppercase tracking-wider flex items-center gap-1">
                  <Info className="w-3.5 h-3.5 text-amber-700 dark:text-amber-400" />
                  Physical Media & Hardware Limitations
                </span>
                <ul className="list-disc list-inside text-[11px] text-amber-800 dark:text-amber-300 space-y-0.5">
                  {plan.limitations.map((lim, idx) => (
                    <li key={idx}>{lim}</li>
                  ))}
                </ul>
              </div>
            )}

            {/* Action to proceed */}
            <div className="flex justify-end gap-2 pt-2">
              <Button
                size="sm"
                variant="flat"
                onPress={handleReset}
                className="text-xs border border-slate-200 dark:border-workstation-border bg-white dark:bg-workstation-panel text-slate-700 dark:text-slate-300"
              >
                Cancel
              </Button>
              <Button
                size="sm"
                onPress={handleOpenConfirmation}
                endContent={<ArrowRight className="w-3.5 h-3.5" />}
                className={`font-semibold text-xs px-4 shadow-xs text-white ${
                  plan.execution_mode === 'RealHardware'
                    ? 'bg-rose-700 hover:bg-rose-800'
                    : 'bg-slate-900 hover:bg-slate-800 dark:bg-slate-100 dark:text-slate-900'
                }`}
              >
                {plan.execution_mode === 'RealHardware' ? 'Authorize Real Hardware Sanitization' : 'Authorize Simulation'}
              </Button>
            </div>
        </div>
      )}

      {/* Step 4: Live Execution Progress */}
      {isExecuting && simulationProgress && (
        <div className="space-y-4 pt-4 border-t border-slate-200 dark:border-workstation-border">
          <div className="flex items-center justify-between border-b border-slate-200 dark:border-workstation-border pb-3">
            <div className="flex items-center space-x-2">
              <Loader2 className={`w-4 h-4 animate-spin ${plan?.execution_mode === 'RealHardware' ? 'text-rose-600' : 'text-slate-800 dark:text-slate-200'}`} />
              <h2 className="text-sm font-bold text-slate-900 dark:text-slate-100">
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
              <div className="flex justify-between text-xs text-slate-600 dark:text-slate-300 font-medium">
                <span>{simulationProgress.current_stage}</span>
                <span className="font-mono">{simulationProgress.percentage.toFixed(0)}%</span>
              </div>
              <Progress
                aria-label="Sanitization progress"
                value={simulationProgress.percentage}
                size="md"
                classNames={{
                  indicator: plan?.execution_mode === 'RealHardware' ? 'bg-rose-600' : 'bg-slate-900 dark:bg-slate-100',
                  track: 'bg-slate-100 dark:bg-workstation-surface',
                }}
              />
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
        <div className="space-y-5 pt-4 border-t border-slate-200 dark:border-workstation-border">
          <div className="flex items-center justify-between border-b border-slate-200 dark:border-workstation-border pb-4">
            <div className="flex items-center space-x-3">
                {result.status === 'Completed' && result.verification.outcome === 'Verified' ? (
                  <div className="w-10 h-10 rounded-full bg-slate-100 dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border flex items-center justify-center text-slate-900 dark:text-slate-100">
                    <CheckCircle2 className="w-6 h-6" />
                  </div>
                ) : result.status === 'VerificationFailed' ? (
                  <div className="w-10 h-10 rounded-full bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900/40 flex items-center justify-center text-rose-600 dark:text-rose-400">
                    <XCircle className="w-6 h-6" />
                  </div>
                ) : result.status === 'UnableToVerify' ? (
                  <div className="w-10 h-10 rounded-full bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-900/40 flex items-center justify-center text-amber-600 dark:text-amber-400">
                    <AlertTriangle className="w-6 h-6" />
                  </div>
                ) : (
                  <div className="w-10 h-10 rounded-full bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900/40 flex items-center justify-center text-rose-600 dark:text-rose-400">
                    <XCircle className="w-6 h-6" />
                  </div>
                )}

                <div>
                  <h2 className="text-base font-bold text-slate-900 dark:text-slate-100">
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
                  <p className="text-xs text-slate-500 dark:text-slate-400">
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

              <Chip
                size="sm"
                variant="flat"
                className={`text-xs font-bold uppercase tracking-wide ${
                  result.status === 'Completed'
                    ? 'bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700'
                    : result.status === 'UnableToVerify'
                    ? 'bg-amber-100 dark:bg-amber-950/50 text-amber-800 dark:text-amber-300'
                    : 'bg-rose-100 dark:bg-rose-950/50 text-rose-800 dark:text-rose-300'
                }`}
              >
                {result.status}
              </Chip>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-xs">
              <div className="p-3 bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs space-y-1">
                <span className="text-[10px] font-semibold text-slate-500 uppercase block">Target Device</span>
                <p className="font-mono font-bold text-slate-900 dark:text-slate-100">{result.physical_device_id}</p>
                <p className="text-slate-600 dark:text-slate-400">{result.display_name}</p>
              </div>
              <div className="p-3 bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs space-y-1">
                <span className="text-[10px] font-semibold text-slate-500 uppercase block">Sanitization Method</span>
                <p className="font-bold text-slate-900 dark:text-slate-100">{result.method}</p>
                <Chip
                  size="sm"
                  variant="flat"
                  className={`text-[10px] font-bold mt-0.5 ${
                    result.execution_mode === 'RealHardware'
                      ? 'bg-rose-100 dark:bg-rose-950/50 text-rose-800 dark:text-rose-300 border border-rose-200 dark:border-rose-900'
                      : 'bg-amber-100 dark:bg-amber-950/50 text-amber-800 dark:text-amber-300 border border-amber-200 dark:border-amber-900'
                  }`}
                >
                  Mode: {result.execution_mode}
                </Chip>
              </div>
              <div className="p-3 bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs space-y-1">
                <span className="text-[10px] font-semibold text-slate-500 uppercase block">Verification Outcome</span>
                <p
                  className={`font-bold flex items-center gap-1 ${
                    result.verification.outcome === 'Verified'
                      ? 'text-slate-900 dark:text-slate-100'
                      : result.verification.outcome === 'UnableToVerify'
                      ? 'text-amber-700 dark:text-amber-400'
                      : 'text-rose-700 dark:text-rose-400'
                  }`}
                >
                  {result.verification.outcome === 'Verified' ? (
                    <Check className="w-3.5 h-3.5" />
                  ) : (
                    <AlertTriangle className="w-3.5 h-3.5" />
                  )}
                  {result.verification.outcome}
                </p>
                <p className="text-[11px] text-slate-600 dark:text-slate-400">{result.verification.strategy}</p>
              </div>
            </div>

            <div className="border border-slate-200 dark:border-workstation-border rounded-xs p-3 bg-slate-50 dark:bg-workstation-surface text-xs space-y-2 font-mono">
              <div className="flex justify-between text-[11px] text-slate-500 dark:text-slate-400 border-b border-slate-200 dark:border-workstation-border pb-1">
                <span>OPERATION AUDIT REFERENCE</span>
                <span>{result.operation_id}</span>
              </div>
              <div className="text-[11px] text-slate-600 dark:text-slate-300 space-y-0.5">
                <p>Processed: {formatBytes(result.bytes_processed)} in {result.elapsed_seconds.toFixed(2)}s</p>
                <p>Verification Details: {result.verification.details}</p>
                <p>Audit Events Emitted: {result.audit_references.join(', ')}</p>
              </div>
            </div>

            <div className="flex justify-between items-center pt-2">
              <Button
                size="sm"
                variant="flat"
                onPress={handleReset}
                startContent={<RotateCcw className="w-3.5 h-3.5" />}
                className="text-xs font-semibold border border-slate-300 dark:border-workstation-border bg-white dark:bg-workstation-panel text-slate-700 dark:text-slate-300"
              >
                Plan Another Drive
              </Button>
              <div className="flex items-center space-x-2">
                <Button
                  size="sm"
                  onPress={handleViewReport}
                  isDisabled={loadingReport}
                  isLoading={loadingReport}
                  startContent={!loadingReport && <FileCheck className="w-3.5 h-3.5" />}
                  className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 text-xs font-semibold px-4"
                >
                  View Sanitization Certificate
                </Button>
                <Button
                  size="sm"
                  variant="flat"
                  onPress={handleCopyCertificate}
                  startContent={copiedText ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                  className="bg-slate-100 dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border text-slate-800 dark:text-slate-200 text-xs font-semibold px-4"
                >
                  {copiedText ? 'Copied JSON' : 'Copy Forensic Record'}
                </Button>
              </div>
            </div>
        </div>
      )}

      {/* Two-Stage Confirmation Modal */}
      <Modal
        isOpen={showConfirmModal && !!plan}
        onClose={() => setShowConfirmModal(false)}
        size="lg"
        classNames={{
          base: 'bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border',
          header: 'border-b border-slate-100 dark:border-workstation-border',
          footer: 'border-t border-slate-100 dark:border-workstation-border',
        }}
      >
        <ModalContent>
          {plan && (
            <>
              <ModalHeader>
                {plan.execution_mode === 'RealHardware' ? (
                  <div className="flex items-center space-x-3 text-rose-600 dark:text-rose-400">
                    <AlertTriangle className="w-6 h-6 shrink-0" />
                    <div>
                      <h3 className="text-base font-bold text-rose-900 dark:text-rose-200">
                        HIGH-RISK PHYSICAL DRIVE SANITIZATION
                      </h3>
                      <p className="text-[11px] text-rose-700 dark:text-rose-300 font-medium">
                        Real Hardware Execution Authorization
                      </p>
                    </div>
                  </div>
                ) : (
                  <div className="flex items-center space-x-3 text-amber-600 dark:text-amber-400">
                    <ShieldAlert className="w-6 h-6 shrink-0" />
                    <h3 className="text-base font-bold text-slate-900 dark:text-slate-100">
                      Two-Stage Safety Authorization (Simulation)
                    </h3>
                  </div>
                )}
              </ModalHeader>

              <ModalBody className="py-4 space-y-4">
                {plan.execution_mode === 'RealHardware' ? (
                  <>
                    <div className="bg-rose-50 dark:bg-rose-950/30 border border-rose-300 dark:border-rose-900/50 rounded-md p-3.5 text-xs text-rose-950 dark:text-rose-200 space-y-2">
                      <p className="font-bold uppercase tracking-wider text-[11px] text-rose-900 dark:text-rose-300">
                        TARGET: {plan.physical_device_id} ({plan.display_name})
                      </p>
                      <div className="grid grid-cols-2 gap-2 text-[11px] bg-white/80 dark:bg-workstation-surface p-2.5 rounded-xs border border-rose-200 dark:border-rose-900/40">
                        <div>
                          <span className="text-slate-500">Vendor/Model:</span>{' '}
                          <span className="font-medium text-slate-800 dark:text-slate-200">{plan.vendor || 'Unknown'} {plan.model || ''}</span>
                        </div>
                        <div>
                          <span className="text-slate-500">Serial Number:</span>{' '}
                          <span className="font-mono text-slate-800 dark:text-slate-200">{plan.serial_number || 'N/A'}</span>
                        </div>
                        <div>
                          <span className="text-slate-500">Capacity:</span>{' '}
                          <span className="font-bold text-slate-900 dark:text-slate-100">{formatBytes(plan.capacity_bytes)}</span>
                        </div>
                        <div>
                          <span className="text-slate-500">Sanitize Method:</span>{' '}
                          <span className="font-bold text-rose-900 dark:text-rose-300">{plan.method}</span>
                        </div>
                        <div>
                          <span className="text-slate-500">Passes:</span>{' '}
                          <span className="font-medium text-slate-800 dark:text-slate-200">{plan.passes} pass(es)</span>
                        </div>
                        <div>
                          <span className="text-slate-500">Verification:</span>{' '}
                          <span className="font-medium text-slate-800 dark:text-slate-200">{plan.verification_plan.strategy}</span>
                        </div>
                      </div>
                      <p className="font-bold text-rose-800 dark:text-rose-300 leading-snug">
                        CRITICAL WARNING: THIS IS A REAL PHYSICAL DRIVE ERASURE - DATA CANNOT BE RECOVERED.
                      </p>
                      <p className="text-[11px] text-rose-900 dark:text-rose-200">
                        All sector contents, partition tables, and volume extents across the physical drive will be irreversibly overwritten or sanitized. This operation cannot be cancelled once active sector writing begins.
                      </p>
                    </div>

                    <Checkbox
                      size="sm"
                      isSelected={warningAcknowledged}
                      onValueChange={setWarningAcknowledged}
                      color="danger"
                      classNames={{
                        label: 'text-xs text-slate-800 dark:text-slate-200',
                      }}
                    >
                      I confirm that I want to physically erase <strong className="font-mono">{plan.physical_device_id}</strong> on real hardware. I understand all data on this drive cannot be recovered.
                    </Checkbox>
                  </>
                ) : (
                  <>
                    <div className="bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-900/40 rounded-md p-3 text-xs text-amber-900 dark:text-amber-200 space-y-1">
                      <p className="font-bold uppercase tracking-wider text-[11px]">
                        Target: {plan.physical_device_id} ({plan.display_name})
                      </p>
                      <p>
                        This execution runs inside the safe simulation engine. Storage hardware sectors will not be modified.
                      </p>
                    </div>

                    <Checkbox
                      size="sm"
                      isSelected={warningAcknowledged}
                      onValueChange={setWarningAcknowledged}
                      classNames={{
                        label: 'text-xs text-slate-700 dark:text-slate-300',
                      }}
                    >
                      I understand whole-disk sanitization specifications and confirm this simulation run.
                    </Checkbox>
                  </>
                )}

                {/* Typed Confirmation */}
                <div className="space-y-1.5 pt-2 border-t border-slate-100 dark:border-workstation-border">
                  <div className="flex items-center justify-between">
                    <label className="block text-xs font-medium text-slate-700 dark:text-slate-300">
                      Type target device to confirm:
                    </label>
                    <button
                      type="button"
                      onClick={() => setTypedConfirmation(plan.physical_device_id)}
                      className="text-[11px] text-slate-700 dark:text-slate-300 hover:text-slate-900 dark:hover:text-slate-100 font-semibold cursor-pointer underline"
                    >
                      Auto-fill "{plan.physical_device_id}"
                    </button>
                  </div>
                  <Input
                    size="sm"
                    value={typedConfirmation}
                    onValueChange={setTypedConfirmation}
                    placeholder={plan.physical_device_id}
                    className="font-mono text-xs"
                    classNames={{
                      inputWrapper: 'bg-white dark:bg-workstation-panel border border-slate-300 dark:border-workstation-border',
                    }}
                  />
                  <p className="text-[10px] text-slate-500">
                    Accepts <code className="font-mono bg-slate-100 dark:bg-workstation-surface px-1 py-0.5 rounded">{plan.physical_device_id}</code>,{' '}
                    <code className="font-mono bg-slate-100 dark:bg-workstation-surface px-1 py-0.5 rounded">{plan.physical_device_id.replace(/^\\\\\.\\/, '')}</code>, or{' '}
                    <code className="font-mono bg-slate-100 dark:bg-workstation-surface px-1 py-0.5 rounded">CONFIRM-ERASE {plan.physical_device_id}</code>.
                  </p>
                </div>
              </ModalBody>

              <ModalFooter>
                <Button
                  size="sm"
                  variant="flat"
                  onPress={() => setShowConfirmModal(false)}
                  className="text-xs border border-slate-300 dark:border-workstation-border bg-white dark:bg-workstation-panel text-slate-700 dark:text-slate-300"
                >
                  Cancel
                </Button>
                <Button
                  size="sm"
                  isDisabled={
                    !warningAcknowledged ||
                    !(
                      typedConfirmation.trim().toLowerCase() === plan.physical_device_id.toLowerCase() ||
                      typedConfirmation.trim().toLowerCase() === plan.physical_device_id.replace(/^\\\\\.\\/, '').toLowerCase() ||
                      typedConfirmation.trim().toLowerCase() === plan.physical_device_id.replace(/^\\\\\\.\\\\/, '').toLowerCase() ||
                      typedConfirmation.trim().toLowerCase() === `confirm-erase ${plan.physical_device_id.toLowerCase()}` ||
                      typedConfirmation.trim().toLowerCase() === `confirm ${plan.physical_device_id.toLowerCase()}`
                    )
                  }
                  onPress={handleExecute}
                  className={`font-semibold text-xs px-4 shadow-xs text-white ${
                    plan.execution_mode === 'RealHardware'
                      ? 'bg-rose-700 hover:bg-rose-800'
                      : 'bg-amber-600 hover:bg-amber-700'
                  }`}
                >
                  {plan.execution_mode === 'RealHardware' ? 'Execute Real Hardware Sanitization' : 'Start Simulation'}
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>

      {/* Sanitization Certificate & Forensic Report Modal */}
      <Modal
        isOpen={showReportModal}
        onClose={() => setShowReportModal(false)}
        size="2xl"
        scrollBehavior="inside"
        classNames={{
          base: 'bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border',
          header: 'border-b border-slate-100 dark:border-workstation-border',
          footer: 'border-t border-slate-100 dark:border-workstation-border',
        }}
      >
        <ModalContent>
          {loadingReport || !report ? (
            <ModalBody className="py-12 flex flex-col items-center justify-center space-y-3 text-slate-500">
              <Loader2 className="w-8 h-8 animate-spin text-slate-800 dark:text-slate-200" />
              <p className="text-xs font-semibold">Generating forensic sanitization certificate...</p>
            </ModalBody>
          ) : (
            <>
              <ModalHeader className="flex items-center justify-between">
                <div className="flex items-center space-x-2.5">
                  <ShieldCheck className="w-6 h-6 text-slate-800 dark:text-slate-200 shrink-0" />
                  <div>
                    <h3 className="text-base font-bold text-slate-900 dark:text-slate-100">
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
              </ModalHeader>

              <ModalBody className="py-4 space-y-4">
                {/* Simulation vs Production Banner */}
                {report.operation_info.is_simulation ? (
                  <div className="bg-amber-50 dark:bg-amber-950/30 border-2 border-amber-300 dark:border-amber-900/60 rounded-md p-3 text-xs text-amber-950 dark:text-amber-200 flex items-center space-x-2">
                    <ShieldAlert className="w-4 h-4 text-amber-600 shrink-0" />
                    <span className="font-bold tracking-wide uppercase text-[11px]">
                      SIMULATION ONLY — NO PHYSICAL STORAGE WAS SANITIZED
                    </span>
                  </div>
                ) : (
                  <div className="bg-slate-100 dark:bg-workstation-surface border-2 border-slate-300 dark:border-slate-700 rounded-md p-3 text-xs text-slate-900 dark:text-slate-100 flex items-center space-x-2">
                    <CheckCircle2 className="w-4 h-4 text-slate-700 dark:text-slate-300 shrink-0" />
                    <span className="font-bold tracking-wide uppercase text-[11px]">
                      PRODUCTION HARDWARE SANITIZATION CERTIFICATE — VERIFIED PURGE
                    </span>
                  </div>
                )}

                {/* Target Device Information */}
                <div className="border border-slate-200 dark:border-workstation-border rounded-md p-3.5 bg-slate-50 dark:bg-workstation-surface text-xs space-y-2">
                  <span className="text-[10px] font-bold text-slate-500 uppercase tracking-wider block">
                    Sanitized Storage Target
                  </span>
                  <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
                    <div>
                      <span className="text-slate-500 text-[11px] block">Physical Device:</span>
                      <span className="font-mono font-semibold text-slate-900 dark:text-slate-100">{report.device_info.physical_device_id}</span>
                    </div>
                    <div>
                      <span className="text-slate-500 text-[11px] block">Media / Bus:</span>
                      <span className="font-medium text-slate-800 dark:text-slate-200">{report.device_info.media_type} ({report.device_info.bus_type || 'N/A'})</span>
                    </div>
                    <div>
                      <span className="text-slate-500 text-[11px] block">Capacity:</span>
                      <span className="font-bold text-slate-900 dark:text-slate-100">{formatBytes(report.device_info.capacity_bytes)}</span>
                    </div>
                    <div>
                      <span className="text-slate-500 text-[11px] block">Make & Model:</span>
                      <span className="font-medium text-slate-800 dark:text-slate-200">{report.device_info.vendor || 'Unknown'} {report.device_info.model || ''}</span>
                    </div>
                    <div>
                      <span className="text-slate-500 text-[11px] block">Serial Number:</span>
                      <span className="font-mono text-slate-800 dark:text-slate-200">{report.device_info.serial_number || 'N/A'}</span>
                    </div>
                    <div>
                      <span className="text-slate-500 text-[11px] block">Sector Geometry:</span>
                      <span className="font-mono text-slate-800 dark:text-slate-200">{report.device_info.sector_size} bytes/sector</span>
                    </div>
                  </div>
                </div>

                {/* Execution & Verification Summary */}
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div className="border border-slate-200 dark:border-workstation-border rounded-md p-3 bg-white dark:bg-workstation-panel space-y-2 text-xs">
                    <span className="text-[10px] font-bold text-slate-500 uppercase tracking-wider block">
                      Execution Metrics
                    </span>
                    <div className="space-y-1 text-[11px]">
                      <div className="flex justify-between">
                        <span className="text-slate-500">Sanitization Method:</span>
                        <span className="font-semibold text-slate-900 dark:text-slate-100">{report.operation_info.sanitization_method}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-500">Execution Status:</span>
                        <span className="font-bold text-slate-900 dark:text-slate-100">{report.execution_metrics.status}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-500">Bytes Processed:</span>
                        <span className="font-mono text-slate-800 dark:text-slate-200">{formatBytes(report.execution_metrics.bytes_processed)}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-500">Elapsed Time:</span>
                        <span className="font-mono text-slate-800 dark:text-slate-200">{report.operation_info.elapsed_seconds.toFixed(2)}s</span>
                      </div>
                    </div>
                  </div>

                  <div className="border border-slate-200 dark:border-workstation-border rounded-md p-3 bg-white dark:bg-workstation-panel space-y-2 text-xs">
                    <span className="text-[10px] font-bold text-slate-500 uppercase tracking-wider block">
                      Post-Sanitization Verification
                    </span>
                    <div className="space-y-1 text-[11px]">
                      <div className="flex justify-between items-center">
                        <span className="text-slate-500">Outcome:</span>
                        <span className={`font-bold flex items-center gap-1 ${
                          report.verification.outcome === 'Verified' ? 'text-slate-900 dark:text-slate-100' : 'text-amber-700 dark:text-amber-400'
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
                        <span className="font-medium text-slate-800 dark:text-slate-200">{report.verification.strategy}</span>
                      </div>
                      <p className="text-[10px] text-slate-600 dark:text-slate-400 pt-1 border-t border-slate-100 dark:border-workstation-border">
                        {report.verification.details}
                      </p>
                    </div>
                  </div>
                </div>

                {/* Evidence & Report Integrity */}
                <div className="border border-slate-200 dark:border-workstation-border rounded-md p-3.5 bg-slate-900 text-slate-100 text-xs font-mono space-y-2">
                  <div className="flex justify-between items-center text-[10px] text-slate-400 border-b border-slate-700 pb-1">
                    <span>TAMPER-EVIDENT FORENSIC INTEGRITY</span>
                    <button
                      type="button"
                      onClick={handleCopyReportDigest}
                      className="text-slate-300 hover:text-white flex items-center gap-1 cursor-pointer"
                    >
                      {copiedDigest ? <Check className="w-3 h-3" /> : <Copy className="w-3 h-3" />}
                      {copiedDigest ? 'Copied SHA-256' : 'Copy Digest'}
                    </button>
                  </div>
                  <div>
                    <span className="text-slate-400 text-[10px] block">Canonical Report SHA-256 Digest:</span>
                    <span className="text-[11px] text-slate-200 break-all">{report.integrity.report_digest}</span>
                  </div>
                  <div>
                    <span className="text-slate-400 text-[10px] block">Evidence Digest (Readback verification):</span>
                    <span className="text-[11px] text-slate-300 break-all">{report.verification.evidence_digest}</span>
                  </div>
                  <div>
                    <span className="text-slate-400 text-[10px] block">Tamper-Evident Audit Event Reference:</span>
                    <span className="text-[11px] text-slate-300">{report.integrity.audit_chain_reference}</span>
                  </div>
                </div>
              </ModalBody>

              <ModalFooter>
                <Button
                  size="sm"
                  variant="flat"
                  onPress={() => setShowReportModal(false)}
                  className="text-xs border border-slate-300 dark:border-workstation-border bg-white dark:bg-workstation-panel text-slate-700 dark:text-slate-300"
                >
                  Close
                </Button>
                <Button
                  size="sm"
                  onPress={handleDownloadReport}
                  startContent={downloadedReport ? <Check className="w-3.5 h-3.5" /> : <Download className="w-3.5 h-3.5" />}
                  className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 text-xs font-semibold px-4 shadow-xs"
                >
                  {downloadedReport ? 'Downloaded!' : 'Download Certificate (.md)'}
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>
    </div>
  );
};
