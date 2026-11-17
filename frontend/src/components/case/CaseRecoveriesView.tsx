import React from 'react';
import { Search } from 'lucide-react';
import {
  Chip,
} from '@heroui/react';
import { CaseRecoveryItem } from '../../types/case';

interface CaseRecoveriesViewProps {
  recoveries: CaseRecoveryItem[];
}

export const CaseRecoveriesView: React.FC<CaseRecoveriesViewProps> = ({ recoveries }) => {
  return (
    <div className="bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] p-4 space-y-4 rounded-xs">
      <div className="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-[#26313D]">
        <div>
          <h2 className="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
            <Search className="w-4 h-4 text-slate-700 dark:text-slate-300" />
            <span>Forensic File Recovery & Carving History</span>
          </h2>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
            Filesystem metadata traversals and file signature carving jobs executed for this case.
          </p>
        </div>
        <Chip size="sm" variant="flat" className="font-mono text-xs bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]">
          {recoveries.length} {recoveries.length === 1 ? 'Job' : 'Jobs'}
        </Chip>
      </div>

      <div>
        {recoveries.length === 0 ? (
          <div className="text-center py-10 text-slate-400 text-xs flex flex-col items-center justify-center">
            <Search className="w-8 h-8 text-slate-300 dark:text-slate-600 mb-2" />
            <p className="font-semibold text-slate-700 dark:text-slate-300">No recovery jobs recorded</p>
            <p className="mt-1">
              Run a recovery or file carving scan under this active case from the Recovery module.
            </p>
          </div>
        ) : (
          <div className="divide-y divide-slate-100 dark:divide-[#26313D] border border-slate-100 dark:border-[#26313D] rounded-lg overflow-hidden">
            {recoveries.map((rec) => (
              <div key={rec.job_id} className="p-4 hover:bg-slate-50/70 dark:hover:bg-[#1B2530]/50 transition-colors">
                <div className="flex flex-col md:flex-row md:items-center justify-between gap-2 mb-2">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="text-xs font-bold text-slate-900 dark:text-white">Job: {rec.job_id}</span>
                    <Chip size="sm" variant="flat" className="text-[11px] font-mono bg-slate-100 dark:bg-[#1B2530] text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]">
                      {rec.recovery_mode}
                    </Chip>
                    <Chip
                      size="sm"
                      variant="flat"
                      className={`text-[10px] font-bold uppercase tracking-wider ${
                        rec.status === 'Completed'
                          ? 'bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700'
                          : rec.status === 'Failed'
                          ? 'bg-rose-50 dark:bg-rose-950/40 text-rose-700 dark:text-rose-300 border border-rose-200 dark:border-rose-900'
                          : 'bg-amber-50 dark:bg-amber-950/40 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800'
                      }`}
                    >
                      {rec.status}
                    </Chip>
                  </div>
                  <div className="text-[11px] text-slate-400 font-mono">
                    {rec.completed_at ? new Date(rec.completed_at).toLocaleString() : 'N/A'}
                  </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs text-slate-600 dark:text-slate-400 mt-2 bg-slate-50/50 dark:bg-[#10161D] p-2.5 rounded-lg border border-slate-100 dark:border-[#26313D]">
                  <div className="md:col-span-2">
                    <span className="text-slate-400">Source Image:</span>{' '}
                    <span className="font-mono text-slate-800 dark:text-slate-200 break-all">{rec.source_image_path}</span>
                  </div>
                  <div>
                    <span className="text-slate-400">Files Recovered:</span>{' '}
                    <strong className="text-slate-900 dark:text-white font-bold">{rec.files_recovered}</strong>
                  </div>
                  <div>
                    <span className="text-slate-400">Source SHA-256:</span>{' '}
                    <span className="font-mono text-[11px] text-slate-800 dark:text-slate-200 truncate inline-block max-w-[200px] align-bottom">
                      {rec.source_image_sha256}
                    </span>
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
