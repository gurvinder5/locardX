import React from 'react';
import { Search } from 'lucide-react';
import { CaseRecoveryItem } from '../../types/case';

interface CaseRecoveriesViewProps {
  recoveries: CaseRecoveryItem[];
}

export const CaseRecoveriesView: React.FC<CaseRecoveriesViewProps> = ({ recoveries }) => {
  return (
    <div className="bg-white border border-slate-200 rounded-md p-5 shadow-2xs space-y-4">
      <div className="flex items-center justify-between pb-3 border-b border-slate-100">
        <div>
          <h2 className="text-sm font-bold text-slate-900 flex items-center gap-2">
            <Search className="w-4 h-4 text-emerald-600" />
            <span>Forensic File Recovery & Carving History</span>
          </h2>
          <p className="text-xs text-slate-500 mt-0.5">
            Filesystem metadata traversals and file signature carving jobs executed for this case.
          </p>
        </div>
        <span className="text-xs font-mono font-medium px-2 py-0.5 rounded bg-slate-100 text-slate-700">
          {recoveries.length} {recoveries.length === 1 ? 'Job' : 'Jobs'}
        </span>
      </div>

      {recoveries.length === 0 ? (
        <div className="text-center py-10 text-slate-400 text-xs flex flex-col items-center justify-center">
          <Search className="w-8 h-8 text-slate-300 mb-2" />
          <p className="font-semibold text-slate-600">No recovery jobs recorded</p>
          <p className="mt-1">
            Run a recovery or file carving scan under this active case from the Recovery module.
          </p>
        </div>
      ) : (
        <div className="divide-y divide-slate-100 border border-slate-100 rounded-md overflow-hidden">
          {recoveries.map((rec) => (
            <div key={rec.job_id} className="p-4 hover:bg-slate-50/70 transition-colors">
              <div className="flex flex-col md:flex-row md:items-center justify-between gap-2 mb-2">
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="text-xs font-bold text-slate-900">Job: {rec.job_id}</span>
                  <span className="text-[11px] font-mono px-1.5 py-0.5 rounded bg-slate-100 text-slate-600 border border-slate-200">
                    {rec.recovery_mode}
                  </span>
                  <span
                    className={`text-[10px] font-bold px-2 py-0.5 rounded uppercase tracking-wider ${
                      rec.status === 'Completed'
                        ? 'bg-emerald-50 text-emerald-700 border border-emerald-200'
                        : rec.status === 'Failed'
                        ? 'bg-rose-50 text-rose-700 border border-rose-200'
                        : 'bg-amber-50 text-amber-700 border border-amber-200'
                    }`}
                  >
                    {rec.status}
                  </span>
                </div>
                <div className="text-[11px] text-slate-400 font-mono">
                  {rec.completed_at ? new Date(rec.completed_at).toLocaleString() : 'N/A'}
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs text-slate-600 mt-2 bg-slate-50/50 p-2.5 rounded border border-slate-100">
                <div className="md:col-span-2">
                  <span className="text-slate-400">Source Image:</span>{' '}
                  <span className="font-mono text-slate-800 break-all">{rec.source_image_path}</span>
                </div>
                <div>
                  <span className="text-slate-400">Files Recovered:</span>{' '}
                  <strong className="text-emerald-700 font-bold">{rec.files_recovered}</strong>
                </div>
                <div>
                  <span className="text-slate-400">Source SHA-256:</span>{' '}
                  <span className="font-mono text-[11px] text-slate-800 truncate inline-block max-w-[200px] align-bottom">
                    {rec.source_image_sha256}
                  </span>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
