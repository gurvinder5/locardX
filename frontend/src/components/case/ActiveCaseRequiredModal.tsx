import React, { useState, useEffect } from 'react';
import { AlertTriangle, FolderPlus, CheckCircle2, X } from 'lucide-react';
import { useCaseStore } from '../../stores/caseStore';
import { useAuthStore } from '../../stores/authStore';
import { CreateCaseModal } from './CreateCaseModal';

interface ActiveCaseRequiredModalProps {
  isOpen: boolean;
  onClose?: () => void;
  operationName?: string;
  onCaseActivated?: () => void;
}

export const ActiveCaseRequiredModal: React.FC<ActiveCaseRequiredModalProps> = ({
  isOpen,
  onClose,
  operationName = 'Forensic Operation',
  onCaseActivated,
}) => {
  const { activeCase, cases, loadCases, setActiveCase, createCase, isLoading } = useCaseStore();
  const { sessionToken } = useAuthStore();
  const [selectedCaseId, setSelectedCaseId] = useState<string>('');
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadCases();
    }
  }, [isOpen, loadCases]);

  const openCases = cases.filter(
    (c) => c.status === 'open' || c.status === 'in_progress'
  );

  useEffect(() => {
    if (openCases.length > 0 && !selectedCaseId) {
      setSelectedCaseId(openCases[0].case_id);
    }
  }, [openCases, selectedCaseId]);

  if (!isOpen) return null;

  const handleSelectCase = async () => {
    if (!selectedCaseId) {
      setError('Please select a valid open case.');
      return;
    }
    try {
      setError(null);
      await setActiveCase(selectedCaseId);
      if (onCaseActivated) onCaseActivated();
      if (onClose) onClose();
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Failed to set active case';
      setError(msg);
    }
  };

  const isCaseClosed = activeCase && (activeCase.status === 'completed' || activeCase.status === 'archived');

  return (
    <>
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/60 backdrop-blur-xs p-4">
        <div className="bg-white rounded-xl shadow-2xl border border-amber-200 w-full max-w-lg overflow-hidden animate-in fade-in duration-200">
          {/* Header */}
          <div className="p-4 border-b border-amber-100 flex items-center justify-between bg-amber-50/70">
            <div className="flex items-center gap-2.5">
              <div className="p-2 rounded-lg bg-amber-100 text-amber-700 border border-amber-200">
                <AlertTriangle className="w-5 h-5" />
              </div>
              <div>
                <h3 className="text-sm font-bold text-slate-900">
                  {isCaseClosed ? 'Active Case is Closed / Archived' : 'Active Forensic Case Required'}
                </h3>
                <p className="text-xs text-slate-500">
                  Chain of Custody & Forensic Policy Enforcement
                </p>
              </div>
            </div>
            {onClose && (
              <button
                onClick={onClose}
                className="text-slate-400 hover:text-slate-600 p-1 rounded hover:bg-amber-100/50 transition-colors"
              >
                <X className="w-4 h-4" />
              </button>
            )}
          </div>

          {/* Body */}
          <div className="p-6 space-y-5">
            <div className="p-3.5 bg-slate-50 rounded-lg border border-slate-200 text-xs text-slate-700 space-y-1.5">
              <p className="font-semibold text-slate-800">
                {operationName} requires an active, open case context.
              </p>
              <p className="text-slate-600 leading-relaxed">
                To guarantee forensic defensibility, all bitstream acquisitions, filesystem recoveries,
                deep carving operations, and certificates must be strictly bound to an open investigation case.
              </p>
            </div>

            {error && (
              <div className="p-3 bg-rose-50 border border-rose-200 rounded-lg text-xs text-rose-700 flex items-center gap-2">
                <AlertTriangle className="w-4 h-4 shrink-0 text-rose-500" />
                <span>{error}</span>
              </div>
            )}

            {/* Select existing open case */}
            <div className="space-y-2">
              <label className="block text-xs font-semibold text-slate-700">
                Select an Existing Open Case
              </label>
              {openCases.length > 0 ? (
                <div className="flex gap-2">
                  <select
                    value={selectedCaseId}
                    onChange={(e) => setSelectedCaseId(e.target.value)}
                    className="flex-1 text-xs border border-slate-300 rounded-lg px-3 py-2 bg-white text-slate-800 focus:outline-hidden focus:ring-2 focus:ring-sky-500 focus:border-sky-500"
                  >
                    {openCases.map((c) => (
                      <option key={c.case_id} value={c.case_id}>
                        {c.case_reference} — {c.title} ({c.status})
                      </option>
                    ))}
                  </select>
                  <button
                    type="button"
                    onClick={handleSelectCase}
                    disabled={isLoading || !selectedCaseId}
                    className="inline-flex items-center gap-1.5 px-3 py-2 text-xs font-semibold text-white bg-sky-600 hover:bg-sky-700 rounded-lg shadow-xs transition-colors disabled:opacity-50"
                  >
                    <CheckCircle2 className="w-4 h-4" />
                    Set Active
                  </button>
                </div>
              ) : (
                <p className="text-xs text-slate-500 italic">
                  No active or in-progress cases found. Please create a new case to proceed.
                </p>
              )}
            </div>

            <div className="relative flex py-1 items-center">
              <div className="grow border-t border-slate-200"></div>
              <span className="shrink mx-3 text-xs text-slate-400 uppercase font-mono">OR</span>
              <div className="grow border-t border-slate-200"></div>
            </div>

            {/* Create new case button */}
            <div>
              <button
                type="button"
                onClick={() => setShowCreateModal(true)}
                className="w-full inline-flex items-center justify-center gap-2 px-4 py-2.5 text-xs font-semibold text-sky-700 bg-sky-50 hover:bg-sky-100 border border-sky-200 rounded-lg transition-colors"
              >
                <FolderPlus className="w-4 h-4" />
                Create New Investigation Case
              </button>
            </div>
          </div>

          {/* Footer */}
          <div className="p-3.5 bg-slate-50 border-t border-slate-200 flex justify-end">
            {onClose && (
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-1.5 text-xs font-medium text-slate-600 hover:text-slate-800 hover:bg-slate-200/60 rounded-md transition-colors"
              >
                Dismiss
              </button>
            )}
          </div>
        </div>
      </div>

      {/* Embedded CreateCaseModal */}
      {showCreateModal && (
        <CreateCaseModal
          isOpen={showCreateModal}
          onClose={() => setShowCreateModal(false)}
          onSubmit={async (req) => {
            const token = sessionToken || 'dev-session-token';
            const newCase = await createCase(req, token);
            setShowCreateModal(false);
            if (onCaseActivated) onCaseActivated();
            if (onClose) onClose();
            return newCase;
          }}
        />
      )}
    </>
  );
};
