import React, { useState } from 'react';
import { X, Briefcase, Plus, AlertCircle } from 'lucide-react';
import { CreateCaseRequest } from '../../types/case';

interface CreateCaseModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (request: CreateCaseRequest) => Promise<any>;
}

export const CreateCaseModal: React.FC<CreateCaseModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
}) => {
  const [reference, setReference] = useState('');
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [priority, setPriority] = useState('Standard');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!reference.trim() || !title.trim()) {
      setError('Please provide both case reference and title.');
      return;
    }

    try {
      setSubmitting(true);
      setError(null);
      await onSubmit({
        case_reference: reference.trim(),
        title: title.trim(),
        description: description.trim(),
        metadata_json: JSON.stringify({ priority }),
      });
      onClose();
    } catch (err: any) {
      setError(err?.message || 'Failed to create case');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4">
      <div className="bg-white rounded-lg shadow-xl border border-slate-200 w-full max-w-lg overflow-hidden animate-in fade-in duration-200">
        <div className="p-4 border-b border-slate-200 flex items-center justify-between bg-slate-50/50">
          <div className="flex items-center gap-2">
            <div className="p-1.5 rounded bg-sky-50 text-sky-600 border border-sky-200">
              <Briefcase className="w-4 h-4" />
            </div>
            <h3 className="text-sm font-bold text-slate-900">Initiate Investigation Case</h3>
          </div>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-slate-600 p-1 rounded hover:bg-slate-100 transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-5 space-y-4">
          {error && (
            <div className="p-3 bg-rose-50 border border-rose-200 rounded text-xs text-rose-700 flex items-center gap-2">
              <AlertCircle className="w-4 h-4 shrink-0 text-rose-500" />
              <span>{error}</span>
            </div>
          )}

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs font-semibold text-slate-700 mb-1">
                Case Reference *
              </label>
              <input
                type="text"
                placeholder="e.g. INV-2026-004"
                value={reference}
                onChange={(e) => setReference(e.target.value)}
                required
                className="w-full text-xs px-3 py-2 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-sky-500 focus:ring-1 focus:ring-sky-500 font-mono"
              />
            </div>
            <div>
              <label className="block text-xs font-semibold text-slate-700 mb-1">Priority</label>
              <select
                value={priority}
                onChange={(e) => setPriority(e.target.value)}
                className="w-full text-xs px-3 py-2 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-sky-500 focus:ring-1 focus:ring-sky-500"
              >
                <option value="Urgent">Urgent / Active Incident</option>
                <option value="High">High</option>
                <option value="Standard">Standard</option>
                <option value="Low">Low / Archival</option>
              </select>
            </div>
          </div>

          <div>
            <label className="block text-xs font-semibold text-slate-700 mb-1">Case Title *</label>
            <input
              type="text"
              placeholder="e.g. Unauthorized Intrusion & Data Extraction - Server Node 3"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              required
              className="w-full text-xs px-3 py-2 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-sky-500 focus:ring-1 focus:ring-sky-500"
            />
          </div>

          <div>
            <label className="block text-xs font-semibold text-slate-700 mb-1">
              Case Description & Scope
            </label>
            <textarea
              rows={3}
              placeholder="Provide context, evidence scope, authority/warrant number, and targets..."
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className="w-full text-xs px-3 py-2 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-sky-500 focus:ring-1 focus:ring-sky-500"
            />
          </div>

          <div className="pt-3 border-t border-slate-100 flex items-center justify-end gap-2">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 text-xs font-medium rounded border border-slate-300 text-slate-700 hover:bg-slate-50 transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={submitting || !reference.trim() || !title.trim()}
              className="inline-flex items-center gap-1.5 px-4 py-1.5 text-xs font-medium rounded bg-sky-600 text-white hover:bg-sky-700 disabled:opacity-50 transition-colors shadow-2xs"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>{submitting ? 'Creating...' : 'Create Case'}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
