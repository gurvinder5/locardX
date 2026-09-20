import React from 'react';
import { Briefcase, AlertTriangle } from 'lucide-react';
import { useCaseStore } from '../../stores/caseStore';

interface ActiveCaseBannerProps {
  associateWithCase: boolean;
  onToggleAssociate: (associate: boolean) => void;
  onOpenCaseSelector?: () => void;
}

export const ActiveCaseBanner: React.FC<ActiveCaseBannerProps> = ({
  associateWithCase,
  onToggleAssociate,
  onOpenCaseSelector,
}) => {
  const { activeCase } = useCaseStore();
  const isClosed = activeCase?.status === 'completed' || activeCase?.status === 'archived';

  if (!activeCase) {
    return (
      <div className="bg-slate-50 border border-slate-200 rounded-md p-3.5 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 text-xs shadow-2xs">
        <div className="flex items-center gap-2 text-slate-700">
          <Briefcase className="w-4 h-4 text-slate-400 shrink-0" />
          <div>
            <span className="font-semibold text-slate-900">Mode: Standalone Operation</span>
            <span className="text-slate-500 ml-1.5">
              (No active case selected. Sanitization records and certificates will not be bound to a forensic case.)
            </span>
          </div>
        </div>

        {onOpenCaseSelector && (
          <button
            type="button"
            onClick={onOpenCaseSelector}
            className="px-2.5 py-1 text-xs font-semibold rounded bg-white text-slate-700 border border-slate-300 hover:bg-slate-100 transition-colors shadow-2xs shrink-0"
          >
            Select Active Case
          </button>
        )}
      </div>
    );
  }

  return (
    <div
      className={`border rounded-md p-3.5 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 text-xs shadow-2xs ${
        isClosed
          ? 'bg-rose-50/70 border-rose-200 text-rose-900'
          : associateWithCase
          ? 'bg-sky-50/70 border-sky-200 text-sky-950'
          : 'bg-slate-50 border-slate-200 text-slate-700'
      }`}
    >
      <div className="flex items-start sm:items-center gap-2.5">
        <Briefcase
          className={`w-4 h-4 shrink-0 mt-0.5 sm:mt-0 ${
            isClosed ? 'text-rose-600' : associateWithCase ? 'text-sky-600' : 'text-slate-400'
          }`}
        />
        <div>
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-semibold uppercase tracking-wider text-[10px] text-slate-500">
              Active Investigation Case:
            </span>
            <span className="font-mono font-bold text-slate-900">{activeCase.case_reference}</span>
            <span className="text-slate-400">—</span>
            <span className="font-medium text-slate-800">{activeCase.title}</span>
            <span
              className={`text-[9px] uppercase px-1.5 py-0.2 rounded font-bold border ${
                isClosed
                  ? 'bg-rose-100 text-rose-800 border-rose-300'
                  : 'bg-emerald-50 text-emerald-700 border-emerald-200'
              }`}
            >
              {activeCase.status}
            </span>
          </div>

          {isClosed && (
            <p className="text-[11px] text-rose-700 mt-1 flex items-center gap-1 font-medium">
              <AlertTriangle className="w-3 h-3 text-rose-600 shrink-0" />
              <span>
                Case is closed. Forensic operations cannot be linked to this case until it is reopened.
              </span>
            </p>
          )}
        </div>
      </div>

      <div className="flex items-center gap-3 shrink-0">
        {!isClosed ? (
          <label className="flex items-center gap-2 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={associateWithCase}
              onChange={(e) => onToggleAssociate(e.target.checked)}
              className="rounded border-slate-300 text-sky-600 focus:ring-sky-500 w-3.5 h-3.5"
            />
            <span className="font-semibold text-xs text-slate-800">
              Associate with this Case
            </span>
          </label>
        ) : (
          <span className="text-[11px] font-semibold text-rose-700 bg-rose-100/60 px-2 py-0.5 rounded border border-rose-200">
            Association Blocked (Case Closed)
          </span>
        )}

        {onOpenCaseSelector && (
          <button
            type="button"
            onClick={onOpenCaseSelector}
            className="px-2 py-0.5 text-[11px] font-medium rounded text-slate-600 hover:text-slate-900 underline transition-colors"
          >
            Change Case
          </button>
        )}
      </div>
    </div>
  );
};
