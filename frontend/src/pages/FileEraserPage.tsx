import React, { useState } from 'react';
import {
  FileText,
  Folder,
  ShieldAlert,
  CheckCircle2,
  AlertTriangle,
  Copy,
  Check,
  RotateCcw,
  Download,
  FileCheck,
  Layers,
  ArrowRight,
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
} from '@heroui/react';
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
import { useCaseStore } from '../stores/caseStore';
import { ActiveCaseBanner } from '../components/case/ActiveCaseBanner';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

export const FileEraserPage: React.FC = () => {
  const sessionToken = useAuthStore((s) => s.sessionToken);
  const { activeCase } = useCaseStore();
  const [associateWithCase, setAssociateWithCase] = useState<boolean>(true);

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

  // Reset plan when target path, scope, or method changes
  React.useEffect(() => {
    setPlan(null);
    setResult(null);
    setErrorMessage(null);
  }, [targetPath, scope, method]);

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
      const targetCaseId = associateWithCase && activeCase ? activeCase.case_id : null;
      const generatedPlan =
        scope === 'File'
          ? await planFileErasure({
              target_path: trimmed,
              method,
              case_id: targetCaseId,
              session_token: sessionToken,
            })
          : await planFolderErasure({
              target_path: trimmed,
              method,
              case_id: targetCaseId,
              session_token: sessionToken,
            });

      setPlan(generatedPlan);
    } catch (err: unknown) {
      const msg =
        err && typeof err === 'object' && 'message' in err
          ? String((err as { message: string }).message)
          : 'Inspection failed. Please verify that path exists and is accessible.';
      setErrorMessage(msg);
      setPlan(null);
    } finally {
      setIsPlanning(false);
    }
  };

  const handleOpenConfirmation = async () => {
    if (!plan) return;
    try {
      setErrorMessage(null);
      const res = await requestDestructiveConfirmation({
        operation_id: `op-file-${Date.now()}`,
        target_type: scope === 'File' ? 'File' : 'Folder',
        target_identifier: plan.canonical_path,
        target_display_name: plan.canonical_path,
        target_size_bytes: plan.pre_metadata.size_bytes,
        operation_type: scope === 'File' ? 'FileErasure' : 'FolderErasure',
        session_token: sessionToken || 'default-session',
      });
      setConfirmationId(res.confirmation_id);
      setOperationId(res.operation_id);
      setTypedConfirmation('');
      setWarningAcknowledged(false);
      setShowConfirmModal(true);
    } catch (err: unknown) {
      const msg =
        err && typeof err === 'object' && 'message' in err
          ? String((err as { message: string }).message)
          : 'Failed to initiate safety challenge protocol.';
      setErrorMessage(msg);
    }
  };

  const handleExecute = async () => {
    if (!plan || !confirmationId || !operationId) return;
    setShowConfirmModal(false);
    setIsExecuting(true);
    setErrorMessage(null);

    try {
      const targetCaseId = associateWithCase && activeCase ? activeCase.case_id : undefined;
      const res =
        scope === 'File'
          ? await executeFileErasure({
              plan_id: plan.plan_id,
              confirmation_id: confirmationId,
              operation_id: operationId,
              typed_confirmation: typedConfirmation,
              warning_acknowledged: warningAcknowledged,
              session_token: sessionToken || 'default-session',
              case_id: targetCaseId,
            })
          : await executeFolderErasure({
              plan_id: plan.plan_id,
              confirmation_id: confirmationId,
              operation_id: operationId,
              typed_confirmation: typedConfirmation,
              warning_acknowledged: warningAcknowledged,
              session_token: sessionToken || 'default-session',
              case_id: targetCaseId,
            });

      setResult(res);
      setPlan(null);
    } catch (err: unknown) {
      const msg =
        err && typeof err === 'object' && 'message' in err
          ? String((err as { message: string }).message)
          : 'Sanitization execution failed.';
      setErrorMessage(msg);
    } finally {
      setIsExecuting(false);
    }
  };

  const handleReset = () => {
    setPlan(null);
    setResult(null);
    setErrorMessage(null);
    setTargetPath('');
  };

  const handleExportJson = () => {
    if (!result) return;
    const blob = new Blob([JSON.stringify(result, null, 2)], {
      type: 'application/json',
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `file_erase_${result.operation_id}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-6 max-w-5xl mx-auto pb-10">
      {/* Active Forensic Case Banner & Context */}
      <ActiveCaseBanner
        associateWithCase={associateWithCase}
        onToggleAssociate={setAssociateWithCase}
      />

      {/* Page Header */}
      <div className="flex items-start justify-between border-b border-slate-200 dark:border-[#26313D] pb-4">
        <div>
          <h1 className="text-xl font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
            <FileCheck className="w-5 h-5 text-slate-700 dark:text-slate-300" />
            Secure File & Folder Eraser
          </h1>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
            Surgical logical sanitization with multi-pass content overwrite, metadata unlinking, and tamper-evident audit logging.
          </p>
        </div>
        <Chip size="sm" variant="flat" className="font-semibold bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-[#26313D]">
          Logical Sanitization
        </Chip>
      </div>

      {/* Global Safety Boundary Notice (No Card) */}
      <div className="border-l-2 border-amber-500 bg-amber-500/10 p-3 text-xs text-amber-900 dark:text-amber-200 flex items-start space-x-3">
        <AlertTriangle className="w-4 h-4 text-amber-600 dark:text-amber-400 mt-0.5 shrink-0" />
        <div className="space-y-0.5 leading-relaxed">
          <span className="font-bold uppercase tracking-wider text-[11px] block text-amber-800 dark:text-amber-300">
            Logical Filesystem Scope Warning
          </span>
          <p className="text-[11px] text-amber-900 dark:text-amber-200">
            Logical sanitization shreds and unlinks target directory entries. It does not bypass SSD/NVMe Flash Translation Layer (FTL) wear leveling, nor does it scrub unallocated clusters or VSS shadow copies.
          </p>
        </div>
      </div>

      {/* Error Alert */}
      {errorMessage && (
        <div className="border border-rose-300 dark:border-rose-900 bg-rose-50 dark:bg-rose-950/20 text-rose-800 dark:text-rose-300 p-3 rounded-sm text-xs flex items-center gap-2">
          <ShieldAlert className="w-4 h-4 shrink-0 text-rose-600" />
          <span>Error: {errorMessage}</span>
        </div>
      )}

      {/* Main Configuration Section (Flat Form) */}
      {!result && (
        <div className="space-y-5 pt-2">
          <div className="flex items-center gap-2 pb-2 border-b border-slate-200 dark:border-[#26313D]">
            <Layers className="w-4 h-4 text-slate-700 dark:text-slate-300" />
            <h2 className="text-sm font-bold text-slate-900 dark:text-slate-100">
              Target Configuration & Scope
            </h2>
          </div>

          <form onSubmit={handleInspectAndPlan} className="space-y-4">
            {/* Scope Toggle */}
            <div className="flex items-center space-x-4">
              <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">Operation Scope:</label>
              <div className="inline-flex rounded-sm border border-slate-200 dark:border-[#26313D] p-0.5 bg-slate-100 dark:bg-[#1B2530]">
                <Button
                  size="sm"
                  variant={scope === 'File' ? 'solid' : 'light'}
                  onPress={() => {
                    setScope('File');
                    setPlan(null);
                  }}
                  startContent={<FileText className="w-3.5 h-3.5" />}
                  className={`h-7 px-3 text-xs font-medium rounded-xs ${
                    scope === 'File'
                      ? 'bg-white dark:bg-[#151C24] text-slate-900 dark:text-slate-100 shadow-2xs font-bold'
                      : 'text-slate-500 hover:text-slate-700 dark:text-slate-400'
                  }`}
                >
                  Single File
                </Button>
                <Button
                  size="sm"
                  variant={scope === 'Folder' ? 'solid' : 'light'}
                  onPress={() => {
                    setScope('Folder');
                    setPlan(null);
                  }}
                  startContent={<Folder className="w-3.5 h-3.5" />}
                  className={`h-7 px-3 text-xs font-medium rounded-xs ${
                    scope === 'Folder'
                      ? 'bg-white dark:bg-[#151C24] text-slate-900 dark:text-slate-100 shadow-2xs font-bold'
                      : 'text-slate-500 hover:text-slate-700 dark:text-slate-400'
                  }`}
                >
                  Recursive Folder
                </Button>
              </div>
            </div>

            {/* Target Path Input */}
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700 dark:text-slate-300 flex items-center justify-between">
                <span>Target Absolute Path</span>
                <span className="text-[11px] font-normal text-slate-400">
                  Must be outside system roots & boot paths
                </span>
              </label>
              <Input
                size="sm"
                value={targetPath}
                onValueChange={(val) => {
                  setTargetPath(val);
                  setPlan(null);
                }}
                placeholder={
                  scope === 'File'
                    ? 'C:\\Forensics\\Case_001\\confidential.docx'
                    : 'C:\\Forensics\\Case_001\\evidence_dump'
                }
                className="font-mono text-xs"
                classNames={{
                  inputWrapper: 'bg-white dark:bg-[#151C24] border border-slate-300 dark:border-[#26313D] h-8',
                }}
              />
            </div>

            {/* Sanitization Algorithm Selector */}
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                Sanitization Algorithm & Overwrite Passes
              </label>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
                <div
                  onClick={() => setMethod('LogicalFileShred')}
                  className={`p-3 rounded-sm border cursor-pointer transition-all ${
                    method === 'LogicalFileShred'
                      ? 'border-slate-900 dark:border-slate-100 bg-slate-100 dark:bg-[#1B2530] ring-1 ring-slate-900 dark:ring-slate-100 font-bold'
                      : 'border-slate-200 dark:border-[#26313D] hover:border-slate-300 dark:hover:border-slate-700 bg-white dark:bg-[#151C24]'
                  }`}
                >
                  <div className="font-semibold text-slate-900 dark:text-slate-100">NIST SP 800-88 Clear</div>
                  <div className="text-[11px] text-slate-500 mt-0.5">1-Pass Pseudorandom/Zero overwrite + Unlink</div>
                </div>

                <div
                  onClick={() => setMethod('DoD522022M3Pass')}
                  className={`p-3 rounded-sm border cursor-pointer transition-all ${
                    method === 'DoD522022M3Pass'
                      ? 'border-slate-900 dark:border-slate-100 bg-slate-100 dark:bg-[#1B2530] ring-1 ring-slate-900 dark:ring-slate-100 font-bold'
                      : 'border-slate-200 dark:border-[#26313D] hover:border-slate-300 dark:hover:border-slate-700 bg-white dark:bg-[#151C24]'
                  }`}
                >
                  <div className="font-semibold text-slate-900 dark:text-slate-100">DoD 5220.22-M (3-Pass)</div>
                  <div className="text-[11px] text-slate-500 mt-0.5">Fixed byte pattern, complement, random pass</div>
                </div>

                <div
                  onClick={() => setMethod('ZeroFillSinglePass')}
                  className={`p-3 rounded-sm border cursor-pointer transition-all ${
                    method === 'ZeroFillSinglePass'
                      ? 'border-slate-900 dark:border-slate-100 bg-slate-100 dark:bg-[#1B2530] ring-1 ring-slate-900 dark:ring-slate-100 font-bold'
                      : 'border-slate-200 dark:border-[#26313D] hover:border-slate-300 dark:hover:border-slate-700 bg-white dark:bg-[#151C24]'
                  }`}
                >
                  <div className="font-semibold text-slate-900 dark:text-slate-100">Zero-Fill (Single Pass)</div>
                  <div className="text-[11px] text-slate-500 mt-0.5">Fast zero-overwrite before metadata truncate</div>
                </div>
              </div>
            </div>

            <div className="pt-2 flex justify-end">
              <Button
                type="submit"
                isLoading={isPlanning}
                isDisabled={!targetPath.trim() || isPlanning}
                endContent={!isPlanning && <ArrowRight className="w-3.5 h-3.5" />}
                className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 text-xs font-semibold px-4 h-8"
              >
                {isPlanning ? 'Inspecting Path...' : 'Inspect Target & Generate Plan'}
              </Button>
            </div>
          </form>
        </div>
      )}

      {/* Pre-Inspection Plan Display */}
      {plan && !result && (
        <div className="space-y-4 pt-4 border-t border-slate-200 dark:border-[#26313D]">
          <div className="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-[#26313D]">
            <div className="flex items-center space-x-2">
              <span className="w-5 h-5 rounded-full bg-slate-900 dark:bg-slate-100 text-white dark:text-slate-900 text-[10px] font-bold flex items-center justify-center">
                2
              </span>
              <h2 className="text-sm font-bold text-slate-900 dark:text-slate-100">
                Pre-Execution Target Assessment
              </h2>
            </div>
            <Chip size="sm" variant="flat" className="font-mono text-[10px] bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-[#26313D]">
              PLAN: {plan.plan_id.substring(0, 16)}...
            </Chip>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 py-3 border-b border-slate-200 dark:border-[#26313D] text-xs">
            <div>
              <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
                Target Canonical Path
              </span>
              <span className="font-mono font-medium text-slate-800 dark:text-slate-200 mt-0.5 block break-all">
                {plan.canonical_path}
              </span>
            </div>

            <div>
              <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
                Target Size
              </span>
              <span className="font-mono font-medium text-slate-800 dark:text-slate-200 mt-0.5 block">
                {formatBytes(plan.pre_metadata.size_bytes)} ({plan.pre_metadata.size_bytes} bytes)
              </span>
            </div>

            <div>
              <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
                Estimated Duration
              </span>
              <span className="font-mono font-medium text-slate-800 dark:text-slate-200 mt-0.5 block">
                ~{Math.max(1, Math.round(plan.pre_metadata.size_bytes / (50 * 1024 * 1024)))} second(s)
              </span>
            </div>

            <div>
              <span className="text-[10px] font-bold uppercase tracking-wider text-slate-400 dark:text-slate-500 block">
                Sanitization Algorithm
              </span>
              <span className="font-medium text-slate-800 dark:text-slate-200 mt-0.5 block">
                {plan.passes} pass(es) ({plan.method})
              </span>
            </div>
          </div>

          {/* Pre-erasure SHA-256 Hash */}
          {plan.pre_metadata.pre_erasure_sha256 && (
            <div className="space-y-1 text-xs">
              <span className="text-slate-400 text-[10px] uppercase font-bold tracking-wider block">
                Pre-Erasure Cryptographic SHA-256 Digest
              </span>
              <div className="flex items-center space-x-2">
                <p className="font-mono text-slate-800 dark:text-slate-200 bg-slate-100 dark:bg-[#1B2530] p-2 rounded-xs border border-slate-200 dark:border-[#26313D] break-all flex-1 text-[11px]">
                  {plan.pre_metadata.pre_erasure_sha256}
                </p>
                <Button
                  size="sm"
                  isIconOnly
                  variant="flat"
                  onPress={() => {
                    navigator.clipboard.writeText(plan.pre_metadata.pre_erasure_sha256 || '');
                    setCopiedHash(true);
                    setTimeout(() => setCopiedHash(false), 2000);
                  }}
                  className="border border-slate-300 dark:border-[#26313D] h-8 w-8"
                >
                  {copiedHash ? <Check className="w-3.5 h-3.5 text-slate-900 dark:text-slate-100" /> : <Copy className="w-3.5 h-3.5" />}
                </Button>
              </div>
            </div>
          )}

          {/* Limitations List */}
          {plan.limitations.length > 0 && (
            <div className="space-y-1 text-xs">
              <span className="text-xs font-semibold text-slate-700 dark:text-slate-300">
                Forensic Limitations & Caveats:
              </span>
              <ul className="text-[11px] text-slate-600 dark:text-slate-400 space-y-1 list-disc list-inside bg-slate-50 dark:bg-[#151C24] p-3 rounded-xs border border-slate-200 dark:border-[#26313D]">
                {plan.limitations.map((lim, idx) => (
                  <li key={idx}>{lim}</li>
                ))}
              </ul>
            </div>
          )}

          {/* Action Buttons */}
          <div className="pt-3 flex justify-between items-center border-t border-slate-200 dark:border-[#26313D]">
            <Button
              size="sm"
              variant="light"
              onPress={handleReset}
              className="text-xs text-slate-600 dark:text-slate-400"
            >
              Cancel & Choose Another Target
            </Button>
            <Button
              size="sm"
              onPress={handleOpenConfirmation}
              startContent={<ShieldAlert className="w-4 h-4" />}
              className="bg-rose-700 hover:bg-rose-800 text-white text-xs font-semibold px-4 shadow-xs"
            >
              Proceed to Two-Stage Authorization
            </Button>
          </div>
        </div>
      )}

      {/* Two-Stage Confirmation Challenge Modal */}
      <Modal
        isOpen={showConfirmModal && !!plan}
        onClose={() => setShowConfirmModal(false)}
        size="lg"
        classNames={{
          base: 'bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D]',
          header: 'border-b border-rose-100 dark:border-rose-900/30',
          footer: 'border-t border-slate-200 dark:border-[#26313D]',
        }}
      >
        <ModalContent>
          {plan && (
            <>
              <ModalHeader className="flex items-center space-x-2 text-rose-700 dark:text-rose-400">
                <ShieldAlert className="w-5 h-5" />
                <h3 className="text-sm font-bold tracking-tight">
                  Two-Stage Destructive Confirmation Challenge
                </h3>
              </ModalHeader>

              <ModalBody className="py-4 space-y-4">
                <div className="p-3 bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/50 rounded-xs text-rose-900 dark:text-rose-200 text-xs space-y-1">
                  <p className="font-bold">WARNING: IRREVERSIBLE OPERATION</p>
                  <p>
                    Executing this operation will physically overwrite and unlink the target filesystem entry. Target contents cannot be restored by standard carving tools.
                  </p>
                </div>

                <div className="space-y-1 text-xs">
                  <span className="text-slate-500 font-medium">Target to be Destroyed:</span>
                  <p className="font-mono text-slate-900 dark:text-slate-100 bg-slate-100 dark:bg-[#1B2530] p-2 rounded-xs border border-slate-200 dark:border-[#26313D] break-all">
                    {plan.canonical_path}
                  </p>
                </div>

                <div className="space-y-1.5">
                  <label className="text-xs font-semibold text-slate-800 dark:text-slate-200">
                    Type the exact target path to confirm:
                  </label>
                  <Input
                    size="sm"
                    value={typedConfirmation}
                    onValueChange={setTypedConfirmation}
                    placeholder={plan.canonical_path}
                    className="font-mono text-xs"
                    classNames={{
                      inputWrapper: 'bg-white dark:bg-[#151C24] border border-slate-300 dark:border-[#26313D]',
                    }}
                  />
                </div>

                <Checkbox
                  size="sm"
                  color="danger"
                  isSelected={warningAcknowledged}
                  onValueChange={setWarningAcknowledged}
                  classNames={{
                    label: 'text-xs text-slate-700 dark:text-slate-300',
                  }}
                >
                  I acknowledge the destructive consequences and confirm that I have legal authorization to destroy this target.
                </Checkbox>
              </ModalBody>

              <ModalFooter>
                <Button
                  size="sm"
                  variant="flat"
                  onPress={() => setShowConfirmModal(false)}
                  className="text-xs border border-slate-200 dark:border-[#26313D] bg-white dark:bg-[#151C24] text-slate-700 dark:text-slate-300"
                >
                  Abort
                </Button>
                <Button
                  size="sm"
                  onPress={handleExecute}
                  isDisabled={
                    !warningAcknowledged ||
                    (typedConfirmation.trim().toLowerCase() !== plan.canonical_path.trim().toLowerCase() &&
                     typedConfirmation.trim().toLowerCase() !== plan.target_path.trim().toLowerCase())
                  }
                  startContent={<ShieldAlert className="w-3.5 h-3.5" />}
                  className="bg-rose-700 hover:bg-rose-800 text-white text-xs font-bold px-4 shadow-xs"
                >
                  Authorize & Execute Sanitization
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>

      {/* Execution in Progress View */}
      {isExecuting && (
        <div className="text-center py-10 space-y-4 border border-slate-200 dark:border-[#26313D] rounded-sm bg-slate-50/50 dark:bg-[#151C24]/30">
          <div className="w-8 h-8 border-2 border-slate-300 border-t-slate-800 dark:border-t-slate-100 rounded-full animate-spin mx-auto" />
          <div className="space-y-1">
            <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100">
              Sanitization in Progress...
            </h3>
            <p className="text-xs text-slate-500 dark:text-slate-400 max-w-sm mx-auto">
              Performing multi-pass streaming overwrite (64 KB bounded memory chunks) and filesystem metadata unlinking.
            </p>
          </div>
        </div>
      )}

      {/* Post-Sanitization Summary Report (Flat) */}
      {result && (
        <div className="space-y-5 pt-2">
          <div className="flex items-center justify-between border-b border-slate-200 dark:border-[#26313D] pb-3">
            <div className="flex items-center space-x-2">
              <CheckCircle2 className="w-5 h-5 text-slate-700 dark:text-slate-300" />
              <h2 className="text-sm font-bold text-slate-900 dark:text-slate-100">
                Post-Sanitization Audit & Verification Report
              </h2>
            </div>
            <Chip size="sm" variant="flat" className="font-bold bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-[#26313D]">
              {result.status.toUpperCase()}
            </Chip>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
            <div className="bg-slate-50 dark:bg-[#1B2530] p-3 rounded-xs border border-slate-200 dark:border-[#26313D] space-y-1">
              <span className="text-slate-500 font-medium">Verification Outcome:</span>
              <p className="font-semibold text-slate-900 dark:text-slate-100 text-sm flex items-center gap-1">
                <CheckCircle2 className="w-4 h-4 text-slate-700 dark:text-slate-300" /> {result.verification.outcome}
              </p>
              <p className="text-[10px] text-slate-500">{result.verification.strategy}</p>
            </div>
            <div className="bg-slate-50 dark:bg-[#1B2530] p-3 rounded-xs border border-slate-200 dark:border-[#26313D] space-y-1">
              <span className="text-slate-500 font-medium">Bytes Overwritten:</span>
              <p className="font-semibold text-slate-800 dark:text-slate-200 text-sm font-mono">
                {formatBytes(result.bytes_processed)}
              </p>
              <p className="text-[10px] text-slate-500">
                {result.bytes_processed.toLocaleString()} bytes total
              </p>
            </div>
            <div className="bg-slate-50 dark:bg-[#1B2530] p-3 rounded-xs border border-slate-200 dark:border-[#26313D] space-y-1">
              <span className="text-slate-500 font-medium">Filesystem Status:</span>
              <p className="font-semibold text-slate-800 dark:text-slate-200 text-sm">
                {result.verification.inaccessible ? 'Inaccessible (Unlinked)' : 'Present'}
              </p>
              <p className="text-[10px] text-slate-500">Directory entry unlinked</p>
            </div>
          </div>

          {/* Target and Operation Details */}
          <div className="space-y-2 text-xs">
            <div className="space-y-0.5">
              <span className="text-slate-500 font-medium">Target Path:</span>
              <p className="font-mono text-slate-900 dark:text-slate-100 bg-slate-50 dark:bg-[#1B2530] p-2 rounded-xs border border-slate-200 dark:border-[#26313D] break-all">
                {result.canonical_path}
              </p>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
              <div className="bg-slate-50 dark:bg-[#1B2530] p-2 rounded-xs border border-slate-200 dark:border-[#26313D]">
                <span className="text-slate-500">Operation ID: </span>
                <span className="font-mono text-slate-800 dark:text-slate-200">{result.operation_id}</span>
              </div>
              <div className="bg-slate-50 dark:bg-[#1B2530] p-2 rounded-xs border border-slate-200 dark:border-[#26313D]">
                <span className="text-slate-500">Method: </span>
                <span className="font-mono text-slate-800 dark:text-slate-200">{result.method}</span>
              </div>
            </div>
          </div>

          {/* Folder Stats if Applicable */}
          {result.folder_stats && (
            <div className="bg-slate-50 dark:bg-[#1B2530] p-3 rounded-xs border border-slate-200 dark:border-[#26313D] space-y-2 text-xs">
              <span className="font-semibold text-slate-700 dark:text-slate-300">Folder Traversal Statistics:</span>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-2 text-center">
                <div className="bg-white dark:bg-[#151C24] p-2 rounded-xs border border-slate-200 dark:border-[#26313D]">
                  <div className="text-slate-500 text-[10px]">Files Shredded</div>
                  <div className="font-bold text-slate-800 dark:text-slate-200">{result.folder_stats.files_sanitized}</div>
                </div>
                <div className="bg-white dark:bg-[#151C24] p-2 rounded-xs border border-slate-200 dark:border-[#26313D]">
                  <div className="text-slate-500 text-[10px]">Directories Removed</div>
                  <div className="font-bold text-slate-800 dark:text-slate-200">{result.folder_stats.directories_removed}</div>
                </div>
                <div className="bg-white dark:bg-[#151C24] p-2 rounded-xs border border-slate-200 dark:border-[#26313D]">
                  <div className="text-slate-500 text-[10px]">Failed Items</div>
                  <div className="font-bold text-slate-800 dark:text-slate-200">{result.folder_stats.files_failed}</div>
                </div>
                <div className="bg-white dark:bg-[#151C24] p-2 rounded-xs border border-slate-200 dark:border-[#26313D]">
                  <div className="text-slate-500 text-[10px]">Cancelled Items</div>
                  <div className="font-bold text-slate-800 dark:text-slate-200">{result.folder_stats.files_cancelled}</div>
                </div>
              </div>
            </div>
          )}

          {/* Verification Details */}
          <div className="space-y-1 text-xs">
            <span className="text-slate-500 font-medium">Verification Summary:</span>
            <p className="text-slate-700 dark:text-slate-300 bg-slate-50 dark:bg-[#1B2530] p-2.5 rounded-xs border border-slate-200 dark:border-[#26313D]">
              {result.verification.details}
            </p>
          </div>

          {/* Limitations */}
          <div className="space-y-1 text-[11px] text-slate-500 bg-amber-50/50 dark:bg-amber-950/20 p-3 rounded-xs border border-amber-200 dark:border-amber-900/40">
            <span className="font-semibold text-amber-800 dark:text-amber-300">Physical Storage Limitations Disclaimer:</span>
            <ul className="list-disc list-inside space-y-0.5 mt-1 text-amber-700 dark:text-amber-400">
              {result.limitations.map((lim, i) => (
                <li key={i}>{lim}</li>
              ))}
            </ul>
          </div>

          {/* Action Buttons */}
          <div className="flex justify-between items-center pt-3 border-t border-slate-200 dark:border-[#26313D]">
            <Button
              size="sm"
              variant="flat"
              onPress={handleReset}
              startContent={<RotateCcw className="w-3.5 h-3.5" />}
              className="bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 text-xs font-medium border border-slate-200 dark:border-[#26313D]"
            >
              Plan Another Erasure
            </Button>
            <Button
              size="sm"
              onPress={handleExportJson}
              startContent={<Download className="w-3.5 h-3.5" />}
              className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 text-xs font-semibold shadow-xs"
            >
              Export Report (JSON)
            </Button>
          </div>
        </div>
      )}
    </div>
  );
};

export default FileEraserPage;
