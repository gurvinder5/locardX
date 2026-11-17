import React, { useState } from 'react';
import { Briefcase, Plus, Search, Clock, ChevronRight, PanelLeftClose } from 'lucide-react';
import {
  Button,
  Input,
  Chip,
  Select,
  SelectItem,
} from '@heroui/react';
import { Case, CaseStatus } from '../../types/case';

interface CaseListProps {
  cases: Case[];
  activeCaseId: string | null;
  onSelectCase: (caseId: string) => void;
  onOpenCreateModal: () => void;
  loading: boolean;
  onToggleCollapse?: () => void;
}

export const CaseList: React.FC<CaseListProps> = ({
  cases,
  activeCaseId,
  onSelectCase,
  onOpenCreateModal,
  loading,
  onToggleCollapse,
}) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState<string>('all');

  const filtered = cases.filter((c) => {
    const matchesSearch =
      c.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
      c.case_reference.toLowerCase().includes(searchTerm.toLowerCase()) ||
      c.lead_investigator.toLowerCase().includes(searchTerm.toLowerCase());
    const matchesStatus = statusFilter === 'all' || c.status.toLowerCase() === statusFilter.toLowerCase();
    return matchesSearch && matchesStatus;
  });

  const getStatusBadge = (status: CaseStatus) => {
    switch (status) {
      case 'open':
      case 'in_progress':
        return 'bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border-slate-300 dark:border-slate-700';
      case 'completed':
        return 'bg-slate-100 dark:bg-[#1B2530] text-slate-600 dark:text-slate-400 border-slate-200 dark:border-slate-800';
      case 'archived':
        return 'bg-amber-50 dark:bg-amber-950/40 text-amber-700 dark:text-amber-300 border-amber-300 dark:border-amber-700';
      default:
        return 'bg-slate-50 dark:bg-[#151C24] text-slate-600 dark:text-slate-400 border-slate-200 dark:border-slate-800';
    }
  };

  return (
    <div className="bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] rounded-xs flex flex-col h-[760px] overflow-hidden">
      {/* Header */}
      <div className="p-3.5 border-b border-slate-200 dark:border-[#26313D] flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Briefcase className="w-4 h-4 text-slate-700 dark:text-slate-300" />
          <h2 className="text-sm font-semibold text-slate-900 dark:text-white">Investigations & Cases</h2>
          <Chip
            size="sm"
            variant="flat"
            className="text-[10px] h-5 font-mono bg-slate-100 dark:bg-[#1B2530] text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]"
          >
            {cases.length}
          </Chip>
        </div>
        <div className="flex items-center gap-1.5">
          <Button
            size="sm"
            color="primary"
            onPress={onOpenCreateModal}
            startContent={<Plus className="w-3.5 h-3.5" />}
            className="bg-slate-900 dark:bg-white text-white dark:text-slate-900 font-semibold h-8"
          >
            New Case
          </Button>
          {onToggleCollapse && (
            <Button
              isIconOnly
              size="sm"
              variant="light"
              onPress={onToggleCollapse}
              className="text-slate-500 hover:text-slate-900 dark:hover:text-white h-8 w-8 min-w-8"
              title="Minimize Investigations & Cases (Maximize investigation view)"
            >
              <PanelLeftClose className="w-4 h-4" />
            </Button>
          )}
        </div>
      </div>

      {/* Search & Filters */}
      <div className="p-3 border-b border-slate-100 dark:border-[#26313D] bg-slate-50/50 dark:bg-[#10161D] flex items-center gap-2">
        <Input
          size="sm"
          variant="bordered"
          placeholder="Search cases..."
          value={searchTerm}
          onValueChange={setSearchTerm}
          startContent={<Search className="w-3.5 h-3.5 text-slate-400" />}
          className="flex-1"
          classNames={{
            input: 'text-xs',
            inputWrapper: 'h-8 min-h-8 bg-white dark:bg-[#151C24] border-slate-200 dark:border-[#26313D]',
          }}
        />
        <Select
          size="sm"
          variant="bordered"
          aria-label="Filter case status"
          selectedKeys={[statusFilter]}
          onChange={(e) => {
            if (e.target.value) setStatusFilter(e.target.value);
          }}
          className="w-32 shrink-0"
          classNames={{
            trigger: 'h-8 min-h-8 bg-white dark:bg-[#151C24] border-slate-200 dark:border-[#26313D]',
            value: 'text-xs capitalize font-medium text-slate-700 dark:text-slate-300',
          }}
        >
          <SelectItem key="all">All Cases</SelectItem>
          <SelectItem key="open">Open</SelectItem>
          <SelectItem key="in_progress">In Progress</SelectItem>
          <SelectItem key="completed">Completed</SelectItem>
          <SelectItem key="archived">Archived</SelectItem>
        </Select>
      </div>

      {/* Case List Viewport */}
      <div className="p-0 flex-1 overflow-y-auto divide-y divide-slate-100 dark:divide-[#26313D]">
        {loading && cases.length === 0 ? (
          <div className="p-8 text-center text-xs text-slate-400">Loading cases...</div>
        ) : filtered.length === 0 ? (
          <div className="p-8 text-center text-xs text-slate-400">
            No matching investigation cases found.
          </div>
        ) : (
          filtered.map((c) => {
            const isSelected = c.case_id === activeCaseId;
            return (
              <div
                key={c.case_id}
                onClick={() => onSelectCase(c.case_id)}
                className={`p-3.5 cursor-pointer transition-all border-l-2 text-left ${
                  isSelected
                    ? 'border-slate-900 dark:border-white bg-slate-100/70 dark:bg-[#1B2530]'
                    : 'border-transparent hover:bg-slate-50/70 dark:hover:bg-[#1B2530]/50'
                }`}
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5 flex-wrap">
                      <span className="font-mono text-[11px] font-bold text-slate-900 dark:text-white">
                        {c.case_reference}
                      </span>
                      <span
                        className={`text-[9px] uppercase font-bold px-1.5 py-0.2 rounded border ${getStatusBadge(
                          c.status
                        )}`}
                      >
                        {c.status.replace('_', ' ')}
                      </span>
                    </div>
                    <h4 className="text-xs font-semibold text-slate-800 dark:text-slate-200 truncate mt-1">
                      {c.title}
                    </h4>
                    <p className="text-[11px] text-slate-500 dark:text-slate-400 truncate mt-0.5">
                      {c.description || 'No description recorded'}
                    </p>
                  </div>
                  <ChevronRight className="w-3.5 h-3.5 text-slate-400 shrink-0 mt-1" />
                </div>

                <div className="mt-2.5 pt-2 border-t border-slate-100 dark:border-[#26313D] flex items-center justify-between text-[10px] text-slate-400 font-mono">
                  <span>Lead: {c.lead_investigator}</span>
                  <div className="flex items-center gap-1">
                    <Clock className="w-2.5 h-2.5 text-slate-400" />
                    <span>{new Date(c.created_at).toLocaleDateString()}</span>
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
