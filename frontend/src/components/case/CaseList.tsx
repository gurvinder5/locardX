import React, { useState } from 'react';
import { Briefcase, Plus, Search, Filter, Clock, ChevronRight } from 'lucide-react';
import { Case, CaseStatus } from '../../types/case';

interface CaseListProps {
  cases: Case[];
  activeCaseId: string | null;
  onSelectCase: (caseId: string) => void;
  onOpenCreateModal: () => void;
  loading: boolean;
}

export const CaseList: React.FC<CaseListProps> = ({
  cases,
  activeCaseId,
  onSelectCase,
  onOpenCreateModal,
  loading,
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
        return 'bg-emerald-50 text-emerald-700 border-emerald-200';
      case 'in_progress':
        return 'bg-blue-50 text-blue-700 border-blue-200';
      case 'completed':
        return 'bg-slate-100 text-slate-700 border-slate-300';
      case 'archived':
        return 'bg-amber-50 text-amber-700 border-amber-200';
      default:
        return 'bg-slate-50 text-slate-600 border-slate-200';
    }
  };

  return (
    <div className="bg-white border border-slate-200 rounded-md shadow-2xs flex flex-col h-[760px]">
      {/* Header */}
      <div className="p-4 border-b border-slate-200 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Briefcase className="w-4 h-4 text-sky-600" />
          <h2 className="text-sm font-semibold text-slate-800">Investigations & Cases</h2>
          <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-slate-100 text-slate-600 border border-slate-200">
            {cases.length}
          </span>
        </div>
        <button
          onClick={onOpenCreateModal}
          className="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded bg-sky-600 text-white hover:bg-sky-700 transition-colors shadow-2xs"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>New Case</span>
        </button>
      </div>

      {/* Search & Filters */}
      <div className="p-3 border-b border-slate-100 bg-slate-50/50 space-y-2">
        <div className="relative">
          <Search className="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-2.5" />
          <input
            type="text"
            placeholder="Search reference, title, or investigator..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full text-xs pl-8 pr-3 py-1.5 rounded border border-slate-200 bg-white focus:outline-hidden focus:border-sky-500 focus:ring-1 focus:ring-sky-500"
          />
        </div>
        <div className="flex items-center gap-1.5 text-xs text-slate-500">
          <Filter className="w-3 h-3 text-slate-400" />
          <span className="text-[11px]">Filter:</span>
          {(['all', 'open', 'in_progress', 'completed', 'archived'] as const).map((st) => (
            <button
              key={st}
              onClick={() => setStatusFilter(st)}
              className={`px-2 py-0.5 rounded text-[10px] uppercase font-semibold transition-colors ${
                statusFilter === st
                  ? 'bg-slate-800 text-white'
                  : 'bg-white border border-slate-200 text-slate-600 hover:bg-slate-100'
              }`}
            >
              {st.replace('_', ' ')}
            </button>
          ))}
        </div>
      </div>

      {/* Case List Viewport */}
      <div className="flex-1 overflow-y-auto divide-y divide-slate-100">
        {loading && cases.length === 0 ? (
          <div className="p-6 text-center text-xs text-slate-400">Loading cases...</div>
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
                className={`p-3.5 cursor-pointer transition-colors flex items-start justify-between gap-2 ${
                  isSelected ? 'bg-sky-50/70 border-l-4 border-sky-600' : 'hover:bg-slate-50'
                }`}
              >
                <div className="min-w-0 space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-[11px] font-bold text-sky-700">
                      {c.case_reference}
                    </span>
                    <span
                      className={`text-[9px] uppercase font-bold tracking-wider px-1.5 py-0.5 rounded border ${getStatusBadge(
                        c.status
                      )}`}
                    >
                      {c.status.replace('_', ' ')}
                    </span>
                  </div>
                  <h3 className="text-xs font-semibold text-slate-800 truncate">{c.title}</h3>
                  <div className="flex items-center gap-3 text-[10px] text-slate-400">
                    <span>Lead: {c.lead_investigator}</span>
                    <span>&bull;</span>
                    <span className="flex items-center gap-1">
                      <Clock className="w-2.5 h-2.5" />
                      {new Date(c.created_at).toLocaleDateString()}
                    </span>
                  </div>
                </div>
                <ChevronRight
                  className={`w-4 h-4 mt-1 transition-transform ${
                    isSelected ? 'text-sky-600 translate-x-0.5' : 'text-slate-300'
                  }`}
                />
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
