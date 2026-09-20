import React from 'react';
import { Layers, ShieldCheck, FileText, HardDrive } from 'lucide-react';
import { CaseErasureItem } from '../../types/case';

interface CaseErasuresViewProps {
  erasures: CaseErasureItem[];
}

export const CaseErasuresView: React.FC<CaseErasuresViewProps> = ({ erasures }) => {
  return (
    <div className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs space-y-4">
      <div className="flex items-center justify-between pb-3 border-b border-slate-100">
        <div>
          <h2 className="text-sm font-bold text-slate-900 flex items-center gap-2">
            <Layers className="w-4 h-4 text-amber-600" />
            <span>Sanitization & Secure Erasure History</span>
          </h2>
          <p className="text-xs text-slate-500 mt-0.5">
            Cryptographically tracked drive sanitizations and secure file/folder erasures linked to this case.
          </p>
        </div>
        <span className="text-xs font-mono font-medium px-2 py-0.5 rounded bg-slate-100 text-slate-700">
          {erasures.length} {erasures.length === 1 ? 'Operation' : 'Operations'}
        </span>
      </div>

      {erasures.length === 0 ? (
        <div className="text-center py-10 text-slate-400 text-xs flex flex-col items-center justify-center">
          <Layers className="w-8 h-8 text-slate-300 mb-2" />
          <p className="font-semibold text-slate-600">No sanitization operations recorded</p>
          <p className="mt-1">
            Perform a secure drive or file erasure associated with this active case.
          </p>
        </div>
      ) : (
        <div className="divide-y divide-slate-100 border border-slate-100 rounded-md overflow-hidden">
          {erasures.map((era) => (
            <div key={era.record_id} className="p-4 hover:bg-slate-50/70 transition-colors">
              <div className="flex flex-col md:flex-row md:items-center justify-between gap-2 mb-2">
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="flex items-center gap-1.5 text-xs font-bold text-slate-900">
                    {era.erasure_type === 'Drive' ? (
                      <HardDrive className="w-3.5 h-3.5 text-amber-600" />
                    ) : (
                      <FileText className="w-3.5 h-3.5 text-sky-600" />
                    )}
                    <span>{era.erasure_type} Sanitization</span>
                  </span>
                  <span className="text-[11px] font-mono px-1.5 py-0.5 rounded bg-slate-100 text-slate-600 border border-slate-200">
                    {era.method}
                  </span>
                  <span
                    className={`text-[10px] font-bold px-2 py-0.5 rounded uppercase tracking-wider ${
                      era.status === 'Completed'
                        ? 'bg-emerald-50 text-emerald-700 border border-emerald-200'
                        : era.status === 'Failed'
                        ? 'bg-rose-50 text-rose-700 border border-rose-200'
                        : 'bg-amber-50 text-amber-700 border border-amber-200'
                    }`}
                  >
                    {era.status}
                  </span>
                  <span
                    className={`text-[10px] font-bold px-2 py-0.5 rounded flex items-center gap-1 ${
                      era.verification_outcome === 'Verified'
                        ? 'bg-emerald-50 text-emerald-700 border border-emerald-200'
                        : 'bg-amber-50 text-amber-700 border border-amber-200'
                    }`}
                  >
                    <ShieldCheck className="w-3 h-3" />
                    <span>{era.verification_outcome}</span>
                  </span>
                </div>
                <div className="text-[11px] text-slate-400 font-mono">
                  {era.completed_at ? new Date(era.completed_at).toLocaleString() : 'N/A'}
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs text-slate-600 mt-2 bg-slate-50/50 p-2.5 rounded border border-slate-100">
                <div className="md:col-span-2">
                  <span className="text-slate-400">Target:</span>{' '}
                  <span className="font-mono text-slate-800 break-all font-semibold">
                    {era.target_identifier}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400">Operation ID:</span>{' '}
                  <span className="font-mono text-slate-700 text-[11px]">{era.operation_id}</span>
                </div>
                <div>
                  <span className="text-slate-400">Record ID:</span>{' '}
                  <span className="font-mono text-slate-700 text-[11px]">{era.record_id}</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
