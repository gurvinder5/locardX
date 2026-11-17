import React, { useState } from 'react';
import {
  ShieldCheck,
  Plus,
  Clock,
  User,
  Hash,
} from 'lucide-react';
import {
  Button,
  Input,
  Textarea,
  Select,
  SelectItem,
  Chip,
} from '@heroui/react';
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
    <div className="bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] p-4 space-y-4 rounded-xs">
      <div className="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-[#26313D]">
        <div className="flex items-center gap-2">
          <ShieldCheck className="w-4 h-4 text-slate-700 dark:text-slate-300" />
          <h3 className="text-xs font-semibold text-slate-800 dark:text-slate-200 uppercase tracking-wider">
            Chain of Custody & Audit Timeline
          </h3>
          <Chip
            size="sm"
            variant="flat"
            className="text-[10px] font-mono bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700"
          >
            Hash-Chain Verified
          </Chip>
        </div>
        <Button
          size="sm"
          variant="light"
          onPress={() => setShowForm(!showForm)}
          startContent={<Plus className="w-3.5 h-3.5" />}
          className="text-xs font-semibold text-slate-700 dark:text-slate-300 h-8"
        >
          {showForm ? 'Cancel' : 'Record Custody Action'}
        </Button>
      </div>

      <div className="space-y-4">
        {/* Record Custody Form */}
        {showForm && (
          <form onSubmit={handleSubmit} className="p-4 bg-slate-50 dark:bg-[#10161D] border border-slate-200 dark:border-[#26313D] rounded-lg space-y-3.5">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <Select
                size="sm"
                variant="bordered"
                label="Event Category *"
                selectedKeys={[eventType]}
                onChange={(e) => setEventType(e.target.value as CustodyEventType)}
              >
                <SelectItem key="evidence_verified">Evidence Verified (Physical / Cryptographic)</SelectItem>
                <SelectItem key="evidence_introduced">Evidence Introduced (Seized / Bagged)</SelectItem>
                <SelectItem key="evidence_acquired">Evidence Acquired (Bitstream Imaged)</SelectItem>
                <SelectItem key="evidence_analyzed">Evidence Analyzed (Carved / Reconstructed)</SelectItem>
                <SelectItem key="evidence_exported">Evidence Exported (Evidential Output)</SelectItem>
                <SelectItem key="report_generated">Report Generated</SelectItem>
                <SelectItem key="case_status_changed">Case Status Changed</SelectItem>
              </Select>

              <Input
                size="sm"
                variant="bordered"
                label="Action Summary *"
                placeholder="e.g. Transferred to secondary forensic lab"
                value={action}
                onValueChange={setAction}
                isRequired
              />
            </div>

            <Textarea
              size="sm"
              variant="bordered"
              label="Detailed Narrative & Location *"
              placeholder="Provide exact evidential disposition, location, and reason..."
              minRows={2}
              value={details}
              onValueChange={setDetails}
              isRequired
            />

            <div className="flex justify-end">
              <Button
                size="sm"
                color="primary"
                type="submit"
                isLoading={recording}
                isDisabled={recording || !action.trim() || !details.trim()}
                startContent={!recording && <ShieldCheck className="w-3.5 h-3.5" />}
                className="bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold h-8"
              >
                {recording ? 'Recording...' : 'Commit to Audit Chain'}
              </Button>
            </div>
          </form>
        )}

        {/* Timeline Stream */}
        {timeline.length === 0 ? (
          <div className="py-8 text-center text-xs text-slate-400">
            No custody or audit events recorded for this case.
          </div>
        ) : (
          <div className="relative pl-6 space-y-4 before:absolute before:bottom-0 before:top-2 before:left-2 before:w-0.5 before:bg-slate-200 dark:before:bg-[#26313D]">
            {timeline.map((item) => (
              <div key={item.id} className="relative group">
                {/* Dot */}
                <div className="absolute -left-6 top-2 w-3.5 h-3.5 rounded-full border-2 border-white dark:border-[#151C24] bg-slate-700 dark:bg-slate-300 ring-2 ring-slate-200 dark:ring-[#26313D]" />

                <div className="bg-slate-50 dark:bg-[#10161D] border border-slate-200 dark:border-[#26313D] rounded-lg p-3 space-y-1.5 group-hover:border-slate-300 dark:group-hover:border-slate-600 transition-colors">
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-bold text-slate-900 dark:text-white">{item.title}</span>
                      <Chip
                        size="sm"
                        variant="flat"
                        className={`text-[9px] uppercase font-bold tracking-wider h-4 ${
                          item.item_type === 'Custody'
                            ? 'bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700'
                            : 'bg-slate-50 dark:bg-[#151C24] text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-slate-800'
                        }`}
                      >
                        {item.item_type}
                      </Chip>
                    </div>
                    <div className="flex items-center gap-1 text-[10px] text-slate-400 font-mono">
                      <Clock className="w-2.5 h-2.5" />
                      <span>{new Date(item.timestamp).toLocaleString()}</span>
                    </div>
                  </div>

                  <p className="text-xs text-slate-600 dark:text-slate-400 leading-relaxed">{item.details}</p>

                  <div className="pt-1.5 border-t border-slate-200/70 dark:border-[#26313D] flex flex-col sm:flex-row sm:items-center justify-between gap-1 text-[10px] text-slate-400 font-mono">
                    <span className="flex items-center gap-1">
                      <User className="w-2.5 h-2.5 text-slate-400" />
                      <span>Actor: <strong className="text-slate-700 dark:text-slate-300">{item.actor_id}</strong></span>
                    </span>
                    {item.audit_hash && (
                      <span className="flex items-center gap-1 font-mono text-slate-500">
                        <Hash className="w-2.5 h-2.5 text-slate-700 dark:text-slate-300" />
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
    </div>
  );
};
