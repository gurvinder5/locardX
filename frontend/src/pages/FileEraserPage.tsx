import React, { useState } from 'react';
import {
  FileText,
  Folder,
  ShieldAlert,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Copy,
  Check,
  RotateCcw,
  Download,
  FileCheck,
  Layers,
  ArrowRight,
} from 'lucide-react';
import {
  FileErasePlanDto,
  FileEraseResultDto,
  FileEraseScope,
} from '../types/fileEraser';
import {
  planFileErasure,
  planFolderErasure,
  executeFileErasure,
  executeFolderErasure,
} from '../services/fileEraser';
import { requestDestructiveConfirmation } from '../services/safety';
import { useAuthStore } from '../stores/authStore';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

export const FileEraserPage: React.FC = () => {
  const sessionToken = useAuthStore((s) => s.sessionToken);

  // Form State
  const [scope, setScope] = useState<FileEraseScope>('File');
  const [targetPath, setTargetPath] = useState('');
  const [method, setMethod] = useState<string>('LogicalFileShred');

  // Async States
  const [isPlanning, setIsPlanning] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Results
  const [plan, setPlan] = useState<FileErasePlanDto | null>(null);
  const [result, setResult] = useState<FileEraseResultDto | null>(null);

  // Confirmation Modal
  const [showConfirmModal, setShowConfirmModal] = useState(false);
  const [confirmationId, setConfirmationId] = useState<string | null>(null);
  const [operationId, setOperationId] = useState<string | null>(null);
  const [typedConfirmation, setTypedConfirmation] = useState('');
  const [warningAcknowledged, setWarningAcknowledged] = useState(false);
  const [copiedHash, setCopiedHash] = useState(false);

  // Handlers
  const handleInspectAndPlan = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorMessage(null);
    setResult(null);

    const trimmed = targetPath.trim();
    if (!trimmed) {
      setErrorMessage('Please enter a valid target absolute path.');
      return;
    }

    try {
      setIsPlanning(true);
      if (scope === 'File') {
        const planned = await planFileErasure({
          target_path: trimmed,
          method,
          session_token: sessionToken,
        });
        setPlan(planned);
      } else {
        const planned = await planFolderErasure({
          target_path: trimmed,
          method,
          session_token: sessionToken,
        });
        setPlan(planned);
      }
    } catch (err: any) {
      setErrorMessage(
        err?.message || 'Failed to inspect and generate sanitization plan for target.'
      );
      setPlan(null);
    } finally {
      setIsPlanning(false);
    }
  };

  const handleOpenConfirmation = async () => {
    if (!plan) return;
    setErrorMessage(null);

    try {
      const opId = `op-fe-${Date.now()}`;
      setOperationId(opId);

      const challenge = await requestDestructiveConfirmation({
        operation_id: opId,
        target_type: plan.scope === 'File' ? 'File' : 'Directory',
        target_identifier: plan.canonical_path,
        target_display_name: plan.target_path,
        target_size_bytes: plan.pre_metadata.size_bytes,
        operation_type: plan.scope === 'File' ? 'FileErasure' : 'FolderErasure',
        session_token: sessionToken || '',
      });

      setConfirmationId(challenge.confirmation_id);
      setTypedConfirmation('');
      setWarningAcknowledged(false);
      setShowConfirmModal(true);
    } catch (err: any) {
      setErrorMessage(
        err?.message || 'Failed to request two-stage confirmation challenge.'
      );
    }
  };

  const handleExecuteSanitization = async () => {
    if (!plan || !confirmationId || !operationId) return;

    if (!warningAcknowledged) {
      setErrorMessage('You must acknowledge the destructive consequences warning.');
      return;
    }

    if (typedConfirmation.trim().toLowerCase() !== plan.canonical_path.trim().toLowerCase() &&
        typedConfirmation.trim().toLowerCase() !== plan.target_path.trim().toLowerCase()) {
      setErrorMessage(
        `Confirmation string does not match the target path: "${plan.canonical_path}"`
      );
      return;
    }

    setShowConfirmModal(false);
    setIsExecuting(true);
    setErrorMessage(null);

    try {
      if (plan.scope === 'File') {
        const res = await executeFileErasure({
          plan_id: plan.plan_id,
          confirmation_id: confirmationId,
          operation_id: operationId,
          typed_confirmation: typedConfirmation.trim(),
          warning_acknowledged: true,
          session_token: sessionToken || '',
        });
        setResult(res);
      } else {
        const res = await executeFolderErasure({
          plan_id: plan.plan_id,
          confirmation_id: confirmationId,
          operation_id: operationId,
          typed_confirmation: typedConfirmation.trim(),
          warning_acknowledged: true,
          session_token: sessionToken || '',
        });
        setResult(res);
      }
    } catch (err: any) {
      setErrorMessage(
        err?.message || 'Sanitization execution failed.'
      );
    } finally {
      setIsExecuting(false);
    }
  };

  const handleReset = () => {
    setPlan(null);
    setResult(null);
    setTargetPath('');
    setErrorMessage(null);
    setShowConfirmModal(false);
  };

  const handleExportJson = () => {
    if (!result) return;
    const blob = new Blob([JSON.stringify(result, null, 2)], {
      type: 'application/json',
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `locardx-erasure-report-${result.operation_id}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-6 max-w-5xl mx-auto">
      {/* Header Banner */}
      <div className="bg-white border border-slate-200 rounded-lg p-6 shadow-xs">
        <div className="flex items-start justify-between">
          <div className="space-y-1">
            <div className="flex items-center space-x-2">
              <span className="p-2 bg-slate-100 rounded-md text-slate-700">
                <FileCheck className="w-5 h-5 text-slate-700" />
              </span>
              <h1 className="text-lg font-bold text-slate-900 tracking-tight">
                Secure File & Folder Eraser
              </h1>
            </div>
            <p className="text-xs text-slate-500">
              Surgical logical sanitization with multi-pass content overwrite, metadata unlinking, and tamper-evident audit logging.
            </p>
          </div>
          <span className="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200">
            Pillar 2 Active
          </span>
        </div>

        {/* Global Disclaimer Banner */}
        <div className="mt-4 p-3.5 bg-amber-50 border border-amber-200 rounded-md flex items-start space-x-3 text-amber-900">
          <AlertTriangle className="w-4 h-4 text-amber-600 mt-0.5 shrink-0" />
          <div className="text-xs space-y-0.5">
            <p className="font-semibold text-amber-800">
              DESTRUCTIVE FILE SANITIZATION — LOGICAL FILESYSTEM SCOPE
            </p>
            <p className="text-amber-700 leading-relaxed">
              Logical sanitization shreds and unlinks target directory entries. It does not bypass SSD/NVMe Flash Translation Layer (FTL) wear leveling, nor does it scrub unallocated clusters or VSS shadow copies.
            </p>
          </div>
        </div>
      </div>

      {/* Main Configuration Card */}
      {!result && (
        <div className="bg-white border border-slate-200 rounded-lg p-6 shadow-xs space-y-5">
          <h2 className="text-sm font-semibold text-slate-800 flex items-center gap-2">
            <Layers className="w-4 h-4 text-slate-500" /> Target Configuration & Scope
          </h2>

          <form onSubmit={handleInspectAndPlan} className="space-y-4">
            {/* Scope Toggle */}
            <div className="flex items-center space-x-4">
              <label className="text-xs font-medium text-slate-700">Operation Scope:</label>
              <div className="inline-flex rounded-md border border-slate-200 p-0.5 bg-slate-50">
                <button
                  type="button"
                  onClick={() => {
                    setScope('File');
                    setPlan(null);
                  }}
                  className={`px-3 py-1.5 text-xs font-medium rounded flex items-center gap-1.5 transition-colors ${
                    scope === 'File'
                      ? 'bg-white text-slate-900 shadow-xs'
                      : 'text-slate-500 hover:text-slate-700'
                  }`}
                >
                  <FileText className="w-3.5 h-3.5" /> Single File
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setScope('Folder');
                    setPlan(null);
                  }}
                  className={`px-3 py-1.5 text-xs font-medium rounded flex items-center gap-1.5 transition-colors ${
                    scope === 'Folder'
                      ? 'bg-white text-slate-900 shadow-xs'
                      : 'text-slate-500 hover:text-slate-700'
                  }`}
                >
                  <Folder className="w-3.5 h-3.5" /> Recursive Folder
                </button>
              </div>
            </div>

            {/* Target Path Input */}
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700 flex items-center justify-between">
                <span>Target Absolute Path</span>
                <span className="text-[11px] font-normal text-slate-400">
                  Must be outside system roots & boot paths
                </span>
              </label>
              <div className="relative">
                <input
                  type="text"
                  value={targetPath}
                  onChange={(e) => {
                    setTargetPath(e.target.value);
                    setPlan(null);
                  }}
                  placeholder={
                    scope === 'File'
                      ? 'C:\\Forensics\\Case_001\\confidential.docx'
                      : 'C:\\Forensics\\Case_001\\evidence_dump'
                  }
                  className="w-full text-xs font-mono px-3 py-2 bg-slate-50 border border-slate-300 rounded-md focus:outline-none focus:ring-1 focus:ring-slate-500 text-slate-800 placeholder-slate-400"
                />
              </div>
            </div>

            {/* Sanitization Method Selector */}
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700">Sanitization Method</label>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div
                  onClick={() => setMethod('LogicalFileShred')}
                  className={`p-3 rounded-md border cursor-pointer transition-all ${
                    method === 'LogicalFileShred'
                      ? 'border-slate-800 bg-slate-50/50 shadow-xs'
                      : 'border-slate-200 hover:border-slate-300'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-semibold text-slate-800">
                      Logical File Shred (3 Passes)
                    </span>
                    <span className="text-[10px] uppercase font-bold text-slate-500 bg-slate-200 px-1.5 py-0.5 rounded">
                      Recommended
                    </span>
                  </div>
                  <p className="text-[11px] text-slate-500 mt-1">
                    Pass 1: Cryptographic Pseudorandom Bytes &bull; Pass 2: Inverted Bit Pattern (0x55) &bull; Pass 3: Zero Bytes (0x00).
                  </p>
                </div>

                <div
                  onClick={() => setMethod('Nist80088ClearZero')}
                  className={`p-3 rounded-md border cursor-pointer transition-all ${
                    method === 'Nist80088ClearZero'
                      ? 'border-slate-800 bg-slate-50/50 shadow-xs'
                      : 'border-slate-200 hover:border-slate-300'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-semibold text-slate-800">
                      NIST SP 800-88 Rev 1 Clear (1 Pass)
                    </span>
                  </div>
                  <p className="text-[11px] text-slate-500 mt-1">
                    Pass 1: Logical overwrite using single-pass fixed 0x00 null bytes across entire allocation.
                  </p>
                </div>
              </div>
            </div>

            {/* Submit Button */}
            <div className="pt-2 flex justify-end">
              <button
                type="submit"
                disabled={isPlanning || !targetPath.trim()}
                className="px-4 py-2 bg-slate-800 text-white text-xs font-medium rounded-md hover:bg-slate-900 disabled:opacity-50 flex items-center gap-1.5 transition-colors shadow-xs"
              >
                {isPlanning ? (
                  <>Inspecting Target...</>
                ) : (
                  <>
                    <span>Inspect Target & Generate Plan</span>
                    <ArrowRight className="w-3.5 h-3.5" />
                  </>
                )}
              </button>
            </div>
          </form>

          {/* Error Message */}
          {errorMessage && (
            <div className="p-3 bg-rose-50 border border-rose-200 rounded-md flex items-start space-x-2 text-rose-800 text-xs">
              <XCircle className="w-4 h-4 text-rose-600 mt-0.5 shrink-0" />
              <span>{errorMessage}</span>
            </div>
          )}
        </div>
      )}

      {/* Target Pre-Inspection Card */}
      {plan && !result && (
        <div className="bg-white border border-slate-200 rounded-lg p-6 shadow-xs space-y-4">
          <div className="flex items-center justify-between border-b border-slate-200 pb-3">
            <h2 className="text-sm font-semibold text-slate-800 flex items-center gap-2">
              <FileCheck className="w-4 h-4 text-emerald-600" /> Target Pre-Inspection Snapshot
            </h2>
            <span className="text-xs font-mono text-slate-500">Plan ID: {plan.plan_id}</span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs">
            <div className="space-y-1">
              <span className="text-slate-500 font-medium">Canonical Path:</span>
              <p className="font-mono text-slate-800 bg-slate-50 p-2 rounded border border-slate-200 break-all">
                {plan.canonical_path}
              </p>
            </div>
            <div className="space-y-1">
              <span className="text-slate-500 font-medium">Target Size:</span>
              <p className="font-mono text-slate-800 bg-slate-50 p-2 rounded border border-slate-200">
                {formatBytes(plan.pre_metadata.size_bytes)} ({plan.pre_metadata.size_bytes.toLocaleString()} bytes)
              </p>
            </div>
            <div className="space-y-1">
              <span className="text-slate-500 font-medium">Read-Only Attribute:</span>
              <p className="font-mono text-slate-800 bg-slate-50 p-2 rounded border border-slate-200">
                {plan.pre_metadata.is_readonly ? 'Yes (Will be stripped prior to wipe)' : 'No'}
              </p>
            </div>
            <div className="space-y-1">
              <span className="text-slate-500 font-medium">Sanitization Passes:</span>
              <p className="font-mono text-slate-800 bg-slate-50 p-2 rounded border border-slate-200">
                {plan.passes} pass(es) ({plan.method})
              </p>
            </div>
          </div>

          {/* Pre-erasure SHA-256 Hash */}
          {plan.pre_metadata.pre_erasure_sha256 && (
            <div className="space-y-1 text-xs">
              <span className="text-slate-500 font-medium">Evidence Pre-Erasure SHA-256 Digest:</span>
              <div className="flex items-center space-x-2">
                <p className="font-mono text-slate-800 bg-slate-50 p-2 rounded border border-slate-200 break-all flex-1 text-[11px]">
                  {plan.pre_metadata.pre_erasure_sha256}
                </p>
                <button
                  type="button"
                  onClick={() => {
                    navigator.clipboard.writeText(plan.pre_metadata.pre_erasure_sha256 || '');
                    setCopiedHash(true);
                    setTimeout(() => setCopiedHash(false), 2000);
                  }}
                  className="p-2 border border-slate-300 rounded hover:bg-slate-50 text-slate-600"
                >
                  {copiedHash ? <Check className="w-4 h-4 text-emerald-600" /> : <Copy className="w-4 h-4" />}
                </button>
              </div>
            </div>
          )}

          {/* Limitations List */}
          <div className="space-y-1.5 pt-2">
            <span className="text-xs font-semibold text-slate-700">Forensic Limitations & Caveats:</span>
            <ul className="text-[11px] text-slate-600 space-y-1 list-disc list-inside bg-slate-50 p-3 rounded border border-slate-200">
              {plan.limitations.map((lim, idx) => (
                <li key={idx}>{lim}</li>
              ))}
            </ul>
          </div>

          {/* Proceed to Confirmation Button */}
          <div className="pt-3 flex justify-between items-center border-t border-slate-200">
            <button
              type="button"
              onClick={handleReset}
              className="px-3 py-1.5 text-xs text-slate-600 hover:text-slate-800"
            >
              Cancel & Choose Another Target
            </button>
            <button
              type="button"
              onClick={handleOpenConfirmation}
              className="px-4 py-2 bg-rose-700 hover:bg-rose-800 text-white text-xs font-semibold rounded-md flex items-center gap-1.5 shadow-xs transition-colors"
            >
              <ShieldAlert className="w-4 h-4" />
              <span>Proceed to Two-Stage Authorization</span>
            </button>
          </div>
        </div>
      )}

      {/* Two-Stage Confirmation Challenge Modal */}
      {showConfirmModal && plan && (
        <div className="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4">
          <div className="bg-white rounded-lg border border-slate-300 shadow-xl max-w-lg w-full p-6 space-y-4">
            <div className="flex items-center space-x-2 text-rose-700 border-b border-rose-100 pb-3">
              <ShieldAlert className="w-5 h-5" />
              <h3 className="text-sm font-bold tracking-tight">
                Two-Stage Destructive Confirmation Challenge
              </h3>
            </div>

            <div className="p-3 bg-rose-50 border border-rose-200 rounded text-rose-900 text-xs space-y-1">
              <p className="font-bold">WARNING: IRREVERSIBLE OPERATION</p>
              <p>
                Executing this operation will physically overwrite and unlink the target filesystem entry. Target contents cannot be restored by standard carving tools.
              </p>
            </div>

            <div className="space-y-1 text-xs">
              <span className="text-slate-500 font-medium">Target to be Destroyed:</span>
              <p className="font-mono text-slate-900 bg-slate-100 p-2 rounded border border-slate-200 break-all">
                {plan.canonical_path}
              </p>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-800">
                Type the exact target path to confirm:
              </label>
              <input
                type="text"
                value={typedConfirmation}
                onChange={(e) => setTypedConfirmation(e.target.value)}
                placeholder={plan.canonical_path}
                className="w-full text-xs font-mono px-3 py-2 border border-slate-300 rounded focus:ring-1 focus:ring-rose-500 text-slate-900"
              />
            </div>

            <label className="flex items-start space-x-2 text-xs text-slate-700 cursor-pointer pt-1">
              <input
                type="checkbox"
                checked={warningAcknowledged}
                onChange={(e) => setWarningAcknowledged(e.target.checked)}
                className="rounded border-slate-300 text-rose-600 focus:ring-rose-500 mt-0.5"
              />
              <span>
                I acknowledge the destructive consequences and confirm that I have legal authorization to destroy this target.
              </span>
            </label>

            {errorMessage && (
              <p className="text-xs text-rose-600 font-medium">{errorMessage}</p>
            )}

            <div className="flex justify-end space-x-2 pt-3 border-t border-slate-200">
              <button
                type="button"
                onClick={() => setShowConfirmModal(false)}
                className="px-3 py-1.5 text-xs text-slate-600 hover:text-slate-800"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleExecuteSanitization}
                disabled={
                  !warningAcknowledged ||
                  (typedConfirmation.trim().toLowerCase() !== plan.canonical_path.trim().toLowerCase() &&
                   typedConfirmation.trim().toLowerCase() !== plan.target_path.trim().toLowerCase())
                }
                className="px-4 py-2 bg-rose-700 hover:bg-rose-800 text-white text-xs font-bold rounded disabled:opacity-50 shadow-xs flex items-center gap-1.5 transition-colors"
              >
                <ShieldAlert className="w-3.5 h-3.5" />
                <span>Authorize & Execute Sanitization</span>
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Execution in Progress View */}
      {isExecuting && (
        <div className="bg-white border border-slate-200 rounded-lg p-8 shadow-xs text-center space-y-4">
          <div className="w-8 h-8 border-2 border-slate-300 border-t-slate-800 rounded-full animate-spin mx-auto" />
          <div className="space-y-1">
            <h3 className="text-sm font-semibold text-slate-900">
              Sanitization in Progress...
            </h3>
            <p className="text-xs text-slate-500 max-w-sm mx-auto">
              Performing multi-pass streaming overwrite (64 KB bounded memory chunks) and filesystem metadata unlinking.
            </p>
          </div>
        </div>
      )}

      {/* Post-Sanitization Summary Report */}
      {result && (
        <div className="bg-white border border-slate-200 rounded-lg p-6 shadow-xs space-y-5">
          <div className="flex items-center justify-between border-b border-slate-200 pb-3">
            <div className="flex items-center space-x-2">
              <CheckCircle2 className="w-5 h-5 text-emerald-600" />
              <h2 className="text-sm font-bold text-slate-900">
                Post-Sanitization Audit & Verification Report
              </h2>
            </div>
            <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-bold bg-emerald-100 text-emerald-800">
              {result.status.toUpperCase()}
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
            <div className="bg-slate-50 p-3 rounded border border-slate-200 space-y-1">
              <span className="text-slate-500 font-medium">Verification Outcome:</span>
              <p className="font-semibold text-emerald-700 text-sm flex items-center gap-1">
                <CheckCircle2 className="w-4 h-4" /> {result.verification.outcome}
              </p>
              <p className="text-[10px] text-slate-500">{result.verification.strategy}</p>
            </div>
            <div className="bg-slate-50 p-3 rounded border border-slate-200 space-y-1">
              <span className="text-slate-500 font-medium">Bytes Overwritten:</span>
              <p className="font-semibold text-slate-800 text-sm font-mono">
                {formatBytes(result.bytes_processed)}
              </p>
              <p className="text-[10px] text-slate-500">
                {result.bytes_processed.toLocaleString()} bytes total
              </p>
            </div>
            <div className="bg-slate-50 p-3 rounded border border-slate-200 space-y-1">
              <span className="text-slate-500 font-medium">Filesystem Status:</span>
              <p className="font-semibold text-slate-800 text-sm">
                {result.verification.inaccessible ? 'Inaccessible (Unlinked)' : 'Present'}
              </p>
              <p className="text-[10px] text-slate-500">Directory entry unlinked</p>
            </div>
          </div>

          {/* Target and Operation Details */}
          <div className="space-y-2 text-xs">
            <div className="space-y-0.5">
              <span className="text-slate-500 font-medium">Target Path:</span>
              <p className="font-mono text-slate-900 bg-slate-50 p-2 rounded border border-slate-200 break-all">
                {result.canonical_path}
              </p>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
              <div className="bg-slate-50 p-2 rounded border border-slate-200">
                <span className="text-slate-500">Operation ID: </span>
                <span className="font-mono text-slate-800">{result.operation_id}</span>
              </div>
              <div className="bg-slate-50 p-2 rounded border border-slate-200">
                <span className="text-slate-500">Method: </span>
                <span className="font-mono text-slate-800">{result.method}</span>
              </div>
            </div>
          </div>

          {/* Folder Stats if Applicable */}
          {result.folder_stats && (
            <div className="bg-slate-50 p-3 rounded border border-slate-200 space-y-2 text-xs">
              <span className="font-semibold text-slate-700">Folder Traversal Statistics:</span>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-2 text-center">
                <div className="bg-white p-2 rounded border border-slate-200">
                  <div className="text-slate-500 text-[10px]">Files Shredded</div>
                  <div className="font-bold text-slate-800">{result.folder_stats.files_sanitized}</div>
                </div>
                <div className="bg-white p-2 rounded border border-slate-200">
                  <div className="text-slate-500 text-[10px]">Directories Removed</div>
                  <div className="font-bold text-slate-800">{result.folder_stats.directories_removed}</div>
                </div>
                <div className="bg-white p-2 rounded border border-slate-200">
                  <div className="text-slate-500 text-[10px]">Failed Items</div>
                  <div className="font-bold text-slate-800">{result.folder_stats.files_failed}</div>
                </div>
                <div className="bg-white p-2 rounded border border-slate-200">
                  <div className="text-slate-500 text-[10px]">Cancelled Items</div>
                  <div className="font-bold text-slate-800">{result.folder_stats.files_cancelled}</div>
                </div>
              </div>
            </div>
          )}

          {/* Verification Details */}
          <div className="space-y-1 text-xs">
            <span className="text-slate-500 font-medium">Verification Summary:</span>
            <p className="text-slate-700 bg-slate-50 p-2.5 rounded border border-slate-200">
              {result.verification.details}
            </p>
          </div>

          {/* Limitations */}
          <div className="space-y-1 text-[11px] text-slate-500 bg-amber-50/50 p-3 rounded border border-amber-200">
            <span className="font-semibold text-amber-800">Physical Storage Limitations Disclaimer:</span>
            <ul className="list-disc list-inside space-y-0.5 mt-1 text-amber-700">
              {result.limitations.map((lim, i) => (
                <li key={i}>{lim}</li>
              ))}
            </ul>
          </div>

          {/* Action Buttons */}
          <div className="flex justify-between items-center pt-3 border-t border-slate-200">
            <button
              type="button"
              onClick={handleReset}
              className="px-3.5 py-1.5 bg-slate-100 hover:bg-slate-200 text-slate-700 text-xs font-medium rounded flex items-center gap-1.5 transition-colors"
            >
              <RotateCcw className="w-3.5 h-3.5" /> Plan Another Erasure
            </button>
            <button
              type="button"
              onClick={handleExportJson}
              className="px-3.5 py-1.5 bg-slate-800 hover:bg-slate-900 text-white text-xs font-semibold rounded flex items-center gap-1.5 transition-colors shadow-xs"
            >
              <Download className="w-3.5 h-3.5" /> Export Report (JSON)
            </button>
          </div>
        </div>
      )}
    </div>
  );
};

export default FileEraserPage;
