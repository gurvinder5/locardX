import { Ban, Search, HardDrive, Database } from 'lucide-react';
import { RecoveryProgress as ProgressType } from '../../types/recovery';

interface RecoveryProgressProps {
  progress: ProgressType | null;
  sourceName?: string | null;
  recoveryMode?: string | null;
  onCancel: () => void;
}

export const RecoveryProgress: React.FC<RecoveryProgressProps> = ({
  progress,
  sourceName,
  recoveryMode,
  onCancel,
}) => {
  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };

  const percentage = progress ? Math.min(100, Math.max(0, progress.percentage)) : 0;
  const isFilesystemMode = recoveryMode === 'filesystem_only' || progress?.filesystem_type;

  return (
    <div className="bg-white border border-slate-200 rounded-md p-5 space-y-4 shadow-xs">
      {/* Header with Badges */}
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-3">
          <div className="p-2.5 rounded-md bg-indigo-50 text-indigo-600">
            <Search className="w-5 h-5 animate-pulse" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h4 className="text-xs font-bold text-slate-800">
                {isFilesystemMode ? 'Filesystem-Based Recovery' : 'Forensic Evidential Recovery'}
              </h4>
              {progress?.filesystem_type && (
                <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-emerald-50 text-emerald-700 border border-emerald-200">
                  {progress.filesystem_type}
                </span>
              )}
              {progress?.phase && (
                <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-indigo-50 text-indigo-700 border border-indigo-200">
                  {progress.phase}
                </span>
              )}
            </div>
            <p className="text-[11px] text-slate-500 mt-0.5">
              {progress ? progress.stage : 'Initializing Evidential Recovery Engine...'}
            </p>
          </div>
        </div>

        <button
          onClick={onCancel}
          className="px-3 py-1.5 rounded text-xs font-medium text-rose-600 hover:bg-rose-50 border border-rose-200 flex items-center gap-1.5 transition-colors"
        >
          <Ban className="w-3.5 h-3.5" />
          Cancel
        </button>
      </div>

      {/* Source Banner */}
      {sourceName && (
        <div className="flex items-center gap-2 text-xs text-slate-600 bg-slate-50 p-2.5 rounded border border-slate-100 font-mono">
          <HardDrive className="w-4 h-4 text-slate-500 shrink-0" />
          <span className="text-slate-400 text-[11px]">Source:</span>
          <span className="truncate font-semibold text-slate-800">{sourceName}</span>
        </div>
      )}

      {/* Progress Track */}
      <div className="space-y-1.5">
        <div className="flex justify-between text-[11px] font-mono text-slate-600">
          <span>{percentage.toFixed(1)}% Scanned</span>
          <span>
            {progress && progress.total_bytes > 0
              ? `${formatBytes(progress.bytes_scanned)} / ${formatBytes(progress.total_bytes)}`
              : '--'}
          </span>
        </div>
        <div className="w-full bg-slate-100 rounded-full h-2.5 overflow-hidden">
          <div
            className="bg-indigo-600 h-2.5 rounded-full transition-all duration-300 ease-out"
            style={{ width: `${percentage}%` }}
          />
        </div>
      </div>

      {/* Metrics Row (Counters actually supplied by backend) */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-2 pt-2 border-t border-slate-100 text-center">
        <div className="p-2.5 bg-slate-50 rounded">
          <div className="text-[10px] text-slate-400 uppercase font-semibold">Files Examined</div>
          <div className="text-xs font-bold text-slate-700 font-mono mt-0.5">
            {progress?.entries_examined !== undefined ? progress.entries_examined : '--'}
          </div>
        </div>

        <div className="p-2.5 bg-slate-50 rounded">
          <div className="text-[10px] text-slate-400 uppercase font-semibold">Deleted Candidates</div>
          <div className="text-xs font-bold text-amber-700 font-mono mt-0.5">
            {progress?.deleted_candidates !== undefined ? progress.deleted_candidates : '--'}
          </div>
        </div>

        <div className="p-2.5 bg-slate-50 rounded">
          <div className="text-[10px] text-slate-400 uppercase font-semibold">Recovered Files</div>
          <div className="text-xs font-bold text-emerald-600 font-mono mt-0.5">
            {progress ? progress.files_found : 0}
          </div>
        </div>

        <div className="p-2.5 bg-slate-50 rounded">
          <div className="text-[10px] text-slate-400 uppercase font-semibold">Validated</div>
          <div className="text-xs font-bold text-blue-600 font-mono mt-0.5">
            {progress?.files_validated !== undefined ? progress.files_validated : '--'}
          </div>
        </div>
      </div>

      {/* Telemetry & Current Operation Footer */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between text-[11px] text-slate-500 pt-1 px-1">
        <div className="flex items-center gap-1.5 font-mono">
          <Database className="w-3.5 h-3.5 text-indigo-500" />
          <span>
            Current Operation:{' '}
            <strong className="text-slate-700 font-medium">
              {progress?.current_operation || progress?.stage || 'Running filesystem analysis...'}
            </strong>
          </span>
        </div>
        <div className="flex items-center gap-3 font-mono text-[10px] mt-1 sm:mt-0 text-slate-400">
          <span>Elapsed: {progress ? `${progress.elapsed_seconds.toFixed(1)}s` : '0.0s'}</span>
          <span>&bull;</span>
          <span>Throughput: {progress ? `${progress.throughput_mbps.toFixed(1)} MB/s` : '--'}</span>
        </div>
      </div>
    </div>
  );
};
