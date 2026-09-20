import React, { useState } from 'react';
import { Activity, Plus, Link, User, CheckCircle2 } from 'lucide-react';
import { CaseOperation } from '../../types/case';

interface CaseOperationsViewProps {
  operations: CaseOperation[];
  onLinkOperation: (operationId: string, operationType: string, notes?: string) => Promise<void>;
}

export const CaseOperationsView: React.FC<CaseOperationsViewProps> = ({
  operations,
  onLinkOperation,
}) => {
  const [showForm, setShowForm] = useState(false);
  const [opId, setOpId] = useState('');
  const [opType, setOpType] = useState('ForensicAcquisition');
  const [notes, setNotes] = useState('');
  const [linking, setLinking] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!opId.trim()) return;
    try {
      setLinking(true);
      await onLinkOperation(opId.trim(), opType, notes.trim() || undefined);
      setOpId('');
      setNotes('');
      setShowForm(false);
    } catch (err) {
      console.error('Failed to link operation:', err);
    } finally {
      setLinking(false);
    }
  };

  return (
    <div className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Activity className="w-4 h-4 text-indigo-600" />
          <h3 className="text-xs font-semibold text-slate-800 uppercase tracking-wider">
            Associated Case Operations
          </h3>
          <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-slate-100 text-slate-600 border border-slate-200">
            {operations.length}
          </span>
        </div>
        <button
          onClick={() => setShowForm(!showForm)}
          className="inline-flex items-center gap-1 text-xs font-medium text-indigo-600 hover:text-indigo-800 transition-colors"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>{showForm ? 'Cancel' : 'Link Operation'}</span>
        </button>
      </div>

      {/* Link Operation Form */}
      {showForm && (
        <form onSubmit={handleSubmit} className="p-3 bg-slate-50 border border-slate-200 rounded-md space-y-3">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            <div>
              <label className="block text-[11px] font-medium text-slate-700 mb-1">
                Operation ID *
              </label>
              <input
                type="text"
                placeholder="e.g. op-acq-1234 or op-rec-5678"
                value={opId}
                onChange={(e) => setOpId(e.target.value)}
                required
                className="w-full text-xs px-2.5 py-1.5 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-indigo-500"
              />
            </div>
            <div>
              <label className="block text-[11px] font-medium text-slate-700 mb-1">
                Operation Type *
              </label>
              <select
                value={opType}
                onChange={(e) => setOpType(e.target.value)}
                className="w-full text-xs px-2.5 py-1.5 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-indigo-500"
              >
                <option value="ForensicAcquisition">Forensic Acquisition (DD Imaging)</option>
                <option value="Recovery">Forensic File Recovery (Carving)</option>
                <option value="DriveErasure">Secure Drive Erasure</option>
                <option value="FileErasure">Secure File/Folder Erasure</option>
                <option value="IntegrityHash">Integrity Hash Verification</option>
              </select>
            </div>
            <div>
              <label className="block text-[11px] font-medium text-slate-700 mb-1">Notes</label>
              <input
                type="text"
                placeholder="Optional context or tag"
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                className="w-full text-xs px-2.5 py-1.5 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-indigo-500"
              />
            </div>
          </div>
          <div className="flex justify-end">
            <button
              type="submit"
              disabled={linking || !opId.trim()}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded bg-indigo-600 text-white hover:bg-indigo-700 disabled:opacity-50 transition-colors"
            >
              <Link className="w-3 h-3" />
              <span>{linking ? 'Linking...' : 'Confirm Association'}</span>
            </button>
          </div>
        </form>
      )}

      {/* Operations Table */}
      {operations.length === 0 ? (
        <div className="py-8 text-center text-xs text-slate-400">
          No operations associated with this case yet. Link an acquisition, recovery, or erasure operation above.
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-xs text-left">
            <thead className="bg-slate-50 text-slate-500 text-[11px] uppercase border-b border-slate-200">
              <tr>
                <th className="py-2 px-3">Operation Type</th>
                <th className="py-2 px-3 font-mono">Operation ID</th>
                <th className="py-2 px-3">Associated By</th>
                <th className="py-2 px-3">Timestamp</th>
                <th className="py-2 px-3">Notes</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {operations.map((op) => (
                <tr key={op.id} className="hover:bg-slate-50 transition-colors">
                  <td className="py-2.5 px-3 font-medium text-slate-800">
                    <span className="inline-flex items-center gap-1.5">
                      <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600" />
                      <span>{op.operation_type}</span>
                    </span>
                  </td>
                  <td className="py-2.5 px-3 font-mono text-slate-600">{op.operation_id}</td>
                  <td className="py-2.5 px-3 text-slate-600">
                    <span className="inline-flex items-center gap-1 text-[11px]">
                      <User className="w-3 h-3 text-slate-400" />
                      {op.associated_by}
                    </span>
                  </td>
                  <td className="py-2.5 px-3 text-slate-400 font-mono text-[11px]">
                    {new Date(op.associated_at).toLocaleString()}
                  </td>
                  <td className="py-2.5 px-3 text-slate-500 italic text-[11px]">
                    {op.notes || '—'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};
