import React from 'react';
import { Ban, Search, HardDrive } from 'lucide-react';
import { Progress, Button, Chip } from '@heroui/react';
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
    <div className="space-y-4 pt-2 border-t border-slate-200 dark:border-[#26313D]">
      {/* Header with Badges */}
      <div className="flex items-center justify-between pb-3 border-b border-slate-200 dark:border-[#26313D]">
        <div className="flex items-center space-x-3">
          <div className="p-2 rounded-xs bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200">
            <Search className="w-5 h-5 animate-pulse" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h4 className="text-xs font-bold text-slate-900 dark:text-slate-100 uppercase tracking-wider">
                {isFilesystemMode ? 'Filesystem-Based Recovery' : 'Forensic Evidential Recovery'}
              </h4>
              {progress?.filesystem_type && (
                <Chip size="sm" variant="flat" className="font-mono font-bold bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-[#26313D]">
                  {progress.filesystem_type}
                </Chip>
              )}
              {progress?.phase && (
                <Chip size="sm" variant="flat" className="font-semibold bg-slate-100 dark:bg-[#1B2530] text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-[#26313D]">
                  {progress.phase}
                </Chip>
              )}
            </div>
            <p className="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
              {progress ? progress.stage : 'Initializing Evidential Recovery Engine...'}
            </p>
          </div>
        </div>

        <Button
          size="sm"
          color="danger"
          variant="flat"
          onPress={onCancel}
          startContent={<Ban className="w-3.5 h-3.5" />}
          className="text-xs font-medium h-8"
        >
          Cancel
        </Button>
      </div>

      {/* Source Banner */}
      {sourceName && (
        <div className="flex items-center gap-2 text-xs text-slate-600 dark:text-slate-300 bg-slate-50 dark:bg-[#151C24] p-2.5 rounded-xs border border-slate-200 dark:border-[#26313D] font-mono">
          <HardDrive className="w-4 h-4 text-slate-500 shrink-0" />
          <span className="text-slate-400 text-[11px]">Source:</span>
          <span className="truncate font-semibold text-slate-800 dark:text-slate-200">{sourceName}</span>
        </div>
      )}

      {/* Progress Track */}
      <div className="space-y-1.5">
        <div className="flex justify-between text-[11px] font-mono text-slate-600 dark:text-slate-300">
          <span>{percentage.toFixed(1)}% Scanned</span>
          <span>
            {progress && progress.total_bytes > 0
              ? `${formatBytes(progress.bytes_scanned)} / ${formatBytes(progress.total_bytes)}`
              : '--'}
          </span>
        </div>
        <Progress
          aria-label="Recovery progress"
          value={percentage}
          size="md"
          classNames={{
            indicator: 'bg-slate-900 dark:bg-slate-100',
            track: 'bg-slate-100 dark:bg-[#1B2530]',
          }}
        />
      </div>

      {/* Live Telemetry Stats Ribbon (No Cards) */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 py-3 border-y border-slate-200 dark:border-[#26313D] text-xs">
        <div>
          <span className="text-slate-400 uppercase text-[10px] tracking-wider block font-semibold">Artifacts Carved</span>
          <span className="font-mono font-bold text-slate-900 dark:text-slate-100 text-sm mt-0.5 block">
            {progress?.files_found || 0}
          </span>
        </div>
        <div>
          <span className="text-slate-400 uppercase text-[10px] tracking-wider block font-semibold">Candidates Checked</span>
          <span className="font-mono font-bold text-slate-900 dark:text-slate-100 text-sm mt-0.5 block">
            {progress?.entries_examined ?? progress?.deleted_candidates ?? 0}
          </span>
        </div>
        <div>
          <span className="text-slate-400 uppercase text-[10px] tracking-wider block font-semibold">Elapsed Time</span>
          <span className="font-mono font-bold text-slate-900 dark:text-slate-100 text-sm mt-0.5 block">
            {progress ? `${progress.elapsed_seconds.toFixed(1)}s` : '0.0s'}
          </span>
        </div>
        <div>
          <span className="text-slate-400 uppercase text-[10px] tracking-wider block font-semibold">Throughput</span>
          <span className="font-mono font-bold text-slate-900 dark:text-slate-100 text-sm mt-0.5 block">
            {progress && progress.elapsed_seconds > 0
              ? `${formatBytes(progress.bytes_scanned / progress.elapsed_seconds)}/s`
              : '-- MB/s'}
          </span>
        </div>
      </div>
    </div>
  );
};
