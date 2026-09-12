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
    } catch (err: any) {
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
    } catch (err: any) {
      setErrorMsg(err?.message || 'Failed to evaluate target safety.');
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
    } catch (err: any) {
      setErrorMsg(err?.message || 'Failed to request destructive confirmation.');
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
    } catch (err: any) {
      setErrorMsg(err?.message || 'Confirmation failed.');
    } finally {
      setSubmittingConfirm(false);
    }
  };

  const getDecisionBadge = (decision: SafetyDecisionOutcome) => {
    switch (decision) {
      case 'Allowed':
        return (
          <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200">
            <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600" />
            ALLOWED
          </span>
        );
      case 'RequiresConfirmation':
        return (
          <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-amber-50 text-amber-700 border border-amber-200">
            <AlertTriangle className="w-3.5 h-3.5 text-amber-600" />
            REQUIRES CONFIRMATION
          </span>
        );
      case 'Denied':
        return (
          <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-rose-50 text-rose-700 border border-rose-200">
            <XCircle className="w-3.5 h-3.5 text-rose-600" />
            DENIED
          </span>
        );
      case 'Blocked':
        return (
          <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-red-100 text-red-800 border border-red-300">
            <Lock className="w-3.5 h-3.5 text-red-600" />
            HARD BLOCKED
          </span>
        );
    }
  };

  const getRiskBadge = (risk: RiskLevel) => {
    switch (risk) {
      case 'Low':
        return <span className="px-2 py-0.5 text-[11px] font-medium rounded bg-slate-100 text-slate-700 border border-slate-200">Low Risk</span>;
      case 'Medium':
        return <span className="px-2 py-0.5 text-[11px] font-medium rounded bg-amber-50 text-amber-800 border border-amber-200">Medium Risk</span>;
      case 'High':
        return <span className="px-2 py-0.5 text-[11px] font-medium rounded bg-orange-50 text-orange-800 border border-orange-200">High Risk</span>;
      case 'Critical':
        return <span className="px-2 py-0.5 text-[11px] font-medium rounded bg-red-50 text-red-800 border border-red-200 font-bold">Critical Risk</span>;
    }
  };

  return (
    <div className="space-y-6 max-w-7xl mx-auto">
      {/* Header Banner with Safety Invariant */}
      <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-2xs">
        <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
          <div className="flex items-center space-x-3">
            <div className="w-10 h-10 rounded-md bg-amber-50 border border-amber-200 flex items-center justify-center text-amber-700">
              <ShieldAlert className="w-5 h-5" />
            </div>
            <div>
              <h1 className="text-base font-semibold text-slate-900 tracking-tight">
                Safety & Authorization Interlocks
              </h1>
              <p className="text-xs text-slate-500">
                Pre-execution validation, device classification verification, and fail-closed gates.
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={() => {
                loadDevices();
                loadHistory();
              }}
              disabled={devicesLoading}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded border border-slate-200 bg-slate-50 text-slate-700 hover:bg-slate-100 transition-colors"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${devicesLoading ? 'animate-spin' : ''}`} />
              Refresh
            </button>
          </div>
        </div>

        {/* Global Safety Notice */}
        <div className="mt-4 p-3 bg-amber-50/60 border border-amber-200 rounded text-xs text-amber-900 flex items-start gap-2.5">
          <AlertOctagon className="w-4 h-4 text-amber-700 shrink-0 mt-0.5" />
          <div className="space-y-1">
            <p className="font-semibold text-amber-950">
              CRITICAL INVARIANT: DEVICE CLASSIFICATION IS NOT AUTHORIZATION
            </p>
            <p className="text-[11px] text-amber-800">
              Identifying a storage device as external or removable does not grant permission. Hard blocks on system/boot devices can never be overridden. Destructive executors are permanently disabled at this foundation stage.
            </p>
          </div>
        </div>
      </div>

      {/* KPI Stats Bar */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-white border border-slate-200 rounded-lg p-4 shadow-2xs">
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium text-slate-500">Active Operator</span>
            <Lock className="w-4 h-4 text-slate-400" />
          </div>
          <div className="mt-2 flex items-baseline gap-2">
            <span className="text-xl font-bold text-slate-900">{currentUser?.username || 'Unauthenticated'}</span>
            <span className="text-[11px] font-semibold text-purple-700">({currentUser?.role || 'None'})</span>
          </div>
          <p className="mt-1 text-[11px] text-slate-400">Server-side RBAC enforced</p>
        </div>

        <div className="bg-white border border-slate-200 rounded-lg p-4 shadow-2xs">
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium text-slate-500">System Protection</span>
            <ShieldCheck className="w-4 h-4 text-emerald-600" />
          </div>
          <div className="mt-2 text-xl font-bold text-emerald-700">HARD BLOCKED</div>
          <p className="mt-1 text-[11px] text-slate-400">OS root & boot volumes protected</p>
        </div>

        <div className="bg-white border border-slate-200 rounded-lg p-4 shadow-2xs">
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium text-slate-500">TOCTOU Defense</span>
            <Layers className="w-4 h-4 text-indigo-600" />
          </div>
          <div className="mt-2 text-xl font-bold text-indigo-700">ACTIVE</div>
          <p className="mt-1 text-[11px] text-slate-400">Live snapshot revalidation</p>
        </div>

        <div className="bg-white border border-slate-200 rounded-lg p-4 shadow-2xs">
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium text-slate-500">Destructive Engines</span>
            <ShieldX className="w-4 h-4 text-red-500" />
          </div>
          <div className="mt-2 text-xl font-bold text-red-600">DISABLED</div>
          <p className="mt-1 text-[11px] text-slate-400">Foundation phase safety invariant</p>
        </div>
      </div>

      {/* Target Evaluation Console */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left Column: Form */}
        <div className="lg:col-span-5 space-y-4">
          <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-2xs space-y-4">
            <h2 className="text-xs font-semibold text-slate-700 uppercase tracking-wider">
              1. Select Target & Operation
            </h2>

            {/* Target Select */}
            <div>
              <label className="block text-xs font-medium text-slate-700 mb-1">
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
                className="w-full text-xs px-3 py-2 border border-slate-200 rounded bg-white text-slate-900 focus:outline-none focus:ring-1 focus:ring-slate-400"
              >
                {devices.map((d) => (
                  <option key={d.device_id} value={d.device_id}>
                    {d.display_name} ({d.classification.replace('_', ' ').toUpperCase()}) - {formatBytes(d.capacity_bytes)}
                  </option>
                ))}
              </select>
            </div>

            {/* Custom Path Override (for Files/Folders/Volumes) */}
            <div>
              <label className="block text-xs font-medium text-slate-700 mb-1">
                Or Custom Path / Mount Point (Optional)
              </label>
              <input
                type="text"
                placeholder="e.g. C:\ or C:\Evidence\file.bin"
                value={customPath}
                onChange={(e) => setCustomPath(e.target.value)}
                className="w-full text-xs px-3 py-2 border border-slate-200 rounded bg-white text-slate-900 font-mono focus:outline-none focus:ring-1 focus:ring-slate-400"
              />
              <p className="text-[11px] text-slate-400 mt-1">
                Specifying a system path like C:\ will test the hard system block.
              </p>
            </div>

            {/* Operation Type Select */}
            <div>
              <label className="block text-xs font-medium text-slate-700 mb-1">
                Operation Category
              </label>
              <select
                value={selectedOpType}
                onChange={(e) => setSelectedOpType(e.target.value as OperationType)}
                className="w-full text-xs px-3 py-2 border border-slate-200 rounded bg-white text-slate-900 focus:outline-none focus:ring-1 focus:ring-slate-400"
              >
                <option value="DriveErasure">DriveErasure (Destructive / Disabled)</option>
                <option value="FileErasure">FileErasure (Destructive / Disabled)</option>
                <option value="FolderErasure">FolderErasure (Destructive / Disabled)</option>
                <option value="IntegrityHash">IntegrityHash (Read-Only / Executable)</option>
                <option value="IntegrityVerify">IntegrityVerify (Read-Only / Executable)</option>
              </select>
            </div>

            {errorMsg && (
              <div className="p-3 text-xs bg-rose-50 border border-rose-200 rounded text-rose-700">
                {errorMsg}
              </div>
            )}

            <button
              onClick={handleEvaluate}
              disabled={evaluating}
              className="w-full py-2 px-4 text-xs font-semibold rounded bg-slate-900 text-white hover:bg-slate-800 transition-colors flex items-center justify-center gap-2 shadow-xs"
            >
              {evaluating ? (
                <RefreshCw className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <ShieldAlert className="w-3.5 h-3.5" />
              )}
              Evaluate Target & Safety Gate
            </button>
          </div>
        </div>

        {/* Right Column: Decision Verdict Card */}
        <div className="lg:col-span-7">
          <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-2xs space-y-5 h-full">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3">
              <h2 className="text-xs font-semibold text-slate-700 uppercase tracking-wider">
                2. Safety Gate Verdict
              </h2>
              {currentDecision && getDecisionBadge(currentDecision.decision)}
            </div>

            {currentDecision ? (
              <div className="space-y-4">
                {/* Status & Code */}
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-mono text-xs px-2.5 py-1 bg-slate-100 border border-slate-200 rounded text-slate-800 font-semibold">
                    {currentDecision.reason_code}
                  </span>
                  {getRiskBadge(currentDecision.risk_level)}
                  <span className="text-xs text-slate-400">
                    Evaluated: {new Date(currentDecision.evaluated_at).toLocaleTimeString()}
                  </span>
                </div>

                {/* Explanation Box */}
                <div
                  className={`p-3.5 rounded-md border text-xs leading-relaxed ${
                    currentDecision.decision === 'Blocked'
                      ? 'bg-red-50/70 border-red-200 text-red-900'
                      : currentDecision.decision === 'RequiresConfirmation'
                      ? 'bg-amber-50/70 border-amber-200 text-amber-900'
                      : currentDecision.decision === 'Allowed'
                      ? 'bg-emerald-50/70 border-emerald-200 text-emerald-900'
                      : 'bg-rose-50/70 border-rose-200 text-rose-900'
                  }`}
                >
                  <p className="font-semibold mb-1">Evaluation Details:</p>
                  <p>{currentDecision.message}</p>
                </div>

                {/* Target Architecture Inspection */}
                {currentDecision.target_snapshot && (
                  <div className="border border-slate-200 rounded p-3.5 bg-slate-50/50 space-y-2.5">
                    <h3 className="text-xs font-semibold text-slate-800 flex items-center gap-1.5">
                      <Cpu className="w-3.5 h-3.5 text-slate-500" />
                      Live Target Snapshot & Geometry
                    </h3>
                    <div className="grid grid-cols-2 gap-x-4 gap-y-2 text-xs font-mono">
                      <div>
                        <span className="text-slate-400">Identifier: </span>
                        <span className="text-slate-800 font-semibold">{currentDecision.target_snapshot.target_identifier}</span>
                      </div>
                      <div>
                        <span className="text-slate-400">Capacity: </span>
                        <span className="text-slate-800">{formatBytes(currentDecision.target_snapshot.capacity_bytes)}</span>
                      </div>
                      <div>
                        <span className="text-slate-400">Classification: </span>
                        <span className="text-slate-800">{currentDecision.target_snapshot.classification || 'N/A'}</span>
                      </div>
                      <div>
                        <span className="text-slate-400">Filesystem: </span>
                        <span className="text-slate-800">{currentDecision.target_snapshot.filesystem || 'Raw / None'}</span>
                      </div>
                      <div>
                        <span className="text-slate-400">System Partition: </span>
                        <span className={currentDecision.target_snapshot.is_system ? 'text-red-700 font-bold' : 'text-slate-800'}>
                          {currentDecision.target_snapshot.is_system ? 'YES (PROTECTED)' : 'NO'}
                        </span>
                      </div>
                      <div>
                        <span className="text-slate-400">Boot Partition: </span>
                        <span className={currentDecision.target_snapshot.is_boot ? 'text-red-700 font-bold' : 'text-slate-800'}>
                          {currentDecision.target_snapshot.is_boot ? 'YES (PROTECTED)' : 'NO'}
                        </span>
                      </div>
                    </div>
                  </div>
                )}

                {/* Two-Stage Confirmation Action */}
                {currentDecision.decision === 'RequiresConfirmation' && (
                  <div className="p-4 bg-amber-50/50 border border-amber-200 rounded-md space-y-2">
                    <div className="flex items-center justify-between">
                      <div>
                        <h4 className="text-xs font-semibold text-amber-900">
                          Stage 1 Review Complete — Confirmation Required
                        </h4>
                        <p className="text-[11px] text-amber-700">
                          Generate a time-bound challenge bound to this exact target snapshot.
                        </p>
                      </div>
                      <button
                        onClick={handleRequestConfirmation}
                        disabled={evaluating}
                        className="px-3.5 py-1.5 text-xs font-semibold rounded bg-amber-600 text-white hover:bg-amber-700 transition-colors flex items-center gap-1.5 shadow-xs"
                      >
                        Request Confirmation
                        <ArrowRight className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>
                )}

                {confirmResult && (
                  <div className="p-3.5 bg-slate-100 border border-slate-300 rounded text-xs text-slate-800 space-y-1">
                    <div className="flex items-center gap-1.5 font-semibold text-emerald-800">
                      <CheckCircle2 className="w-4 h-4 text-emerald-600" />
                      Confirmation Committed (ID: {confirmResult.evaluation_id.slice(0, 8)}...)
                    </div>
                    <p className="text-[11px] text-slate-600">
                      {confirmResult.message}
                    </p>
                  </div>
                )}
              </div>
            ) : (
              <div className="py-16 text-center space-y-2 text-slate-400">
                <ShieldAlert className="w-10 h-10 mx-auto text-slate-300" />
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
      {isConfirmModalOpen && challenge && (
        <div className="fixed inset-0 bg-slate-900/40 backdrop-blur-2xs flex items-center justify-center p-4 z-50">
          <div className="bg-white border border-slate-200 rounded-lg max-w-lg w-full p-6 shadow-xl space-y-4">
            <div className="flex items-center justify-between border-b border-slate-100 pb-3">
              <div className="flex items-center gap-2 text-amber-700">
                <AlertTriangle className="w-5 h-5" />
                <h3 className="text-sm font-semibold text-slate-900">
                  Stage 2: Explicit Destructive Confirmation
                </h3>
              </div>
              <button
                onClick={() => setIsConfirmModalOpen(false)}
                className="text-slate-400 hover:text-slate-600 text-xs font-mono"
              >
                ✕
              </button>
            </div>

            <div className="space-y-3 text-xs text-slate-700">
              <div className="p-3 bg-red-50 border border-red-200 rounded text-red-900 space-y-1">
                <p className="font-semibold">WARNING: HIGH-RISK SANITIZATION ACTION</p>
                <p className="text-[11px]">
                  All partitions, filesystem structures, and data on this target would be unrecoverable.
                </p>
              </div>

              <div className="border border-slate-200 rounded p-3 bg-slate-50 space-y-1.5 font-mono text-[11px]">
                <div><span className="text-slate-400">Operation ID:</span> {challenge.operation_id}</div>
                <div><span className="text-slate-400">Target:</span> {challenge.target.identifier}</div>
                <div><span className="text-slate-400">Snapshot Capacity:</span> {formatBytes(challenge.target_snapshot.capacity_bytes)}</div>
                <div><span className="text-slate-400">Challenge TTL:</span> 5 minutes (Expires: {new Date(challenge.expires_at).toLocaleTimeString()})</div>
              </div>

              {/* Warning Acknowledgment */}
              <label className="flex items-start gap-2 pt-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={warningAck}
                  onChange={(e) => setWarningAck(e.target.checked)}
                  className="rounded border-slate-300 text-slate-900 focus:ring-slate-500 mt-0.5"
                />
                <span className="text-xs text-slate-800">
                  I understand the consequences and confirm that I intend to target this specific storage device.
                </span>
              </label>

              {/* Typed Confirmation */}
              <div>
                <label className="block text-xs font-medium text-slate-700 mb-1">
                  Type <span className="font-mono font-bold text-slate-900">{challenge.target.identifier}</span> to confirm:
                </label>
                <input
                  type="text"
                  placeholder={challenge.target.identifier}
                  value={typedTargetInput}
                  onChange={(e) => setTypedTargetInput(e.target.value)}
                  className="w-full text-xs px-3 py-2 border border-slate-200 rounded bg-white text-slate-900 font-mono focus:outline-none focus:ring-1 focus:ring-slate-400"
                />
              </div>

              {/* Safety notice in modal */}
              <p className="text-[11px] text-slate-500 italic">
                * Note: Submitting confirmation will re-verify the live device to prevent TOCTOU substitution, and safely terminate because destructive executors are disabled.
              </p>
            </div>

            <div className="flex justify-end gap-2 pt-2 border-t border-slate-100">
              <button
                onClick={() => setIsConfirmModalOpen(false)}
                className="px-3 py-1.5 text-xs font-medium rounded border border-slate-200 text-slate-600 hover:bg-slate-50"
              >
                Cancel
              </button>
              <button
                onClick={handleSubmitConfirmation}
                disabled={!warningAck || typedTargetInput.trim() !== challenge.target.identifier.trim() || submittingConfirm}
                className={`px-4 py-1.5 text-xs font-semibold rounded text-white transition-colors flex items-center gap-1.5 shadow-xs ${
                  warningAck && typedTargetInput.trim() === challenge.target.identifier.trim() && !submittingConfirm
                    ? 'bg-red-600 hover:bg-red-700'
                    : 'bg-slate-300 cursor-not-allowed'
                }`}
              >
                {submittingConfirm ? (
                  <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <Check className="w-3.5 h-3.5" />
                )}
                Confirm Operation
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Evaluation Audit History Table */}
      <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-2xs space-y-3">
        <h3 className="text-xs font-semibold text-slate-700 uppercase tracking-wider flex items-center gap-1.5">
          <Clock className="w-3.5 h-3.5 text-slate-400" />
          Recent Safety Evaluations (Tamper-Evident Audit Record)
        </h3>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs text-slate-700">
            <thead className="bg-slate-50 border-b border-slate-200 text-slate-500 font-medium text-[11px] uppercase tracking-wider">
              <tr>
                <th className="py-2.5 px-3">Timestamp</th>
                <th className="py-2.5 px-3">Actor</th>
                <th className="py-2.5 px-3">Operation</th>
                <th className="py-2.5 px-3">Target</th>
                <th className="py-2.5 px-3">Decision</th>
                <th className="py-2.5 px-3">Reason Code</th>
                <th className="py-2.5 px-3">Risk</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100 font-mono text-[11px]">
              {evalHistory.length === 0 ? (
                <tr>
                  <td colSpan={7} className="py-4 text-center text-slate-400 italic">
                    No safety evaluations recorded yet.
                  </td>
                </tr>
              ) : (
                evalHistory.map((item) => (
                  <tr key={item.evaluation_id} className="hover:bg-slate-50/50">
                    <td className="py-2.5 px-3 text-slate-500">
                      {new Date(item.evaluated_at).toLocaleTimeString()}
                    </td>
                    <td className="py-2.5 px-3 text-slate-900 font-sans font-medium">
                      {item.actor_id || 'System'}
                    </td>
                    <td className="py-2.5 px-3 text-slate-800">{item.operation_type}</td>
                    <td className="py-2.5 px-3 text-slate-600 truncate max-w-xs" title={item.target.identifier}>
                      {item.target.identifier}
                    </td>
                    <td className="py-2.5 px-3 font-sans">
                      {item.decision === 'Allowed' ? (
                        <span className="text-emerald-700 font-semibold">Allowed</span>
                      ) : item.decision === 'RequiresConfirmation' ? (
                        <span className="text-amber-700 font-semibold">Requires Confirmation</span>
                      ) : item.decision === 'Blocked' ? (
                        <span className="text-red-700 font-bold">Blocked</span>
                      ) : (
                        <span className="text-rose-700 font-semibold">Denied</span>
                      )}
                    </td>
                    <td className="py-2.5 px-3 text-slate-600">{item.reason_code}</td>
                    <td className="py-2.5 px-3 font-sans">{getRiskBadge(item.risk_level)}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
