import React, { useState } from 'react';
import { HardDrive, Plus, Tag, Hash, FileCode } from 'lucide-react';
import {
  Button,
  Input,
  Select,
  SelectItem,
  Chip,
} from '@heroui/react';
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
        return 'bg-amber-50 dark:bg-amber-950/40 text-amber-700 dark:text-amber-300 border-amber-300 dark:border-amber-700';
      case 'acquisition_image':
      case 'recovered_dataset':
        return 'bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border-slate-300 dark:border-slate-700';
      default:
        return 'bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 border-slate-200 dark:border-slate-800';
    }
  };

  return (
    <div className="bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] p-4 space-y-4 rounded-xs">
      <div className="flex items-center justify-between border-b border-slate-100 dark:border-[#26313D] pb-3">
        <div className="flex items-center gap-2">
          <HardDrive className="w-4 h-4 text-slate-700 dark:text-slate-300" />
          <h3 className="text-xs font-semibold text-slate-800 dark:text-slate-200 uppercase tracking-wider">
            Case Evidence Inventory
          </h3>
          <Chip
            size="sm"
            variant="flat"
            className="text-[10px] h-5 font-mono bg-slate-100 dark:bg-[#1B2530] text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]"
          >
            {evidence.length}
          </Chip>
        </div>
        <Button
          size="sm"
          variant="light"
          onPress={() => setShowForm(!showForm)}
          startContent={<Plus className="w-3.5 h-3.5" />}
          className="text-xs font-semibold text-slate-700 dark:text-slate-300 h-8"
        >
          {showForm ? 'Cancel' : 'Introduce Evidence'}
        </Button>
      </div>

      <div className="space-y-4">
        {/* Add Evidence Form */}
        {showForm && (
          <form onSubmit={handleSubmit} className="p-4 bg-slate-50 dark:bg-[#10161D] border border-slate-200 dark:border-[#26313D] rounded-lg space-y-3.5">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              <Input
                size="sm"
                variant="bordered"
                label="Label / Tag *"
                placeholder="e.g. Seized Flash Drive"
                value={label}
                onValueChange={setLabel}
                isRequired
              />

              <Select
                size="sm"
                variant="bordered"
                label="Evidence Type *"
                selectedKeys={[evType]}
                onChange={(e) => setEvType(e.target.value)}
              >
                <SelectItem key="physical_storage">Physical Storage Device</SelectItem>
                <SelectItem key="acquisition_image">Forensic Bitstream Image (.dd/.raw)</SelectItem>
                <SelectItem key="recovered_dataset">Recovered Files Dataset</SelectItem>
                <SelectItem key="logical_file">Logical File Artifact</SelectItem>
              </Select>

              <Input
                size="sm"
                variant="bordered"
                label="Device ID / Path *"
                placeholder="\\\\.\\PhysicalDrive1 or image.dd"
                value={identifier}
                onValueChange={setIdentifier}
                isRequired
              />
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <Input
                size="sm"
                variant="bordered"
                label="SHA-256 Digest (Optional)"
                placeholder="64-character hex digest"
                value={sha256}
                onValueChange={setSha256}
                classNames={{
                  input: 'font-mono text-xs',
                }}
              />

              <Input
                size="sm"
                variant="bordered"
                type="number"
                label="Size in Bytes (Optional)"
                placeholder="e.g. 1000204886016"
                value={sizeBytes}
                onValueChange={setSizeBytes}
              />
            </div>

            <div className="flex justify-end">
              <Button
                size="sm"
                color="primary"
                type="submit"
                isLoading={adding}
                isDisabled={adding || !label.trim() || !identifier.trim()}
                startContent={!adding && <Plus className="w-3 h-3" />}
                className="bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold h-8"
              >
                {adding ? 'Registering...' : 'Register Evidence'}
              </Button>
            </div>
          </form>
        )}

        {/* Evidence Grid */}
        {evidence.length === 0 ? (
          <div className="py-8 text-center text-xs text-slate-400">
            No evidence items logged under this case. Introduce physical media or link an acquisition image above.
          </div>
        ) : (
          <div className="grid grid-cols-1 xl:grid-cols-2 gap-3 min-w-0">
            {evidence.map((ev) => (
              <div
                key={ev.evidence_id}
                className="p-3.5 bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] rounded-xs shadow-2xs space-y-2 hover:border-slate-300 dark:hover:border-slate-600 transition-colors min-w-0 overflow-hidden"
              >
                <div className="flex items-start justify-between gap-2 min-w-0">
                  <div className="min-w-0 flex-1">
                    <span
                      className={`text-[9px] uppercase font-bold tracking-wider px-1.5 py-0.5 rounded border inline-block ${getBadge(
                        ev.evidence_type
                      )}`}
                    >
                      {ev.evidence_type.replace('_', ' ')}
                    </span>
                    <h4 className="text-xs font-semibold text-slate-900 dark:text-white mt-1 truncate" title={ev.label}>
                      {ev.label}
                    </h4>
                  </div>
                  <Chip
                    size="sm"
                    variant="flat"
                    className="text-[10px] font-mono bg-slate-50 dark:bg-[#1B2530] text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-[#26313D] shrink-0 max-w-[130px] truncate"
                    title={ev.evidence_id}
                  >
                    {ev.evidence_id}
                  </Chip>
                </div>

                <div className="space-y-1 text-[11px] text-slate-600 dark:text-slate-400 min-w-0">
                  <div className="flex items-center gap-1.5 min-w-0">
                    <Tag className="w-3 h-3 text-slate-400 shrink-0" />
                    <span className="font-mono text-slate-700 dark:text-slate-300 truncate" title={ev.identifier}>
                      {ev.identifier}
                    </span>
                  </div>
                  {ev.size_bytes && (
                    <div className="flex items-center gap-1.5 min-w-0">
                      <FileCode className="w-3 h-3 text-slate-400 shrink-0" />
                      <span className="truncate">
                        Size: <strong>{(ev.size_bytes / (1024 * 1024 * 1024)).toFixed(2)} GB</strong> ({ev.size_bytes.toLocaleString()} bytes)
                      </span>
                    </div>
                  )}
                  {ev.sha256 && (
                    <div className="flex items-start gap-1.5 pt-1 min-w-0">
                      <Hash className="w-3 h-3 text-slate-700 dark:text-slate-300 shrink-0 mt-0.5" />
                      <span className="font-mono text-[10px] text-slate-600 dark:text-slate-400 break-all bg-slate-50 dark:bg-[#10161D] p-1 rounded-xs border border-slate-200 dark:border-[#26313D] block min-w-0 flex-1">
                        {ev.sha256}
                      </span>
                    </div>
                  )}
                </div>

                <div className="pt-2 border-t border-slate-100 dark:border-[#26313D] flex items-center justify-between text-[10px] text-slate-400 font-mono">
                  <span className="truncate mr-2">Introduced by: {ev.introduced_by}</span>
                  <span className="shrink-0">{new Date(ev.introduced_at).toLocaleDateString()}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
