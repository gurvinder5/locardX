import React, { useEffect, useState } from 'react';
import {
  ShieldAlert,
  ShieldCheck,
  ShieldX,
  AlertTriangle,
  Lock,
  CheckCircle2,
  XCircle,
  Clock,
  RefreshCw,
  Cpu,
  Layers,
  ArrowRight,
  Check,
  AlertOctagon,
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
  Checkbox,
  Table,
  TableHeader,
  TableColumn,
  TableBody,
  TableRow,
  TableCell,
} from '@heroui/react';
import {
  ConfirmationChallenge,
  RiskLevel,
  SafetyDecision,
  SafetyDecisionOutcome,
} from '../types/safety';
import { OperationType } from '../types/operation';
import { StorageDeviceDto } from '../types/device';
import { listStorageDevices } from '../services/device';
import {
  confirmDestructiveOperation,
  evaluateOperationSafety,
  listSafetyEvaluations,
  requestDestructiveConfirmation,
} from '../services/safety';
import { useAuthStore } from '../stores/authStore';

function formatBytes(bytes?: number | null): string {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

export const SafetyInterlocksPage: React.FC = () => {
  const { sessionToken, currentUser } = useAuthStore();

  const [devices, setDevices] = useState<StorageDeviceDto[]>([]);
  const [devicesLoading, setDevicesLoading] = useState<boolean>(false);

  // Form State
  const [selectedTargetId, setSelectedTargetId] = useState<string>('');
  const [selectedTargetType, setSelectedTargetType] = useState<string>('PhysicalDevice');
  const [selectedOpType, setSelectedOpType] = useState<OperationType>('DriveErasure');
  const [customPath, setCustomPath] = useState<string>('');

  // Evaluation State
  const [evaluating, setEvaluating] = useState<boolean>(false);
  const [currentDecision, setCurrentDecision] = useState<SafetyDecision | null>(null);
  const [evalHistory, setEvalHistory] = useState<SafetyDecision[]>([]);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Two-Stage Confirmation State
  const [challenge, setChallenge] = useState<ConfirmationChallenge | null>(null);
  const [isConfirmModalOpen, setIsConfirmModalOpen] = useState<boolean>(false);
  const [warningAck, setWarningAck] = useState<boolean>(false);
  const [typedTargetInput, setTypedTargetInput] = useState<string>('');
  const [submittingConfirm, setSubmittingConfirm] = useState<boolean>(false);
  const [confirmResult, setConfirmResult] = useState<SafetyDecision | null>(null);

  const loadDevices = async () => {
    try {
      setDevicesLoading(true);
      const list = await listStorageDevices();
      setDevices(list);
      if (list.length > 0 && !selectedTargetId) {
        setSelectedTargetId(list[0].device_id);
        setSelectedTargetType('PhysicalDevice');
      }
    } catch (err: unknown) {
      console.error('Failed to load storage devices:', err);
    } finally {
      setDevicesLoading(false);
    }
  };

  const loadHistory = async () => {
    try {
      const history = await listSafetyEvaluations(20);
      setEvalHistory(history);
    } catch (err) {
      console.error('Failed to load evaluation history:', err);
    }
  };

  useEffect(() => {
    loadDevices();
    loadHistory();
  }, []);

  const handleEvaluate = async () => {
    setErrorMsg(null);
    setChallenge(null);
    setConfirmResult(null);

    let targetIdentifier = selectedTargetId;
    let targetType = selectedTargetType;
    let displayName = selectedTargetId;
    let sizeBytes: number | undefined = undefined;

    if (customPath.trim()) {
      targetIdentifier = customPath.trim();
      targetType = 'File';
      displayName = customPath.trim().split('\\').pop() || customPath.trim();
    } else {
      const dev = devices.find((d) => d.device_id === selectedTargetId);
      if (dev) {
        displayName = dev.display_name;
        sizeBytes = dev.capacity_bytes;
      }
    }

    try {
      setEvaluating(true);
      const decision = await evaluateOperationSafety({
        target_type: targetType,
        target_identifier: targetIdentifier,
        target_display_name: displayName,
        target_size_bytes: sizeBytes,
        operation_type: selectedOpType,
        session_token: sessionToken || undefined,
      });
      setCurrentDecision(decision);
      await loadHistory();
    } catch (err: unknown) {
      const msg = err && typeof err === 'object' && 'message' in err ? String((err as { message: string }).message) : 'Failed to evaluate target safety.';
      setErrorMsg(msg);
    } finally {
      setEvaluating(false);
    }
  };

  const handleRequestConfirmation = async () => {
    if (!currentDecision) return;
    setErrorMsg(null);

    try {
      setEvaluating(true);
      const challengeRes = await requestDestructiveConfirmation({
        operation_id: `op-preview-${Date.now()}`,
        target_type: currentDecision.target.target_type,
        target_identifier: currentDecision.target.identifier,
        target_display_name: currentDecision.target.display_name,
        target_size_bytes: currentDecision.target.size_bytes || undefined,
        operation_type: currentDecision.operation_type,
        session_token: sessionToken || '',
      });
      setChallenge(challengeRes);
      setWarningAck(false);
      setTypedTargetInput('');
      setIsConfirmModalOpen(true);
      await loadHistory();
    } catch (err: unknown) {
      const msg = err && typeof err === 'object' && 'message' in err ? String((err as { message: string }).message) : 'Failed to request destructive confirmation.';
      setErrorMsg(msg);
    } finally {
      setEvaluating(false);
    }
  };

  const handleSubmitConfirmation = async () => {
    if (!challenge) return;
    setErrorMsg(null);

    try {
      setSubmittingConfirm(true);
      const result = await confirmDestructiveOperation({
        confirmation_id: challenge.confirmation_id,
        operation_id: challenge.operation_id,
        warning_acknowledged: warningAck,
        typed_target_confirmation: typedTargetInput.trim(),
        session_token: sessionToken || '',
      });
      setConfirmResult(result);
      setCurrentDecision(result);
      setIsConfirmModalOpen(false);
      await loadHistory();
    } catch (err: unknown) {
      const msg = err && typeof err === 'object' && 'message' in err ? String((err as { message: string }).message) : 'Confirmation failed.';
      setErrorMsg(msg);
    } finally {
      setSubmittingConfirm(false);
    }
  };

  const getDecisionBadge = (decision: SafetyDecisionOutcome) => {
    switch (decision) {
      case 'Allowed':
        return (
          <Chip
            size="sm"
            variant="flat"
            startContent={<CheckCircle2 className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300" />}
            className="text-xs font-semibold bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700"
          >
            ALLOWED
          </Chip>
        );
      case 'RequiresConfirmation':
        return (
          <Chip
            size="sm"
            variant="flat"
            startContent={<AlertTriangle className="w-3.5 h-3.5 text-amber-600 dark:text-amber-400" />}
            className="text-xs font-semibold bg-amber-50 dark:bg-amber-950/30 text-amber-700 dark:text-amber-400 border border-amber-200 dark:border-amber-900/40"
          >
            REQUIRES CONFIRMATION
          </Chip>
        );
      case 'Denied':
        return (
          <Chip
            size="sm"
            variant="flat"
            startContent={<XCircle className="w-3.5 h-3.5 text-rose-600 dark:text-rose-400" />}
            className="text-xs font-semibold bg-rose-50 dark:bg-rose-950/30 text-rose-700 dark:text-rose-400 border border-rose-200 dark:border-rose-900/40"
          >
            DENIED
          </Chip>
        );
      case 'Blocked':
        return (
          <Chip
            size="sm"
            variant="flat"
            startContent={<Lock className="w-3.5 h-3.5 text-rose-600 dark:text-rose-400" />}
            className="text-xs font-semibold bg-rose-100 dark:bg-rose-950/50 text-rose-800 dark:text-rose-300 border border-rose-300 dark:border-rose-800"
          >
            HARD BLOCKED
          </Chip>
        );
    }
  };

  const getRiskBadge = (risk: RiskLevel) => {
    switch (risk) {
      case 'Low':
        return (
          <Chip size="sm" variant="flat" className="text-[11px] font-medium bg-slate-100 dark:bg-workstation-surface text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-workstation-border">
            Low Risk
          </Chip>
        );
      case 'Medium':
        return (
          <Chip size="sm" variant="flat" className="text-[11px] font-medium bg-amber-50 dark:bg-amber-950/30 text-amber-800 dark:text-amber-400 border border-amber-200 dark:border-amber-900/40">
            Medium Risk
          </Chip>
        );
      case 'High':
        return (
          <Chip size="sm" variant="flat" className="text-[11px] font-medium bg-amber-100 dark:bg-amber-950/50 text-amber-800 dark:text-amber-300 border border-amber-300 dark:border-amber-800">
            High Risk
          </Chip>
        );
      case 'Critical':
        return (
          <Chip size="sm" variant="flat" className="text-[11px] font-bold bg-rose-50 dark:bg-rose-950/50 text-rose-800 dark:text-rose-300 border border-rose-200 dark:border-rose-900/50">
            Critical Risk
          </Chip>
        );
    }
  };

  return (
    <div className="space-y-6 w-full min-w-0">
      {/* Header Banner with Safety Invariant */}
      <div className="border border-slate-200 dark:border-workstation-border rounded-xs bg-white dark:bg-workstation-panel p-4 space-y-3">
        <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
          <div className="flex items-center space-x-3">
            <div className="w-8 h-8 rounded-xs bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-900/40 flex items-center justify-center text-amber-700 dark:text-amber-400">
              <ShieldAlert className="w-4 h-4" />
            </div>
            <div>
              <h1 className="text-sm font-bold text-slate-900 dark:text-slate-100 uppercase tracking-wider">
                Safety & Authorization Interlocks
              </h1>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                Pre-execution validation, device classification verification, and fail-closed gates.
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Button
              size="sm"
              variant="flat"
              onPress={() => {
                loadDevices();
                loadHistory();
              }}
              isDisabled={devicesLoading}
              startContent={<RefreshCw className={`w-3.5 h-3.5 ${devicesLoading ? 'animate-spin' : ''}`} />}
              className="text-xs font-medium bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border text-slate-700 dark:text-slate-300"
            >
              Refresh
            </Button>
          </div>
        </div>

        {/* Global Safety Notice */}
        <div className="p-3 bg-amber-50/60 dark:bg-amber-950/20 border border-amber-200 dark:border-amber-900/40 rounded-xs text-xs text-amber-900 dark:text-amber-200 flex items-start gap-2.5">
          <AlertOctagon className="w-4 h-4 text-amber-700 dark:text-amber-400 shrink-0 mt-0.5" />
          <div className="space-y-1">
            <p className="font-semibold text-amber-950 dark:text-amber-100">
              CRITICAL INVARIANT: DEVICE CLASSIFICATION IS NOT AUTHORIZATION
            </p>
            <p className="text-[11px] text-amber-800 dark:text-amber-300">
              Identifying a storage device as external or removable does not grant permission. Hard blocks on system/boot devices can never be overridden. Destructive executors are permanently disabled at this foundation stage.
            </p>
          </div>
        </div>
      </div>

      {/* KPI Stats Ribbon (Information-First Inline Metric Group, No Cards) */}
      <div className="grid grid-cols-2 lg:grid-cols-4 divide-y sm:divide-y-0 sm:divide-x divide-slate-200 dark:divide-workstation-border border border-slate-200 dark:border-workstation-border rounded-xs bg-white dark:bg-workstation-panel">
        <div className="p-3.5">
          <div className="flex items-center justify-between">
            <span className="text-[11px] uppercase tracking-wider font-semibold text-slate-500 dark:text-slate-400">Active Operator</span>
            <Lock className="w-3.5 h-3.5 text-slate-400" />
          </div>
          <div className="mt-1 flex items-baseline gap-2">
            <span className="text-base font-bold text-slate-900 dark:text-slate-100">{currentUser?.username || 'Unauthenticated'}</span>
            <span className="text-[10px] font-semibold text-slate-600 dark:text-slate-400">({currentUser?.role || 'None'})</span>
          </div>
          <p className="mt-0.5 text-[10px] text-slate-400">Server-side RBAC enforced</p>
        </div>

        <div className="p-3.5">
          <div className="flex items-center justify-between">
            <span className="text-[11px] uppercase tracking-wider font-semibold text-slate-500 dark:text-slate-400">System Protection</span>
            <ShieldCheck className="w-3.5 h-3.5 text-slate-500" />
          </div>
          <div className="mt-1 text-base font-bold text-slate-900 dark:text-slate-100">HARD BLOCKED</div>
          <p className="mt-0.5 text-[10px] text-slate-400">OS root & boot volumes protected</p>
        </div>

        <div className="p-3.5">
          <div className="flex items-center justify-between">
            <span className="text-[11px] uppercase tracking-wider font-semibold text-slate-500 dark:text-slate-400">TOCTOU Defense</span>
            <Layers className="w-3.5 h-3.5 text-slate-500" />
          </div>
          <div className="mt-1 text-base font-bold text-slate-900 dark:text-slate-100">ACTIVE</div>
          <p className="mt-0.5 text-[10px] text-slate-400">Live snapshot revalidation</p>
        </div>

        <div className="p-3.5">
          <div className="flex items-center justify-between">
            <span className="text-[11px] uppercase tracking-wider font-semibold text-slate-500 dark:text-slate-400">Destructive Engines</span>
            <ShieldX className="w-3.5 h-3.5 text-rose-500" />
          </div>
          <div className="mt-1 text-base font-bold text-rose-600 dark:text-rose-400">DISABLED</div>
          <p className="mt-0.5 text-[10px] text-slate-400">Foundation phase safety invariant</p>
        </div>
      </div>

      {/* Target Evaluation Console */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left Column: Form */}
        <div className="lg:col-span-5 space-y-4">
          <div className="border border-slate-200 dark:border-workstation-border rounded-xs bg-white dark:bg-workstation-panel p-4 space-y-4">
            <h2 className="text-xs font-bold text-slate-800 dark:text-slate-200 uppercase tracking-wider">
              1. Select Target & Operation
            </h2>

            {/* Target Select */}
            <div>
              <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1">
                Discovered Physical Device / Volume
              </label>
              <select
                value={selectedTargetId}
                onChange={(e) => {
                  setSelectedTargetId(e.target.value);
                  setCustomPath('');
                  const dev = devices.find((d) => d.device_id === e.target.value);
                  if (dev) {
                    setSelectedTargetType('PhysicalDevice');
                  }
                }}
                className="w-full text-xs px-3 py-2 border border-slate-200 dark:border-workstation-border rounded-xs bg-white dark:bg-workstation-surface text-slate-900 dark:text-slate-100 focus:outline-none"
              >
                {devices.map((d) => (
                  <option key={d.device_id} value={d.device_id}>
                    {d.display_name} ({d.classification.replace('_', ' ').toUpperCase()}) - {formatBytes(d.capacity_bytes)}
                  </option>
                ))}
              </select>
            </div>

            {/* Custom Path Override */}
            <div>
              <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1">
                Or Custom Path / Mount Point (Optional)
              </label>
              <Input
                size="sm"
                placeholder="e.g. C:\ or C:\Evidence\file.bin"
                value={customPath}
                onValueChange={setCustomPath}
                className="font-mono text-xs"
                classNames={{
                  inputWrapper: 'bg-white dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs',
                }}
              />
              <p className="text-[11px] text-slate-400 mt-1">
                Specifying a system path like C:\ will test the hard system block.
              </p>
            </div>

            {/* Operation Type Select */}
            <div>
              <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1">
                Operation Category
              </label>
              <select
                value={selectedOpType}
                onChange={(e) => setSelectedOpType(e.target.value as OperationType)}
                className="w-full text-xs px-3 py-2 border border-slate-200 dark:border-workstation-border rounded-xs bg-white dark:bg-workstation-surface text-slate-900 dark:text-slate-100 focus:outline-none"
              >
                <option value="DriveErasure">DriveErasure (Destructive / Disabled)</option>
                <option value="FileErasure">FileErasure (Destructive / Disabled)</option>
                <option value="FolderErasure">FolderErasure (Destructive / Disabled)</option>
                <option value="IntegrityHash">IntegrityHash (Read-Only / Executable)</option>
                <option value="IntegrityVerify">IntegrityVerify (Read-Only / Executable)</option>
              </select>
            </div>

            {errorMsg && (
              <div className="p-3 text-xs bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 rounded-xs text-rose-700 dark:text-rose-300">
                {errorMsg}
              </div>
            )}

            <Button
              onPress={handleEvaluate}
              isDisabled={evaluating}
              isLoading={evaluating}
              startContent={!evaluating && <ShieldAlert className="w-3.5 h-3.5" />}
              className="w-full bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 text-xs font-semibold py-2 shadow-xs rounded-xs"
            >
              Evaluate Target & Safety Gate
            </Button>
          </div>
        </div>

        {/* Right Column: Decision Verdict */}
        <div className="lg:col-span-7">
          <div className="border border-slate-200 dark:border-workstation-border rounded-xs bg-white dark:bg-workstation-panel p-4 space-y-4 h-full">
            <div className="flex items-center justify-between border-b border-slate-200 dark:border-workstation-border pb-3">
              <h2 className="text-xs font-bold text-slate-800 dark:text-slate-200 uppercase tracking-wider">
                2. Safety Gate Verdict
              </h2>
              {currentDecision && getDecisionBadge(currentDecision.decision)}
            </div>

            {currentDecision ? (
              <div className="space-y-4">
                {/* Status & Code */}
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-mono text-xs px-2.5 py-1 bg-slate-100 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border rounded-xs text-slate-800 dark:text-slate-200 font-semibold">
                    {currentDecision.reason_code}
                  </span>
                  {getRiskBadge(currentDecision.risk_level)}
                  <span className="text-xs text-slate-400">
                    Evaluated: {new Date(currentDecision.evaluated_at).toLocaleTimeString()}
                  </span>
                </div>

                {/* Explanation Box */}
                <div
                  className={`p-3.5 rounded-xs border text-xs leading-relaxed ${
                    currentDecision.decision === 'Blocked'
                      ? 'bg-rose-50/70 dark:bg-rose-950/30 border-rose-200 dark:border-rose-900/40 text-rose-900 dark:text-rose-200'
                      : currentDecision.decision === 'RequiresConfirmation'
                      ? 'bg-amber-50/70 dark:bg-amber-950/30 border-amber-200 dark:border-amber-900/40 text-amber-900 dark:text-amber-200'
                      : currentDecision.decision === 'Allowed'
                      ? 'bg-slate-100 dark:bg-workstation-surface border-slate-300 dark:border-workstation-border text-slate-900 dark:text-slate-100'
                      : 'bg-rose-50/70 dark:bg-rose-950/30 border-rose-200 dark:border-rose-900/40 text-rose-900 dark:text-rose-200'
                  }`}
                >
                  <p className="font-semibold mb-1">Evaluation Details:</p>
                  <p>{currentDecision.message}</p>
                </div>

                {/* Target Architecture Inspection */}
                {currentDecision.target_snapshot && (
                  <div className="border border-slate-200 dark:border-workstation-border rounded-xs p-3.5 bg-slate-50/50 dark:bg-workstation-surface space-y-2.5">
                    <h3 className="text-xs font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5 uppercase tracking-wider">
                      <Cpu className="w-3.5 h-3.5 text-slate-500" />
                      Live Target Snapshot & Geometry
                    </h3>
                    <div className="grid grid-cols-2 gap-x-4 gap-y-2 text-xs font-mono">
                      <div>
                        <span className="text-slate-400">Identifier: </span>
                        <span className="text-slate-800 dark:text-slate-200 font-semibold">{currentDecision.target_snapshot.target_identifier}</span>
                      </div>
                      <div>
                        <span className="text-slate-400">Capacity: </span>
                        <span className="text-slate-800 dark:text-slate-200">{formatBytes(currentDecision.target_snapshot.capacity_bytes)}</span>
                      </div>
                      <div>
                        <span className="text-slate-400">Classification: </span>
                        <span className="text-slate-800 dark:text-slate-200">{currentDecision.target_snapshot.classification || 'N/A'}</span>
                      </div>
                      <div>
                        <span className="text-slate-400">Filesystem: </span>
                        <span className="text-slate-800 dark:text-slate-200">{currentDecision.target_snapshot.filesystem || 'Raw / None'}</span>
                      </div>
                      <div>
                        <span className="text-slate-400">System Partition: </span>
                        <span className={currentDecision.target_snapshot.is_system ? 'text-rose-700 dark:text-rose-400 font-bold' : 'text-slate-800 dark:text-slate-200'}>
                          {currentDecision.target_snapshot.is_system ? 'YES (PROTECTED)' : 'NO'}
                        </span>
                      </div>
                      <div>
                        <span className="text-slate-400">Boot Partition: </span>
                        <span className={currentDecision.target_snapshot.is_boot ? 'text-rose-700 dark:text-rose-400 font-bold' : 'text-slate-800 dark:text-slate-200'}>
                          {currentDecision.target_snapshot.is_boot ? 'YES (PROTECTED)' : 'NO'}
                        </span>
                      </div>
                    </div>
                  </div>
                )}

                {/* Two-Stage Confirmation Action */}
                {currentDecision.decision === 'RequiresConfirmation' && (
                  <div className="p-4 bg-amber-50/50 dark:bg-amber-950/20 border border-amber-200 dark:border-amber-900/40 rounded-xs space-y-2">
                    <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                      <div>
                        <h4 className="text-xs font-semibold text-amber-900 dark:text-amber-200">
                          Stage 1 Review Complete — Confirmation Required
                        </h4>
                        <p className="text-[11px] text-amber-700 dark:text-amber-400">
                          Generate a time-bound challenge bound to this exact target snapshot.
                        </p>
                      </div>
                      <Button
                        size="sm"
                        onPress={handleRequestConfirmation}
                        isDisabled={evaluating}
                        endContent={<ArrowRight className="w-3.5 h-3.5" />}
                        className="bg-amber-600 hover:bg-amber-700 text-white text-xs font-semibold shadow-xs rounded-xs"
                      >
                        Request Confirmation
                      </Button>
                    </div>
                  </div>
                )}

                {confirmResult && (
                  <div className="p-3.5 bg-slate-100 dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border rounded-xs text-xs text-slate-800 dark:text-slate-200 space-y-1">
                    <div className="flex items-center gap-1.5 font-semibold text-slate-900 dark:text-slate-100">
                      <CheckCircle2 className="w-4 h-4 text-slate-700 dark:text-slate-300" />
                      Confirmation Committed (ID: {confirmResult.evaluation_id.slice(0, 8)}...)
                    </div>
                    <p className="text-[11px] text-slate-600 dark:text-slate-400">
                      {confirmResult.message}
                    </p>
                  </div>
                )}
              </div>
            ) : (
              <div className="py-16 text-center space-y-2 text-slate-400">
                <ShieldAlert className="w-10 h-10 mx-auto text-slate-300 dark:text-slate-600" />
                <p className="text-xs font-medium">Select a target and click &ldquo;Evaluate Target & Safety Gate&rdquo;</p>
                <p className="text-[11px] text-slate-400">
                  Evaluates RBAC role permissions, OS protection, risk classification, and method compatibility.
                </p>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Confirmation Modal */}
      <Modal
        isOpen={isConfirmModalOpen && !!challenge}
        onClose={() => setIsConfirmModalOpen(false)}
        size="lg"
        classNames={{
          base: 'bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border',
          header: 'border-b border-slate-100 dark:border-workstation-border',
          footer: 'border-t border-slate-100 dark:border-workstation-border',
        }}
      >
        <ModalContent>
          {challenge && (
            <>
              <ModalHeader className="flex items-center gap-2 text-amber-700 dark:text-amber-400">
                <AlertTriangle className="w-5 h-5" />
                <span className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                  Stage 2: Explicit Destructive Confirmation
                </span>
              </ModalHeader>

              <ModalBody className="py-4 space-y-3 text-xs text-slate-700 dark:text-slate-300">
                <div className="p-3 bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 rounded text-rose-900 dark:text-rose-200 space-y-1">
                  <p className="font-semibold">WARNING: HIGH-RISK SANITIZATION ACTION</p>
                  <p className="text-[11px]">
                    All partitions, filesystem structures, and data on this target would be unrecoverable.
                  </p>
                </div>

                <div className="border border-slate-200 dark:border-workstation-border rounded p-3 bg-slate-50 dark:bg-workstation-surface space-y-1.5 font-mono text-[11px]">
                  <div><span className="text-slate-400">Operation ID:</span> {challenge.operation_id}</div>
                  <div><span className="text-slate-400">Target:</span> {challenge.target.identifier}</div>
                  <div><span className="text-slate-400">Snapshot Capacity:</span> {formatBytes(challenge.target_snapshot.capacity_bytes)}</div>
                  <div><span className="text-slate-400">Challenge TTL:</span> 5 minutes (Expires: {new Date(challenge.expires_at).toLocaleTimeString()})</div>
                </div>

                {/* Warning Acknowledgment */}
                <Checkbox
                  size="sm"
                  color="danger"
                  isSelected={warningAck}
                  onValueChange={setWarningAck}
                  classNames={{
                    label: 'text-xs text-slate-800 dark:text-slate-200',
                  }}
                >
                  I understand the consequences and confirm that I intend to target this specific storage device.
                </Checkbox>

                {/* Typed Confirmation */}
                <div>
                  <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1">
                    Type <span className="font-mono font-bold text-slate-900 dark:text-slate-100">{challenge.target.identifier}</span> to confirm:
                  </label>
                  <Input
                    size="sm"
                    placeholder={challenge.target.identifier}
                    value={typedTargetInput}
                    onValueChange={setTypedTargetInput}
                    className="font-mono text-xs"
                    classNames={{
                      inputWrapper: 'bg-white dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border',
                    }}
                  />
                </div>

                <p className="text-[11px] text-slate-500 italic">
                  * Note: Submitting confirmation will re-verify the live device to prevent TOCTOU substitution, and safely terminate because destructive executors are disabled.
                </p>
              </ModalBody>

              <ModalFooter>
                <Button
                  size="sm"
                  variant="flat"
                  onPress={() => setIsConfirmModalOpen(false)}
                  className="text-xs border border-slate-200 dark:border-workstation-border bg-white dark:bg-workstation-panel text-slate-700 dark:text-slate-300"
                >
                  Cancel
                </Button>
                <Button
                  size="sm"
                  onPress={handleSubmitConfirmation}
                  isDisabled={!warningAck || typedTargetInput.trim() !== challenge.target.identifier.trim() || submittingConfirm}
                  isLoading={submittingConfirm}
                  startContent={!submittingConfirm && <Check className="w-3.5 h-3.5" />}
                  className={`text-xs font-semibold px-4 text-white ${
                    warningAck && typedTargetInput.trim() === challenge.target.identifier.trim() && !submittingConfirm
                      ? 'bg-rose-600 hover:bg-rose-700'
                      : 'bg-slate-300 dark:bg-slate-700 cursor-not-allowed'
                  }`}
                >
                  Confirm Operation
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>

      {/* Evaluation Audit History Table */}
      <div className="border border-slate-200 dark:border-workstation-border rounded-xs bg-white dark:bg-workstation-panel overflow-hidden">
        <div className="px-4 py-3 border-b border-slate-200 dark:border-workstation-border flex items-center justify-between">
          <h3 className="text-xs font-bold text-slate-800 dark:text-slate-200 uppercase tracking-wider flex items-center gap-1.5">
            <Clock className="w-3.5 h-3.5 text-slate-400" />
            Recent Safety Evaluations (Tamper-Evident Audit Record)
          </h3>
          <span className="text-[10px] font-mono text-slate-400">{evalHistory.length} Record(s)</span>
        </div>

        <Table
          aria-label="Safety Evaluations Table"
          removeWrapper
          classNames={{
            base: 'min-w-full',
            table: 'min-w-full',
            thead: 'border-b border-slate-200 dark:border-workstation-border',
            th: 'bg-slate-50 dark:bg-workstation-surface text-slate-500 dark:text-slate-400 text-[10px] font-bold uppercase tracking-wider h-8 py-0 px-3',
            td: 'py-2 px-3 text-xs text-slate-700 dark:text-slate-300 border-b border-slate-100 dark:border-workstation-border',
          }}
        >
          <TableHeader>
            <TableColumn>TIMESTAMP</TableColumn>
            <TableColumn>ACTOR</TableColumn>
            <TableColumn>OPERATION</TableColumn>
            <TableColumn>TARGET</TableColumn>
            <TableColumn>DECISION</TableColumn>
            <TableColumn>REASON CODE</TableColumn>
            <TableColumn>RISK</TableColumn>
          </TableHeader>
          <TableBody emptyContent="No safety evaluations recorded yet.">
            {evalHistory.map((item) => (
              <TableRow key={item.evaluation_id} className="hover:bg-slate-50/50 dark:hover:bg-workstation-surface/50">
                <TableCell className="font-mono text-[11px] text-slate-400">
                  {new Date(item.evaluated_at).toLocaleTimeString()}
                </TableCell>
                <TableCell className="font-medium text-slate-900 dark:text-slate-100">
                  {item.actor_id || 'System'}
                </TableCell>
                <TableCell className="font-mono text-xs">
                  {item.operation_type}
                </TableCell>
                <TableCell className="font-mono text-xs text-slate-600 dark:text-slate-400 truncate max-w-xs" title={item.target.identifier}>
                  {item.target.identifier}
                </TableCell>
                <TableCell>
                  {getDecisionBadge(item.decision)}
                </TableCell>
                <TableCell className="font-mono text-[11px]">
                  {item.reason_code}
                </TableCell>
                <TableCell>
                  {getRiskBadge(item.risk_level)}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </div>
  );
};

export default SafetyInterlocksPage;
