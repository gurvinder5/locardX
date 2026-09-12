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
  X,
  Copy,
  Check,
  Play,
  StopCircle,
  FileText,
  Shield,
  Layers,
  Info,
} from 'lucide-react';
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
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200">
            <CheckCircle2 className="w-3 h-3" />
            Completed
          </span>
        );
      case 'Running':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-semibold bg-blue-50 text-blue-700 border border-blue-200 animate-pulse">
            <RefreshCw className="w-3 h-3 animate-spin" />
            Running
          </span>
        );
      case 'Queued':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-semibold bg-amber-50 text-amber-700 border border-amber-200">
            <Clock className="w-3 h-3" />
            Queued
          </span>
        );
      case 'Cancelling':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-semibold bg-orange-50 text-orange-700 border border-orange-200">
            <StopCircle className="w-3 h-3 animate-pulse" />
            Cancelling
          </span>
        );
      case 'Cancelled':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-semibold bg-slate-100 text-slate-600 border border-slate-200">
            <StopCircle className="w-3 h-3" />
            Cancelled
          </span>
        );
      case 'Failed':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-semibold bg-rose-50 text-rose-700 border border-rose-200">
            <XCircle className="w-3 h-3" />
            Failed
          </span>
        );
      case 'Created':
      default:
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-semibold bg-slate-50 text-slate-700 border border-slate-200">
            <Info className="w-3 h-3" />
            Created
          </span>
        );
    }
  };

  const renderTypeBadge = (opType: OperationType) => {
    switch (opType) {
      case 'IntegrityHash':
        return (
          <span className="text-[11px] font-medium text-indigo-700 bg-indigo-50 px-2 py-0.5 rounded border border-indigo-100">
            SHA-256 Hashing
          </span>
        );
      case 'IntegrityVerify':
        return (
          <span className="text-[11px] font-medium text-teal-700 bg-teal-50 px-2 py-0.5 rounded border border-teal-100">
            Integrity Verification
          </span>
        );
      case 'DriveErasure':
      case 'FileErasure':
      case 'FolderErasure':
        return (
          <span className="text-[11px] font-medium text-slate-500 bg-slate-100 px-2 py-0.5 rounded border border-slate-200">
            {opType} (Disabled)
          </span>
        );
      default:
        return (
          <span className="text-[11px] font-medium text-slate-600 bg-slate-100 px-2 py-0.5 rounded">
            {opType}
          </span>
        );
    }
  };

  return (
    <div className="space-y-6 max-w-7xl mx-auto p-4 sm:p-6 lg:p-8">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 border-b border-slate-200 pb-5">
        <div>
          <div className="flex items-center gap-2">
            <div className="p-2 rounded-lg bg-indigo-50 border border-indigo-100 text-indigo-600">
              <Activity className="w-6 h-6" />
            </div>
            <div>
              <h1 className="text-xl font-bold text-slate-900">
                Operation Manager & Lifecycle
              </h1>
              <p className="text-sm text-slate-500">
                Asynchronous task orchestration, progress telemetry, cooperative cancellation, and audit correlation.
              </p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <button
            onClick={() => refresh()}
            disabled={loading}
            className="inline-flex items-center gap-1.5 px-3 py-2 text-xs font-medium rounded-lg text-slate-700 bg-white border border-slate-200 hover:bg-slate-50 transition-colors shadow-2xs"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
            Refresh
          </button>

          <button
            onClick={() => setShowNewModal(true)}
            className="inline-flex items-center gap-1.5 px-4 py-2 text-xs font-medium rounded-lg text-white bg-indigo-600 hover:bg-indigo-700 transition-colors shadow-sm"
          >
            <Play className="w-3.5 h-3.5" />
            New Integrity Operation
          </button>
        </div>
      </div>

      {/* Safety Banner */}
      <div className="p-4 rounded-lg bg-slate-100/80 border border-slate-200 text-sm text-slate-600 flex items-start gap-3">
        <Shield className="w-5 h-5 text-emerald-600 shrink-0 mt-0.5" />
        <div>
          <p className="font-medium text-slate-900">Operation Orchestration Safety Boundary</p>
          <p className="mt-0.5 text-xs text-slate-500">
            The Operation Manager coordinates authorized tasks without executing destructive storage routines.
            Only read-only integrity operations (SHA-256 calculation and verification) are executable in this stage.
            Destructive actions remain disabled.
          </p>
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="p-4 rounded-lg bg-rose-50 border border-rose-200 text-sm text-rose-700 flex items-start gap-3">
          <AlertTriangle className="w-5 h-5 text-rose-500 shrink-0 mt-0.5" />
          <div className="flex-1">
            <p className="font-semibold">Operation Manager Error</p>
            <p className="text-xs text-rose-600 mt-0.5">{error}</p>
          </div>
        </div>
      )}

      {/* Metric Cards */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
        <div className="bg-white p-4 rounded-xl border border-slate-200 shadow-2xs">
          <span className="text-[11px] font-semibold text-slate-500 uppercase tracking-wider">
            Total Operations
          </span>
          <p className="text-2xl font-bold text-slate-900 mt-1">{stats.total}</p>
        </div>

        <div className="bg-white p-4 rounded-xl border border-slate-200 shadow-2xs">
          <span className="text-[11px] font-semibold text-blue-600 uppercase tracking-wider">
            In-Flight / Active
          </span>
          <p className="text-2xl font-bold text-blue-700 mt-1">{stats.active}</p>
        </div>

        <div className="bg-white p-4 rounded-xl border border-slate-200 shadow-2xs">
          <span className="text-[11px] font-semibold text-emerald-600 uppercase tracking-wider">
            Completed
          </span>
          <p className="text-2xl font-bold text-emerald-700 mt-1">{stats.completed}</p>
        </div>

        <div className="bg-white p-4 rounded-xl border border-slate-200 shadow-2xs">
          <span className="text-[11px] font-semibold text-slate-500 uppercase tracking-wider">
            Failed / Cancelled
          </span>
          <p className="text-2xl font-bold text-slate-700 mt-1">{stats.failedOrCancelled}</p>
        </div>
      </div>

      {/* Filter and Search Bar */}
      <div className="flex flex-col sm:flex-row items-center justify-between gap-3 bg-white p-3 rounded-xl border border-slate-200 shadow-2xs">
        <div className="flex items-center gap-2 w-full sm:w-auto">
          <div className="relative w-full sm:w-64">
            <Search className="w-3.5 h-3.5 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              placeholder="Search by ID, target, operator..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-8 pr-3 py-1.5 text-xs rounded-lg border border-slate-300 focus:outline-none focus:ring-1 focus:ring-indigo-500"
            />
          </div>
        </div>

        <div className="flex items-center gap-3 w-full sm:w-auto justify-end text-xs">
          <div className="flex items-center gap-1.5">
            <Filter className="w-3.5 h-3.5 text-slate-400" />
            <span className="text-slate-500 font-medium">State:</span>
            <select
              value={stateFilter}
              onChange={(e) => setStateFilter(e.target.value)}
              className="px-2 py-1 text-xs rounded-md border border-slate-300 bg-white text-slate-700"
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
            <span className="text-slate-500 font-medium">Type:</span>
            <select
              value={typeFilter}
              onChange={(e) => setTypeFilter(e.target.value)}
              className="px-2 py-1 text-xs rounded-md border border-slate-300 bg-white text-slate-700"
            >
              <option value="ALL">All Types</option>
              <option value="IntegrityHash">SHA-256 Hashing</option>
              <option value="IntegrityVerify">Integrity Verification</option>
            </select>
          </div>
        </div>
      </div>

      {/* Operations Table */}
      <div className="bg-white rounded-xl border border-slate-200 shadow-2xs overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs text-slate-600">
            <thead className="bg-slate-50 text-[11px] font-semibold text-slate-500 uppercase tracking-wider border-b border-slate-100">
              <tr>
                <th className="px-6 py-3">Status</th>
                <th className="px-6 py-3">Operation ID</th>
                <th className="px-6 py-3">Type</th>
                <th className="px-6 py-3">Target Reference</th>
                <th className="px-6 py-3">Progress / Stage</th>
                <th className="px-6 py-3">Operator</th>
                <th className="px-6 py-3">Created</th>
                <th className="px-6 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {filteredOperations.length === 0 ? (
                <tr>
                  <td colSpan={8} className="px-6 py-12 text-center text-slate-400">
                    {loading
                      ? 'Loading operations...'
                      : 'No matching operations found. Launch a new integrity operation to begin.'}
                  </td>
                </tr>
              ) : (
                filteredOperations.map((op) => {
                  const isCancellable =
                    op.current_state === 'Running' || op.current_state === 'Queued';

                  return (
                    <tr
                      key={op.operation_id}
                      className="hover:bg-slate-50/60 transition-colors cursor-pointer"
                      onClick={() => setSelectedOperation(op)}
                    >
                      <td className="px-6 py-3.5 whitespace-nowrap">
                        {renderStateBadge(op.current_state)}
                      </td>
                      <td className="px-6 py-3.5 whitespace-nowrap">
                        <div className="flex items-center gap-1 font-mono text-[11px] text-slate-700">
                          <span>{op.operation_id.substring(0, 8)}...</span>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              handleCopy(op.operation_id, op.operation_id);
                            }}
                            className="text-slate-400 hover:text-slate-600 p-0.5"
                            title="Copy full UUID"
                          >
                            {copiedKey === op.operation_id ? (
                              <Check className="w-3 h-3 text-emerald-600" />
                            ) : (
                              <Copy className="w-3 h-3" />
                            )}
                          </button>
                        </div>
                      </td>
                      <td className="px-6 py-3.5 whitespace-nowrap">
                        {renderTypeBadge(op.operation_type)}
                      </td>
                      <td className="px-6 py-3.5">
                        <div className="flex items-center gap-2 max-w-xs">
                          <FileText className="w-4 h-4 text-slate-400 shrink-0" />
                          <div className="truncate">
                            <p className="font-semibold text-slate-800 truncate" title={op.target.display_name}>
                              {op.target.display_name}
                            </p>
                            <p className="text-[10px] text-slate-400 font-mono truncate" title={op.target.identifier}>
                              {op.target.identifier}
                            </p>
                          </div>
                        </div>
                      </td>
                      <td className="px-6 py-3.5 whitespace-nowrap min-w-[140px]">
                        <div>
                          <div className="flex justify-between items-center text-[10px] text-slate-500 mb-1">
                            <span className="truncate max-w-[100px]">{op.progress.stage}</span>
                            <span className="font-semibold">
                              {op.progress.percentage !== null && op.progress.percentage !== undefined
                                ? `${Math.round(op.progress.percentage)}%`
                                : '--'}
                            </span>
                          </div>
                          <div className="w-full bg-slate-100 rounded-full h-1.5 overflow-hidden">
                            <div
                              className={`h-1.5 rounded-full transition-all duration-300 ${
                                op.current_state === 'Completed'
                                  ? 'bg-emerald-500'
                                  : op.current_state === 'Failed'
                                  ? 'bg-rose-500'
                                  : 'bg-indigo-600'
                              }`}
                              style={{
                                width: `${
                                  op.progress.percentage !== null && op.progress.percentage !== undefined
                                    ? op.progress.percentage
                                    : op.current_state === 'Completed'
                                    ? 100
                                    : 25
                                }%`,
                              }}
                            />
                          </div>
                        </div>
                      </td>
                      <td className="px-6 py-3.5 whitespace-nowrap text-slate-600">
                        {op.actor_id || <span className="text-slate-400 italic">system</span>}
                      </td>
                      <td className="px-6 py-3.5 whitespace-nowrap text-slate-400 text-[11px]">
                        {new Date(op.created_at).toLocaleTimeString()}
                      </td>
                      <td className="px-6 py-3.5 whitespace-nowrap text-right">
                        <div className="flex items-center justify-end gap-2" onClick={(e) => e.stopPropagation()}>
                          <button
                            onClick={() => setSelectedOperation(op)}
                            className="px-2.5 py-1 text-[11px] font-medium text-indigo-600 hover:text-indigo-800 hover:bg-indigo-50 rounded"
                          >
                            Details
                          </button>

                          {isCancellable && (
                            <button
                              onClick={() => cancel(op.operation_id)}
                              className="px-2.5 py-1 text-[11px] font-medium text-rose-600 hover:text-rose-800 hover:bg-rose-50 rounded"
                              title="Request cooperative cancellation"
                            >
                              Cancel
                            </button>
                          )}
                        </div>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Operation Detail Modal */}
      {selectedOperation && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/40 backdrop-blur-xs">
          <div className="bg-white rounded-xl border border-slate-200 shadow-xl max-w-2xl w-full overflow-hidden animate-in fade-in duration-200">
            <div className="px-6 py-4 border-b border-slate-100 flex items-center justify-between bg-slate-50/60">
              <div className="flex items-center gap-2">
                <Layers className="w-5 h-5 text-indigo-600" />
                <div>
                  <h3 className="text-sm font-bold text-slate-900">
                    Operation Details
                  </h3>
                  <p className="text-[11px] font-mono text-slate-500">
                    {selectedOperation.operation_id}
                  </p>
                </div>
              </div>
              <button
                onClick={() => setSelectedOperation(null)}
                className="text-slate-400 hover:text-slate-600 p-1"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="p-6 space-y-5 text-xs">
              {/* Status Header */}
              <div className="flex items-center justify-between p-3.5 rounded-lg bg-slate-50 border border-slate-100">
                <div className="flex items-center gap-2">
                  <span className="text-slate-500 font-medium">Lifecycle State:</span>
                  {renderStateBadge(selectedOperation.current_state)}
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-slate-500 font-medium">Type:</span>
                  {renderTypeBadge(selectedOperation.operation_type)}
                </div>
              </div>

              {/* Target Metadata */}
              <div className="grid grid-cols-2 gap-4 p-4 rounded-lg bg-slate-50/70 border border-slate-100">
                <div>
                  <span className="text-[10px] text-slate-400 uppercase font-semibold block">Target Name</span>
                  <span className="font-semibold text-slate-800 text-xs block mt-0.5">
                    {selectedOperation.target.display_name}
                  </span>
                </div>
                <div>
                  <span className="text-[10px] text-slate-400 uppercase font-semibold block">Target Category</span>
                  <span className="font-medium text-slate-700 text-xs block mt-0.5">
                    {selectedOperation.target.target_type}
                  </span>
                </div>
                <div className="col-span-2">
                  <span className="text-[10px] text-slate-400 uppercase font-semibold block">Normalized Path</span>
                  <span className="font-mono text-[11px] text-slate-700 block mt-0.5 break-all">
                    {selectedOperation.target.identifier}
                  </span>
                </div>
                <div>
                  <span className="text-[10px] text-slate-400 uppercase font-semibold block">Size</span>
                  <span className="font-medium text-slate-700 text-xs block mt-0.5">
                    {formatBytes(selectedOperation.target.size_bytes)}
                  </span>
                </div>
                <div>
                  <span className="text-[10px] text-slate-400 uppercase font-semibold block">Operator</span>
                  <span className="font-medium text-slate-700 text-xs block mt-0.5">
                    {selectedOperation.actor_id || 'System'}
                  </span>
                </div>
              </div>

              {/* Progress Detail */}
              <div className="space-y-2">
                <div className="flex justify-between items-center text-xs">
                  <span className="font-semibold text-slate-700">Execution Progress</span>
                  <span className="text-slate-500 font-medium">
                    {selectedOperation.progress.percentage !== null &&
                    selectedOperation.progress.percentage !== undefined
                      ? `${Math.round(selectedOperation.progress.percentage)}%`
                      : 'Indeterminate'}
                  </span>
                </div>
                <div className="w-full bg-slate-100 rounded-full h-2 overflow-hidden">
                  <div
                    className="h-2 bg-indigo-600 rounded-full transition-all duration-300"
                    style={{
                      width: `${
                        selectedOperation.progress.percentage !== null &&
                        selectedOperation.progress.percentage !== undefined
                          ? selectedOperation.progress.percentage
                          : selectedOperation.current_state === 'Completed'
                          ? 100
                          : 20
                      }%`,
                    }}
                  />
                </div>
                <p className="text-[11px] text-slate-500 italic">
                  {selectedOperation.progress.stage}: {selectedOperation.progress.message}
                </p>
              </div>

              {/* Execution Summary or Failure */}
              {selectedOperation.result_summary && (
                <div className="p-3.5 rounded-lg bg-emerald-50/80 border border-emerald-200">
                  <span className="text-[10px] text-emerald-800 font-bold uppercase block tracking-wider">
                    Execution Outcome
                  </span>
                  <p className="font-mono text-[11px] text-emerald-900 mt-1 break-all select-all">
                    {selectedOperation.result_summary}
                  </p>
                </div>
              )}

              {selectedOperation.failure_reason && (
                <div className="p-3.5 rounded-lg bg-rose-50 border border-rose-200">
                  <span className="text-[10px] text-rose-800 font-bold uppercase block tracking-wider">
                    Failure Diagnostic
                  </span>
                  <p className="text-xs text-rose-900 mt-1 break-all">
                    {selectedOperation.failure_reason}
                  </p>
                </div>
              )}

              {/* Timestamps */}
              <div className="grid grid-cols-3 gap-2 pt-2 border-t border-slate-100 text-[11px] text-slate-500">
                <div>
                  <span className="block text-slate-400">Created At:</span>
                  <span className="font-mono text-slate-700">
                    {new Date(selectedOperation.created_at).toLocaleString()}
                  </span>
                </div>
                <div>
                  <span className="block text-slate-400">Started At:</span>
                  <span className="font-mono text-slate-700">
                    {selectedOperation.started_at
                      ? new Date(selectedOperation.started_at).toLocaleTimeString()
                      : '--'}
                  </span>
                </div>
                <div>
                  <span className="block text-slate-400">Completed At:</span>
                  <span className="font-mono text-slate-700">
                    {selectedOperation.completed_at
                      ? new Date(selectedOperation.completed_at).toLocaleTimeString()
                      : '--'}
                  </span>
                </div>
              </div>
            </div>

            <div className="px-6 py-3 border-t border-slate-100 flex items-center justify-between bg-slate-50/60">
              {(selectedOperation.current_state === 'Running' ||
                selectedOperation.current_state === 'Queued') && (
                <button
                  onClick={() => {
                    cancel(selectedOperation.operation_id);
                    setSelectedOperation(null);
                  }}
                  className="px-3 py-1.5 text-xs font-semibold rounded-lg text-white bg-rose-600 hover:bg-rose-700 transition-colors"
                >
                  Cancel Operation
                </button>
              )}
              <button
                onClick={() => setSelectedOperation(null)}
                className="ml-auto px-4 py-1.5 text-xs font-medium rounded-lg text-slate-700 hover:bg-slate-200/60 transition-colors"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}

      {/* New Integrity Operation Modal */}
      {showNewModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/40 backdrop-blur-xs">
          <div className="bg-white rounded-xl border border-slate-200 shadow-xl max-w-lg w-full overflow-hidden animate-in fade-in duration-200">
            <div className="px-6 py-4 border-b border-slate-100 flex items-center justify-between bg-slate-50/60">
              <div className="flex items-center gap-2">
                <Play className="w-4 h-4 text-indigo-600" />
                <h3 className="text-sm font-bold text-slate-900">
                  Launch Managed Integrity Operation
                </h3>
              </div>
              <button
                onClick={() => setShowNewModal(false)}
                className="text-slate-400 hover:text-slate-600 p-1"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <form onSubmit={handleCreateSubmit} className="p-6 space-y-4">
              {modalError && (
                <div className="p-3 rounded-lg bg-rose-50 border border-rose-200 text-xs text-rose-700">
                  {modalError}
                </div>
              )}

              {/* Mode Toggle */}
              <div className="space-y-1">
                <label className="block text-xs font-semibold text-slate-700 uppercase tracking-wider">
                  Operation Subtype
                </label>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    type="button"
                    onClick={() => setModalMode('hash')}
                    className={`px-3 py-2 text-xs font-medium rounded-lg border text-center transition-colors ${
                      modalMode === 'hash'
                        ? 'bg-indigo-50 text-indigo-700 border-indigo-300 font-semibold'
                        : 'bg-white text-slate-600 border-slate-200 hover:bg-slate-50'
                    }`}
                  >
                    SHA-256 Calculation
                  </button>
                  <button
                    type="button"
                    onClick={() => setModalMode('verify')}
                    className={`px-3 py-2 text-xs font-medium rounded-lg border text-center transition-colors ${
                      modalMode === 'verify'
                        ? 'bg-teal-50 text-teal-700 border-teal-300 font-semibold'
                        : 'bg-white text-slate-600 border-slate-200 hover:bg-slate-50'
                    }`}
                  >
                    Integrity Verification
                  </button>
                </div>
              </div>

              {/* Target File Path */}
              <div className="space-y-1">
                <label className="block text-xs font-semibold text-slate-700 uppercase tracking-wider">
                  Target Evidence File Path
                </label>
                <input
                  type="text"
                  placeholder="e.g. C:\Cases\evidence_001.raw or /tmp/disk.img"
                  value={modalPath}
                  onChange={(e) => setModalPath(e.target.value)}
                  className="w-full px-3 py-2 text-xs rounded-lg border border-slate-300 font-mono focus:outline-none focus:ring-1 focus:ring-indigo-500"
                />
              </div>

              {/* Expected Digest (Verification Mode Only) */}
              {modalMode === 'verify' && (
                <div className="space-y-1">
                  <label className="block text-xs font-semibold text-slate-700 uppercase tracking-wider">
                    Expected SHA-256 Digest (64 hex characters)
                  </label>
                  <input
                    type="text"
                    placeholder="e.g. e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    value={modalExpectedDigest}
                    onChange={(e) => setModalExpectedDigest(e.target.value)}
                    className="w-full px-3 py-2 text-xs rounded-lg border border-slate-300 font-mono focus:outline-none focus:ring-1 focus:ring-indigo-500"
                  />
                </div>
              )}

              {/* Non-Destructive Notice */}
              <div className="p-3 rounded-lg bg-slate-50 border border-slate-200 text-[11px] text-slate-500 flex items-start gap-2">
                <Shield className="w-4 h-4 text-emerald-600 shrink-0 mt-0.5" />
                <span>
                  Strictly non-destructive streaming operation. Target evidence files are never modified,
                  formatted, or altered.
                </span>
              </div>

              <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-100">
                <button
                  type="button"
                  onClick={() => setShowNewModal(false)}
                  className="px-3 py-1.5 text-xs font-medium text-slate-600 hover:text-slate-800"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={submitting || !modalPath.trim()}
                  className="px-4 py-2 text-xs font-semibold rounded-lg text-white bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 transition-colors shadow-sm"
                >
                  {submitting ? 'Launching...' : 'Launch Asynchronous Operation'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};

export default OperationsPage;
