import React from 'react';
import { HardDrive, ShieldCheck, CheckCircle2 } from 'lucide-react';
import { RecoverySourceSnapshot } from '../../types/recovery';

interface RecoverySourceSelectorProps {
  sources: RecoverySourceSnapshot[];
  loading: boolean;
  selectedSourceId: string | null;
  onSelectSource: (source: RecoverySourceSnapshot) => void;
}

export const RecoverySourceSelector: React.FC<RecoverySourceSelectorProps> = ({
  sources,
  loading,
  selectedSourceId,
  onSelectSource,
}) => {
  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };

  if (loading) {
    return (
      <div className="p-8 text-center bg-slate-50 border border-slate-200 rounded-md text-slate-500 text-xs">
        Loading verified acquisition sources...
      </div>
    );
  }

  if (sources.length === 0) {
    return (
      <div className="p-8 text-center bg-slate-50 border border-slate-200 rounded-md text-slate-500 space-y-2">
        <HardDrive className="w-8 h-8 text-slate-400 mx-auto" />
        <p className="text-xs font-medium text-slate-700">No Verified Acquisition Images Found</p>
        <p className="text-[11px] text-slate-500">
          Acquire a physical storage drive in the "Disk Imaging" tab to generate evidential raw DD images.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <label className="block text-xs font-semibold text-slate-700">
        Select Verified Evidence Image (AcquisitionArtifact)
      </label>
      <div className="grid grid-cols-1 gap-2.5">
        {sources.map((src) => {
          const isSelected = selectedSourceId === src.source_id;
          return (
            <div
              key={src.source_id}
              onClick={() => onSelectSource(src)}
              className={`p-3.5 border rounded-md cursor-pointer transition-all ${
                isSelected
                  ? 'border-indigo-600 bg-indigo-50/40 shadow-xs ring-1 ring-indigo-500'
                  : 'border-slate-200 bg-white hover:border-slate-300 hover:bg-slate-50/50'
              }`}
            >
              <div className="flex items-start justify-between">
                <div className="flex items-start space-x-3">
                  <div className={`p-2 rounded-md ${isSelected ? 'bg-indigo-600 text-white' : 'bg-slate-100 text-slate-600'}`}>
                    <HardDrive className="w-4 h-4" />
                  </div>
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-semibold text-slate-900 font-mono">
                        {src.image_path}
                      </span>
                      {src.is_trusted && (
                        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-emerald-50 text-emerald-700 border border-emerald-200">
                          <ShieldCheck className="w-3 h-3" />
                          Hash Verified
                        </span>
                      )}
                    </div>
                    <div className="text-[11px] text-slate-500 mt-1 flex flex-wrap gap-x-4 gap-y-1">
                      <span>Source: <strong className="font-mono text-slate-700">{src.original_device_id}</strong></span>
                      <span>Size: <strong className="text-slate-700">{formatBytes(src.image_size_bytes)}</strong></span>
                      {src.original_serial && <span>Serial: <strong className="font-mono text-slate-700">{src.original_serial}</strong></span>}
                    </div>
                    <div className="text-[10px] text-slate-400 font-mono mt-1 truncate max-w-xl">
                      SHA-256: {src.image_sha256}
                    </div>
                  </div>
                </div>
                {isSelected && (
                  <CheckCircle2 className="w-5 h-5 text-indigo-600 shrink-0" />
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
