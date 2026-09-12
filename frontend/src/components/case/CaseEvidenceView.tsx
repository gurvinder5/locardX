import React, { useState } from 'react';
import { HardDrive, Plus, Tag, Hash, FileCode } from 'lucide-react';
import { CaseEvidence, EvidenceType } from '../../types/case';

interface CaseEvidenceViewProps {
  evidence: CaseEvidence[];
  onAddEvidence: (request: {
    evidence_type: string;
    identifier: string;
    label: string;
    sha256?: string | null;
    size_bytes?: number | null;
    notes?: string | null;
  }) => Promise<void>;
}

export const CaseEvidenceView: React.FC<CaseEvidenceViewProps> = ({
  evidence,
  onAddEvidence,
}) => {
  const [showForm, setShowForm] = useState(false);
  const [label, setLabel] = useState('');
  const [evType, setEvType] = useState<string>('physical_storage');
  const [identifier, setIdentifier] = useState('');
  const [sha256, setSha256] = useState('');
  const [sizeBytes, setSizeBytes] = useState('');
  const [notes, setNotes] = useState('');
  const [adding, setAdding] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!label.trim() || !identifier.trim()) return;
    try {
      setAdding(true);
      await onAddEvidence({
        label: label.trim(),
        evidence_type: evType,
        identifier: identifier.trim(),
        sha256: sha256.trim() || null,
        size_bytes: sizeBytes ? parseInt(sizeBytes, 10) : null,
        notes: notes.trim() || null,
      });
      setLabel('');
      setIdentifier('');
      setSha256('');
      setSizeBytes('');
      setNotes('');
      setShowForm(false);
    } catch (err) {
      console.error('Failed to add evidence:', err);
    } finally {
      setAdding(false);
    }
  };

  const getBadge = (t: EvidenceType) => {
    switch (t) {
      case 'physical_storage':
        return 'bg-amber-50 text-amber-700 border-amber-200';
      case 'acquisition_image':
        return 'bg-sky-50 text-sky-700 border-sky-200';
      case 'recovered_dataset':
        return 'bg-emerald-50 text-emerald-700 border-emerald-200';
      default:
        return 'bg-slate-100 text-slate-700 border-slate-200';
    }
  };

  return (
    <div className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <HardDrive className="w-4 h-4 text-sky-600" />
          <h3 className="text-xs font-semibold text-slate-800 uppercase tracking-wider">
            Case Evidence Inventory
          </h3>
          <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-slate-100 text-slate-600 border border-slate-200">
            {evidence.length}
          </span>
        </div>
        <button
          onClick={() => setShowForm(!showForm)}
          className="inline-flex items-center gap-1 text-xs font-medium text-sky-600 hover:text-sky-800 transition-colors"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>{showForm ? 'Cancel' : 'Introduce Evidence'}</span>
        </button>
      </div>

      {/* Add Evidence Form */}
      {showForm && (
        <form onSubmit={handleSubmit} className="p-3 bg-slate-50 border border-slate-200 rounded-md space-y-3">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            <div>
              <label className="block text-[11px] font-medium text-slate-700 mb-1">
                Label / Tag *
              </label>
              <input
                type="text"
                placeholder="e.g. Seized Kingston Flash Drive"
                value={label}
                onChange={(e) => setLabel(e.target.value)}
                required
                className="w-full text-xs px-2.5 py-1.5 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-sky-500"
              />
            </div>
            <div>
              <label className="block text-[11px] font-medium text-slate-700 mb-1">
                Evidence Type *
              </label>
              <select
                value={evType}
                onChange={(e) => setEvType(e.target.value)}
                className="w-full text-xs px-2.5 py-1.5 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-sky-500"
              >
                <option value="physical_storage">Physical Storage Device</option>
                <option value="acquisition_image">Forensic Bitstream Image (.dd/.raw)</option>
                <option value="recovered_dataset">Recovered Files Dataset</option>
                <option value="logical_file">Logical File Artifact</option>
              </select>
            </div>
            <div>
              <label className="block text-[11px] font-medium text-slate-700 mb-1">
                Device ID / Path *
              </label>
              <input
                type="text"
                placeholder="\\\\.\\PhysicalDrive1 or /evidence/image.raw"
                value={identifier}
                onChange={(e) => setIdentifier(e.target.value)}
                required
                className="w-full text-xs px-2.5 py-1.5 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-sky-500"
              />
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div>
              <label className="block text-[11px] font-medium text-slate-700 mb-1">
                SHA-256 Digest (Optional)
              </label>
              <input
                type="text"
                placeholder="64-character hex digest"
                value={sha256}
                onChange={(e) => setSha256(e.target.value)}
                className="w-full text-xs px-2.5 py-1.5 font-mono rounded border border-slate-300 bg-white focus:outline-hidden focus:border-sky-500"
              />
            </div>
            <div>
              <label className="block text-[11px] font-medium text-slate-700 mb-1">
                Size in Bytes (Optional)
              </label>
              <input
                type="number"
                placeholder="e.g. 1000204886016"
                value={sizeBytes}
                onChange={(e) => setSizeBytes(e.target.value)}
                className="w-full text-xs px-2.5 py-1.5 rounded border border-slate-300 bg-white focus:outline-hidden focus:border-sky-500"
              />
            </div>
          </div>

          <div className="flex justify-end">
            <button
              type="submit"
              disabled={adding || !label.trim() || !identifier.trim()}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded bg-sky-600 text-white hover:bg-sky-700 disabled:opacity-50 transition-colors"
            >
              <Plus className="w-3 h-3" />
              <span>{adding ? 'Registering...' : 'Register Evidence'}</span>
            </button>
          </div>
        </form>
      )}

      {/* Evidence Grid */}
      {evidence.length === 0 ? (
        <div className="py-8 text-center text-xs text-slate-400">
          No evidence items logged under this case. Introduce physical media or link an acquisition image above.
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {evidence.map((ev) => (
            <div
              key={ev.evidence_id}
              className="p-3.5 bg-white border border-slate-200 rounded-md shadow-2xs space-y-2 hover:border-slate-300 transition-colors"
            >
              <div className="flex items-start justify-between gap-2">
                <div>
                  <span
                    className={`text-[9px] uppercase font-bold tracking-wider px-1.5 py-0.5 rounded border ${getBadge(
                      ev.evidence_type
                    )}`}
                  >
                    {ev.evidence_type.replace('_', ' ')}
                  </span>
                  <h4 className="text-xs font-semibold text-slate-900 mt-1">{ev.label}</h4>
                </div>
                <span className="text-[10px] font-mono text-slate-400 bg-slate-50 px-1.5 py-0.5 rounded border border-slate-200">
                  {ev.evidence_id}
                </span>
              </div>

              <div className="space-y-1 text-[11px] text-slate-600">
                <div className="flex items-center gap-1.5">
                  <Tag className="w-3 h-3 text-slate-400" />
                  <span className="font-mono text-slate-700 truncate">{ev.identifier}</span>
                </div>
                {ev.size_bytes && (
                  <div className="flex items-center gap-1.5">
                    <FileCode className="w-3 h-3 text-slate-400" />
                    <span>
                      Size: <strong>{(ev.size_bytes / (1024 * 1024 * 1024)).toFixed(2)} GB</strong> ({ev.size_bytes.toLocaleString()} bytes)
                    </span>
                  </div>
                )}
                {ev.sha256 && (
                  <div className="flex items-start gap-1.5 pt-1">
                    <Hash className="w-3 h-3 text-emerald-600 shrink-0 mt-0.5" />
                    <span className="font-mono text-[10px] text-slate-600 break-all bg-slate-50 p-1 rounded border border-slate-100">
                      {ev.sha256}
                    </span>
                  </div>
                )}
              </div>

              <div className="pt-2 border-t border-slate-100 flex items-center justify-between text-[10px] text-slate-400">
                <span>Introduced by: {ev.introduced_by}</span>
                <span>{new Date(ev.introduced_at).toLocaleDateString()}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
