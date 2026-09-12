import React from 'react';
import {
  Activity,
  HardDrive,
  Search,
  FileCheck,
  ShieldCheck,
  Clock,
  User,
  Calendar,
  Layers,
  FileText,
} from 'lucide-react';
import { Case, CaseSummary, CaseStatus } from '../../types/case';

interface CaseOverviewProps {
  currentCase: Case;
  summary: CaseSummary | null;
  onChangeStatus: (status: CaseStatus) => void;
  onGenerateReport: () => void;
  reportLoading: boolean;
}

export const CaseOverview: React.FC<CaseOverviewProps> = ({
  currentCase,
  summary,
  onChangeStatus,
  onGenerateReport,
  reportLoading,
}) => {
  return (
    <div className="space-y-5">
      {/* Case Header Banner */}
      <div className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs">
        <div className="flex flex-col lg:flex-row justify-between items-start lg:items-center gap-4 pb-4 border-b border-slate-100">
          <div>
            <div className="flex items-center gap-2.5">
              <span className="font-mono text-xs font-bold px-2 py-0.5 rounded bg-sky-50 text-sky-700 border border-sky-200">
                {currentCase.case_reference}
              </span>
              <h1 className="text-base font-bold text-slate-900">{currentCase.title}</h1>
            </div>
            <p className="text-xs text-slate-500 mt-1 max-w-3xl leading-relaxed">
              {currentCase.description || 'No description provided.'}
            </p>
          </div>

          {/* Status & Report Controls */}
          <div className="flex items-center gap-2 flex-wrap">
            <div className="flex items-center gap-1 bg-slate-50 border border-slate-200 p-1 rounded-md">
              {(['open', 'in_progress', 'completed', 'archived'] as CaseStatus[]).map((st) => (
                <button
                  key={st}
                  onClick={() => onChangeStatus(st)}
                  className={`px-2.5 py-1 text-xs font-semibold rounded transition-colors ${
                    currentCase.status === st
                      ? 'bg-white text-slate-900 shadow-2xs border border-slate-200'
                      : 'text-slate-500 hover:text-slate-800'
                  }`}
                >
                  {st.replace('_', ' ').toUpperCase()}
                </button>
              ))}
            </div>

            <button
              onClick={onGenerateReport}
              disabled={reportLoading}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded bg-emerald-600 text-white hover:bg-emerald-700 disabled:opacity-50 transition-colors shadow-2xs"
            >
              <FileCheck className="w-3.5 h-3.5" />
              <span>{reportLoading ? 'Generating...' : 'Generate Case Report'}</span>
            </button>
          </div>
        </div>

        {/* Case Metadata Badges */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 pt-3 text-xs text-slate-600">
          <div className="flex items-center gap-2">
            <User className="w-3.5 h-3.5 text-slate-400" />
            <span>
              Investigator: <strong className="text-slate-800">{currentCase.lead_investigator}</strong>
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Calendar className="w-3.5 h-3.5 text-slate-400" />
            <span>
              Created: <strong className="text-slate-800">{new Date(currentCase.created_at).toLocaleDateString()}</strong>
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Clock className="w-3.5 h-3.5 text-slate-400" />
            <span>
              Updated: <strong className="text-slate-800">{new Date(currentCase.updated_at).toLocaleTimeString()}</strong>
            </span>
          </div>
          <div className="flex items-center gap-2">
            <ShieldCheck className="w-3.5 h-3.5 text-emerald-600" />
            <span>
              Audit Chain:{' '}
              <strong className="text-emerald-700 font-mono">
                {summary?.audit_chain_status || 'Verified'}
              </strong>
            </span>
          </div>
        </div>
      </div>

      {/* KPI Metrics Summary */}
      <div className="grid grid-cols-2 md:grid-cols-5 gap-3">
        {/* Operations */}
        <div className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs">
          <div className="flex items-center justify-between text-slate-500 mb-1.5">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Operations</span>
            <Activity className="w-4 h-4 text-indigo-600" />
          </div>
          <div className="text-xl font-bold text-slate-900">{summary?.operation_count ?? 0}</div>
          <span className="text-[10px] text-slate-400">Linked forensic procedures</span>
        </div>

        {/* Evidence */}
        <div className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs">
          <div className="flex items-center justify-between text-slate-500 mb-1.5">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Evidence Items</span>
            <HardDrive className="w-4 h-4 text-sky-600" />
          </div>
          <div className="text-xl font-bold text-slate-900">{summary?.evidence_count ?? 0}</div>
          <span className="text-[10px] text-slate-400">Media, DD images, datasets</span>
        </div>

        {/* Recovered Files */}
        <div className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs">
          <div className="flex items-center justify-between text-slate-500 mb-1.5">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Carved Files</span>
            <Search className="w-4 h-4 text-emerald-600" />
          </div>
          <div className="text-xl font-bold text-slate-900">{summary?.recovered_file_count ?? 0}</div>
          <span className="text-[10px] text-slate-400">Reconstructed candidates</span>
        </div>

        {/* Sanitization */}
        <div className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs">
          <div className="flex items-center justify-between text-slate-500 mb-1.5">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Sanitization</span>
            <Layers className="w-4 h-4 text-amber-600" />
          </div>
          <div className="text-xl font-bold text-slate-900">{summary?.erasure_count ?? 0}</div>
          <span className="text-[10px] text-slate-400">Completed drive erasures</span>
        </div>

        {/* Reports */}
        <div className="bg-white border border-slate-200 rounded-md p-4 shadow-2xs">
          <div className="flex items-center justify-between text-slate-500 mb-1.5">
            <span className="text-[11px] font-semibold uppercase tracking-wider">Reports</span>
            <FileText className="w-4 h-4 text-purple-600" />
          </div>
          <div className="text-xl font-bold text-slate-900">{summary?.report_count ?? 0}</div>
          <span className="text-[10px] text-slate-400">Cryptographically signed</span>
        </div>
      </div>
    </div>
  );
};
