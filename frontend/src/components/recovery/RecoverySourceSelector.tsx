import React, { useState } from 'react';
import {
  HardDrive,
  ShieldCheck,
  CheckCircle2,
  FolderOpen,
  FileCheck,
  AlertCircle,
  Loader2,
  FileDigit,
  RefreshCw,
} from 'lucide-react';
import { RecoverySourceSnapshot } from '../../types/recovery';
import { importLocalRecoverySource, pickRawImageFile } from '../../services/recovery';

interface RecoverySourceSelectorProps {
  sources: RecoverySourceSnapshot[];
  loading: boolean;
  selectedSourceId: string | null;
  onSelectSource: (source: RecoverySourceSnapshot) => void;
  onRefreshSources?: () => void;
  sessionToken?: string | null;
}

export const RecoverySourceSelector: React.FC<RecoverySourceSelectorProps> = ({
  sources,
  loading,
  selectedSourceId,
  onSelectSource,
  onRefreshSources,
  sessionToken,
}) => {
  const [sourceMode, setSourceMode] = useState<'local' | 'library'>(
    sources.length > 0 ? 'library' : 'local'
  );
  const [localPath, setLocalPath] = useState('');
  const [isImporting, setIsImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const [justImported, setJustImported] = useState<RecoverySourceSnapshot | null>(null);

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };

  const handleBrowseFile = async () => {
    try {
      const selected = await pickRawImageFile();
      if (selected) {
        setLocalPath(selected);
        setImportError(null);
      }
    } catch (err: unknown) {
      console.error('File picker failed:', err);
    }
  };

  const handleImportAndSelect = async () => {
    const trimmed = localPath.trim();
    if (!trimmed) {
      setImportError('Please enter or browse to a raw disk image file path.');
      return;
    }

    setIsImporting(true);
    setImportError(null);
    try {
      const snapshot = await importLocalRecoverySource(trimmed, sessionToken || undefined);
      setJustImported(snapshot);
      onSelectSource(snapshot);
      if (onRefreshSources) {
        onRefreshSources();
      }
    } catch (err: unknown) {
      const msg =
        err && typeof err === 'object' && 'message' in err
          ? (err as { message: string }).message
          : err instanceof Error
          ? err.message
          : 'Failed to inspect and validate raw image file.';
      setImportError(msg);
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <div className="space-y-4">
      {/* Tab Switcher: Local Computer File vs. Known Image Library */}
      <div className="flex border-b border-slate-200 gap-4">
        <button
          type="button"
          onClick={() => setSourceMode('local')}
          className={`pb-2.5 text-xs font-semibold flex items-center gap-1.5 border-b-2 transition-colors cursor-pointer ${
            sourceMode === 'local'
              ? 'border-indigo-600 text-indigo-700'
              : 'border-transparent text-slate-500 hover:text-slate-700'
          }`}
        >
          <FolderOpen className="w-4 h-4" />
          <span>Local Image File (.raw, .dd, .img, .bin, .iso)</span>
        </button>

        <button
          type="button"
          onClick={() => setSourceMode('library')}
          className={`pb-2.5 text-xs font-semibold flex items-center gap-1.5 border-b-2 transition-colors cursor-pointer ${
            sourceMode === 'library'
              ? 'border-indigo-600 text-indigo-700'
              : 'border-transparent text-slate-500 hover:text-slate-700'
          }`}
        >
          <HardDrive className="w-4 h-4" />
          <span>Acquired Images Library</span>
          <span className="text-[10px] px-1.5 py-0.2 rounded-full bg-slate-100 text-slate-600 font-mono">
            {sources.length}
          </span>
        </button>
      </div>

      {/* Mode 1: Local Computer File Input / Native Browse */}
      {sourceMode === 'local' && (
        <div className="space-y-4">
          <div className="bg-white border border-slate-200 rounded-lg p-5 shadow-2xs space-y-4">
            <div className="space-y-1">
              <h3 className="text-xs font-bold text-slate-900 flex items-center gap-2">
                <FileDigit className="w-4 h-4 text-indigo-600" />
                Select Forensic Disk Image from Local Computer
              </h3>
              <p className="text-[11px] text-slate-500">
                Provide a raw bitstream disk image (<code className="text-indigo-600 font-mono">.raw</code>,{' '}
                <code className="text-indigo-600 font-mono">.dd</code>,{' '}
                <code className="text-indigo-600 font-mono">.img</code>,{' '}
                <code className="text-indigo-600 font-mono">.bin</code>,{' '}
                <code className="text-indigo-600 font-mono">.iso</code>, or{' '}
                <code className="text-indigo-600 font-mono">.001</code>). LocardX will compute its streaming
                SHA-256 digest in read-only mode to anchor forensic chain-of-custody before carving.
              </p>
            </div>

            <div className="space-y-2">
              <label className="block text-[11px] font-semibold text-slate-700">
                Image File Path on Local System
              </label>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={localPath}
                  onChange={(e) => {
                    setLocalPath(e.target.value);
                    setImportError(null);
                  }}
                  placeholder="e.g. C:\Users\gurvi\Desktop\evidence.raw or /path/to/disk.dd"
                  className="flex-1 px-3 py-2 text-xs font-mono border border-slate-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500"
                  disabled={isImporting}
                />
                <button
                  type="button"
                  onClick={handleBrowseFile}
                  disabled={isImporting}
                  className="px-3.5 py-2 text-xs font-semibold rounded-md border border-slate-300 bg-slate-50 hover:bg-slate-100 text-slate-700 flex items-center gap-1.5 transition-colors cursor-pointer shrink-0 disabled:opacity-50"
                >
                  <FolderOpen className="w-3.5 h-3.5 text-slate-600" />
                  Browse File...
                </button>
                <button
                  type="button"
                  onClick={handleImportAndSelect}
                  disabled={isImporting || !localPath.trim()}
                  className="px-4 py-2 text-xs font-semibold rounded-md bg-indigo-600 hover:bg-indigo-700 text-white flex items-center gap-1.5 transition-colors cursor-pointer shrink-0 shadow-xs disabled:opacity-50"
                >
                  {isImporting ? (
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  ) : (
                    <FileCheck className="w-3.5 h-3.5" />
                  )}
                  {isImporting ? 'Verifying...' : 'Load & Verify Image'}
                </button>
              </div>
            </div>

            {isImporting && (
              <div className="p-3 bg-indigo-50 border border-indigo-200 rounded-md flex items-center gap-2.5 text-xs text-indigo-900">
                <Loader2 className="w-4 h-4 animate-spin text-indigo-600 shrink-0" />
                <span>Streaming cryptographic SHA-256 over image bitstream in read-only mode...</span>
              </div>
            )}

            {importError && (
              <div className="p-3 bg-rose-50 border border-rose-200 rounded-md flex items-start gap-2.5 text-xs text-rose-800">
                <AlertCircle className="w-4 h-4 text-rose-600 shrink-0 mt-0.5" />
                <div className="space-y-0.5">
                  <span className="font-semibold block">Failed to load evidence image</span>
                  <span>{importError}</span>
                </div>
              </div>
            )}

            {justImported && (
              <div className="p-3.5 bg-emerald-50 border border-emerald-300 rounded-md space-y-1.5">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-xs font-bold text-emerald-900">
                    <CheckCircle2 className="w-4 h-4 text-emerald-600" />
                    <span>Evidence Image Verified & Selected</span>
                  </div>
                  <span className="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded bg-emerald-200 text-emerald-900">
                    Ready for Carving
                  </span>
                </div>
                <div className="text-[11px] text-emerald-800 font-mono space-y-0.5">
                  <div>Path: <strong className="break-all">{justImported.image_path}</strong></div>
                  <div>Size: <strong>{formatBytes(justImported.image_size_bytes)}</strong> ({justImported.image_size_bytes} bytes)</div>
                  <div className="text-[10px] text-emerald-700 truncate">SHA-256: {justImported.image_sha256}</div>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Mode 2: Existing Sources in Library */}
      {sourceMode === 'library' && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <label className="block text-xs font-semibold text-slate-700">
              Verified Evidence Images in Database
            </label>
            {onRefreshSources && (
              <button
                type="button"
                onClick={onRefreshSources}
                disabled={loading}
                className="text-[11px] text-indigo-600 hover:text-indigo-800 flex items-center gap-1 font-medium cursor-pointer"
              >
                <RefreshCw className={`w-3 h-3 ${loading ? 'animate-spin' : ''}`} />
                Refresh
              </button>
            )}
          </div>

          {loading ? (
            <div className="p-8 text-center bg-slate-50 border border-slate-200 rounded-md text-slate-500 text-xs">
              Loading verified acquisition sources...
            </div>
          ) : sources.length === 0 ? (
            <div className="p-8 text-center bg-slate-50 border border-slate-200 rounded-md text-slate-500 space-y-2">
              <HardDrive className="w-8 h-8 text-slate-400 mx-auto" />
              <p className="text-xs font-medium text-slate-700">No Acquired Images in Database</p>
              <p className="text-[11px] text-slate-500 max-w-md mx-auto">
                No images were found in the database. Use the{' '}
                <button
                  type="button"
                  onClick={() => setSourceMode('local')}
                  className="text-indigo-600 font-semibold hover:underline"
                >
                  Local Image File
                </button>{' '}
                tab to pick any raw file (.raw, .dd, .img) from your computer.
              </p>
            </div>
          ) : (
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
                        <div
                          className={`p-2 rounded-md ${
                            isSelected ? 'bg-indigo-600 text-white' : 'bg-slate-100 text-slate-600'
                          }`}
                        >
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
                            <span>
                              Source: <strong className="font-mono text-slate-700">{src.original_device_id}</strong>
                            </span>
                            <span>
                              Size: <strong className="text-slate-700">{formatBytes(src.image_size_bytes)}</strong>
                            </span>
                            {src.original_serial && (
                              <span>
                                Serial: <strong className="font-mono text-slate-700">{src.original_serial}</strong>
                              </span>
                            )}
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
          )}
        </div>
      )}
    </div>
  );
};
