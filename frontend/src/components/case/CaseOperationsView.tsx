import React, { useState } from 'react';
import { Activity, Plus, Link, User, CheckCircle2 } from 'lucide-react';
import {
  Button,
  Input,
  Select,
  SelectItem,
  Chip,
  Table,
  TableHeader,
  TableColumn,
  TableBody,
  TableRow,
  TableCell,
} from '@heroui/react';
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
    <div className="bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] p-4 space-y-4 rounded-xs overflow-hidden w-full min-w-0">
      <div className="flex items-center justify-between border-b border-slate-100 dark:border-[#26313D] pb-3">
        <div className="flex items-center gap-2">
          <Activity className="w-4 h-4 text-slate-700 dark:text-slate-300" />
          <h3 className="text-xs font-semibold text-slate-800 dark:text-slate-200 uppercase tracking-wider">
            Associated Case Operations
          </h3>
          <Chip
            size="sm"
            variant="flat"
            className="text-[10px] h-5 font-mono bg-slate-100 dark:bg-[#1B2530] text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]"
          >
            {operations.length}
          </Chip>
        </div>
        <Button
          size="sm"
          variant="light"
          onPress={() => setShowForm(!showForm)}
          startContent={<Plus className="w-3.5 h-3.5" />}
          className="text-xs font-semibold text-slate-700 dark:text-slate-300 h-8"
        >
          {showForm ? 'Cancel' : 'Link Operation'}
        </Button>
      </div>

      <div className="space-y-4">
        {/* Link Operation Form */}
        {showForm && (
          <form onSubmit={handleSubmit} className="p-4 bg-slate-50 dark:bg-[#10161D] border border-slate-200 dark:border-[#26313D] rounded-lg space-y-3.5">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              <Input
                size="sm"
                variant="bordered"
                label="Operation ID *"
                placeholder="e.g. op-acq-1234 or op-rec-5678"
                value={opId}
                onValueChange={setOpId}
                isRequired
                classNames={{
                  input: 'font-mono text-xs',
                }}
              />

              <Select
                size="sm"
                variant="bordered"
                label="Operation Type *"
                selectedKeys={[opType]}
                onChange={(e) => setOpType(e.target.value)}
              >
                <SelectItem key="ForensicAcquisition">Forensic Acquisition (DD Imaging)</SelectItem>
                <SelectItem key="Recovery">Forensic File Recovery (Carving)</SelectItem>
                <SelectItem key="DriveErasure">Secure Drive Erasure</SelectItem>
                <SelectItem key="FileErasure">Secure File/Folder Erasure</SelectItem>
                <SelectItem key="IntegrityHash">Integrity Hash Verification</SelectItem>
              </Select>

              <Input
                size="sm"
                variant="bordered"
                label="Notes (Optional)"
                placeholder="Context, reason or tag"
                value={notes}
                onValueChange={setNotes}
              />
            </div>

            <div className="flex justify-end">
              <Button
                size="sm"
                color="primary"
                type="submit"
                isLoading={linking}
                isDisabled={linking || !opId.trim()}
                startContent={!linking && <Link className="w-3.5 h-3.5" />}
                className="bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold h-8"
              >
                {linking ? 'Linking...' : 'Confirm Association'}
              </Button>
            </div>
          </form>
        )}

        {/* Operations Table */}
        {operations.length === 0 ? (
          <div className="py-8 text-center text-xs text-slate-400">
            No operations associated with this case yet. Link an acquisition, recovery, or erasure operation above.
          </div>
        ) : (
          <div className="w-full overflow-x-auto scrollbar-thin min-w-0">
            <Table
              aria-label="Associated Case Operations"
              removeWrapper
              classNames={{
                base: 'w-full min-w-0 overflow-hidden',
                table: 'w-full min-w-[580px] table-fixed',
                th: 'bg-slate-50 dark:bg-[#10161D] text-slate-600 dark:text-slate-400 text-[11px] font-semibold border-b border-slate-200 dark:border-[#26313D] py-2 px-3',
                td: 'py-2.5 px-3 text-xs border-b border-slate-100 dark:border-[#1E2732] align-top',
              }}
            >
              <TableHeader>
                <TableColumn className="w-[22%]">OPERATION TYPE</TableColumn>
                <TableColumn className="w-[20%]">OPERATION ID</TableColumn>
                <TableColumn className="w-[14%]">ASSOCIATED BY</TableColumn>
                <TableColumn className="w-[18%]">TIMESTAMP</TableColumn>
                <TableColumn className="w-[26%]">NOTES</TableColumn>
              </TableHeader>
              <TableBody>
                {operations.map((op) => (
                  <TableRow key={op.id} className="hover:bg-slate-50/60 dark:hover:bg-[#1B2530]/50 transition-colors">
                    <TableCell className="w-[22%]">
                      <div className="flex items-center gap-1.5 font-medium text-slate-900 dark:text-white truncate">
                        <CheckCircle2 className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300 shrink-0" />
                        <span className="truncate" title={op.operation_type}>{op.operation_type}</span>
                      </div>
                    </TableCell>
                    <TableCell className="w-[20%]">
                      <span className="font-mono text-slate-600 dark:text-slate-400 text-xs block truncate" title={op.operation_id}>
                        {op.operation_id}
                      </span>
                    </TableCell>
                    <TableCell className="w-[14%]">
                      <span className="inline-flex items-center gap-1 text-[11px] text-slate-600 dark:text-slate-400 truncate">
                        <User className="w-3 h-3 text-slate-400 shrink-0" />
                        <span className="truncate">{op.associated_by}</span>
                      </span>
                    </TableCell>
                    <TableCell className="w-[18%]">
                      <span className="font-mono text-[10px] text-slate-500 dark:text-slate-400 block truncate" title={new Date(op.associated_at).toLocaleString()}>
                        {new Date(op.associated_at).toLocaleString()}
                      </span>
                    </TableCell>
                    <TableCell className="w-[26%]">
                      <div className="text-slate-600 dark:text-slate-400 text-[11px] break-all whitespace-normal leading-relaxed" title={op.notes || undefined}>
                        {op.notes || '—'}
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </div>
    </div>
  );
};
