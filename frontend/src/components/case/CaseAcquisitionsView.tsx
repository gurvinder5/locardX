import React from 'react';
import { HardDrive, ShieldCheck } from 'lucide-react';
import {
  Chip,
} from '@heroui/react';
import { CaseAcquisitionItem } from '../../types/case';

interface CaseAcquisitionsViewProps {
  acquisitions: CaseAcquisitionItem[];
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}

export const CaseAcquisitionsView: React.FC<CaseAcquisitionsViewProps> = ({ acquisitions }) => {
  return (
    <div className="bg-white dark:bg-[#151C24] border border-slate-200 dark:border-[#26313D] p-4 space-y-4 rounded-xs">
      <div className="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-[#26313D]">
        <div>
          <h2 className="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
            <HardDrive className="w-4 h-4 text-slate-700 dark:text-slate-300" />
            <span>Forensic Acquisition & Disk Imaging History</span>
          </h2>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
            Bitstream forensic images acquired and cryptographically verified under this case.
          </p>
        </div>
        <Chip size="sm" variant="flat" className="font-mono text-xs bg-slate-100 dark:bg-[#1B2530] text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]">
          {acquisitions.length} {acquisitions.length === 1 ? 'Image' : 'Images'}
        </Chip>
      </div>

      <div>
        {acquisitions.length === 0 ? (
          <div className="text-center py-10 text-slate-400 text-xs flex flex-col items-center justify-center">
            <HardDrive className="w-8 h-8 text-slate-300 dark:text-slate-600 mb-2" />
            <p className="font-semibold text-slate-700 dark:text-slate-300">No bitstream acquisitions recorded</p>
            <p className="mt-1">
              Perform a forensic acquisition under this active case from the Disk Imaging module.
            </p>
          </div>
        ) : (
          <div className="divide-y divide-slate-100 dark:divide-[#26313D] border border-slate-100 dark:border-[#26313D] rounded-lg overflow-hidden">
            {acquisitions.map((acq) => (
              <div key={acq.acquisition_id} className="p-4 hover:bg-slate-50/70 dark:hover:bg-[#1B2530]/50 transition-colors">
                <div className="flex flex-col md:flex-row md:items-center justify-between gap-2 mb-2">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="text-xs font-bold text-slate-900 dark:text-white">{acq.source_display_name}</span>
                    <Chip size="sm" variant="flat" className="text-[11px] font-mono bg-slate-100 dark:bg-[#1B2530] text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-[#26313D]">
                      {acq.image_format.toUpperCase()}
                    </Chip>
                    <Chip
                      size="sm"
                      variant="flat"
                      className={`text-[10px] font-bold uppercase tracking-wider ${
                        acq.status === 'Completed'
                          ? 'bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700'
                          : acq.status === 'Failed'
                          ? 'bg-rose-50 dark:bg-rose-950/40 text-rose-700 dark:text-rose-300 border border-rose-200 dark:border-rose-900'
                          : 'bg-amber-50 dark:bg-amber-950/40 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800'
                      }`}
                    >
                      {acq.status}
                    </Chip>
                  </div>
                  <div className="text-[11px] text-slate-400 font-mono">
                    {acq.completed_at ? new Date(acq.completed_at).toLocaleString() : 'N/A'}
                  </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs text-slate-600 dark:text-slate-400 mt-2 bg-slate-50/50 dark:bg-[#10161D] p-2.5 rounded-lg border border-slate-100 dark:border-[#26313D]">
                  <div>
                    <span className="text-slate-400">Destination Image:</span>{' '}
                    <span className="font-mono text-slate-800 dark:text-slate-200 break-all">{acq.destination_path}</span>
                  </div>
                  <div>
                    <span className="text-slate-400">Size:</span>{' '}
                    <span className="font-semibold text-slate-800 dark:text-slate-200">{formatBytes(acq.image_size_bytes)}</span>
                  </div>
                  <div className="md:col-span-2 flex items-start gap-1">
                    <ShieldCheck className="w-3.5 h-3.5 text-slate-700 dark:text-slate-300 shrink-0 mt-0.5" />
                    <span className="text-slate-400 shrink-0">SHA-256 Digest:</span>
                    <span className="font-mono text-[11px] text-slate-800 dark:text-slate-200 break-all select-all">
                      {acq.image_sha256}
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
