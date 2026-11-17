import React, { useState, useMemo } from 'react';
import {
  Activity,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Clock,
  RefreshCw,
  Search,
  Filter,
  Copy,
  Check,
  Play,
  StopCircle,
  FileText,
  Shield,
  Layers,
  Info,
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
  Table,
  TableHeader,
  TableColumn,
  TableBody,
  TableRow,
  TableCell,
} from '@heroui/react';
import { OperationState, OperationType } from '../types/operation';
import { useOperation } from '../hooks/useOperation';

function formatBytes(bytes?: number | null): string {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

export const OperationsPage: React.FC = () => {
  const {
    operations,
    loading,
    error,
    stateFilter,
    setStateFilter,
    typeFilter,
    setTypeFilter,
    selectedOperation,
    setSelectedOperation,
    refresh,
    cancel,
    submitHash,
    submitVerify,
  } = useOperation();

  const [searchQuery, setSearchQuery] = useState('');
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  // New Operation Modal State
  const [showNewModal, setShowNewModal] = useState(false);
  const [modalMode, setModalMode] = useState<'hash' | 'verify'>('hash');
  const [modalPath, setModalPath] = useState('');
  const [modalExpectedDigest, setModalExpectedDigest] = useState('');
  const [modalError, setModalError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleCopy = (text: string, key: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const handleCreateSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!modalPath.trim()) {
      setModalError('Target file path is required.');
      return;
    }

    if (modalMode === 'verify') {
      const cleaned = modalExpectedDigest.trim().toLowerCase();
      if (!cleaned) {
        setModalError('Expected SHA-256 digest is required for verification.');
        return;
      }
      if (cleaned.length !== 64 || !/^[0-9a-f]{64}$/.test(cleaned)) {
        setModalError('Expected digest must be exactly 64 hexadecimal characters.');
        return;
      }
    }

    try {
      setSubmitting(true);
      setModalError(null);

      if (modalMode === 'hash') {
        await submitHash(modalPath.trim());
      } else {
        await submitVerify(modalPath.trim(), modalExpectedDigest.trim().toLowerCase());
      }

      setShowNewModal(false);
      setModalPath('');
      setModalExpectedDigest('');
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Failed to launch operation';
      setModalError(msg);
    } finally {
      setSubmitting(false);
    }
  };

  const filteredOperations = useMemo(() => {
    return operations.filter((op) => {
      if (searchQuery.trim()) {
        const query = searchQuery.toLowerCase();
        const matchesId = op.operation_id.toLowerCase().includes(query);
        const matchesTarget =
          op.target.display_name.toLowerCase().includes(query) ||
          op.target.identifier.toLowerCase().includes(query);
        const matchesActor = (op.actor_id || '').toLowerCase().includes(query);
        if (!matchesId && !matchesTarget && !matchesActor) return false;
      }
      return true;
    });
  }, [operations, searchQuery]);

  const stats = useMemo(() => {
    const total = operations.length;
    const active = operations.filter(
      (o) => o.current_state === 'Running' || o.current_state === 'Queued' || o.current_state === 'Cancelling'
    ).length;
    const completed = operations.filter((o) => o.current_state === 'Completed').length;
    const failedOrCancelled = operations.filter(
      (o) => o.current_state === 'Failed' || o.current_state === 'Cancelled'
    ).length;
    return { total, active, completed, failedOrCancelled };
  }, [operations]);

  const renderStateBadge = (state: OperationState) => {
    switch (state) {
      case 'Completed':
        return (
          <Chip
            size="sm"
            variant="flat"
            startContent={<CheckCircle2 className="w-3 h-3 text-slate-700 dark:text-slate-300" />}
            className="text-[11px] font-semibold bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700"
          >
            Completed
          </Chip>
        );
      case 'Running':
        return (
          <Chip
            size="sm"
            variant="flat"
            startContent={<RefreshCw className="w-3 h-3 animate-spin text-slate-700 dark:text-slate-300" />}
            className="text-[11px] font-semibold bg-slate-100 dark:bg-workstation-surface text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700 animate-pulse"
          >
            Running
          </Chip>
        );
      case 'Queued':
        return (
          <Chip
            size="sm"
            variant="flat"
            startContent={<Clock className="w-3 h-3 text-amber-700 dark:text-amber-400" />}
            className="text-[11px] font-semibold bg-amber-50 dark:bg-amber-950/30 text-amber-700 dark:text-amber-400 border border-amber-200 dark:border-amber-900/40"
          >
            Queued
          </Chip>
        );
      case 'Cancelling':
        return (
          <Chip
            size="sm"
            variant="flat"
            startContent={<StopCircle className="w-3 h-3 text-amber-700 dark:text-amber-400 animate-pulse" />}
            className="text-[11px] font-semibold bg-amber-50 dark:bg-amber-950/30 text-amber-700 dark:text-amber-400 border border-amber-200 dark:border-amber-900/40"
          >
            Cancelling
          </Chip>
        );
      case 'Cancelled':
        return (
          <Chip
            size="sm"
            variant="flat"
            startContent={<StopCircle className="w-3 h-3 text-slate-500" />}
            className="text-[11px] font-semibold bg-slate-100 dark:bg-workstation-surface text-slate-600 dark:text-slate-400 border border-slate-200 dark:border-workstation-border"
          >
            Cancelled
          </Chip>
        );
      case 'Failed':
        return (
          <Chip
            size="sm"
            variant="flat"
            startContent={<XCircle className="w-3 h-3 text-rose-600 dark:text-rose-400" />}
            className="text-[11px] font-semibold bg-rose-50 dark:bg-rose-950/30 text-rose-700 dark:text-rose-400 border border-rose-200 dark:border-rose-900/40"
          >
            Failed
          </Chip>
        );
      case 'Created':
      default:
        return (
          <Chip
            size="sm"
            variant="flat"
            startContent={<Info className="w-3 h-3 text-slate-500" />}
            className="text-[11px] font-semibold bg-slate-50 dark:bg-workstation-surface text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-workstation-border"
          >
            Created
          </Chip>
        );
    }
  };

  const renderTypeBadge = (opType: OperationType) => {
    switch (opType) {
      case 'IntegrityHash':
        return (
          <Chip size="sm" variant="flat" className="text-[11px] font-medium text-slate-800 dark:text-slate-200 bg-slate-100 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border">
            SHA-256 Hashing
          </Chip>
        );
      case 'IntegrityVerify':
        return (
          <Chip size="sm" variant="flat" className="text-[11px] font-medium text-slate-800 dark:text-slate-200 bg-slate-100 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border">
            Integrity Verification
          </Chip>
        );
      case 'DriveErasure':
      case 'FileErasure':
      case 'FolderErasure':
        return (
          <Chip size="sm" variant="flat" className="text-[11px] font-medium text-slate-500 dark:text-slate-400 bg-slate-100 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border">
            {opType} (Disabled)
          </Chip>
        );
      default:
        return (
          <Chip size="sm" variant="flat" className="text-[11px] font-medium text-slate-600 dark:text-slate-300 bg-slate-100 dark:bg-workstation-surface">
            {opType}
          </Chip>
        );
    }
  };

  return (
    <div className="space-y-6 w-full min-w-0">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 border-b border-slate-200 dark:border-workstation-border pb-5">
        <div>
          <div className="flex items-center gap-2">
            <div className="p-2 rounded-lg bg-slate-100 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border text-slate-800 dark:text-slate-200">
              <Activity className="w-6 h-6" />
            </div>
            <div>
              <h1 className="text-xl font-bold text-slate-900 dark:text-slate-100 tracking-tight">
                Operation Manager & Lifecycle
              </h1>
              <p className="text-sm text-slate-500 dark:text-slate-400">
                Asynchronous task orchestration, progress telemetry, cooperative cancellation, and audit correlation.
              </p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <Button
            size="sm"
            variant="flat"
            onPress={() => refresh()}
            isDisabled={loading}
            startContent={<RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />}
            className="text-xs font-medium text-slate-700 dark:text-slate-300 bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border shadow-2xs"
          >
            Refresh
          </Button>

          <Button
            size="sm"
            onPress={() => setShowNewModal(true)}
            startContent={<Play className="w-3.5 h-3.5" />}
            className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 text-xs font-semibold px-4 shadow-sm"
          >
            New Integrity Operation
          </Button>
        </div>
      </div>

      {/* Safety Banner */}
      <div className="border-l-2 border-slate-500 bg-slate-50 dark:bg-workstation-surface p-3 text-xs text-slate-800 dark:text-slate-200 flex items-start gap-3">
        <Shield className="w-5 h-5 text-slate-700 dark:text-slate-300 shrink-0 mt-0.5" />
        <div>
          <p className="font-semibold text-slate-900 dark:text-slate-100">Operation Orchestration Safety Boundary</p>
          <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
            The Operation Manager coordinates authorized tasks without executing destructive storage routines.
            Only read-only integrity operations (SHA-256 calculation and verification) are executable in this stage.
            Destructive actions remain disabled.
          </p>
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="p-4 rounded-lg bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 text-sm text-rose-700 dark:text-rose-300 flex items-start gap-3">
          <AlertTriangle className="w-5 h-5 text-rose-500 shrink-0 mt-0.5" />
          <div className="flex-1">
            <p className="font-semibold">Operation Manager Error</p>
            <p className="text-xs text-rose-600 dark:text-rose-400 mt-0.5">{error}</p>
          </div>
        </div>
      )}

      {/* Inline Metrics Ribbon */}
      <div className="border border-slate-200 dark:border-workstation-border rounded-xs bg-white dark:bg-workstation-panel divide-y sm:divide-y-0 sm:divide-x divide-slate-200 dark:divide-workstation-border grid grid-cols-2 sm:grid-cols-4">
        <div className="p-4">
          <span className="text-[11px] font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">
            Total Operations
          </span>
          <p className="text-2xl font-bold text-slate-900 dark:text-slate-100 mt-1 font-mono">{stats.total}</p>
        </div>

        <div className="p-4">
          <span className="text-[11px] font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">
            In-Flight / Active
          </span>
          <p className="text-2xl font-bold text-slate-900 dark:text-slate-100 mt-1 font-mono">{stats.active}</p>
        </div>

        <div className="p-4">
          <span className="text-[11px] font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">
            Completed
          </span>
          <p className="text-2xl font-bold text-slate-900 dark:text-slate-100 mt-1 font-mono">{stats.completed}</p>
        </div>

        <div className="p-4">
          <span className="text-[11px] font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">
            Failed / Cancelled
          </span>
          <p className="text-2xl font-bold text-slate-700 dark:text-slate-300 mt-1 font-mono">{stats.failedOrCancelled}</p>
        </div>
      </div>

      {/* Filter and Search Bar */}
      <div className="border border-slate-200 dark:border-workstation-border rounded-xs bg-white dark:bg-workstation-panel p-3 flex flex-col sm:flex-row items-center justify-between gap-3">
        <div className="w-full sm:w-72">
          <Input
            size="sm"
            placeholder="Search by ID, target, operator..."
            value={searchQuery}
            onValueChange={setSearchQuery}
            startContent={<Search className="w-3.5 h-3.5 text-slate-400 shrink-0" />}
            classNames={{
              inputWrapper: 'bg-white dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border',
            }}
          />
        </div>

        <div className="flex items-center gap-3 w-full sm:w-auto justify-end text-xs">
          <div className="flex items-center gap-1.5">
            <Filter className="w-3.5 h-3.5 text-slate-400" />
            <span className="text-slate-500 dark:text-slate-400 font-medium">State:</span>
            <select
              value={stateFilter}
              onChange={(e) => setStateFilter(e.target.value)}
              className="px-2 py-1 text-xs rounded-md border border-slate-300 dark:border-workstation-border bg-white dark:bg-workstation-surface text-slate-700 dark:text-slate-300"
            >
              <option value="ALL">All States</option>
              <option value="Running">Running</option>
              <option value="Queued">Queued</option>
              <option value="Completed">Completed</option>
              <option value="Failed">Failed</option>
              <option value="Cancelled">Cancelled</option>
            </select>
          </div>

          <div className="flex items-center gap-1.5">
            <span className="text-slate-500 dark:text-slate-400 font-medium">Type:</span>
            <select
              value={typeFilter}
              onChange={(e) => setTypeFilter(e.target.value)}
              className="px-2 py-1 text-xs rounded-md border border-slate-300 dark:border-workstation-border bg-white dark:bg-workstation-surface text-slate-700 dark:text-slate-300"
            >
              <option value="ALL">All Types</option>
              <option value="IntegrityHash">SHA-256 Hashing</option>
              <option value="IntegrityVerify">Integrity Verification</option>
            </select>
          </div>
        </div>
      </div>

      {/* Operations Table */}
      <div className="border border-slate-200 dark:border-workstation-border rounded-xs overflow-hidden bg-white dark:bg-workstation-panel min-w-0">
        <div className="w-full overflow-x-auto scrollbar-thin min-w-0">
          <Table
            aria-label="Operations Table"
            removeWrapper
            classNames={{
              base: 'min-w-[760px]',
              table: 'min-w-[760px]',
              th: 'bg-slate-50 dark:bg-workstation-surface text-slate-500 dark:text-slate-400 text-[11px] font-semibold uppercase tracking-wider',
              td: 'py-3 text-xs text-slate-700 dark:text-slate-300 border-b border-slate-100 dark:border-workstation-border',
            }}
          >
          <TableHeader>
            <TableColumn>STATUS</TableColumn>
            <TableColumn>OPERATION ID</TableColumn>
            <TableColumn>TYPE</TableColumn>
            <TableColumn>TARGET REFERENCE</TableColumn>
            <TableColumn>PROGRESS / STAGE</TableColumn>
            <TableColumn>OPERATOR</TableColumn>
            <TableColumn>CREATED</TableColumn>
            <TableColumn className="text-right">ACTIONS</TableColumn>
          </TableHeader>
          <TableBody emptyContent="No matching operations found. Launch a new integrity operation to begin.">
            {filteredOperations.map((op) => {
              const isCancellable =
                op.current_state === 'Running' || op.current_state === 'Queued';

              return (
                <TableRow
                  key={op.operation_id}
                  className="hover:bg-slate-50/60 dark:hover:bg-workstation-surface/50 cursor-pointer"
                  onClick={() => setSelectedOperation(op)}
                >
                  <TableCell>
                    {renderStateBadge(op.current_state)}
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1 font-mono text-[11px] text-slate-700 dark:text-slate-300">
                      <span>{op.operation_id.substring(0, 8)}...</span>
                      <Button
                        isIconOnly
                        size="sm"
                        variant="light"
                        onPress={() => handleCopy(op.operation_id, op.operation_id)}
                        className="w-5 h-5 min-w-5 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
                        title="Copy full UUID"
                      >
                        {copiedKey === op.operation_id ? (
                          <Check className="w-3 h-3 text-slate-800 dark:text-slate-200" />
                        ) : (
                          <Copy className="w-3 h-3" />
                        )}
                      </Button>
                    </div>
                  </TableCell>
                  <TableCell>
                    {renderTypeBadge(op.operation_type)}
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2 max-w-xs">
                      <FileText className="w-4 h-4 text-slate-400 shrink-0" />
                      <div className="truncate">
                        <p className="font-semibold text-slate-800 dark:text-slate-200 truncate" title={op.target.display_name}>
                          {op.target.display_name}
                        </p>
                        <p className="text-[10px] text-slate-400 font-mono truncate" title={op.target.identifier}>
                          {op.target.identifier}
                        </p>
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="min-w-[140px]">
                      <div className="flex justify-between items-center text-[10px] text-slate-500 dark:text-slate-400 mb-1">
                        <span className="truncate max-w-[100px]">{op.progress.stage}</span>
                        <span className="font-semibold font-mono">
                          {op.progress.percentage !== null && op.progress.percentage !== undefined
                            ? `${Math.round(op.progress.percentage)}%`
                            : '--'}
                        </span>
                      </div>
                      <Progress
                        aria-label="Operation progress"
                        value={
                          op.progress.percentage !== null && op.progress.percentage !== undefined
                            ? op.progress.percentage
                            : op.current_state === 'Completed'
                            ? 100
                            : 25
                        }
                        size="sm"
                        classNames={{
                          indicator: op.current_state === 'Failed' ? 'bg-rose-500' : 'bg-slate-900 dark:bg-slate-100',
                          track: 'bg-slate-100 dark:bg-workstation-surface',
                        }}
                      />
                    </div>
                  </TableCell>
                  <TableCell>
                    {op.actor_id || <span className="text-slate-400 italic">system</span>}
                  </TableCell>
                  <TableCell className="font-mono text-[11px] text-slate-400">
                    {new Date(op.created_at).toLocaleTimeString()}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-2" onClick={(e) => e.stopPropagation()}>
                      <Button
                        size="sm"
                        variant="light"
                        onPress={() => setSelectedOperation(op)}
                        className="text-[11px] font-medium h-7 px-2.5 text-slate-700 dark:text-slate-300"
                      >
                        Details
                      </Button>

                      {isCancellable && (
                        <Button
                          size="sm"
                          variant="flat"
                          color="danger"
                          onPress={() => cancel(op.operation_id)}
                          className="text-[11px] font-medium h-7 px-2.5"
                          title="Request cooperative cancellation"
                        >
                          Cancel
                        </Button>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        </div>
      </div>

      {/* Operation Detail Modal */}
      <Modal
        isOpen={!!selectedOperation}
        onClose={() => setSelectedOperation(null)}
        size="2xl"
        classNames={{
          base: 'bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border',
          header: 'border-b border-slate-100 dark:border-workstation-border',
          footer: 'border-t border-slate-100 dark:border-workstation-border',
        }}
      >
        <ModalContent>
          {selectedOperation && (
            <>
              <ModalHeader className="flex items-center gap-2">
                <Layers className="w-5 h-5 text-slate-700 dark:text-slate-300" />
                <div>
                  <h3 className="text-sm font-bold text-slate-900 dark:text-slate-100">
                    Operation Details
                  </h3>
                  <p className="text-[11px] font-mono text-slate-500 font-normal">
                    {selectedOperation.operation_id}
                  </p>
                </div>
              </ModalHeader>

              <ModalBody className="p-6 space-y-4 text-xs">
                {/* Status Header */}
                <div className="flex items-center justify-between p-3.5 rounded-lg bg-slate-50 dark:bg-workstation-surface border border-slate-100 dark:border-workstation-border">
                  <div className="flex items-center gap-2">
                    <span className="text-slate-500 dark:text-slate-400 font-medium">Lifecycle State:</span>
                    {renderStateBadge(selectedOperation.current_state)}
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-slate-500 dark:text-slate-400 font-medium">Type:</span>
                    {renderTypeBadge(selectedOperation.operation_type)}
                  </div>
                </div>

                {/* Target Metadata */}
                <div className="grid grid-cols-2 gap-4 p-4 rounded-lg bg-slate-50/70 dark:bg-workstation-surface border border-slate-100 dark:border-workstation-border">
                  <div>
                    <span className="text-[10px] text-slate-400 uppercase font-semibold block">Target Name</span>
                    <span className="font-semibold text-slate-800 dark:text-slate-200 text-xs block mt-0.5">
                      {selectedOperation.target.display_name}
                    </span>
                  </div>
                  <div>
                    <span className="text-[10px] text-slate-400 uppercase font-semibold block">Target Category</span>
                    <span className="font-medium text-slate-700 dark:text-slate-300 text-xs block mt-0.5">
                      {selectedOperation.target.target_type}
                    </span>
                  </div>
                  <div className="col-span-2">
                    <span className="text-[10px] text-slate-400 uppercase font-semibold block">Normalized Path</span>
                    <span className="font-mono text-[11px] text-slate-700 dark:text-slate-300 block mt-0.5 break-all">
                      {selectedOperation.target.identifier}
                    </span>
                  </div>
                  <div>
                    <span className="text-[10px] text-slate-400 uppercase font-semibold block">Size</span>
                    <span className="font-medium text-slate-700 dark:text-slate-300 text-xs block mt-0.5">
                      {formatBytes(selectedOperation.target.size_bytes)}
                    </span>
                  </div>
                  <div>
                    <span className="text-[10px] text-slate-400 uppercase font-semibold block">Operator</span>
                    <span className="font-medium text-slate-700 dark:text-slate-300 text-xs block mt-0.5">
                      {selectedOperation.actor_id || 'System'}
                    </span>
                  </div>
                </div>

                {/* Progress Detail */}
                <div className="space-y-2">
                  <div className="flex justify-between items-center text-xs">
                    <span className="font-semibold text-slate-700 dark:text-slate-300">Execution Progress</span>
                    <span className="text-slate-500 dark:text-slate-400 font-medium font-mono">
                      {selectedOperation.progress.percentage !== null &&
                      selectedOperation.progress.percentage !== undefined
                        ? `${Math.round(selectedOperation.progress.percentage)}%`
                        : 'Indeterminate'}
                    </span>
                  </div>
                  <Progress
                    aria-label="Progress detail"
                    value={
                      selectedOperation.progress.percentage !== null &&
                      selectedOperation.progress.percentage !== undefined
                        ? selectedOperation.progress.percentage
                        : selectedOperation.current_state === 'Completed'
                        ? 100
                        : 20
                    }
                    size="md"
                    classNames={{
                      indicator: selectedOperation.current_state === 'Failed' ? 'bg-rose-500' : 'bg-slate-900 dark:bg-slate-100',
                      track: 'bg-slate-100 dark:bg-workstation-surface',
                    }}
                  />
                  <p className="text-[11px] text-slate-500 dark:text-slate-400 italic">
                    {selectedOperation.progress.stage}: {selectedOperation.progress.message}
                  </p>
                </div>

                {/* Execution Summary or Failure */}
                {selectedOperation.result_summary && (
                  <div className="p-3.5 rounded-lg bg-slate-100 dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border">
                    <span className="text-[10px] text-slate-700 dark:text-slate-300 font-bold uppercase block tracking-wider">
                      Execution Outcome
                    </span>
                    <p className="font-mono text-[11px] text-slate-900 dark:text-slate-100 mt-1 break-all select-all">
                      {selectedOperation.result_summary}
                    </p>
                  </div>
                )}

                {selectedOperation.failure_reason && (
                  <div className="p-3.5 rounded-lg bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40">
                    <span className="text-[10px] text-rose-800 dark:text-rose-300 font-bold uppercase block tracking-wider">
                      Failure Diagnostic
                    </span>
                    <p className="text-xs text-rose-900 dark:text-rose-200 mt-1 break-all">
                      {selectedOperation.failure_reason}
                    </p>
                  </div>
                )}

                {/* Timestamps */}
                <div className="grid grid-cols-3 gap-2 pt-2 border-t border-slate-100 dark:border-workstation-border text-[11px] text-slate-500 dark:text-slate-400">
                  <div>
                    <span className="block text-slate-400">Created At:</span>
                    <span className="font-mono text-slate-700 dark:text-slate-300">
                      {new Date(selectedOperation.created_at).toLocaleString()}
                    </span>
                  </div>
                  <div>
                    <span className="block text-slate-400">Started At:</span>
                    <span className="font-mono text-slate-700 dark:text-slate-300">
                      {selectedOperation.started_at
                        ? new Date(selectedOperation.started_at).toLocaleTimeString()
                        : '--'}
                    </span>
                  </div>
                  <div>
                    <span className="block text-slate-400">Completed At:</span>
                    <span className="font-mono text-slate-700 dark:text-slate-300">
                      {selectedOperation.completed_at
                        ? new Date(selectedOperation.completed_at).toLocaleTimeString()
                        : '--'}
                    </span>
                  </div>
                </div>
              </ModalBody>

              <ModalFooter>
                {(selectedOperation.current_state === 'Running' ||
                  selectedOperation.current_state === 'Queued') && (
                  <Button
                    size="sm"
                    color="danger"
                    onPress={() => {
                      cancel(selectedOperation.operation_id);
                      setSelectedOperation(null);
                    }}
                    className="text-xs font-semibold mr-auto"
                  >
                    Cancel Operation
                  </Button>
                )}
                <Button
                  size="sm"
                  variant="flat"
                  onPress={() => setSelectedOperation(null)}
                  className="text-xs font-medium bg-slate-100 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border text-slate-700 dark:text-slate-300"
                >
                  Close
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>

      {/* New Integrity Operation Modal */}
      <Modal
        isOpen={showNewModal}
        onClose={() => setShowNewModal(false)}
        size="lg"
        classNames={{
          base: 'bg-white dark:bg-workstation-panel border border-slate-200 dark:border-workstation-border',
          header: 'border-b border-slate-100 dark:border-workstation-border',
          footer: 'border-t border-slate-100 dark:border-workstation-border',
        }}
      >
        <ModalContent>
          <ModalHeader className="flex items-center gap-2">
            <Play className="w-4 h-4 text-slate-700 dark:text-slate-300" />
            <span className="text-sm font-bold text-slate-900 dark:text-slate-100">
              Launch Managed Integrity Operation
            </span>
          </ModalHeader>

          <form onSubmit={handleCreateSubmit}>
            <ModalBody className="p-6 space-y-4">
              {modalError && (
                <div className="p-3 rounded-lg bg-rose-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-900/40 text-xs text-rose-700 dark:text-rose-300">
                  {modalError}
                </div>
              )}

              {/* Mode Toggle */}
              <div className="space-y-1">
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider">
                  Operation Subtype
                </label>
                <div className="grid grid-cols-2 gap-2">
                  <Button
                    size="sm"
                    variant={modalMode === 'hash' ? 'solid' : 'flat'}
                    onPress={() => setModalMode('hash')}
                    className={`text-xs h-9 ${
                      modalMode === 'hash'
                        ? 'bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 font-semibold'
                        : 'bg-white dark:bg-workstation-surface text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-workstation-border'
                    }`}
                  >
                    SHA-256 Calculation
                  </Button>
                  <Button
                    size="sm"
                    variant={modalMode === 'verify' ? 'solid' : 'flat'}
                    onPress={() => setModalMode('verify')}
                    className={`text-xs h-9 ${
                      modalMode === 'verify'
                        ? 'bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 font-semibold'
                        : 'bg-white dark:bg-workstation-surface text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-workstation-border'
                    }`}
                  >
                    Integrity Verification
                  </Button>
                </div>
              </div>

              {/* Target File Path */}
              <div className="space-y-1">
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider">
                  Target Evidence File Path
                </label>
                <Input
                  size="sm"
                  placeholder="e.g. C:\Cases\evidence_001.raw or /tmp/disk.img"
                  value={modalPath}
                  onValueChange={setModalPath}
                  className="font-mono text-xs"
                  classNames={{
                    inputWrapper: 'bg-white dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border',
                  }}
                />
              </div>

              {/* Expected Digest (Verification Mode Only) */}
              {modalMode === 'verify' && (
                <div className="space-y-1">
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider">
                    Expected SHA-256 Digest (64 hex characters)
                  </label>
                  <Input
                    size="sm"
                    placeholder="e.g. e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    value={modalExpectedDigest}
                    onValueChange={setModalExpectedDigest}
                    className="font-mono text-xs"
                    classNames={{
                      inputWrapper: 'bg-white dark:bg-workstation-surface border border-slate-300 dark:border-workstation-border',
                    }}
                  />
                </div>
              )}

              {/* Non-Destructive Notice */}
              <div className="p-3 rounded-lg bg-slate-50 dark:bg-workstation-surface border border-slate-200 dark:border-workstation-border text-[11px] text-slate-500 dark:text-slate-400 flex items-start gap-2">
                <Shield className="w-4 h-4 text-slate-700 dark:text-slate-300 shrink-0 mt-0.5" />
                <span>
                  Strictly non-destructive streaming operation. Target evidence files are never modified,
                  formatted, or altered.
                </span>
              </div>
            </ModalBody>

            <ModalFooter>
              <Button
                size="sm"
                variant="flat"
                onPress={() => setShowNewModal(false)}
                className="text-xs font-medium text-slate-600 dark:text-slate-400"
              >
                Cancel
              </Button>
              <Button
                size="sm"
                type="submit"
                isDisabled={submitting || !modalPath.trim()}
                isLoading={submitting}
                className="bg-slate-900 hover:bg-slate-800 text-white dark:bg-slate-100 dark:text-slate-900 text-xs font-semibold px-4 shadow-sm"
              >
                {submitting ? 'Launching...' : 'Launch Asynchronous Operation'}
              </Button>
            </ModalFooter>
          </form>
        </ModalContent>
      </Modal>
    </div>
  );
};

export default OperationsPage;
