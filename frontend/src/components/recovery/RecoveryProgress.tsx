import React from 'react';
import { Ban, Search } from 'lucide-react';
import { RecoveryProgress as ProgressType } from '../../types/recovery';

interface RecoveryProgressProps {
  progress: ProgressType | null;
  onCancel: () => void;
}

export const RecoveryProgress: React.FC<RecoveryProgressProps> = ({ progress, onCancel }) => {
  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };

  const percentage = progress ? Math.min(100, Math.max(0, progress.percentage)) : 0;

  return (
    <div className="bg-white border border-slate-200 rounded-md p-5 space-y-4 shadow-xs">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-2.5">
          <div className="p-2 rounded-md bg-indigo-50 text-indigo-600">
            <Search className="w-5 h-5 animate-pulse" />
          </div>
          <div>
            <h4 className="text-xs font-semibold text-slate-800">
              {progress ? progress.stage : 'Initializing Evidential Carving Engine...'}
            </h4>
            <p className="text-[11px] text-slate-500">
              Streaming bitstream DD image strictly read-only
            </p>
          </div>
        </div>
        <button
          onClick={onCancel}
          className="px-2.5 py-1.5 rounded text-xs font-medium text-rose-600 hover:bg-rose-50 border border-rose-200 flex items-center gap-1.5 transition-colors"
        >
          <Ban className="w-3.5 h-3.5" />
          Cancel
        </button>
      </div>

      {/* Progress Track */}
      <div className="space-y-1.5">
        <div className="flex justify-between text-[11px] font-mono text-slate-600">
          <span>{percentage.toFixed(1)}% Scanned</span>
          <span>
            {progress ? `${formatBytes(progress.bytes_scanned)} / ${formatBytes(progress.total_bytes)}` : '--'}
          </span>
        </div>
        <div className="w-full bg-slate-100 rounded-full h-2.5 overflow-hidden">
          <div
            className="bg-indigo-600 h-2.5 rounded-full transition-all duration-300 ease-out"
            style={{ width: `${percentage}%` }}
          />
        </div>
      </div>

      {/* Metrics Row */}
      <div className="grid grid-cols-3 gap-2 pt-1 border-t border-slate-100 text-center">
        <div className="p-2 bg-slate-50 rounded">
          <div className="text-[10px] text-slate-400 uppercase font-semibold">Throughput</div>
          <div className="text-xs font-bold text-slate-700 font-mono">
            {progress ? `${progress.throughput_mbps.toFixed(1)} MB/s` : '--'}
          </div>
        </div>
        <div className="p-2 bg-slate-50 rounded">
          <div className="text-[10px] text-slate-400 uppercase font-semibold">Elapsed Time</div>
          <div className="text-xs font-bold text-slate-700 font-mono">
            {progress ? `${progress.elapsed_seconds.toFixed(1)}s` : '--'}
          </div>
        </div>
        <div className="p-2 bg-slate-50 rounded">
          <div className="text-[10px] text-slate-400 uppercase font-semibold">Files Discovered</div>
          <div className="text-xs font-bold text-emerald-600 font-mono">
            {progress ? progress.files_found : 0}
          </div>
        </div>
      </div>
    </div>
  );
};
