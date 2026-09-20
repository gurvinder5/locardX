import React, { useState } from 'react';
import {
  ShieldCheck,
  Plus,
  Clock,
  User,
  Hash,
} from 'lucide-react';
import { CaseTimelineItem, CustodyEventType, RecordCustodyRequest } from '../../types/case';

interface CaseCustodyTimelineProps {
  timeline: CaseTimelineItem[];
  onRecordCustody: (request: RecordCustodyRequest) => Promise<void>;
}

export const CaseCustodyTimeline: React.FC<CaseCustodyTimelineProps> = ({
  timeline,
  onRecordCustody,
}) => {
  const [showForm, setShowForm] = useState(false);
  const [eventType, setEventType] = useState<CustodyEventType>('evidence_verified');
  const [action, setAction] = useState('');
  const [details, setDetails] = useState('');
  const [recording, setRecording] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!action.trim() || !details.trim()) return;
    try {
      setRecording(true);
      await onRecordCustody({
        event_type: eventType,
        action: action.trim(),
        details: details.trim(),
      });
      setAction('');
      setDetails('');
      setShowForm(false);
    } catch (err) {
      console.error('Failed to record custody event:', err);
    } finally {
      setRecording(false);
    }
  };

  return (
    <div className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <ShieldCheck className="w-4 h-4 text-emerald-600" />
          <h3 className="text-xs font-semibold text-slate-800 uppercase tracking-wider">
            Chain of Custody & Audit Timeline
          </h3>
          <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-emerald-50 text-emerald-700 border border-emerald-200">
            Hash-Chain Verified
          </span>
        </div>
        <button
          onClick={() => setShowForm(!showForm)}
          className="inline-flex items-center gap-1 text-xs font-medium text-emerald-600 hover:text-emerald-800 transition-colors"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>{showForm ? 'Cancel' : 'Record Custody Action'}</span>
        </button>
      </div>

      {/* Record Custody Form */}
      {showForm && (
        <form onSubmit={handleSubmit} className="p-3 bg-slate-50 border border-slate-200 rounded-md space-y-3">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div>
              <label className="block text-[11px] font-medium text-slate-700 mb-1">
                Event Category *
              </label>
              <select
                value={eventType}
                onChange={(e) => setEventType(e.target.value as CustodyEventType)}
                className="w-full text-xs px-2.5 py-1.5 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-emerald-500"
              >
                <option value="evidence_verified">Evidence Verified (Physical / Cryptographic)</option>
                <option value="evidence_introduced">Evidence Introduced (Seized / Bagged)</option>
                <option value="evidence_acquired">Evidence Acquired (Bitstream Imaged)</option>
                <option value="evidence_analyzed">Evidence Analyzed (Carved / Reconstructed)</option>
                <option value="evidence_exported">Evidence Exported (Evidential Output)</option>
                <option value="report_generated">Report Generated</option>
                <option value="case_status_changed">Case Status Changed</option>
              </select>
            </div>
            <div>
              <label className="block text-[11px] font-medium text-slate-700 mb-1">
                Action Summary *
              </label>
              <input
                type="text"
                placeholder="e.g. Transferred to secondary forensic lab"
                value={action}
                onChange={(e) => setAction(e.target.value)}
                required
                className="w-full text-xs px-2.5 py-1.5 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-emerald-500"
              />
            </div>
          </div>
          <div>
            <label className="block text-[11px] font-medium text-slate-700 mb-1">
              Detailed Narrative & Location *
            </label>
            <textarea
              rows={2}
              placeholder="Provide exact evidential disposition, location, and reason..."
              value={details}
              onChange={(e) => setDetails(e.target.value)}
              required
              className="w-full text-xs px-2.5 py-1.5 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-emerald-500"
            />
          </div>
          <div className="flex justify-end">
            <button
              type="submit"
              disabled={recording || !action.trim() || !details.trim()}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded bg-emerald-600 text-white hover:bg-emerald-700 disabled:opacity-50 transition-colors"
            >
              <ShieldCheck className="w-3 h-3" />
              <span>{recording ? 'Recording...' : 'Commit to Audit Chain'}</span>
            </button>
          </div>
        </form>
      )}

      {/* Timeline Stream */}
      {timeline.length === 0 ? (
        <div className="py-8 text-center text-xs text-slate-400">
          No custody or audit events recorded for this case.
        </div>
      ) : (
        <div className="relative pl-6 space-y-6 before:absolute before:bottom-0 before:top-2 before:left-2 before:w-0.5 before:bg-slate-200">
          {timeline.map((item) => (
            <div key={item.id} className="relative group">
              {/* Dot */}
              <div
                className={`absolute -left-6 top-1 w-3.5 h-3.5 rounded-full border-2 border-white ${
                  item.item_type === 'Custody' ? 'bg-emerald-500 ring-2 ring-emerald-100' : 'bg-sky-500 ring-2 ring-sky-100'
                }`}
              />

              <div className="bg-slate-50 border border-slate-200 rounded-md p-3 space-y-1.5 group-hover:border-slate-300 transition-colors">
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2">
                    <span className="text-xs font-bold text-slate-900">{item.title}</span>
                    <span
                      className={`text-[9px] uppercase font-bold tracking-wider px-1.5 py-0.5 rounded border ${
                        item.item_type === 'Custody'
                          ? 'bg-emerald-50 text-emerald-700 border-emerald-200'
                          : 'bg-sky-50 text-sky-700 border-sky-200'
                      }`}
                    >
                      {item.item_type}
                    </span>
                  </div>
                  <div className="flex items-center gap-1 text-[10px] text-slate-400 font-mono">
                    <Clock className="w-2.5 h-2.5" />
                    <span>{new Date(item.timestamp).toLocaleString()}</span>
                  </div>
                </div>

                <p className="text-xs text-slate-600 leading-relaxed">{item.details}</p>

                <div className="pt-1.5 border-t border-slate-200/70 flex flex-col sm:flex-row sm:items-center justify-between gap-1 text-[10px] text-slate-400">
                  <span className="flex items-center gap-1">
                    <User className="w-2.5 h-2.5 text-slate-400" />
                    <span>Actor: <strong className="text-slate-700">{item.actor_id}</strong></span>
                  </span>
                  {item.audit_hash && (
                    <span className="flex items-center gap-1 font-mono text-slate-500">
                      <Hash className="w-2.5 h-2.5 text-emerald-600" />
                      <span className="truncate max-w-[200px]" title={item.audit_hash}>
                        Digest: {item.audit_hash.slice(0, 16)}...
                      </span>
                    </span>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
