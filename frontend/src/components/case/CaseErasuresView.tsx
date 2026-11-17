import React from 'react';
import { Layers, ShieldCheck, FileText, HardDrive } from 'lucide-react';
import {
  Chip,
} from '@heroui/react';
import { CaseErasureItem } from '../../types/case';

interface CaseErasuresViewProps {
  erasures: CaseErasureItem[];
}

export const CaseErasuresView: React.FC<CaseErasuresViewProps> = ({ erasures }) => {
  return (
    <div className="bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] p-4 space-y-4 rounded-xs">
      <div className="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-[#26313D]">
        <div>
          <h2 className="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
            <Layers className="w-4 h-4 text-amber-600 dark:text-amber-400" />
            <span>Sanitization & Secure Erasure History</span>
          </h2>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
            Cryptographically tracked drive sanitizations and secure file/folder erasures linked to this case.
          </p>
        </div>
        <Chip size="sm" variant="flat" className="font-mono text-xs bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]">
          {erasures.length} {erasures.length === 1 ? 'Operation' : 'Operations'}
        </Chip>
      </div>

      <div>
        {erasures.length === 0 ? (
          <div className="text-center py-10 text-slate-400 text-xs flex flex-col items-center justify-center">
            <Layers className="w-8 h-8 text-slate-300 dark:text-slate-600 mb-2" />
            <p className="font-semibold text-slate-700 dark:text-slate-300">No sanitization operations recorded</p>
            <p className="mt-1">
              Perform a secure drive or file erasure associated with this active case.
            </p>
          </div>
        ) : (
          <div className="divide-y divide-slate-100 dark:divide-[#26313D] border border-slate-100 dark:border-[#26313D] rounded-lg overflow-hidden">
            {erasures.map((era) => (
              <div key={era.record_id} className="p-4 hover:bg-slate-50/70 dark:hover:bg-[#1B2530]/50 transition-colors">
                <div className="flex flex-col md:flex-row md:items-center justify-between gap-2 mb-2">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="flex items-center gap-1.5 text-xs font-bold text-slate-900 dark:text-white">
                      {era.erasure_type === 'Drive' ? (
                        <HardDrive className="w-3.5 h-3.5 text-amber-600 dark:text-amber-400" />
                      ) : (
                        <FileText className="w-3.5 h-3.5 text-slate-600 dark:text-slate-400" />
                      )}
                      <span>{era.erasure_type} Sanitization</span>
                    </span>
                    <Chip size="sm" variant="flat" className="text-[11px] font-mono bg-slate-100 dark:bg-[#1B2530] text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]">
                      {era.method}
                    </Chip>
                    <Chip
                      size="sm"
                      variant="flat"
                      className={`text-[10px] font-bold uppercase tracking-wider ${
                        era.status === 'Completed'
                          ? 'bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700'
                          : era.status === 'Failed'
                          ? 'bg-rose-50 dark:bg-rose-950/40 text-rose-700 dark:text-rose-300 border border-rose-200 dark:border-rose-900'
                          : 'bg-amber-50 dark:bg-amber-950/40 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800'
                      }`}
                    >
                      {era.status}
                    </Chip>
                    <Chip
                      size="sm"
                      variant="flat"
                      startContent={<ShieldCheck className="w-3 h-3 text-slate-700 dark:text-slate-300" />}
                      className={`text-[10px] font-bold ${
                        era.verification_outcome === 'Verified'
                          ? 'bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700'
                          : 'bg-amber-50 dark:bg-amber-950/40 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800'
                      }`}
                    >
                      {era.verification_outcome}
                    </Chip>
                  </div>
                  <div className="text-[11px] text-slate-400 font-mono">
                    {era.completed_at ? new Date(era.completed_at).toLocaleString() : 'N/A'}
                  </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs text-slate-600 dark:text-slate-400 mt-2 bg-slate-50/50 dark:bg-[#10161D] p-2.5 rounded-lg border border-slate-100 dark:border-[#26313D]">
                  <div className="md:col-span-2">
                    <span className="text-slate-400">Target:</span>{' '}
                    <span className="font-mono text-slate-800 dark:text-slate-200 break-all font-semibold">
                      {era.target_identifier}
                    </span>
                  </div>
                  <div>
                    <span className="text-slate-400">Operation ID:</span>{' '}
                    <span className="font-mono text-slate-700 dark:text-slate-300 text-[11px]">{era.operation_id}</span>
                  </div>
                  <div>
                    <span className="text-slate-400">Record ID:</span>{' '}
                    <span className="font-mono text-slate-700 dark:text-slate-300 text-[11px]">{era.record_id}</span>
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
